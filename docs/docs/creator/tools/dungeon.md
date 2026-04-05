---
title: "Dungeon Tool"
sidebar_position: 7
---

The **Dungeon Tool** (keyboard shortcut **`U`**) lets you paint **conceptual dungeon structure** in a top-down grid view and generate normal map geometry from it.

It is intended as a faster workflow for blockouts such as:

- rooms
- corridors
- shafts
- door openings
- connected floor levels

Instead of placing vertices, linedefs, and sectors by hand, you paint **structural tiles** and let Eldiron build the matching floor, ceiling, wall, and door geometry.

## Concept Workflow

The Dungeon Tool edits a **conceptual dungeon layer** stored on the map.

That conceptual layer is used to generate normal map geometry:

- sectors
- linedefs
- surfaces

This means the generated result can still be refined later with the normal editor tools.

## Dock Layout

When the Dungeon Tool is active, the lower picker area shows the **Dungeon dock** instead of the **Tile Picker**.

The dock has two parts:

- a scrollable palette of structural dungeon tiles
- a TOML settings panel on the right

## Painted Tiles

Dungeon tiles describe the structure of **one grid cell**.

Typical examples are:

- floor
- single wall edges
- wall corners
- multi-edge wall cells
- oriented door cells

Rooms are built by combining many painted tiles.

## Settings

The Dungeon dock settings are edited as TOML.

The main sections are:

```toml
[dungeon]
floor_base = 0.0
height = 4.0
floors = true
ceilings = true
standalone = false

[tile]
door_width = 2
door_depth = 0.5
door_height = 2.25
open_mode = "Auto"
item = "Door Handler"

[steps]
floor_delta = -1.0
steps = 4
tile_id = ""
tile_mode = "Repeat"

[render]
transition_seconds = 1.0
sun_enabled = false
shadow_enabled = true
fog_density = 5.0
fog_color = "#000000"
```

### `dungeon`

- `floor_base`: base height of newly painted tiles
- `height`: wall height and ceiling offset above `floor_base`
- `floors`: whether generated dungeon geometry creates floor sectors
- `ceilings`: whether generated dungeon geometry creates ceiling sectors
- `standalone`: keeps newly painted cells separate instead of merging them into larger generated pieces

### `tile`

These settings apply to the currently selected dungeon tile when relevant.

For door tiles:

- `door_width`: width of the generated opening in tiles
- `door_depth`: thickness of the generated door panel
- `door_height`: height of the moving door panel
- `open_mode`: how the runtime door opens
- `item`: item handler attached to the generated door sector

### `steps`

These settings apply to stair tiles.

- `floor_delta`: relative floor-base change across the stair tile. Negative values go down.
- `steps`: number of generated steps inside the tile.
- `tile_id`: default source applied to generated stair geometry.
- `tile_mode`: how the stair source is mapped. `Repeat` is the normal mode, `Scale` stretches the source across the stair assembly.

### `render`

These settings override the normal region or game `[render]` settings while the player is inside dungeon-generated geometry.

- `transition_seconds`: smooth blend duration when entering or leaving the dungeon space
- `sun_enabled`: override sun lighting inside the dungeon
- `shadow_enabled`: override sun shadows inside the dungeon
- `fog_density`: fog density, using the same percent-style value as normal `[render]`
- `fog_color`: fog color override

The global game or region `[render]` settings remain the source of truth.  
Dungeon Tool only overrides the keys you specify in its own `[render]` block, and normal rendering is restored automatically when the player leaves the dungeon geometry.

## Tile Types

The palette is not room-based. Each icon represents the structure of **one tile**.

That includes:

- floor
- single-edge wall tiles
- corner and multi-edge wall tiles
- oriented door tiles
- oriented stair tiles

You compose rooms, corridors, and shafts by painting many tiles together.

## Navigation

Dungeon Tool switches into the top-down authoring view and uses the normal map HUD.

While active:

- the conceptual preview is always shown
- a hover rectangle shows the target cell
- the map subdivisions strip is hidden
- subdivisions are temporarily forced to `1`

When you leave the tool, the previous editor view and subdivision setting are restored.

## Painting

- **Click / drag** to paint dungeon tiles
- Hold **Shift** while painting to erase
- Hold **Cmd/Ctrl** while dragging to lock the stroke to a straight horizontal or vertical line

Door tiles can stamp wider openings based on `door_width`.

## Reference Geometry

The conceptual view can show nearby existing world geometry as a weak reference layer.

This is useful when:

- continuing walls toward an existing staircase or shaft
- aligning a dungeon blockout to already authored geometry
- connecting generated dungeon spaces to hand-built world structures

## Doors

Dungeon doors generate **real geometry**, not only billboards.

That means:

- door panels are paintable in the editor
- split doors can use two moving leaves
- jambs and lintels are generated as normal geometry
- the generated door can still be driven by an item handler such as `Door Handler`

Door panels support the configured `open_mode`, including:

- `Auto`
- `Slide Up`
- `Slide Down`
- `Slide Left`
- `Slide Right`
- `Split Sides`

At runtime, the generated door geometry is animated from item state.

## Stairs

Stair tiles generate editable stair geometry directly into the map.

This includes:

- stair treads
- risers
- optional stair ceilings when dungeon ceilings are enabled

Because the result is normal geometry, it can still be painted and refined with the existing tools.

## Generated Geometry

Dungeon output is written as normal map geometry and tagged with provenance metadata.

That generated geometry can then be:

- painted with tiles
- detailed further with normal tools
- connected to the rest of the world

Door panels use real geometry and can be painted like normal sectors.

Stair assemblies can also receive a shared source through the stair tile settings.

## Texturing Workflow

Dungeon geometry is often hidden under terrain, roofs, or other authored world geometry.

For texturing and detailing, use the general **Filter Geometry** action:

- `All`: normal editor view
- `Dungeon`: show only dungeon-generated geometry
- `Dungeon No Ceiling`: hide dungeon ceilings as well, so interior spaces stay visible

This is especially useful when painting dungeon walls, doors, and stair assemblies in 3D editor views.

## Related Pages

- [Overview](/docs/creator/tools/overview)
- [Rect Tool](/docs/creator/tools/rect)
- [Builder Tool](/docs/creator/tools/builder)
- [Creating 3D Maps](/docs/building_maps/creating_3d_maps)
