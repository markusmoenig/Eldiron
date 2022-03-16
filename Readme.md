[![forthebadge made-with-rust](http://ForTheBadge.com/images/badges/made-with-rust.svg)](https://www.rust-lang.org/)

# Eldiron - Classic RPG Creation

Eldiron is a creator for classic RPGs written in Rust. Eldiron v1 will be able to create games similar to Ultima 4 and 5 but with a modern twist and features.

[![MIT License](https://img.shields.io/apm/l/atomic-design-ui.svg?)](https://github.com/tterb/atomic-design-ui/blob/master/LICENSEs) [![version](https://img.shields.io/badge/version-0.1-red.svg)](https://shields.io/) [![macOS](https://svgshare.com/i/ZjP.svg)](https://svgshare.com/i/ZjP.svg) [![Windows](https://svgshare.com/i/ZhY.svg)](https://svgshare.com/i/ZhY.svg) [![Linux](https://svgshare.com/i/Zhy.svg)](https://svgshare.com/i/Zhy.svg)

The game creator (which contains the game engine) is cross platform and should run on all Desktops. The game engine will work on Desktops but also on iOS and Android devices and any other device Rust compiles on. It is designed from the ground up to be extremely portable.

Eldiron comes with a range of freely usable tilemaps for environment and characters, however you can of course use your own tilemaps, see the instructions below.

The game engine contains a client and server modules, although currently no multi-player options exist yet, the code has been written with multi-player support in mind from the ground up.

Join the community on [Discord](https://discord.gg/ybfTnqy8).

## Design Goals

* Run everywhere.
* Keep the engine design flexible, so even if I target Ultima 4 / 5 games (or any game with square tiles) for v1, it will be possible to add support for other perspectives and game types after v1.
* Develop the engine design with multi-player support in mind from the ground up (instances or MMORPGs etc).
* Provide a set of game assets so that users can instantly start their own games.
* Develop a node based Behavior AI system which is easy to use but powerful enough to handle every possible aspect of an RPG (combat, crafting, farming, exploration etc.).

## Why ?

I played the Ultima games day and night in my youth so I decided to create my own similar games as I have some time to spare (what could be a better hobby ?), and on the way also develop a game creator for this type of game which I need for my own game anyway.

## Features and Status

For v1 I target these features:

* Tilemap editor to select tiles in bitmaps and assign tile types and animations. (Done)
* Commission game tiles and create a modular character tile system (In progress).
* World editor to create the world and it's areas (In progress).
* Behavior tree node system for creating the RPG system and AI (In progress).
* 3D Dungeons ? (Unsure if v1 or later)

Eldiron is currently under heavy development and not yet ready for consumption.

## Installation

Eldiron is written in Rust, to run it you have to install Rust and its package manager cargo. Please follow the instructions on this [page](https://www.rust-lang.org/tools/install).

After you successfully installed Rust, check out this repository (or download the source via a .zip file), open a terminal, navigate to the Eldiron directory and start Eldiron with `cargo run --release`.

## The assets directory

Eldiron reads all assets from the assets directory, this is a top level directory in this repository. If you want to add your own tilemaps to Eldiron you will need to paste the tilemap image into the assets/tilemaps directory. Note that right now only tilemaps with square tiles are supported.

Eldiron comes with a range of standard tilemaps by default.

## License

The source and all assets I commissioned for Eldiron are licensed under the MIT. You can use the source and assets freely.

## Support

You can support the Eldiron project by becoming a [Patreon](https://patreon.com/eldiron).
