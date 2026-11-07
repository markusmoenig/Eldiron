---
title: "Pixel Tile Editor"
sidebar_position: 4
---

The **Pixel Tile Editor** is Eldiron’s integrated pixel editor for authored
bitmap tiles and linked image sources.

Select a bitmap tile in the Tile Picker and use the top-right [Edit / Maximize
Dock control](/docs/creator/actions/#dock-controls), or press `Cmd/Ctrl + [`.
Procedural tiles use the [MZ row editor](/docs/building_maps/procedural_materials).
Items now open through [Appearance](/docs/characters_items/appearance); **Icon On**
and **Icon Off** edit their independent MZ state recipes.

Use the adjacent **Restore Dock** control or `Cmd/Ctrl + ]` to return to the normal split layout.

Changes are reflected immediately in the project and on the map or UI.

## What It Edits

The pixel editor paints bitmap tile textures and their animation frames. It
supports direct drawing, filling, erasing, selecting and pasting pixel regions.
Edits update the source tile used in 2D/3D. Use `Space` to preview its animation.

For an image-backed MZ item icon, use **Image / Tile Source** in the item's recipe.
A linked project bitmap can be opened here through the Tile Picker; editing it
updates recipe consumers. Embedded imported images remain part of their MZ
bundle. Procedural shape/color nodes are edited in MZ, not painted here.

Projects can also retain explicit bitmap item-state frame overrides, serialized
as PNGs under `binaries/items/<item-id>/icons/<state>/`. These are separate from
the source tile and from MZ recipes. The current item authoring entry point is
the [Appearance workspace](/docs/characters_items/appearance).

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
- drawing uses the selected project palette color
- render material comes from high-level material metadata, such as tile presets, project palette entry presets, or object overrides

To use bitmap artwork inside an MZ item icon, add **Image / Tile Source** to its
recipe. A linked project bitmap remains editable here through the Tile Picker;
the recipe combines it with procedural operations. Official MZ icon output
includes generated roughness, metallic, opacity and emission alongside color.

Ruleset PNG icon state directories reserve an optional `material.png` for future
artist-authored material data: red is roughness, blue metallic, green emission,
and alpha is unused. The editor does not currently load or edit this file.

## Related Pages

- [Tile Picker](/docs/creator/docks/tile_picker_editor)
