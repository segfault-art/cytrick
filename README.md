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

Cytrick is under active development and already supports the full lifecycle of installing, upgrading, extending, and
removing projects.

**Implemented features:**

* **GitHub Integration:** Fetching and installing the latest release of a repository (`cyt install user/repo`).
* **Project Removal:** Uninstalling projects, cleaning up their directories, symlinks, and database entries
  (`cyt remove <project>`).
* **Self-Upgrading:** Upgrading Cytrick either from a local source folder (`cyt upgrade [dir]`) or directly from GitHub
  (`cyt upgrade --web`).
* **Native Extension System:** Loading dynamic libraries (`.so`, `.dylib`, `.dll`) from `~/.cytrick/ext/` via FFI to
  register custom argument modifiers.
* **OS-Specific Configurations:** Automatic detection of `.trick.unix` and `.trick.win` files, falling back to `.trick`.
* **Cross-Platform & Custom Modifiers:** Conditional argument handling via `%{bin}%`, `%{dll}%`, `%{win}%`, and `%{unix}%`, plus
  custom modifiers provided by installed extensions.
* **Local Interpreter & Execution:**
  * Running external programs directly (`run`).
  * Executing commands through the system shell (`shell`).
  * Downloading files over HTTP(S) (`download`).
  * Extracting ZIP archives with optional root-folder unwrapping (`extract ... no-root`).
  * Installing regular files (`install`), `PATH` binaries (`install bin`), and native extensions (`install ext`).
* **Project Inspection & Tracking:** Querying installed project details (`cyt query <project>`), listing installed
  projects (`cyt list`), and tracking versions, binaries, extensions, and repositories in a local SQLite database.

**Planned / Not implemented yet:**

* Extracting archive formats other than ZIP (such as `.tar.gz` or `.tar.xz`).
* Installing specific versions, tags, or branches from GitHub (currently fetches `releases/latest`).

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

| Command | Description | 
| ----- | ----- | 
| `cyt setup` | Creates the `~/.cytrick` directories (`bin`, `data`, `ext`) and project database, and adds `~/.cytrick/bin` to your `PATH`. Run it once. | 
| `cyt install <user/repo> [--no-load]` | Downloads the latest GitHub release for the repository and executes its `.trick` file. | 
| `cyt remove <project>` | Removes an installed project, its symlinks in `~/.cytrick/bin` and `~/.cytrick/ext`, its files, and its database record. | 
| `cyt run [options]` | Runs a `.trick` file locally. Supports `--file <path>`, `--folder <path>`, `--repo <user/repo>`, `--no-load`, and `--nothing` (Unix). | 
| `cyt upgrade [dir] [--web]` | Upgrades Cytrick from a local source directory (default `.`) or fetches the latest release directly from GitHub (`--web`). | 
| `cyt query <project>` | Displays details of an installed project (`binaries`, `version`, `name`, and `repo`). | 
| `cyt list` | Displays all installed projects. | 
| `cyt version` | Shows the installed version of Cytrick. | 

### Upgrading Cytrick

You can upgrade Cytrick either from a local clone of the repository or directly from GitHub releases:

```bash
# Upgrade from the latest GitHub release
cyt upgrade --web

# Or upgrade from a local repository folder
git pull
cyt upgrade                  # uses the current folder (.)
cyt upgrade ~/dev/cytrick    # or specify the Cytrick source folder
```

During an upgrade, Cytrick renames its running executable to a temporary file (`cyt.tmp`), builds or downloads the new
version, and spawns an internal cleanup step (`after-update`) to remove the temporary file.

## The `.trick` File

The `.trick` file describes a project and its build and installation instructions using YAML (parsed via `noyalib`).

When running `cyt run` without `--file`, Cytrick automatically looks for OS-specific files first:

* On Unix-like systems (Linux, macOS): `.trick.unix` $\rightarrow$ `.trick`
* On Windows: `.trick.win` $\rightarrow$ `.trick`

### Example

This is an example of a `.trick` file used to build and install Cytrick:

```yaml
version: 0.1.1
name: Cytrick

build:
  - run cargo build --release
  - install bin %{bin}% target/release/cytrick cyt
```

### Structure

| Field | Description | 
| ----- | ----- | 
| `name` | Project name (used as the directory name inside `~/.cytrick/`). | 
| `version` | Project version, in the format `major.minor.patch` (for example `0.1.1`). | 
| `build` | List of instructions executed sequentially by Cytrick. | 

### Available Instructions

| Instruction | Syntax | Description | 
| ----- | ----- | ----- | 
| `run` | `run <program> [args...]` | Runs an external program directly, without a shell. | 
| `shell` | `shell <command>` | Executes a command through the system shell (pipes, redirects, and shell variables work here). | 
| `download` | `download <url> <destination>` | Downloads a file over HTTP(S). Missing parent directories are created automatically. | 
| `extract` | `extract <archive.zip> <destination> [no-root]` | Extracts a ZIP archive into a folder. Passing `no-root` unwraps the top-level directory inside the zip. | 
| `install` | `install <file> [new-name]` | Copies a file into `~/.cytrick/<project-name>/`. | 
| `install bin` | `install bin <file> [new-name]` | Copies an executable to the project folder and creates a symlink in `~/.cytrick/bin/`. | 
| `install ext` | `install ext <file> [new-name]` | Copies a dynamic library extension to the project folder and creates a symlink in `~/.cytrick/ext/`. | 

Instructions are executed in the order they appear. If any instruction fails, execution stops immediately, rolls back
any files, binaries, and extensions already installed during that run, and reports which command failed. Relative paths
are resolved from the folder where Cytrick is executed.

### Modifiers

Cytrick supports inline modifiers in instruction arguments using the `%{modifier}%` syntax:

* `%{bin}% <path>`: Automatically appends the platform's executable suffix to the following argument (`.exe` on Windows,
  empty on Unix). For example, `%{bin}% target/release/app` becomes `target/release/app.exe` on Windows and
  `target/release/app` on Linux/macOS.
* `%{dll}% <path>`: Automatically adds the platform's shared library prefix and suffix to the library name (for example,
  `%{dll}% target/release/my_ext` becomes `target/release/libmy_ext.so` on Linux, `target/release/libmy_ext.dylib` on
  macOS, and `target/release/my_ext.dll` on Windows).
* `%{win}% <arg>`: Includes `<arg>` only on Windows; omits it (along with `<arg>`) on Unix-like systems.
* `%{unix}% <arg>`: Includes `<arg>` only on Unix-like systems; omits it (along with `<arg>`) on Windows.
* **Custom Modifiers:** Extensions loaded from `~/.cytrick/ext/` can register additional `%{...}%` modifiers. Use
  `--no-load` on `cyt run` or `cyt install` to skip loading extensions.

### Installing Files, Binaries & Extensions

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

To install a native Cytrick extension and expose it in `~/.cytrick/ext/`:

```yaml
build:
  - install ext %{dll}% target/release/my_ext
```

In both `install bin` and `install ext`, the file is copied to `~/.cytrick/<project-name>/` and a symbolic link is
created in `~/.cytrick/bin/` or `~/.cytrick/ext/`. Because the symlink points to the installed copy, it continues
working even if the original build directory is deleted. If a symlink with the same name already exists, it is replaced.

### Extensions

Cytrick supports native extensions written in Rust (or any C-ABI compatible language using `safer-ffi`). Before running
a `.trick` file (unless `--no-load` is passed), Cytrick loads all libraries found in `~/.cytrick/ext/` and calls their
`init` symbol, passing a registration callback (`add_argument_modifier`) so extensions can define custom `%{name}%`
argument modifiers.

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
├── ext/
│   └── symlinks to installed native extensions
└── <project-name>/
    └── installed files, binaries, and extensions
```

`projects.db` is a SQLite database that records each installed project (`name`, `version`, `binaries`, `extensions`, and
`repo`). Re-installing an already registered project replaces its previous files/binaries and updates its database
entry.

## Requirements

* A recent Rust toolchain, to build Cytrick from source or run local upgrades (`cyt upgrade`).
* A shell compatible with the operating system when using `shell` (`$SHELL` or `/bin/sh` on Unix, `cmd.exe` on Windows).
* Any build tools or compilers required by the specific projects you choose to install.

SQLite is bundled, and HTTP downloads/ZIP extraction are handled natively, so external tools like `curl` or `unzip` are
not required.

## Limitations

* Instruction arguments are split on whitespace, so file paths or arguments containing spaces are not currently
  supported in `run`, `download`, `extract`, and `install`.
* `cyt install <user/repo>` currently requires the repository to have a published GitHub Release (`releases/latest`)
  containing a `.trick` file at its root.
* On Windows, creating symbolic links requires Administrator privileges or Developer Mode enabled.
* Only `.zip` archives are supported by the `extract` instruction.

## Developers

Cytrick is written in Rust.

The project is still under active development, and contributions are welcome. If you find a bug or have an idea to
improve Cytrick, feel free to open an Issue or submit a Pull Request.

## License

Licensed under either of the following licenses, at your option:

* [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
* [MIT License](https://opensource.org/license/mit)