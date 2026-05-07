# Eldiron 3D Concept

## Decision

Eldiron's 3D authoring should move away from the current linedef/surface/profile-first workflow as the main construction model.

The current system is powerful, but it behaves like a compiler:

```text
linedefs + sectors + surfaces + profiles + terrain rules
-> chunk builder
-> generated 3D geometry
```

That makes non-destructive procedural work possible, but it is the wrong center for realtime 3D editing. Users expect to select, drag, extrude, paint, duplicate, and reshape things directly in the scene. If every edit has to pass through a semantic terrain/surface/profile rebuild, Eldiron will keep feeling slow and indirect.

Because Eldiron is still early in 3D development and `Village3D.eldiron` is the only real 3D project, this is the right time to make the cut.

## New Center

The new 3D core should be based on editable geometry objects.

```text
GeometryObject
  vertices
  faces
  face material / tile data
  transform
  optional semantic tags
  optional generator metadata
```

Everything that creates 3D content should eventually produce or edit `GeometryObject`s:

```text
Direct brush editing -> GeometryObject
Dungeon tool         -> GeometryObject
BuilderGraph         -> GeometryObject
Linedef helpers      -> GeometryObject
Imported props       -> GeometryObject
```

This avoids maintaining two competing 3D systems. The renderer and editor only need one final editable geometry model.

## Authoring Goal

Eldiron 3D should feel like a fast retro brush and prop editor with optional procedural helpers.

All 3D geometry should be creatable inside Eldiron.

External geometry import should not be part of the core workflow. Eldiron should provide enough direct editing, prop building, tile painting, and procedural helper tools that users can build their worlds and props without leaving the editor.

This is a product constraint, not just a technical one:

```text
No Blender dependency
No required mesh import workflow
No "make the real asset somewhere else" expectation
```

Eldiron can still export or debug geometry later if useful, but the normal creative path should be internal.

The first useful toolset should be small:

```text
Create box / prism
Move object
Select face
Move face
Extrude face
Inset face
Assign tile
Duplicate
Group as prop
Save prop to Treasury
Place prop instances
```

This is not meant to become Blender. It should be constrained, grid-aware, tile-friendly, and fast.

## Current Direct Editing Shortcuts

Keep this list updated while the new 3D path is changing quickly.

```text
G             Object Tool
V             Vertex Tool
L             Linedef / edge tool
E             Sector / face tool
M             Object gizmo: move
S             Object gizmo: size
Shift + 1..0  Grid subdivision
, / .         Grid size down / up
Cmd/Ctrl + D  Duplicate selection

Face selected:
E             Extrude face by one grid step
X             Subdivide quad face
T             Apply current tile / color / procedural source
+ / -         Push / pull along face normal
[ / ]         Move vertically
Delete        Delete face, keep boundary vertices selected

Edge / vertices selected:
X             Split selected geometry edges
F             Fill selected boundary with a face
L             Select edge loop on quad geometry
[ / ]         Move vertically
```

## Tile Painting

The tile-based drawing workflow should stay. It is one of Eldiron's strongest 3D ideas because it matches the retro RPG style and avoids complex UV editing for normal users.

The important rule:

```text
Painting tiles changes surface/face tile data.
It must not rebuild geometry into one mesh per painted tile during live editing.
```

A face can have a simple tile projection:

```text
FaceTileMapping
  origin
  u_axis
  v_axis
  tile_scale
  default_tile
  tile_overrides: map from (u, v) cell -> tile
```

The Rect tool can still paint tiles onto any face by converting the hit position into face-local tile coordinates.

```text
3D hit position
-> face local UV
-> tile cell coordinate
-> update tile override
-> mark face material data dirty
```

The mesh itself does not need to be rebuilt just because a tile changes.

Rendering can handle tiled faces in one of three ways:

1. CPU raster/material lookup uses face-local tile coordinates at render time.
2. Build a lightweight per-face tile material table, but keep the face mesh stable.
3. Bake tile-painted faces into render chunks only after editing stops or when exporting.

For the editor, option 1 or 2 is preferred. The editing experience must stay immediate.

## UV Editing

Manual UV editing should not be required for normal Eldiron work.

Default mapping rules should cover most cases:

```text
Walls: horizontal = world/face X along edge, vertical = Y height
Floors: horizontal = world X, vertical = world Z
Ceilings: same as floors
Props: generated box/prism mapping per face
```

Advanced UV controls can come later, but the default experience should be tile painting, not UV unwrapping.

## Terrain

Terrain should remain a world layer, but it must not control realtime object editing.

During drag:

```text
Move house / prop / brush immediately
Show terrain unchanged or with a cheap preview mask
Do not rebuild terrain synchronously
```

After drag:

```text
Update terrain masks / cutouts
Rebuild affected terrain chunks in the background
Swap in updated terrain when ready
```

Terrain cutouts should be driven by object footprints or tags, not by forcing every house edit through terrain geometry generation.

Example:

```text
GeometryObject tag: terrain_cutout
footprint: polygon or projected bounds
mode: hide / flatten / lower / blend edge
```

The terrain system can read these tags and generate terrain holes or flattened pads, but it must not block the direct editing loop.

## Props

Props should be built from the same `GeometryObject` model.

Users should be able to:

```text
Create brush shapes
Edit faces
Paint tiles
Group shapes
Save as Treasury prop
Place instances in the world
```

This makes props feel hand-built and immediate. BuilderGraph can still generate prop variants, but it should not be the only way to create them.

## BuilderGraph

BuilderGraph should become optional.

Its role should shift from "main way to make complex 3D content" to "generator/helper for users who want procedural variation."

New role:

```text
BuilderGraph script
-> GeometryObject or prop asset
-> editable or locked result
```

Generated objects can keep metadata pointing back to their source graph:

```text
source_generator: builder_graph
source_data: ...
editable: true / false
```

If `editable` is true, the generated object can be modified directly after creation. If the user regenerates it, Eldiron can warn that manual edits may be replaced.

## Existing Linedef/Sector Tools

The old linedef/sector/surface workflow should not remain the primary 3D construction path.

Useful parts can become helpers:

```text
Draw room outline -> create wall/floor GeometryObjects
Draw path -> create road/terrain mask GeometryObject
Dungeon cells -> create room/corridor GeometryObjects
Profile hole -> create face cut/inset operation
```

The authored result should become editable geometry, not a hidden recipe that must always be rebuilt to see changes.

## 2D And 3D Map Integration

Eldiron should keep 2D maps. They are part of the engine's identity and are still useful for gameplay, navigation, scripting, regions, minimaps, triggers, and classic 2D games.

The change is that 2D map geometry should no longer be the mandatory source of 3D mesh geometry.

Current model:

```text
2D vertices / linedefs / sectors
-> used directly in 2D
-> extruded / profiled into 3D
```

New model:

```text
2D map layer
  gameplay space
  navigation
  triggers
  regions
  tile maps
  optional layout sketches

3D geometry layer
  GeometryObjects
  props
  terrain
  direct brush/mesh editing
```

The two layers can be linked, but neither should force the other to be slow.

### 2D Maps In 2D Games

For 2D games, nothing fundamental has to change.

```text
2D map vertices / linedefs / sectors / tiles
-> remain the authored world
```

The existing 2D workflow can continue as-is.

### 2D Maps In 3D Games

For 3D games, the 2D map should become a gameplay and planning layer.

Possible uses:

```text
Top-down reference while building
Minimap source
Navigation regions
Collision/navigation hints
Room IDs
Script trigger areas
Encounter zones
Minimap shapes
Camera bounds
Terrain masks
Dungeon layout input
```

This is important: a 3D project should still have a quick readable 2D plan. The user should be able to understand the region from above, place gameplay areas, inspect room relationships, and generate a minimap even if the visible 3D geometry was built by hand.

A 3D region can still show and edit the 2D layer, but editing the 2D layer should not automatically force realtime 3D mesh rebuilds.

### Optional Conversion

2D geometry can still help create 3D content, but as an explicit conversion/generation step.

Examples:

```text
Draw 2D room outline -> Generate wall/floor GeometryObjects
Draw 2D path -> Generate road mask or road mesh
Draw 2D sector -> Generate terrain cutout/flatten mask
Dungeon 2D cells -> Generate editable 3D room GeometryObjects
```

After generation, the result should be editable as normal 3D geometry.

There are two useful modes:

```text
Linked mode:
  2D outline remains the source.
  Regenerate updates the 3D objects.
  Manual edits may be overwritten.

Baked mode:
  2D outline creates 3D objects once.
  Result is fully hand-editable.
  No automatic rebuild link remains.
```

Default for user-created content should be baked mode. It gives users ownership and avoids the feeling that the editor is controlled by hidden recipes.

### Shared Coordinates

2D and 3D should share the same ground-plane coordinate system.

```text
2D x -> 3D x
2D y -> 3D z
3D y -> height
```

This keeps minimaps, terrain painting, navigation regions, and 3D placement understandable.

### Collision And Gameplay

3D geometry should be able to generate collision/navigation data.

The 2D layer can still override or annotate gameplay:

```text
2D sector: "Town"
2D sector: "No Combat"
2D linedef: "Door Trigger"
2D region: "Quest Area"
```

But physical 3D collision should come from the actual `GeometryObject`s when possible.

### Editor UX

In a 3D project, the user should be able to toggle:

```text
Show 2D gameplay layer
Edit 2D gameplay layer
Show 2D layer as top-down reference
Show 3D geometry layer
Edit 3D geometry layer
Show generated links
```

This preserves the strengths of the 2D map without forcing all 3D construction to happen through 2D linedefs.

## Performance Principle

Realtime editing must update only what the user is touching.

The target for direct manipulation is 60 fps.

This applies to:

```text
Dragging objects
Moving vertices / edges / faces
Extruding and insetting faces
Painting tiles with the Rect tool
Placing and transforming props
Camera movement while editing
```

Any operation that cannot stay inside the realtime interaction budget must not run synchronously during pointer movement.

During drag:

```text
Object transform edits: update transform only
Face move/extrude: update that object's mesh only
Tile paint: update face tile data only
Terrain response: delayed/background
Procedural rebuild: delayed/manual
```

The editor should never require full terrain or procedural chunk rebuilds to keep the cursor responsive.

Slow work is allowed, but only outside the direct manipulation loop:

```text
During drag: cheap preview / immediate local object update
On mouse-up: commit expensive rebuilds
In background: terrain/procedural/chunk updates
On export: optional baking/optimization
```

The user should always feel that the object follows the mouse immediately, even if terrain, lighting, procedural details, or baked render chunks catch up a moment later.

## Migration Plan

### Phase 0: First Vertical Slice

Before replacing the old 3D system, build one small direct-editing proof.

Goal:

```text
Create one box GeometryObject
Render it
Select it in 3D
Drag it at 60 fps
Paint one face with the Rect tool
Save/load it in the project file
```

This proves the new architecture before a larger rewrite begins.

The first slice should avoid hard problems:

```text
No terrain cutout
No face extrusion
No prop library
No BuilderGraph integration
No advanced gizmo
```

The only acceptable question for this phase is:

```text
Does this object follow the mouse instantly?
```

### Phase 1: GeometryObject Core

Add a new editable 3D geometry model.

Minimum data:

```text
id
name
vertices
faces
face tile mapping
transform
tags
```

Render these objects in the existing 3D renderer/chunk path.

### Phase 1.5: Minimal Gizmos

Direct editing needs gizmos, but Eldiron should start with simple, purpose-built handles rather than a full 3D DCC transform system.

Minimum gizmos:

```text
Object move: plane drag using the active edit plane
Object height move: vertical Y handle
Face move: drag selected face along its normal
Face extrude: command/button first, gizmo later
```

Existing editor concepts can be reused:

```text
Current 3D hit position
Current hovered GeoId
Current active edit plane: XZ / XY / YZ
Existing plane picker UI
MapDragged events
```

The first gizmo does not need rotation or scale. Those can come later after object movement and face movement feel good.

Gizmo rule:

```text
Gizmos manipulate GeometryObject data directly.
They must not trigger terrain/procedural/chunk rebuilds during pointer movement.
```

During a drag, the renderer should receive an immediate object transform or mesh update. Expensive scene sync happens after the edit is committed.

### Phase 1.6: Selection Modes

The direct editor should have a tiny mode set:

```text
Object mode
Face mode
Edge mode later
Vertex mode later
```

Start with object mode and face mode only.

Object mode allows:

```text
select object
move object
duplicate object
delete object
paint all selected faces
```

Face mode allows:

```text
select face
move face along normal
paint face
extrude face
inset face later
```

### Phase 2: Brush Tool

Add direct creation and editing:

```text
Create box
Select object
Select face
Move object
Move face
Extrude face
Paint face with Rect tool
```

This phase is the proof that Eldiron 3D can feel fast.

### Phase 3: Props

Allow multiple `GeometryObject`s to be grouped and saved as a prop asset.

```text
Group selection
Save to Treasury
Place prop instance
Edit prop source
Update instances
```

### Phase 4: Terrain Integration

Add terrain tags and delayed terrain updates:

```text
terrain_cutout
terrain_flatten
terrain_blend_edge
```

Terrain reads object footprints after editing and updates chunks asynchronously.

### Phase 5: Convert Existing Generators

Make existing systems output `GeometryObject`s:

```text
Dungeon tool -> GeometryObject rooms/corridors
BuilderGraph -> GeometryObject props/details
Linedef wall helper -> GeometryObject walls
```

### Phase 6: Retire Old 3D Center

Once direct editing is viable, stop treating surfaces/profiles as the main 3D data model.

They can remain as compatibility/import/generator helpers if useful, but new 3D work should be object-centered.

## Product Statement

Eldiron 3D should be:

```text
A fast retro RPG world builder where users directly shape and tile-paint simple 3D geometry, with procedural tools available when they help.
```

Not:

```text
A procedural geometry compiler that users must indirectly manipulate through linedefs, profiles, and scripts.
```
