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

For local play on **Linux** or **Windows**, start the client and pass the
authored **.eldiron** file on the command line.

For local play in the **browser**, put the authored project beside the
browser Player’s **index.html** as **game.eldiron**. For an online multiplayer
release, follow [Public Client Export](creator/public_client_export).

# Project Files

Creator saves editable projects as versioned **.eldiron** containers. Clients
can open the same document for local play. A public multiplayer Player opens
the separate **.eldiron-client** release and connects to the host that runs
the authored project.

Older **.eldiron** files that contain plain JSON remain supported. Opening and saving one in Creator upgrades it to the compressed container automatically. See [Project Files](creator/project_files.md) for the format and compatibility details.

Projects also carry their editable MZ material/appearance assets and baked render
data. Graphical clients consume the same palette bindings, item states, clothing
and animation data as Creator: procedural characters use sprites in 2D and posed
meshes in 3D. The terminal client continues to use gameplay and descriptive data
rather than requiring graphical artwork. A standalone MaterialZ checkout is not
a runtime dependency.

Material recipes are baked textures/pages rather than a live per-pixel graph VM.
Keep recipes and linked images with an editable project. See
[Appearance](characters_items/appearance) and [Procedural Materials](building_maps/procedural_materials).
