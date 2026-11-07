---
title: "Project Files"
sidebar_position: 2
---

# Project Files

Eldiron uses the **.eldiron** extension for Creator projects. The same file can be reopened in Creator and played by the desktop, terminal, or web clients.

## Creating a Project

Startup and **File → New** show a compact chooser. **New 2D Project** and
**New 3D Project** generate a project locally using the current
bundled settings and ruleset. Each includes a region with a floor, a player
character with WASD and arrow-key movement, and a responsive screen with a game
widget. The 2D region starts with a small patch of painted grass tiles and a
path, editable with the Rect tool. The 3D project opens in isometric view and
uses an isometric game camera.

**Browse Examples** opens the downloadable example catalog separately. Examples
open as unsaved projects; save a copy to keep your changes. Internet access is
needed only to browse and download examples. **Open Project…** opens an existing
local project.

## Container Format

New **.eldiron** files are ZIP containers with this initial layout:

```text
MyGame.eldiron
├── manifest.json
├── project.json
└── binaries/
    ├── bakes/
    │   └── <region-id>/<map-id>/...
    └── items/
        └── <item-id>/icons/
            ├── on/0.png
            └── off/0.png
```

`manifest.json` identifies the container and its format version. Version 1 contains:

```json
{
  "format": "eldiron.project",
  "version": 1
}
```

`project.json` contains the serialized project model and stable paths to its
binary payloads. Large byte arrays are written as separate entries through one
shared archive API. Background-bake color, depth, albedo, normal, and material
payloads live below `binaries/bakes/`. Customized item icon frames live below
`binaries/items/` as ordinary PNGs. PNG entries are stored without redundant
ZIP recompression, while raw data such as bake depth is deflated.

Inherited ruleset and tile icons are not copied into the project. An item gains
binary icon entries only after its On or Off frames are customized. **Load
Default** can replace those frames from their current source on the next save.

The internal layout is an interchange format, not a second project-authoring workflow. Normally, edit projects in Creator rather than changing `project.json` by hand.

## Procedural assets

Projects retain editable MZ material bundles and palette bindings, compositions
and their map assignments, geometry/rig/motion documents, and saved bake data.
Rendered pixels are derived data; do not delete the recipes when distributing
an editable project. Linked project image tiles must travel with their consumers.
Project-owned appearance edits remain authoritative when ruleset defaults refresh.

Eldiron includes the Rust material core in `crates/materialz`. Building or opening
a project does not require a sibling MaterialZ repository, the standalone Swift
application or a network lookup for its source code. Fresh or invalidated bakes
can still take time; continuous world materials can require larger coverage than
self-contained repeating tiles.

## Portable screens

Screen import/export uses a self-contained `eldiron.screen` package. It carries
the screen, referenced tiles, editable MZ recipes, composition dependencies,
linked image tiles and source palette. Import allocates new identities, remaps
references and maps colors into the receiving project's palette. The change is
undoable. Screen names are made unique, but commands and navigation to other
screens remain the receiving project's responsibility.

See [Screens](/docs/screens/#procedural-screen-artwork).

## Ruleset Storage

Projects save their complete current rules document, preserved original, and
recovery checkpoints in `project.json` under `rules_document`. These snapshots
travel with the project. **Restore All** restores the project's original rules,
even if the bundled rules change later. Save the project to persist edits and
checkpoints. See [Ruleset Editor](docks/ruleset_nodes).

## Compatibility

Legacy **.eldiron** files consisting of raw JSON are still accepted by Creator and all clients. Saving a legacy project in Creator writes it back in the current ZIP format. This is a one-way storage upgrade, so use version control or keep a copy if an older Eldiron build must still open the file.

Early version-one archives that stored background payloads directly below
`bakes/` also remain readable. Saving them again moves those entries under the
common `binaries/bakes/` layout. Legacy inline icon frames are similarly
externalized as PNG entries on save.

Creator writes saves through a temporary file beside the destination and replaces the old project only after the new archive has been written successfully. This reduces the risk of leaving a partially written project if a save is interrupted.

## Web Client

For web builds, place the project beside the client files and name it `game.eldiron`. The web client downloads it as binary data and supports both the current container and legacy JSON files.
