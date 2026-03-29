# tilegraph

`tilegraph` is [Eldiron’s](https://crates.io/crates/eldiron-creator) procedural tile graph crate for creating retro, wrapping tiles and tile groups for walls, floors, and other repeating surfaces.

![stones output](examples/stones.png)

It contains:

- the human-readable `.tilegraph` document format
- the graph runtime and evaluator
- the renderer for grouped procedural tile output
- the `tilegraph` CLI binary for rendering `.tilegraph` files

## What It Is

`tilegraph` is built around a height-first workflow for procedural retro tile generation.

Typical flow:

1. layout nodes generate a structural field such as `Height`, `Center`, and `Cell Id`
2. shaping nodes sculpt the height field
3. color nodes map the result to palette colors
4. output writes color, height, and material channels

The resulting graph can generate:

- a single tile
- a tile group such as `2x2` or `3x3`
- matching height-driven normals and packed material data

## Format

The portable graph format is TOML-based and intended to stay readable and diffable.

Node definitions look like:

```toml
[node.voronoi.main]
scale = 0.349
seed = 11
```

Connections are embedded directly in node fields:

```toml
[node.output.main]
color = "colorize4.main:field"
height = "subtract.main:field"
```

## CLI

The package also contains a CLI binary:

```bash
cargo run -p tilegraph -- crates/tilegraph/examples/stones.tilegraph
```

This renders the graph to:

- `sheet_color.png`
- `sheet_material.png`
- per-tile output images

If no output directory is provided, it writes a single PNG next to the input file.

## Example

Below is the current `stones.tilegraph` example included with the crate.

```toml
version = 1
name = "Voronoi Stones"
grid = "3x3"
tile_size = "32x32"
palette_source = "local"

[node.colorize4.main]
auto_range = true
color_1 = 7
color_2 = 15
color_3 = 2
color_4 = 19
dither = false
in = "subtract.main:field"
pixel_size = 1
pos = [
    653,
    30,
]

[node.disc.main]
falloff = 1.504
jitter = 0.82
pos = [
    266,
    254,
]
radius = 0.28
radius_in = "id_random.main:field"
scale = 1.0
seed = 21
warp = "noise.main:field"
warp_amount = 0.14

[node.height_shape.main]
bias = 0.12
contrast = 1.55
in = "voronoi.main:height"
plateau = 1.4
pos = [
    434,
    19,
]
rim = 0.0
warp_amount = 0.0

[node.id_random.main]
id = "voronoi.main:cell_id"
pos = [
    61,
    275,
]

[node.multiply.main]
a = "disc.main:height"
b = "scalar.main:field"
pos = [
    624,
    440,
]

[node.noise.main]
pos = [
    4,
    60,
]
scale = 0.18
seed = 7
wrap = true

[node.output.main]
color = "colorize4.main:field"
emissive = 0.0
height = "subtract.main:field"
metallic = 0.0
opacity = 1.0
pos = [
    827,
    196,
]
roughness = 0.9

[node.scalar.main]
pos = [
    421,
    430,
]
value = 0.553

[node.subtract.main]
a = "height_shape.main:field"
b = "multiply.main:field"
pos = [
    597,
    217,
]

[node.voronoi.main]
falloff = 0.36
jitter = 0.593
pos = [
    193,
    7,
]
scale = 0.349
seed = 11
warp = "noise.main:field"
warp_amount = 0.035

[palette]
name = "Local Palette"
colors = [
    "#f2f0e5", "#b8b5b9", "#868188",
    "#646365", "#45444f", "#3a3858",
    "#212123", "#352b42", "#43436a",
    "#4b80ca", "#68c2d3", "#a2dcc7",
    "#ede19e", "#d3a068", "#b45252",
    "#6a536e", "#4b4158", "#80493a",
    "#a77b5b", "#e5ceb4", "#c2d368",
    "#8ab060", "#567b79", "#4e584a",
    "#7b7243", "#b2b47e", "#edc8c4",
    "#cf8acb", "#5f556a",
]
```
