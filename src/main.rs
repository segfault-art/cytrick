// Copyright (c) 2026 Project Contributors
//
// Licensed under either of the following licenses, at your option:
//
// 1. The Apache License, Version 2.0
//    (https://www.apache.org/licenses/LICENSE-2.0)
// 2. The MIT License
//    (https://opensource.org/license/mit)
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use clap::{Parser, Subcommand};
use cytrick::github::install_github_project;
use cytrick::interpreter;
use cytrick::manager;
use cytrick::setup;
use std::process::{Command, Stdio};
use std::{path::PathBuf, process::exit};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Version,
    Setup,
    Run {
        #[arg(short, long, default_value = "default")]
        file: String,

        #[arg(short, long)]
        repo: Option<String>,
    },

    Update {
        #[arg(short, long, default_value = ".", conflicts_with = "web")]
        folder: String,

        #[arg(long)]
        web: bool,
    },

    Install {
        repo: String,
    },

    Remove {
        project: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) => {
            println!("Cytrick version {}", env!("CARGO_PKG_VERSION"));
        }

        Some(Commands::Setup) => {
            if let Err(err) = setup::setup() {
                eprintln!("Error during setup: {err}");
                exit(1);
            }
        }

        Some(Commands::Run { file, repo }) => {
            #[cfg(unix)]
            let mut final_file = PathBuf::from(".trick.unix");

            #[cfg(windows)]
            let mut final_file = PathBuf::from(".trick.win");

            if !final_file.exists() {
                final_file = PathBuf::from(".trick");
            }

            if file != "default" {
                final_file = PathBuf::from(file);
            }

            let trick = match interpreter::deserialize(final_file) {
                Ok(trick) => trick,
                Err(err) => {
                    eprintln!("Cannot deserialize .trick file: {err}");
                    exit(1);
                }
            };

            println!("Installing {} version {}", trick.name, trick.version);
            if let Err(err) = interpreter::execute_all(trick, repo.as_deref()) {
                eprintln!("Error running .trick file: {err}");
                exit(1);
            }

            println!("Installed project successfully!");
        }

        Some(Commands::Update { folder, web }) => {
            if web {
                let _ = Command::new("updater")
                    .args(["--", "yes"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
            } else {
                interpreter::update(&folder);
            }
        }

        Some(Commands::Install { repo }) => {
            if let Err(err) = install_github_project(&repo) {
                eprintln!("Cannot install project: {err}");
            }
        }

        Some(Commands::Remove { project }) => {
            if let Err(err) = manager::remove_project(&project) {
                eprintln!("Cannot remove project: {err}");
            }
        }

        None => {
            eprintln!("Cytrick: No command");
            exit(1);
        }
    }
}
