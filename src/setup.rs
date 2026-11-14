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

use std::fs;

use home::home_dir;

pub fn setup() -> Result<(), String> {
    let bin_dir = home_dir()
        .ok_or(String::from("Cannot get home directory"))?
        .join(".cytrick/bin");

    fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Cannot create Cytrick binaries directory: {e}"))?;
    let _ = onpath::add(bin_dir, "cytrick")
        .map_err(|e| format!("Cannot add Cytrick binaries directory to PATH: {e}"))?;

    Ok(())
}
