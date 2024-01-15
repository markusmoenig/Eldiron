![screenshot](images/eldiron_header.png)

Classic RPG Creation

---

![Windows](https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white) ![macOS](https://img.shields.io/badge/mac%20os-000000?style=for-the-badge&logo=macos&logoColor=F0F0F0) ![Linux](https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)

[![MIT license](https://img.shields.io/badge/License-MIT-blue.svg)](https://lbesson.mit-license.org/) [![version](https://img.shields.io/badge/version-0.8.2-yellow.svg)](https://shields.io/) [![Discord](https://badgen.net/badge/icon/discord?icon=discord&label)](https://discord.gg/ZrNj6baSZU) [![Patreon](https://badgen.net/badge/icon/patreon?icon=patreon&label)](https://patreon.com/eldiron) [![Twitter](https://badgen.net/badge/icon/twitter?icon=twitter&label)](https://twitter.com/EldironRPG)


<!---
[![YouTube](https://img.shields.io/badge/YouTube-FF0000?style=for-the-badge&logo=youtube&logoColor=white)](https://www.youtube.com/channel/UCCmrO356zLQv_m8dPEqBUfA)
-->

Eldiron is currently under active development and a v1 is planned for the first half of 2024. Eldiron is open source and licensed under the MIT.

I am in the process of rewriting Eldiron, the current release on GitHub still reflects the old code base. The first release for the new code base (v0.85) will come soon.

![Screenshot](images/character_screenshot.png)
![Screenshot](images/tilemap.png)

## Features of v1

* Support games similar to Ultima 4 / 5 or any game which uses a rectangular grid layout.
* Eldiron has an easy to use, grid based, visual scripting language for creating the game logic with visual feedback and debugging.
* Either render 2D tiles directly or in 3D. Games can switch between the two modes at runtime or display both at the same time (for example use the tiles view as a mini-map).
* Eldiron comes with integrated tile-maps or your can use your own square tiles with up to four levels of transparency.
* Single-player or multi-player experiences. Eldiron has a sophisticated multi-threaded server architecture to allow for as many player or NPC characters as possible.
* Cross platform. Eldiron is written in Rust and can nearly run everywhere, i.e. on the Web, macOS, Windows, Linux, iOS, Android etc. Eldiron Creator can run on any desktop.

Join the community on [Discord](https://discord.gg/ZrNj6baSZU) to get in contact.

## Goals

* Being able to create games similar to the RPGs of the 80's and 90's.
* Support single-player or multi-player games and even MMOs.
* Support 2D tiles and over time integrate procedural systems for tiles, particles and 3D objects and characters.

## Installation

You can download the current pre-release in [Releases](https://github.com/markusmoenig/Eldiron/releases).

On macOS you can get access to the current Beta via a public [TestFlight Link](https://testflight.apple.com/join/50oZ5yds).

For ArchLinux users, simply add Eldiron from AUR:
```
yay -S eldiron-bin
```

## Building Eldiron Locally

If you have [Rust installed](https://www.rust-lang.org/tools/install), you can build Eldiron Creator simply via
```cargo build --release --bin creator```

Linux:

 Make sure these dependencies are installed: `libasound2-dev` `libatk1.0-dev` `libgtk-3-dev`

## Supporting Eldiron

You can support the Eldiron project by becoming a [Patreon](https://patreon.com/eldiron) or a [GitHub Sponsor](https://github.com/sponsors/markusmoenig).

## License

The source and all assets I commissioned for Eldiron are licensed under the MIT.

Unless explicitly stated otherwise, any contribution intentionally submitted for inclusion in Eldiron, shall be MIT licensed as above, without any additional terms or conditions.

  ## Acknowledgements

* [Aleksandr Makarov](https://twitter.com/iknowkingrabbit) created the tilemaps which are currently shipped with Eldiron, you can see his work on [Twitch](https://iknowkingrabbit.itch.io).

## Sponsors

[![Digital Ocean](sponsors/DO_Logo_Horizontal_Blue.png)](https://www.digitalocean.com/?utm_medium=opensource&utm_source=Eldiron)
