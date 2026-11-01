# Image and optional distance per tile

Every tile now evaluates to an image (straight-alpha RGBA) and an optional signed
distance. The sign convention is negative inside, zero on the boundary, positive
outside. Distance availability is computed in the IR, not tested separately at
each pixel. CPU results expose an `Option<f32>`; Metal carries a distance member
whose validity comes from the compiled node capability. Missing distance is never
implicitly interpreted as an all-inside shape.

## Shapes and combinations

Two native shape sources are included alongside the five preserved historical
sources: `Circle` and `Rounded Rectangle`. Their controls are declared in
`shaders/native/` and appear in the same metadata catalog as the original shaders.

```text
  row "silhouette" called "Badge with Cutout"
    use "Rounded Rectangle"
      set "halfSize" to 0.37 0.32
      set "radius" to 0.09
    use "Circle"
      combine subtract
      set "radius" to 0.115
      set "center" to 0.16 0.11
```

Each shape has a `combine` option: `replace` (default), `union`, `subtract`,
`intersect`, or `smooth union`. Subtract means incoming shape minus the new shape.
Combination consumes the preceding tile, or `input from row "id" xyz`; the image
selector does not alter the parallel distance field. A non-replace combination
requires an incoming valid distance. Smooth union uses the shape's `smoothness`
parameter. These are shape operations, distinct from numeric image addition.

Primitive distances use document coordinates. A rectangle's `halfSize` is half
its full width/height. Its corner radius is capped by the smaller half size.
Constants for circle radius, half size, and smoothness must be at least 1e-6;
rectangle corner radius and border width must be nonnegative. Dynamic shape
dimensions use corresponding numerical guards, but see distance validity below.

The initial CSG formulas are the conventional min/max/smooth-min field operations.
They preserve the intended sign/silhouette; complex joins need not be exact
Euclidean distances everywhere inside overlapping shapes. This is a useful
distance-field foundation, not a general exact geometric distance solver.

## Ordinary color tiles, border and masks

`Color` is an ordinary solid-color source. With no input it covers the plane;
with an input it replaces RGB and multiplies its color alpha by the incoming
image alpha. It preserves optional distance but never needs it. Repeated Color
tiles therefore accumulate opacity normally.

Noise, Mix, and Multiply operate directly on a shape's image while
preserving its alpha mask and optional distance. For example, a row can use:
`Rounded Rectangle → Circle (subtract) → Noise → Mix → Border`.
There are no SDF-specific styling tiles.

`Border` is one ordinary tile with two engine-selected paths: use distance when
available, otherwise derive an inside band from the incoming image's alpha mask.
It preserves the optional distance and the outer silhouette coverage. Border width is in
document units, not screen pixels. For a square N-pixel icon, a two-pixel border
is approximately `2/N` units under the default unit-square preview domain.

```text
  row "styled" called "Fill and Border"
    use "Color"
      input from row "silhouette" xyz
      set "color" to 0.06 0.32 0.65 1
    use "Border"
      set "width" to 0.025
      set "color" to 1 0.72 0.15 1
  output base color from row "styled" xyz
  output opacity from row "styled" w
  output distance from row "silhouette" distance
```

Use the opacity component `w` as a reusable mask, or select `distance` explicitly
for a scalar parameter. The compiler rejects distance references to image-only
tiles/rows. `output distance` must select the actual distance field. Color remains
straight-alpha; RGB can be nonzero under zero alpha. Material base-color rendering
does not composite alpha; tile image previews retain it. The thumbnail CLI
composites over a checkerboard for inspection.

Coverage uses a smooth transition around distance zero. Renderers supply a pixel
footprint of `1/min(width,height)` in document units, shared by CPU and Metal.
`sample_tile` / `dm_tile_sample` take it explicitly; material point sampling uses
0.001 as its default. Inside borders compose in the covered region and apply the
outer silhouette coverage once, avoiding double-opacity at antialiased edges.

## Propagation and validity

| Tile/action | Image | Distance |
| --- | --- | --- |
| Shape | Antialiased white silhouette with alpha | Creates/combines |
| Multiply | Updates RGB; preserves alpha | Preserves |
| Mix | Interpolates RGBA endpoints; multiplies input coverage | Preserves if present |
| Grid / Warp / Directional Blur | Resamples RGBA | Clears |
| Perlin following an input | Generates noise RGB; preserves input alpha | Preserves |
| Color | Solid RGB; multiplies input alpha by color alpha | Preserves if present |
| Border | Inside edge from distance or image alpha | Preserves if present |
| Historical Disc | Original image behavior | Absent |
| Fixed uniform Transform | Transforms input | Scales distance by absolute scale |
| Nonuniform or spatially varying Transform | Transforms input | Clears |
| Row coordinate override | Evaluates in new coordinates | Clears |

Native shape geometry driven by per-sample row bindings also clears the advertised
distance: its sign may describe a silhouette, but it is not generally a metric
distance suitable for uniform-width borders. Image previews remain available.
The policy is deliberately conservative until the IR gains richer distance/field
quality information or distance reconstruction. Color, Noise, Mix, and
Border still work on that image. Border automatically switches to its alpha-mask
path; only explicit distance references and distance previews require a valid field.

Transform preservation requires constant scale, offset, and rotation, with equal
absolute local X/Y scales. Parameter edits remain buffered while this capability
stays unchanged. An edit that changes distance availability is structural and
requires a new shader snapshot.

## Border without distance

The image path approximates alpha erosion with a fixed stencil: 16 directions on
three concentric rings up to `width`, plus the original center sample. The band is
`center_alpha - minimum_sampled_alpha`. It uses alpha, not image brightness, and
preserves antialiased/partial alpha coverage without making edges more opaque.
A fully opaque constant-color plane has no alpha edge to outline.

This bounded approximation can miss small features between stencil samples,
especially for wide borders. It is not distance reconstruction. The analytic
field path remains preferable where available; this choice is internal, not a
separate tile or authoring workflow.

The compiler charges the alpha path for up to 49 evaluations of its source.
Nested expensive borders can exceed the structural complexity budget and are
rejected. Changing border width or color uses the parameter buffer as usual.
Changing the input's distance capability requires compiling a new shader path.

`examples/raster-border.material` demonstrates an ellipse whose nonuniform
transform clears distance, followed by the same Color and Border tiles.

## Tile thumbnails

Every tile, SDF or otherwise, supports `image` preview. Valid distance-producing
tiles additionally support `shape` (coverage mask) and `distance` (grayscale,
`clamp(0.5 + 4*distance, 0, 1)`). This makes the thumbnail slot uniform in the UI.

```sh
cargo run --offline -- thumbnail examples/icon.material icon styled 1 image 256 256 work/icon.ppm
cargo run --offline -- thumbnail examples/icon.material icon silhouette 1 shape 64 64 work/mask.ppm
cargo run --offline -- thumbnail examples/warped-disc.material demo noise 0 image 64 64 work/noise.ppm
```

Rust: `Evaluator::render_tile`, `Evaluator::sample_tile`, and
`metal::compile_tile_preview`. Swift/C: `dm_tile_sample` returns RGBA and distance
in five floats plus a presence result; `dm_tile_shader_compile` produces a normal
buffered shader package for any tile and supported preview mode. It has the same
parameter refresh/lifetime contract as a material shader. Its layout JSON includes
a `preview` descriptor, which is null for material-output shaders.

`examples/icon.material` contains solid-color and Noise → Mix → Border
versions of the badge plus union/intersection studies. `examples/icon-preview.png`
is the textured version, CPU-rendered and checkerboard-composited.
These tiles and preview modes are available in the [native SwiftUI editor](../apple/README.md).
