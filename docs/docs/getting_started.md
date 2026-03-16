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

## What Eldiron Is

**Eldiron** is a game creator built around one shared world model:

- maps made from sectors, linedefs, and entities
- reusable character and item templates
- scripts for behavior
- rules for shared gameplay math
- authoring data for descriptive text
- multiple presentation layers like 2D, 3D, and terminal/text

If you want the bigger picture first, read [Eldiron Architecture](/docs/architecture).

## First Launch

After starting **Eldiron Creator**, open one of the included example projects first.

That is the fastest way to understand how Eldiron is structured, because you can inspect:

- regions and geometry
- character and item templates
- scripts and attributes
- game settings, rules, locales, and authoring config

The examples are much more useful as a starting point than trying to create everything from an empty project immediately.

## Recommended First Session

This is a good first path through Eldiron:

1. Open an example project.
2. Look at the [Project Tree](/docs/creator/project_tree) to see how the game is organized.
3. Switch between 2D and 3D region views in the Creator.
4. Press **Play** and walk around.
5. Inspect one character template and one item template.
6. Open **Game / Rules**, **Game / Locales**, and **Game / Authoring** to see the game-wide systems.

That gives you a much better first impression than geometry alone, because Eldiron is not just a map editor.

## How to Learn Eldiron

The best learning order is:

### 1. Creator Basics

Start with:

- [Introduction](/docs/creator/introduction)
- [Project Tree](/docs/creator/project_tree)
- [Tools Overview](/docs/creator/tools/overview)

This shows how the editor is laid out and where different kinds of data live.

### 2. Maps and Geometry

Then learn how the world is built:

- [Working with Geometry](/docs/working_with_geometry)
- [Working with Tiles](/docs/building_maps/working_with_tiles)
- [2D or 3D Maps ?](/docs/building_maps/2d_or_3d)

### 3. Characters and Items

Next, learn how gameplay content is defined:

- [Characters and Items Getting Started](/docs/characters_items/getting_started)
- [Attributes](/docs/characters_items/attributes)
- [Player Input](/docs/characters_items/player_input)
- [Events](/docs/characters_items/events)

### 4. Shared Game Systems

After that, move to the game-wide systems:

- [Rules](/docs/rules)
- [Localization](/docs/localization)
- [Authoring Configuration](/docs/configuration/authoring)

This is where Eldiron starts to feel like a full game framework instead of just an editor.

## A Good Mindset

When starting with Eldiron, it helps to think in layers:

- maps define the world
- characters and items define behavior
- scripts decide what happens
- rules define the shared formulas
- authoring metadata defines descriptive text
- clients present the same game in different ways

That separation is one of Eldiron’s main strengths.
