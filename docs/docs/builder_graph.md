---
title: "Builder Graph"
sidebar_position: 6.1
---

**Builder Graph** is Eldiron's text-based procedural authoring system for reusable structures, props, surface details, and placed decorative geometry.

Builder graphs are stored as **`.buildergraph`** scripts. They can be edited in Eldiron with a live 3D preview and then applied to map geometry with the [Builder Tool](/docs/creator/tools/builder).

Builder Graph is used for assets such as:

- tables, torches, lanterns, campfires, fences, and railings
- wall-attached details such as columns, pilasters, trims, and arches
- sector surface details such as raised borders, recesses, and freestanding columns
- future procedural vegetation and billboard-style assets

## Core Idea

A Builder Graph describes reusable geometry relative to a **host**.

The host provides dimensions and orientation. The script stays generic, and the placed instance decides where the result appears in the map.

```txt
name = "Wall Columns";
host = linedef;

param radius = 0.14;
param cap = 0.22;

detail column {
    center = vec2(host.width * 0.50, 0.0);
    height = host.height;
    radius = radius;
    offset = -0.08;
    base = 0.16;
    cap = cap;
    transition = 0.08;
    material = COLUMN;
    tile_alias = stone;
};

output = [];
```

## Host Targets

Every script declares one host target:

- `host = sector;`
  - floor, platform, terrain, and sector-surface details
  - supports relief details and freestanding details

- `host = linedef;`
  - wall spans, rails, fences, wall columns, pilasters, and long edge-based structures
  - uses wall-side and outward direction from the placed linedef

- `host = vertex;`
  - point-mounted props such as wall torches, wall lanterns, campfires, posts, and markers

The host target controls what the [Builder Tool](/docs/creator/tools/builder) can apply the graph to.

## Preview Block

The optional `preview` block defines the host dimensions used by the editor preview and preview CLI.

```txt
preview {
    width = 4.0;
    depth = 2.5;
    height = 2.0;
}
```

Use preview dimensions that make the asset easy to inspect. They do not hardcode the final map size.

## Parameters

Scripts can expose tunable parameters with `param` declarations. Parameters can be numeric or symbolic.

```txt
param radius = 0.14;
param spacing = 2.0;
param placement = attached;
param broken_chance = 0.0;
param seed = 1.0;
```

Parameters can be used anywhere a scalar expression is expected:

```txt
detail columns {
    start = host.width * 0.08;
    end = host.width * 0.92;
    y = 0.0;
    spacing = spacing;
    height = host.depth;
    radius = radius;
    placement = placement;
    broken_chance = broken_chance;
    seed = seed;
};
```

When a Builder Graph is selected in the Builder dock, Eldiron shows exposed parameters in a TOML sidebar next to the lower picker. Editing the sidebar updates the corresponding `param ... = ...;` lines in the script. The script remains the source of truth.

Use parameters for values that tune a template:

- radius, spacing, height, offset
- placement modes such as `attached` or `structural`
- cap/base/transition size
- damage amount or chance
- deterministic seed values
- material or item behavior later when supported by the script

Do not try to make one script cover every structural choice. If the question is "cut the wall or keep masonry behind it?", "attached or structural?", or "arches or columns?", that should usually be a different template rather than another parameter.

## Primitives

Builder scripts can emit object-style primitives such as `box`, `cylinder`, and procedural `planks`.

These are useful for props and assemblies:

```txt
let top = box {
    attach = host.middle + host.up * 0.75;
    size = vec3(1.2, 0.12, 0.8);
    material = TOP;
};

output = [top];
```

Primitives can expose anchors and material slots so other builder assets can attach to them.

Use `planks` for seeded rows of boards, shingles, or broken wooden siding. It expands into multiple box primitives, so the result is stable geometry rather than a special renderer path.

```txt
let wall_planks = planks {
    attach = host.middle + host.out * (host.depth * -0.5 - 0.095);
    size = vec3(host.width * 0.96, wall_height * 0.92, 0.035);
    count = plank_count;
    direction = vertical;
    jitter = 0.55;
    unevenness = 0.40;
    missing_chance = damage;
    seed = seed;
    material = PLANK;
};
```

## Surface Details

Surface details decorate sector or linedef surfaces without needing a custom editor action for every case.

### Rect Details

Rect details create raised or inset rectangular regions.

```txt
detail rect {
    min = vec2(host.width * 0.10, host.depth * 0.10);
    max = vec2(host.width * 0.90, host.depth * 0.90);
    offset = -0.06;
    shape = border;
    inset = 0.12;
    material = TRIM;
    tile_alias = stone;
};
```

### Plank Details

Plank details create seeded rows of rectangular relief details on a sector or linedef surface. Use them when the desired result should remain part of the surface, not a freestanding object assembly.

```txt
detail planks {
    min = vec2(host.width * 0.05, host.depth * 0.10);
    max = vec2(host.width * 0.95, host.depth * 0.90);
    count = 12.0;
    direction = horizontal;
    jitter = 0.35;
    unevenness = 0.30;
    missing_chance = 0.05;
    seed = seed;
    offset = 0.035;
    material = PLANK;
    tile_alias = wood;
};
```

`jitter` changes plank lengths and widths. `unevenness` / `alignment_jitter` shifts each plank within its lane, which makes boards look less mechanically aligned. `missing_chance` is a `0.0..1.0` probability per plank.

### Column Details

Column details support four placement modes:

- `placement = relief;`
  - surface decoration
  - the column is treated like raised or recessed surface geometry
  - useful for shallow columns, trims, pilasters, and decorative surface work

- `placement = attached;`
  - real protruding geometry mounted to the host surface
  - useful for round wall columns, half-columns, and architectural columns in front of masonry

- `placement = structural;`
  - real geometry centered in the host surface thickness
  - useful for replacing a cut-out wall span with columns under the same ceiling line

- `placement = freestanding;`
  - real upright 3D geometry anchored on the surface
  - useful for floor columns, posts, statues, pillars, and architectural props
  - can optionally cut a prepared footprint out of the sector surface

Relief is the default for compatibility.

`base` and `cap` add square blocks. `transition` adds a short connector between the square blocks and the round shaft, which is useful for columns that should not look like a cylinder simply stacked on a cube.

`material` is the fallback material for the full column. Use `rect_material` for square base/cap sections and `cyl_material` for round shaft/transition sections when those parts need separate material slots.

```txt
detail column {
    placement = freestanding;
    center = vec2(host.width * 0.30, host.depth * 0.50);
    height = 1.45;
    radius = 0.12;
    cut_footprint = true;
    base = 0.14;
    cap = 0.14;
    transition = 0.06;
    material = COLUMN;
    rect_material = BLOCK;
    cyl_material = SHAFT;
    tile_alias = stone;
};
```

Use `detail columns` for an aligned row. The first and last columns are placed exactly at `start` and `end`; the requested `spacing` is rounded to a whole number of equal gaps.

```txt
detail columns {
    start = host.width * 0.16;
    end = host.width * 0.84;
    y = 0.0;
    spacing = 2.0;
    height = host.depth;
    radius = 0.10;
    placement = attached;
    material = COLUMN;
    rect_material = BLOCK;
    cyl_material = SHAFT;
    tile_alias = stone;
};
```

Column rows can also use deterministic damage controls:

```txt
detail columns {
    start = host.width * 0.08;
    end = host.width * 0.92;
    y = 0.0;
    spacing = spacing;
    height = host.depth;
    radius = radius;
    broken_chance = broken_chance;
    broken_min_height = broken_min_height;
    seed = seed;
    placement = structural;
    material = COLUMN;
    rect_material = BLOCK;
    cyl_material = SHAFT;
    tile_alias = stone;
};
```

`broken_chance` is a `0.0..1.0` probability per column. `broken_min_height` controls the shortest allowed broken column as a fraction of the original height. `seed` makes the result stable across rebuilds.

See `crates/buildergraph/examples/wall_column_series.buildergraph` for a standalone attached series example without masonry. See `crates/buildergraph/examples/wall_column_opening.buildergraph` for a structural series that cuts out the wall span first.

## Cuts

Sector-hosted graphs can cut or replace parts of the sector surface.

```txt
cut rect {
    min = vec2(host.width * 0.25, host.depth * 0.25);
    max = vec2(host.width * 0.75, host.depth * 0.75);
    mode = cut;
};
```

Cut modes:

- `cut`
  - removes an area from the host surface

- `replace`
  - removes the host sector surface and emits replacement geometry

- `cut_overlay`
  - reserved for overlay-style cut workflows

Cuts and details can be combined. For example, a script can cut a sector opening and place freestanding columns around it.

Freestanding sector columns can also request an automatic footprint cut:

```txt
detail column {
    placement = freestanding;
    cut_footprint = true;
    center = vec2(host.width * 0.50, host.depth * 0.50);
    height = 1.45;
    radius = 0.12;
    base = 0.14;
    cap = 0.14;
};
```

When `cut_footprint = true`, Eldiron rebuilds the host surface with a small hole under the column footprint and emits the upright column as separate 3D geometry.

### Masonry Details

Masonry details split a surface area into real raised stone or brick relief blocks. The base surface remains visible in the mortar gaps, so a tile can provide the color and the builder graph provides the geometry.

```txt
detail masonry {
    min = vec2(host.width * 0.08, host.depth * 0.12);
    max = vec2(host.width * 0.92, host.depth * 0.88);
    block = vec2(0.52, 0.28);
    mortar = 0.035;
    offset = -0.04;
    pattern = running_bond;
    material = STONE;
    tile_alias = stone;
};
```

Supported patterns:

- `pattern = grid;`
- `pattern = running_bond;`

Masonry blocks are inset slightly from the requested bounds. That avoids adding relief geometry exactly on sector boundaries, but it does not replace the need for a topology-level fix for duplicate coplanar surfaces on shared edges.

The Builder dock's **New** menu creates an empty project graph. Starter templates such as `Table`, `Wall Torch`, `Wall Lantern`, `Campfire`, `Surface Masonry`, `Wall Linedef Masonry`, and `Wall Columns Masonry` live in Treasury so they have metadata, aliases, previews, and versioning.

## Templates And Treasury

Builder Graph is designed around focused procedural templates, not one giant universal graph.

A template should encode the structural intent:

- wall masonry relief
- wall columns with masonry
- wall column opening
- broken wall column opening
- wall arch opening
- floor cracked tiles
- floor border relief

Parameters then tune that intent. For example, `Wall Column Opening` can expose `radius`, `spacing`, `broken_chance`, and `seed`, while `Wall Masonry Relief` exposes `block`, `mortar`, and `offset`.

This keeps non-scripting workflows manageable:

1. Pick the right template from the Treasury or Builder dock.
2. Tune exposed parameters in the TOML sidebar.
3. Assign material slots through the HUD.
4. Apply the graph to matching map hosts.

The Treasury should contain several clear templates rather than one script with dozens of unrelated controls.

Published Treasury templates are indexed in `Eldiron-Treasury/index.json` under `builder_graphs`. Each entry points at a package folder containing `package.toml` metadata and a `graph.buildergraph` script. The Builder dock reads the index when the Treasury tab is opened, so metadata such as aliases, tags, target type, and description can be used for filtering without crawling the repository.

Treasury templates can be applied directly. Use **Install** when you want to copy a Treasury template into the current project and edit it as a normal project Builder Graph.

## Materials

Builder graphs should expose named material slots instead of hardcoding tiles.

```txt
material = COLUMN;
```

When a graph is applied in Eldiron, the placed instance can assign tiles to those slots through the Builder HUD.

Scripts can also provide a tile alias:

```txt
tile_alias = stone;
```

If no explicit material tile is assigned, Eldiron can look for project tiles with a matching alias. If multiple tiles match, it can pick one deterministically from the available matches.

## Item Slots

Builder graphs can expose item slots for child builders.

Use item slots when a builder asset should receive another asset, such as:

- a tabletop prop
- a shelf item
- a torch flame attachment
- a decorative object mounted on a stand

Point attachments use item anchors. Surface attachments use item surfaces.

## Example: Freestanding Sector Columns

```txt
name = "Surface Freestanding Columns";
host = sector;

preview {
    width = 4.0;
    depth = 2.5;
    height = 2.0;
}

detail column {
    placement = freestanding;
    center = vec2(host.width * 0.30, host.depth * 0.50);
    height = 1.45;
    radius = 0.12;
    cut_footprint = true;
    base = 0.14;
    cap = 0.14;
    material = COLUMN;
    tile_alias = stone;
};

detail column {
    placement = freestanding;
    center = vec2(host.width * 0.70, host.depth * 0.50);
    height = 1.45;
    radius = 0.12;
    cut_footprint = true;
    base = 0.14;
    cap = 0.14;
    material = COLUMN;
    tile_alias = stone;
};

output = [];
```

Apply this graph to a sector with the Builder Tool. It keeps the sector surface and adds two upright columns.

## Example: Wall Columns

```txt
name = "Wall Columns";
host = linedef;

preview {
    width = 4.0;
    depth = 0.3;
    height = 2.4;
    surface = wall;
}

detail column {
    center = vec2(host.width * 0.20, 0.0);
    height = host.height;
    radius = 0.10;
    offset = -0.08;
    base = 0.16;
    cap = 0.16;
    material = COLUMN;
    rect_material = BLOCK;
    cyl_material = SHAFT;
    tile_alias = stone;
};

output = [];
```

Apply this graph to a linedef. The detail is emitted on the selected wall side.

## CLI Preview

Builder Graph also has a CLI for quick checks and preview generation.

```sh
cargo run -p buildergraph -- surface crates/buildergraph/examples/surface_freestanding_columns.buildergraph
cargo run -p rusterix --bin builderpreview -- crates/buildergraph/examples/surface_freestanding_columns.buildergraph --size 256
```

The `builderpreview` command writes a PNG next to the script using the same file name with a `.png` extension.

## Current Limits

Builder Graph is still evolving. Current limits include:

- no Tree-sitter grammar yet
- no visual node editor for Builder Graph scripts
- collision for generated details is not complete
- arches and richer architectural profiles are still planned
- exact selected-host texture inheritance is separate from alias/material resolution

## Related Pages

- [Builder Tool](/docs/creator/tools/builder)
- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Working With Geometry](/docs/working_with_geometry)
- [Working With Tiles](/docs/building_maps/working_with_tiles)
