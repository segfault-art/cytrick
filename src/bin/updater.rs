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

use cytrick::interpreter::{ExecutionData, Version, install_bin};
use std::{
    env,
    env::{args, consts::EXE_EXTENSION},
    process::{Command, Stdio, exit},
    thread::sleep,
    time::Duration,
};

fn main() {
    let arg = args().skip(1).next().expect("Failed to get argument");

    let version = Version {
        major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
        patch: env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
    };

    let extension = format!(
        "{}{}",
        if EXE_EXTENSION.is_empty() { "" } else { "." },
        EXE_EXTENSION
    );

    sleep(Duration::from_secs(1));
    if let Ok(status) = Command::new("cargo")
        .args(["build", "--release"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        if !status.success() {
            eprintln!("Failed to build Cytrick binaries");
            exit(1);
        }
    } else {
        eprintln!("Failed to build Cytrick binaries");
        exit(1);
    }

    install_bin(
        &["bin", &format!("target/release/cytrick{extension}"), "cyt"],
        &ExecutionData {
            version: &version,
            name: "Cytrick",
        },
    )
    .expect("Failed to install binary");

    env::set_current_dir(arg).expect("Failed to set current directory");
    println!("Autoinstalled successfully");
}
