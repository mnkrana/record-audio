#![allow(unused)]
use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
enum Commands
{    
    Record
    {
        name: Option<String>
    },    
    #[clap(arg_required_else_help = true)]
    Play
    {
        name: String
    }    
}

#[derive(Debug, Parser)]
#[clap(name="record-audio")]
#[clap(about="simple audio recorder")]
struct Cli{
    #[clap(subcommand)]
    command:Commands,
}

fn main() {
    
let args = Cli::parse();

match args.command
    {
        Commands::Record { name } =>
        {
            println!("Record!");
            todo!()
        }
        Commands::Play { name } =>
        {
            println!("Play!");
            todo!()
        }        
    };
}
