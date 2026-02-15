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

Extrude selected linedefs by *distance* (signed) and *angle* (degrees around the edge axis). Can emit front/back faces when surfaces exist.

### Extrude Sector

*Shortcut: Alt + E*

Push selected sectors along their normal. Params: toggle *surface extrusion* (only when a surface is selected), *depth*, and *open back* to leave the rear uncapped for facades or interiors.

### Add Arch

Bend each selected linedef into a quadratic arch. Params: *height* (bulge) and *segments* (curve resolution).

### Recess

*Shortcut: Alt + R*

Cut a recess into the active profile surface. Params: *depth*, *target* front/back face, and cap/jamb tiles chosen via two icons (shows the textures that will be stamped).

### Relief

*Shortcut: Alt + E*

Emboss the active profile surface outward. Params mirror Recess: *height*, *target* (front/back), and cap/side tiles.

### Gate / Door

*Shortcut: Alt + G*

Carve an inset opening in the profile surface and fill it with a tile. Params: *inset*, *repeat/scale* mode, gate/door *tile* icon, and speed/behavior flags (hidden, locked, secret) stored on the sector.

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

Single-vertex editor. Params: *name*, *X/Y/Z*, *terrain control* toggle, *terrain smoothness* radius (wider, smoother terrain influence), *tile* picker for a billboard at the vertex, and *tile size*. Writes `terrain_control`, `smoothness`, `source`, and `source_size` into vertex properties.

### Edit Linedef

Rename the selected linedef (keeps geometry intact).

### Edit Sector

Edit one selected sector. Params: *name*, *item* string (spawned content), *visible* toggle, *terrain mode* (none / exclude / ridge), and ridge shaping params: *ridge height*, *ridge plateau width*, *ridge falloff distance*. Ridge options control crest height, flat top width, and smoothing distance for terrain deformation.

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
