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

pub mod interpreter;
pub mod setup;

use std::{path::PathBuf, process::exit};

use clap::{Parser, Subcommand};

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
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) => {
            println!("Cytrick version 0.1.0.0");
        }

        Some(Commands::Setup) => {
            if let Err(message) = setup::setup() {
                eprintln!("Error during setup: {message}");
                exit(1);
            }
        }

        Some(Commands::Run { file }) => {
            let trick = match interpreter::deserialize(PathBuf::from(file)) {
                Ok(trick) => trick,
                Err(message) => {
                    eprintln!("Cannot deserialize .trick file: {message}");
                    exit(1);
                }
            };

            println!("Installing {} version {}", trick.name, trick.version);
            if let Err(message) = interpreter::execute_all(trick) {
                eprintln!("Cannot run .trick file: {message}");
                exit(1);
            }
        }

        None => {
            eprintln!("Cytrick: No command");
            exit(1);
        }
    }
}
