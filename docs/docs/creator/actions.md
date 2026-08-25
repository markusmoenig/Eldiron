---
title: "Actions"
sidebar_position: 3
---

Actions do the real work in **Eldiron Creator**, from creating geometry to editing metadata or switching cameras. The centralized action system only displays actions that are currently applicable to the selected geometry, project tree item, tool, and camera.

Open the **Actions** page in the right sidebar to see the available actions and the parameters of the selected action. Use `Tab` and `Shift+Tab` to move between sidebar pages when a text field is not being edited.

A compact row between the action list and its parameters shows the selected action's keyboard shortcut. Actions without a registered shortcut display an em dash.

Action rows are sorted into the **Camera**, **Bake**, **Face**, **Surface**, **Geometry**, **Map**, **Prefab**, **Procedural**, **Tile**, **Palette**, **View**, and **General** groups. Each group uses its own slot in the theme's modular action palette, so a custom theme can change the colors without changing action behavior.

Frequently used camera actions are also exposed as icon shortcuts beside the project tabs. These shortcuts execute the same camera actions without requiring a switch to the Actions sidebar page.

## Dock Controls

Opening and restoring dock editors is handled directly by two small controls beside the camera shortcuts, separated from the camera group by a vertical divider:

- **Edit / Maximize Dock** (frame-corners icon), `Cmd/Ctrl + [`: opens the editor associated with the current dock or maximizes that dock.
- **Restore Dock** (down-caret icon), `Cmd/Ctrl + ]`: returns a maximized dock or dock editor to the normal split layout.

These controls are always available from the project strip when their operation is applicable. They are UI controls rather than Actions-page entries.

If the **Automatic** mode is enabled, selecting an action (or changing the parameter of an action) will automatically execute it. If the automatic mode is disabled, you need to click the **Apply** button manually to execute the action. Automatic mode is off by default.

## Grouped Names And Stable IDs

The sidebar qualifies localized action names with their group. For example, it can display **Camera: 3D Iso Camera** or **Face: Face Extrude**. Existing names that already contain a group, such as **Bake: Render**, are left unchanged instead of receiving a second prefix.

This is a presentation change, not a rename of the underlying action or its saved UUID. Scripts, Scepter clients, shortcuts, and future plugins must use the stable, non-localized dotted ID instead of the visible label:

| Sidebar label | Stable ID |
| --- | --- |
| Camera: 3D Iso Camera | `camera.isometric` |
| Face: Face Extrude | `face.extrude` |
| Bake: Render | `bake.render` |
| Tile: Edit Tile Meta Data | `tile.edit_metadata` |

Use the interactive Console's `actions` command or Scepter's `action.list` command to discover the live catalog, including group, display name, applicability, shortcut, and stable ID. This keeps automation compatible when a label is translated or refined. See [Console](console#actions-and-tools) and [Scepter: Remote Editing](scepter_remote_editing#creator-actions-and-tools).

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

`WASD` moves whenever the viewport is focused; movement does not depend on a separate fly-navigation mode. Hold the right mouse button to look with a physical mouse. `Space` toggles pointer-from-center look for a touchpad. First-person view does not pan.

Controls:

* `W` / `S`: move forward and backward along the current look direction.
* `A` / `D`: strafe left and right.
* Hold right mouse button + mouse movement: look with captured raw mouse motion.
* Release right mouse button: stop mouse look; `WASD` remains available.
* `Space`: toggle touchpad look; press again or press `Escape` to stop looking.
* Mouse wheel: zoom.
* macOS trackpad pinch, or `Command` + two-finger scroll: zoom.

Normal pan gestures are intentionally ignored in First Person because `WASD` owns camera translation.

Options: none.

### Iso Camera

*Shortcut: Ctrl/Cmd + 4*

Use the isometric editor camera for layout and readability checks.

Controls:

* Mouse wheel: zoom.
* Right-drag, `Alt`-drag, or `Ctrl/Cmd`-drag: pan.
* `Shift` + mouse wheel: pan.
* Arrow keys: move the target position.
* macOS trackpad two-finger scroll: pan.
* macOS trackpad pinch: zoom.

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
* `B`: arm one-shot Box Select in the active Object, Face, Edge, or Vertex mode. `Shift` adds and `Alt/Option` removes.
* Click a visible **ViewCube** face: align the camera to that axis.
* macOS trackpad two-finger scroll: pan.
* macOS trackpad pinch, or `Command` + two-finger scroll: zoom.
* macOS primary click-drag on empty space or the ViewCube: orbit.

Right-drag uses captured raw mouse motion in the desktop and Xcode/macOS builds so the pointer cannot hit the screen edge while orbiting. The ViewCube follows the camera and is shared by the region and isolated Prefab editors.

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

### Create Unit Box

Create a fixed `1 × 1 × 1` direct Geometry Object centered on and attached to the selected face or surface. Unlike Create Box, this action keeps the primitive's unit dimensions instead of fitting its in-plane size to the selected surface. Use it when you want a predictable modeling primitive while retaining surface attachment.

Options: none.

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

### Cut Stairs

Cut a stair profile into one selected Geometry Object. Select one top face and one adjacent side face, then run the action. The top face defines the stair run, the side face defines the rise, and the result remains a single editable object.

Options:

* `step_height`: target height for each stair step; the action derives the step count and adjusts the actual height to fit.
* `landing`: distance on the back of the selected top face to leave flat behind the stairs.

### Create Fitted Geometry

Create an independent solid that exactly matches an existing opening, including rectangular, arched, concave, and irregular contours. In the 3D Edge Tool, select one opening rim edge, press **C** to select that closed contour, then press **L** to include the matching contour across the opening depth. Run **Create Fitted Geometry** to copy the selected reveal band, reverse its side faces for the new solid, and triangulate caps on both ends.

The source wall is not modified. The new Geometry Object inherits the opening's transform and adjacent face materials, and is selected immediately so it can be inset, reshaped, painted, or converted into a Prefab.

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

### Clear Surface Detail

Remove every editable 3D surface-line guide attached to the selected Geometry Object faces. In Edge mode, selecting any surface-guide point or segment identifies its host face, so the action can clear every guide on that face while the details remain visible. Face selections remain selected, and the action does not alter 3D Paint, materials, cutouts, or separate geometry generated by Ridge and Groove.

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
* `[material].preset`: high-level object material override. Use `default` to inherit from tile/material defaults.
* `[material].finish`: high-level material finish modifier. Supported values are `natural`, `matte`, `polished`, and `wet`.
* `[geometry].x`: object bounds center X.
* `[geometry].y`: object bounds center Y / elevation.
* `[geometry].z`: object bounds center Z.
* `[geometry].width`: object bounds width.
* `[geometry].height`: object bounds height.
* `[geometry].depth`: object bounds depth.

Supported `preset` values are `default`, `stone`, `dirt`, `wood`, `metal`, `glass`, `water`, `mirror`, `emissive`, `fabric`, `plastic`, `foliage`, `skin`, `bone`, and `wax`.

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

# Prefab Actions

Prefab creation and source management are contextual Object-mode actions. The Prefab Tool and Prefabs dock are used for browsing and placement rather than duplicating these source actions in a second toolbar.

### Create Linked Prefab

Create a reusable, UUID-backed Prefab from the selected Geometry Objects and replace the selection with one linked instance. Editing the Prefab source later updates every linked instance; each placement retains its own transform and runtime state.

### Create Prefab Copy

Create the same reusable Prefab source while keeping the selected region Geometry Objects unchanged.

### Update Prefab Source

Replace the selected project Prefab's source geometry from the current Geometry Object selection while preserving the Prefab UUID and all linked instances.

### Make Prefab Unique

Duplicate the source asset for the selected linked instance. The selected placement is relinked to the new UUID, so later source edits do not affect the other instances.

### Unpack Prefab

Convert selected linked Prefab instances into ordinary editable Geometry Objects and remove their asset links.

---

# Tile And Palette Actions

Tile actions operate from the **Tiles** dock when they need a selected tile or palette source. Selection-based actions such as **Clear Tile** can also operate on selected geometry.

## Palette

### Clear Palette

Empty the editable Art Palette and reapply it project-wide. The Ruleset Palette is not changed. Undo is supported.

Options: none.

### Load Palette

Open a file requester for Paint.NET `.txt` palettes; load colors into the Art Palette at the currently selected palette index. The Ruleset Palette is not changed. Undo is supported.

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

### Duplicate Tile

Clone the currently selected tile, including all frames, generated normal data, and tile metadata.

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

[material]
preset = "stone"
finish = "natural"
```

Supported `kind` values are `floor`, `wall`, `entrance`, and `exit`. Use `none` in the editor selector for non-procedural tiles. Gameplay objects such as doors, traps, and potions should be generated as item instances from the region `[procedural.items.*]` settings, not as tile kinds.

Procedural tile metadata is consumed by **Build Procedural**, which is available in the 2D editor view. See [Procedural Map Generation](/docs/building_maps/procedural_generation) for the full workflow and [Region Settings: Procedural](/docs/building_maps/region_settings/#procedural) for the matching region-side settings.

Material tile metadata stores the tile's default high-level render material. Object material overrides take priority over tile defaults. Art Palette entries have their own material metadata when a surface uses a `PaletteIndex` source. If no preset applies, Eldiron uses the material library's default material.

Supported `preset` values are `default`, `stone`, `dirt`, `wood`, `metal`, `glass`, `water`, `mirror`, `emissive`, `fabric`, `plastic`, `foliage`, `skin`, `bone`, and `wax`.

Options:

* `role`: tile role used by editor/game systems.
* `blocking`: 2D collision flag.
* `alias`: optional human-readable tile source name.
* `[procedural].style`: generator style hint, such as `stone`.
* `[procedural].kind`: `floor`, `wall`, `entrance`, `exit`, or `none`.
* `[procedural].weight`: generator weighting value.

### New Tile

Create a square tile sized 8-64 px with 1-8 frames, filled with the currently selected Art Palette color.

Options:

* `tile_size`: tile width and height in pixels.
* `tile_frames`: animation frame count.

### Remap Tile

Map every pixel to the closest palette color while preserving alpha and leaving magenta (255,0,255) transparent pixels untouched.

Options:

* `mode`: `nearest`, `floyd-steinberg`, `bayer-4x4`, or `exact`.
* `range`: palette range to use, for example `all`, `2`, or `2-8`.
* `all`: when enabled, remaps all tiles instead of only the selected tile.
