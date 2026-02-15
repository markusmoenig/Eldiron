---
title: "Working with Tiles"
sidebar_position: 1
---

## Importing Tilesets

Tiles are the basic building block of your Eldiron maps, a basic Ultima 4 style tileset is included in the starter project of Eldiron.

You can add more tilesets to your project by clicking the **+** button in the bottom of the project tree and select **Add Tileset**.

![Tileset](/img/screenshots/tilesets.png)

Tilesets are PNG images with grid based tiles of pixel art. After importing a tile set and applying the correct **Grid Size** you can select individual or multiple tiles (click and drag horizontally or vertically to select) and add them to the tile picker.

Before adding the tile(s) make sure you selected the role as needed (to be able to filter and sort tiles correctly) and you can select one of three import modes:

- **Single**. Add the selected tiles as individual tiles.
- **Anim**. Add the selected tiles as one animated tile.
- **Multi**. Add the selected tiles as one big tile, containing multiple tiles.

In the tile picker you can further edit tiles, both their meta data and their pixels, using the corresponding *Actions*. 

Added tiles are shown with a slight gray overlay in the tileset editor, making it easy to see which files you already added. You can delete a tileset any time, already added tiles will not be affected.

Once you added tiles they are visible in the tile picker and ready to use in 2D and 3D.

## Using the built-in Tile Editor

In the tile picker you can use the [New Tile](/docs/creator/actions/#new-tile) or [Duplicate Tile](/docs/creator/actions/#duplicate-tile) actions to create new tiles.

Selecting a tile and activating the [Edit / Maximize](/docs/creator/actions/#edit--maximize) action opens the integrated tile editor which has its own set of tools.

Drawing pixels uses the currently selected color in the project tree palette. There are various palette based actions available like [Clear Palette](/docs/creator/actions/#clear-palette) or [Import Palette](/docs/creator/actions/#import-palette).

Editing tiles auto-updates them on the map. Undo is supported for each tile individually.
