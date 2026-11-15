// Copyright (c) 2026 Segfault
//
// Licensed under either of the following licenses, at your option:
//
// 1. The Apache License, Version 2.0
//    (https://www.apache.org/licenses/LICENSE-2.0)
// 2. The MIT License
//    (https://opensource.org/license/mit)
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{path::PathBuf, process::exit};

use clap::{Parser, Subcommand};

use cytrick::interpreter;
use cytrick::setup;

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
        #[arg(long, default_value = ".trick")]
        file: String,
    },

    Update {
        #[arg(long, default_value = ".")]
        folder: String,
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

        Some(Commands::Run { file }) => {
            let trick = match interpreter::deserialize(PathBuf::from(file)) {
                Ok(trick) => trick,
                Err(err) => {
                    eprintln!("Cannot deserialize .trick file: {err}");
                    exit(1);
                }
            };

            println!("Installing {} version {}", trick.name, trick.version);
            if let Err(err) = interpreter::execute_all(trick) {
                eprintln!("Error running .trick file: {err}");
                exit(1);
            }

            println!("Installed project successfully!");
        }

        Some(Commands::Update { folder }) => {
            interpreter::update(&folder);
        }

        None => {
            eprintln!("Cytrick: No command");
            exit(1);
        }
    }
}
