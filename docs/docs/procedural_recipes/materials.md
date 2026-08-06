---
title: "Materials and Color"
sidebar_position: 4
---

A Material recipe defines reusable appearance independently of Tile height or Avatar shape. It can produce color, roughness, metallic response, opacity, emission, and a micro-height field used for normals.

## Material structure

```recipe
Material worn_iron
  name = "Worn Iron"
  wrap = Repeat
  seed = 12

  Noise Wear
    type = Gradient
    fractal = Ridged
    scale = F2(10.0, 10.0)
    octaves = 3

  Color Shadow
    nearest = #53606d

  Color Highlight
    nearest = #c5cbd1

  Color Iron
    source = Mix(Shadow, Highlight, Wear)

  Surface
    color = Iron
    palette = BaseOnly
    roughness = Clamp(0.45 + Wear * 0.25, 0.0, 1.0)
    metallic = 0.9
    opacity = 1.0
    emission = 0.0
    normal = Wear
    normal_strength = 0.2
```

A material may contain any number of `Noise`, `Pattern`, `Value`, and named `Color` blocks. It has one required `Surface` and an optional preview-only `Output`.

## Authored and palette colors

Each named `Color` declares one of:

| Field | Behavior |
| --- | --- |
| `exact` or `base` | Keep the authored color. |
| `nearest` | Select the closest active-palette color using perceptual distance. |
| `source` | Evaluate a color expression such as `Mix`. |

Colors accept names, `#RRGGBB`, `#RRGGBBAA`, and normalized `F3(r, g, b)` values.

The `Surface.palette` policy controls the final result:

- `BaseOnly` palette-matches `nearest` anchors while preserving smooth procedural mixes.
- `Strict` maps the final mixed color back to the active palette.

Use `Strict` for tightly quantized pixel art. Use `BaseOnly` when cloth, metal, lighting, or gradual wear needs more tonal detail while still beginning from the project palette.

## Surface channels

| Field | Default | Purpose |
| --- | --- | --- |
| `color` | required | Named or inline color expression. |
| `palette` | `BaseOnly` | Final palette policy. |
| `roughness` | `1.0` | Surface roughness. |
| `metallic` | `0.0` | Metal response; `metal` is an alias. |
| `opacity` | `1.0` | Surface opacity. |
| `emission` | `0.0` | Emissive intensity; `emissive` is an alias. |
| `normal` | none | Scalar micro-height field. |
| `normal_strength` | `0.35` | Strength of the generated micro-normal. |

Scalar surface channels are clamped to `0..1`. Micro-normal data adds surface detail but does not replace the Tile's geometric height.

## Layering materials on a tile

`MaterialMap` fills the Tile with a base material and then blends complete material layers in declaration order:

```recipe
MaterialMap
  base = dungeon/mortar
  space = Global
  tiling = F2(1.0, 1.0)

  Layer
    material = dungeon/stone
    mask = Pow(Stones.height, 1.6)
    space = Stones.local
    tiling = F2(0.8, 0.8)
```

Every layer blends color and all renderer surface channels, not just RGB. The mask remains a Tile scalar expression. Its material can evaluate globally or independently inside each pattern unit.

## Reuse

The same material alias can currently be used by:

- a Tile's `material` shorthand;
- a Tile `MaterialMap` base or layer;
- an Avatar wearable's `appearance_recipe`;
- an SDF headgear item's `appearance_recipe`.

The consumer supplies the surface mapping, dimensions, palette, seed offset, and any mask. The material remains unaware of equipment slots, races, map cells, or UI states.
