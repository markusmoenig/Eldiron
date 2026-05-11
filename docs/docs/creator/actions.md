---
title: "Actions"
sidebar_position: 3
---

Actions do the real work in the **Eldiron Creator**. From maximizing the dock widget to creating geometry or switching cameras. It is a centralized system which only displays actions which are currently applicable (depending on the selected geometry, project tree item and camera).

Actions listed in blue represent camera based actions, red actions are applicable to the current content of the **geometry editor**, while yellow actions are applicable to the content of the **dock widget**.

If the **Automatic** mode is enabled, selecting an action (or changing the parameter of an action) will automatically execute it. If the automatic mode is disabled, you need to click the **Apply** button manually to execute the action. Automatic mode is off by default.

Tile assignment is handled by buttons in the **Tile Picker** dock and operates on either:

- the currently selected geometry material slot, or
- the currently selected action material slot when the active Region action exposes HUD material slots.

---

# Camera Actions

### Editing Camera

*Shortcut: Ctrl/Cmd + 2*

Switch to the top-down 2D editing view while remaining in the current region.

### Orbit Camera

*Shortcut: Ctrl/Cmd + 3*

Enable the orbitable 3D camera for inspecting and placing geometry in the region.

Controls: mouse wheel zooms, right-drag or `Alt`-drag orbits, `Ctrl/Cmd`-drag pans, `Shift` + mouse wheel pans, and arrow keys move the target position. Right-drag uses captured raw mouse motion in the desktop and Xcode/macOS builds so the pointer cannot hit the screen edge while orbiting.

### Iso Camera

*Shortcut: Ctrl/Cmd + 4*

Use the isometric editor camera for layout and readability checks.

Controls: mouse wheel zooms, right-drag, `Alt`-drag, or `Ctrl/Cmd`-drag pans, `Shift` + mouse wheel pans, and arrow keys move the target position.

### First-Person Camera

*Shortcut: Ctrl/Cmd + 5*

Jump into a first-person preview of the region. This also clears any active surface-edit overlay so the scene renders cleanly.

In first-person view, hold the right mouse button and use `WASD` to fly. Release the right mouse button or press `Escape` to return to normal editing. Right-drag uses captured raw mouse motion in the desktop and Xcode/macOS builds so turning is not limited by the screen edge. `Space` still toggles fly navigation as a touchpad-friendly fallback; in that mode the pointer position relative to the center of the view controls looking.

Fly navigation controls:

- move the pointer away from the center of the view to turn and look up/down
- `W` / `S` move forward and backward along the current look direction
- `A` / `D` strafe left and right
- `Space` toggles fly navigation off again
- `Escape` also exits fly navigation

This mode is useful for touchpads as well as mice because it does not require holding a mouse button. While fly navigation is active, normal geometry-editing tool input is suspended so `WASD` can be used for movement instead of tool shortcuts.

### Editing Slice

Offsets the slice plane when in 2D editing without an active surface, letting you peek through layered geometry.

The slice position is not fixed to a small range (useful for tall maps / mountains).

It also defines the **slice height/thickness** (`1..10`, default `2`).  
A higher value includes more geometry around the slice for both preview and selection.

### Direct 3D Geometry

In 3D editor views, Eldiron uses direct Geometry Object editing.

The Object, Vertex, Linedef, and Sector tools operate on whole objects, vertices, edges, and faces when a 3D camera is active.

---

# Geometry Editor Actions

## Create & Select

### Create Linedef

Connect the two selected vertices with a linedef (manual creation to avoid unintended sector auto-detection).

### Create Sector

Form a sector from ≥3 selected vertices (ordered clockwise) or a closed set of selected linedefs. Adds default floor/ceiling surfaces so tiles can be applied immediately.

### Create Center Vertex

Add a vertex at the centroid of each selected sector—handy for arches, props, or snapping guides.

### Split

If a linedef is selected, split it at midpoint. If two vertices are selected, insert a linedef between them.

In direct 3D geometry, `X` splits selected geometry edges at their midpoint. If two non-neighboring vertices on the same face are selected, `X` splits that face along the selected diagonal.

### Toggle Rect Geometry

In 2D view (no surface selected), toggle rectangular placement helpers for geometry edits. The dock state is left unchanged.

## Edit Geometry

> Any `tile_id`-style parameter in actions accepts either:
> - a tile UUID string (v4), or
> - a tile alias string, or
> - a palette index (integer, or numeric string like `"2"`).

### Edit Geometry

*Shortcut: G*

Edit selected direct 3D geometry objects. This action is available in 3D editing views when a geometry object is selected.

Parameters include object name, optional group label, visibility, mesh-collision solidity, and exact object bounds. Turning **Visible** off skips the object in the rendered scene. Turning **Solid** off skips it in mesh collision while keeping it editable in the creator.

### Edit Face Texture

Edit texture placement on selected direct 3D geometry faces, or on every face of selected Geometry Objects. Explicit face selections take priority, so selecting one face on an object edits only that face.
Parameter changes update the selected geometry in the 3D view immediately, so texture adjustments can be judged while editing.

Parameters:

- `offset_x` / `offset_y`: slide the source across the face UVs.
- `scale_x` / `scale_y`: scale the source. Larger values cover more surface area; smaller values repeat more tightly.
- `rotation`: rotate the source in degrees around the face UV center.

### Face Extrude

*Shortcut: Ctrl/Cmd + E*

Extrude selected direct 3D geometry faces by the configured amount. Select one or more faces with the Sector / Face Tool, then use the action parameters to set the extrusion distance.

Extrusion replaces the selected source face with a new cap and connected side faces, so the result stays usable as normal editable geometry instead of leaving an internal duplicate face behind.

### Face Cut Opening

Cut a rectangular opening through the selected direct 3D geometry face and its opposite face. This creates front and back opening loops plus reveal faces, so walls and boxes keep real thickness around windows and doors.

Use this action when a rectangular window or doorway is enough. For custom drawn shapes, use **Create Cutout** with a closed surface-line loop.

### Create Cutout

Convert one or more selected closed 3D surface-line loops into openings through the host geometry object. Draw loops on a selected face with the Linedef / Edge Tool, click any point or segment on a loop to select the connected shape, use **Shift** to add more loops, then run Create Cutout.

Create Cutout uses the actual loop shapes, not only their bounding boxes. It rebuilds the selected front face and the opposite face around the loops, then creates reveal faces through the wall or floor thickness. This is the preferred action for custom windows, holes, vents, floor openings, and non-rectangular cuts.

The host object needs an opposite face in the cut direction. If the object has old duplicate coplanar caps from earlier geometry, the action removes the overlapping cap face while building the cutout.

Create Cutout validates the selection before editing the object. All selected guide components must be closed loops on one host surface. Create Cutout keeps the selected surface-line loops as reusable guide geometry after the openings are created. The Linedef / Edge Tool can reselect those guides on the rebuilt surface ring, so the same host object can receive additional cutouts later. Delete the guides explicitly when they are no longer needed.

### Duplicate Surface Detail

*Shortcut: Ctrl/Cmd + Shift + D*

Duplicate the selected 3D surface-line guide geometry on its host face. The action uses face-local `U` and `V` offsets, so one drawn window, arch, groove guide, or ridge guide can be repeated across the same wall or floor before committing selected loops into real geometry. After a cutout, duplicate a reselected guide to place another matching opening.

### Surface Curve

*Shortcut: Ctrl/Cmd + Shift + C*

Set selected 3D surface-line segments to straight lines or configurable arcs. You can also select two points on the same connected guide to curve the shortest path between them, which keeps the rest of a closed opening shape intact. Positive and negative amounts bend the arc in opposite directions. Curved segments stay editable as surface guides, and Create Cutout, Create Ridge, and Create Groove tessellate them into the resulting geometry.

### Face Inset

*Shortcut: Ctrl/Cmd + I*

Inset selected direct 3D geometry faces by the configured amount. This creates a smaller editable face inside the selected face and keeps surrounding ring faces connected.

### Face Delete

*Shortcut: Delete*

Delete selected direct 3D geometry faces. The boundary vertices remain selected so the opening can be filled again from the Vertex Tool.

### Face Merge

*Shortcut: Ctrl/Cmd + M*

Merge selected connected direct 3D geometry faces into one editable face.

### Face Subdivide

*Shortcut: Ctrl/Cmd + U*

Subdivide selected direct 3D quad faces into smaller editable faces. Newly created child faces stay selected so the action can be repeated quickly. Shared boundary edges also add matching midpoint vertices to neighboring faces, keeping subdivided faces attached to the surrounding mesh.

### Create Ridge

Convert selected 3D surface lines into persistent raised ridge geometry. Draw surface lines with the Linedef / Edge Tool, click a point or segment to select the connected shape, then set the ridge shape, width, and height in the action parameters.

Ridges are generated as a separate Geometry Object and are selected after creation. By default they inherit the tile, color, tilegraph, or nodegraph source from the host face, which makes small surface details usable without manually painting each tiny face.

Shapes:

- **Box**: blocky rectangular ridge for lips, raised mortar, and retro tile-like detail.
- **Triangle**: sharp triangular ridge for bevel-like decoration and carved-looking strokes.

### Create Groove

Convert selected 3D surface lines into persistent recessed groove geometry. It uses the same connected surface-line selection workflow and the same shape, width, and height parameters as Create Ridge.

Grooves are the inverted version of ridges. They create depressed line detail for carved seams, block patterns, mortar cuts, and similar surface relief. Like ridges, they become persistent Geometry Objects and inherit the host face source by default.

Shapes:

- **Box**: a flat-bottom groove for mortar lines, seams, and block cuts.
- **Triangle**: a sharp V-shaped groove for carved decoration.

### Duplicate

*Shortcut: Ctrl/Cmd + D*

Duplicate the current selection with XYZ offsets.

For direct 3D geometry objects, Duplicate remembers the last used geometry offset so repeated duplication can quickly build rows of objects.
- `x`: horizontal world offset on the map X axis.
- `y`: vertical offset (applied to vertex height / elevation).
- `z`: depth offset on the map Z axis.
- `[sector].connect`: when duplicating sectors, auto-create connector sectors between old and new boundaries (useful for walls/bridges between levels).

### Toggle Editor Post

Toggle editor-only 3D post-processing preview. This affects the editor viewport only and does not change project render settings.

### Toggle Editor Lighting

Toggle editor-only 3D lighting preview. When off, the editor viewport disables sun and shadow overrides and uses full ambient light for cleaner geometry editing. This affects the editor viewport only and does not change project render settings.

### Edit Vertex

Edit one selected 2D map vertex or one selected 3D Geometry Object vertex.

For 2D vertices, the action includes two parameter groups:
- `[action]`: `name`, `x`, `y`, `z`
- `[billboard]`: `tile_id`, `size`

Parameter meaning:
- `[action].name`: display name for the vertex.
- `[action].x`, `[action].z`: planar map position.
- `[action].y`: vertex elevation (height).
- `[billboard].tile_id`: optional sprite/billboard tile attached to the vertex.
- `[billboard].size`: billboard size scale.

For 3D Geometry Object vertices, the same position fields edit the selected vertex in world coordinates. This is useful for exact placement when grid snapping is not precise enough.

This writes to vertex position/name plus billboard properties `source` and `source_size`.

### Edit Linedef

2D-only action.

Single/multi-linedef editor:
- `[action]`: `name`

Parameter meaning:
- `[action].name`: linedef name.

### Edit Sector

2D-only action.

Single-sector editor:
- `[action]`: `name`, `item`, `visible`

Parameter meaning:
- `[action].name`: sector name.
- `[action].item`: optional item/source reference associated with the sector.
- `[action].visible`: editor/runtime visibility flag.

---

# Dock Actions (Tiles)

All tile actions require the **Tiles** dock to be active.

### Edit / Maximize

*Shortcut: Ctrl/Cmd + [*

Maximize the active dock.

For the **Tile Picker**, this is context-sensitive:

- if a tile is selected, it opens the **pixel tile editor**
- if a node group is selected, it opens the **tile node graph editor**

### Minimize

*Shortcut: Ctrl/Cmd + ]*

Restore a maximized dock to normal size.

## Palette

### Clear Palette

Empty the palette and reapply it project-wide. Undo is supported.

### Import Palette

Open a file requester for Paint.NET `.txt` palettes; load colors into the project palette at the currently selected palette index. Undo is supported.

## Tiles

### New Tile

Create a tile sized 8–64 px with 1–8 frames, filled with the currently selected palette color.

### Duplicate Tile

Clone the currently selected tile, including all frames and material data.

### Copy Tile ID

Copy the selected tile’s UUID to both the internal and system clipboard.

### Edit Tile Meta

Set tile *role*, *blocking* flag (2D collisions), *alias*, and optional procedural generator hints for the currently selected tile in the tile picker.

The alias can then be used anywhere a `tile_id`-style tile source is accepted, alongside UUIDs and palette indices.

Procedural tile metadata is stored as:

```toml
[procedural]
style = "stone"
kind = "floor"
weight = 1
```

Supported `kind` values are `floor`, `wall`, `entrance`, and `exit`. Use `none` in the editor selector for non-procedural tiles. Gameplay objects such as doors, traps, and potions should be generated as item instances from the region `[procedural.items.*]` settings, not as tile kinds.

Procedural tile metadata is consumed by **Build Procedural**, which is available in the 2D editor view. See [Procedural Map Generation](/docs/building_maps/procedural_generation) for the full workflow and [Region Settings: Procedural](/docs/building_maps/region_settings/#procedural) for the matching region-side settings.

### Set Tile Material

*Shortcut: Alt + A*

Apply material values to every pixel of the tile textures. Params: *roughness*, *metallic*, *opacity*, and *emissive*.

### Remap Tile

Map every pixel to the closest palette color while preserving alpha and leaving magenta (255,0,255) transparent pixels untouched.

## Visual Code

### Import Visual Code

Imports Visual Code module JSON via the file requester.
This action has no parameters.

### Export Visual Code

Exports the current Visual Code module JSON via the file requester.
This action has no parameters.

### Copy Visual Code

Copies the current Visual Code module JSON to the clipboard.
Writes to both the internal app clipboard and the system clipboard.
This action has no parameters.

### Paste Visual Code

Pastes Visual Code module JSON from the clipboard into the current Visual Code dock.
Works with either internal app clipboard content or system clipboard text.
This action has no parameters.
