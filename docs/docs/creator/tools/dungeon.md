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

## Reference Geometry

The conceptual view can show nearby existing world geometry as a weak reference layer.

This is useful when:

- continuing walls toward an existing staircase or shaft
- aligning a dungeon blockout to already authored geometry
- connecting generated dungeon spaces to hand-built world structures

## Generated Geometry

Dungeon output is written as normal map geometry and tagged with provenance metadata.

That generated geometry can then be:

- painted with tiles
- detailed further with normal tools
- connected to the rest of the world

Door panels use real geometry and can be painted like normal sectors.

## Related Pages

- [Overview](/docs/creator/tools/overview)
- [Rect Tool](/docs/creator/tools/rect)
- [Builder Tool](/docs/creator/tools/builder)
- [Creating 3D Maps](/docs/building_maps/creating_3d_maps)
