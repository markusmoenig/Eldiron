---
title: "Project Tree"
sidebar_position: 2
---

The project tree is available on the **Project** page of the right sidebar in *Eldiron Creator* and contains all editable content of your game. Use the **+** and **-** buttons at the bottom of the tree to add or remove content. To the right of these buttons, the current content context is displayed together with the **import** and **export** buttons.

The compact sidebar tabs switch between the **Project** tree, contextual **Actions**, **Node List**, **Console**, **Debug**, **Help**, and **Procedural Materials**. Selecting a procedural tile opens its row editor automatically. Console presents structured runtime output above its command entry. [Debug](debug) contains live diagnostics and works with the Nodes dock's execution feedback. Help provides ruleset-aware answers. Use `Tab` and `Shift+Tab` to cycle. Direct shortcuts are `Ctrl/Cmd+Shift+F` (Project), `Ctrl/Cmd+Shift+G` (Actions), `Ctrl/Cmd+Shift+H` (Console), `Ctrl/Cmd+Shift+J` (Debug), and `Ctrl/Cmd+Shift+K` (Help). Material previews use the minimap; geometry/rig editing uses the main viewport.

Selecting specific content in the project tree displays its corresponding editor dock widget. Camera shortcuts remain available as right-aligned icons beside the project tabs. After the separator, use the frame-corners control (`Cmd/Ctrl + [`) to open or maximize the current dock editor and the down-caret control (`Cmd/Ctrl + ]`) to restore the normal split.

---

## Regions

Regions are the maps in your game which define the world, dungeons and towns.

You use the [geometry tools](tools/overview#map-tools-specifics) to create geometry for the regions. Regions can be viewed using a **2D** or various **3D** cameras.

Each region also has its own behavior entry in the tree:

- **Behavior Nodes** for region events.

Region settings configure map-local rendering, including:

- region-specific palette remapping
- local fog or background overrides

Use the region graph for region events.

## Characters

A **character template** is a reusable blueprint that defines the **behavior, attributes, and appearance** of a character in the game.

Use **Behavior Nodes** for character reactions and **Entity Nodes** for initial configuration, race/class selection, and player input.

**Appearance** opens a full preview with 2D/3D and animation controls. Choose
clothing and equipment in character setup, including drag-and-drop onto an
**Equipment** node. Use **Edit model** only when changing the body's geometry or
rig. See [Character and Item Appearance](../characters_items/appearance).

You can instantiate a character template into the map of the region by simply dragging and dropping the character template into the map (Click left of the *Name* item and drag).

In 3D region views, the drop uses the actual surface hit position, so the instance also stores the correct `y` height. This makes drag-and-drop placement work correctly on stairs, elevated geometry, and lowered dungeon floors.

You can use the [Entity Tool](tools/entity) to move or delete character instances after creation.

## Items

Item templates have similar functionality as *characters templates* but define a static or dynamic item in the game world.

Use **Behavior Nodes** for item reactions and **Entity Nodes** for initial configuration.

Each item's **Appearance** entry opens the consolidated workspace. **Icon On**,
**Icon Off**, **Model On** and **Model Off** open the respective MZ artwork;
**Worn model** and **Materials** are available when the item defines them.
The **On / Off** control selects the preview state, and **On character / Object**
switches between equipped and standalone views. The old separate icon-state
tree rows are replaced by this workspace. State recipes are independent and
remain editable through save/reopen and Undo/Redo.

The ordinary picker lists world tiles; internal ruleset icon/material swatches
are accessed through Appearance rather than filling the world tile board.
See [Appearance](../characters_items/appearance) for bitmap sources, state
behavior and shared asset editing.

You can instantiate an item template into the map of the region by simply dragging and dropping the item template into the map (Click left of the *Name* item and drag).

In 3D region views, the drop uses the actual surface hit position, so the instance also stores the correct `y` height. This makes drag-and-drop placement work correctly on stairs, elevated geometry, and lowered dungeon floors.

You can use the [Entity Tool](tools/entity) to move or delete item instances after creation.

## Tilesets

![Tileset](/img/screenshots/tilesets.png)

Tilesets are PNG images with grid based tiles of pixel art. After importing a tile set and applying the correct **Grid Size** you can select individual or multiple tiles (click and drag horizontally or vertically to select) and add them to the tile picker.

Before adding the tile(s) make sure you selected the role as needed (to be able to filter and sort tiles correctly) and you can select one of two import modes:

- **Single**. Add the selected tiles as individual tiles.
- **Anim**. Add the selected tiles as one animated tile.

In the tile picker you can further edit and organize tile sources. The tile picker toolbar also provides **Apply Tile** and **Clear Tile** for map geometry and action material slots.

The tile picker contains individual tiles, which can be organised in collections.

Added tiles are shown with a slight gray overlay in the tileset editor, making it easy to see which files you already added. You can delete a tileset any time, already added tiles will not be affected.

## Screens

Screens in *Eldiron* define the visible user interface of your game. Paint background decoration with the *Rect* tool and create **Sectors** and apply tiles.

See [Screens](/docs/screens/) and [Widgets](../screens/widgets).
Screen decoration and widget artwork can use the same procedural materials as
the world, including opacity, animation and repeating/continuous coordinates.

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

The project has one editable palette shared by world art, procedural materials,
avatars, item defaults and game UI. The official default is Endesga 64. Use the
[Palette Tool](tools/palette) to edit slots or preview a palette from Lospec or a
local file. Project choices survive ruleset refreshes.

## Game

In the game section you can select:

- **Settings**. Edit your game settings and see all supported options in [Game Configuration](../configuration/game).
- **World / Behavior Nodes**. Edit global event-driven behavior.
- **Authoring**. Edit global text-adventure and authoring behavior like startup text and sector description policies, see [Authoring Configuration](../configuration/authoring).
- **Rules**. Edit project-wide gameplay definitions, formulas, icons, and particle effects in the rules editor, see [Ruleset Editor](docks/ruleset_nodes).
- **Locales**. Edit shared localization tables like `[en]` and `[de]` in a TOML-based data editor, see [Localization](../localization).
- **Audio FX**. Edit generated micro sound effects in a TOML-based data editor with built-in preview, see [Audio](../audio).

Use world and region graphs for their respective events. Available nodes depend on the selected graph owner.

For editing per-sector, per-linedef, per-entity, and per-item narrative metadata inside regions, see [Authoring](./authoring).
