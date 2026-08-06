# procedural-recipes

`procedural-recipes` is a small, human-readable recipe language and deterministic
renderer for procedural game assets.

The language is height-first: noises, patterns, and ordered height operations
produce scalar fields. Those fields can then drive color, surface properties,
and normals. Recipes are ordinary text files with the `.recipe` extension, so
they are easy to author, review, version, and generate.

The currently supported procedural assets are:

- **Tiles** — renderable images made from coordinated patterns and height fields,
  with optional collision, placement-time architectural geometry, named
  attachments, lights, and particle emitters.
- **Materials** — reusable color, roughness, metalness, opacity, emission, and
  normal definitions that can be referenced by many tiles.
- **SDF shapes** — reusable, resolution-independent 2D coverage silhouettes
  made from primitives and boolean operations. Consumers can place and color
  them independently, for example as Avatar headgear.

The same parser and evaluator are used by the command-line preview tool and by
Eldiron. This README is the canonical reference for the current format.

## A tile recipe

This tile builds a large, seamless stone surface. It delegates its appearance
and renderer material data to reusable stone and mortar materials.

```text
Tile
  name = "Dungeon Stone Wall"
  size = I2(128, 128)
  coverage = I2(2, 2)
  wrap = Repeat
  seed = 17

  Noise Warp
    type = Gradient
    fractal = FBm
    scale = F2(2.2, 2.2)
    octaves = 3
    persistence = 0.55

  Pattern Stones
    Voronoi
      cells = I2(5, 4)
      jitter = 0.82
      warp = Warp
      warp_amount = 0.08
      falloff = 1.4

  Height Surface
    source = Stones.height

    Shape
      contrast = 1.25
      rim = 0.08

    Add
      source = Warp
      amount = 0.05

    Clamp
      min = 0.0
      max = 1.0

  MaterialMap
    base = dungeon/mortar

    Layer
      material = dungeon/stone
      mask = Pow(Stones.height, 1.6)
      space = Stones.local

  Output
    height = Surface
```

`size` is the pixel size of one tile. `coverage` makes one coordinated
`2 × 2` image rather than four repeated copies of a smaller image. `Repeat`
keeps the outer edges seamless. The Voronoi pattern provides the stones, while
the same low-frequency noise gently distorts and varies their surface.

See [examples/stones.recipe](examples/stones.recipe) for a complete standalone
tile and [examples/bricks.recipe](examples/bricks.recipe) for a tile using a
referenced material. [examples/planks.recipe](examples/planks.recipe)
demonstrates a material evaluated independently inside every pattern unit.

Tile recipes can keep procedural props and their effects together. Attachments
use the same placement-local coordinates as `Geometry`; hosts transform the
complete program for a wall, ceiling, Avatar, or another target:

```text
Geometry
  Box Basket
    operation = Add
    surface = torch-iron
    position = F3(0.35, 1.32, -0.56)
    size = F3(0.30, 0.11, 0.20)

Attachment Flame
  position = F3(0.50, 1.58, -0.47)
  direction = F3(0.0, 1.0, 0.0)

Light Glow
  attach = Flame
  color = #ff9a45
  intensity = 1.75
  range = 5.2
  flicker = 0.22

Particles Fire
  attach = Flame
  rate = 25.0
  color_ramp = #fff2a8, #ffc14f, #f0641f, #401008
  lifetime = F2(0.32, 0.78)
  radius = F2(0.025, 0.065)
  speed = F2(0.28, 0.72)
```

Effects reference named attachments instead of material surfaces, so a surface
reused by several Boxes does not accidentally create duplicate emitters.

## A material recipe

Materials are also `.recipe` files. The normal layout uses one named material
per file, so the recipe and its preview have matching names. For example,
`recipes/dungeon/stone.recipe` contains:

```text
Material stone
  name = "Dungeon Stone"
  wrap = Repeat
  seed = 41

  Noise Grain
    type = Gradient
    fractal = Ridged
    scale = F2(9.0, 9.0)
    octaves = 3
    persistence = 0.48

  Color Shadow
    nearest = #292b2c

  Color Face
    nearest = F3(0.47, 0.48, 0.45)

  Color Stone
    source = Mix(Shadow, Face, Smoothstep(0.18, 0.82, Grain))

  Surface
    color = Stone
    palette = BaseOnly
    roughness = Clamp(0.66 + Grain * 0.26, 0.0, 1.0)
    metallic = 0.0
    opacity = 1.0
    emission = 0.0
    normal = Grain
    normal_strength = 0.30

  Output
    color = Stone
```

Materials do not receive tile height. They generate reusable color, renderer
channels, and micro-normal detail in their own coordinate space. The tile owns
geometry and uses `MaterialMap` masks to decide where complete materials
appear. All surface data fields accept scalar expressions.

The material reference is the recipe path without `.recipe`, relative to the
recipe root. A tile selects this file as a `MaterialMap` base or layer with
`dungeon/stone`. See
[examples/materials/stone.recipe](examples/materials/stone.recipe),
[examples/materials/mortar.recipe](examples/materials/mortar.recipe), and
[examples/materials/worn_metal.recipe](examples/materials/worn_metal.recipe),
[examples/materials/wood.recipe](examples/materials/wood.recipe), and
[examples/materials/marble.recipe](examples/materials/marble.recipe)
for complete examples.

A file may still group several `Material` declarations when that is useful.
Grouped materials use the reference `<file>/<material-id>` and the
`--material` selector described below.

## An SDF recipe

SDF recipes describe shape coverage, not color or surface properties. This
keeps geometry reusable: the same helmet silhouette can use hammered iron,
painted steel, bone, or magical materials without duplicating the shape.

```text
Sdf orc_helm
  name = "Orc Steel Helm"

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

  Output
    coverage = Shell
```

Coordinates are consumer-local and normally use `0..1` across the target
surface. Primitive `position` values are centers; `size` contains full width
and height. `rotation` is optional and expressed in degrees.

The initial shape blocks are:

| Block | Fields | Meaning |
| --- | --- | --- |
| `Ellipse` | `position`, `size`, `rotation` | Elliptical primitive. |
| `RoundedRectangle` | `position`, `size`, `radius`, `rotation` | Box with rounded corners. |
| `Capsule` | `from`, `to`, `radius` | Rounded line segment. |
| `Union` | `a`, `b` | Coverage from either named shape. |
| `Subtract` | `a`, `b` | Shape `a` with shape `b` removed. |
| `Intersect` | `a`, `b` | Coverage shared by both named shapes. |
| `Expand` | `source`, `amount` | Grow a named shape. |
| `Contract` | `source`, `amount` | Shrink a named shape. |

`Output.coverage` selects the final named shape. References are validated when
the recipe is parsed; recursive shape graphs are rejected by the evaluator.
The renderer returns antialiased 8-bit coverage at the resolution requested by
the consumer.

Avatar headgear is the first consumer. It derives a local surface from the
head marker bounding box in each rendered frame, evaluates the SDF there, then
colors the result with an ordinary Material recipe. Consequently placement
adapts to all eight Avatar directions and to different head proportions. The
current adapter composites one foreground overlay; explicit rear/front layer
splitting, smooth boolean operations, extrusion, and complete SDF-authored
Avatars remain future extensions.

## Command-line use

Validate a recipe:

```sh
procedural-recipes validate recipes/wall.recipe
```

Render a tile beside its recipe:

```sh
procedural-recipes render recipes/wall.recipe \
  --palette lospec:resurrect-64
```

Rendering an SDF recipe writes its coverage as a 256 × 256 grayscale preview:

```sh
procedural-recipes render recipes/orc-helm.recipe \
  --output orc-helm.png
```

When `--output` is omitted, `wall.recipe` becomes `wall.png`. Referenced
material files are resolved relative to the recipe root.

An uncolored tile does not need a palette:

```sh
procedural-recipes render recipes/heightmap.recipe
```

Without a tile `Colorize` block or referenced material, its height is written
directly as grayscale. A referenced material supplies its own colorization.
When that material uses `palette = BaseOnly`, authored `nearest` anchors provide
a self-contained fallback palette for CLI previews. Strict palette materials
require `--palette`.

Rerender automatically while editing:

```sh
procedural-recipes render recipes/heightmap.recipe \
  --watch
```

Watch mode renders immediately and then watches the recipe, its referenced
material recipe, and a local palette file. Invalid intermediate edits report an
error without stopping the watcher; the next saved change triggers another
attempt. Press `Ctrl-C` to stop.

Render an individual material beside its recipe:

```sh
procedural-recipes render recipes/dungeon/stone.recipe \
  --palette palette.hex \
  --output previews/stone.png
```

Without `--output`, `stone.recipe` renders to `stone.png`. For an intentionally
grouped material document, omit `--material` to render every declaration or
select one declaration with `--material <id>`. A grouped file named
`dungeon.recipe` produces names such as `dungeon-stone.png` and
`dungeon-iron.png`. Material previews use a broad seamless substrate so that
procedural color, surface data, and micro-normal detail are visible. A material
`Output` can temporarily replace that preview with a scalar or color debug
channel.

Write the evaluated height field as an additional image:

```sh
procedural-recipes render recipes/wall.recipe \
  --palette palette.hex \
  --height-output previews/wall-height.png
```

The optional `--palette` argument accepts either a local `.hex` palette or a
Lospec palette slug in the form `lospec:<slug>`. Animated recipes produce
numbered frames such as `wall-000.png`, `wall-001.png`, and so on.

# Language reference

## File and syntax rules

- Recipe files use the `.recipe` extension.
- A document contains exactly one `Tile`, one or more `Material` declarations,
  or one or more `Sdf` declarations. Document kinds cannot be mixed.
- Blocks are indentation-based. Use spaces; tabs are rejected.
- `//` starts a line comment.
- Declaration and field names are case-insensitive. The spelling used in this
  reference is canonical.
- Unknown fields, unknown references, duplicate fields, and cyclic scalar
  references are errors.
- Strings use double quotes: `"Dungeon Wall"`.
- Boolean values are `true` or `false`.
- `I2(x, y)` is a pair of positive integers where a size is expected.
- `F2(x, y)` is a pair of floating-point values.
- Identifiers begin with an ASCII letter or underscore and continue with ASCII
  letters, digits, or underscores.

## Document roots

### `Tile`

```text
Tile
  name = "Untitled Tile"
  placement = Surface
  blocking = false
  size = I2(64, 64)
  coverage = I2(1, 1)
  wrap = Repeat
  seed = 1
  pixelate = 1
```

| Field | Default | Meaning |
| --- | --- | --- |
| `name` | `"Untitled Tile"` | Human-readable name. |
| `placement` | `Surface` | `Surface` owns its host; `Fixture` overlays an existing host surface. |
| `blocking` | `false` | Whether map compilers should treat placements as solid by default. |
| `material` | none | Optional one-material shorthand; `MaterialMap` is used for layered surfaces. |
| `size` | `I2(64, 64)` | Pixel size of each tile. |
| `coverage` | `I2(1, 1)` | Number of coordinated tiles across and down. |
| `wrap` | `Repeat` | Sampling outside the recipe domain. |
| `seed` | `1` | Root deterministic seed. |
| `pixelate` | `1` | Positive sampling-pixel size. |

A tile may contain one `Animation`, any number of `Noise`, `Pattern`, and
`Height` blocks, and optional `Geometry`, `Colorize`, `MaterialMap`, and
`Output` blocks.

When `Output` is omitted, the last scalar-producing top-level block becomes the
output: a noise or height field uses its declared name, a pattern uses its
`.height` channel, and `Colorize` uses its `source`. If no such block exists,
the recipe is invalid.

When `Colorize`, `material`, and `MaterialMap` are all omitted, the renderer
outputs the heightmap directly as grayscale. `material` is a concise base-only
surface assignment. `MaterialMap` owns layered color, surface data, and
micro-normal composition. An inline tile `Colorize` is used only when no
material is assigned. When both `Colorize` and `Output` are present, their
sources must match.

The full rendered image size is `size × coverage`.

`placement = Fixture` makes geometry, attachments, lights, and particles a
host-independent overlay. The placement adapter retains the existing wall,
floor, or ceiling surface and instantiates only the fixture content. The baked
Tile remains an internal carrier for thumbnails and effect metadata; it is not
painted over the host. This allows a torch, switch, or door mechanism to sit on
top of a larger continuously mapped background Tile.

### `Geometry`

`Geometry` is a placement-local constructive program. Hosts supply a placement
basis, such as an exposed wall face or a ceiling cell, and execute the same
generic primitives and operations. Feature names such as niche or beam do not
exist in the geometry AST.

```text
Tile
  name = "Wall Niche"
  blocking = true

  Geometry
    Box Recess
      operation = Subtract
      surface = niche-stone
      position = F3(0.12, 0.76, 0.0)
      size = F3(0.76, 1.06, 0.46)

  Output
    height = 0.5
```

`Box <name>` is the first primitive. `Add` emits a solid and `Subtract` removes
its volume from the placement solid. `surface` is the Tile recipe used on the
emitted solid or newly exposed subtraction faces. `position`, `size`, and
`spacing` are placement-local `F3` world-unit vectors. `repeat` is an `I3`
instance count. Source maps wall-local X to the face tangent, Y to world up,
and Z into the wall; ceiling-local Y projects down into the room.

Every named primitive exposes `<name>.distance`. For a Box this is the signed
distance to its placement-local XY footprint: negative inside, zero on the
edge, and positive outside. This lets material masks follow constructive
geometry without duplicating its dimensions:

```text
Noise Wear
  scale = F2(11.0, 8.0)
  octaves = 3

Height Joint
  source = 1.0 - Smoothstep(0.025, 0.080, Abs(Recess.distance) + (Wear - 0.5) * 0.028)

MaterialMap
  base = materials/mortar

  Layer
    material = materials/stone
    mask = Stone.height * (1.0 - Joint)
```

| Field | Default | Meaning |
| --- | --- | --- |
| `operation` | `Add` | `Add` emits the Box; `Subtract` cuts it from the placement solid. |
| `surface` | required | Tile alias for emitted or newly exposed faces. |
| `position` | `F3(0, 0, 0)` | Placement-local minimum in world units. |
| `size` | `F3(1, 1, 1)` | Positive placement-local dimensions. |
| `repeat` | `I3(1, 1, 1)` | Instance count on each local axis, from 1 through 64. |
| `spacing` | `F3(0, 0, 0)` | Translation between repeated instances. |

### `Material`

```text
Material stone
  name = "Stone"
  wrap = Repeat
  seed = 1
```

| Field | Default | Meaning |
| --- | --- | --- |
| material identifier | required | Stable identifier; also used by grouped-file aliases and `--material`. |
| `name` | identifier | Human-readable name; underscores become spaces. |
| `wrap` | `Repeat` | Sampling outside the material domain. |
| `seed` | `1` | Root deterministic seed. |

A material may contain any number of `Noise`, `Pattern`, `Value`, and named
`Color` blocks, one required `Surface`, and one optional preview-only `Output`.
One material per file is the conventional layout and gives that material the
file-path alias. Multiple declarations remain allowed; grouped materials use
`<file>/<material-id>` aliases.

Materials are independent of tile geometry: `Input.height` is rejected in a
material. A material may use different scalar graphs for color,
roughness/metalness/opacity/emission, and micro-normal detail. Tile height and
material masks remain visible in the tile recipe.

## Wrapping

The `wrap` field accepts:

| Value | Behavior |
| --- | --- |
| `Clamp` | Hold sampling at the nearest outer edge. |
| `Repeat` | Wrap both axes seamlessly. |
| `Mirror` | Reflect alternate repetitions at each edge. |

Wrapping controls sampling outside the recipe domain. Pattern cells are
periodic within their own domain. `Repeat` is the normal choice for seamless
environment tiles.

## Scalar expressions

Scalar expressions are the common currency of the language. They can reference
noise, height fields, pattern channels, coordinates, and—in materials—the
incoming tile height.

```text
source = Clamp(Stones.height * 0.85 + Grain * 0.15, 0.0, 1.0)
```

### Values and operators

- Floating-point literals: `0.5`, `-1.25`
- Named noise, height, and value fields: `Grain`, `Surface`, `Tone`
- Pattern channels: `Stones.height`, `Stones.edge`, `Stones.center`
- Coordinates: `U`, `V`, `Radius`, `Angle`
- Binary operators: `+`, `-`, `*`, `/`
- Unary operators: `+`, `-`
- Parentheses with conventional arithmetic precedence

`U` and `V` are normalized coordinates. `Radius` is distance from the image
center. `Angle` is the normalized range `0..1` around that center. Division by
zero evaluates to zero, and final output channels are clamped to `0..1`.

Pattern `.id` is a stable discrete identity, not a directly renderable scalar.
Use it through `Random(...)`, `key`, or a pattern-local domain.

### Functions

| Function | Meaning |
| --- | --- |
| `Abs(x)` | Absolute value. |
| `Invert(x)` | `1 - x`. |
| `Sin(x)` | Sine where `x` is measured in cycles. |
| `Cos(x)` | Cosine where `x` is measured in cycles. |
| `Fract(x)` | Positive fractional part. |
| `Sqrt(x)` | Square root after clamping the input to zero or above. |
| `Min(a, b)` | Smaller value. |
| `Max(a, b)` | Larger value. |
| `Pow(a, b)` | Power after clamping the base to zero or above. |
| `Clamp(x, min, max)` | Clamp to the supplied range. |
| `Mix(a, b, factor)` | Linear interpolation; factor is clamped to `0..1`. |
| `Smoothstep(min, max, x)` | Smooth Hermite transition. |
| `Random(id, min, max)` | Stable random value for a pattern unit. |
| `Random(id, min, max, seed)` | Stable random value with an extra integer seed. |
| `Wave(min, max, cycles)` | Animated value over normalized frame time. |
| `Wave(min, max, cycles, phase)` | Animated value with phase offset. |

`Random` accepts `Id`, `Current.id`, or `<Pattern>.id` as its first argument.
Its range and optional non-negative integer seed are literals. `Wave`
arguments are literals and only vary when the tile has multiple animation
frames.

## `Animation`

```text
Animation
  frames = 8
  fps = 12.0
  looping = true
```

| Field | Default | Range |
| --- | --- | --- |
| `frames` | `1` | `1..1024` |
| `fps` | `12.0` | greater than zero |
| `looping` | `true` | boolean |

`Wave(...)` receives normalized animation time. Rendered frames retain the FPS
and looping metadata for consuming applications.

## `Noise`

```text
Noise Grain
  type = Gradient
  fractal = FBm
  scale = F2(8.0, 8.0)
  octaves = 4
  persistence = 0.5
  seed = 3
  space = Global
```

| Field | Default | Supported values or range |
| --- | --- | --- |
| `type` | `Value` | `Value`, `Gradient`, `Perlin` (`Perlin` aliases `Gradient`) |
| `kind` | — | Alias for `type`; do not specify both. |
| `fractal` | `FBm` | `FBm`, `Ridged`, `Billow`, `Turbulence` |
| `scale` | `F2(4.0, 4.0)` | Positive frequency on each axis. |
| `octaves` | `1` | `1..8` |
| `persistence` | `0.5` | `0..1` |
| `seed` | `0` | Additional deterministic integer seed. |
| `space` | `Global` | `Global` or `<Pattern>.local` |
| `key` | none | `Id`, `Current.id`, or `<Pattern>.id` |

Noise is periodic and respects the recipe's wrapping. A pattern-local noise
domain restarts coordinates inside each pattern unit. `key` adds stable
per-unit variation without causing the result to shimmer or change when other
units are added. When a noise with `key = Id` is evaluated globally, the missing
unit ID resolves to stable key `0`, allowing the noise to be output directly
for debugging. Other uses of `Id`, such as `Random(Id, ...)`, still require a
pattern-unit context in tile graphs. Complete material evaluation supplies
stable `Id = 0` under `Global` and the tile pattern's ID under a local material
binding.

## `Pattern`

```text
Pattern Stones
  Voronoi
    cells = I2(5, 4)
    jitter = 0.8
    falloff = 1.4
```

Every `Pattern <name>` contains exactly one generator. All generators expose:

| Channel | Meaning |
| --- | --- |
| `<Pattern>.height` | Filled shape height. |
| `<Pattern>.edge` | Emphasis near unit boundaries. |
| `<Pattern>.center` | Emphasis near unit centers. |
| `<Pattern>.id` | Stable unit identity for `Random`, `key`, and local domains. |

All pattern generators accept these common fields:

| Field | Default | Meaning |
| --- | --- | --- |
| `space` | `Global` | Use global coordinates or `<OtherPattern>.local`. |
| `key` | none | Key variation with `Id`, `Current.id`, or `<Pattern>.id`. |
| `bevel` | `0.08` | Width of the boundary-to-flat-face transition, clamped to `0..1`; `0` produces a hard, flat profile. |
| `warp` | none | Scalar field used as a two-axis coordinate warp before generating the pattern. |
| `warp_amount` | `0.05` | Coordinate displacement, clamped to `0..1`; requires `warp`. |
| `perturb` | none | Scalar field used to disturb the generated unit boundary. |
| `perturb_amount` | `0.05` | Boundary displacement, clamped to `0..0.5`; requires `perturb`. |
| `seed` | `0` | Additional deterministic integer seed. |

A pattern-local space evaluates once within every unit of another pattern.
`warp` and `perturb` are available on every pattern generator, but operate at
different stages:

- `warp` changes coordinates before unit selection. It bends the entire pattern
  coherently. A noise using `key = Id` resolves that key to `0` at this stage
  because the current pattern unit does not exist yet.
- `bevel` shapes the boundary profile after unit selection and works identically
  for bricks, Voronoi cells, and discs.
- `perturb` changes the boundary profile after unit selection. Its source is
  sampled in unit-local coordinates and may use `key = Id` for an independent,
  stable variation per unit. A value of `0.5` causes no displacement.

### `Bricks`

```text
Pattern Masonry
  Bricks
    columns = 6
    rows = 5
    stagger = 0.5
    gap = 0.07
    bevel = 0.08
    rounding = 0.04
    rotation = 0.0
    size_variation = F2(0.10, 0.06)
    falloff = 1.2
```

| Field | Default | Meaning |
| --- | --- | --- |
| `columns` | `8` | Positive number of columns. |
| `rows` | `8` | Positive number of rows. |
| `stagger` | `0.5` | Horizontal offset of alternating rows. |
| `gap` | `0.08` | Scalar mortar/gap width, clamped to `0..0.95`. |
| `rounding` | `0.04` | Scalar corner rounding, clamped to `0..0.49`. |
| `rotation` | `0.0` | Scalar rotation in degrees. |
| `size_variation` | `F2(0.0, 0.0)` | Per-brick variation, clamped to `0..0.45`. |
| `falloff` | `1.0` | Scalar edge profile, clamped to `0.1..8`. |

The fields described as scalar accept full scalar expressions.

`gap = 0` makes neighboring brick shapes touch. The common `bevel` modifier
independently controls the narrow transition from the boundary to the flat
face. `falloff` controls the curve within that bevel only.

### `Voronoi`

```text
Pattern StoneCells
  Voronoi
    cells = I2(6, 6)
    jitter = 0.8
    warp = LowNoise
    warp_amount = 0.05
    falloff = 1.0
```

| Field | Default | Meaning |
| --- | --- | --- |
| `cells` | `I2(6, 6)` | Positive cell count. |
| `jitter` | `0.8` | Site displacement, clamped to `0..1`. |
| `falloff` | `1.0` | Edge profile, clamped to `0.1..8`. |

### `Discs`

```text
Pattern Pebbles
  Discs
    cells = I2(8, 8)
    jitter = 0.9
    radius = 0.42
    warp = LowNoise
    warp_amount = 0.04
    falloff = 1.3
```

| Field | Default | Meaning |
| --- | --- | --- |
| `cells` | `I2(8, 8)` | Positive cell count. |
| `jitter` | `1.0` | Scalar site displacement, clamped to `0..1`. |
| `radius` | `0.5` | Scalar disc radius, clamped to `0.01..2`. |
| `falloff` | `1.0` | Scalar edge profile, clamped to `0.1..8`. |

For every pattern, specifying `warp_amount` requires `warp`, and specifying
`perturb_amount` requires `perturb`.

### Debugging pattern IDs

Pattern IDs are 64-bit discrete values and cannot be used directly as scalar
height. Map each ID to a stable grayscale value with `Random`:

```text
Output
  height = Random(Wall.id, 0.0, 1.0)
```

Remove or comment out `Colorize` to see the values directly in grayscale. Each
brick is then a flat shade, which makes unit boundaries and ID stability easy
to inspect. These shades are deterministic debug representations, not the
literal numeric IDs; the 8-bit image can give distant units the same shade.

## `Height`

A tile height block begins with a scalar source and applies its child operations
in declaration order:

```text
Height Surface
  source = Stones.height

  Shape
    contrast = 1.3
    rim = 0.08

  Add
    source = Grain
    amount = 0.1

  Terrace
    steps = 6
    smoothness = 0.25
```

### Height operations

| Operation | Fields and defaults | Behavior |
| --- | --- | --- |
| `Shape` | `contrast = 1`, `bias = 0`, `plateau = 0`, `rim = 0` | Reshapes a surface and can flatten its center or raise its edge. |
| `Add` | required `source`, `amount = 1` | Adds `source × amount`. |
| `Subtract` | required `source`, `amount = 1` | Subtracts `source × amount`. |
| `Multiply` | required `source`, `amount = 1` | Blends from unchanged to multiplied by source. |
| `Min` | required `source`, `amount = 1` | Blends from unchanged to the smaller value. |
| `Max` | required `source`, `amount = 1` | Blends from unchanged to the larger value. |
| `Clamp` | `min = 0`, `max = 1` | Clamps to scalar limits. |
| `Remap` | `from = F2(0,1)`, `to = F2(0,1)` | Maps one constant range into another. |
| `Terrace` | `steps = 4`, `smoothness = 0` | Quantizes to `2..64` levels with optional smoothing. |
| `Invert` | no fields | Replaces the value with `1 - value`. |

`Shape.contrast` is clamped to `0.1..8`, `bias` to `-1..1`, and `plateau`
and `rim` to `0..4`. Height output is clamped to `0..1`.

`Height` is reserved for tile geometry and is rejected inside materials.

## `Value`

A material uses named values for intermediate scalar calculations:

```text
Value DetailTone
  source = Smoothstep(0.18, 0.82, Mineral)

Value UnitTone
  source = Random(Id, 0.0, 1.0, 37)
```

`Value` has one required scalar `source` and no child operations. It does not
alter tile geometry. Named values can drive color mixes, surface channels,
micro-normal detail, and material preview output.

## `Colorize`

The optional tile `Colorize` block maps a scalar field to colors from the active
project palette or to a gradient anchored in that palette. Without it, a tile
renders its heightmap as grayscale. Material recipes use named `Color` values
and a `Surface` block instead.

There are two colorization styles.

### Base-color ramp

```text
Colorize
  source = Surface
  base = DarkBrown
  palette = BaseOnly
  brightness = F2(-0.22, 0.28)
  saturation = F2(-0.08, 0.12)
  hue = F2(-0.01, 0.01)
  steps = 12
  range = Auto
  dither = false
```

The authored `base` is first matched to the active project palette:

- `palette = Strict` maps every generated ramp step back to the palette.
- `palette = BaseOnly` maps only the base color to the palette, then retains
  the smooth HSL gradient around that mapped anchor. This deliberately permits
  colors between palette entries.

| Field | Default | Meaning |
| --- | --- | --- |
| `source` | required | Scalar field to colorize. |
| `base` | none | Named color, `#RRGGBB`, or `#RRGGBBAA`. |
| `palette` | `Strict` | `Strict` or `BaseOnly`. |
| `brightness` | `F2(-0.22, 0.22)` | Low/high lightness offsets, clamped to `-1..1`. |
| `saturation` | `F2(-0.08, 0.08)` | Low/high saturation offsets, clamped to `-1..1`. |
| `hue` | `F2(0.0, 0.0)` | Low/high hue offsets, clamped to `-1..1`. |
| `steps` | `4` | Ramp steps, clamped to `2..256`. |
| `range` | `Auto` | `Auto` or `F2(min, max)`. |
| `dither` | `false` | Apply ordered dithering between steps. |

Supported named base colors are:

```text
Black        Charcoal     DarkGray     Gray          LightGray     White
DarkBrown    Brown        LightBrown   DarkRed       Red           Orange
Gold         Yellow       Olive        DarkGreen     Green         Teal
Cyan         DarkBlue     Blue         Purple        Magenta
```

`DarkGrey`, `Grey`, and `LightGrey` are aliases for their `Gray` spellings.

### Palette-family ramp

```text
Colorize
  source = Surface
  palette = Strict
  ramp = Earth
  ramp_range = F2(0.08, 0.92)
  saturation_range = F2(0.20, 0.85)
  steps = 8
  range = F2(0.1, 0.9)
  dither = true
```

Without `base`, colors are selected directly from a palette family:

| Field | Default | Supported values |
| --- | --- | --- |
| `ramp` | `Any` | `Any`, `Neutral`, `Warm`, `Cool`, `Earth`, `Red`, `Orange`, `Yellow`, `Green`, `Cyan`, `Blue`, `Purple`, `Magenta` |
| `ramp_range` | `F2(0, 1)` | Fraction of the ordered family ramp. |
| `saturation_range` | `F2(0, 1)` | Allowed saturation interval. |

Family mode is palette-strict. `BaseOnly` requires `base`. Base mode cannot be
combined with `ramp`, `ramp_range`, or `saturation_range`; family mode cannot
use `brightness`, `saturation`, or `hue`.

`range = Auto` normalizes from the evaluated field's minimum and maximum.
An explicit range gives stable authored thresholds. Dithering uses an ordered
4 × 4 pattern.

## `Output`

A tile can explicitly select its output:

```text
Output
  height = Surface
```

When present, `height` is a required scalar expression and is clamped to
`0..1`. It becomes the tile's final geometric height and the optional
`--height-output` image.

`space` optionally evaluates that scalar inside a pattern unit:

```text
Output
  height = LocalWarp
  space = Wall.local
```

The default is `space = Global`. A `<Pattern>.local` space supplies both the
pattern's local coordinates and its current `Id`. This makes it possible to
preview the exact keyed noise used by a pattern modifier:

Local-space and `.id` references resolve pattern placement only; they do not
evaluate that pattern's height or modifiers. A child pattern can therefore use
`space = Wall.local` and shape `Wall` without creating a height dependency
cycle. A genuine height loop, such as `Wall` using `Wall.height`, remains an
error.

```text
Noise LocalWarp
  key = Id
  scale = F2(2.0, 2.0)

Pattern Wall
  Bricks
    perturb = LocalWarp
    perturb_amount = 0.02
```

When the block is absent, the last scalar-producing top-level block is selected
implicitly. An explicit `Output` is useful when the desired result is not the
last block or is a composed scalar expression.

In a material, `Output` is optional and controls only direct previews:

```text
Output
  value = Grain
  space = Global
```

This renders the selected scalar as grayscale. To inspect a color graph:

```text
Output
  color = StoneColor
```

`value` and `color` are mutually exclusive. A material `Output` is ignored
when a tile references that material, so debugging never changes runtime
surface composition.

## `Color`

Named colors make material color graphs editable and reusable:

```text
Color Shadow
  nearest = #324524

Color Face
  nearest = F3(0.52, 0.58, 0.45)

Color StoneColor
  source = Mix(Shadow, Face, Mineral)
```

Each `Color <name>` declares exactly one of:

| Field | Meaning |
| --- | --- |
| `base` or `exact` | Use an exact authored color. |
| `nearest` | Replace the authored color with the perceptually nearest active-palette color. |
| `source` | Evaluate a color expression. |

Authored colors accept standard names, `#RRGGBB`, `#RRGGBBAA`, or normalized
`F3(r, g, b)`. Nearest-color matching uses perceptual OKLab distance.

Color expressions accept color names, inline exact colors,
`Nearest(color)`, `Exact(color)`, and:

```text
Mix(ColorA, ColorB, scalar_mask)
```

The mask is clamped to `0..1`. Color interpolation uses linear RGB. Multiple
mixes can be nested or assigned to intermediate named colors. With
`palette = BaseOnly`, nearest anchors are palette-matched but their gradients
remain smooth. `palette = Strict` maps the final mixed color back to the active
palette.

## `Surface`

Every material has one unified final surface block:

```text
Surface
  color = StoneColor
  palette = BaseOnly
  roughness = Mix(0.45, 0.95, Grain)
  metallic = 0.0
  opacity = 1.0
  emission = EdgeGlow
  normal = Grain
  normal_strength = 0.25
```

| Field | Default | Meaning |
| --- | --- | --- |
| `color` | required | Named or inline color expression. |
| `palette` | `BaseOnly` | `BaseOnly` keeps smooth mixes; `Strict` quantizes the final color. |
| `roughness` | `1.0` | Scalar roughness. |
| `metallic` | `0.0` | Scalar metalness; alias `metal`. |
| `opacity` | `1.0` | Scalar opacity. |
| `emissive` | `0.0` | Scalar emission; alias `emission`. |
| `normal` | none | Optional scalar micro-height field. |
| `normal_strength` | `0.35` | Micro-normal strength, clamped to `0..8`; requires `normal`. |

All scalar surface channels are clamped to `0..1`. Do not specify a field and
its alias together. Materials never alter tile height; their `normal` is
small-scale surface detail combined with the tile’s geometric normal.

## `MaterialMap`

```text
MaterialMap
  base = materials/mortar
  space = Global
  tiling = F2(1.0, 1.0)

  Layer
    material = materials/stone
    mask = Pow(Wall.height, 1.6)
    space = Wall.local
    tiling = F2(0.75, 0.75)
```

`base` fills the complete tile. Its optional `space` defaults to `Global`, and
its optional `tiling` defaults to `F2(1.0, 1.0)`. Layers are then evaluated in
declaration order. Every layer also accepts an independent `space` and
`tiling`, in addition to its required `material` and `mask`.

`Global` evaluates material coordinates continuously across the complete tile.
`<Pattern>.local` resets material `U` and `V` to `0..1` inside every unit of
that tile pattern. The unit's stable identity is exposed as `Id` while
evaluating the complete material, including its colors, noises, roughness,
metallicity, opacity, emission, and normal field:

```text
MaterialMap
  base = materials/mortar

  Layer
    material = materials/wood
    mask = Planks.height
    space = Planks.local
    tiling = F2(0.45, 0.65)
```

A reusable material can therefore use `key = Id` on noise and
`Random(Id, ...)` in scalar or color-mix expressions. It receives deterministic
variation for every local plank, brick, or stone. Under `Global`, material
`Id` resolves to stable value `0`, so the same material remains valid as one
continuous surface.

`tiling` scales coordinates after selecting the global or pattern-local domain
and before applying the material's wrap mode. Values above `1` repeat the
material more frequently; positive values below `1` enlarge it and sample less
than one authored repeat. This is especially useful for keeping grain, pores,
and veins at a sensible visual scale inside long, thin pattern units. Both
components must be finite and greater than zero.

Each `mask` remains a tile scalar expression and is clamped to `0..1`; changing
the material space does not change mask evaluation. A layer blends the complete
material—linear-RGB color, roughness, metallic, opacity, emission, and
micro-normal detail. Use scalar functions such as `Pow` and `Smoothstep` to
control transition sharpness explicitly.

## Determinism, domains, and variation

A recipe produces the same result for the same recipe text, root seed, render
options, and—when colorized—palette. Random-looking variation should be
expressed through declared seeds and stable pattern identities.

Use these mechanisms together:

- Root `seed` changes the overall tile or material.
- Per-noise and per-pattern `seed` decorrelates individual generators.
- `space = <Pattern>.local` creates detail in the coordinate system of each
  pattern unit.
- `key = <Pattern>.id` gives those units stable independent variants.
- `Random(<Pattern>.id, min, max, seed)` varies scalar parameters per unit.
- `coverage` lets large-scale structure span several output tiles.

This keeps irregular materials coherent and reproducible without hardcoding
specialized “wood,” “marble,” or “stone” generators. Such materials are built
by composing the same noises, patterns, domains, expressions, and ramps.

## Diagnostics

Recipe diagnostics have stable machine-readable codes, one-based line and
column numbers, and the offending source line:

```text
error[PR0008]: unknown pattern 'Missing'
 --> recipes/wall.recipe:14:9
    |
 14 |         source = Missing.height
    |         ^
```

The explanatory text may improve over time. Integrations should match
`ParseError.code` or `ParseError::stable_code()` instead of parsing that text.

| Code | `ParseErrorCode` | Meaning |
| --- | --- | --- |
| `PR0001` | `Syntax` | Invalid recipe or scalar-expression syntax. |
| `PR0002` | `Document` | Invalid or incompatible document root. |
| `PR0003` | `MissingRequired` | A required declaration, block, or field is missing. |
| `PR0004` | `DuplicateDefinition` | A field, block, identifier, or definition is duplicated. |
| `PR0005` | `UnknownConstruct` | An unknown block, field, operation, function, or channel was used. |
| `PR0006` | `InvalidValue` | A value has the wrong type, format, or supported choice. |
| `PR0007` | `ConflictingFields` | Individually valid fields cannot be used together. |
| `PR0008` | `UnknownReference` | A scalar, pattern, domain, or stable-ID reference cannot be resolved. |

`ParseError` exposes `code`, `line`, `column`, `message`, `source_line`, and
`source_name`. Parsers attach source text automatically. A host can add its
filename or asset name with `with_source_name(...)`; the CLI does this for
recipe paths.

Fields left over after changing one pattern generator to another are ignored
and reported as `warning[PRW0001]`. This keeps live previews renderable while
the recipe is being edited. Unknown fields in other blocks, malformed values,
unknown generators, and missing dependencies remain errors.

## Rust API

The crate exposes the parser, AST, palette model, and renderer:

```rust
use procedural_recipes::{
    parse_document, RecipeDocument, RecipeRenderer, RenderOptions,
};

let source = std::fs::read_to_string("recipes/wall.recipe")?;
let document = parse_document(&source)?;

if let RecipeDocument::Tile(recipe) = document {
    let renderer = if recipe.colorize.is_some()
        || recipe.material.is_some()
        || recipe.material_map.is_some()
    {
        RecipeRenderer::new(&palette)?
    } else {
        RecipeRenderer::grayscale()
    };
    let rendered = renderer.render(&recipe, &RenderOptions::default())?;
    // rendered.frames contains the tile's color and height data.
}
```

The principal parsing entry points are:

- `parse_document` — parse either supported document kind.
- `parse_recipe` — parse a tile document.
- `parse_material_document` — parse a material document.
- `parse_sdf_document` — parse an SDF document.

`RecipeRenderer::render` evaluates tiles, and
`RecipeRenderer::render_material` evaluates reusable materials. Use
`RenderOptions::seed_offset` to request a deterministic variant without
rewriting the recipe. Material rendering additionally produces the four
renderer material channels and the micro-normal height field.
`render_material_preview` honors a material’s debug `Output`;
`render_material` deliberately ignores it. `render_material_in_space` evaluates
the same complete material using a tile's global or pattern-local context and
binding tiling.
`render_scalar_field` evaluates tile layer masks, and
`RenderedMaterial::blend_layer` composes complete materials.
`SdfRenderer::render` evaluates an `SdfRecipe` on any `RenderSurface` and
returns consumer-neutral 8-bit coverage.

## Current scope

The format currently compiles procedural **tiles**, **materials**, and 2D
**SDF coverage shapes**. The
language deliberately describes reusable procedural fields rather than naming
fixed visual algorithms such as marble or wood. Future procedural asset types
can build on the same surfaces, expressions, deterministic domains, palette
mapping, shape coverage, and material outputs while keeping existing recipes
readable.
