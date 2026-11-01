# Project format 1

The file is a UTF-8 controlled language with explicit syntax. It does not
require an LLM. Two-space indentation establishes scope. Blank lines and comments
starting with `#` outside quoted strings are ignored. Quoted strings support
escaped quotes and backslashes; multiline strings are not supported. Numeric
constants must be finite 32-bit floats. Boolean constants can be `true` or `false`.

```text
project 1

material "surface" called "My Surface"
  position 245.25 708.5
  row "noise" called "Noise"
    use "Perlin Noise"
      set "globalScale" to 0.8
      set "localScale" to 1 1.3
  row "shape" called "Shape"
    use "Disc"
      set "inBEVEL" to 0.2
    use "Transform"
      set "rot" from row "noise" x mapped -45 to 45
    use "Mix"
      set "factor" to 1 1 1
      set "color1" to 0.02 0.03 0.05 1
      set "color2" to 0.2 0.8 0.6 1
  output base color from row "shape" xyz
  output roughness from row "noise" x mapped 0.3 to 0.8
```

Material and row IDs are unique within their respective scopes. Labels are for
display and can be renamed independently. A row label is optional: `row "noise"`
is valid and serializes without a `called` clause. Unnamed rows retain their stable
IDs; the UI supplies a positional display title. Row references are material-local and
may refer forward. A dependency cycle is an error. Every row needs a tile and
material outputs are optional: unconnected BSDF fields use renderer defaults.
`mode art` / `mode pbr` / `mode bsdf` at material scope selects the surface
presentation (default bsdf), preserving all stored outputs and defaults when
switching modes. Art exposes Color and Opacity. PBR exposes Color (`base color`),
Roughness, Metallic, Emission and Opacity for Eldiron; its lit preview ignores stored
advanced BSDF inputs. BSDF exposes the full surface model. PBR uses the same typed
channels, CPU evaluation and Metal generation as BSDF, with no separate map format.
Unknown sources, parameters, channels,
and duplicate declarations are errors.

## Persistent tile identities

Tiles may declare an opaque identity: `use "Color" id "tile-pigment"`. Identities
must be nonempty and unique across the document. Canonical serialization always
writes them. Legacy `use "Color"` declarations receive IDs on import; persist the
migrated recipe before saving bindings. Reimporting unsaved legacy text assigns
new IDs. Reordering or moving tiles keeps their IDs, while newly copied tiles
must receive new IDs through `format::new_tile_id()`.

The Eldiron integration API binds by asset, tile, and parameter ID. Row names,
array positions, and generated parameter-buffer offsets are not persistent tile
identities. See [the integration API](eldiron-integration.md) for editable bundles
and live instance overrides. Older readers must be upgraded to accept the added
tile identity clause.

## Material fallback values

Unconnected material terminals can store constants at material scope:

```text
  default roughness to 0.35
  default metallic to 0.8
  default base color to 0.1 0.2 0.3
```

Values must match the channel width and be finite. Unknown/duplicate defaults and
Distance defaults are rejected. A row connection takes precedence, preserving the
constant as a fallback; disconnecting restores it. Omitted constants use schema
defaults. Defaults do not create procedural rows. They are preserved in project
text, the structured ABI snapshot, and Undo/Redo. CPU sampling and Metal output
compilation resolve the same fallback values. Buffered default-output shaders use
one float4 at buffer(2), with no tile parameter slots; refreshing the value keeps
the shader source unchanged.

## Tiles and callable inputs

`use "Source Name"` appends a tile. Parameters default to their original shader
metadata. A tile that consumes an input uses the preceding tile in its row, unless
an explicit `input from row "id" xyz` is supplied. A first Multiply or Transform needs an explicit input. Mix accepts constant
endpoints and Factor without an image, or uses an image as a factor mask.
Noise sources accept an optional incoming alpha mask; standalone Disc is a
retained historical source that does not consume an image.

Transform is not a post-process over one previously evaluated value: it evaluates
the input function at transformed coordinates. Parameter functions are evaluated
at the receiving tile's coordinates before that tile's own transform is applied.

`coordinates from row "id" xy` (or `xyz` for position) at row scope replaces the coordinates at the entry
to the entire row. The coordinate source is evaluated using the incoming
coordinates; all of the row's tiles then run in the replaced domain. This applies
once, not once per tile. Coordinate displacement can also be driven through a
Transform tile's `offset` parameter. There is no implicit conversion of RGB into
an angle or coordinates.

Row-owned authoring connections use `row input "connection-id" from row "source" xyzw`
at the same indentation as `coordinates` and `use`. The ID is stable within its
owning row. Source channels and optional `mapped LOW to HIGH` are preserved in
canonical text, native bundles, and structured document snapshots. Tile bindings
still determine how a connection is used. Removing a receiving tile can leave
the connection unassigned so another tile can use it later. Dormant connections
do not evaluate their source or introduce dependency cycles, but unknown source
rows and invalid channels are diagnosed. An empty row that owns connections is
retained for editing and evaluates to neutral white until a tile is added.

## Types and mapping

Every row returns an RGBA image (`xyzw`) and optionally signed `distance`.
Component selection is explicit: `x`, `xy`, `xyz`, `zy`, `xxx`, and other selections of x/y/z/w are valid
when their width matches the destination. A scalar parameter needs one component,
a vector2 needs two, vector3 three, and color four (for example `xyzw`). Width
mismatches are errors. `distance` is scalar and broadcasts to the receiving
width, including image inputs. Boolean parameters currently accept constants only.

`mapped LOW to HIGH` means `LOW + value * (HIGH - LOW)` per selected component.
It assumes a nominal 0..1 source range but does not clamp. Noise, ID fields, and
other channels can have different natural ranges; the author must choose a mapping.

Metadata slider ranges are authoring hints, not execution clamps. Constants used
as divisors in the implemented kernels must have magnitude at least 0.000001.
Dynamic divisors use a signed epsilon of that magnitude on both backends.

Mix interpolates RGBA endpoints using a per-channel Factor, multiplied by an
optional image mask. Endpoint alpha uses the mean clamped factor and multiplies
input coverage; valid input distance is preserved. Color and Border use full RGBA and work with image-only inputs as
well as shape results. See [distance propagation and styling](sdf.md).

## Outputs and sampling

Surface channels and their widths/defaults are defined in `schema/bsdf.json` and
exposed through `dm_bsdf_catalog_json`: base color, specular/transmission/coat/sheen,
subsurface, volume, emission, geometry and coverage. Color and normal outputs
use three components; scalar fields use one. See [renderer integration](renderer.md).
An auxiliary `distance` diagnostic/export channel remains available to core clients,
while the material UI exposes distance only through tile previews and row inputs. Distance must reference
`distance` from a row with valid distance capability; opacity commonly selects `w`.
Output references may also remap their values.
Output names do not imply clamping or color-space conversion.

Standalone Disc receives the original default shape box `(1,1)` and element ID
zero. Per-element IDs, pattern contexts, and scatter generators are not part of
this initial slice. CPU and GPU renders sample pixel centers in `[-0.5,0.5]^2`,
with the first row at minimum Y. No aspect compensation or seamless tiling is
implicitly added. Native SDF shapes and their per-shape `combine` options are
documented in [SDFs and icons](sdf.md); these extend format 1 without changing the
meaning of existing documents.

`format::write` produces canonical text, preserving IDs and semantic structure.
Comments and whitespace are not retained. Lossless editor/text round-tripping and
optional layout metadata remain future work. Unsupported format versions fail
explicitly instead of silently changing meaning.

## Tile source overrides

A tile may contain a `source` / `end source` block at six-space indentation,
with its complete MaterialZ definition indented eight spaces. Source text and
comments inside that block are preserved. Per-instance source overrides keep
the built-in parameter declarations fixed. Algorithms are parsed/type-checked
before the project is accepted. See [Tile Language 1](tile-language.md).

## Material widget positions

An optional `position x y` at material scope stores the widget’s absolute top-left
position in unscaled canvas coordinates. Both numbers must be finite and
nonnegative; duplicate position declarations are rejected. Omitting it retains
the editor’s automatic placement. Positions round-trip through the text format
and structured ABI snapshot, without affecting procedural evaluation or exports.
Dragging commits one project edit on release, supports Undo/Redo, and restores
that placement when the project is reopened. Layout-only edits reuse the preview.

An empty editor document is `project 1` with no materials. Materials may have no
rows. A retained processor whose source was deleted can use `input disconnected`;
this explicitly evaluates against the neutral white image until reconnected.
It keeps the processor and its controls editable without a dangling row reference.

## Material lock

An optional `locked true` at material scope saves the editor lock; omitted means
unlocked. `locked false` is also accepted. Locking hides the tile/row growth
targets and prevents editor changes to the material’s
properties, tiles and connections. Locked materials remain movable. Selection and rendering remain available, as
does using the complete material as an input to an unlocked mixer. The lock is
editor metadata and does not change material evaluation.

Material mixer records may include `stack slot 1` through `stack slot 4` to bind
them to a fixed editor terminal. Slots must be unique and only appear on mixers.
The explicit mixer inputs and project result describe their render dependency
chain, so CPU and Metal evaluation use the same saved composition.

Named data fields use `field.components` on a normal connection, for example
`input from row "cells" border.xxx` or `set "offset" from row "cells" site.xy`.
The field is independent of the source tile's displayed Output selection. Cell
Data exposes instance size/center/angle without referencing the parent placement
row. Coordinate connections accept two or three components and estimate the
transformed sampling footprint.

## Looped material animation and continuous Grid identities

Materials may declare `animation 16 2`: 16 baked frames played over two seconds.
The supported range is 1–32 frames and 0.25–60 seconds; one frame is static.
Exports evaluate normalized sample times `i / frames` over [0,1). Playback duration
changes speed, independent of the sampled loop. `Loop Flow` moves and crossfades
two translated copies with quintic weights so time 0 and 1 join in value and
velocity. Its `velocity` is travel in UV units per cycle. Time-aware sources that
are not designed to loop still require authored loop closure.

Rust GPU hosts provide `RenderRequest.time`; static requests use 0. CPU reference
sampling is available through `Evaluator::sample_at_time`. Generated native Metal
preview and cache kernels take a float time at buffer 5; hosts must bind it,
including zero for static previews. Time changes reuse pipelines. Intermediate
caches must include time in their identity or invalidate their last sample.

Grid's **Cell Identities → Continuous** option uses unwrapped instance cell keys.
`Cell Data` in Instance scope then produces stable, distinct values across world
coordinates. Material Repeat/Seamless modes still wrap the complete pattern. The
`continuous-bricks.material` example uses three nearby clay tones with continuous
keys; `looping-water.material` demonstrates the animated PBR workflow.
