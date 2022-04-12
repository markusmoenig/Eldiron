[![Discord](https://badgen.net/badge/icon/discord?icon=discord&label)](https://discord.gg/ZrNj6baSZU) [![Patreon](https://badgen.net/badge/icon/patreon?icon=patreon&label)](https://patreon.com/eldiron) [![Twitter](https://badgen.net/badge/icon/twitter?icon=twitter&label)](https://twitter.com/MarkusMoenig)

<!---
[![YouTube](https://img.shields.io/badge/YouTube-FF0000?style=for-the-badge&logo=youtube&logoColor=white)](https://www.youtube.com/channel/UCCmrO356zLQv_m8dPEqBUfA) -->

Create RPGs for every platform with Eldiron.

![screenshot](moody_goes_raiding_2.gif)

Eldiron v1 will be able to create games similar to Ultima 4 and Ultima 5 or every RPG or tactical game utilizing square tiles.

Eldiron is cross platform and runs on all Desktops. The game client will work on Desktops but also on iOS and Android devices and any other device Rust compiles on. It is designed from the ground up to be extremely portable.

Eldiron comes with a range of freely usable tilemaps for environment and characters, however you can of course use your own tilemaps, see the instructions below.

The game engine contains client and server modules, although currently no multi-player options exist yet, the code has been written with multi-player support in mind from the ground up.

Support for retro 3D areas like dungeons is upcoming. In general I want to explore many different ways to display and create content, from procedurally created tiles to 3D assets. These kind of features will get implemented over time.

Join the community on [Discord](https://discord.gg/ZrNj6baSZU).

<!---
I also try to maintain a development blog on [YouTube](https://www.youtube.com/channel/UCCmrO356zLQv_m8dPEqBUfA).
-->

## Updates

#### 12th April '22

[![IMAGE ALT TEXT](http://img.youtube.com/vi/Qt9cgmQjN-4/0.jpg)](http://www.youtube.com/watch?v=Qt9cgmQjN-4 "Orc closes in on Moody")

An orc is on the lookout for a character which has an *Align* variable greater than 0. He can only see a maximum distance of 5 so he has to wait until somebody passes by. In this case Moody passes by and the Orc is closing in on him.

Newly added features:

* Named behavior trees can be selected via the tab bar at the top of the node graph.
* New *Lookout* and *Close In* nodes.
* *Expression* and *Script* nodes now have a full scripting system behind them.
* New Systems editor where soon the user can create systems for Combat, Magic, Crafting and so on. Systems will allow the implementation of basic mechanics every character can utilize.

Coming soon:

* The *System* node will allow to call a behavior tree inside a System, like Combat -> Melee and the behavior tree will handle the combat mechanics.

## Installation

Eldiron is written in Rust, to run it you have to install Rust and its package manager cargo. Please follow the instructions on this [page](https://www.rust-lang.org/tools/install).

After you successfully installed Rust, check out this repository (or download the source via a .zip file), open a terminal, navigate to the Eldiron directory and start Eldiron with `cargo run --release`.

At a later stage I will provide pre-build binaries for each platform.

## The Tiles View (90% Done)

![tiles_screenshot](tiles.png)

In the Tiles view you can assign roles and animations to individual tiles in the currently available tilemaps.

Eldiron reads all assets from the assets directory, this is a top level directory in this repository. If you want to add your own tilemaps to Eldiron you will need to paste the tilemap image into the assets/tilemaps directory. Note that right now only tilemaps with square tiles are supported.

There are currently 2 tilemaps available, both with tiles in 16x16, the basic one contains tiles similar to Ultima 4 and the extended one has tiles similar to Ultima 5.

You can multi-select a range of tiles (via mouse click and drag) and by clicking the *Set Anim* button you create an animation for the first tile in the range. The other tiles will be set to *Unused* by default.

The *Clear Anim* button will remove an animation sequence form the currently selected tile.

The *Set Default* button will set the currently selected tile as the default tile of the tilemap, it will be shown as a tilemap icon in the tilemap overview.

The roles a tile can have are:

* **Unused** - This tile is ignored and will not be shown in the area editor.
* **Environment** - This is the default tile type for any kind of non blocking terrain. Use it for grass, floors etc.
* **Road** - Same as Environment but the AI in the upcoming Pathfinder node will prefer road tiles over environment tiles.
* **Blocking** - Every environment tile which is not accessible to the player, like rocks, mountains, walls etc.
* **Character** - Character tiles, like animation tiles for a warrior.
* **Utility** - Utility character tiles. Like a ship or a horse.
* **Water** - Water tiles. Tiles where a ship can go.
* **Effect** - Effect tiles, like an explosion.
* **Icon** - Icons.

Note that the behavior and look for tiles in a certain area or for a given tile in general can be freely adjusted via the Behavior node system.

## Areas View (30% Done)

![areas_screenshot](screen.png)

In the areas view you create specific in-game areas like a dungeon, the world itself or cities.

At the bottom of the view you can see the environment tiles of the selected tilemap (or tilemaps).

Select a tile and click in the area to apply the tile to the area map.

While the basic functionality of the area editor is working, many functions are missing:

* Undo / Redo
* Rectangular operations (cut / copy / paste / move / copy) etc.
* Assigning tiles to groups
* Sooner or later we need layers

## Behavior View (25% Done)

![behavior_screenshot](behavior.png)

In this view you will be able to create any kind of behavior for characters (combat, progression, crafting) or tiles (spawning, traps etc.).

The node system is simple but powerful and should be accessible by everybody. This is where currently the main work is happening.

## Items View (0% Done)

A specialized node view for in game items and their stats, modifiers and special effects.

## Game View (0% Done)

A specialized node view for the overall game logic.

## License

The source and all assets I commissioned for Eldiron are licensed under the MIT. You can use the source and assets freely.

## Support

You can support the Eldiron project by becoming a [Patreon](https://patreon.com/eldiron). I am also looking for art donations. If interested contact me on [Discord](https://discord.gg/ZrNj6baSZU).
