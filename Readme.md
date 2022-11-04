![screenshot](docs/eldiron_logo.png)

Classic RPG Creation

[![MIT License](https://img.shields.io/apm/l/atomic-design-ui.svg?)](https://github.com/tterb/atomic-design-ui/blob/master/LICENSEs) [![version](https://img.shields.io/badge/version-0.7.0-red.svg)](https://shields.io/) [![macOS](https://svgshare.com/i/ZjP.svg)](https://svgshare.com/i/ZjP.svg) [![Windows](https://svgshare.com/i/ZhY.svg)](https://svgshare.com/i/ZhY.svg) [![Linux](https://svgshare.com/i/Zhy.svg)](https://svgshare.com/i/Zhy.svg) [![Discord](https://badgen.net/badge/icon/discord?icon=discord&label)](https://discord.gg/ZrNj6baSZU) [![Patreon](https://badgen.net/badge/icon/patreon?icon=patreon&label)](https://patreon.com/eldiron) [![Twitter](https://badgen.net/badge/icon/twitter?icon=twitter&label)](https://twitter.com/EldironRPG)

<!---
[![YouTube](https://img.shields.io/badge/YouTube-FF0000?style=for-the-badge&logo=youtube&logoColor=white)](https://www.youtube.com/channel/UCCmrO356zLQv_m8dPEqBUfA)
-->

Eldiron is currently under active development and a v1 is planned for the first quarter of 2023. Eldiron is open source and licensed under the MIT.

<table><tr>
<td> <img src="docs/moody_goes_raiding_3.gif" alt="Screen 1" style="width: 300px;"/> </td>
<td> <img src="docs/screen_regions_tiles.png" alt="Screen 2" style="width: 300px;"/> </td>
<td> <img src="docs/screen_tiles.png" alt="Screen 2" style="width: 300px;"/> </td>
</tr></table>

## Features of v1

* Support games similar to Ultima 4 / 5 or any game which uses a rectangular grid layout.
* Eldiron comes with integrated tile-maps or your can use your own square tiles with up to four levels of transparency.
* Single-player or multi-player experiences. Eldiron has a sophisticated multi-threaded server architecture to allow for as many player or NPC characters as possible.
* Cross platform. Eldiron is written in Rust and can nearly run everywhere, i.e. on the Web, macOS, Windows, Linux, iOS, Android etc. Eldiron Creator can run on any desktop.
* A sophisticated behavior node graph makes creation of AI behavior for NPCs easy. The node system is backed by a full-featured scripting language.
* Eldiron Creator has editors for tile-maps, regions and node based graphs for character behavior, systems (like crafting), items and the overall game logic.

Retro 3D dungeons and regions will be supported in either v1 or v1.5. I am working on a [procedural language](https://github.com/markusmoenig/RPU) just for that.

You can download the current pre-release in [Releases](https://github.com/markusmoenig/Eldiron/releases).

Join the community on [Discord](https://discord.gg/ZrNj6baSZU) to get in contact.

## Goals

* Being able to create games similar to the RPGs of the 80's and 90's.
* Support single-player or multi-player games and even MMOs.
* Over time support more perspectives like top-down and isometric.

## Eldiron Book

I am currently working on the [Eldiron Book](https://book.eldiron.com). Please refer to the book for more detailed information on the creator, the server and clients and the overall project.

## Supporting Eldiron

You can support the Eldiron project by becoming a [Patreon](https://patreon.com/eldiron) or a [GitHub Sponsor](https://github.com/sponsors/markusmoenig).

## Building Eldiron Locally

First, create the directory `embedded` in the `core_embed_binaries` directory:

```sh
$ mkdir core_embed_binaries/embedded
```

Linux:

 Make sure these dependencies are installed: `libasound2-dev` `libatk1.0-dev` `libgtk-3-dev`

## License

The source and all assets I commissioned for Eldiron are licensed under the MIT. You can use the source and assets freely.

  ## Acknowledgements

* [Aleksandr Makarov](https://twitter.com/iknowkingrabbit) created the tilemaps which are currently shipped with Eldiron, you can see his work on [Twitch](https://iknowkingrabbit.itch.io).
