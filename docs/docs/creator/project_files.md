---
title: "Project Files"
sidebar_position: 2
---

# Project Files

Eldiron uses **.eldiron** for editable projects, local play and dedicated
hosting. For public multiplayer distribution, export a separate
**.eldiron-client** package for players.

## Creating a Project

Startup and **File → New** offer **New 2D Project**, **New 2D Iso Project** and
**New 3D Project**. Each creates a local project with the bundled ruleset, a
region, a player with WASD and arrow-key movement, and a responsive screen
with a game widget. The 2D region starts with painted grass and a path that
you can edit with Rect. The 2D Iso template configures the editor and player
for the isometric sprite grid. The 3D template opens in isometric view and
uses an isometric game camera.

**Browse Examples** opens the downloadable example catalog separately. Examples
open as unsaved projects; save a copy to keep your changes. Internet access is
needed only to browse and download examples. **Open Project…** opens an existing
local project.

Follow the [Wayfarer’s Rest walkthrough](wayfarers_rest.md) to try the village’s
daily routines, crafting, trade, and exploration.

## Container Format

New **.eldiron** files are ZIP containers with this initial layout:

```text
MyGame.eldiron
├── manifest.json
├── project.json
└── binaries/
    ├── bakes/
    │   └── <region-id>/<map-id>/...
    ├── tile-bakes/
    │   └── v1/
    │       ├── index.json
    │       └── <tile-id>.bin
    └── items/
        └── <item-id>/icons/
            ├── on/0.png
            └── off/0.png
```

`manifest.json` identifies the container and its format version. A minimal
version 1 manifest looks like this.

```json
{
  "format": "eldiron.project",
  "version": 1
}
```

`project.json` keeps the editable project and the metadata for saved bakes.
The binary sections are present when a project has the corresponding payloads.

| Archive path | Contents |
| --- | --- |
| `binaries/bakes/` | Background-bake color, depth, albedo, normal and material data, including raytraced region bakes. |
| `binaries/tile-bakes/v1/` | Pixel buffers for eligible baked tiles, including normal 2D Iso terrain and sprite frames and their material channels. The optional manifest field `tile_bakes_path` points to the tile index. Creator can load individual tile entries on demand. |
| `binaries/items/` | Customized item icon frames as PNGs. |

PNG entries are stored without redundant ZIP recompression. Raw data such as
bake depth and tile-bake buffers is deflated.

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

Projects store the ruleset selection in **Game / Settings**. Creator and the runtime
resolve that selection from the bundled catalogue. Legacy rules snapshots are
ignored on open and omitted on save. The project defines its world, character
options, placements, resource bindings and behavior graphs. Bundled artwork
uses shared references, and explicit appearance variants are saved with the
project. See [Rules in Eldiron](../rules_in_eldiron#source-of-truth).

## Compatibility

Legacy **.eldiron** files consisting of raw JSON are still accepted by Creator and all clients. Saving a legacy project in Creator writes it back in the current ZIP format. This is a one-way storage upgrade, so use version control or keep a copy if an older Eldiron build must still open the file.

Early version-one archives that stored background payloads directly below
`bakes/` also remain readable. Saving them again moves those entries under the
common `binaries/bakes/` layout. Legacy inline icon frames are similarly
externalized as PNG entries on save. Older archives with tile pixels inline
also open, and eligible buffers move to the tile-bake section on save.

Creator writes saves through a temporary file beside the destination and replaces the old project only after the new archive has been written successfully. This reduces the risk of leaving a partially written project if a save is interrupted.

## Web Client

For a browser build that runs a local game, place the authored project beside
the client files and name it `game.eldiron`. The web client supports both the
current container and legacy JSON. For a browser client that connects to a
published multiplayer world, use the export workflow below.

## Public multiplayer clients

For a multiplayer project, **File → Export Public Client…** writes a separate
`.eldiron-client` release. Set the project’s WSS endpoint first. Give the
package to the desktop Player or serve it alongside the browser Player.
Export includes public visuals and UI while leaving behavior source, authored
occupants and saves private. Keep saving your editable `.eldiron` project
separately. See [Public Client Export](public_client_export).
