---
title: "Tile Picker"
sidebar_position: 3
---

![Tile Picker](/img/screenshots/dungeon3d_iso.png)

The **Tile Picker** dock is the main asset browser and assignment dock for map work. It is used to:

- browse project tiles
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

Click any vacant board cell to select it. The selected cell has a white outline
and becomes the destination for the **New Tile** action. In a collection, the new
tile is also added to that collection. Clicking an empty cell does not create a
tile until you apply the creation action.

Use **New** in the picker toolbar to select the New Tile action and configure its
size and animation frames. Existing fully transparent tiles remain selectable
and editable. Click a tile to select an asset, or press Escape while the picker
has focus to clear selection.

Tile Groups and their nested editing and grouped placement workflows have been
removed. Older projects convert groups to ordinary image tiles while keeping the
original member tiles available individually. Existing per-cell surface
assignments remain in place.

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

Select a tile and use the top-right **Edit / Maximize Dock** control (`Cmd/Ctrl + [`) to open the **pixel tile editor**.

Use the adjacent **Restore Dock** control (`Cmd/Ctrl + ]`) to return to the normal editor/dock split.

See:

- [Pixel Tile Editor](/docs/creator/docks/pixel_tile_editor)

## Authoring Mode

If **Authoring** mode is enabled in the tool strip, the tile picker is replaced by the **Authoring** dock for tile-backed map contexts. This lets you edit narrative metadata instead of tiles.

## Related Pages

- [Pixel Tile Editor](/docs/creator/docks/pixel_tile_editor)
- [Working With Tiles](/docs/building_maps/working_with_tiles)
