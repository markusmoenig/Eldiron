# buildergraph

`buildergraph` is Eldiron's reusable prop and structural-assembly crate.

It contains:

- the text-based `.buildergraph` document format
- parsing and evaluation for builder scripts and graph documents
- a preview renderer for quick in-editor feedback
- built-in presets such as tables, wall torches, wall lanterns, and campfires

![wall torch preview](examples/wall_torch.png)

## What It Is

`buildergraph` is used for authoring reusable 3D assemblies that can be attached to different host types inside Eldiron.

Typical uses include:

- furniture such as tables
- wall-mounted props such as torches and lanterns
- floor props such as campfires
- edge-based structures such as rails or fences

Builder assets can expose:

- geometry
- named material slots
- named item slots
- a host target such as sector, linedef, or vertex

That makes one builder reusable across many placements and material setups.

## Script Example

Builder scripts are intentionally human-readable:

```txt
name = "Wall Torch";
host = vertex;

preview {
    width = 1.0;
    depth = 0.4;
    height = 2.0;
}

let plate = box {
    attach = host.middle + host.out * 0.03;
    size = vec3(0.18, 0.28, 0.05);
    material = BASE;
};

slot material base_mat = plate.center;
output = [plate];
```

## Library Use

The crate can parse either script-based or node-graph-based builder documents:

```rust
use buildergraph::BuilderDocument;

let source = std::fs::read_to_string("examples/table.buildergraph")?;
let document = BuilderDocument::from_text(&source)?;
let preview = document.render_preview(256);

assert!(preview.width > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Scope

`buildergraph` is primarily designed for Eldiron's Builder Tool workflow. The format is still evolving, but the goal is clear: readable, reusable structural assets that can be previewed and instanced quickly.
