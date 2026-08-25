---
title: "Pixel Tile Editor"
sidebar_position: 4
---

The **Pixel Tile Editor** is Eldiron’s integrated pixel editor for authored
tiles and item icon frames.

It opens when:

- you click a frame in an item's **Icon: On** or **Icon: Off** tree row, or
- a single tile is selected in the tile picker and you use the top-right [Edit
  / Maximize Dock control](/docs/creator/actions/#dock-controls), or press
  `Cmd/Ctrl + [`

Use the adjacent **Restore Dock** control or `Cmd/Ctrl + ]` to return to the normal split layout.

Changes are reflected immediately in the project and on the map or UI.

## What It Edits

The pixel editor works on authored tile textures and item icon state frames. It
is used for:

- painting tile pixels directly
- editing animated tile frames
- editing an item's default **On** frames and optional **Off** frames
- selecting and pasting pixel regions
- updating the final tile that is used in 2D and 3D

Undo / redo belongs to the selected tile or item icon state, so edits in On and
Off do not share a history. Use `Space` to preview the frames of the current
state as an animation.

For a ruleset-backed item, editing a frame stores project-owned frames in the
project archive. **Load Default** replaces the customized state frames with
that state's current ruleset artwork. Loading the default for On does not
change Off, or vice versa.

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

Tiles in Eldiron are used in both 2D and 3D. The pixel tile editor edits visible
color frames; it does not paint per-pixel render material values.

For authored pixel tiles:

- editing changes the visible tile texture directly
- normals are generated from the tile texture data used by Eldiron
- drawing uses the selected Art Palette color
- render material comes from high-level material metadata, such as tile presets, Art Palette entry presets, or object overrides

Procedural node groups use the node graph editor instead and can generate height-driven normals from graph output.

Ruleset icon state directories reserve an optional `material.png` for future
artist-authored material data: red is roughness, blue metallic, green emission,
and alpha is unused. The editor does not currently load or edit this file.

## Related Pages

- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Tile Node Graph](/docs/creator/docks/tile_node_graph)
