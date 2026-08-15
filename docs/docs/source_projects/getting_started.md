---
title: "Getting Started"
sidebar_position: 2
---

# Getting Started with Eldiron Source

## Download

Open the [latest Eldiron release](https://github.com/markusmoenig/Eldiron/releases) and download the command-line tool for your platform.

For **Windows**, download:

- **eldiron-source-x86_64-pc-windows-msvc.zip**

For **Linux**, download:

- **eldiron-source-x86_64-unknown-linux-gnu.tar.gz**

Extract the archive and place `eldiron-source` (or `eldiron-source.exe`) somewhere on your command path. You can also invoke it directly from the extracted folder.

## Install via Cargo

If you have [Rust installed](https://www.rust-lang.org/tools/install), install the latest published version directly from [crates.io](https://crates.io/crates/eldiron-source):

```bash
cargo install eldiron-source
```

## Build from source

To build the tool from the Eldiron repository instead, run:

```bash
cargo build --release -p eldiron-source
```

The executable is written to `target/release/eldiron-source` on Linux and macOS, or `target/release/eldiron-source.exe` on Windows.

## Create a project

Create a scaffolded project folder:

```bash
eldiron-source new my-game
cd my-game
```

Use `--name` to choose a display name that differs from the folder name:

```bash
eldiron-source new my-game --name "My Game"
```

The command does not replace existing files. Scaffolding into a non-empty directory requires `--force`, and even then existing files are preserved.

## Build

From the project folder, run:

```bash
eldiron-source build
```

You can also pass the project folder from elsewhere:

```bash
eldiron-source build path/to/my-game
```

The default scaffold writes the compiled project to `build/game.eldiron`. The `[build]` section of `eldiron.toml` can change that path.

## Play

Build and start the configured client in one step:

```bash
eldiron-source play
```

The `[game] client_mode` setting selects a graphical or terminal client. When using standalone release binaries, keep the matching client executable beside `eldiron-source` or available through the Eldiron source checkout. You can always build first and start the desired [Eldiron client](../clients.md) with the generated `.eldiron` file yourself.

## Watch for changes

To rebuild when source files or assets change:

```bash
eldiron-source watch
```

The default debounce delay is 250 milliseconds. It can be changed when an editor writes several files in quick succession:

```bash
eldiron-source watch --debounce-ms 500
```

The watcher ignores generated `build/`, `dist/`, and `.git/` contents.

## Command summary

| Command | Purpose |
| --- | --- |
| `eldiron-source new <folder>` | Create a new Source project. |
| `eldiron-source build [folder]` | Compile the project into a `.eldiron` file. |
| `eldiron-source play [folder]` | Build and run the configured client. |
| `eldiron-source watch [folder]` | Build and continue rebuilding after changes. |
| `eldiron-source help <command>` | Show all options for a command. |
