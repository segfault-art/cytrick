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

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::interpreter::{deserialize, execute_all, extract};

#[derive(Deserialize, Debug)]
struct Release {
    #[allow(dead_code)]
    tag_name: String,
    zipball_url: String,
}

fn download_project(project: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();
    let dest = PathBuf::from("tmp");
    let release = client
        .get(format!(
            "https://api.github.com/repos/{project}/releases/latest"
        ))
        .header("User-Agent", "Cytrick")
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| format!("Cannot reach github: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Github returned error: {e}"))?
        .json::<Release>()
        .map_err(|e| format!("Error deserializing release result: {e}"))?;

    fs::create_dir_all(&dest).map_err(|e| format!("Error creating tmp directory: {e}"))?;
    let mut file = fs::File::create(dest.join("project.zip"))
        .map_err(|e| format!("Error creating zip file: {e}"))?;
    let mut response = client
        .get(&release.zipball_url)
        .header("User-Agent", "Cytrick")
        .send()
        .map_err(|e| format!("Cannot download source: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Bad status: {e}"))?;

    response
        .copy_to(&mut file)
        .map_err(|e| format!("Cannot write file: {e}"))?;

    extract(&["tmp/project.zip", "tmp", "no-root"])?;

    Ok(())
}

pub fn install_github_project(project: &str, load: bool) -> Result<(), String> {
    download_project(project)?;
    let trick = deserialize(Path::new("tmp/.trick"))
        .map_err(|e| format!("Error deserializing trick file: {e}"))?;
    env::set_current_dir("tmp").map_err(|e| format!("Error setting current_dir: {e}"))?;
    execute_all(trick, Some(project), load, true)?;

    match env::current_dir() {
        Err(err) => {
            eprintln!("Error getting current dir: {err}. Skipping delete dir step.");

            Ok(())
        }

        Ok(current_dir) => {
            let parent = current_dir.parent().unwrap();

            if let Err(err) = env::set_current_dir(parent) {
                eprintln!("Error setting current dir: {err}. Skipping delete dir step.");

                Ok(())
            } else {
                if let Err(err) = fs::remove_dir_all("tmp") {
                    eprintln!("Error deleting tmp directory: {err}. Skipping delete dir step");

                    Ok(())
                } else {
                    Ok(())
                }
            }
        }
    }
}
