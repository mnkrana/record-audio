// #![allow(unused)]

use std::sync::{mpsc::channel, Arc, Mutex};

use color_eyre::eyre::eyre;
use color_eyre::eyre::Result;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;

type ClipHandle = Arc<Mutex<Option<AudioClip>>>;

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
