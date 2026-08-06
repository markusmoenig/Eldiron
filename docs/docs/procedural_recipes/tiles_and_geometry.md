---
title: "Tiles and Geometry"
sidebar_position: 5
---

Tile recipes are height-first procedural assets. Fields and patterns build scalar structure; materials then color that structure and provide renderer surface channels.

## Tile root

```recipe
Tile
  name = "Dungeon Stone Wall"
  placement = Surface
  blocking = true
  size = I2(128, 128)
  coverage = I2(2, 2)
  wrap = Repeat
  seed = 17
  pixelate = 1
```

| Field | Default | Meaning |
| --- | --- | --- |
| `name` | `"Untitled Tile"` | Human-readable name. |
| `placement` | `Surface` | `Surface` owns the host; `Fixture` overlays an existing host. |
| `blocking` | `false` | Default placement solidity for map compilers. |
| `size` | `I2(64, 64)` | Pixel size of one Tile cell. |
| `coverage` | `I2(1, 1)` | Coordinated Tile cells across and down. |
| `wrap` | `Repeat` | Sampling outside the domain. |
| `seed` | `1` | Root deterministic seed. |
| `pixelate` | `1` | Positive sampling-pixel size. |

The complete rendered image is `size × coverage`. A coverage of `I2(2, 2)` produces one coordinated four-cell surface rather than four copies of a smaller image.

Use `placement = Fixture` for host-independent props such as torches, switches,
and door mechanisms. A fixture carries geometry, lights, particles, and named
attachments but does not replace the wall, floor, or ceiling beneath it. The
placement host therefore keeps its own continuous texture coverage.

## Height workflow

```recipe
Pattern Stones
  Voronoi
    cells = I2(5, 4)
    jitter = 0.82
    falloff = 1.4

Height Surface
  source = Stones.height

  Shape
    contrast = 1.25
    rim = 0.08

  Clamp
    min = 0.0
    max = 1.0

Output
  height = Surface
```

Height operations are ordered. They can reshape, add, multiply, clamp, terrace, blur, or otherwise combine scalar sources. An explicit `Output.height` makes the final geometric field clear.

When no material or `Colorize` block is present, the height can be rendered directly as grayscale for inspection.

## Surface assignment

Use `material = alias` for a single material or `MaterialMap` for masked layers. Tile height remains visible to the Tile masks but is deliberately not an implicit input inside reusable Material documents.

This division prevents a material that works on stone geometry from becoming unusable on an Avatar, button, or prop that has a different shape source.

## Programmable placement geometry

The `Geometry` block contains generic constructive primitives. The host supplies
a local placement basis; the recipe does not contain hardcoded concepts such as
"niche" or "beam":

```recipe
Geometry
  Box Recess
    operation = Subtract
    surface = niche-stone
    position = F3(0.12, 0.76, 0.0)
    size = F3(0.76, 1.06, 0.46)
```

`Add` emits a solid; `Subtract` carves the Box from the placement solid. A wall
placement maps local X across the face, Y upward, and Z into the wall. A ceiling
placement maps X/Z across the cell and Y down into the room. The same IR is
therefore useful for cavities, beams, frames, ledges, and later non-map hosts.

Use `repeat = I3(x, y, z)` with `spacing = F3(x, y, z)` for arrays. The Source
adapter clips subtractive volumes to the placement and applies `surface` to the
newly exposed faces. Additive Boxes use that surface on every face.

Every named Box exposes `<name>.distance`, a signed distance to its local XY footprint. A Tile can use it to align mortar, wear, or other material masks with the generated shape:

```recipe
Height RecessJoint
  source = 1.0 - Smoothstep(0.025, 0.080, Abs(Recess.distance))
```

The geometry and appearance therefore stay synchronized in one recipe while the adapter remains responsible for map-aware placement.

## Named attachments and placement effects

`Attachment` declares a reusable point in the same local coordinates as
`Geometry`. `Light` and `Particles` blocks refer to that point by name:

```recipe
Attachment Flame
  position = F3(0.50, 1.58, -0.47)
  direction = F3(0.0, 1.0, 0.0)

Light TorchLight
  attach = Flame
  color = #ff9a45
  intensity = 1.75
  range = 5.2
  flicker = 0.22

Particles TorchFlame
  attach = Flame
  direction = F3(0.0, 1.0, 0.0)
  spread = 0.48
  rate = 25.0
  color_ramp = #fff2a8, #ffc14f, #f0641f, #401008
  lifetime = F2(0.32, 0.78)
  radius = F2(0.025, 0.065)
  speed = F2(0.28, 0.72)
  spawn_area = F3(0.045, 0.015, 0.035)
  flame_base = true
```

The host transforms attachments, geometry, lights, and particle directions
through one placement basis. A wall torch therefore keeps its flame over its
basket on every exposed wall direction. Effects belong to the owning Tile
recipe rather than a Box `surface`, preventing one emitter from being created
for every textured geometry face.

`Light` fields are `attach`, `color`, `intensity`, `range`, `flicker`, and
`lift`. `Particles` supports `attach`, `direction`, `spread` (radians), `rate`,
`color`, an optional four-color `color_ramp`, `color_variation`, ordered
`lifetime`, `radius`, and `speed` ranges, `spawn_area`, and `flame_base`.
Multiple named attachments and effects may coexist in one recipe.
