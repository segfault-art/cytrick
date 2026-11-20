<!--suppress SpellCheckingInspection -->

# Cytrick

**Install GitHub projects without having to figure out how to build them.**

Cytrick is a tool designed to simplify project installation. The idea is to make installing a program as simple as
downloading a repository, without having to go through a sequence of instructions, dependencies, and build commands.

Instead of requiring users to figure out how each project works, Cytrick uses `.trick` files, which describe the
instructions needed to prepare and install a project.

## Goal

Imagine finding an interesting project on GitHub. You download the repository and discover that you need to install a
compiler, configure the environment, install dependencies, run several commands, and finally figure out where to put the
executable.

Cytrick exists to simplify this process.

The goal is to provide an experience like this:

```bash
cyt install user/project
```

A single command to handle the entire installation process.

## Current Status

Cytrick is still under development.

It currently has a `.trick` file interpreter capable of executing instructions locally, and it can install and update
itself.

Available operations include:

- Running external programs.
- Executing commands through the shell.
- Downloading files over HTTP(S).
- Extracting ZIP archives.
- Installing files and binaries into Cytrick directories.
- Keeping track of installed projects in a local database.
- Updating Cytrick itself from its source code.
- Fetching projects from GitHub (`cyt install project`).

  Not implemented yet:

- Fetching projects from GitHub (`cyt install project`).
- Extracting archives other than ZIP (such as `.tar.gz`).
- Uninstalling projects.

## Getting Started

Build Cytrick from source and let it install itself:

```bash
git clone <repository-url>
cd cytrick
cargo build --release

target/release/cytrick setup   # creates ~/.cytrick, the project database and updates your PATH
target/release/cytrick run     # runs the .trick file in the current directory
```

`setup` adds `~/.cytrick/bin` to your `PATH`. Open a new terminal afterward so the change takes effect. If `cyt` is
still not found, add the folder manually:

```bash
export PATH="$HOME/.cytrick/bin:$PATH"
```

Check that it works:

```bash
cyt version
```

### Commands

| Command                        | Description                                                                                                           |
|--------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| `cyt setup`                    | Creates the `~/.cytrick` directories and the project database, and adds `~/.cytrick/bin` to your `PATH`. Run it once. |
| `cyt run [--file <path>]`      | Runs a `.trick` file. Defaults to `.trick` in the current directory.                                                  |
| `cyt update [--folder <path>]` | Rebuilds Cytrick from its source folder and replaces the installed binary. Defaults to `.`; no `.trick` file needed.  |
| `cyt version`                  | Shows the installed version.                                                                                          |
| `cyt install <project>`        | Install the project from Github.                                                                                      |

### Updating Cytrick

`cyt update` rebuilds Cytrick from its source code and replaces the installed binary. Run it from the Cytrick
repository,
or point it to the folder:

```bash
git pull
cyt update                           # uses the current folder
cyt update --folder ~/dev/cytrick    # or the folder of the Cytrick project
```

The update is done by a separate `updater` program that must be installed next to `cyt` (the `.trick` file of Cytrick
itself takes care of that, see below). `cyt` starts it and exits right away. The updater then runs
`cargo build --release` in the project folder and reinstalls the `cyt` binary, so the terminal prompt may return before
the update finishes.

## The `.trick` File

The `.trick` file is responsible for describing a project and its build and installation instructions.

It uses YAML as its configuration format.

### Example

This is the `.trick` file used by Cytrick itself:

```yaml
version: 0.1.1
name: Cytrick

build:
  - run cargo build --release
  - install bin target/release/cytrick cyt
  - install target/release/updater
```

### Structure

| Field     | Description                                                               |
|-----------|---------------------------------------------------------------------------|
| `name`    | Project name.                                                             |
| `version` | Project version, in the format `major.minor.patch` (for example `0.1.1`). |
| `build`   | List of instructions executed by Cytrick.                                 |

### Available Instructions

| Instruction   | Syntax                                | Description                                                                         |
|---------------|---------------------------------------|-------------------------------------------------------------------------------------|
| `run`         | `run <program> [args...]`             | Runs an external program directly, without a shell.                                 |
| `shell`       | `shell <command>`                     | Executes a command through the system shell (pipes, redirects and so on work here). |
| `download`    | `download <url> <destination>`        | Downloads a file. Missing folders in the destination are created.                   |
| `extract`     | `extract <archive.zip> <destination>` | Extracts a ZIP archive into a folder, creating it if needed.                        |
| `install`     | `install <file> [new-name]`           | Copies a file into the project's folder.                                            |
| `install bin` | `install bin <file> [new-name]`       | Installs an executable and makes it available in your `PATH`.                       |

Instructions are executed in the order they appear. If any instruction fails, execution is stopped and the error shows
which command failed.

Relative paths are resolved from the folder where Cytrick is run.

### Installing Files

Files can be installed into the project's directory:

```yaml
build:
  - install file.txt
```

The destination will be `~/.cytrick/<project-name>/`.

To install an executable:

```yaml
build:
  - install bin program
```

The file is copied to `~/.cytrick/<project-name>/` and a symbolic link to that copy is created in `~/.cytrick/bin/`.
Because the link points to the installed copy, the program keeps working even if the original build folder is deleted.

You can also specify a different name for the executable:

```yaml
build:
  - install bin program my-program
```

If a link with the same name already exists, it is replaced.

### Downloading and Extracting

```yaml
build:
  - download https://example.com/tool.zip downloads/tool.zip
  - extract downloads/tool.zip tool
  - install bin tool/program
```

Archive entries are extracted safely: paths that try to escape the destination folder (such as `../../file`) are
rejected.

## Directories

Cytrick uses the user's home directory to store its files.

```text
~/.cytrick/
├── bin/
│   └── links to installed executables
├── data/
│   └── projects.db
└── project-name/
    └── installed files
```

`projects.db` is a SQLite database that records each installed project (name, version and installed binaries).
Installing a project that is already registered updates its entry.

## Requirements

- A recent Rust toolchain, to build Cytrick and to update it with `cyt update`.
- A shell compatible with the operating system, to use `shell` (`$SHELL` or `/bin/sh` on Unix, `cmd.exe` on Windows).
- The build tools required by the projects you install, when applicable.

SQLite is bundled, and downloads are handled internally, so no extra tools such as `curl` are needed.

## Limitations

- Each instruction is split on whitespace, so paths and arguments containing spaces are not supported in `run`,
  `download`, `extract` and `install`.
- `cyt update` runs silently in the background, so build errors are not shown. If the installed binary does not change,
  run `cargo build --release` in the project folder to see what went wrong.
- On Windows, creating symbolic links requires administrator privileges or Developer Mode.
- Only ZIP archives can be extracted for now.

## Developers

Cytrick is written in Rust.

The project is still under development, and contributions are welcome.

If you find a bug or have an idea to improve Cytrick, open an Issue or submit a Pull Request.

## License

Licensed under either of the following licenses, at your option:

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](https://opensource.org/license/mit)