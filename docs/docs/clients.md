---
title: "Clients"
sidebar_position: 7
---

# Clients

In [Getting Started](getting_started), we discussed how to download the binary files for **Eldiron Creator**.

Here we cover the **Eldiron Clients**, i.e. stand-alone apps that can play the game you created.

In the **Assets** folder of the latest GitHub release [here](https://github.com/markusmoenig/Eldiron/releases), you can download the following files.

For **Windows**, download

* **eldiron-client-x86_64-pc-windows-msvc.zip**

For **Linux**, download

* **eldiron-client-x86_64-unknown-linux-gnu.tar.gz**

For the **Web / WASM**, download

* **eldiron-client-wasm32-unknown-unknown.tar.gz**

For **macOS**, we currently do not have a pre-built binary.

### Install via Cargo

If you have [Rust installed](https://www.rust-lang.org/tools/install), you can install Eldiron Creator directly from [crates.io](https://crates.io):

```bash
cargo install eldiron-client
```

# Running your Game

On **Linux** and **Windows**, just start the client and pass the name of the **.eldiron** game file on the command line.

For the **Web**, rename your **.eldiron** file to **game.eldiron** and put it in the same directory as **index.html** and the other files. You can then run the game on any web server.

# Project Files

Creator and every Eldiron client use the same **.eldiron** project file. New files are compressed, versioned containers, so even projects with embedded pixel art remain practical to store and transfer. A project saved by Creator can be opened again in Creator or passed directly to a client; there is no separate client-only export.

Older **.eldiron** files that contain plain JSON remain supported. Opening and saving one in Creator upgrades it to the compressed container automatically. See [Project Files](creator/project_files.md) for the format and compatibility details.
