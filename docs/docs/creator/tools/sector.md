---
title: "Sector Tool"
sidebar_position: 4
---

The **Sector Tool** (keyboard shortcut **'E'**) allows you to **select, edit, move and delete sectors** in the map.

Unlike the **Selection Tool**, which can select multiple types of geometry at once, the **Sector Tool** is specifically designed for working with **sectors only**. Unlike the **Linedef Tool**, it does not include a creation mode, as sectors are automatically formed when a closed shape is created.

## Selection Modes

- **Click**: Select a sector.
- **Shift + Click**: Select multiple sectors.
- **Alt (Mac: Option) + Click**: Remove sectors from the selection.
- **Click + Drag**: Move selected sectors without moving embedded sectors.
- **Click + Cmd / Ctrl + Drag**: Move selected sectors including embedded sectors.
- **Click + Drag (Empty Area)**: Select a rectangular area of sectors.
- **Delete Key**: Remove selected sectors.
- **Escape Key**: Clear the selection.

## Assigning Tiles

You can **assign tiles** to selected sectors with the [Apply Tile](../actions/#apply-tile) action.

## Tips and Tricks

### 2D

Use sectors to create logical units, such as a house, and fill them with content using the [Rect Tool](rect) and other sub-sectors. You can move a sector with all its embedded content at once by holding **Command (macOS) / Ctrl**, making sectors convenient logical units of content.

### 3D

Use sectors to create foundations of structures; you can [Extrude](../actions/#extrude) the linedefs of sectors to create walls and build up complex **3D** objects.

### 2D and 3D

Create named sectors as logical units for areas NPCs can move in or as destinations for the [goto](../../reference/scripting_server/#goto) command. Your characters receive [entered](../../reference/events/#entered) and [left](../../reference/events/#left) events whenever they enter or leave a sector, providing a powerful way to interact with the environment.
