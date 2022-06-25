use std::path::Path;

use clap::{Parser, Subcommand};
use color_eyre::eyre::{eyre, Result};
use record_audio::audio_clip::AudioClip;

#[derive(Debug, Subcommand)]
enum Commands {
    Record {
        name: Option<String>,
    },
    #[clap(arg_required_else_help = true)]
    Play {
        path: String        
    },
}

#[derive(Debug, Parser)]
#[clap(name = "record-audio")]
#[clap(about = "simple audio recorder")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Record { name } => {
            let clip = AudioClip::record(name)?;
            clip.export(format!("{}.wav", clip.name).as_str())?;
            println!("Audio clip saved as {}.wav", clip.name);
        }
        Commands::Play { path } => {
            Path::new(&path)
                .file_stem()
                .ok_or_else(|| eyre!("Invalid path: {}", path))?
                .to_str()
                .ok_or_else(|| eyre!("Path is not utf8"))?
                .to_string();

            let clip = AudioClip::import(path)?;
            clip.play()?;
        }
    };

    Ok(())
}
