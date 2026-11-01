# Eldiron integration concept

MaterialZ should expose a Rust API that evaluates and composes PBR surfaces over
an explicit coordinate region. Eldiron supplies material instances, shape masks,
pixelization settings, and palette colors. MaterialZ produces GPU material maps
that Eldiron uses for 2D tiles and surfaces such as walls and floors in 3D.

The same interface serves reusable tiles, continuous terrain, and layered shape
transitions. A single tile is a composition containing only a base material. A
road adds a masked material layer over that base.

This is the integration design. The Rust interfaces below illustrate the broader
target contract rather than the exact currently exported API.

The first usable slice now has a separate [integration API guide](eldiron-integration.md)
covering editable assets, stable identities, single-material WGPU rendering, and
live parameter updates. The examples in this concept describe the broader target
API, including composition features beyond that slice.

## Responsibilities

| MaterialZ | Eldiron |
| --- | --- |
| Parse, validate, and compile editable material recipes | Store maps, editable shapes, compositions, neighbour rules, surface mappings, and palettes |
| Evaluate procedural materials through a WGSL backend | Supply the WGPU device, queue, and command encoder |
| Blend complete PBR surfaces using supplied masks | Rasterize roads, curves, and other shapes into masks |
| Plan intermediate image passes and cache evaluations | Choose tile domains, resolution, and pixel style |
| Expose document structure, source metadata, texture layouts, and diagnostics | Own the row editor and dock UI, render geometry, apply lighting, and present the result |

Keep the shared material compiler independent of WGPU. Add the GPU runtime as an
optional feature or companion crate. Eldiron should use its existing GPU context
and be able to render into caller-provided textures, texture-array layers, or
atlas regions. A convenience allocator can create standalone output surfaces.

## Material assets and compositions

A **material asset** retains the editable recipe, selected stable material ID,
required dependencies, and saved instance data. Compiled handles and generated
maps are derived resources. Compile the asset when loading or structurally
editing it; saving the asset must preserve its editable representation.

A **material instance** references an asset and supplies parameter overrides,
coordinate transforms, and any explicit seed. Different instances can share the
compiled asset while using different settings.

A **composition** contains a base instance and ordered layers. Each layer combines
the accumulated surface with one incoming material using a mask and blend mode.
This implements two-material operations while allowing several materials to
contribute at a point through successive layers.

For example, start with grass, add asphalt inside a curved road, add gravel along
its shoulder, and add rocks inside another region. Layer order is significant.
Only layers whose coverage intersects the requested region, including required
filter padding, need evaluation.

Map compositions belong to Eldiron. Placing a road must not require creating a
new MaterialZ document or changing an authored material graph. Eldiron also stores
editable shapes, neighbour rules, and surface mappings. Rasterized masks are
derived resources and do not replace the editable shapes. Automatic neighbour
resolution and the dock UI remain Eldiron responsibilities.

## Persistent parameter identities

Give every tile an opaque, document-unique `tile_id` and preserve it in the
structured document and serialized recipe. Use a persistent parameter key made
from the asset identity, tile identity, and source parameter ID. Material and row
IDs remain available for editor lookup and diagnostics, but row membership and
the tile's position in a row must not determine parameter identity.

Insertion, reordering, and moving a tile between rows preserve its identity.
Save/reopen and canonical formatting preserve identities as well. Duplicating a
tile or importing another copy into the same document creates a new identity;
the import result exposes any ID remapping. Deleting a tile leaves its bindings
unresolved with a diagnostic, rather than retargeting them to the next tile.
Replacing a tile's source must validate parameter IDs and types before retaining
bindings. Labels and array indices never serve as persistent keys.

Palette bindings and per-placement overrides use the same persistent parameter
keys, with instance identity selecting which placement receives an override.
Parameter metadata exposes these keys independently of GPU buffer offsets,
which may change after compilation.

This requires extending serialized tile records with stable identities. Legacy
recipes receive IDs once on import; the import result maps legacy material, row,
and tile-index locations to the new keys. Eldiron persists that migrated recipe
before retaining new bindings, so subsequent loads do not assign different IDs.
The existing index-based live-edit calls can remain implementation adapters that
resolve a persistent key against the current document on each edit.

## Editable asset round trips

An Eldiron asset stores a versioned editable bundle containing:

- The structured root recipe and its serializable source, including stable IDs,
  row/tile ordering, connections, defaults, custom source code, and authored
  settings. Inactive and unconnected content remains editable.
- The selected material ID within that recipe.
- A dependency manifest and the referenced material documents, custom tile
  definitions, and any other resources required by the recipe. Entries retain
  stable IDs and version or content-hash information. Built-in definitions refer
  to an identified source-library version; unavailable versions are diagnosed.
- Saved material-instance overrides, palette bindings, coordinate transforms,
  and explicit seeds. Overrides remain distinct from the shared recipe defaults.

Dependencies may be embedded or resolved from Eldiron's asset store. The loader
must enumerate and validate the complete dependency closure before compilation;
missing or incompatible dependencies cannot silently become default materials.
Compiled programs, GPU handles, and cached maps can be regenerated from the bundle.

MaterialZ should expose an owned structured document plus a source catalog and
parameter metadata. The document describes materials, rows, stable tile IDs,
connections, and source references. Metadata supplies stable source/parameter IDs,
types, defaults, ranges, grouping, help keys, and connection capabilities. Instance
inspection exposes effective values separately from recipe defaults and overrides.
Together these let Eldiron reconstruct its row editor without inspecting generated
shader code or GPU buffer layouts.

In the proposed Rust interface, `EditableMaterialAsset::load(bundle, resolver)`
resolves dependencies; `document()` returns the editable structure;
`source_catalog()` and `parameter_metadata()` expose editor definitions and
persistent parameter keys. An instance `snapshot()` returns a serializable record
of its identity, asset reference, overrides, palette bindings, mapping, and seed.
The asset bundle includes those saved records, not runtime handles. These accessors
must also work before GPU compilation so opening an editor does not require
successful shader creation.

The initial editing flow can parse/load the bundle, obtain the document and
metadata, edit a candidate document in Eldiron, serialize it, validate and recompile
it, then commit the validated bundle and derived program together. Validation
covers identity uniqueness, selected-material existence, dependencies, parameter
types, connections, cycles, and execution limits, with diagnostics tied to stable
editor identities. A failed edit preserves the last valid asset and program.
Structural edits may recompile; a complete graph-mutation API is not required.
An operation such as `with_document(candidate)` retains the selected material,
dependencies, and saved instance records, then `validate()` and
`serialize_bundle()` provide the validation and persistence boundary. If an edit
deletes an overridden parameter, preserve the unresolved override and report it
for explicit repair rather than silently losing it on save.

Serialization and reopen must preserve semantic editability and instance data;
exact whitespace and comments are not required. A self-contained export includes
the required dependency content and source-library information so another editor
can resolve the same definitions. Eldiron's shapes, composition order, neighbour
rules, and map mappings remain in Eldiron's own project data.

## Coordinate domains and continuous terrain

Every render request declares its resolution and a mapping from output pixels to
surface coordinates. An origin and two axis vectors can represent rectangular,
rotated, or scaled regions; a rectangle constructor covers ordinary map tiles.
Each material instance applies its own UV transform to those coordinates.

For continuous grass, neighboring tiles sample neighboring regions of the same
procedural field with the same parameters and seed. For example, one tile samples
the region beginning at `[12, 8]`, and the next begins at `[13, 8]`. Do not reset
coordinates or randomize the seed per tile when continuity is required.

Continuous materials use MaterialZ's Raw boundary mode, shown in the editor as
Continuous UVs. Sampling at map coordinates does not override an authored Repeat
or Seamless mode; those modes intentionally remain periodic. A reusable tile can
render the same domain repeatedly with the desired authored boundary mode.

Shape coordinates and material coordinates are separate. A road mask can follow a
map-space curve while asphalt retains a consistent scale and orientation. Curving
coverage does not automatically bend asphalt UVs along the path. Path-aligned
markings or texture flow would need an additional coordinate mapping capability.

Walls and floors supply their own surface coordinate frames. Material evaluation
does not need to know whether the resulting maps will appear in 2D or 3D.

## Shape masks

The initial mask interface accepts a constant, CPU pixels, or an existing GPU
texture:

```rust
Mask::constant(0.5)
Mask::pixels(&values, width, height)
Mask::texture(&texture_view)
```

A coverage mask contains one finite grayscale float per pixel in `0.0..=1.0`:

| Value | Meaning |
| --- | --- |
| `0.0` | Preserve the accumulated surface |
| `1.0` | Select the incoming material |
| `0.5` | Equal blend in Coverage mode |

CPU pixel buffers should have an explicit row order and stride; the convenience
form above can assume tightly packed rows. Invalid dimensions, buffer lengths,
nonfinite values, and values outside the coverage range should be diagnosed.
CPU uploads must be copied or staged before the borrowed input can be released.

The GPU path avoids a CPU round trip. A single-channel `R32Float` texture is a
baseline representation, with sampling that does not require optional float
filtering support. Additional texture formats can be exposed through a supported
format list.

By default, the mask covers the same region as the render request, with texel
centers mapped consistently to that region. A mask descriptor should also support
an explicit domain transform for shared map masks. Mask resolution may differ
from output resolution, but nearest or linear reconstruction must be explicit.

Eldiron owns shape geometry and rasterization. A later distance-mask input can
support adjustable feather widths, shoulders, and edge breakup. It must declare
its sign convention and distance units; grayscale coverage is not a distance field.

## Minimal Eldiron output profile

The initial `PbrProfile::Eldiron` requires exactly five outputs: Color, Opacity,
Roughness, Metallic, and Emission. Produce unlit material properties and let Eldiron
apply its lighting. Color maps to the existing `base color` channel. Normal and
Height are optional extensions, not prerequisites for initial integration.

The baseline GPU layout uses separate textures with these contracts:

| Output | Format and components | Default | Range and interpretation |
| --- | --- | --- | --- |
| Color | `Rgba16Float`, RGB; A is `1` | `(0.8, 0.8, 0.8)` | Linear RGB in `0..1`, using sRGB primaries; unlit reflectance |
| Opacity | `R32Float`, R | `1` | Scalar in `0..1`; zero removes the surface, one covers it |
| Roughness | `R32Float`, R | `0.5` | Perceptual roughness in `0..1`; Eldiron performs any BRDF-specific conversion |
| Metallic | `R32Float`, R | `0` | Metallic weight in `0..1` |
| Emission | `Rgba16Float`, RGB; A is `1` | `(0, 0, 0)` | Nonnegative linear HDR RGB, with strength already applied |

Color and Emission use straight RGB, not RGB premultiplied by Opacity. Their alpha
components are padding, not surface opacity or shape coverage. There is no sRGB
transfer encoding in the baseline float textures. Eldiron converts palette inputs
from their declared color space and performs any final display encoding. An
explicit packed RGBA export may put Opacity in Color's alpha, but must declare
whether its RGB remains straight or is premultiplied.

Profile export clamps Color, Opacity, Roughness, and Metallic to their declared
ranges, without rewriting authored values in the editable document. Emission color
and emission strength are nonnegative at this boundary: resolve their product as
`max(emission_rgb, 0) * max(emission_strength, 0)` for each input material before
composition. The existing strength default is `1`; the output has no separate
strength channel, and Eldiron must not multiply it by strength again. It represents
scene-linear emissive intensity, with exposure and lighting units owned by Eldiron.
Nonfinite results or HDR values exceeding the selected format's finite range
produce a diagnostic; a caller can explicitly request a compatible float32 layout.

Expose a versioned layout descriptor for every output: semantic/channel ID,
texture format, dimensions, texture/binding identity, array layer or atlas region,
component mapping, defaults, ranges, color space, and alpha convention. Eldiron
can bind the baseline textures or request an explicit supported packing, such as
Opacity in Color alpha or scalar properties in selected channels. Unsupported
packings are diagnosed. The execution plan may split output writes into multiple
passes to fit the device's storage-texture limits.

Unsupported authored channels remain in the editable asset and produce a profile
diagnostic instead of being silently discarded. An optional Normal extension
declares tangent-frame and Y conventions; a baseline choice is encoded tangent
RGB in `Rgba16Float`, default `(0.5, 0.5, 1)`, with +Y along the supplied bitangent.
An optional Height extension uses `R32Float`, default `0`, and finite signed values
with an explicit conversion to Eldiron's surface units.

## PBR surface composition

Start with **Coverage** blending: use one scalar shape coverage consistently to
interpolate the five surface properties, including Opacity and resolved Emission.
For accumulated surface A, incoming surface B, and mask `w`, each required property
is `mix(A, B, w)`. Zero preserves A and one selects B, including B's own opacity.
This is material interpolation; do not implicitly replace it with alpha-over.

Shape coverage selects material contribution; material Opacity controls the
resulting surface's coverage. Do not overwrite Opacity with the shape mask or
premultiply the material maps by it. At mask `1`, a material with Opacity `0.25`
still has Opacity `0.25`. A transparent incoming material can therefore make the
composed surface transparent; exposing an underlying material through a decal
would require an explicit alpha-compositing operation with separate semantics.

Curved road coverage and top/bottom decorative strip coverage are ordinary ordered
layers. Eldiron supplies the editable shapes and strip placement, including any
neighbour decisions, then rasterizes their masks. MaterialZ applies the resulting
masks and order without generating a new material-specific shader when a shape
moves or layers are reordered.

With the optional Normal extension, decode normal maps before blending and
normalize the result in a common tangent frame. The optional **HeightAware** mode
uses coverage to establish the transition region, then adjusts contribution using
relative material heights and explicit influence settings. It requires Height
support and must preserve the mask endpoints. Height can affect protrusion within
the transition without extending the road everywhere.

When optional height supplies bump relief, derive that relief from the final
composed height, including neighboring samples. This captures slopes introduced
by the transition itself. Retain authored normal detail and combine it with the
derived relief using a documented convention. Height output does not itself
displace Eldiron geometry; Eldiron chooses how to consume it.

## Dynamic pixelization

Pixelization is a render setting, separate from the material recipe. Specify a
logical pixel density per map tile and a shared grid origin:

```rust
PixelStyle {
    pixels_per_tile: [32, 32],
    grid_origin: [0.0, 0.0],
    mask_style: MaskStyle::Coverage,
}
```

A tile displayed at 128 by 128 pixels can contain 32 by 32 logical material
pixels, enlarged through nearest sampling. The runtime may store the maps at
logical resolution and let Eldiron enlarge them; allocating the enlarged maps is
not required.

Sample materials and masks on the same logical grid, anchored in surface
coordinates so neighboring tiles align. Continuous terrain should use consistent
pixel density across adjoining requests. Use the logical pixel footprint for
procedural filtering and height derivatives. Apply the style consistently to all
PBR maps, then let Eldiron choose its screen-space presentation policy.

Mask style is independent of pixel density. Coverage retains grayscale blending;
Threshold makes a hard material choice; Dither distributes material choices on
the logical grid. Dither patterns should use shared surface coordinates so they
remain stable across tile boundaries. These operations modify composition
coverage and do not replace material opacity.

## Palette colors and bindings

Initially, Eldiron owns the palette and stores bindings from palette indices to
editable color parameters on material instances. A palette update resolves those
indices to linear color values and updates the instance parameter buffers.

Use the persistent parameter keys defined above for palette bindings and
per-placement overrides. Bindings target color constants; a connected parameter
needs an editable upstream color source or an explicit structural edit.

This approach needs no palette-index extension to the MaterialZ file format;
the stable tile identities still require serialization support.
Eldiron can persist recipes with resolved colors when exporting a self-contained
asset, while retaining palette bindings in its own project data. Regenerating
recipe text when colors change is possible, but routine updates should reuse
compiled shaders where only values have changed.

Palette references in the MaterialZ format become useful if the MaterialZ editor
must author and preserve them directly. That would require a palette declaration,
typed color references, and defined handling of missing or invalid entries. It is
separate from the initial embedding API.

Palette inputs do not guarantee palette-only output. Procedural operations,
material blending, and PBR lighting produce intermediate colors. If Eldiron
requires every displayed pixel to use a palette entry, it should map the final
lit image to the palette, optionally using dithering. Quantizing base color alone
does not enforce a palette after lighting. Normal, height, roughness, and other
data maps must not be color-quantized.

## Proposed Rust usage

The examples share the following setup. All API names are illustrative, including
the Eldiron shape helper. Compilation and surface allocation can be reused across
render requests.

```rust
let mut materialz = MaterialZ::new(&device, &queue, PbrProfile::Eldiron)?;

// Retain this editable asset and its dependencies when saving Eldiron's asset.
let grass_editable = EditableMaterialAsset::parse(&grass_source, "grass")?;
let grass_asset = materialz.compile(&grass_editable)?;
let grass = grass_asset.instance()
    .with_uv_transform(UvTransform::scale([4.0, 4.0]));

let mut request = RenderRequest {
    resolution: [128, 128], // Output resolution before optional pixelization.
    domain: SurfaceDomain::rect([0.0, 0.0], [1.0, 1.0]),
    pixel_style: Some(PixelStyle {
        pixels_per_tile: [32, 32],
        grid_origin: [0.0, 0.0],
        mask_style: MaskStyle::Coverage,
    }),
};
```

### Render a single tile

```rust
let surface = materialz.create_surface(&request)?;

materialz.encode(
    &mut encoder,
    &request,
    &Composition::base(&grass),
    &surface,
)?;

// GPU maps for Eldiron's material bindings; no CPU readback required.
let base_color = surface.base_color();
let opacity = surface.opacity();
let roughness = surface.roughness();
let metallic = surface.metallic();
let emission = surface.emission();

// Describes formats, component mappings, and bindings, including explicit packing.
let layout = surface.layout();
```

### Mix grass and a curved road

```rust
let road_editable = EditableMaterialAsset::parse(&road_source, "asphalt")?;
let road_asset = materialz.compile(&road_editable)?;
let road = road_asset.instance()
    .with_uv_transform(UvTransform::scale([2.0, 2.0]));

request.domain = SurfaceDomain::rect([12.0, 8.0], [1.0, 1.0]);

// Eldiron rasterizes this path in map coordinates into a coverage texture.
// The helper accounts for the request's domain and logical pixel grid.
let road_mask = eldiron_shapes.encode_coverage(
    &mut encoder,
    &request,
    &Shape::quadratic_stroke(
        [11.8, 8.2], // Start.
        [12.5, 9.1], // Control point.
        [13.2, 8.7], // End.
        0.35,       // Full road width in map units.
    ),
    0.04, // Feather width in map units.
)?;

let composition = Composition::base(&grass)
    .layer(
        Layer::new(&road)
            .mask(Mask::texture(&road_mask))
            .blend(BlendMode::Coverage),
    );

materialz.encode(&mut encoder, &request, &composition, &surface)?;
```

The same composition can append gravel or rocks with additional masks. Changing
the neighboring request's origin to `[13.0, 8.0]` continues the material field and
shape. Eldiron submits the command buffer and retains referenced resources until
GPU work completes.

Top and bottom decorative strips use the same layer operation. Once Eldiron has
resolved their placement and generated masks, it can append them in the desired
order:

```rust
let composition = composition
    .layer(Layer::new(&top_strip).mask(Mask::texture(&top_mask))
        .blend(BlendMode::Coverage))
    .layer(Layer::new(&bottom_strip).mask(Mask::texture(&bottom_mask))
        .blend(BlendMode::Coverage));
```

The strip instances and masks are supplied by Eldiron. MaterialZ has no automatic
neighbour resolution or dock-specific behavior.

### Update palette colors

```rust
// Document-unique tile IDs and parameter IDs come from editable asset metadata.
// The instance selects the placement; the asset scopes these parameter keys.
let bindings = [
    ColorBinding::new(
        grass.id(),
        ParameterKey::new("tile-dark-color", "color"),
        PaletteIndex(4),
    ),
    ColorBinding::new(
        grass.id(),
        ParameterKey::new("tile-light-color", "color"),
        PaletteIndex(7),
    ),
];

materialz.update_palette(&palette, &bindings)?;
```

The update validates bindings, converts palette colors from their declared color
space, and updates instance values. It invalidates dependent cached maps while
reusing compatible shader pipelines. Updates need frame-safe parameter storage
so they cannot overwrite values used by in-flight rendering.

## Execution and caching

Start with material-region evaluation into cached PBR maps followed by generic
composition passes. Do not require a new shader for each material pair, shape,
or layer stack. Preserve the existing execution planner's ability to materialize
expensive intermediate filters. Fusing selected procedural compositions can be
a later optimization behind the same public API.

Cache material evaluation separately from composition. Cache identity includes
asset structure, instance values, coordinate mapping, sampled domain, logical
resolution, requested channels, and filter requirements. Composition additionally
depends on layer order, mask revisions and mappings, and blend settings.

A shape edit invalidates the derived mask and affected compositions. Reordering
layers invalidates composition results. Both changes reuse already evaluated
material regions and compatible pipelines; mask contents and layer order are
runtime inputs to generic composition passes. Newly exposed, uncached material
regions may still need evaluation, but moving a shape does not recompile a
material shader. A material or palette value edit invalidates dependent material
maps and compositions while reusing compatible pipelines. Lighting changes need
not invalidate unlit PBR maps. Pixel-density changes invalidate evaluations whose
sampling footprint changes.

Expand evaluation and mask domains for filters and final height derivatives.
Handle atlas gutters and any requested mip levels using neighboring surface
coverage. Eldiron can select nearest presentation without smooth atlas sampling.
Keep pass, memory, and shader-complexity budgets explicit and return actionable
diagnostics when a request exceeds them.

## Implementation sequence

1. Define stable tile IDs, legacy identity migration, editable asset bundles,
   dependency resolution, and document/source/parameter inspection.
2. Define the five-output Eldiron profile, texture layout, coordinate domain, and
   persistent instance parameter keys. Add WGSL generation and a WGPU runtime for
   one material region, including required intermediate image passes.
3. Add CPU and GPU coverage masks, ordered composition, and Coverage blending.
4. Add the shared logical pixel grid and mask presentation modes.
5. Add persistent palette bindings, instance snapshots, and value-update
   invalidation without embedding palette references in material recipes.
6. Optionally add Normal and Height outputs, HeightAware transitions, and
   final-height normal generation with padding.

## Integration acceptance cases

These cases form the initial integration contract. Compare CPU and GPU results at
the same coordinates and pixel footprint with a declared numeric tolerance, and
use runtime counters or pipeline identities to verify reuse.

| Case | Required result |
| --- | --- |
| Persistent parameter bindings | Bind a palette entry and a per-placement override to a color parameter. Insert a tile before it, reorder tiles and rows, delete an unrelated tile, save, and reopen. Both bindings still target the same tile ID and parameter. Deleting the target produces an unresolved binding, never a binding to a replacement at its former index. |
| Editable asset round trip | Save and reopen a recipe with referenced materials, a custom tile definition, selected material ID, instance overrides, and palette bindings. Reconstruct the row editor from the document and metadata, edit a value or connection, validate, serialize, and recompile. Identities, dependency references, inactive content, and instance data remain editable and semantically unchanged except for the intended edit. |
| Continuous adjacent tiles | Render two adjacent Raw material regions separately and as one larger region, using the same seed, mappings, logical grid, footprint, and padding. Each tile matches its corresponding larger-render region within the declared tolerance, including edge samples. |
| Curved road and decorative strips | Compose grass, a curved asphalt mask crossing a tile boundary, and top/bottom decorative strips using Eldiron-supplied masks. Verify the interior, exterior, feathered transition, strip placement, overlap order, and pixel-grid alignment. Changing layer order changes overlaps as specified without compiling a new material-specific shader. |
| Opacity distinct from coverage | At shape coverage `1`, an incoming material with Opacity `0.25` produces Opacity `0.25`, not `1`. With underlying Opacity `0.4`, incoming Opacity `0.8`, and coverage `0.5`, Coverage blending gives `0.6`. Color remains straight RGB and Emission has strength applied once; neither output is premultiplied by the mask or Opacity. |
| Pipeline and evaluation reuse | Change a color parameter or palette entry without changing structure: dependent maps update while compatible pipeline identities remain unchanged. Move a shape or reorder layers over already cached material domains: masks/composition update while material-evaluation counters and compiled material pipelines remain unchanged. New uncached domains may evaluate without recompilation. |

Automatic neighbour resolution, the dock UI, editable shape storage, and surface
mapping decisions remain Eldiron work. Path-aligned texture coordinates remain a
later extension, separate from the curved coverage masks required here.

## Existing MaterialZ foundations

- [Architecture](architecture.md): typed callable IR, scope analysis, execution
  planning, and complexity limits.
- [Material mixing](material-mixing.md): existing complete-material mixing and
  material dependency validation.
- [Tiling](tiling.md): Raw, Repeat, Seamless, and continuous coordinate sampling.
- [Renderer contract](renderer.md): shared PBR channels, opacity, normals, and
  height semantics.
- [Live editing](live.md): validated constant edits and buffered shader values.
  The existing buffered GPU implementation targets Metal; the proposed WGPU
  runtime extends that design to WGSL and material instances.
