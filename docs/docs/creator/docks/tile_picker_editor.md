---
title: "Tile Picker"
sidebar_position: 3
---

![Current Tile Picker with Project and MZ Materials tabs, collections, and tile assignment controls.](/img/creator/tile-picker.png)

The **Tile Picker** dock is the main asset browser and assignment dock for map work. It is used to:

- browse project tiles
- create and select [procedural materials](/docs/building_maps/procedural_materials)
- organize them in collections
- preview and select the current tile source
- copy the selected tile UUID from the dock toolbar
- apply or clear that source on geometry
- open the pixel editor with the top-right [dock control](/docs/creator/actions/#dock-controls)

For single tiles, **Edit Tile Meta** can assign an alias and gameplay tags. The alias identifies the visual tile anywhere a `tile_id`-style source is accepted, including `set_tile(...)`; it does not trigger events. Gameplay tags describe behavior and trigger `entered_tile` and `left_tile` character events when a character crosses a painted 2D tile placement.

The currently selected source is used by **Copy Tile ID**, **Apply Tile**, and **Clear Tile** in the dock toolbar, and by tools such as [Rect](/docs/creator/tools/rect). **Copy Tile ID** places the quoted UUID on the system clipboard for use in settings, scripts, and other source fields.

## Tiles And Collections

The picker shows individual tile assets, including animated and fully transparent
tiles. Use the **Project** tab to browse all project tiles. Custom **Collections**
provide curated views with their own board layout. Collections reference shared
assets; adding a tile to a collection does not copy it.

The **Collections** menu creates collections, manages membership, and imports or
exports local `.eldiron_tiles` collection files. The legacy Treasury tab and its
online package workflow have been removed.

## Empty Cells And New Tiles

Click any vacant board cell to select it, then use **Make Proc** to create a
procedural material there. Selecting an existing tile and choosing Make Proc
makes that tile procedural. The procedural sidebar opens automatically for
procedural selections, and its texture preview uses the minimap.

The separate **New Tile** action still creates bitmap tiles with a chosen size
and frame count. Fully transparent bitmap tiles remain selectable and editable.
Press Escape while the picker has focus to clear selection. Tile Groups and
the former Treasury workflow are no longer part of the picker.

## Applying Sources

### Apply Tile

Apply the currently selected source from the tile picker.

Behavior:

- in normal geometry editing, it assigns the selected source to the currently selected geometry/material slot
- in Region geometry, if the current action exposes HUD material slots, it assigns the selected source to the currently selected action icon slot instead
- on screens, it keeps the existing screen-specific material assignment behavior

Use this together with the HUD icon selection to decide which slot is being edited.

### Clear Tile

Clear the currently selected tile/material slot.

Behavior:

- in normal geometry editing, it removes the current geometry source assignment
- in Region geometry, if the selected action exposes HUD material slots, it clears the current action icon slot instead
- on screens, it keeps the existing screen-specific material clearing behavior

## Opening The Integrated Editors

Procedural tiles open the [MZ row editor](/docs/building_maps/procedural_materials)
in the Procedural Materials sidebar. It includes Props, Sources, Row Input and
Tile tabs. Compose individual placements directly on the map with the Rect
tool's **Compose** mode; use **Save copy to Tilepicker** to reuse a composition.

Select a tile and use the top-right **Edit / Maximize Dock** control (`Cmd/Ctrl + [`) to open the **pixel tile editor**.

Use the adjacent **Restore Dock** control (`Cmd/Ctrl + ]`) to return to the normal editor/dock split.

See:

- [Pixel Tile Editor](/docs/creator/docks/pixel_tile_editor)

## Authoring Mode

If **Authoring** mode is enabled in the tool strip, the tile picker is replaced by the **Authoring** dock for tile-backed map contexts. This lets you edit narrative metadata instead of tiles.

## Related Pages

- [Pixel Tile Editor](/docs/creator/docks/pixel_tile_editor)
- [Working With Tiles](/docs/building_maps/working_with_tiles)

## Bitmap images inside procedural materials

Choose **Make Proc** on a tile, open the procedural sidebar, and drag
**Image / Tile Source** from Sources' Image category into a row. Select the node and use
**Props → Source** to import an image or select an existing bitmap tile.

The source includes transparency and preserves animation/material maps from
linked project tiles. Editing a linked bitmap refreshes the procedural material.
Choose **Edges → Repeat** for a repeating bitmap, or Transparent for a stamp.
Color and W/Alpha can connect to the material's Color and Opacity outputs; RGBA
can also feed Alpha Over to combine the image with shapes or noise.
