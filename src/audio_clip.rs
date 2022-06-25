use std::fs::File;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::{mpsc::channel, Arc, Mutex};

use color_eyre::eyre::eyre;
use color_eyre::eyre::Result;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use dasp::interpolate::linear::Linear;
use dasp::{signal, Signal};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

type ClipHandle = Arc<Mutex<Option<AudioClip>>>;
type StateHandle = Arc<Mutex<Option<(usize, Vec<f32>, Sender<()>)>>>;

#[derive(Clone)]
pub struct AudioClip {
    pub name: String,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl AudioClip {
    pub fn record(name: String) -> Result<AudioClip> {
        //get the host
        let host = cpal::default_host();

        //get the default input device
        let device = host
            .default_input_device()
            .ok_or_else(|| eyre!("No input device!"))?;
        println!("Input device: {}", device.name()?);

        //get default config - channels, sample_rate,buffer_size, sample_format
        let config = device.default_input_config()?;

        //init a audio clip
        let clip = AudioClip {
            name,
            samples: Vec::new(),
            sample_rate: config.sample_rate().0,
        };

        let clip = Arc::new(Mutex::new(Some(clip)));

        // Run the input stream on a separate thread.
        let clip_2 = clip.clone();

        println!("Begin recording...");

        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        //get number of channels
        let channels = config.channels();

        //create stream
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<f32>(data, channels, &clip_2),
                err_fn,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i16>(data, channels, &clip_2),
                err_fn,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<u16>(data, channels, &clip_2),
                err_fn,
            )?,
        };

        //run stream
        stream.play()?;

        //ctrl c signal
        let (tx, rx) = channel();
        ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))?;
        println!("Waiting for Ctrl-C...");

        rx.recv()?;
        println!("\nGot it! Exiting...");

        drop(stream);
        let clip = clip.lock().unwrap().take().unwrap();
        eprintln!("Recorded {} samples", clip.samples.len());
        Ok(clip)
    }

    pub fn export(&self, path: &str) -> Result<()> {
        if !path.ends_with(".wav") {
            return Err(eyre!("Expected {} to end in .wav", path));
        }

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;
        for sample in &self.samples {
            writer.write_sample(*sample)?;
        }

        writer.finalize()?;

        Ok(())
    }

    pub fn import(name: String, path: String) -> Result<AudioClip> {
        // Create a media source
        let file = Box::new(File::open(Path::new(&path))?);        

        // Create the media source stream
        let mss = MediaSourceStream::new(file, Default::default());

        // Create a hint to help the format registry
        let hint = Hint::new();

        // Use the default options when reading and decoding.
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        // Probe the media source stream for a format.
        let probed =
            symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;

        // Get the format reader yielded by the probe operation.
        let mut format = probed.format;

        // Get the default track.
        let track = format
            .default_track()
            .ok_or_else(|| eyre!("No default track"))?;

        // Create a decoder for the track.
        let mut decoder =
            symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)?;

        // Store the track identifier, we'll use it to filter packets.
        let track_id = track.id;

        let mut sample_count = 0;
        let mut sample_buf = None;
        let channels = track
            .codec_params
            .channels
            .ok_or_else(|| eyre!("Unknown channel count"))?;

        let mut clip = AudioClip {
            name,
            samples: Vec::new(),
            sample_rate: track
                .codec_params
                .sample_rate
                .ok_or_else(|| eyre!("Unknown sample rate"))?,
        };

        loop {
            // Get the next packet from the format reader.
            let packet = match format.next_packet() {
                Ok(packet_ok) => packet_ok,
                Err(Error::IoError(ref packet_err))
                    if packet_err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(packet_err) => {
                    return Err(packet_err.into());
                }
            };

            // If the packet does not belong to the selected track, skip it.
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet into audio samples, ignoring any decode errors.
            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    // If this is the *first* decoded packet, create a sample buffer matching the
                    // decoded audio buffer format.
                    if sample_buf.is_none() {
                        // Get the audio buffer specification.
                        let spec = *audio_buf.spec();

                        // Get the capacity of the decoded buffer. Note: This is capacity, not length!
                        let duration = audio_buf.capacity() as u64;

                        // Create the f32 sample buffer.
                        sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                    }

                    // Copy the decoded audio buffer into the sample buffer in an interleaved format.
                    if let Some(buf) = &mut sample_buf {
                        buf.copy_interleaved_ref(audio_buf);
                        let mono: Vec<f32> = buf
                            .samples()
                            .iter()
                            .step_by(channels.count())
                            .copied()
                            .collect();
                        clip.samples.extend_from_slice(&mono);

                        // The samples may now be access via the `samples()` function.
                        sample_count += buf.samples().len();
                        log::info!("\rDecoded {} samples", sample_count);
                    }
                }
                Err(Error::DecodeError(_)) => (),
                Err(_) => break,
            }
        }

        Ok(clip)
    }

    pub fn play(&self) -> Result<()> {
        //get the host
        let host = cpal::default_host();

        //get the default output device
        let device = host
            .default_output_device()
            .ok_or_else(|| eyre!("No output device!"))?;
        println!("Output device: {}", device.name()?);

        //get default config - channels, sample_rate,buffer_size, sample_format
        let config = device.default_output_config()?;

        println!("Begin playback...");

        //get number of channels
        let channels = config.channels();
        let sample_rate = config.sample_rate().0;

        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        
        let (done_tx, done_rx) = channel::<()>();
        let state = (0, self.resample(sample_rate).samples, done_tx);
        let state = Arc::new(Mutex::new(Some(state)));

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data, _: &_| write_output_data::<f32>(data, channels, &state),
                err_fn,
            )?,
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config.into(),
                move |data, _: &_| write_output_data::<i16>(data, channels, &state),
                err_fn,
            )?,
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config.into(),
                move |data, _: &_| write_output_data::<u16>(data, channels, &state),
                err_fn,
            )?,
        };

        stream.play()?;

        done_rx.recv()?;

        Ok(())
    }

    pub fn resample(&self, sample_rate: u32) -> AudioClip {
        if sample_rate == self.sample_rate {
            return self.clone();
        }

        let mut signal = signal::from_iter(self.samples.iter().copied());
        let a = signal.next();
        let b = signal.next();

        let linear = Linear::new(a, b);

        let clip = AudioClip {            
            name: self.name.clone(),            
            samples: signal
                .from_hz_to_hz(linear, self.sample_rate as f64, sample_rate as f64)
                .take(self.samples.len() * (sample_rate as usize) / (self.sample_rate as usize))
                .collect(),
            sample_rate: sample_rate,
        };

        clip
    }
}

fn write_input_data<T>(input: &[T], channels: u16, writer: &ClipHandle)
where
    T: cpal::Sample,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for frame in input.chunks(channels.into()) {
                writer.samples.push(frame[0].to_f32());
            }
        }
    }
}

fn write_output_data<T>(output: &mut [T], channels: u16, writer: &StateHandle)
where
    T: cpal::Sample,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some((i, clip_samples, done)) = guard.as_mut() {
            for frame in output.chunks_mut(channels.into()) {
                for sample in frame.iter_mut() {
                    *sample = cpal::Sample::from(clip_samples.get(*i).unwrap_or(&0f32));
                }
                *i += 1;
            }

            if *i >= clip_samples.len() {
                if let Err(_) = done.send(()) {
                    //playback has already stopped
                }
            }
        }
    }
}
