---
title: "Vertex Tool"
sidebar_position: 3
---

The **Vertex Tool** (keyboard shortcut **'V'**) allows you to **select, edit, and delete vertices** in the map.

It is specifically designed for working with **vertices only**.

In 3D views, the Vertex Tool selects vertices on direct geometry objects.

## Selection Modes

- **Click**: Select a vertex.
- **Shift + Click** (on empty area): Create a new vertex.
- **Shift + Click**: Select multiple vertices.
- **Alt (Mac: Option) + Click**: Remove vertices from the selection.
- **Click + Drag**: Move selected vertices.
- **Click + Drag onto another vertex**: Auto-merge moved 3D vertices when they land on the same grid position.
- **Click + Drag (Empty Area)**: Select a rectangular area of vertices.
- **Delete Key**: Remove selected vertices.
- **Escape Key**: Clear the selection.

## 3D Shortcuts

- **X**: Split selected geometry edges when the selected vertices form object edges.
- **M**: Merge selected vertices to their center and rebuild affected faces.
- **F**: Fill a selected vertex boundary with a face.
- **L**: Expand a selected edge into an edge loop on quad geometry.
- **[ / ]**: Move selected vertices vertically by one grid step.

When moving or merging vertices creates a concave or non-planar face, Eldiron automatically resolves the affected face into triangles so the mesh remains valid.
