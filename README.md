
# Cytrick

**Install GitHub projects without having to figure out how to build them.**

Cytrick is a tool designed to simplify project installation. The idea is to make installing a program as simple as downloading a repository, without having to go through a sequence of instructions, dependencies, and build commands.

Instead of requiring users to figure out how each project works, Cytrick uses `.trick` files, which describe the instructions needed to prepare and install a project.

## Goal

Imagine finding an interesting project on GitHub. You download the repository and discover that you need to install a compiler, configure the environment, install dependencies, run several commands, and finally figure out where to put the executable.

Cytrick exists to simplify this process.

The goal is to provide an experience like this:

```bash
cyt install project
```

A single command to handle the entire installation process.

## Current Status

Cytrick is still under development.

It currently has a `.trick` file interpreter capable of executing instructions locally.

Available operations include:

- Running external programs.
- Executing commands through the shell.
- Downloading files using `curl`.
- Extracting ZIP archives.
- Installing files and binaries into Cytrick directories.

Fetching projects from GitHub has not yet been implemented.

## The `.trick` File

The `.trick` file is responsible for describing a project and its build and installation instructions.

It uses YAML as its configuration format.

### Example

```yaml
version: 0.1.0
name: Cytrick

build:
  - cargo build
  - install target/debug/cytrick cyt
```

### Structure

| Field | Description |
| --- | --- |
| `name` | Project name. |
| `version` | Project version. |
| `build` | List of instructions executed by Cytrick. |

### Available Instructions

| Instruction | Description |
| --- | --- |
| `run` | Runs an external program. |
| `shell` | Executes a command through the system shell. |
| `download` | Downloads a file using `curl`. |
| `extract` | Extracts a ZIP archive. |
| `install` | Installs files or binaries. |

Instructions are executed in the order they appear. If any instruction fails, execution is stopped.

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

The destination will be `~/.cytrick/bin/`.

You can also specify a different name for the executable:

```yaml
build:
  - install bin program my-program
```

## Directories

Cytrick uses the user's home directory to store its files.

```text
~/.cytrick/
├── bin/
│   └── executables
└── project-name/
    └── installed files
```

## Requirements

- `curl`, to use the `download` instruction.
- A shell compatible with the operating system, to use `shell`.
- The build tools required by the project, when applicable.

## Developers

Cytrick is written in Rust.

The project is still under development, and contributions are welcome.

If you find a bug or have an idea to improve Cytrick, open an Issue or submit a Pull Request.
