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
use cytrick::{github::install_github_project, interpreter, manager, setup};
use home::home_dir;
use std::env::{current_dir, set_current_dir};
use std::{fs::remove_file, path::PathBuf, process::exit};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(hide = true)]
    AfterUpdate,
    Version,
    Setup,
    List,
    Run {
        #[arg(short, long, default_value = "default", conflicts_with = "nothing")]
        file: String,

        #[arg(short, long, default_value = ".", conflicts_with = "nothing")]
        folder: String,

        #[arg(short, long, conflicts_with = "nothing")]
        repo: Option<String>,

        #[arg(short, long)]
        no_load: bool,

        #[arg(short, long)]
        nothing: bool,
    },

    Install {
        repo: String,

        #[arg(short, long)]
        no_load: bool,
    },

    Remove {
        project: String,
    },

    Query {
        project: String,
    },

    Upgrade {
        #[arg(conflicts_with = "web", default_value = ".")]
        dir: PathBuf,

        #[arg(short, long)]
        web: bool,
    },
}
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::AfterUpdate) => {
            let home = home_dir().expect("Error getting home dir");

            remove_file(home.join(".cytrick/Cytrick/cyt.tmp"))
                .expect("Error deleting temporary file");
        }

        Some(Commands::Version) => {
            println!("Cytrick version {}", env!("CARGO_PKG_VERSION"));
        }

        Some(Commands::Setup) => {
            if let Err(err) = setup::setup() {
                eprintln!("Error during setup: {err}");
                exit(1);
            }
        }

        Some(Commands::List) => {
            let names = match manager::list_all() {
                Ok(names) => names,
                Err(err) => {
                    eprintln!("{err}");
                    exit(1);
                }
            };

            for name in names {
                println!("{name}");
            }
        }

        Some(Commands::Run {
            file,
            folder,
            repo,
            no_load,
            nothing,
        }) => {
            #[cfg(unix)]
            if nothing {
                interpreter::nothing(!no_load);
            }

            let mut final_file = PathBuf::from(".trick.unix");

            #[cfg(windows)]
            let mut final_file = PathBuf::from(".trick.win");

            if !final_file.exists() {
                final_file = PathBuf::from(".trick");
            }

            if file != "default" {
                final_file = PathBuf::from(file);
            }

            let pwd = current_dir().expect("Error getting current directory");
            set_current_dir(folder).expect("Error setting current directory");

            let trick = match interpreter::deserialize(&final_file) {
                Ok(trick) => trick,
                Err(err) => {
                    eprintln!("Cannot deserialize .trick file: {err}");
                    set_current_dir(pwd).expect("Error setting current directory");
                    exit(1);
                }
            };

            println!("Installing {} version {}", trick.name, trick.version);
            if let Err(err) = interpreter::execute_all(trick, repo.as_deref(), !no_load, true) {
                eprintln!("Error running .trick file: {err}");
                set_current_dir(pwd).expect("Error setting current directory");
                exit(1);
            }

            println!("Installed project successfully!");
            set_current_dir(pwd).expect("Error setting current directory");
        }

        Some(Commands::Install { repo, no_load }) => {
            if let Err(err) = install_github_project(&repo, !no_load) {
                eprintln!("Cannot install project: {err}");
            }
        }

        Some(Commands::Remove { project }) => {
            if let Err(err) = manager::remove_project(&project) {
                eprintln!("Cannot remove project: {err}");
            }
        }

        Some(Commands::Query { project }) => {
            let project = match manager::query_project(&project) {
                Ok(project) => project,
                Err(err) => {
                    eprintln!("Cannot query project: {err}");
                    exit(1);
                }
            };

            println!(
                "binaries: {}\nversion: {}\nname: {}\nrepo: {}",
                project.binaries.join(", "),
                project.version,
                project.name,
                project.repo.unwrap_or(String::from("No repo")),
            )
        }

        Some(Commands::Upgrade { dir, web }) => {
            interpreter::upgrade(&dir, web);
        }

        None => {
            eprintln!("Cytrick: No command");
            exit(1);
        }
    }
}
