---
title: "Getting Started"
sidebar_position: 1
---

![Logo](/img/eldiron-banner.png)

## Downloading Eldiron

The first step is to download your copy of **Eldiron** from the GitHub repository [here](https://github.com/markusmoenig/Eldiron/releases). Expand the **Assets** folder of the latest release and download the binary for your operating system.

For **Windows** download

- **eldiron-creator-x86_64-pc-windows-msvc.zip** which is the raw executable 
- or **Eldiron-Creator.msi** which is installer based.

For **Linux** download

- **eldiron-client-x86_64-unknown-linux-gnu.tar.gz**
- or **Eldiron-Creator.deb**
- or for Arch Linux Users you can install the [AUR](https://aur.archlinux.org/packages/eldiron-bin)

For **macOS** download

- **eldiron_creator_macOS.zip**
- A macOS AppStore version is planned after we reach v1

### Install via Cargo

If you have [Rust installed](https://www.rust-lang.org/tools/install), you can install Eldiron Creator directly from [crates.io](https://crates.io):

```bash
cargo install eldiron-creator
```

### Building from Source

Clone the repository and build:

```bash
git clone https://github.com/markusmoenig/Eldiron
cd Eldiron
cargo run --release --package creator
```

### Linux Dependencies

Make sure these dependencies are installed: `libasound2-dev` `libatk1.0-dev` `libgtk-3-dev`

## First Steps

After installing **Eldiron Creator** read [Working with Geometry](/docs/working_with_geometry) and after that the *Building Maps* chapter, especially [Working with Tiles](/docs/building_maps/working_with_tiles) and  [2D or 3D Maps ?](/docs/building_maps/2d_or_3d).

Learning about **characters** and **clients** would be a good next step, start with [Getting Started](/docs/characters_items/getting_started).
