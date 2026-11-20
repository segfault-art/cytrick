# Cytrick

**Install GitHub projects without having to figure out how to build them.**

Cytrick is a tool designed to simplify project installation. The idea is to make installing a program as simple as
downloading a repository, without having to go through a sequence of instructions, dependencies, and build commands.

Instead of requiring users to figure out how each project works, Cytrick uses `.trick` files, which describe the
instructions needed to prepare and install a project.

## Goal

Imagine finding an interesting project on GitHub. You find a repository, and normally you would have to install a
compiler, configure the environment, install dependencies, run several commands, and finally figure out where to put the
executable.

Cytrick exists to simplify this process with a single command:

```bash
cyt install user/project
```

## Current Status

Cytrick is under active development and already supports the full lifecycle of installing, updating, and removing
projects.

**Implemented features:**

- **GitHub Integration:** Fetching and installing the latest release of a repository (`cyt install user/repo`).
- **Project Removal:** Uninstalling projects, cleaning up their directories, symlinks, and database entries
  (`cyt remove <project>`).
- **Self-Updating:** Updating Cytrick either from a local source folder (`cyt update`) or directly from GitHub
  (`cyt update --web`).
- **OS-Specific Configurations:** Automatic detection of `.trick.unix` and `.trick.win` files, falling back to `.trick`.
- **Cross-Platform Modifiers:** Conditional argument handling via `%{bin}%`, `%{win}%`, and `%{unix}%`.
- **Local Interpreter & Execution:**
    - Running external programs directly (`run`).
    - Executing commands through the system shell (`shell`).
    - Downloading files over HTTP(S) (`download`).
    - Extracting ZIP archives with optional root-folder unwrapping (`extract ... no-root`).
    - Installing files and creating `PATH` symlinks for binaries (`install` / `install bin`).
- **Local Tracking:** Keeping track of installed projects, versions, binaries, and repositories in a local SQLite
  database. **Project Inspection & Tracking:** Querying installed project details (`cyt query <project>`) and tracking
  versions, binaries, and repositories in a local SQLite database.
- **Listing Projects:** Listing installed projects via a CLI command (e.g., `cyt list`).

**Planned / Not implemented yet:**

- Extracting archive formats other than ZIP (such as `.tar.gz` or `.tar.xz`).
- Installing specific versions, tags, or branches from GitHub (currently fetches `releases/latest`).

## Getting Started

Build Cytrick from source and let it install itself:

```bash
git clone https://github.com/segfault-art/cytrick.git
cd cytrick
cargo build --release

target/release/cytrick setup   # creates ~/.cytrick, the project database, and updates your PATH
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

| Command                                        | Description                                                                                                              |
|------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|
| `cyt setup`                                    | Creates the `~/.cytrick` directories and the project database, and adds `~/.cytrick/bin` to your `PATH`. Run it once.    |
| `cyt install <user/repo>`                      | Downloads the latest GitHub release for the repository and executes its `.trick` file.                                   |
| `cyt remove <project>`                         | Removes an installed project, its symlinks in `~/.cytrick/bin`, its files, and its database record.                      |
| `cyt run [--file <path>] [--repo <user/repo>]` | Runs a `.trick` file locally. Defaults to `.trick.unix`/`.trick.win`, falling back to `.trick` in the current directory. |
| `cyt update [--folder <path> \| --web]`        | Updates Cytrick from a local source folder (default `.`) or fetches the latest release directly from GitHub (`--web`).   |
| `cyt version`                                  | Shows the installed version of Cytrick.                                                                                  |
| `cyt query`                                    | Displays details of an installed project (`name`, `version`, `binaries`, and `repo`).                                    |
| `cyt list`                                     | Displays all installed projects.                                                                                         |

### Updating Cytrick

You can update Cytrick either from a local clone of the repository or directly from GitHub releases:

```bash
# Update from the latest GitHub release
cyt update --web

# Or update from a local repository folder
git pull
cyt update                           # uses the current folder
cyt update --folder ~/dev/cytrick    # or specify the Cytrick source folder
```

The update is handled by a separate `updater` binary installed alongside `cyt`. When you run `cyt update`, `cyt` spawns
the `updater` in the background and exits immediately so the binary can be safely overwritten. Because it runs in the
background, the terminal prompt may return before the build/download finishes.

## The `.trick` File

The `.trick` file describes a project and its build and installation instructions using YAML (parsed via `noyalib`).

When running `cyt run` without `--file`, Cytrick automatically looks for OS-specific files first:

- On Unix-like systems (Linux, macOS): `.trick.unix` $\rightarrow$ `.trick`
- On Windows: `.trick.win` $\rightarrow$ `.trick`

### Example

This is an example of a `.trick` file used to build and install Cytrick:

```yaml
version: 0.1.1
name: Cytrick

build:
  - run cargo build --release
  - install bin %{bin}% target/release/cytrick cyt
  - install %{bin}% target/release/updater
```

### Structure

| Field     | Description                                                               |
|-----------|---------------------------------------------------------------------------|
| `name`    | Project name (used as the directory name inside `~/.cytrick/`).           |
| `version` | Project version, in the format `major.minor.patch` (for example `0.1.1`). |
| `build`   | List of instructions executed sequentially by Cytrick.                    |

### Available Instructions

| Instruction   | Syntax                                          | Description                                                                                             |
|---------------|-------------------------------------------------|---------------------------------------------------------------------------------------------------------|
| `run`         | `run <program> [args...]`                       | Runs an external program directly, without a shell.                                                     |
| `shell`       | `shell <command>`                               | Executes a command through the system shell (pipes, redirects, and shell variables work here).          |
| `download`    | `download <url> <destination>`                  | Downloads a file over HTTP(S). Missing parent directories are created automatically.                    |
| `extract`     | `extract <archive.zip> <destination> [no-root]` | Extracts a ZIP archive into a folder. Passing `no-root` unwraps the top-level directory inside the zip. |
| `install`     | `install <file> [new-name]`                     | Copies a file into `~/.cytrick/<project-name>/`.                                                        |
| `install bin` | `install bin <file> [new-name]`                 | Copies an executable to the project folder and creates a symlink in `~/.cytrick/bin/`.                  |

Instructions are executed in the order they appear. If any instruction fails, execution stops immediately and reports
which command failed. Relative paths are resolved from the folder where Cytrick is executed.

### Modifiers

Cytrick supports inline modifiers in instruction arguments:

- `%{bin}% <path>`: Automatically appends the platform's executable suffix to the following argument (`.exe` on Windows,
  empty on Unix). For example, `%{bin}% target/release/app` becomes `target/release/app.exe` on Windows and
  `target/release/app` on Linux/macOS.
- `%{win}% <arg>`: Includes `<arg>` only on Windows; omits it (along with `<arg>`) on Unix-like systems.
- `%{unix}% <arg>`: Includes `<arg>` only on Unix-like systems; omits it (along with `<arg>`) on Windows.

### Installing Files & Binaries

To copy a regular asset or file into `~/.cytrick/<project-name>/`:

```yaml
build:
  - install config.toml
```

To install an executable and expose it globally via `~/.cytrick/bin/`:

```yaml
build:
  - install bin program
  - install bin program custom-alias
```

The file is copied to `~/.cytrick/<project-name>/` and a symbolic link is created in `~/.cytrick/bin/`. Because the
symlink points to the installed copy, the program continues working even if the original build directory is deleted.
If a symlink with the same name already exists, it is replaced.

### Downloading and Extracting

```yaml
build:
  - download https://example.com/tool.zip downloads/tool.zip
  - extract downloads/tool.zip tool no-root
  - install bin %{bin}% tool/program
```

Using `no-root` is especially useful for GitHub source archives, which usually wrap all files inside a single
`repo-tag/` root directory. Archive entries are extracted safely, rejecting path-traversal attempts (such as
`../../file`).

## Directories

Cytrick uses the user's home directory to store its files:

```text
~/.cytrick/
├── bin/
│   └── symlinks to installed executables
├── data/
│   └── projects.db
└── <project-name>/
    └── installed files and binaries
```

`projects.db` is a SQLite database that records each installed project (`name`, `version`, `binaries`, and `repo`).
Re-installing an already registered project updates its database entry.

## Requirements

- A recent Rust toolchain, to build Cytrick from source or run local updates (`cyt update`).
- A shell compatible with the operating system when using `shell` (`$SHELL` or `/bin/sh` on Unix, `cmd.exe` on Windows).
- Any build tools or compilers required by the specific projects you choose to install.

SQLite is bundled, and HTTP downloads/ZIP extraction are handled natively, so external tools like `curl` or `unzip` are
not required.

## Limitations

- Instruction arguments are split on whitespace, so file paths or arguments containing spaces are not currently
  supported in `run`, `download`, `extract`, and `install`.
- `cyt update` runs silently in the background (`stdin`/`stdout`/`stderr` redirected to null), so build or download
  errors are not displayed in the terminal.
- `cyt install <user/repo>` currently requires the repository to have a published GitHub Release (`releases/latest`)
  containing a `.trick` file at its root.
- On Windows, creating symbolic links requires Administrator privileges or Developer Mode enabled.
- Only `.zip` archives are supported by the `extract` instruction.

## Developers

Cytrick is written in Rust.

The project is still under active development, and contributions are welcome. If you find a bug or have an idea to
improve Cytrick, feel free to open an Issue or submit a Pull Request.

## License

Licensed under either of the following licenses, at your option:

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](https://opensource.org/license/mit)