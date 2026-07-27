# procedural-recipes

`procedural-recipes` is a small, human-readable recipe language and deterministic
renderer for procedural game assets.

The language is height-first: noises, patterns, and ordered height operations
produce scalar fields. Those fields can then drive color, surface properties,
and normals. Recipes are ordinary text files with the `.recipe` extension, so
they are easy to author, review, version, and generate.

The currently supported procedural assets are:

- **Tiles** — renderable images made from coordinated patterns and height fields.
- **Materials** — reusable color, roughness, metalness, opacity, emission, and
  normal definitions that can be referenced by many tiles.

The same parser and evaluator are used by the command-line preview tool and by
Eldiron. This README is the canonical reference for the current format.

## A tile recipe

This tile builds a large, seamless stone surface. It delegates its appearance
and renderer material data to the reusable `stone` material in
`recipes/dungeon.recipe`.

```text
Tile
  name = "Dungeon Stone Wall"
  material = dungeon/stone
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

  Output
    height = Surface
```

`size` is the pixel size of one tile. `coverage` makes one coordinated
`2 × 2` image rather than four repeated copies of a smaller image. `Repeat`
keeps the outer edges seamless. The Voronoi pattern provides the stones, while
the same low-frequency noise gently distorts and varies their surface.

See [examples/stones.recipe](examples/stones.recipe) for a complete standalone
tile and [examples/bricks.recipe](examples/bricks.recipe) for a tile using a
referenced material.

## A material recipe

Materials are also `.recipe` files. A single file may declare several named
materials:

```text
Material stone
  name = "Dark Brown Dungeon Stone"
  wrap = Repeat
  seed = 41

  Noise Grain
    type = Gradient
    fractal = Ridged
    scale = F2(9.0, 9.0)
    octaves = 3
    persistence = 0.48

  Height Tone
    source = Input.height

    Add
      source = Grain
      amount = 0.10

    Clamp
      min = 0.0
      max = 1.0

  Colorize
    source = Tone
    base = DarkBrown
    palette = BaseOnly
    brightness = F2(-0.20, 0.28)
    saturation = F2(-0.10, 0.12)
    steps = 12
    range = Auto
    dither = false

  MaterialData
    roughness = 0.88
    metallic = 0.0
    opacity = 1.0
    emissive = 0.0

  Normal
    source = Tone
    strength = 0.65

Material iron
  name = "Worn Iron"

  Colorize
    source = Input.height
    base = Charcoal
    palette = BaseOnly
    brightness = F2(-0.08, 0.24)
    saturation = F2(-0.05, 0.02)
    steps = 10

  MaterialData
    roughness = 0.42
    metallic = 0.92
```

`Input.height` is the final height produced by the referencing tile. The first
material adds its own grain before using that field for color and normals.
Material data fields accept scalar expressions too, so roughness or emission
can vary across the surface instead of being constant.

If this file is `dungeon.recipe`, its material identifiers are `stone` and
`iron`. A tile can select one of them through a resolved material reference
such as `dungeon/stone`. See
[examples/materials.recipe](examples/materials.recipe) for a larger example.

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

When `--output` is omitted, `wall.recipe` becomes `wall.png`. Referenced
material files are resolved relative to the recipe root.

Render one material from a multi-material document:

```sh
procedural-recipes render recipes/dungeon.recipe \
  --material stone \
  --palette palette.hex \
  --output previews/stone.png
```

Omit `--material` to render every material. A multi-material file named
`dungeon.recipe` then produces names such as `dungeon-stone.png` and
`dungeon-iron.png`. Material previews use a broad seamless relief substrate so
that height-responsive color, surface data, and normals are visible.

Write the evaluated height field as an additional image:

```sh
procedural-recipes render recipes/wall.recipe \
  --palette palette.hex \
  --height-output previews/wall-height.png
```

The palette argument accepts either a local `.hex` palette or a Lospec palette
slug in the form `lospec:<slug>`. Animated recipes produce numbered frames such
as `wall-000.png`, `wall-001.png`, and so on.

# Language reference

## File and syntax rules

- Recipe files use the `.recipe` extension.
- A document contains either exactly one `Tile`, or one or more `Material`
  declarations. Tiles and materials cannot be mixed in the same document.
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
  material = dungeon/stone
  size = I2(64, 64)
  coverage = I2(1, 1)
  wrap = Repeat
  seed = 1
  pixelate = 1
```

| Field | Default | Meaning |
| --- | --- | --- |
| `name` | `"Untitled Tile"` | Human-readable name. |
| `material` | none | Optional resolved material reference. |
| `size` | `I2(64, 64)` | Pixel size of each tile. |
| `coverage` | `I2(1, 1)` | Number of coordinated tiles across and down. |
| `wrap` | `Repeat` | Sampling outside the recipe domain. |
| `seed` | `1` | Root deterministic seed. |
| `pixelate` | `1` | Positive sampling-pixel size. |

A tile may contain one `Animation`, any number of `Noise`, `Pattern`, and
`Height` blocks, one `Output`, and a `Colorize` block when it does not reference
a material. A tile with a material may still provide `Colorize` as a fallback.
For a standalone tile, `Colorize.source` must match `Output.height`.

The full rendered image size is `size × coverage`.

### `Material`

```text
Material stone
  name = "Stone"
  wrap = Repeat
  seed = 1
```

| Field | Default | Meaning |
| --- | --- | --- |
| material identifier | required | Stable identifier used by aliases and `--material`. |
| `name` | identifier | Human-readable name; underscores become spaces. |
| `wrap` | `Repeat` | Sampling outside the material domain. |
| `seed` | `1` | Root deterministic seed. |

A material may contain any number of `Noise`, `Pattern`, and `Height` blocks,
one required `Colorize`, and optional `MaterialData` and `Normal` blocks.
Multiple `Material` declarations are allowed in one file.

Unlike a tile, a material may use different scalar graphs for color,
roughness/metalness/opacity/emission, and normals. `Input.height` refers to the
final height field supplied by the tile or preview substrate.

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
- Named noise and height fields: `Grain`, `Surface`
- Pattern channels: `Stones.height`, `Stones.edge`, `Stones.center`
- Material input: `Input.height`
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
units are added.

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
| `seed` | `0` | Additional deterministic integer seed. |

A pattern-local space evaluates once within every unit of another pattern.

### `Bricks`

```text
Pattern Masonry
  Bricks
    columns = 6
    rows = 5
    stagger = 0.5
    gap = 0.07
    rounding = 0.04
    rotation = 0.0
    size_variation = F2(0.10, 0.06)
    perturb = Grain
    perturb_amount = 0.08
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
| `perturb` | none | Scalar field used to disturb brick boundaries. |
| `perturb_amount` | `0.0` | Boundary disturbance, clamped to `0..0.5`. |
| `falloff` | `1.0` | Scalar edge profile, clamped to `0.1..8`. |

The fields described as scalar accept full scalar expressions. `Bricks` does
not currently provide the coordinate `warp` fields used by `Voronoi` and
`Discs`; use `perturb` for irregular brick boundaries.

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
| `warp` | none | Scalar field used to distort coordinates. |
| `warp_amount` | `0.05` | Distortion strength, clamped to `0..1`. |
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
| `warp` | none | Scalar field used to distort coordinates. |
| `warp_amount` | `0.05` | Distortion strength, clamped to `0..1`. |
| `falloff` | `1.0` | Scalar edge profile, clamped to `0.1..8`. |

For `Voronoi` and `Discs`, specifying `warp_amount` requires `warp`.

## `Height`

A height block begins with a scalar source and applies its child operations in
declaration order:

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

## `Colorize`

`Colorize` maps a scalar field to colors from the active project palette or to
a gradient anchored in that palette.

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

Every tile ends with one output block:

```text
Output
  height = Surface
```

`height` is a required scalar expression and is clamped to `0..1`. It becomes
the tile's final height, the source for a referenced material's
`Input.height`, and the optional `--height-output` image.

## `MaterialData`

```text
MaterialData
  roughness = Mix(0.45, 0.95, Grain)
  metallic = 0.0
  opacity = 1.0
  emissive = EdgeGlow
```

All fields accept scalar expressions and are clamped to `0..1`.

| Field | Default | Alias |
| --- | --- | --- |
| `roughness` | `0.5` | — |
| `metallic` | `0.0` | `metal` |
| `opacity` | `1.0` | — |
| `emissive` | `0.0` | `emission` |

Do not specify a field and its alias together. Rendered material data stores
the channels in roughness, metallic, opacity, emissive order.

`Data` is accepted as a shorter alias for the `MaterialData` block name.

## `Normal`

```text
Normal
  source = Surface
  strength = 0.6
```

| Field | Default | Meaning |
| --- | --- | --- |
| `source` | `Input.height` | Scalar field from which normals are generated. |
| `strength` | `0.35` | Normal intensity, clamped to `0..8`. |

Normals are generated automatically from the selected scalar field. Authors
control the source and strength rather than supplying normal vectors directly.

## Determinism, domains, and variation

A recipe produces the same result for the same recipe text, root seed, palette,
and render options. Random-looking variation should be expressed through
declared seeds and stable pattern identities.

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

## Rust API

The crate exposes the parser, AST, palette model, and renderer:

```rust
use procedural_recipes::{
    parse_document, RecipeDocument, RecipeRenderer, RenderOptions,
};

let source = std::fs::read_to_string("recipes/wall.recipe")?;
let document = parse_document(&source)?;

if let RecipeDocument::Tile(recipe) = document {
    let renderer = RecipeRenderer::new(&palette)?;
    let rendered = renderer.render(&recipe, &RenderOptions::default())?;
    // rendered.frames contains the tile's color and height data.
}
```

The principal parsing entry points are:

- `parse_document` — parse either supported document kind.
- `parse_recipe` — parse a tile document.
- `parse_material_document` — parse a material document.

`RecipeRenderer::render` evaluates tiles, and
`RecipeRenderer::render_material` evaluates reusable materials. Use
`RenderOptions::seed_offset` to request a deterministic variant without
rewriting the recipe. Material rendering additionally produces the four
renderer material channels and the normal-height field.

## Current scope

The format currently compiles procedural **tiles** and **materials**. The
language deliberately describes reusable procedural fields rather than naming
fixed visual algorithms such as marble or wood. Future procedural asset types
can build on the same expressions, deterministic domains, palette mapping, and
material outputs while keeping existing recipes readable.
