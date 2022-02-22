# Eldiron - Classic RPG Creation

Eldiron is a creator for classic RPGs written in Rust. Eldiron v1 will be able to create games similar to Ultima 4 and 5 but with a modern twist and features.

The game creator (which contains the game engine) is cross platform and should run on all Desktops. The game engine will work on Desktops but also on iOS and Android devices and possibly other devices. It is designed from the ground up to be extremely portable.

Eldiron comes with a range of freely usable 16x16 and 24x24 tilemaps for environment and characters, however you can of course use your own tilemaps, instructions in the Wiki which serves as the main documentation hub.

## Why ?

I played the Ultima games day and night in my youth so I decided to create my own similar games as I have some time to spare (what could be a better hobby ?), and on the way also develop a game creator for this type of game which I have todo for my own games anyway.

It's fun, and creating an RPG system has a lot of unique challenges. The behavior node system I am working on should make it possible to easily implement any kind of functionality, be it combat, crafting or any other feature.

## Features and Status

For v1 I target these features:

* Tilemap editor to select tiles in bitmaps and assign tile types and animations. (Done)
* Commission game tiles and create a modular character tile system (In progress).
* World editor to create the world and it's areas (In progress).
* Behavior tree node system for creating the RPG system and AI (TBD).
* 3D Dungeons ? (Unsure if v1 or later)

Eldiron is currently under heavy development and not yet ready for consumption.

## Installation and Documentation

The game creator can be easily installed using Rust and it's cargo package manager. This is very easy to-do even if you have no programming experience. The installation guide and documentation is available in the Wiki of this repo.

## License

The source and all assets I commissioned for Eldiron are licensed under the MIT. You can use the source and assets freely.
