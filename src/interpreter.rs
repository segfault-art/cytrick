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

use std::{
    env, fmt, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use serde::{Deserialize, Deserializer};
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

struct ExecutionData<'a> {
    #[allow(dead_code)]
    version: &'a Version,
    name: &'a str,
}

pub trait PathExt {
    fn create_and_join<P: AsRef<Path>>(&self, sub: P) -> io::Result<PathBuf>;
}

impl<T: AsRef<Path> + ?Sized> PathExt for T {
    fn create_and_join<P: AsRef<Path>>(&self, sub: P) -> io::Result<PathBuf> {
        let novo = self.as_ref().join(sub);
        fs::create_dir_all(&novo)?;
        Ok(novo)
    }
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
    let data = ExecutionData {
        version: &trick.version,
        name: &trick.name,
    };

    for command in trick.build {
        if let Err(message) = execute(command.as_ref(), &data) {
            err = Some(message);
            break;
        }
    }

    match err {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

fn execute(command: &str, data: &ExecutionData) -> Result<(), String> {
    let mut splitted = command.split_whitespace();

    if let Some(function) = splitted.next() {
        execute_function(function, splitted.collect::<Vec<&str>>().as_ref(), data)
    } else {
        Err(String::from("Empty"))
    }
}

fn execute_function(function: &str, args: &[&str], data: &ExecutionData) -> Result<(), String> {
    match function {
        "run" => Command::new(args.first().ok_or(String::from("No program"))?)
            .args(args.get(1..).unwrap_or(&[]))
            .status()
            .map_err(|e| format!("Cannot spawn process: {e}"))
            .and_then(|s| {
                if s.success() {
                    Ok(())
                } else {
                    Err(String::from(format!(
                        "Process returned error ({})",
                        s.code()
                            .ok_or(String::from("Process terminated by signal"))?
                    )))
                }
            }),

        "shell" => system(args.join(" ").as_ref())
            .map_err(|e| format!("Cannot execute command: {e}"))
            .and_then(|s| {
                if s.success() {
                    Ok(())
                } else {
                    Err(String::from(format!(
                        "Command returned error ({})",
                        s.code()
                            .ok_or(String::from("Process terminated by signal"))?
                    )))
                }
            }),

        "download" => Command::new("curl")
            .args(&[
                "-L",
                "--fail",
                "-O",
                args.first().ok_or(String::from("No URL"))?,
            ])
            .status()
            .map_err(|e| format!("Cannot download file: {e}"))
            .and_then(|s| {
                if s.success() {
                    Ok(())
                } else {
                    Err(String::from(format!(
                        "Curl returned error ({})",
                        s.code().ok_or(String::from("Curl terminated by signal"))?
                    )))
                }
            }),

        "extract" => ZipArchive::new(
            fs::File::open(args.first().ok_or(String::from("No file"))?)
                .map_err(|e| format!("Cannot open file: {e}"))?,
        )
        .map_err(|e| format!("Cannot open file as zip file: {e}"))?
        .extract(args.get(1).ok_or(String::from("No output directory"))?)
        .map_err(|e| format!("Cannot extract file: {e}")),

        "install" if *args.first().ok_or(String::from("No source file"))? == "bin" => fs::copy(
            args.get(1).ok_or(String::from("No binary file"))?,
            home::home_dir()
                .ok_or(String::from("Cannot get home folder"))?
                .join(".cytrick/bin")
                .join(
                    args.get(2).copied().unwrap_or(
                        PathBuf::from(args.get(1).unwrap())
                            .file_name()
                            .ok_or(String::from("Cannot get source file name"))?
                            .to_str()
                            .ok_or(String::from("Final name is not encoded with UTF-8"))?,
                    ),
                ),
        )
        .map_err(|e| format!("Cannot install binary: {e}"))
        .and_then(|_| Ok(())),

        "install" => fs::copy(
            args.first().ok_or(String::from("No source file/dir"))?,
            home::home_dir()
                .ok_or(String::from("Cannot get home folder"))?
                .create_and_join(format!(".cytrick/{}", data.name))
                .map_err(|e| format!("Cannot create project folder: {e}"))?
                .join(
                    PathBuf::from(args.first().unwrap())
                        .file_name()
                        .ok_or(String::from("Cannot get source file name"))?,
                ),
        )
        .map_err(|e| format!("Cannot install file/folder: {e}"))
        .and_then(|_| Ok(())),

        _ => Err(format!("Unknown function: {function}")),
    }
}
