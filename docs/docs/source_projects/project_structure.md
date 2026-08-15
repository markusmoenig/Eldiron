---
title: "Project Structure"
sidebar_position: 3
---

# Project Structure

A newly scaffolded Eldiron Source project has this layout:

```text
my-game/
├── eldiron.toml
├── main.els
├── assets/
├── characters/
│   └── player.els
├── items/
├── recipes/
├── regions/
│   └── cellar.els
├── screens/
│   └── play.els
├── scripts/
├── tiles/
└── build/
```

## Configuration

`eldiron.toml` is the project manifest. It names the main source file, selects the starting region and screen, configures the runtime, and chooses the generated file path.

The initial project uses settings similar to these:

```toml
[project]
name = "My Game"
version = "0.1.0"

[source]
main = "main.els"

[game]
start_region = "cellar"
start_screen = "play"
client_mode = "terminal"
terminal_mode = "roguelike"
simulation_mode = "hybrid"
collision_mode = "tile"
auto_create_player = true
player = "player"

[build]
output = "build/game.eldiron"
```

Other runtime sections, such as `[viewport]`, `[terminal]`, `[render]`, and `[post]`, are carried into the generated project's configuration.

Graphical projects can scale their authored viewport with the client window while preserving its aspect ratio:

```toml
[viewport]
width = 960
height = 600
upscale = "aspect"
```

An optional `window_scale = 2.0` opens the initial window at twice the authored viewport size. It does not change the internal rendering resolution.

## Source files

The compiler reads:

- the file selected by `[source] main`;
- direct `.els` files in `characters/`;
- direct `.els` files in `items/`;
- direct `.els` files in `regions/`;
- direct `.els` files in `screens/`.

Declarations can technically share a file, but keeping them in the matching directories makes larger projects easier to navigate. The current compiler reads `.els` files directly inside these directories; nested source directories are not scanned.

Character and item behavior is currently written in an embedded `script` block in its `.els` declaration. The scaffolded `scripts/` directory is reserved for source-side organization and future standalone script loading.

## Assets and tiles

`assets/` is scanned recursively for general project assets:

- `.ttf` and `.otf` fonts;
- `.wav`, `.ogg`, `.mp3`, and `.flac` audio;
- `.png`, `.jpg`, and `.jpeg` images.

Image files in `tiles/` and `images/` are imported recursively as tiles. Their aliases are derived from their paths relative to the directory root.

Additional image-tile roots, explicitly described tiles, and animations can be configured in the `[source]` section of `eldiron.toml`.

## Procedural recipes

The `recipes/` directory is scanned recursively for `.recipe` files. They are compiled and embedded in the generated project as normal procedural recipe assets and tiles.

See [Procedural Recipes](../procedural_recipes/index.md) for the recipe language and rendering workflow.

## Build output

The file configured by `[build] output` is recreated by every successful build. Do not make lasting edits directly to this generated `.eldiron` file; make the change in `eldiron.toml`, an `.els` file, or an asset and rebuild.
