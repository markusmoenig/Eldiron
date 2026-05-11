---
title: "Object Tool"
sidebar_position: 2
---

The **Object Tool** (keyboard shortcut **`G`**) selects and edits direct 3D geometry objects.

It is the main tool for the new 3D editing workflow. Use it to select whole geometry objects, move them, resize them, duplicate them, and switch between object-level movement and sizing.

## 3D Selection

- **Click object**: Select a geometry object.
- **Shift + Click object**: Add an object to the selection.
- **Alt (Mac: Option) + Click object**: Remove an object from the selection.
- **Click empty space**: Clear the object selection.
- **Delete**: Delete selected geometry objects.

## Object Gizmos

The 3D HUD shows `MOVE` and `SIZE` controls when the Object Tool is active.

- **M**: Switch the object gizmo to move mode.
- **S**: Switch the object gizmo to size mode.
- **R**: Rotate selected objects 90 degrees around the vertical axis.
- **Shift + R**: Rotate selected objects 90 degrees in the opposite direction.
- **T**: Apply the current tile, palette color, tilegraph, or nodegraph source to every face on the selected objects.
- **Drag axis handle**: Move or resize along that axis.
- **+ / -**: Resize selected objects on the horizontal axes.
- **[ / ]**: Resize selected objects vertically.

Object movement and resizing use the current grid snap step. When multiple objects are selected, dragging a selected object or its move gizmo moves the selection together. Gizmo handles are sized from the current camera view, so they stay usable on both small details and large objects.

## Creating And Duplicating

- **Create Box**: Creates a new geometry box.
- **Create Box with a selected face**: Attaches a box to that face. The new box matches the face size on the in-plane axes and uses the action's remaining size parameter as thickness.
- **Edit Geometry**: Sets exact object bounds, object visibility, mesh-collision solidity, and an optional group label. Solid objects feed their walkable face planes and vertical side barriers into mesh collision.
- **Duplicate**: Duplicate the current object selection with XYZ offsets.
- **Cmd / Ctrl + D**: Duplicate the current selection.

Duplicate remembers the last 3D object offset so repeated duplication can be used for quick blockout placement. After duplicating Geometry Objects, the duplicated objects become the active object-level selection so they can be moved together immediately and undone as one map edit.

## 3D Tool Switching

The existing map tools become direct geometry sub-object tools in 3D:

- **Sector / Face Tool (`E`)**: Select and edit faces.
- **Linedef / Edge Tool (`L`)**: Select and edit edges, and draw surface lines on selected faces.
- **Vertex Tool (`V`)**: Select and edit vertices.

Those tools keep their normal 2D behavior when the editor is in 2D view.

In 3D, switching tools carries the current selection into the new selection mode. Selected objects become all faces, edges, or vertices on those objects. Selected faces become their boundary vertices when switching to the Vertex Tool. Switching a selected face to the Linedef / Edge Tool keeps the face as the surface-line drawing host. Switching back to the Object Tool keeps the owning objects selected.

The editor status bar updates after each 3D selection change and shows the shortcuts currently available for the selected object, face, edge, or vertex.

See the per-tool pages for the shortcuts owned by those modes:

- [Sector / Face Tool](sector): face selection, extrusion, inset, subdivision, merge/delete, face push/pull, and tile assignment.
- [Linedef / Edge Tool](linedef): edge selection, edge splitting/merging, edge-loop selection, surface-line drawing, ridges, grooves, and cutouts.
- [Vertex Tool](vertex): vertex selection, boundary fill, edge splitting/merging, and vertical vertex movement.

## Grid Shortcuts

- **1 ... 6**: Set the current grid snap step to `1`, `1/2`, `1/4`, `1/8`, `1/16`, or `1/32` world units.
- **, / .**: Decrease or increase the 3D grid snap subdivision.

The grid snap step is shared by object movement, face extrusion, vertex moves, resize handles, duplication offsets, and surface-detail editing. In 3D views the visible grid subdivision lines match this same snap step.
