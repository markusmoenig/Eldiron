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
- **Drag axis handle**: Move or resize along that axis.
- **+ / -**: Resize selected objects on the horizontal axes.
- **[ / ]**: Resize selected objects vertically.

Object movement and resizing use the current grid subdivision for snapping.

## Creating And Duplicating

- **Create Box**: Creates a new geometry box.
- **Create Box with a selected face**: Attaches a box to that face. The new box matches the face size on the in-plane axes and uses the action's remaining size parameter as thickness.
- **Duplicate**: Duplicate the current object selection with XYZ offsets.
- **Cmd / Ctrl + D**: Duplicate the current selection.

Duplicate remembers the last 3D object offset so repeated duplication can be used for quick blockout placement.

## 3D Tool Switching

The existing map tools become direct geometry sub-object tools in 3D:

- **Sector / Face Tool (`E`)**: Select and edit faces.
- **Linedef Tool (`L`)**: Select and edit edges.
- **Vertex Tool (`V`)**: Select and edit vertices.

Those tools keep their normal 2D behavior when the editor is in 2D view.

The editor status bar updates after each 3D selection change and shows the shortcuts currently available for the selected object, face, edge, or vertex.

See the per-tool pages for the shortcuts owned by those modes:

- [Sector / Face Tool](sector): face selection, extrusion, inset, subdivision, merge/delete, face push/pull, and tile assignment.
- [Linedef Tool](linedef): edge selection, edge splitting, and edge-loop selection.
- [Vertex Tool](vertex): vertex selection, boundary fill, edge splitting, and vertical vertex movement.

## Grid Shortcuts

- **1 ... 0**: Set the current grid subdivision.
- **, / .**: Decrease or increase the 3D grid size.

The grid subdivision is shared with snapping for object movement, face extrusion, vertex moves, and resize handles.
