# Material modes and rendering

Each material declares `mode art`, `mode pbr` or `mode bsdf` (default bsdf). Switching mode
retains all rows and outputs; inactive BSDF connections remain in the document.
Art exposes Color and Opacity and displays the unlit texture. PBR exposes Color,
Roughness, Metallic, Emission and Opacity with a lit preview. BSDF shows common
inputs first: Base Color, Roughness, Metallic, Height, Subsurface and Emission. Normal Map is
an optional advanced input. Other
fields (including cutout Opacity) are added through a searchable Add Input picker.
Connected advanced inputs remain visible below the common fields, in usage order.
There are no collapsible output folders. Clicking a connected terminal disconnects
it; a material may have no connections and use defaults throughout. Unconnected
terminals have inline numeric/color/vector controls, persisted with the material.
Wiring overrides those constants and disconnecting restores them. Distance is auxiliary
procedural data used for shapes/masks/borders and diagnostic export, not a BSDF
terminal. Distance previews remain on individual tiles.

## Shared material contract

`schema/bsdf.json` specifies 40 channel names, widths, defaults, groups, and renderer
bindings. `scripts/generate-bsdf.py` generates `src/bsdf.rs` plus the local
renderer's texture-field mappings. `dm_bsdf_catalog_json` exposes the contract to
Swift. All channels accept explicit row references, component selections and
remapping. Defaults apply when unconnected; the renderer interprets physical
ranges without altering unclamped procedural values.

The supported renderer fields cover diffuse/base color, roughness, metallic,
specular weight/color, IOR, anisotropy, transmission/tint/roughness/IOR, absorption,
thin-walled transmission, coat/tint/attenuation/thickness/roughness/IOR,
sheen/tint/roughness, subsurface weight/color/radius/scale/anisotropy, thin-film
strength/thickness/IOR, volume scattering/tint/distance/anisotropy, emission and
strength, opacity, normal, and height. Normal is encoded tangent-space RGB;
height is displacement in preview scene units. Defaults are in the schema.

## Renderer ownership

`apple/Packages/MaterialZRenderer` is a source copy of the rendering core from
`../DenrimRendererKit`, under Apache-2.0. There is no sibling-checkout or network
package dependency. MaterialZ owns this local copy and its app-facing adapter;
the upstream project is unchanged. The sibling CLI, examples, tests and assets
were not copied. The core retains its mesh and distance-volume infrastructure.

The local extension adds uniform material constants and absolute UV textures for
all BSDF fields, applied at
surface hits before evaluating/sampling the existing layered BSDF. The preview
uses this path tracer, including refraction, absorption, coat, sheen, anisotropy,
subsurface random walks and participating media. Rust still owns procedural
computation; Swift owns scene setup, texture transfer and rendering sessions.

## Surface coverage

BSDF Opacity is an explicit scalar input, independent of Base Color and Height.
Zero removes the surface, one covers it, and intermediate values represent
fractional coverage. The progressive software and hardware paths sample this
coverage stochastically and accumulate premultiplied RGBA. Rejected intersections
do not consume scattering bounces (up to 128 extra intersections per path).
Shadow rays sample the same opacity textures and attenuate light by coverage.
Opacity is separate from glass transmission, refraction and absorption.

The preview composites its backdrop behind transparent pixels. On a closed mesh,
a back-facing strand may still be visible through a front-facing hole. Wicker
connects the union of its strip and rib profiles to Opacity; dark RGB alone does
not make a gap transparent.

## Mesh versus SDF

The default preview is a standalone UV mesh sphere, sky lighting and an area
light. There is no ground geometry. An editable background color, with a subtle
gradient/vignette, is composited behind the sphere independently of environment
illumination and reflections. The accumulated radiance is retained when recoloring
the backdrop. The mesh path can use Metal ray tracing or the copied BVH fallback.
The preview uses 128 segments and 64 rings for a smooth sphere, and 256 segments
and 128 rings when actual displacement is present. In Displacement mode, Height moves actual vertices, affecting silhouettes
and shadows. Displacement uses area-weighted mesh normals with welded seams and poles.
This is a resolution-limited mesh approximation. Displacement does not require
SDF geometry. Height must stay in the preview sphere's supported radius range;
input remapping controls displacement amplitude.

SDFs remain useful for implicit geometry/booleans and future previews. Arbitrary
noise added to a distance function is not automatically a safe metric distance;
traversal needs conservative bounds or a baked volume. The copied core has dense
and sparse distance-volume paths, but MaterialZ does not yet expose a second SDF
preview mode. Material definitions and BSDFs are independent of the geometry path.

## Current preview limits

Connected procedural channels are baked to 512x512 linear float textures, with
bilinear sampling for BSDFs. This is explicit finite-resolution preview caching,
not direct procedural evaluation at every path hit. Texture resolution, filtering,
UV seams and displacement subdivision limit visible detail. The 768x768 path
trace accumulates up to 64 samples with 8 bounces and cancels on edits. Monte Carlo
noise is expected, especially in glass and subsurface materials. Exposure,
external HDRI selection, custom mesh import and production output are future UI
work. A general CPU/GPU execution planner remains future core work. The Art preview does not use the BSDF path.

Native checks cover schema decoding, mode preservation, tile source validation,
mesh displacement, graph edits, and actual texture/thumbnail/path-trace output.
Separate Rust/Metal comparisons exercise the same tile programs on both backends.

## Preview settings and color workflow

The compact settings popup replaces the Sphere/Texture tabs and pattern-display
menu. It contains pattern UV and display controls, background and four lighting
presets. Studio uses neutral key/rim panels, Soft Light uses broad panels and a
bright diffuse environment, Raking Light uses a narrow side strip and dim fill,
and Warm/Cool uses contrasting warm key and cool rim/environment. Presets change
actual area lights and environment radiance, resetting accumulation; the backdrop
is separate and can change without resampling. Flat output is available in the
bottom preview, alongside the selection or by itself.
The preview settings persist in application preferences, not material source text.

All UI color wells use the same custom hue-ring/HSL-triangle picker, including
material defaults, tile properties and preview background. The triangle represents
hue/white/black barycentric weights, with HSL sliders, hex input, opacity for RGBA,
original-color restore. The same reusable picker has a 160-point inline tile-property layout and a
250-point popover layout. The pure-hue vertex rotates to the selected hue and
reaches the hue ring’s inner edge; the image, hit region and marker share that
rotation. Mouse dragging clamps to the triangle;
sliders and the adjustable hue-wheel accessibility control provide alternatives.
Display sRGB colors convert to/from linear material and backdrop storage. Opening
a picker does not rewrite the material; edits retain existing Undo behavior.

## Height, bump and displacement

Height is the single scalar relief input. The compact Height menu in Preview
Settings chooses Bump or Displacement. Bump derives shading normals without
moving geometry; Displacement moves the mesh and uses its normals. Normal Map
remains an optional advanced vector input and combines with height-derived
shading in Bump mode.

The setting defaults to Displacement and persists with other application preview
preferences; it does not alter material documents or exported Height data.
Changing it resets accumulation. Constant height changes geometry in Displacement;
only variation affects Bump. The displaced mesh remains a finite-resolution
approximation. Art and bottom flat previews do not use this setting.

Displacement samples the baked field bilinearly, welds pole positions and seam
normals, and derives normals from the actual mesh triangles. The renderer keeps
geometric normals separate from shading normals for ray offsets and front/back
classification. Perturbed normals are constrained to the visible geometric
hemisphere, preventing false black cracks from rays starting inside the surface.

## Control styling

The dark UI shares Forge’s #FFAB43 accent, including text selections, focus borders
and controls. Numeric fields use a thin custom border. Fields with a 0…1 range
support horizontal dragging while unfocused, Shift for fine adjustment, and click
to type. Dragging updates the document and preview during the gesture, throttled to ten
preview requests per second. Interactive renders skip path-trace refinement; the
full preview refines on release. Undo retains only the gesture’s starting state.

A single Color property omits redundant group and parameter headings. Its compact
picker shows the current swatch and a restore arrow for the color captured when
the tile was selected. The larger popover labels its swatches Current and Original.
