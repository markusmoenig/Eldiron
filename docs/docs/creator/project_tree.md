---
title: "Project Tree"
sidebar_position: 2
---

The project tree, located to the right of the *Eldiron Creator* contains all editable content of your game. Use the **+** and **-** buttons at the bottom of the tree to add or remove content, to the right of these buttons the current content context is displayed as well as **import** and **export** buttons.

Selecting specific content in the project tree will display a corresponding editor dock widget.

---

## Regions

Regions are the maps in your game which define the world, dungeons and towns.

You use the [geometry tools](tools/overview#map-tool-specifics) to create geometry for the regions. Regions can be viewed using a **2D** or various **3D** cameras.

Each region also has its own script entries in the tree:

- **Visual Scripting** for graph-based region logic.
- **Eldrin Scripting** for text-based region logic.

These region scripts are the right place for map-local behavior and runtime overrides, for example:

- region-specific palette remapping
- local fog or background overrides
- region-only quest or event coordination

## Characters

A **character template** is a reusable blueprint that defines the **behavior, attributes, and appearance** of a character in the game.

You can edit character behavior using either **Visual Scripting** or **Eldrin Scripting** or edit the initial **Attributes** of the character.

You can instantiate a character template into the map of the region by simply dragging and dropping the character template into the map (Click left of the *Name* item and drag).

In 3D region views, the drop uses the actual surface hit position, so the instance also stores the correct `y` height. This makes drag-and-drop placement work correctly on stairs, elevated geometry, and lowered dungeon floors.

You can use the [Entity Tool](tools/entity) to move or delete character instances after creation.

## Items

Item templates have similar functionality as *characters templates* but define a static or dynamic item in the game world.

Like with *characters* you can edit item behavior using either **Visual Scripting** or **Eldrin Scripting** or edit the **Attributes** of the item.

You can instantiate an item template into the map of the region by simply dragging and dropping the item template into the map (Click left of the *Name* item and drag).

In 3D region views, the drop uses the actual surface hit position, so the instance also stores the correct `y` height. This makes drag-and-drop placement work correctly on stairs, elevated geometry, and lowered dungeon floors.

You can use the [Entity Tool](tools/entity) to move or delete item instances after creation.

## Tilesets

![Tileset](/img/screenshots/tilesets.png)

Tilesets are PNG images with grid based tiles of pixel art. After importing a tile set and applying the correct **Grid Size** you can select individual or multiple tiles (click and drag horizontally or vertically to select) and add them to the tile picker.

Before adding the tile(s) make sure you selected the role as needed (to be able to filter and sort tiles correctly) and you can select one of three import modes:

- **Single**. Add the selected tiles as individual tiles.
- **Anim**. Add the selected tiles as one animated tile.
- **Multi**. Add the selected tiles as one big tile, containing multiple tiles.

In the tile picker you can further edit and organize tile sources. The tile picker toolbar also provides **Apply Tile** and **Clear Tile** for map geometry and action material slots.

The tile picker can contain:

- single tiles
- tile groups
- node groups

Added tiles are shown with a slight gray overlay in the tileset editor, making it easy to see which files you already added. You can delete a tileset any time, already added tiles will not be affected.

## Screens

Screens in *Eldiron* define the visible user interface of your game. Paint background decoration with the *Rect* tool and create **Sectors** and apply tiles.

See the **Working with Screens** chapter (coming soon).

Screen widgets can now bind character-facing UI to a party target instead of being hardcoded to the current player. This is used for things like:

- equipped hand slots for different party members
- inventory slot widgets bound to one member
- portrait buttons using a character's `portrait_tile_id`
- avatar preview widgets bound to `leader` or `party.N`

See [Widgets](../screens/widgets) for the `party` and `portrait` widget settings.

## Assets

Assets you added for your game will be listed here. Currently supported assets:

- **TTF** fonts for in-game text drawing.
- **WAV** and **OGG** audio files for music, ambience, UI sounds and effects.

Use the **+** button in the project tree to add assets:

- **Add Font Asset** to import a font.
- **Add Audio Asset** to import WAV/OGG audio.

For the complete audio workflow (assets, buses, and scripting), see [Audio](../audio).

## Palette

Shows the colors of your palette. Use the palette based **Actions** to clear or import palettes.

## Game

In the game section you can select:

- **Settings**. Edit your game settings, see all supported settings in the [reference](../reference/configuration).
- **World / Visual Scripting**. Edit graph-based world/global logic.
- **World / Eldrin Scripting**. Edit text-based world/global logic.
- **Authoring**. Edit global text-adventure and authoring behavior like startup text and sector description policies, see [Authoring Configuration](../configuration/authoring).
- **Rules**. Edit project-wide gameplay rules and formulas in a TOML-based data editor, see [Rules](../rules).
- **Locales**. Edit shared localization tables like `[en]` and `[de]` in a TOML-based data editor, see [Localization](../localization).
- **Audio FX**. Edit generated micro sound effects in a TOML-based data editor with built-in preview, see [Audio](../audio).
- **Debug Log**. Displays state during game play, important especially to diagnose server startup or runtime errors. Shown by default after starting the game server.

Use the **world** scripts for global state that should survive across regions, and use the **region** scripts for state and behavior local to one map.

For editing per-sector, per-linedef, per-entity, and per-item narrative metadata inside regions, see [Authoring](./authoring).
