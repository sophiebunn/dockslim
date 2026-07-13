use clap::{Parser, Subcommand};
use std::process::Command;
use std::fs::File;
use std::io::Read;
use tar::Archive;
use flate2::read::GzDecoder;
use serde::{Serialize, Deserialize};

// describe the shape of the commands --help and --version
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    // exports the image so we can inspect it later
    Analyze {
        image: String,
    },
}

// struct for reading the manifest json with serde
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Manifest {
    #[serde(rename = "Config")]
    config: String,
    #[serde(rename = "RepoTags")]
    repo_tags: Vec<String>,
    #[serde(rename = "Layers")]
    layers: Vec<String>,
}


fn main() {
    let args = Cli::parse();
    
    match args.command {
        Some(Commands::Analyze { image }) => {
            println!("Analyzing Image: {}", image);

            // use docker save to get status and analyze
            let status = Command::new("docker")
                .arg("save")
                .arg("-o")
                .arg("image.tar")
                .arg(&image)
                .status()
                .expect("failed to execute process");

            if status.success() {
                // open the .tar file and loop through the entries
                let file = File::open("image.tar").expect("failed to open file");
                let mut archive = Archive::new(file);
                let mut manifest: Option<Manifest> = None;

                for entry in archive.entries().expect("failed to read tar") {
                    // print each entry and its data
                    let mut entry = entry.expect("failed to read entry");
                    let path = entry.path().expect("failed to read path");
                    let size = entry.size();
                    let path_str = path.display().to_string();

                    if path_str == "manifest.json" {
                        // read manifest.json contents only
                        let mut contents = String::new();
                        entry.read_to_string(&mut contents).expect("failed to read manifest");
                        manifest = Some(serde_json::from_str::<Vec<Manifest>>(&contents).expect("failed to parse manifest")[0].clone());

                    }
                }

                // second for loop to get whats in the right entry from the manifest
                let file2 = File::open("image.tar").expect("failed to open file");
                let mut archive2 = Archive::new(file2);
                let manifest = manifest.expect("no manifest found");

                for entry in archive2.entries().expect("failed to read tar") {
                    let mut entry = entry.expect("failed to read entry");
                    let path_str = entry.path().expect("failed to read path").display().to_string();

                    if manifest.layers.contains(&path_str) {
                        // this is a tar file
                        let mut layer_data = Vec::new();
                        entry.read_to_end(&mut layer_data).expect("failed to read layer");

                        let decoder = GzDecoder::new(layer_data.as_slice());
                        let mut layer_archive = Archive::new(decoder);
                        
                        for layer_entry in layer_archive.entries().expect("failed to read layer tar") {
                            let layer_entry = layer_entry.expect("failed to read layer entry");
                            let lpath = layer_entry.path().expect("failed to read path");
                            let lsize = layer_entry.size();
                            println!("{} - {} bytes", lpath.display(), lsize);
                        }
                    }
                }


            } else {
                println!("Invalid Command, try --help");
            }
        }

        // no or unrecognizable subcommand given
        _ => println!("Invalid Command, try --help"),
    }
}