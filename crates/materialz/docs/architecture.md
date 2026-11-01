# Shared engine boundaries

## Implemented

The document model is independent of SwiftUI and any particular Rust application.
It stores stable identities, display names, ordered tiles, bindings, and material
outputs. Lowering resolves forward references, supplies defaults from shader
declarations, validates types, rejects cycles, and computes a dependency order.

The initial IR is a typed, high-level kernel graph. Nodes return RGBA plus optional
signed distance and take implicit coordinates and pixel-footprint arguments.
Distance availability is computed in dependency order. Parameters are typed constants or mapped,
component-selected calls to other nodes. Transform explicitly calls its input in
a different coordinate domain. This represents computation rather than UI layout.
Each node also owns a typed immutable expression program compiled from MaterialZ
Tile Language 1. CPU evaluation and Metal emission share that representation.
Per-instance source overrides are validated and serialized with the project.
See [tile language](tile-language.md) for the supported subset and bounds.

CPU evaluation is a scalar reference implementation. It makes no claim to SIMD
or production texture throughput. Metal generation emits reachable functions in
dependency order and one `material_preview` compute entry point. A consumer binds
a float4 output buffer at index 0 and a uint2 dimension buffer at index 1, then
dispatches the image dimensions. The live backend additionally binds a float4
parameter array at index 2. Buffers must be appropriately sized. Fast math
must be disabled when comparing with the CPU reference.

There are no dependencies beyond the Rust standard library. The small C ABI
passes opaque documents/shader snapshots and explicitly owned strings, rather
than Rust enum layouts or references. CPU and Metal generation can share a
document across threads provided callers synchronize edits and destruction.
Swift owns its UI and Metal
device/command queue; the core has no AppKit, UIKit, or SwiftUI dependency.

## Shader growth protection

Default lowering limits: 256 nodes, depth 64, 4,096 estimated expanded node calls
per root. Shared dependencies count again at each call site because GPU compilation
can inline them. Call count is a conservative structural estimate for these fixed
kernels, not a measurement of GPU instructions or time. Built-in kernel loops have
fixed cost; future user loops require additional work estimation and restrictions.
Image-only Border uses 49 source samples, which are included in expanded-call
costs after distance capability resolution. Border with a distance field needs
only one source evaluation.

The prototype returns a diagnostic when the budget is exceeded. It does not
silently substitute a texture or choose CPU execution. Running everything through
small-looking function definitions does not prevent compiler inlining and growth.

## Execution planning and evaluation scopes

The row document remains procedural. Scope analysis tracks pixel, instance and
footprint dependencies per RGBA component. Instance-uniform shape parameters
preserve metric distance. Cell Data's Instance scope uses a fixed zero key outside
placement; local UV remains pixel-dependent. Standard Cell Data output modes are
specialized without embedding editable numeric controls. Loop control and custom
sources are analyzed conservatively.

Grid, Scatter and Partition pass keys, group identity and composed placement
frames to child rows and parameter references. Cell Data exposes center, size and
orientation directly, avoiding cycles back to a placement row. Frames compose
through affine transforms and describe placement space before nonlinear warps.

Sources can declare `execution image radius_parameter units_parameter`. When
repeated evaluation would exceed the per-pass instruction/call limits, the planner
materializes the input and keeps the filter and downstream work in another pass.
Directional Blur, Height Gradient, Vector Filter, Height to Normal and generated
periodic wrappers use this contract; future neighborhood operators can use it too.
Bounded filters retain their query domain and contribute their halo even when
they stay callable above another filter’s cached input. The compiler keeps the original limits rather than raising them.

`metal::Shader` owns prerequisite pass sources, a shared parameter layout, halo
requirements and affine query-domain transforms. The C API exposes the same plan.
A host executes `material_cache` passes in order into a read/write RGBA32F texture
array at texture(0), two slices per image (premultiplied RGBA and sampled distance),
then executes the ordinary output kernel. All passes share parameter buffer(2).
The material/cache domain is supplied at buffer(4). Standalone embedded-constant
source generation diagnoses outputs that require this execution API.

The native renderer expands domains by filter halos and propagates affine
transforms. It builds premultiplied mip levels with compute passes, samples them
using the requested footprint, caches intermediates by their dependencies, and
invalidates upstream changes while reusing images for downstream edits. Periodic
wrappers bound cached queries to their wrapped domain, including seamless blends,
so distant viewport positions do not enlarge the allocation. Their raster resolution
has a minimum based on the document period. Cache
records retain completion state rather than completed command buffers. Limits are
16 intermediates, 256 MB for a request's image array and 128 MB of persistent
logical image data. These complement the existing graph and per-pass limits.

Rasterization has a resolution and interpolation error; the CPU evaluator remains
an exact callable reference. Sampled distance is an image field. Filters clear
metric-distance capability. A cache reached through an unsupported dynamic domain
or instance-dependent placement is diagnosed, rather than sampled from the wrong
context. Put filtering after that operation, or use a cheaper callable motif.
Compatible cached domains currently share an array/resolution, so extreme scale
differences can reach the resource budget before a future per-resource allocator.

Distance transforms, connected-component/flood-fill passes, morphology and
height-derived AO/curvature are not implemented yet. They require additional pass
kernels and, for iterative algorithms, resource lifetime/reuse scheduling. The
execution interface and row model can accommodate these additions; arbitrary
context-dependent procedural calls cannot be silently replaced with 2D textures.

## Named fields and coordinate domains

A connection can select `border.xxx`, `site.xyz`, `key.xy` or another declared
field. Stable field IDs are independent of the source's visible Output menu.
Lowering shares specialized field evaluations; the document and UI retain the
original row identity. This removes the four-image-channel ceiling for source
fields without adding Split tiles. Sharing the underlying Voronoi/stamp search
between different fields is a future common-subexpression optimization.

Coordinate bindings accept XY or XYZ, update the position context, and estimate a
local footprint. Coordinates provides affine 3D, polar and cylindrical data.
Grid Rings varies sector count with circumference. Partition uses shared 1D
Voronoi boundaries; Scatter evaluates a bounded seven-by-seven neighborhood and
supports multiple stamps, layers, Over/Add/Maximum and winning fields. Dense
minification and curved/nonlinear domains remain approximate, as documented in
source help.

The native macOS app owns a local copy of the RendererKit rendering core, without
a sibling dependency. Procedural output channels are shared through a generated
BSDF schema. Art/BSDF modes, texture baking, path tracing and displacement limits
are described in [renderer integration](renderer.md).

## Repeated parameter samples

Within one node evaluation, parameters that reference the same resolved source
node share its full RGBA/distance sample. Selection and remapping are applied to
each binding afterward. CPU evaluation, Metal emission and cost accounting use
the same rule. Input sampling and coordinate evaluation retain separate domain
semantics; this is not cross-instance or cross-field search reuse.

The modern Polygon computes shortest distance to ordered polygon edges and a
ray-crossing sign. Seeded radius/angle variation retains vertex order. Geometry
controls must be instance-uniform to expose metric distance, as for the other
analytic shapes. No raster distance transform is implied.

Height Gradient uses centered UV derivatives and a signed Laplacian. Vector
Filter uses a locally fixed, bounded vector trail; extrema select a complete RGBA
sample by red-channel height and the mean uses premultiplied alpha. UV radii keep
the effect scale fixed across bake resolutions; pixel radii follow the requested
footprint. Arbitrary distance transforms and evolving-flow erosion remain future
kernels.
