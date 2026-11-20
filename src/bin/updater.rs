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

use cytrick::github::install_github_project;
use cytrick::interpreter::{deserialize, execute_all};

use std::env::args;
use std::path::PathBuf;

fn main() {
    let dir = args().nth(1).expect("Failed to get argument");
    let github = args().nth(2).expect("Failed to get argument");

    if github == "yes" {
        update_from_github();
    } else {
        update(&dir);
    }
}

fn update(dir: &str) {
    let trick =
        deserialize(PathBuf::from(dir).join(".trick")).expect("Failed to deserialize trick");
    execute_all(trick, Some("segfault-art/Cytrick")).expect("Failed to execute trick");
}

fn update_from_github() {
    install_github_project("segfault-art/cytrick").expect("Failed to install Cytrick");
}
