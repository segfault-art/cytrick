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

use crate::interpreter::Version;
use home::home_dir;
use rusqlite::Connection;
use std::collections::HashMap;

pub struct Project {
    pub binaries: Vec<String>,
    pub version: Version,
    pub name: String,
    pub repo: Option<String>,
}

pub struct ProjectRef<'a> {
    pub binaries: &'a [&'a str],
    pub version: &'a Version,
    pub name: &'a str,
    pub repo: Option<&'a str>,
}

pub fn create_connection() -> Result<Connection, String> {
    let home = home_dir().ok_or(String::from("Home directory not found"))?;
    let conn = Connection::open(home.join(".cytrick/data/projects.db"))
        .map_err(|e| format!("Error opening database: {e}"))?;

    Ok(conn)
}

pub fn new_project(conn: &Connection, project: &ProjectRef) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO project (binaries, version, name, repo) VALUES (?, ?, ?, ?)",
        (
            project.binaries.join(","),
            project.version.to_string(),
            project.name,
            project.repo,
        ),
    )
    .map(|_| ())
}

pub fn update_project(
    conn: &Connection,
    project_name: &str,
    update: &HashMap<&str, Option<&str>>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE project SET
            binaries = COALESCE(?, binaries),
            version = COALESCE(?, version),
            name = COALESCE(?, name),
            repo = COALESCE(?, repo)
        WHERE name = ?",
        (
            update.get("binaries"),
            update.get("version"),
            update.get("name"),
            update.get("repo"),
            project_name,
        ),
    )
    .map(|_| ())
}

pub fn delete_project(conn: &Connection, project_name: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM project WHERE name = ?", (project_name,))
        .map(|_| ())
}

pub fn get_project(conn: &Connection, project_name: &str) -> rusqlite::Result<Project> {
    conn.query_row(
        "SELECT * FROM project WHERE name = ?",
        (project_name,),
        |row| {
            Ok(Project {
                binaries: row
                    .get::<usize, String>(1)?
                    .split(",")
                    .map(str::to_owned)
                    .collect::<Vec<String>>(),

                version: {
                    let value = row.get::<usize, String>(2)?;

                    let mut split = value.splitn(3, '.').map(str::to_string);

                    let mut get_value =
                        || split.next().unwrap_or(String::from("0")).parse().unwrap();

                    Version {
                        major: get_value(),
                        minor: get_value(),
                        patch: get_value(),
                    }
                },

                name: row.get(3)?,
                repo: row.get(4)?,
            })
        },
    )
}

pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM project")?;
    stmt.query_map([], |row| row.get(0))?.collect()
}
