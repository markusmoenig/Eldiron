---
title: "Pixel Tile Editor"
sidebar_position: 4
---

The **Pixel Tile Editor** is Eldiron’s integrated pixel editor for authored tiles.

It opens when:

- a single tile is selected in the tile picker
- and you use [Edit / Maximize](/docs/creator/actions/#edit--maximize)

Changes are reflected immediately in the project and on the map.

## What It Edits

The pixel editor works on authored tile textures and their frames. It is used for:

- painting tile pixels directly
- editing animated tile frames
- selecting and pasting pixel regions
- updating the final tile that is used in 2D and 3D

Undo / redo is tile-based. Each tile has its own undo stack.

## Core Tools

The editor currently has these core tools:

- **Draw Tool (`D`)**: paint pixels with the current color
- **Fill Tool (`F`)**: flood-fill connected pixels
- **Eraser Tool (`E`)**: clear pixels to transparency
- **Selection Tool (`S`)**: create, add, or subtract rectangular selections

If a selection exists, drawing, filling, and erasing are limited to that selected area.

## Useful Shortcuts

- `Cmd/Ctrl + C`: copy selected pixels, or the whole tile if nothing is selected
- `Cmd/Ctrl + X`: cut the current selection
- `Cmd/Ctrl + V`: paste image data as a paste preview
- `Enter`: apply the current paste preview
- `Escape`: cancel the current paste preview
- `H`: flip horizontally
- `V`: flip vertically
- `F`: activate Fill
- `E`: activate Eraser
- `Space`: toggle animated preview

Paste preview and direct drawing are separate modes. While paste preview is active, place or cancel it first, then continue painting.

## Materials And Normals

Tiles in Eldiron are used in both 2D and 3D, so the final tile data also carries packed material data and normals.

For authored pixel tiles:

- editing changes the visible tile texture directly
- normals are generated from the tile texture/material data path used by Eldiron

Procedural node groups use the node graph editor instead and can generate height-driven normals from graph output.

## Related Pages

- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Tile Node Graph](/docs/creator/docks/tile_node_graph)
