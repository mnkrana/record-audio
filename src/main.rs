#![allow(unused)]
mod audio_clip;

use chrono::Local;
use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

use crate::audio_clip::AudioClip;

#[derive(Debug, Subcommand)]
enum Commands {
    Record {
        name: Option<String>,
    },
    #[clap(arg_required_else_help = true)]
    Play {
        name: String,
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
            let name = name.unwrap_or_else(|| Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
            let clip = AudioClip::record(name)?;
            clip.export(format!("{}.wav", clip.name).as_str())?;
            println!("Audio clip saved as {}.wav", clip.name);
        }
        Commands::Play { name } => {
            println!("Play!");
            todo!()
        }
    };

    Ok(())
}
