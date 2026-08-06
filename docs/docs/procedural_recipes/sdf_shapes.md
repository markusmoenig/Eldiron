---
title: "SDF Shapes and Headgear"
sidebar_position: 7
---

SDF recipes describe reusable 2D coverage independently of color. The evaluator rasterizes an antialiased silhouette at the resolution requested by a consumer. A separate Material recipe colors that coverage.

This layer is experimental. Avatar headgear is its first runtime consumer.

## Shape recipe

```recipe
Sdf simple_helm
  name = "Simple Steel Helm"

  Shape Dome
    Ellipse
      position = F2(0.5, 0.36)
      size = F2(0.78, 0.52)

  Shape FaceOpening
    RoundedRectangle
      position = F2(0.5, 0.56)
      size = F2(0.58, 0.30)
      radius = 0.08

  Shape Shell
    Subtract
      a = Dome
      b = FaceOpening

  Shape Brow
    Capsule
      from = F2(0.14, 0.47)
      to = F2(0.86, 0.47)
      radius = 0.045

  Shape Helmet
    Union
      a = Shell
      b = Brow

  Output
    coverage = Helmet
```

Coordinates normally span `0..1` across the consumer-local surface. Primitive `position` values are centers and `size` contains full width and height. Optional primitive rotation is expressed in degrees.

## Implemented operations

| Operation | Fields | Meaning |
| --- | --- | --- |
| `Ellipse` | `position`, `size`, `rotation` | Elliptical primitive. |
| `RoundedRectangle` | `position`, `size`, `radius`, `rotation` | Rounded rectangular primitive. |
| `Capsule` | `from`, `to`, `radius` | Rounded line segment. |
| `Union` | `a`, `b` | Coverage from either shape. |
| `Subtract` | `a`, `b` | Shape `a` with shape `b` removed. |
| `Intersect` | `a`, `b` | Coverage shared by both shapes. |
| `Expand` | `source`, `amount` | Grow a shape. |
| `Contract` | `source`, `amount` | Shrink a shape. |

`Output.coverage` selects the final named shape. Unknown references and recursive shape graphs are errors.

## Headgear binding

An equipped head item supplies both shape and material aliases through ordinary item attributes:

```toml
[attributes]
slot = "head"
headgear_recipe = "armor/simple-helm"
appearance_recipe = "materials/worn-steel"
appearance_seed = 613
```

The surrounding item can be authored in Creator, supplied by a ruleset, or defined in a text project. Those authoring paths share the same runtime binding.

The Avatar adapter finds the rendered head-marker bounds, expands them slightly, maps the SDF into that local surface, evaluates the Material in the same coordinates, and composites the result as a foreground overlay. Because the bounds are derived separately for every frame, the overlay adapts to different head proportions and all eight directions.

The helmet is real equipment when it is defined and equipped as a ruleset or project item. The SDF itself remains only its visual silhouette; armor values, inventory behavior, slot rules, drops, and other gameplay properties belong to the item.

## Current limitations

- One foreground overlay is composited; there are no separate behind-head and in-front-of-face layers.
- Shapes are 2D coverage, not extruded geometry.
- Smooth boolean operations, strokes, repetition, and authored head anchors are not implemented yet.
- A complete SDF-authored Avatar body is future work.
- Headgear fitting uses marker bounds rather than authored per-direction guides.

Keeping shape and material separate is already useful: one helmet silhouette can be rendered as iron, bone, painted steel, or an emissive magical material without duplicating the SDF.
