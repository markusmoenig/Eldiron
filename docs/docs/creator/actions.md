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

Controls: mouse wheel zooms, right-drag or `Alt`-drag orbits, `Ctrl/Cmd`-drag pans, `Shift` + mouse wheel pans, and arrow keys move the target position.

### Iso Camera

*Shortcut: Ctrl/Cmd + 4*

Use the isometric editor camera for layout and readability checks.

Controls: mouse wheel zooms, right-drag, `Alt`-drag, or `Ctrl/Cmd`-drag pans, `Shift` + mouse wheel pans, and arrow keys move the target position.

### First-Person Camera

*Shortcut: Ctrl/Cmd + 5*

Jump into a first-person preview of the region. This also clears any active surface-edit overlay so the scene renders cleanly.

In first-person view, hold the right mouse button and use `WASD` to fly. Release the right mouse button or press `Escape` to return to normal editing. `Space` still toggles fly navigation as a touchpad-friendly fallback; in that mode the pointer position relative to the center of the view controls looking.

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

### Face Extrude

*Shortcut: Ctrl/Cmd + E*

Extrude selected direct 3D geometry faces by the configured amount. Select one or more faces with the Sector / Face Tool, then use the action parameters to set the extrusion distance.

### Face Cut Opening

Cut a rectangular opening through the selected direct 3D geometry face and its opposite face. This creates front and back opening loops plus reveal faces, so walls and boxes keep real thickness around windows and doors.

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

Subdivide selected direct 3D quad faces into smaller editable faces.

### Duplicate

*Shortcut: Ctrl/Cmd + D*

Duplicate the current selection with XYZ offsets.

For direct 3D geometry objects, Duplicate remembers the last used geometry offset so repeated duplication can quickly build rows of objects.
- `x`: horizontal world offset on the map X axis.
- `y`: vertical offset (applied to vertex height / elevation).
- `z`: depth offset on the map Z axis.
- `[sector].connect`: when duplicating sectors, auto-create connector sectors between old and new boundaries (useful for walls/bridges between levels).

### Edit Vertex

Single-vertex editor with three parameter groups:
- `[action]`: `name`, `x`, `y`, `z`
- `[terrain]`: `terrain`, `smoothness`, `tile_id`, `tile_falloff`
- `[billboard]`: `tile_id`, `size`

Parameter meaning:
- `[action].name`: display name for the vertex.
- `[action].x`, `[action].z`: planar map position.
- `[action].y`: vertex elevation (height).
- `[terrain].terrain`: marks this vertex as a terrain control point.
- `[terrain].smoothness`: influence radius of this control point (higher = broader hill/valley).
- `[terrain].tile_id`: optional terrain tile override centered on this control point.
- `[terrain].tile_falloff`: blend distance outside the control-point radius for terrain texturing.
- `[billboard].tile_id`: optional sprite/billboard tile attached to the vertex.
- `[billboard].size`: billboard size scale.

This writes to vertex properties `terrain_control`, `smoothness`, `terrain_source`, `terrain_tile_falloff`, `source`, and `source_size`.

### Edit Linedef

Single/multi-linedef editor:
- `[action]`: `name`
- `[terrain]`: `smooth`, `width`, `falloff_distance`, `falloff_steepness`, `tile_id`, `tile_falloff`, `road_organic`

Parameter meaning:
- `[action].name`: linedef name.
- `[terrain].smooth`: enables terrain smoothing/deformation along the linedef corridor.
- `[terrain].width`: full-effect corridor width around the linedef.
- `[terrain].falloff_distance`: distance beyond width where deformation fades out.
- `[terrain].falloff_steepness`: falloff curve sharpness (higher = harder edge).
- `[terrain].tile_id`: optional road tile for this corridor.
- `[terrain].tile_falloff`: texture blend distance from road tile into surrounding terrain.
- `[terrain].road_organic`: organic road mask amount. Higher values add deterministic center wobble, width variation, noisy edges, and patchy breakup while keeping the linedef itself straight.

### Edit Sector

Single-sector editor:
- `[action]`: `name`, `item`, `visible`
- `[terrain]`: `terrain`, `ridge_height`, `ridge_plateau`, `ridge_falloff`, `ridge_subdiv`, `tile_id`, `tile_falloff`, `ridge_water_enabled`, `ridge_water_level`, `ridge_water_tile_id`
- `[iso]`: `hide_on_enter` pattern list

Parameter meaning:
- `[action].name`: sector name.
- `[action].item`: optional item/source reference associated with the sector.
- `[action].visible`: editor/runtime visibility flag.
- `[terrain].terrain`: terrain mode (`None`, `Exclude`, `Ridge`).
- `[terrain].ridge_height`: ridge elevation above base terrain.
- `[terrain].ridge_plateau`: flat top width of the ridge before falloff starts.
- `[terrain].ridge_falloff`: distance over which ridge height fades to surrounding terrain.
- `[terrain].ridge_subdiv`: terrain tessellation quality for ridge areas (`1..8`, higher = smoother ridge geometry, higher cost).
- `[terrain].tile_id`: optional tile used for ridge terrain texturing.
- `[terrain].tile_falloff`: blend distance from ridge tile into neighboring terrain tiles.
- `[terrain].ridge_water_enabled`: enables generation of a water surface for this ridge sector.
- `[terrain].ridge_water_level`: relative water height offset added to `ridge_height` for the generated water surface.
- `[terrain].ridge_water_tile_id`: tile used to render the generated ridge water surface.
- `[iso].hide_on_enter`: wildcard sector-name patterns to hide while the player is inside this sector in iso gameplay preview.

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
