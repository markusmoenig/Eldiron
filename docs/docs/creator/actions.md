---
title: "Actions"
sidebar_position: 3
---

Actions do the real work in the **Eldiron Creator**. From maximizing the dock widget to creating geometry or switching cameras. It is a centralized system which only displays actions which are currently applicable (depending on the selected geometry, project tree item and camera).

Actions listed in blue represent camera based actions, red actions are applicable to the current content of the **geometry editor**, while yellow actions are applicable to the content of the **dock widget**.

If the **Automatic** mode is enabled, selecting an action (or changing the parameter of an action) will automatically execute it. If the automatic mode is disabled, you need to click the **Apply** button manually to execute the action. Automatic mode is off by default.

Tile assignment is handled by buttons in the **Tile Picker** dock and operates on either:

* the currently selected geometry material slot, or
* the currently selected action material slot when the active Region action exposes HUD material slots.

---

# Camera Actions

### Direct 3D Geometry

In 3D editor views, Eldiron uses direct Geometry Object editing.

Tools:

* **Object Tool**: edits whole Geometry Objects.
* **Vertex Tool**: edits Geometry Object vertices.
* **Linedef / Edge Tool**: edits Geometry Object edges and surface-line guides.
* **Sector / Face Tool**: edits Geometry Object faces.

Options: none.

### Editing Camera

*Shortcut: Ctrl/Cmd + 2*

Switch to the top-down 2D editing view while remaining in the current region.

Options: none.

### Editing Slice

Offsets the slice plane when in 2D editing without an active surface, letting you peek through layered geometry.

The slice position is not fixed to a small range, which is useful for tall maps and mountains.

Options:

* `slice_pos`: slice plane position.
* `slice_height`: slice height/thickness (`1..10`, default `2`), which includes more geometry around the slice for both preview and selection.

### First-Person Camera

*Shortcut: Ctrl/Cmd + 5*

Jump into a first-person preview of the region. This also clears any active surface-edit overlay so the scene renders cleanly.

In first-person view, hold the right mouse button and use `WASD` to fly. Release the right mouse button or press `Escape` to return to normal editing. Right-drag uses captured raw mouse motion in the desktop and Xcode/macOS builds so turning is not limited by the screen edge. `Space` is only a touchpad fallback for the older pointer-from-center fly mode.

Controls:

* Hold right mouse button + mouse movement: fly look.
* `W` / `S` while flying: move forward and backward along the current look direction.
* `A` / `D` while flying: strafe left and right.
* Release right mouse button: exit normal fly navigation.
* `Space`: optional touchpad fallback; press again or press `Escape` to exit that mode.
* `Escape`: exit fly navigation.

While fly navigation is active, normal geometry-editing tool input is suspended so `WASD` can be used for movement instead of tool shortcuts.

Options: none.

### Iso Camera

*Shortcut: Ctrl/Cmd + 4*

Use the isometric editor camera for layout and readability checks.

Controls:

* Mouse wheel: zoom.
* Right-drag, `Alt`-drag, or `Ctrl/Cmd`-drag: pan.
* `Shift` + mouse wheel: pan.
* Arrow keys: move the target position.

Options:

* `azimuth`: isometric camera yaw in degrees.
* `elevation`: isometric camera pitch in degrees.
* `scale`: isometric camera scale.

### Orbit Camera

*Shortcut: Ctrl/Cmd + 3*

Enable the orbitable 3D camera for inspecting and placing geometry in the region.

Controls:

* Mouse wheel: zoom.
* Right-drag or `Alt`-drag: orbit.
* `Ctrl/Cmd`-drag: pan.
* `Shift` + mouse wheel: pan.
* Arrow keys: move the target position.

Right-drag uses captured raw mouse motion in the desktop and Xcode/macOS builds so the pointer cannot hit the screen edge while orbiting.

Options: none.

---

# Geometry Editor Actions

## Create & Select

### Build Procedural

Build the current region from its procedural region settings. This action is available in the 2D editor view for regions.

Options:

* No action parameters.

Region settings used:

* `enabled`: must be true in the region config.
* `generator`: currently expects `connected_rooms`.
* `mode`: currently expects `2d`.

### Create Center Vertex

Add a vertex at the centroid of each selected sector. This is handy for arches, props, or snapping guides.

Options: none.

### Create Linedef

Connect the two selected vertices with a linedef (manual creation to avoid unintended sector auto-detection).

Options: none.

### Create Sector

Form a sector from three or more selected vertices (ordered clockwise) or a closed set of selected linedefs. Adds default floor/ceiling surfaces so tiles can be applied immediately.

Options: none.

### Make Sector Rectangular

Move the selected four-corner sector vertices onto the sector's bounding rectangle.

Options: none.

### Split

If a linedef is selected, split it at midpoint. If two vertices are selected, insert a linedef between them.

In direct 3D geometry, `X` splits selected geometry edges at their midpoint. If two non-neighboring vertices on the same face are selected, `X` splits that face along the selected diagonal.

Options: none.

### Toggle Rect Geometry

In 2D view (no surface selected), toggle rectangular placement helpers for geometry edits. The dock state is left unchanged.

Options: none.

## Edit Geometry

> Any `tile_id`-style parameter in actions accepts either:
> - a tile UUID string (v4), or
> - a tile alias string, or
> - a palette index (integer, or numeric string like `"2"`).

### Create Box

Create a new direct 3D Geometry Object box. With no face or edge selection, the box is created at the current 3D placement position. With a selected face, it is aligned to that face. With a selected horizontal edge on a vertical face, it creates a wall-like box connected to the face below the edge.

Options:

* `width`: box size on the world X axis, or the derived face-local width when aligned to a face or edge.
* `height`: box size on the world Y / elevation axis.
* `depth`: box size on the world Z axis, or grid-step thickness for edge-created wall boxes.
* Selected face: aligns the box to the selected face.
* Selected edge: aligns the box to the adjacent face below the edge, using the current grid step for thickness.

### Create Cutout

Convert one or more selected closed 3D surface-line loops into openings through the host geometry object. Draw loops on a selected face with the Linedef / Edge Tool, click any point or segment on a loop to select the connected shape, use **Shift** to add more loops, then run Create Cutout.

Create Cutout uses the actual loop shapes, not only their bounding boxes. It rebuilds the selected front face and the opposite face around the loops, then creates reveal faces through the wall or floor thickness. This is the preferred action for custom windows, holes, vents, floor openings, and non-rectangular cuts.

Options:

* Selected closed surface-line loops: all selected guide components must be closed loops on one host surface.
* Host object: needs an opposite face in the cut direction.
* Existing guides: kept as reusable guide geometry after the openings are created.
* Old duplicate caps: overlapping coplanar cap faces are removed while building the cutout.

### Create Face

Convert one or more selected closed 3D surface-line loops into new selectable faces on the host geometry object without cutting through the object. This is useful for drawing a floor plan or footprint on an existing face, creating a coplanar face from it, then extruding that new face into walls, raised trim, platforms, or other connected blockout geometry.

Options:

* Selected closed surface-line loops: uses the same closed-loop selection validation as Create Cutout.
* Host face: is not rebuilt or cut through.
* New face selection: the created face is selected after creation so it can be extruded immediately.

### Create Groove

Convert selected 3D surface lines into persistent recessed groove geometry. It uses the same connected surface-line selection workflow and the same shape, width, and height parameters as Create Ridge.

Grooves are the inverted version of ridges. They create depressed line detail for carved seams, block patterns, mortar cuts, and similar surface relief. Like ridges, they become persistent Geometry Objects and inherit the host face source by default.

Options:

* **Shape: Box**: flat-bottom groove for mortar lines, seams, and block cuts.
* **Shape: Triangle**: sharp V-shaped groove for carved decoration.
* **Shape: Round**: rounded U-shaped groove for softer carved lines, vines, roots, and other organic wall detail.
* `ridge_shape`: `Box`, `Triangle`, or `Round`.
* `ridge_width`: groove width on the selected face.
* `ridge_height`: groove depth into the host surface.
* Source material: inherited from the host face by default.

### Create Pattern

Create patterns on the selected 3D face. In **guide** mode, the action creates editable surface-line guides without directly changing topology. In **relief** mode, the same pattern creates generated raised surface geometry immediately, with separate foreground/background material slots.

Options:

* `mode`: select `guide` or `relief`.
* `pattern`: select `disc`, `triangle`, `quad`, `line`, `tile`, or `cobble`.
* `sequence`: optional comma-separated pattern sequence such as `disc,triangle`; leave empty to use `pattern`.
* `repeat`: when off, creates one centered stamp; when on, repeats across the selected face.
* `interleave`: offsets every second repeated row by half the X spacing. A brick layout is `pattern = "tile"` with `interleave = true`.
* `[shape].scale`: overall stamp size.
* `[shape].rotation`: stamp rotation in degrees.
* `[shape].margin`: inset margin used when fitting shapes inside the selected face.
* `[shape].sides`: side count for disc-like shapes.
* `[shape].roundness`: cobble corner roundness, from squarer stones to softer rounded stones.
* `[shape].jitter`: cobble size and placement variation.
* `[shape].seed`: cobble variation seed.
* `[spacing].x`: horizontal spacing between repeated stamps.
* `[spacing].y`: vertical spacing between repeated stamps.
* `[relief].height`: raised pattern height in relief mode.
* `[relief].height_jitter`: per-stamp height variation in relief mode.
* `[relief].dome`: rounded top amount in relief mode.
* `[relief].edge_depth`: base edge offset for generated relief geometry.
* `[relief].color_jitter`: palette-index variation for the generated pattern material.
* `[fit].rows`: row count; `0` lets the action compute the count from the face and spacing.
* `[fit].columns`: column count; `0` lets the action compute the count from the face and spacing.

Pattern notes:

* `guide` mode selects the created surface details after creation so they can be committed with **Create Face**, **Create Cutout**, **Create Ridge**, or **Create Groove**.
* `relief` mode creates a generated non-solid Geometry Object and selects it after creation.
* While Create Pattern is selected, the minimap previews the current pattern outline on the selected face before applying.
* The HUD exposes **PATTERN** and **BACKGROUND** material slots while Create Pattern is active. Applying a tile or palette color to those slots controls generated relief material and optional host-face background material.
* `tile`: creates a regular grid guide in guide mode and rectangular raised cells in relief mode.
* `tile` plus `interleave`: creates a staggered brick-like layout.
* `cobble`: creates repeated irregular rounded closed loops in guide mode and rounded raised cobbles in relief mode.
* Repeated patterns are centered in the remaining space and skip stamps that do not fit inside the actual face polygon.
* A fixed row or column count is useful for decorative one-row patterns, such as alternating disc and triangle cutouts.

### Create Ridge

Convert selected 3D surface lines into persistent raised ridge geometry. Draw surface lines with the Linedef / Edge Tool, click a point or segment to select the connected shape, then set the ridge shape, width, and height in the action parameters.

Ridges are generated as a separate Geometry Object and are selected after creation. By default they inherit the tile, color, tilegraph, or nodegraph source from the host face, which makes small surface details usable without manually painting each tiny face.

Options:

* **Shape: Box**: blocky rectangular ridge for lips, raised mortar, and retro tile-like detail.
* **Shape: Triangle**: sharp triangular ridge for bevel-like decoration and carved-looking strokes.
* **Shape: Round**: rounded raised stroke for vines, roots, cables, and softer trim.
* `ridge_shape`: `Box`, `Triangle`, or `Round`.
* `ridge_width`: ridge width on the selected face.
* `ridge_height`: ridge height above the host surface.
* Source material: inherited from the host face by default.

### Cut Profile

Cut a repeated profile into the whole selected Geometry Object. The first profile is `crenellation`, useful for castle battlements: it keeps the lower wall continuous and rebuilds the top into centered merlon blocks with crenel gaps.

Options:

* `profile`: currently `crenellation`.
* `axis`: `auto`, `x`, or `z`; `auto` uses the longest horizontal object axis.
* `height`: vertical cut depth down from the object top.
* `merlon`: width of each solid battlement block.
* `crenel`: width of each gap between battlement blocks.

### Cut Stairs

Cut a stair profile into one selected Geometry Object. Select one top face and one adjacent side face, then run the action. The top face defines the stair run, the side face defines the rise, and the result remains a single editable object.

Options:

* `step_height`: target height for each stair step; the action derives the step count and adjusts the actual height to fit.
* `landing`: distance on the back of the selected top face to leave flat behind the stairs.

### Duplicate

*Shortcut: Ctrl/Cmd + D*

Duplicate the current selection with XYZ offsets.

For direct 3D geometry objects, Duplicate remembers the last used geometry offset so repeated duplication can quickly build rows of objects. The duplicated objects become the active object-level selection so they can be moved together immediately and undone as one map edit.

Options:

* `x`: horizontal world offset on the map X axis.
* `y`: vertical offset (applied to vertex height / elevation).
* `z`: depth offset on the map Z axis.
* `[sector].connect`: when duplicating sectors, auto-create connector sectors between old and new boundaries, useful for walls or bridges between levels.

### Duplicate Surface Detail

*Shortcut: Ctrl/Cmd + Shift + D*

Duplicate the selected 3D surface-line guide geometry on its host face. The action uses face-local `U` and `V` offsets, so one drawn window, arch, groove guide, or ridge guide can be repeated across the same wall or floor before committing selected loops into real geometry. After a cutout, duplicate a reselected guide to place another matching opening.

Options:

* `surface_detail_u`: face-local horizontal offset.
* `surface_detail_v`: face-local vertical offset.

### Edit Face Texture

Edit texture placement on selected direct 3D geometry faces, or on every face of selected Geometry Objects. Explicit face selections take priority, so selecting one face on an object edits only that face. Parameter changes update the selected geometry in the 3D view immediately, so texture adjustments can be judged while editing.

Shortcuts for selected textured faces:

* `Arrow keys`: adjust texture offset.
* `Shift + Left / Right`: adjust texture rotation.
* `Ctrl/Cmd + Arrow keys`: adjust texture scale.

Options:

* `texture_offset_x`: slide the source horizontally across the face UVs.
* `texture_offset_y`: slide the source vertically across the face UVs.
* `texture_scale_x`: scale the source horizontally. Larger values cover more surface area; smaller values repeat more tightly.
* `texture_scale_y`: scale the source vertically. Larger values cover more surface area; smaller values repeat more tightly.
* `texture_rotation`: rotate the source in degrees around the face UV center.

### Edit Geometry

Edit selected direct 3D geometry objects. This action is available in 3D editing views when a geometry object is selected.

Options:

* `[metadata].name`: object name used for scripts and editor organization.
* `[metadata].group`: optional group label.
* `[metadata].item`: optional item/handler metadata for this 3D area. When set to a valid Item class, the game server creates a static item linked to this Geometry Object.
* `[metadata].area`: marks named objects for sector-style script destinations.
* `[metadata].hide_iso`: fades the object out while the player is inside that area in isometric gameplay.
* `[metadata].visible`: initial object render visibility.
* `[metadata].solid`: initial object mesh collision state.
* `[geometry].x`: object bounds center X.
* `[geometry].y`: object bounds center Y / elevation.
* `[geometry].z`: object bounds center Z.
* `[geometry].width`: object bounds width.
* `[geometry].height`: object bounds height.
* `[geometry].depth`: object bounds depth.

Scripted area behavior:

* If `[metadata].item` creates a static item for the Geometry Object, `set_attr("visible", false)` / `set_attr("visible", true)` on that item hides or shows the backing 3D object.
* `set_attr("blocking", false)` / `set_attr("blocking", true)` on that item updates the backing object's solidity and rebuilds runtime collision/navigation.
* Hidden objects remain present in the scene data so scripts can reveal them later.

### Edit Linedef

2D-only action.

Options:

* `[action].name`: linedef name.

### Edit Sector

2D-only action.

Options:

* `[action].name`: sector name.
* `[action].item`: optional item/source reference associated with the sector.
* `[action].visible`: editor/runtime visibility flag.

### Edit Vertex

Edit one selected 2D map vertex or one selected 3D Geometry Object vertex.

For 3D Geometry Object vertices, the same position fields edit the selected vertex in world coordinates. This is useful for exact placement when grid snapping is not precise enough.

Options:

* `[action].name`: display name for the vertex.
* `[action].x`: planar map X position.
* `[action].y`: vertex elevation / height.
* `[action].z`: planar map Z position.
* `[billboard].tile_id`: optional sprite/billboard tile attached to the vertex.
* `[billboard].size`: billboard size scale.

This writes to vertex position/name plus billboard properties `source` and `source_size`.

### Face Cut Opening

Cut a rectangular opening through the selected direct 3D geometry face and its opposite face. This creates front and back opening loops plus reveal faces, so walls and boxes keep real thickness around windows and doors.

Use this action when a rectangular window or doorway is enough. For custom drawn shapes, use **Create Cutout** with a closed surface-line loop.

Options:

* `cut_opening_width`: opening width on the selected face.
* `cut_opening_height`: opening height on the selected face.
* Opposite face: required so the opening can cut through real object thickness.

### Face Delete

*Shortcut: Delete*

Delete selected direct 3D geometry faces. The boundary vertices remain selected so the opening can be filled again from the Vertex Tool.

Options: none.

### Face Extrude

*Shortcut: Ctrl/Cmd + E*

Extrude selected direct 3D geometry faces by the configured amount. Select one or more faces with the Sector / Face Tool, then use the action parameters to set the extrusion distance.

Extrusion replaces the selected source face with a new cap and connected side faces, so the result stays usable as normal editable geometry instead of leaving an internal duplicate face behind.

Options:

* `extrude_amount`: extrusion distance along the selected face normal.

### Face Inset

*Shortcut: Ctrl/Cmd + I*

Inset selected direct 3D geometry faces by the configured amount. This creates a smaller editable face inside the selected face and keeps surrounding ring faces connected.

Options:

* `inset_amount`: inset distance from the selected face boundary.

### Face Merge

*Shortcut: Ctrl/Cmd + M*

Merge selected connected direct 3D geometry faces into one editable face.

Options: none.

### Face Subdivide

*Shortcut: Ctrl/Cmd + U*

Subdivide selected direct 3D quad faces into smaller editable faces. Newly created child faces stay selected so the action can be repeated quickly. Shared boundary edges also add matching midpoint vertices to neighboring faces, keeping subdivided faces attached to the surrounding mesh.

Options: none.

### Filter Geometry

Choose which editor geometry remains visible while editing.

Options:

* `editing_geo_filter_mode`: `All` shows normal editor geometry.

### Surface Curve

*Shortcut: Ctrl/Cmd + Shift + C*

Set selected 3D surface-line segments to straight lines or configurable arcs. You can also select two points on the same connected guide to curve the shortest path between them, which keeps the rest of a closed opening shape intact. Curved segments stay editable as surface guides, and Create Cutout, Create Ridge, and Create Groove tessellate them into the resulting geometry.

Options:

* `curve_mode`: `Line` or `Arc`.
* `curve_amount`: curve strength. Positive and negative values bend the arc in opposite directions.

### Surface Noise

Apply procedural surface noise to the selected direct 3D geometry faces. The action exposes a **NOISE** HUD material slot; use the Tiles or Palette dock's Apply/Clear controls to assign or clear that slot before applying the action.

The noise is stored on each selected face and uses object/world-space coordinates for evaluation, so adjoining faces with matching noise settings can continue around corners instead of restarting per face.

Options:

* **NOISE material slot**: tile or palette color used as the noise material.
* Empty **NOISE** slot: clears noise from the selected faces.
* `scale`: noise frequency; higher values create finer detail.
* `amount`: blend strength between the base face material and the noise material.
* `seed`: deterministic noise seed.

### Toggle Editing Geometry

Toggle the editor geometry overlay on or off. This affects the editor viewport and does not change project geometry.

Options: none.

### Toggle Editor Lighting

Toggle editor-only 3D lighting preview. When off, the editor viewport disables sun and shadow overrides and uses full ambient light for cleaner geometry editing. This affects the editor viewport only and does not change project render settings.

Options: none.

### Toggle Editor Post

Toggle editor-only 3D post-processing preview. This affects the editor viewport only and does not change project render settings.

Options: none.

---

# Dock Actions (Tiles)

Tile actions operate from the **Tiles** dock when they need a selected tile or palette source. Selection-based actions such as **Clear Tile** can also operate on selected geometry.

### Edit / Maximize

*Shortcut: Ctrl/Cmd + [*

Maximize the active dock.

For the **Tile Picker**, this is context-sensitive:

* if a tile is selected, it opens the **pixel tile editor**
* if a node group is selected, it opens the **tile node graph editor**

Options: none.

### Minimize

*Shortcut: Ctrl/Cmd + ]*

Restore a maximized dock to normal size.

Options: none.

## Palette

### Clear Palette

Empty the palette and reapply it project-wide. Undo is supported.

Options: none.

### Import Palette

Open a file requester for Paint.NET `.txt` palettes; load colors into the project palette at the currently selected palette index. Undo is supported.

Options: none.

## Tiles

### Apply Tile

Apply the current tile, palette color, tilegraph, or nodegraph source to selected 2D sectors, selected 3D faces, or all faces of selected 3D Geometry Objects.

Options:

* `tile_mode`: `repeat` or `scale` texture application mode.
* Selected sectors: applies to the current 2D floor/ceiling target.
* Selected 3D faces: applies to those faces only.
* Selected 3D objects: applies to all faces when no explicit 3D face selection exists.

### Clear Tile

Clear the assigned tile/source from selected 2D sectors, selected 3D faces, or all faces of selected 3D Geometry Objects.

Options: none.

### Copy Tile ID

Copy the selected tile's UUID to both the internal and system clipboard.

Options: none.

### Duplicate Tile

Clone the currently selected tile, including all frames and material data.

Options: none.

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

Options:

* `role`: tile role used by editor/game systems.
* `blocking`: 2D collision flag.
* `alias`: optional human-readable tile source name.
* `[procedural].style`: generator style hint, such as `stone`.
* `[procedural].kind`: `floor`, `wall`, `entrance`, `exit`, or `none`.
* `[procedural].weight`: generator weighting value.

### New Tile

Create a square tile sized 8-64 px with 1-8 frames, filled with the currently selected palette color.

Options:

* `tile_size`: tile width and height in pixels.
* `tile_frames`: animation frame count.

### Remap Tile

Map every pixel to the closest palette color while preserving alpha and leaving magenta (255,0,255) transparent pixels untouched.

Options:

* `mode`: `nearest`, `floyd-steinberg`, `bayer-4x4`, or `exact`.
* `range`: palette range to use, for example `all`, `2`, or `2-8`.
* `all`: when enabled, remaps all tiles instead of only the selected tile.

### Set Tile Material

*Shortcut: Alt + A*

Apply material values to every pixel of the tile textures.

Options:

* `tile_material_roughness`: surface roughness value.
* `tile_material_metallic`: metallic value.
* `tile_material_opacity`: opacity value.
* `tile_material_emissive`: emissive value.
