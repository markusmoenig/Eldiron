---
title: "Working with Tiles"
sidebar_position: 1
---

## Importing Tilesets

Tiles are reusable visual sources for Eldiron maps. They can be imported bitmaps,
animated images, or editable [procedural materials](procedural_materials). New
projects include procedural world materials; importing a tileset is optional.

Choose **Add → Add Tiles…** in the project tree to load PNG artwork, then choose
**Sprite sheet** for one grid of frames or **Separate images** for individual
PNGs. **Layout** chooses ordinary **2D** or **2D Iso** artwork. See the
[Add Tiles dialog reference](isometric_sprite_tilesets#add-tiles-dialog)
for frame size, isometric ground cells, pivots and direction grouping.
Use **Import → Import Tileset…** for a saved Eldiron tileset or collection.

Under **After loading**, **Select in Tileset Editor** opens the source when
you click **Open Source**. Choose **Add All as Collection** to import the
frames directly into a named tile picker collection.

![Current tileset editor showing a source sheet and the Single Tiles, Animation, and Add Selected controls.](/img/creator/tileset-editor.png)

For a sheet, enter its **Image-frame width** and **Image-frame height** in
the import dialog. In the source editor, click or drag to select frames, choose
a role, and use **Add Selected** to put them in the tile picker.

Before adding the tile(s) make sure you selected the role as needed (to be able to filter and sort tiles correctly) and you can select one of two import modes:

- **Single Tiles**. Add the selected tiles as individual tiles.
- **Animation**. Add the selected tiles as one animated tile.

In the tile picker you can further edit tile sources and assign them to geometry. The dock toolbar contains **Copy Tile ID** for copying the selected source UUID, plus **Apply Tile** and **Clear Tile** for assigning or removing tile sources on map geometry and action material slots.

Single tiles can also have a human-readable alias. Set it with **Edit Tile Meta** and then use that alias anywhere a `tile_id`-style source is accepted in TOML, actions, or `set_tile(...)`.

**Edit Tile Meta Data** also provides comma-separated **Gameplay Tags**. When a character enters or leaves a painted 2D tile carrying these tags, it receives an [`entered_tile`](/docs/characters_items/events) or [`left_tile`](/docs/characters_items/events) event for each tag. This keeps tile-based interactions attached to the artwork when a tile is moved. Aliases and gameplay tags are separate: aliases identify visual sources, while gameplay tags describe behavior such as `chair`, `water`, `lava`, or `trap`.

Single tiles can also have high-level material metadata. Use **Edit Tile Meta** to assign a material preset and finish for tile sources. project palette colors carry their own material preset and finish in the **Palette** dock.

The tile picker contains individual tiles, which can be organised in collections.

Added tiles are shown with a slight gray overlay in the tileset editor, making it easy to see which files you already added. You can delete a tileset any time, already added tiles will not be affected.

Once you added tiles they are visible in the tile picker and ready to use in 2D and 3D.

## Using The Integrated Editors

In the tile picker you can use the [New Tile](/docs/creator/actions/#new-tile) or [Duplicate Tile](/docs/creator/actions/#duplicate-tile) actions to create new tiles.

Selecting a tile and clicking the top-right **Edit / Maximize Dock** control, or pressing `Cmd/Ctrl + [`, opens the integrated **pixel tile editor**.

Use the adjacent **Restore Dock** control or `Cmd/Ctrl + ]` to return to the normal split layout.

Drawing pixels uses the selected project palette color. The project has one
editable palette shared by world art, procedural recipes, appearances and UI;
see [Palette Tool](/docs/creator/tools/palette) for editing and switching it.

Editing tiles auto-updates them on the map. Undo is supported for each tile individually.

## Procedural Sources and Composition

Use **Make Proc** in the picker to create a procedural tile in a vacant cell or
make the selected tile procedural. Edit its rows in the Procedural Materials
sidebar and preview its channels in the minimap. The **Continuous**, **Repeat**
and **Seamless** controls are saved tiling settings and also affect map placements.

Use the Rect tool's **Compose** mode to combine sources on a painted cell, with
shape thumbnails for roads, corners, strips and blends. Several layers can build
on each other's output, including bitmap sources. **Save copy to Tilepicker**
makes the result reusable. Whole floors and walls can also use surface compositions.

See [Procedural Materials](procedural_materials) for the complete workflow and
[Game Configuration](/docs/configuration/game#tile-and-appearance-settings) for
global resolution, map cutout backgrounds and character/item pixelization.

See:

- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Pixel Tile Editor](/docs/creator/docks/pixel_tile_editor)
