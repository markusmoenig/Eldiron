# MaterialZ Tile Language 1

Tile algorithms and their controls live in `shaders/tiles/*.tile`. Historical
GLSL remains in `shaders/original/` for provenance and regression comparisons.
The Tile tab shows and edits the MaterialZ source that actually executes.

```text
tile 1
name "Color"
category "Style"
group "Color" true
param color color "Color" default 0.2 0.7 0.9 1.0

let alpha = clamp(color.a, 0, 1) * clamp(input.a, 0, 1)
image = vec4(color.rgb, alpha)
```

Declarations provide name, category, description, reference links, groups,
parameter types/defaults/ranges, and tooltips. The UI catalog is read from these
declarations. Types are `float`, `vec2`, `vec3`, `color` (RGBA), and `bool`.
`range MIN MAX` follows a parameter default; `choices "First" "Second"` supplies
an integer-indexed menu; `tooltip "..."` applies to the
preceding parameter. Named output fields use `output_channel y "Nearest Distance"`;
`default_channel y` selects the initial field for new connections. Components must
be unique x/y/z/w values, and a default must refer to a declared field. The catalog
exports these names; connections still store ordinary swizzles such as y or yyy.

Algorithms use immutable `let` values, arithmetic, vector constructors, xyzw/rgba
swizzles, and the math functions abs/floor/fract/sqrt/inversesqrt/sin/cos/radians,
length/dot, min/max/pow/clamp/mix/nonzero, and select(condition, true, false).
Scalars broadcast in arithmetic and compatible math calls. Division uses the
same signed epsilon on CPU and Metal. Inputs are `uv` (vec2), `aa` (pixel width),
`input` (RGBA), and the declared parameters. Image inputs respect the row
reference's RGB component selection and mapping, with alpha carried alongside.

`image = ...` produces RGBA. Shape kernels use `shape = ...` to produce signed
distance; common shape-combination and antialiasing code produces their image.
Transform's `domain = ...` computes the coordinates used to call the input,
followed by `image = input`. Domain expressions cannot depend on that input.
This preserves callable warp behavior rather than warping a precomputed color.
Custom Transform sources conservatively clear metric-distance capability.

Perlin is ported as ordinary vector expressions. The `disc(...)` intrinsic keeps
the curated standalone gradient/hash behavior. Border is an explicit bounded
`primitive border`: distance-band styling when a metric distance is available,
otherwise 16 directions on three rings of callable alpha samples. These two
primitives have CPU/Metal implementations; they are not arbitrary embedded GLSL.

Parsing and type checking lower into the same typed immutable expression IR used
by the Rust evaluator and Metal emitter. Limits: 32 KiB source, 128 SSA statements,
256 tokens per expression, nesting depth 32, and 4096 expression operations per
tile. Bundled curated imports allow 256 KiB, 2048 statements and 32768 body operations.
Graph budgeting also counts expanded expression operations (default 131072)
and repeated source samples, including loop headers. Unbounded loops, recursion, user functions, new
registered kernel types, and iterative image-pass scheduling are not implemented.
Bounded image materialization is available through the execution planner.

Bounded iteration uses mutable `state` declarations and simultaneous-style
expression temporaries before `update` assignments:

```text
state total = 0
repeat 32 times layers while 1
  update total = total + 0.1
end repeat
```

The literal maximum is 1..1024, nesting is limited to eight, and expanded graph
budgets account for the bound. `sample(uv)` re-evaluates the upstream tile.
Imported cell/scatter algorithms use typed `sample_context(uv, position, normal,
time, auxiliary1, auxiliary2, auxiliary3)` to carry per-cell data upstream.
Implicit `pos`, `normal`, `sample_time`, `aux1`..`aux3`, and `aux_valid` expose
that context. Ordinary previews use a plane; 3D sources expose Z Slice and animated
sources expose Time Offset.

Per-tile overrides are stored inside a project:

```text
    use "Color"
      source
        tile 1
        ... complete declarations and algorithm ...
      end source
```

The editor's Apply button validates before replacing the current document. Invalid
code retains the last valid material. Reset restores the bundled definition.
Per-instance overrides currently keep the built-in parameter declarations fixed;
the algorithm body can change. Editing bundled definitions changes the catalog
when the core is rebuilt. Syntax highlighting remains future editor work.

## Tile help and localization

Every registered kernel has a stable `source_id` and `help_key` in the catalog.
Help keys are `tile.<source_id>`, independent of display names and generated
library indices. Imported/maintained library IDs are explicit `id` values in
`shaders/library.json`; base kernel IDs are in `Kernel::source_id`. Keep the ID
when renaming a tile or moving its source file. Parameter IDs also stay stable.

`schema/tile-help.json` is the portable English source of truth. Each tile has
`title`, `summary`, `input`, `output`, `usage`, `notes`, and a `parameters` map.
Every parameter has a localizable title; explanations are optional and may be
expanded over time. Do not change behavior merely to match an old description:
help must explain what the current implementation actually does.

Run `python3 scripts/generate-tile-help.py` after editing English help. It creates
`apple/DenrimMaterialZ/Resources/TileHelp.xcstrings` and preserves existing
non-English translations. Add or edit translations in that string catalog using
Xcode. The project generator retains the languages present in that catalog.
The current shipped content is English; other languages are not yet
translated. Keys use `.summary`, `.input`, `.output`, `.usage`, `.notes`, and
`.parameter.<parameter_id>.title` / `.text`. Never translate source IDs,
parameter IDs, component selections, or document syntax.

The native TileHelp reader first resolves English, then the selected localized
bundle. A missing translated key falls back to English; a missing English title
or summary falls back to catalog metadata, and optional missing sections stay
hidden. Inspector and library help buttons show the same content. Parameter
help tooltips use the same keys. This does not yet localize the rest of the app.

`python3 scripts/generate-tile-help.py --check` validates full active-tile and
parameter coverage, the audit inventory, and catalog freshness without writing.
Native tests additionally compare the core IDs with the help catalog, exercise
partial translations and missing-key fallback, and can verify the compiled app
resources using `MATERIALZ_HELP_BUNDLE=/absolute/path/DenrimMaterialZ.app`.

`schema/tile-audit.json` records the implemented core pass, retired sources, and
remaining conditional replacements. Specialized layouts and legacy shape/profile
behavior remain until equivalent row workflows exist.

## Repetition and instance data

`sample_scoped(uv, footprint, cell_key, group_key)` samples the complete incoming
RGBA with an explicit local footprint and instance identity. `instance_cell`,
`instance_group` and `instance_valid` are read-only built-ins. Ordinary sampling,
coordinate transforms, row inputs and parameter references preserve that scope.
Grid wraps integer keys over its unit repeat. A nested Grid hashes its parent
cell into the child's namespace, and exposes the immediate parent as Group Key.
Cell Data's Instance scope reads those keys independently of transformed UV.

Grid searches three neighboring rows and, after each row's stagger, its nearest
three columns. Bounded jitter, rotation and scale keep this search valid. It
filters its instance clipping and composites premultiplied RGBA, returning
straight alpha. Very dense minification uses conservative footprints rather
than a cached area-average texture. Grid and Warp declare `distance clear`;
repetition and arbitrary warps do not retain an exact metric distance. Distance
connections broadcast to the receiving width, including image inputs.

## Image execution and named fields

`execution image radius units` declares the scalar controls that bound an image
operation's input halo. The algorithm must respect that radius contract. The
current Directional Blur clamps its radius to 0–64. Expensive inputs can become
cached GPU images; arbitrary instance or dynamic-domain calls are not assumed to
be cacheable. See [execution planning](architecture.md#execution-planning-and-evaluation-scopes).

`output_field border "Border Distance" 1 x` declares a stable named field: its
Output mode and packed components. Consumers select `border.x`, `border.xxx`,
`site.xyz`, etc. The field selection is independent of the visible Output menu.
The selected source must declare that field. Lowering shares each specialized
field evaluation and validates its width; no extra authoring row is created.

`sample_scoped` optionally accepts three additional vec3 arguments: local center,
local X basis and local Y basis. The engine composes them with the current
placement frame. Read it through `instance_origin`, `instance_axis_x` and
`instance_axis_y`, or Cell Data's named center/size/angle fields. Parameter slots
follow the 16 read-only built-ins. Standard affine Transform also composes the
frame; nonlinear warps retain the original placement frame.

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
