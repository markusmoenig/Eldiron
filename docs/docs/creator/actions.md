---
title: "Actions"
sidebar_position: 3
---

Actions do the real work in the **Eldiron Creator**. From maximizing the dock widget to creating geometry or switching cameras. It is a centralized system which only displays actions which are currently applicable (depending on the selected geometry, project tree item and camera).

Actions listed in blue represent camera based actions, red actions are applicable to the current content of the **geometry editor**, while yellow actions are applicable to the content of the **dock widget**.

If the **Automatic** mode is enabled, selecting an action (or changing the parameter of an action) will automatically execute it. If the automatic mode is disabled, you need to click the **Apply** button manually to execute the action.

---

# Camera Actions

### Editing Camera

*Shortcut: Ctrl/Cmd + 2*

Switch to the top-down 2D editing view while remaining in the current region.

### Orbit Camera

*Shortcut: Ctrl/Cmd + 3*

Enable the orbitable 3D camera for inspecting and placing geometry in the region.

### Iso Camera

*Shortcut: Ctrl/Cmd + 4*

Use the isometric editor camera for layout and readability checks.

### First-Person Camera

*Shortcut: Ctrl/Cmd + 5*

Jump into a first-person preview of the region. This also clears any active surface-edit overlay so the scene renders cleanly.

### Editing Slice

Offsets the slice plane (-5..5) when in 2D editing without an active surface, letting you peek through layered geometry.

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

### Extrude Linedef

*Shortcut: Alt + E*

Extrude selected linedefs by *distance* (how far the wall is pushed out) and *angle* (degrees around the edge axis).  
Also supports top shaping via a `top` section:
- *style*: `flat`, `crenelated`, `palisade`, or `random`
- *segment_size*: shared segment width used by patterned styles
- *variation*: style intensity (tooth/stake height or random break amount)

This is currently additive (re-applying creates new generated geometry), so use undo or delete old result before re-extruding if needed.

### Create Palisade

Create a non-destructive palisade along selected linedefs. You can re-open the action any time to tweak values.

- `[material].tile_id`: tile used by the palisade.
- `[layout].spacing`: distance between stakes along the linedef.
- `[layout].segment_size`: width of each stake along the linedef.
- `[shape].stake_shape`: `flat`, `square`, or `round`.
- `[shape].depth`: cross-depth/thickness of the stake.
- `[shape].round_segments`: radial segment count for round stakes.
- `[height].base`: main stake height. `0.0` disables generation.
- `[height].variation`: deterministic per-stake height variation.
- `[top].mode`: `flat`, `spike`, `bevel`, or `random` (per stake).
- `[top].height`: extra height used by spike/bevel tops.
- `[lean].amount`: max lean offset.
- `[lean].randomness`: 0..1 multiplier for random lean variation.

### Extrude Sector

*Shortcut: Alt + E*

Push selected sectors along their normal. Params: toggle *surface extrusion* (only when a surface is selected), *depth*, and *open back* to leave the rear uncapped for facades or interiors.

### Add Arch

Bend each selected linedef into a quadratic arch. Params: *height* (bulge) and *segments* (curve resolution).

### Duplicate

Duplicate the current selection with XYZ offsets.
- `x`: horizontal world offset on the map X axis.
- `y`: vertical offset (applied to vertex height / elevation).
- `z`: depth offset on the map Z axis.
- `[sector].connect`: when duplicating sectors, auto-create connector sectors between old and new boundaries (useful for walls/bridges between levels).

### Recess

*Shortcut: Alt + R*

Cut a recess into the active profile surface. Params: *depth*, *target* front/back face, and cap/jamb tiles chosen via two icons (shows the textures that will be stamped).

### Relief

*Shortcut: Alt + E*

Emboss the active profile surface outward. Params mirror Recess: *height*, *target* (front/back), and cap/side tiles.

### Gate / Door

*Shortcut: Alt + G*

Carve an inset opening in the profile surface and fill it with a tile. Params: *inset*, *repeat/scale* mode, gate/door *tile* icon, and speed/behavior flags (hidden, locked, secret) stored on the sector.

### Create Prop

Create/edit parametric props on selected profile sectors (first preset: `table`).

- `[table].create`: enables table generation for the selected sectors.
- `[table].height`: relief amount used for tabletop height.
- `[table].connection_mode`: edge style (`hard`, `smooth`, `bevel`).
- `[table].bevel_segments`: bevel tessellation when using bevel mode.
- `[table].bevel_radius`: bevel size when using bevel mode.
- `[billboard].tile_id`: optional tile applied to table cap/jamb material.

### Set Editing Surface

*Shortcut: Alt + U*

Choose which face of the selected sector is being profiled (front/back/left/right/top/bottom depending on view). This updates overlays in 3D.

### Clear Profile

*Shortcut: Alt + G*

Remove any stored profile operation (`profile_op`) from selected sectors. This restores a flat face.

### Toggle Editing Geometry

*Shortcut: Ctrl/Cmd + T*

Show or hide the 3D editing overlay geometry. Toggling this also refreshes the overlay.

## Object Properties

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
- `[terrain]`: `smooth`, `width`, `falloff_distance`, `falloff_steepness`, `tile_id`, `tile_falloff`

Parameter meaning:
- `[action].name`: linedef name.
- `[terrain].smooth`: enables terrain smoothing/deformation along the linedef corridor.
- `[terrain].width`: full-effect corridor width around the linedef.
- `[terrain].falloff_distance`: distance beyond width where deformation fades out.
- `[terrain].falloff_steepness`: falloff curve sharpness (higher = harder edge).
- `[terrain].tile_id`: optional road tile for this corridor.
- `[terrain].tile_falloff`: texture blend distance from road tile into surrounding terrain.

### Edit Sector

Single-sector editor:
- `[action]`: `name`, `item`, `visible`
- `[terrain]`: `terrain`, `ridge_height`, `ridge_plateau`, `ridge_falloff`, `tile_id`, `tile_falloff`
- `[iso]`: `hide_on_enter` pattern list

Parameter meaning:
- `[action].name`: sector name.
- `[action].item`: optional item/source reference associated with the sector.
- `[action].visible`: editor/runtime visibility flag.
- `[terrain].terrain`: terrain mode (`None`, `Exclude`, `Ridge`).
- `[terrain].ridge_height`: ridge elevation above base terrain.
- `[terrain].ridge_plateau`: flat top width of the ridge before falloff starts.
- `[terrain].ridge_falloff`: distance over which ridge height fades to surrounding terrain.
- `[terrain].tile_id`: optional tile used for ridge terrain texturing.
- `[terrain].tile_falloff`: blend distance from ridge tile into neighboring terrain tiles.
- `[iso].hide_on_enter`: wildcard sector-name patterns to hide while the player is inside this sector in iso gameplay preview.

---

# Dock Actions (Tiles)

All tile actions require the **Tiles** dock to be active.

### Edit / Maximize

*Shortcut: Ctrl/Cmd + [*

Maximize the active dock. The Tile Picker reveals the pixel editor when maximized.

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

Set tile *role*, *blocking* flag (2D collisions), and *tags* for the currently selected tile in the tile picker.

### Set Tile Material

*Shortcut: Alt + A*

Apply material values to every pixel of the tile textures. Params: *roughness*, *metallic*, *opacity*, and *emissive*.

### Remap Tile

Map every pixel to the closest palette color while preserving alpha and leaving magenta (255,0,255) transparent pixels untouched.

### Clear Tile

Remove tile sources from selected sectors (clears the `source` property).

### Apply Tile

*Shortcut: Alt + A*

Assign the selected tile to the selected sectors. Param: *mode* = repeat or scale. This applies to floor or ceiling depending on the active HUD icon.

### Import VCode / Export VCode

Round-trip tile and pixel-editor data via VCode modules using the file requester.
