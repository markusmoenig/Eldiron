---
title: "Linedef / Edge Tool"
sidebar_position: 4
---

The **Linedef / Edge Tool** (keyboard shortcut **'L'**) allows you to **select, edit, and create linedefs** in 2D maps and **select/edit edges** on direct 3D geometry objects.

In 2D views it works with **linedefs** and includes **creation modes** for quickly building map geometry (sectors).

In 3D views, the same tool becomes the edge/surface-line editing tool for direct geometry objects. It selects existing edges and can draw surface-local line segments on a selected face. These drawn lines are stored as editable points plus segments, so later actions can turn them into cuts, ridges, grooves, or other edge-based surface detail.

## Selection Modes

- **Click**: Select a linedef in 2D or an edge in 3D.
- **Shift + Click**: Add linedefs/edges to the selection.
- **Alt (Mac: Option) + Click**: Remove linedefs/edges from the selection.
- **Click + Drag**: Move selected linedefs in 2D.
- **Click + Drag (Empty Area)**: Select a rectangular area of linedefs in 2D.
- **Delete Key**: Remove selected linedefs in 2D.
- **Escape Key**: Clear the selection in 2D/edge selection, or end the current 3D surface-line polyline while drawing.

## 3D Shortcuts

- **X**: Split selected geometry edges.
- **L**: Expand a selected edge into an edge loop on quad geometry.
- **[ / ]**: Move selected edge vertices vertically by one grid step.

## 3D Surface Lines

To draw a surface line, select a face with the **Sector / Face Tool**, switch to the **Linedef / Edge Tool**, then click points on that face.

- The first click starts a new polyline and immediately creates the first visible surface-line point.
- Each next click creates a straight segment from the previous point.
- Clicking the first point closes the loop and creates the final closing segment.
- Press **Escape** to end the current polyline without clearing the surface-line selection.

Click an existing surface-line point or segment to select it. Drag selected surface-line points or segments to move them on the selected face. Press **Delete** to remove the selected surface-line points or segments.

Surface lines are editor geometry attached to the face. They do not cut or deform the mesh by themselves. Use actions to commit selected lines into real geometry:

- **Create Cutout** converts a selected closed loop into an opening through the host object. The action uses the loop shape, rebuilds the front and opposite faces around it, and creates reveal faces through the thickness.
- **Create Ridge** converts selected surface lines into persistent raised geometry.
- **Create Groove** converts selected surface lines into persistent recessed geometry.

Ridge and Groove can create box-shaped or triangular strokes. They generate a separate Geometry Object, select it after creation, and inherit the tile, color, tilegraph, or nodegraph source from the host face by default.

Use surface lines for custom detail that should be drawn directly on a face: mortar lines, stone blocks, floor seams, decorative raised trim, grooves, vents, custom window cuts, or other geometry-first surface relief.

## Creation Mode (Manual)

- **Click on free space**: Creates a new **vertex** (or uses an existing one at the click position).
- **Click again on another free space**: Creates a **linedef** between the new vertex and the previous one.
- **Clicking on an existing vertex**: Extends the shape by connecting to the selected vertex.
- **Closing a polygon** (by connecting the last linedef to the starting vertex) **automatically creates a sector**.

This manual mode creates sectors by keeping a history of vertex clicks. You can only close already existing shapes when you click on every vertex in the path.

## Creation Mode (Automatic)

 Hold **Command (macOS) / Ctrl** while clicking on vertices; on every click, the automatic mode checks if it can close an existing polygon. For example, if you have a shape that is not closed, you can add linedefs to this shape to close the shape and create a **sector**.

This mode fails if you have a grid of existing geometry created by the [Rect Tool](rect).

## Terrain (Region Maps)

In Region map context, **Edit Linedef** includes a **terrain** section for roads/paths:

- `terrain_smooth`: enable terrain smoothing along the linedef.
- `terrain_source`: optional road tile override for this linedef corridor.
- `terrain_width`: full influence width.
- `terrain_falloff_distance`: additional blend distance beyond the width.
- `terrain_falloff_steepness`: falloff curve sharpness.
- `terrain_tile_falloff`: texture fade distance for smoothed roads (default `1.0`).
- `terrain_road_organic`: organic road mask amount (`0.0` = straight/uniform, `1.0` = stronger center wobble, width variation, noisy edges, and breakup).

The target road height is interpolated from the start/end vertex `z` values.

## Authoring

With **Authoring** mode enabled, the lower dock shows the Authoring editor instead of the tile picker for selected linedefs.

Use the same minimal TOML format:

```toml
title = ""
description = """
"""
```

For linedefs, this is intended for connection or passage descriptions rather than geometric direction names.
