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

use std::fs;

use home::home_dir;
use rusqlite::Connection;

pub fn setup() -> Result<(), String> {
    let base_dir = home_dir()
        .ok_or(String::from("Cannot get home directory"))?
        .join(".cytrick");

    let bin_dir = base_dir.join("bin");
    let data_dir = base_dir.join("data");
    let ext_dir = base_dir.join("ext");

    fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Cannot create Cytrick binaries directory: {e}"))?;
    let _ = onpath::add(bin_dir, "cytrick")
        .map_err(|e| format!("Cannot add Cytrick binaries directory to PATH: {e}"))?;

    fs::create_dir_all(&data_dir).map_err(|e| format!("Cannot create data directory: {e}"))?;
    fs::create_dir_all(&ext_dir).map_err(|e| format!("Cannot create extension directory: {e}"))?;

    let projects = Connection::open(data_dir.join("projects.db"))
        .map_err(|e| format!("Cannot open data file: {e}"))?;

    projects
        .execute(
            "CREATE TABLE IF NOT EXISTS project (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            binaries TEXT NOT NULL,
            extensions TEXT NOT NULL,
            version TEXT NOT NULL,
            name TEXT NOT NULL UNIQUE,
            repo TEXT UNIQUE
        )",
            [],
        )
        .map_err(|e| format!("Cannot create project table: {e}"))?;

    println!("Setup executed successfully");
    Ok(())
}
