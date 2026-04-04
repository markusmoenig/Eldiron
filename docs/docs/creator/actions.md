---
title: "Actions"
sidebar_position: 3
---

Actions do the real work in the **Eldiron Creator**. From maximizing the dock widget to creating geometry or switching cameras. It is a centralized system which only displays actions which are currently applicable (depending on the selected geometry, project tree item and camera).

Actions listed in blue represent camera based actions, red actions are applicable to the current content of the **geometry editor**, while yellow actions are applicable to the content of the **dock widget**.

If the **Automatic** mode is enabled, selecting an action (or changing the parameter of an action) will automatically execute it. If the automatic mode is disabled, you need to click the **Apply** button manually to execute the action. Automatic mode is off by default.

Tile assignment is no longer handled by `Apply Tile` / `Clear Tile` actions. Those are now buttons in the **Tile Picker** dock and operate on either:

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

### Iso Camera

*Shortcut: Ctrl/Cmd + 4*

Use the isometric editor camera for layout and readability checks.

### First-Person Camera

*Shortcut: Ctrl/Cmd + 5*

Jump into a first-person preview of the region. This also clears any active surface-edit overlay so the scene renders cleanly.

### Editing Slice

Offsets the slice plane when in 2D editing without an active surface, letting you peek through layered geometry.

The slice position is not fixed to a small range (useful for tall maps / mountains).

It also defines the **slice height/thickness** (`1..10`, default `2`).  
A higher value includes more geometry around the slice for both preview and selection.

### Geometry / Detail Mode

In 3D editor views, the HUD exposes two geometry modes:

- `GEOM`: edit world geometry directly.
- `DETAIL`: edit the profile of the clicked surface directly in 3D.

`DETAIL` replaces the older separate surface-edit workflow. Profile-specific actions only appear when `DETAIL` is active and the current profile selection makes them applicable.

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
> - a palette index (integer, or numeric string like `"2"`).

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
- UVs follow the linedef direction (continuous flow along the feature, not world X/Z projection).

### Create Fence

Create a non-destructive fence along selected linedefs.

- `[material].tile_id`: tile used by posts and connectors.
- `[layout].spacing`: distance between posts.
- `[posts].shape`: `square` or `round`.
- `[posts].size`: post thickness.
- `[posts].height`: post height. `0.0` disables generation.
- `[posts].round_segments`: radial complexity for round posts.
- `[connectors].count`: number of horizontal connectors between posts.
- `[connectors].style`: `plank`, `square`, or `round`.
- `[connectors].size`: connector thickness.
- `[connectors].drop`: how far connectors step down from the top.
- `[lean].amount`: max lean offset.
- `[lean].randomness`: 0..1 multiplier for random lean variation.
- UVs follow the linedef direction (continuous flow along the feature, not world X/Z projection).

### Create Stairs

Create non-destructive stairs on selected sectors (3D editor views).

Parameter groups:
- `[stairs]`: `direction`, `steps`, `total_height`, `fill_sides`
- `[material]`: `tile_id`, `tread_tile_id`, `riser_tile_id`, `side_tile_id`

Parameter meaning:
- `[stairs].direction`: stair run direction (`north`, `east`, `south`, `west`).
- `[stairs].steps`: number of treads (`1..64`).
- `[stairs].total_height`: total vertical rise of the full staircase (`0..16` world units).
- `[stairs].fill_sides`: when enabled (default), side geometry is generated so stairs are closed instead of hanging.
- `[material].tile_id`: default tile UUID used by stair geometry if a per-part tile is not set.
- `[material].tread_tile_id`: optional tile UUID for tread surfaces.
- `[material].riser_tile_id`: optional tile UUID for riser (vertical) surfaces.
- `[material].side_tile_id`: optional tile UUID for side surfaces.

Material fallback order:
- tread: `tread_tile_id` -> `tile_id` -> sector/source fallback
- riser: `riser_tile_id` -> `tile_id` -> sector/source fallback
- side: `side_tile_id` -> `tile_id` -> sector/source fallback

Set `[stairs].total_height = 0` to clear stair generation on the sector.

### Create Roof

Create a non-destructive roof on sectors touched by selected linedefs (3D editor views).

Parameter groups:
- `[roof]`: `name`, `style`, `height`, `overhang`
- `[material]`: `tile_id`, `side_tile_id`

Parameter meaning:
- `[roof].name`: logical roof label stored as `roof_name` on target sectors.
- `[roof].style`: `flat`, `pyramid`, or `gable`.
- `[roof].height`: roof rise above the sector top surface. `0` clears roof generation.
- `[roof].overhang`: outward roof extension in world units (applies to top and side eaves).
- `[material].tile_id`: optional tile UUID for roof top surfaces.
- `[material].side_tile_id`: optional tile UUID for roof side surfaces.

Material fallback order:
- top: `tile_id` -> sector `cap_source` -> sector `source`
- side: `side_tile_id` -> top fallback chain

### Create Campfire

Create a non-destructive campfire on selected sectors (3D editor views).

Parameter groups:
- `[campfire]`
- `[material]`

### `[campfire]`

- `flame_height`: flame height.
- `flame_width`: flame width.
- `log_count`: number of logs arranged in a ring (`3..24`).
- `log_length`: per-log length.
- `log_thickness`: per-log thickness.
- `log_radius`: ring radius from center to log centers.
- `light_intensity`: point-light intensity.
- `light_range`: point-light end distance.
- `light_flicker`: light flicker amount (`0..1`).
- `light_lift`: extra Y offset added on top of flame center.

### `[material]`

- `flame_tile_id`: flame material source (UUID or palette index).
- `base_tile_id`: log/ember material source (UUID or palette index).

Notes:
- Logs are procedural 3D meshes placed in a circle and oriented inward.
- Flame is billboard-based (crossed center quads).
- Campfire point-light origin is anchored to flame height (`flame_base_y + flame_height * 0.5 + light_lift`).
- Set `flame_height = 0` or `light_range = 0` to clear campfire generation on the sector.

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

Cut a recess into the active profile surface. Use it in `DETAIL` mode on a selected profile sector. Params: *depth*, *target* front/back face, and cap/jamb tiles chosen via two icons (shows the textures that will be stamped).

### Relief

*Shortcut: Alt + E*

Emboss the active profile surface outward. Use it in `DETAIL` mode on a selected profile sector. Params mirror Recess: *height*, *target* (front/back), and cap/side tiles.

### Gate / Door

*Shortcut: Alt + G*

Carve an inset opening in the profile surface and fill it with a tile. Use it in `DETAIL` mode on a selected profile sector. Params: *inset*, *repeat/scale* mode, gate/door *tile* icon, and speed/behavior flags (hidden, locked, secret) stored on the sector.

### Window

Create a static window inside the selected profile hole. The window generates frame geometry and a glass pane (no dynamic open/close behavior).

Use it in `DETAIL` mode on a selected profile sector.

Parameters:
- `[window]`
- `[material]`

### `[window]`

- `inset`: push/pull the full window assembly along the surface normal.
- `frame_width`: thickness of the frame in profile UV space.

### `[material]`

- `frame_tile_id`: frame material source (UUID or palette index).
- `glass_tile_id`: glass material source (UUID or palette index).
  - if `glass_tile_id` is empty/unset, no glass mesh is generated and the opening remains a passable hole.

### Cut Hole

Convert one selected sector into a profile hole inside another selected sector.

Use it in `GEOM` mode in 3D editor views with exactly two sectors selected on the same height plane:
- the larger containing sector becomes the host surface
- the contained sector becomes the cutout handle

What it does:
- creates or reuses the host surface profile
- inserts the inner sector shape as a profile hole
- hides the original cutout sector in world geometry
- links the generated profile hole back to the source sector so edits can stay synchronized

Notes:
- this is the intended first step for shaft/stair workflows
- after cutting, the visible opening is driven by the host surface profile, not by the original sector polygon

### Build Shaft

Build vertical shaft walls from a selected cutout handle sector.

Use it in `GEOM` mode in 3D editor views with exactly one sector selected, and that sector must be a `Cut Hole` handle.

Parameters:
- `[action].direction`: `Down` or `Up`
- `[action].depth`: shaft depth in world units
- `[action].bottom_cap`: create a cap at the far end of the shaft

Behavior:
- generates wall sectors around the cutout perimeter
- optionally generates a bottom/top cap sector
- copies source material/shader/layer state from the handle sector
- marks generated sectors so later tools can detect the shaft structure

Notes:
- default depth is `3.0`
- the original cutout handle remains the control shape for rebuilding

### Build Room

Build a room volume from a selected vertical wall sector.

Use it in `GEOM` mode in 3D editor views with exactly one sector selected. The selected sector must be wall-like, not a floor/ceiling surface.

Parameters:
- `[action].room_type`: room footprint shape.
  - `Rect`
  - `Corridor`
  - `Chamfered`
  - `Octagon`
- `[action].depth`: room depth measured inward from the selected wall.
- `[action].height`: room interior height.
- `[action].width_mode`: how room width is chosen.
  - `Match Wall`
  - `Expand`
  - `Custom`
- `[action].width`: width value used by `Expand` / `Custom`.
- `[action].ceiling_mode`: `Flat` or `None`.
- `[action].keep_original_wall`: keep the selected wall sector visible instead of opening the room front.
- `[action].close_front_lip`: when the source wall is taller than the room opening, create a closing strip from room ceiling up to the original wall top.
- `[material].room_tile_id`: shared fallback material for the whole generated room.
- `[material].room_floor_tile_id`: optional floor override.
- `[material].room_wall_tile_id`: optional wall override.
- `[material].room_ceiling_tile_id`: optional ceiling override.

Behavior:
- creates a floor sector for the room footprint
- creates wall sectors around the perimeter
- optionally creates a ceiling sector
- optionally creates a front lip sector above the opening
- hides the original selected wall sector unless `keep_original_wall` is enabled
- copies shader/layer/property context from the source sector onto generated sectors

Material fallback:
- floor: `room_floor_tile_id` -> `room_tile_id` -> source sector material
- wall: `room_wall_tile_id` -> `room_tile_id` -> source sector material
- ceiling: `room_ceiling_tile_id` -> `room_tile_id` -> source sector material
- front lip: wall fallback chain

Notes:
- Build Room creates surfaces only; wall thickness should be added afterward with [Extrude Sector](/docs/creator/actions/#extrude-sector)
- the room is generated relative to the selected wall’s direction and normal

### Build Stairs

Build a stair run between two selected linedefs.

Use it in `GEOM` mode in 3D editor views with exactly two linedefs selected and no sectors selected.

Selection rules:
- the lower linedef becomes the stair start
- the higher linedef becomes the stair end
- both edges are aligned automatically before stair generation

Parameters:
- `[stairs].steps`: number of steps (`1..64`)
- `[stairs].side_walls`: generate side walls along the stair run
- `[material].tile_id`: default stair material
- `[material].tread_tile_id`: optional tread override
- `[material].riser_tile_id`: optional riser override
- `[material].side_tile_id`: optional side-wall override

Behavior:
- creates generated stair sectors between the two selected edges
- writes stair metadata/material overrides onto the generated geometry
- if the stair connects to a `Build Shaft` opening, the matching top shaft wall is opened automatically

Material fallback:
- tread: `tread_tile_id` -> `tile_id` -> sector/source fallback
- riser: `riser_tile_id` -> `tile_id` -> sector/source fallback
- side walls: `side_tile_id` -> `tile_id` -> sector/source fallback

### Create Prop

Create/edit parametric props on selected sectors (2D editor view with an active editing surface).

Parameter groups:
- `[prop]`
- `[table]`
- `[bookcase]`
- `[crate]`
- `[barrel]`
- `[bed]`
- `[material]`

### `[prop]`

- `type`: prop generator to apply.
  - `table` (or `0`)
  - `bookcase` (or `1`)
  - `crate` (or `2`)
  - `barrel` (or `3`)
  - `bed` (or `4`)

### `[table]`

- `height`: total table height.
- `chairs`: enable/disable generated chairs.
- `chair_count`: number of chairs (`0..8`).
- `chair_offset`: offset from table bounds to chair centers.
- `chair_width`: chair seat width/footprint scale.
- `chair_back_height`: multiplier for chair back height.
- `chair_tile_id`: optional chair material source.
  - accepts UUID or palette index.

### `[bookcase]`

- `height`: total cupboard/bookcase height.
- `shelves`: number of internal shelves (`1..12`).
- `books`: enable/disable procedural books.
  - when enabled, books use deterministic per-book random palette colors.

### `[crate]`

- `height`: total crate height.

### `[barrel]`

- `height`: total barrel height.
- `bulge`: middle ring scale (`1.0..1.5`).
- `segments`: radial segment count (`6..32`).

### `[bed]`

- `height`: total bed height.
- `headboard`: enable/disable headboard.
- `headboard_side`: choose which side of the bed length gets the headboard (`start`/`end`).
- `headboard_height`: height of the headboard above mattress/frame.
- `mattress_tile_id`: optional mattress material source.
  - accepts UUID or palette index.

### `[material]`

- `tile_id`: base prop material source (carcass/table surfaces).
  - accepts UUID or palette index.

Notes:
- Bookcase footprint is derived from the selected floor sector shape/depth.
- `prop.type` fully controls which generator is applied (`table`, `bookcase`, `crate`, `barrel`, `bed`).

### Clear Profile

*Shortcut: Alt + G*

Remove any stored profile operation (`profile_op`) from selected sectors. This restores a flat face.

### Toggle Editing Geometry

*Shortcut: Ctrl/Cmd + T*

Show or hide the 3D editing overlay geometry. Toggling this also refreshes the overlay.

### Filter Geometry

Filter editor rendering to isolate generated dungeon geometry while texturing or inspecting buried spaces.

Modes:
- `All`: normal editor rendering, including region preview rules such as `[preview].hide`.
- `Dungeon`: shows only geometry generated by the Dungeon Tool.

Option:
- `Dungeon No Ceiling`: when enabled, dungeon ceiling geometry is hidden as well. This is useful when you want to paint floors, walls, stairs, or doors inside enclosed dungeon spaces.

Notes:
- this is an editor-only filter and does not change authored sector visibility
- in 3D it also filters terrain and non-dungeon generated feature geometry such as palisades/fences
- the 3D editing overlay respects the same filter

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

Set tile *role*, *blocking* flag (2D collisions), and *tags* for the currently selected tile in the tile picker.

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
