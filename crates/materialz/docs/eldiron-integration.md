# Eldiron integration API

The first integration slice provides GPU-independent editable assets, persistent
tile parameters, single-material WGPU rendering, and live value updates. Enable
`integration` for authoring alone, or `wgpu` for authoring and GPU rendering:

```toml
denrim-materials = { path = "../MaterialZ", features = ["wgpu"] }
```

The runtime uses WGPU 29, compatible with Eldiron's renderer. The default crate
features remain dependency-free; the integration feature adds serialization, and
the WGPU feature adds WGPU and Naga. Shapes, neighbour rules, compositions, dock UI,
and palette storage remain Eldiron responsibilities. Mask composition is a
subsequent integration slice; the runtime currently renders one selected material.

## Editable assets before GPU compilation

```rust
use denrim_materials::integration::{EditableMaterialAsset, InstanceRecord};

let mut asset = EditableMaterialAsset::parse("asset-grass", &recipe, "grass")?;
let document = asset.document();       // Typed format::Project, no GPU required.
let sources = asset.source_catalog();  // Definitions, source code, input capabilities.
let parameters = asset.parameter_metadata()?;
asset.validate()?;                    // CPU validation; no shader device needed.
let bundle = asset.save()?;
let reopened = EditableMaterialAsset::reopen(&bundle)?;
```

The public `root.project` owns the editable AST. Structural edits can modify a
cloned asset, validate it, serialize it, and compile it before replacing the last
working asset. Parsing and parameter inspection do not compile algorithm bodies;
an invalid algorithm remains inspectable and can be saved as an editable draft.
Call `validate()` before committing a working asset. Syntax errors still prevent
construction of an AST.

The versioned JSON bundle retains canonical recipe text, selected material ID,
the built-in source-library fingerprint, embedded dependency documents, and saved
instance records. `dependencies` holds the complete flat closure of
`EditableDocument` values; the host resolves those documents from its asset store
before validation. Referenced materials resolve against the combined document.
Missing dependencies and duplicate identities are errors. Custom per-tile source
bodies remain in their recipes with their existing declaration schemas. Reopen
rejects unsupported bundle/library versions rather than interpreting them against
different source definitions.

Parameter metadata includes stable source and parameter IDs, types, defaults,
ranges, choices, UI groups, current bindings, and authoring locations. The source
catalog also exposes help keys, output fields, and executable tile definitions.
These are sufficient to construct an Eldiron row editor without GPU compilation.
Structured document properties retain labels, row order, connections, modes,
tiling, defaults, and inactive content. Exact comments/whitespace are not retained.

## Persistent identities and instance values

Tiles serialize as `use "Color" id "tile-pigment"`. Legacy recipes receive fresh
IDs on import. Save the migrated asset before retaining bindings; importing the
same unsaved legacy text again creates a new set of identities. Swift's editor
also retains these IDs through its serialization and undo history.

A `ParameterKey` consists of owning asset ID, tile ID, and parameter ID. It does
not contain a row index or tile index. `ParameterInfo.tile_index` describes the
current editor position only. The AST and parameter metadata expose the mapping
from legacy locations to persistent identities. Moving tiles/rows preserves IDs;
when copying a tile to create another tile, assign `format::new_tile_id()` to the
copy. Duplicate IDs are rejected. Deleting a bound tile returns an unresolved-key
diagnostic and never retargets the binding to its former position.
Different dependency documents may use the same local tile ID: persistent keys
include the owning asset, and the compiler scopes their derived identities without
changing the saved recipes.

```rust
use denrim_materials::integration::ParameterKey;

let key = ParameterKey::new("asset-grass", "tile-pigment", "color");
let mut record = InstanceRecord::new("placement-1");
record.set(key.clone(), vec![0.2, 0.4, 0.1, 1.0]);
asset.instances.push(record.clone());
```

Instance records preserve overrides, palette index bindings, and an affine UV
transform separately from the shared recipe. Procedural seed controls are ordinary
persistent parameter overrides. Eldiron resolves palette entries to linear color
values and applies them through these overrides; no palette-reference syntax was
added to recipes. Connected parameters must be edited through their upstream
sources or an explicit structural edit.

## Render a material region

```rust
use denrim_materials::gpu::{MaterialZ, RenderRequest, SurfaceDomain};

let runtime = MaterialZ::new(&device, &queue).await?;
let mut material = runtime.compile(&asset, record).await?;
let request = RenderRequest {
    resolution: [32, 32],
    domain: SurfaceDomain::rect([12.0, 8.0], [1.0, 1.0]),
};
let surface = runtime.create_surface(&request)?;
let ticket = runtime.encode(&mut encoder, &material, &request, &surface)?;
```

The runtime clones handles to the host's device and queue, records work on the
host's encoder, and never submits or polls it. Submit through Eldiron's queue.
The same domain and settings are used for all output channels. `SurfaceDomain`
also accepts explicit U/V axis vectors; the instance UV transform applies to the
origin and axes before sampling.

For continuous terrain, use a Raw material and adjacent map regions with the
same parameters. A neighbour at `[13, 8]` continues the region above. Repeat and
Seamless remain authored periodic modes. For pixel art, render at the logical
tile resolution and let Eldiron enlarge the maps with nearest sampling; further
pixel-grid/mask controls remain part of the composition work.

The backend shares typed algorithms and C-style function emission with the native
backend. Naga lowers the emitted GLSL dialect to validated WGSL. Cached filters
use explicit intermediate passes, affine/fixed domains, halos, premultiplied
intermediates, and mip levels. Unsupported dynamic cache domains retain the core's
diagnostics. Working allocations are bounded by device limits and a 256 MB budget.

## Output layout

`surface.maps` and `gpu::output_layout()` expose the binding order and semantic
layout. Each `SurfaceMap` contains its texture, view, and `OutputLayout`:

| Channel | Texture format | Components | Default | Export range |
| --- | --- | --- | --- | --- |
| Color (`base color`) | `Rgba16Float` | RGB, alpha always 1 | 0.8, 0.8, 0.8 | 0 to 1 |
| Opacity | `R32Float` | R | 1 | 0 to 1 |
| Roughness | `R32Float` | R | 0.5 | 0 to 1 |
| Metallic | `R32Float` | R | 0 | 0 to 1 |
| Emission | `Rgba16Float` | RGB, alpha always 1 | 0, 0, 0 | 0 to 65504 |

Color and emission are linear RGB using sRGB primaries, with straight RGB rather
than opacity-premultiplied RGB. Scalar maps are linear data. Roughness is the
authored perceptual roughness; Eldiron applies its BRDF conversion. Emission is
`max(emission_rgb, 0) * max(emission_strength, 0)`; strength defaults to 1 and must
not be applied a second time. Other required outputs are clamped at export without
changing the editable recipe. No lighting, tone mapping, or palette quantization
is baked into these maps.

Normal and Height are outside the five-map runtime output profile. Authoring
clients can use `compile_preview`, `create_preview_surface`, and `encode_preview`
to render seven maps instead: the five maps above, then Height (`R32Float`) and
Normal Map (`Rgba16Float`). `preview_output_layout()` documents that order. Normal
is encoded tangent-space RGB and remains linear data when displayed; it must not
receive an sRGB transfer function. The extra maps do not change runtime baking or
`encode_into`. Call `material.diagnostics()` for authored channels omitted by the
compiled profile; their recipes remain intact.
`encode_into` accepts five caller-owned full 2D texture views in the documented
order, with matching formats/resolution and `STORAGE_BINDING` usage. Views must
belong to the host device and expose the expected full mip and layer.

## Live updates and diagnostics

```rust
let generation = material.pipeline_generation();
material.set_parameter(key, vec![0.3, 0.5, 0.2, 1.0])?;
assert_eq!(generation, material.pipeline_generation());
let saved_instance = material.snapshot();
```

Updates validate an instance candidate and refresh parameter buffers while keeping
compatible GPU pipelines. `update(record)` replaces a full instance record;
`update_asset(&edited_asset)` supports compatible recipe value/default edits.
Structural changes that alter generated source/layout return a diagnostic and
leave the previous asset, record, and pipelines untouched; compile a new instance
for such changes. Validation may regenerate source on the CPU; it does not compile
a GPU pipeline for a compatible update.

Every encode uses immutable parameter/domain snapshots. Updating values between
two encodes, even before their shared encoder is submitted, preserves the values
captured for each request. Save `material.snapshot()` in the asset's instance list
to retain live overrides on reopen.

Diagnostics expose severity, operation, asset, and available material/row/tile/
parameter identity. Tile source validation and parameter edits identify their
authoring location; graph-wide errors identify the selected material and operation.
Nonfinite or overflowing GPU outputs set per-channel diagnostic flags on the
returned `RenderTicket`. After submitting the encoder, call
`ticket.map_diagnostics(callback)` and let Eldiron poll the device or service its
normal event loop. The callback reports errors without requiring readback of the
material maps. Hosts may also read the ticket's small `readback` buffer directly.

## Example and verification

Run `cargo run --features wgpu --example eldiron_wgpu` for a headless render,
editable bundle round trip, and live color update using the same pipelines.

`cargo test --features wgpu` covers persistent bindings, editable dependencies,
metadata without shader compilation, and WGSL lowering for all built-in sources.
Run `cargo test --features wgpu --test wgpu_runtime -- --ignored --nocapture` on a
machine with a GPU for map values, CPU comparisons, continuous tile boundaries,
cached filters, emission strength, opacity, live updates, and queued snapshots.
