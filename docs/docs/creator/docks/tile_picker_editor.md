---
title: "Tile Picker & Editor"
sidebar_position: 3
---

![Tile Picker](/img/screenshots/dungeon3d_iso.png)

The tile picker dock is open when working on maps, like regional maps or screens. The currently selected tile is used by the **Apply Tile** button in the dock toolbar, by the **Clear Tile** button for removing a tile source, and by the [Rect](/docs/creator/tools/rect) tool.

In Region geometry, these buttons work in two modes:

- direct geometry mode: apply or clear the selected sector/linedef/vertex material source
- action slot mode: if the current action exposes HUD material slots, apply or clear the currently selected HUD icon slot instead

This is used by actions such as **Build Room**, where `ROOM`, `FLOOR`, `WALL`, and `CEIL` can be assigned directly from the tile picker before the action is applied.

## Tile Picker Buttons

### Apply Tile

Apply the currently selected tile from the tile picker.

Behavior:

- in normal geometry editing, it assigns the tile to the currently selected geometry/material slot
- in Region geometry, if the currently selected action exposes HUD material slots, it assigns the tile to the currently selected action icon slot instead
- on screens, it keeps the existing screen-specific material assignment behavior

Use this together with the HUD icon selection to decide which slot is being edited.

### Clear Tile

Clear the currently selected tile/material slot.

Behavior:

- in normal geometry editing, it removes the current geometry source assignment
- in Region geometry, if the currently selected action exposes HUD material slots, it clears the currently selected action icon slot instead
- on screens, it keeps the existing screen-specific material clearing behavior

If **Authoring** mode is enabled in the tool strip, the tile picker is replaced by the **Authoring** dock for tile-backed map contexts. This lets you edit narrative metadata instead of tiles.

The [Edit / Maximize](/docs/creator/actions/#edit--maximize) action opens the integrated tile editor where you can directly edit the tile.
 
Changes to the tile are instantly reflected on the map. Undo / redo are tile based. Each tile has its own undo stack.

## Tile Editor Tools

The editor currently has three core tools:

- **Draw Tool (`D`)**: Click or drag to paint pixels with the current color. If a selection exists, drawing only affects selected pixels. Hold `Shift` while drawing to erase pixels.
- **Fill Tool (`F`)**: Click to flood-fill connected pixels with the current color. If a selection exists, fill is limited to the selected area.
- **Eraser Tool (`E`)**: Click or drag to clear pixels to transparency. If a selection exists, erasing is limited to the selected area.
- **Selection Tool (`S`)**: Drag to create a selection rectangle. Hold `Shift` while dragging to add to the current selection, or `Alt` to subtract from it.

## Useful Shortcuts

- `Cmd/Ctrl + C`: Copy selected pixels (or the full tile if nothing is selected).
- `Cmd/Ctrl + X`: Cut selection (copy + clear selected area).
- `Cmd/Ctrl + V`: Paste image from clipboard as a paste preview.
- `Enter`: Apply the current paste preview.
- `Escape`: Cancel the current paste preview.
- `H`: Flip horizontally.
- `V`: Flip vertically.
- `F`: Activate Fill tool.
- `E`: Activate Eraser tool.
- `Space`: Toggle animated preview.

Paste preview and direct drawing are separate modes. While paste preview is active, place/apply/cancel the preview first, then continue painting.

## Authoring Dock

The Authoring dock is used to enter TOML metadata for selected sectors, linedefs, entities, and items.

The current base template is:

```toml
title = ""
description = """
"""
```

`title` is a short label, while `description` is intended for longer room, connection, item, or character text.
