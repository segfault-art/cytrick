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

use home::home_dir;
use rusqlite::{Connection, ErrorCode};
use serde::{Deserialize, Deserializer};
use std::process::exit;
use std::{
    env, fmt, fs, io,
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
};
use symlink::{remove_symlink_file, symlink_file};
use zip::ZipArchive;

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
        bin: bool,
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

pub fn deserialize(path: PathBuf) -> Result<Trick, Box<dyn std::error::Error>> {
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

pub fn execute_all(trick: Trick) -> Result<(), String> {
    let mut err: Option<String> = None;
    let mut i: usize = 1;
    let mut binaries: Vec<String> = Vec::new();
    let conn = Connection::open(
        home_dir()
            .ok_or(String::from("Cannot get home dir"))?
            .join(".cytrick/data/projects.db"),
    )
    .map_err(|e| format!("Cannot open data file: {e}, try run `cyt setup`"))?;

    let data = ExecutionData {
        version: &trick.version,
        name: &trick.name,
    };

    for command in trick.build {
        match execute(&command, &data) {
            Ok(status) => match status {
                Status::Installed { bin, name, .. } => {
                    if bin {
                        binaries.push(name);
                    }
                }

                _ => {}
            },

            Err(e) => {
                err = Some(format!(
                    "Error executing command ({command}): {e}. Command {i}"
                ));
                break;
            }
        }

        i += 1;
    }

    if let Err(err) = conn.execute(
        "INSERT INTO project (binaries, version, name, repo) VALUES (?, ?, ?, ?)",
        (
            binaries.join(","),
            &trick.version.to_string(),
            &trick.name,
            None::<&str>,
        ),
    ) {
        if let Some(ErrorCode::ConstraintViolation) = err.sqlite_error_code() {
            println!("Project already installed, overwriting");
            conn.execute(
                "UPDATE project SET
                    binaries = ?,
                    version = ?
                WHERE name = ?
                ",
                (binaries.join(","), &trick.version.to_string(), &trick.name),
            )
            .map_err(|e| format!("Failed to update project: {e}"))?;
        } else {
            return Err(format!("Failed to install project: {err}"));
        }
    }

    match err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

fn execute(command: &str, data: &ExecutionData) -> Result<Status, String> {
    let mut split = command.split_whitespace();

    if let Some(function) = split.next() {
        execute_function(function, &split.collect::<Vec<&str>>(), data)
    } else {
        Err(String::from("Empty"))
    }
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
    let mut reply = reqwest::blocking::get(url)
        .map_err(|e| format!("Cannot download file: {e}"))?
        .error_for_status()
        .map_err(|e| format!("URL returned bad status: {e}"))?;

    fs::create_dir_all(
        file.parent()
            .ok_or(String::from("Cannot access folder's parent"))?,
    )
    .map_err(|e| format!("Cannot create directory: {e}"))?;

    reply
        .copy_to(&mut fs::File::create(&file).map_err(|e| format!("Cannot create file: {e}"))?)
        .map_err(|e| format!("Cannot write in file: {e}"))?;

    Ok(Status::Downloaded {
        url: url.to_owned(),
        file,
    })
}

fn extract(args: &[&str]) -> Result<Status, String> {
    let source = PathBuf::from(*args.first().ok_or("No archive")?);
    let item = PathBuf::from(*args.get(1).ok_or("No destination path")?);

    let file = fs::File::open(&source).map_err(|e| format!("Cannot open archive: {e}"))?;
    let mut zip = ZipArchive::new(file).map_err(|e| format!("Invalid zip: {e}"))?;

    fs::create_dir_all(&item).map_err(|e| format!("Cannot create directory: {e}"))?;
    zip.extract(&item)
        .map_err(|e| format!("Cannot extract: {e}"))?;

    Ok(Status::Extracted { item, source })
}

pub fn install(args: &[&str], data: &ExecutionData) -> Result<Status, String> {
    let home = home_dir().ok_or(String::from("Cannot get the home folder"))?;
    let name = get_name(args[0], args.get(1).and_then(|v| Some(*v)))?;
    let source = args.first().ok_or(String::from("No source file"))?;
    let file = home.join(format!(".cytrick/{}/{}", data.name, name));

    fs::create_dir_all(home.join(format!(".cytrick/{}", data.name)))
        .map_err(|e| format!("Cannot create project directory: {e}"))?;
    fs::copy(&source, &file).map_err(|e| format!("Cannot copy file: {e}"))?;

    Ok(Status::Installed {
        bin: false,
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

pub fn install_bin(args: &[&str], data: &ExecutionData) -> Result<Status, String> {
    let home = home_dir().ok_or(String::from("Cannot get the home folder"))?;
    let name = get_name(args[1], args.get(2).and_then(|v| Some(*v)))?;
    let source = args.get(1).ok_or(String::from("No source file"))?;
    let file = home.join(format!(".cytrick/bin/{}", name));
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

    Ok(Status::Installed {
        bin: true,
        name,
        file,
        source: source.into(),
    })
}

fn execute_function(function: &str, args: &[&str], data: &ExecutionData) -> Result<Status, String> {
    match function {
        "run" => run(args),
        "shell" => shell(args),
        "download" => download(args),
        "extract" => extract(args),
        "install" if *args.first().ok_or(String::from("No source file"))? == "bin" => {
            install_bin(args, data)
        }

        "install" => install(args, data),
        _ => Err(format!("Unknown function: {function}")),
    }
}

pub fn update(folder: &str) -> ! {
    let old_cwd = env::current_dir().expect("Cannot get current directory");
    let updater = env::current_exe()
        .expect("Failed to get current executable")
        .parent()
        .unwrap()
        .join("updater");

    env::set_current_dir(folder).expect("Failed to set current directory");

    let _ = Command::new(updater)
        .args([old_cwd])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    exit(0);
}
