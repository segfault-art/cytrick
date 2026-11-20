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

use crate::sql;
use home::home_dir;
use std::fs::remove_dir_all;
use symlink::remove_symlink_file;

pub fn remove_project(project_name: &str) -> Result<(), String> {
    let home = home_dir().ok_or(String::from("Could not find home directory"))?;

    let conn = match sql::create_connection() {
        Ok(conn) => conn,
        Err(err) => return Err(format!("Failed to create connection with database: {err}")),
    };

    let project = match sql::get_project(&conn, project_name) {
        Ok(project) => project,
        Err(err) => return Err(format!("Failed to get project from database: {err}")),
    };

    for binary in project.binaries {
        remove_symlink_file(home.join(".cytrick/bin").join(&binary))
            .map_err(|e| format!("Error removing symlink: {e}"))?;

        println!("Removed symlink: {binary}");
    }

    remove_dir_all(home.join(".cytrick").join(project_name))
        .map_err(|e| format!("Error removing project directory: {e}"))?;

    sql::delete_project(&conn, project_name)
        .map_err(|e| format!("Failed to delete project from database: {e}"))?;

    Ok(())
}
