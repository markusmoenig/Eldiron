[![forthebadge made-with-rust](http://ForTheBadge.com/images/badges/made-with-rust.svg)](https://www.rust-lang.org/)

[![MIT License](https://img.shields.io/apm/l/atomic-design-ui.svg?)](https://github.com/tterb/atomic-design-ui/blob/master/LICENSEs) [![version](https://img.shields.io/badge/version-0.1-red.svg)](https://shields.io/) [![macOS](https://svgshare.com/i/ZjP.svg)](https://svgshare.com/i/ZjP.svg) [![Windows](https://svgshare.com/i/ZhY.svg)](https://svgshare.com/i/ZhY.svg) [![Linux](https://svgshare.com/i/Zhy.svg)](https://svgshare.com/i/Zhy.svg)


# Eldiron - Classic RPG Creation

Eldiron is a creator for classic RPGs written in Rust. Eldiron v1 will be able to create games similar to Ultima 4 and 5 but with a modern twist and features.

The game creator (which contains the game engine) is cross platform and should run on all Desktops. The game engine will work on Desktops but also on iOS and Android devices and any other device Rust compiles on. It is designed from the ground up to be extremely portable.

Eldiron comes with a range of freely usable tilemaps for environment and characters, however you can of course use your own tilemaps, instructions in the Wiki which serves as the main documentation hub.

The game engine contains a client and a server modules, although currently no multi-player options exist yet, the code has been written with multi-player support in mind from the ground up.

## Design Goals

* Keep the engine design flexible, so even if I target Ultima 4 / 5 games (or any game with square tiles) for v1, it will be possible to add support for other perspectives and game types after v1.
* Develop the engine design with multi-player support in mind from the ground up (instances or MMORPGs etc).
* Provide a set of game assets so that users can instantly start their own games.
* Develop a node based Behavior AI system which is easy to use but powerful enough to handle every possible aspect of an RPG (combat, crafting, farming, exploration etc.).
* Run everywhere

## Why ?

I played the Ultima games day and night in my youth so I decided to create my own similar games as I have some time to spare (what could be a better hobby ?), and on the way also develop a game creator for this type of game which I need my own games anyway.

It's fun, and creating an RPG system has a lot of unique challenges. The behavior node system I am working on should make it possible to easily implement any kind of functionality, be it combat, crafting or any other feature.

## Features and Status

For v1 I target these features:

* Tilemap editor to select tiles in bitmaps and assign tile types and animations. (Done)
* Commission game tiles and create a modular character tile system (In progress).
* World editor to create the world and it's areas (In progress).
* Behavior tree node system for creating the RPG system and AI (In progress).
* 3D Dungeons ? (Unsure if v1 or later)

Eldiron is currently under heavy development and not yet ready for consumption.

## Installation and Documentation

The game creator can be easily installed using Rust and it's cargo package manager. This is very easy to-do even if you have no programming experience. The installation guide and documentation is available in the Wiki of this repo.

## License

The source and all assets I commissioned for Eldiron are licensed under the MIT. You can use the source and assets freely.
