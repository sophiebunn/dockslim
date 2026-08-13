use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use humansize::{format_size, BINARY};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use tar::Archive;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Cache,
    Artifact,
}

#[derive(Debug, Clone, Copy)]
enum Matcher {
    Contains(&'static str),
    Extension(&'static str),
}

impl Matcher {
    fn matches(self, path: &str) -> bool {
        match self {
            Matcher::Contains(substring) => path.contains(substring),
            Matcher::Extension(ext) => path.rsplit('.').next() == Some(ext) && path.contains('.'),
        }
    }
}

struct BloatPattern {
    matcher: Matcher,
    type_bloat: Category,
    return_text: &'static str,
}

// common stuff that ends up baked into layers by accident
const BLOAT_PATTERNS: &[BloatPattern] = &[
    BloatPattern { matcher: Matcher::Contains("var/lib/apt/lists/"), type_bloat: Category::Cache, return_text: "apt package lists - rm -rf /var/lib/apt/lists/* after install" },
    BloatPattern { matcher: Matcher::Contains("var/cache/apt/"), type_bloat: Category::Cache, return_text: "apt archive cache - apt-get clean after install" },
    BloatPattern { matcher: Matcher::Contains(".cache/pip/"), type_bloat: Category::Cache, return_text: "pip cache - use pip install --no-cache-dir" },
    BloatPattern { matcher: Matcher::Contains(".npm/_cacache/"), type_bloat: Category::Cache, return_text: "npm cache - npm ci then npm cache clean --force" },
    BloatPattern { matcher: Matcher::Contains("__pycache__/"), type_bloat: Category::Artifact, return_text: "python bytecode cache - set PYTHONDONTWRITEBYTECODE=1" },
    BloatPattern { matcher: Matcher::Extension("log"), type_bloat: Category::Artifact, return_text: "log files - don't bake logs into the image" },
    BloatPattern { matcher: Matcher::Contains(".git/"), type_bloat: Category::Artifact, return_text: "git metadata - add .git to .dockerignore" },
    BloatPattern { matcher: Matcher::Contains("tmp/"), type_bloat: Category::Artifact, return_text: "scratch files in /tmp - clean up in the same layer that wrote them" },
];

fn classify(path: &str) -> Option<&'static BloatPattern> {
    BLOAT_PATTERNS.iter().find(|p| p.matcher.matches(path))
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

                // hash map accumulator to track layer size and file count
                let mut stats: HashMap<&'static str, (u64, u64)> = HashMap::new();

                for entry in archive2.entries().expect("failed to read tar") {
                    let mut entry = entry.expect("failed to read entry");
                    let path_str = entry.path().expect("failed to read path").display().to_string();

                    if manifest.layers.contains(&path_str) {
                        // this is a layer tar, gzipped
                        let mut layer_data = Vec::new();
                        entry.read_to_end(&mut layer_data).expect("failed to read layer");

                        let decoder = GzDecoder::new(layer_data.as_slice());
                        let mut layer_archive = Archive::new(decoder);
                        let mut layer_size: u64 = 0;

                        for layer_entry in layer_archive.entries().expect("failed to read layer tar") {
                            let layer_entry = layer_entry.expect("failed to read layer entry");
                            let lpath = layer_entry.path().expect("failed to read path").display().to_string();
                            let lsize = layer_entry.size();
                            layer_size += lsize;

                            // bucket it into a bloat pattern if it looks bloaty
                            if let Some(pattern) = classify(&lpath) {
                                let stat = stats.entry(pattern.return_text).or_insert((0, 0));
                                stat.0 += 1;
                                stat.1 += lsize;
                            }
                        }

                        println!("{} - {}", path_str, format_size(layer_size, BINARY));
                    }
                }

                println!();
                get_bloat_analyze(&stats);
            } else {
                println!("Invalid Command, try --help");
            }
        }

        // no or unrecognizable subcommand given
        _ => println!("Invalid Command, try --help"),
    }
}

fn get_bloat_analyze(bloat_stats: &HashMap<&'static str, (u64, u64)>) {
    let mut entries: Vec<(&'static BloatPattern, u64, u64)> = bloat_stats
        .iter()
        .filter_map(|(return_text, (file_count, file_bytes))| BLOAT_PATTERNS.iter().find(|p| p.return_text == *return_text).map(|p| (p, *file_count, *file_bytes)))
        .collect();

    // sort entries in descending order by size
    entries.sort_by(|a, b| b.2.cmp(&a.2));

    println!("Analysis done. Found {} potential bloats", entries.len());
    if entries.is_empty() {
        println!("No bloat found!");
        return;
    }

    let mut total = 0;
    for (pattern, file_count, file_bytes) in &entries {
        let tag = match pattern.type_bloat {
            Category::Cache => "cache",
            Category::Artifact => "artifact",
        };
        println!("  {}  {} files  [{}]  {}", format_size(*file_bytes, BINARY), file_count, tag, pattern.return_text);
        total += file_bytes;
    }
}