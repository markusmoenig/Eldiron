---
title: "Sector / Face Tool"
sidebar_position: 5
---

The **Sector / Face Tool** (keyboard shortcut **'E'**) selects and edits sectors in 2D views and direct geometry faces in 3D views.

In 2D, it is specifically designed for working with **sectors only**. Unlike the **Linedef / Edge Tool**, it does not include a creation mode, as sectors are automatically formed when a closed shape is created.

In 3D, it selects faces on direct geometry objects.

## Selection Modes

- **Click**: Select a sector.
- **Shift + Click**: Select multiple sectors.
- **Alt (Mac: Option) + Click**: Remove sectors from the selection.
- **Click + Drag**: Move selected sectors without moving embedded sectors.
- **Click + Cmd / Ctrl + Drag**: Move selected sectors including embedded sectors.
- **Click + Drag (Empty Area)**: Select a rectangular area of sectors.
- **Delete Key**: Remove selected sectors.
- **Escape Key**: Clear the selection.

## 3D Shortcuts

- **Cmd/Ctrl+E**: Run the Face Extrude action. Use the action parameters to set the extrusion amount.
- **Face Cut Opening action**: Cut a rectangular opening through the selected face and its opposite face.
- **Create Cutout action**: Cut a selected closed surface-line loop through the host object.
- **Cmd/Ctrl+U**: Run the Face Subdivide action on selected quad faces.
- **Cmd/Ctrl+I**: Run the Face Inset action. Use the action parameters to set the inset amount.
- **Cmd/Ctrl+M**: Run the Face Merge action on selected connected faces.
- **T**: Apply the current tile, color, tilegraph, or nodegraph source.
- **+ / -**: Push or pull selected faces along their normals.
- **[ / ]**: Move selected faces vertically by one grid step.
- **Delete**: Delete selected faces. Boundary vertices remain selected so the opening can be filled with **F** in vertex editing.

## Assigning Tiles

You can **assign tiles** to selected sectors with the **Apply Tile** button in the **Tile Picker** dock.

## Authoring

With **Authoring** mode enabled, the lower dock shows the Authoring editor instead of the tile picker for selected sectors. This is where you enter room metadata such as:

```toml
title = ""
description = """
"""
```

This metadata can be used for room descriptions and text-adventure style presentation.

## Tips and Tricks

### 2D

Use sectors to create logical units, such as a house, and fill them with content using the [Rect Tool](rect) and other sub-sectors. You can move a sector with all its embedded content at once by holding **Command (macOS) / Ctrl**, making sectors convenient logical units of content.

### 3D

Use the Sector / Face Tool to edit faces on direct geometry objects. Select a face and press **Cmd/Ctrl+E** to run Face Extrude, **Cmd/Ctrl+U** to run Face Subdivide, **Cmd/Ctrl+I** to run Face Inset, **Cmd/Ctrl+M** to merge connected faces, or **T** to apply the current tile source. Explicit face selections take priority over object selections when applying or clearing tile and palette sources.

Face Subdivide keeps the newly created child faces selected. When a selected quad shares an edge with an unselected face, the subdivision adds matching midpoint vertices to the neighboring face boundary so the mesh stays attached.

For custom surface details, select the host face first, then switch to the **Linedef / Edge Tool** to draw surface lines. Closed surface-line loops can become cutouts, while open or closed selected surface lines can become ridges or grooves.

### 2D and 3D

Create named sectors as logical units for areas NPCs can move in or as destinations for the [goto](../../reference/scripting_server/#goto) command. Your characters receive [entered](../../reference/events/#entered) and [left](../../reference/events/#left) events whenever they enter or leave a sector, providing a powerful way to interact with the environment.
