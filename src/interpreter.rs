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

use home::home_dir;
use rand::prelude::IndexedRandom;
use rusqlite::ErrorCode;
use safer_ffi::prelude::*;
use serde::{Deserialize, Deserializer};
use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
use std::{
    cell::RefCell,
    collections::HashMap,
    env::{self, consts::EXE_SUFFIX},
    fmt, fs, io,
    mem::transmute,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, exit},
};
use symlink::{remove_symlink_file, symlink_file};
use zip::{ZipArchive, read::root_dir_common_filter};

use crate::{extension, extension::load_all, github::install_github_project, sql};

macro_rules! installer_factory {
    ($fn_name:ident, $folder:expr, $datat:expr) => {
        fn $fn_name(args: &[&str], data: &ExecutionData) -> Result<Status, String> {
            let home = home_dir().ok_or(String::from("Cannot get the home folder"))?;
            let result = install_something(args, data, &home.join(".cytrick").join($folder))?;

            Ok(Status::Installed {
                data: $datat,
                name: result.name,
                file: result.file,
                source: result.source,
            })
        }
    };
}

thread_local! {
    pub static MODIFIERS: RefCell<HashMap<String, extension::ModifierHandler>> = RefCell::new(HashMap::new());
}

#[derive(Debug)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Deserialize)]
pub struct Trick {
    #[serde(deserialize_with = "deserialize_version")]
    pub version: Version,
    pub name: String,
    build: Vec<String>,
}

pub struct ExecutionData<'a> {
    #[allow(dead_code)]
    pub version: &'a Version,
    pub name: &'a str,
}

#[allow(dead_code)]
pub enum Status {
    Installed {
        data: DataType,
        name: String,
        file: PathBuf,
        source: PathBuf,
    },

    Downloaded {
        url: String,
        file: PathBuf,
    },

    Extracted {
        item: PathBuf,
        source: PathBuf,
    },

    Ran {
        program: String,
        args: Vec<String>,
    },

    Shell {
        command: String,
    },
}

pub enum DataType {
    Binary,
    Extension,
    File,
}

struct InstalledSomething {
    name: String,
    file: PathBuf,
    source: PathBuf,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn deserialize_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;

    let parts: Vec<u32> = value
        .split('.')
        .map(|part| part.parse().map_err(serde::de::Error::custom))
        .collect::<Result<_, _>>()?;

    if parts.len() != 3 {
        return Err(serde::de::Error::custom("Expected version in format x.x.x"));
    }

    Ok(Version {
        major: parts[0],
        minor: parts[1],
        patch: parts[2],
    })
}

pub fn deserialize(path: &Path) -> Result<Trick, Box<dyn std::error::Error>> {
    let yaml = fs::read_to_string(path)?;
    let config: Trick = noyalib::from_str(&yaml)?;

    Ok(config)
}

fn system(command: &str) -> io::Result<ExitStatus> {
    #[cfg(unix)]
    let mut process = {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());

        let mut cmd = Command::new(shell);
        cmd.args(["-c", command]);
        cmd
    };

    #[cfg(windows)]
    let mut process = {
        let shell = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into());

        let mut cmd = Command::new(shell);
        cmd.args(["/C", command]);
        cmd
    };

    process.status()
}

pub fn execute_all(
    trick: Trick,
    repo: Option<&str>,
    load: bool,
    delete: bool,
) -> Result<(), String> {
    if load {
        load_all().map_err(|e| format!("Error loading extensions: {e}"))?;
    }

    let home = home_dir().ok_or(String::from("Cannot get home dir"))?;
    let mut binaries: Vec<String> = Vec::new();
    let mut extensions: Vec<String> = Vec::new();
    let conn = sql::create_connection()
        .map_err(|e| format!("Cannot open data file: {e}, try run `cyt setup`"))?;

    let data = ExecutionData {
        version: &trick.version,
        name: &trick.name,
    };

    if delete && let Ok(project) = sql::get_project(&conn, &trick.name) {
        if let Err(err) = fs::remove_dir_all(home.join(".cytrick").join(&trick.name)) {
            eprintln!("Error removing old project folder: {err}");
        }

        for binary in &project.binaries {
            if let Err(err) = fs::remove_file(binary) {
                eprintln!("Error removing binary {binary}: {err}");
            }
        }
    }

    for (i, command) in (1..).zip(trick.build) {
        match execute(&command, &data) {
            Ok(status) => {
                if let Status::Installed { data, name, .. } = status {
                    match data {
                        DataType::Binary => binaries.push(name),
                        DataType::Extension => extensions.push(name),
                        _ => {}
                    }
                }
            }

            Err(e) => {
                if let Err(err) = fs::remove_dir_all(home.join(".cytrick").join(&trick.name))
                    && err.kind() != io::ErrorKind::NotFound
                {
                    eprintln!("Error removing project dir: {err}. Try remove manually later");
                }

                for binary in &binaries {
                    println!("Removing binary {binary}");

                    if let Err(err) = fs::remove_file(home.join(".cytrick/bin").join(binary)) {
                        eprintln!(
                            "Error removing binary {binary}: {err}. Try remove manually later"
                        );
                    }
                }

                for extension in &extensions {
                    println!("Removing extension {extension}");

                    if let Err(err) = fs::remove_file(home.join(".cytrick/ext").join(extension)) {
                        eprintln!("Error removing extension {extension}: {err}",)
                    }
                }

                return Err(format!(
                    "Error executing command ({command}): {e}. Command nº {i}"
                ));
            }
        }
    }

    if let Err(err) = sql::new_project(
        &conn,
        &sql::ProjectRef {
            binaries: &binaries.iter().map(String::as_str).collect::<Vec<&str>>(),
            extensions: &extensions.iter().map(String::as_str).collect::<Vec<&str>>(),
            version: &trick.version,
            name: &trick.name,
            repo,
        },
    ) {
        if let Some(ErrorCode::ConstraintViolation) = err.sqlite_error_code() {
            eprintln!("Project already installed, overwriting...");

            sql::update_project(
                &conn,
                &trick.name,
                &HashMap::from([
                    ("binaries", Some::<&str>(&binaries.join(","))),
                    ("extensions", Some::<&str>(&extensions.join(","))),
                    ("version", Some(&trick.version.to_string())),
                    ("repo", repo),
                ]),
            )
            .map_err(|e| format!("Failed to update project in database: {e}"))?;

            Ok(())
        } else {
            Err(format!("Failed to add project to database: {err}"))
        }
    } else {
        Ok(())
    }
}

fn execute(command: &str, data: &ExecutionData) -> Result<Status, String> {
    let mut split = command.split_whitespace();

    if let Some(function) = split.next() {
        let args = process_args(&split.collect::<Vec<&str>>());

        execute_function(
            function,
            &args.iter().map(String::as_ref).collect::<Vec<&str>>(),
            data,
        )
    } else {
        Err(String::from("Empty"))
    }
}

fn process_args(args: &[&str]) -> Vec<String> {
    let mut i = 0;
    let mut result: Vec<String> = Vec::new();

    while i < args.len() {
        if args[i].starts_with("%{") && args[i].ends_with("}%") {
            match args[i] {
                "%{bin}%" => {
                    if args.len() == i + 1 {
                        eprintln!("Warning: Unused modifier 'bin'");
                    } else {
                        result.push(format!("{}{EXE_SUFFIX}", args[i + 1]));
                    }

                    i += 2;
                }

                "%{win}%" => {
                    if args.len() == i + 1 {
                        eprintln!("Warning: Unused modifier 'win'");
                    } else {
                        #[cfg(windows)]
                        result.push(args[i + 1].to_string());
                    }

                    i += 2;
                }

                "%{unix}%" => {
                    if args.len() == i + 1 {
                        eprintln!("Warning: Unused modifier 'unix'");
                    } else {
                        #[cfg(unix)]
                        result.push(args[i + 1].to_string());
                    }

                    i += 2;
                }

                "%{dll}%" => {
                    if args.len() == i + 1 {
                        eprintln!("Warning: Unused modifier 'lib'");
                    } else {
                        result.push(format!("{DLL_PREFIX}{}{DLL_SUFFIX}", args[i + 1]));
                    }
                }

                _ => {
                    let modifier = match MODIFIERS.with_borrow(|mods| mods.get(args[i]).copied()) {
                        Some(m) => m,
                        None => {
                            eprintln!("Warning: Unknown modifier: '{}'", args[i]);
                            i += 1;
                            continue;
                        }
                    };

                    unsafe {
                        let owned_args: Vec<char_p::Box> =
                            args.iter().map(|v| char_p::new(*v)).collect();

                        let args_refs: Vec<char_p::Ref<'_>> =
                            owned_args.iter().map(char_p::Box::as_ref).collect();

                        let args_c: c_slice::Ref<'_, char_p::Ref<'_>> =
                            c_slice::Ref::from(&args_refs[..]);

                        let mut owned_result: Vec<char_p::Box> =
                            result.iter().map(|s| char_p::new(s.as_str())).collect();

                        let result_c: c_slice::Mut<'_, char_p::Box> =
                            c_slice::Mut::from(owned_result.as_mut_slice());

                        let args_static = transmute::<
                            c_slice::Ref<'_, char_p::Ref<'_>>,
                            c_slice::Ref<'static, char_p::Ref<'static>>,
                        >(args_c);

                        let result_static: c_slice::Mut<'static, char_p::Box> =
                            transmute::<c_slice::Mut<'_, _>, c_slice::Mut<'static, _>>(result_c);

                        modifier(i, args_static, result_static);
                    }
                }
            }
        } else {
            result.push(args[i].into());
            i += 1;
        }
    }

    result
}

fn run(args: &[&str]) -> Result<Status, String> {
    let program = String::from(*args.first().ok_or(String::from("No program"))?);
    let args = args
        .get(1..)
        .unwrap_or(&[])
        .iter()
        .map(|e| String::from(*e))
        .collect::<Vec<String>>();

    let status = Command::new(&program)
        .args(&args)
        .status()
        .map_err(|e| format!("Cannot spawn process: {e}"))?;

    if status.success() {
        Ok(Status::Ran { program, args })
    } else {
        Err(format!("Program returned error: {status}"))
    }
}

fn shell(args: &[&str]) -> Result<Status, String> {
    let command = args.join(" ");
    let status = system(&command).map_err(|e| format!("Cannot execute command: {e}"))?;

    if status.success() {
        Ok(Status::Shell { command })
    } else {
        Err(format!("Command returned error: {status}"))
    }
}

fn download(args: &[&str]) -> Result<Status, String> {
    let url = *args.first().ok_or(String::from("No URL"))?;
    let file = PathBuf::from(args.get(1).ok_or(String::from("No destination path"))?);
    let mut response = reqwest::blocking::get(url)
        .map_err(|e| format!("Cannot download file: {e}"))?
        .error_for_status()
        .map_err(|e| format!("URL returned bad status: {e}"))?;

    fs::create_dir_all(
        file.parent()
            .ok_or(String::from("Cannot access folder's parent"))?,
    )
    .map_err(|e| format!("Cannot create directory: {e}"))?;

    response
        .copy_to(&mut fs::File::create(&file).map_err(|e| format!("Cannot create file: {e}"))?)
        .map_err(|e| format!("Cannot write in file: {e}"))?;

    Ok(Status::Downloaded {
        url: url.to_owned(),
        file,
    })
}

pub fn extract(args: &[&str]) -> Result<Status, String> {
    let source = PathBuf::from(*args.first().ok_or("No archive")?);
    let dest = PathBuf::from(*args.get(1).ok_or("No destination path")?);

    let file = fs::File::open(&source).map_err(|e| format!("Cannot open source archive: {e}"))?;
    let mut zip = ZipArchive::new(file).map_err(|e| format!("Invalid zip: {e}"))?;

    if let Some(remove_root) = args.get(2) {
        if *remove_root == "no-root" {
            zip.extract_unwrapped_root_dir(&dest, root_dir_common_filter)
                .map_err(|e| format!("Cannot extract directory: {e}"))?;
        } else {
            return Err(format!("Unknown option: {remove_root}"));
        }
    } else {
        zip.extract(&dest)
            .map_err(|e| format!("Cannot extract directory: {e}"))?;
    }

    Ok(Status::Extracted { item: dest, source })
}

pub fn install(args: &[&str], data: &ExecutionData) -> Result<Status, String> {
    let home = home_dir().ok_or(String::from("Cannot get the home folder"))?;
    let name = get_name(args[0], args.get(1).copied())?;
    let source = args.first().ok_or(String::from("No source file"))?;
    let file = home.join(format!(".cytrick/{}/{}", data.name, name));

    fs::create_dir_all(home.join(format!(".cytrick/{}", data.name)))
        .map_err(|e| format!("Cannot create project directory: {e}"))?;
    fs::copy(source, &file).map_err(|e| format!("Cannot copy file: {e}"))?;

    Ok(Status::Installed {
        data: DataType::File,
        name,
        file,
        source: source.into(),
    })
}

fn get_name(path: &str, new_name: Option<&str>) -> Result<String, String> {
    Ok(new_name
        .unwrap_or(
            PathBuf::from(path)
                .file_name()
                .ok_or(String::from("Cannot get file name"))?
                .to_str()
                .ok_or(String::from("Not valid UTF-8"))?,
        )
        .to_owned())
}

fn install_something(
    args: &[&str],
    data: &ExecutionData,
    folder: &Path,
) -> Result<InstalledSomething, String> {
    let home = home_dir().ok_or(String::from("Cannot get the home folder"))?;
    let name = get_name(args[1], args.get(2).copied())?;
    let file = folder.join(&name);
    let source = PathBuf::from(args.get(1).ok_or(String::from("No source file"))?);
    let link_source = home.join(format!(".cytrick/{}/{}", data.name, name));

    install(args.get(1..).unwrap(), data)?;

    if let Err(err) = symlink_file(&link_source, &file) {
        if err.kind() == io::ErrorKind::AlreadyExists {
            println!("Symlink already exists, overwriting");

            remove_symlink_file(&file).map_err(|e| format!("Cannot remove symlink: {e}"))?;
            symlink_file(&link_source, &file).map_err(|e| format!("Cannot create symlink: {e}"))?;
        } else {
            return Err(format!("Cannot create symlink: {err}"));
        }
    }

    Ok(InstalledSomething { name, file, source })
}

installer_factory!(install_bin, "bin", DataType::Binary);
installer_factory!(install_ext, "ext", DataType::Extension);

fn execute_function(function: &str, args: &[&str], data: &ExecutionData) -> Result<Status, String> {
    match function {
        "run" => run(args),
        "shell" => shell(args),
        "download" => download(args),
        "extract" => extract(args),
        "install" if *args.first().ok_or(String::from("No source file"))? == "bin" => {
            install_bin(args, data)
        }

        "install" if *args.first().ok_or(String::from("No source file"))? == "ext" => {
            install_ext(args, data)
        }

        "install" => install(args, data),
        _ => Err(format!("Unknown function: {function}")),
    }
}

pub fn upgrade(dir: &Path, web: bool) -> ! {
    let original = env::current_exe().expect("Cannot get binary directory");

    fs::rename(&original, original.with_extension("tmp")).expect("Cannot rename file");
    println!("{}", original.display());

    if web && let Err(err) = install_github_project("segfault-art/Cytrick", false) {
        eprintln!("Error updating from github: {err}");
        exit(1);
    }

    if !dir.is_empty() && !web {
        let trick = match deserialize(&dir.join(".trick")) {
            Ok(trick) => trick,
            Err(err) => {
                eprintln!("Error deserializing trick file: {err}");
                exit(1);
            }
        };

        if let Err(err) = execute_all(trick, Some("segfault-art/Cytrick"), false, false) {
            eprintln!("Error executing trick file: {err}");
            exit(1);
        }
    }

    if let Err(err) = Command::new(&original).args(["after-update"]).spawn() {
        eprintln!("Error executing clean after update: {err}");
        exit(1);
    }

    exit(0);
}

pub fn nothing(load: bool) -> ! {
    let version = Version {
        major: 0,
        minor: 0,
        patch: 0,
    };

    let trick = Trick {
        version,
        name: String::from("Nothing"),
        build: Vec::new(),
    };

    execute_all(trick, None, load, true).expect("Error executing trick file");

    let conn = sql::create_connection().expect("Cannot create database connection");

    sql::delete_project(&conn, "Nothing").expect("Cannot delete project from database");

    let phrases = [
        "Nothing to see here",
        "Achievement unlocked: doing nothing",
        "Absolutely nothing happened, and it was glorious",
        "Zero changes, zero problems",
        "I’ve done nothing, and I’m quite proud of it",
        "Successfully did nothing",
        "Done. (Nothing was changed, as expected)",
        "0 files changed, 0 regrets",
        "Nothing to do. Exactly as planned",
        "A perfectly executed no-op",
    ];

    let mut rng = rand::rng();

    println!("{}", phrases.choose(&mut rng).unwrap());
    exit(0);
}
