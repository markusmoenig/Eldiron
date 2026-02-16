---
title: "Tile Picker & Editor"
sidebar_position: 3
---

![Tile Picker](/img/screenshots/dungeon3d_iso.png)

The tile picker dock is open when working on maps, like regional maps or screens. The currently selected tile is used for example for the [Apply Tile](/docs/creator/actions/#apply-tile) sector based action and the [Rect](/docs/creator/tools/rect) tool.

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
