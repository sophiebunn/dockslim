use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Analyze {
        image: String,
    },
}

fn main() {
    let args = Cli::parse();
    match args.command {
        Some(Commands::Analyze { image }) => {
            println!("Analyzing Image: {}", image);
            let status = Command::new("docker")
                .arg("save")
                .arg("-o")
                .arg("image.tar")
                .arg(&image)
                .status()
                .expect("failed to execute process");

            if status.success() {
                println!("Success");
            } else {
                println!("Invalid Command, try --help");
            }
        }
        _ => println!("Invalid Command, try --help"),
    }
}