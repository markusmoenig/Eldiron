---
title: "Linedef Tool"
sidebar_position: 3
---

The **Linedef Tool** (keyboard shortcut **'L'**) allows you to **select, edit, and create linedefs** in the map.

Unlike the **Selection Tool**, which can select multiple types of geometry at once, the **Linedef Tool** is specifically designed for working with **linedefs only**. It also includes **creation modes** for quickly building map geometry (sectors).

## Selection Modes

- **Click**: Select a linedef.
- **Shift + Click**: Select multiple linedefs.
- **Alt (Mac: Option) + Click**: Remove linedefs from the selection.
- **Click + Drag**: Move selected linedefs.
- **Click + Drag (Empty Area)**: Select a rectangular area of linedefs.
- **Delete Key**: Remove selected linedefs.
- **Escape Key**: Clear the selection.

## Creation Mode (Manual)

- **Click on free space**: Creates a new **vertex** (or uses an existing one at the click position).
- **Click again on another free space**: Creates a **linedef** between the new vertex and the previous one.
- **Clicking on an existing vertex**: Extends the shape by connecting to the selected vertex.
- **Closing a polygon** (by connecting the last linedef to the starting vertex) **automatically creates a sector**.

This manual mode creates sectors by keeping a history of vertex clicks. You can only close already existing shapes when you click on every vertex in the path.

## Creation Mode (Automatic)

 Hold **Command (macOS) / Ctrl** while clicking on vertices; on every click, the automatic mode checks if it can close an existing polygon. For example, if you have a shape that is not closed, you can add linedefs to this shape to close the shape and create a **sector**.

This mode fails if you have a grid of existing geometry created by the [Rect Tool](rect).
