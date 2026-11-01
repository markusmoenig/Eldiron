# Material tiling

Materials carry an explicit boundary mode and repeat period in the shared IR:

```text
material "m" called "Seamless Material"
  tiling seamless
  period 1 1
  seam blend 0.15
```

Raw preserves the source coordinate domain. Repeat wraps coordinates into the
centered repeat rectangle; mismatched edges may still have a discontinuity.
Seamless wraps and crossfades shifted source evaluations in the upper edge band
of each axis. Quintic weights have zero first and second derivatives at the blend
boundaries. For continuous source fields this joins values and slopes at the
repeat seam while preserving the central region. Blend width is a fraction of
the period (0.01..0.5); periods must be finite and positive.

Seamless reconstruction changes the pattern inside the blend bands. It does not
remove intentional internal discontinuities such as checker boundaries or turn
white noise into a smooth field. Native periodic noise/pattern formulas remain
intact. No shader parameter is silently rounded or coerced to an integer period.

The compiler lowers periodic material channels to ordinary typed callable nodes
before backend compilation. Material mixers consume their inputs' periodic
channels. The wrapping is shared by CPU, Metal, live parameter buffers and the
C ABI; it is not a preview-only texture sampler setting. Seamless nodes cost four
source evaluations; Repeat costs one. Existing graph/call/instruction limits
include those costs and reject excessive expansion.

Image tile previews use the same reconstruction, including alpha. Signed distance
and shape diagnostic previews preserve the original distance field; blended images
are not advertised as metric SDFs. Material Height participates as an image field.
The sphere's curvature-aware bump conversion is still sphere shading, not a
flat normal-map export implementation.

New materials and the default new document use Seamless. Documents without a
tiling declaration retain Raw interpretation.
Select a material header or its small tiling menu to choose the mode. Properties
edits period and blend width. The bottom preview fills its entire area with
gutter-free repeats, using tiles half its height to show seams along both axes.
Toolbar buttons animate the bottom preview and right sidebar in or out.

Regression tests cover wrapping, edge values/slopes, central-region preservation,
non-square periods, alpha, live buffer updates and nested material inputs. Actual
Metal results are compared with the CPU evaluator for material and tile previews.

## Preview UV domains

The bottom preview has two UV modes, available from its grid and infinity icon buttons. **UV 0…1** evaluates the literal square [0,1] on each axis and
repeats that image at half the panel height. **Continuous UVs** evaluates one
rectangular region with square UV proportions across the full panel; it does not
repeat a baked unit image. Drag to pan through positive or negative coordinates.
The active UV mode is highlighted; coordinate readouts and reset commands are omitted.

Material wrapping remains part of the authored program in either view. Use Raw
material tiling for an endless, non-repeating procedural field. Repeat and
Seamless intentionally remain periodic even when sampled at world coordinates.
For adjacent world tiles, use the same material parameters and supply each tile’s
UVs from its world position, rather than resetting UVs independently to [0,1].
Library source previews always inspect raw default source behavior. Tiling checks
sample the raw [0,1] square and unit offsets, and are only a default-setting check.

The Rust Metal generator provides a `material_preview_region` entry point with
a region origin/span uniform in buffer 3. The historical centered unit-preview
entry point remains available for material spheres and thumbnails. Integration
checks compare a wide raw-noise render with two adjacent world-coordinate tiles.

## Selection and material output

The bottom toolbar selects Output, Selection, or Mixed using three icons with
help text. Mixed is the default: the selected tile or library source appears on
the left, and the resolved final material’s flat base color plus opacity appears
on the right. Both panes use the same UV origin and span. The selected display
mode is saved with preview preferences, separately from the project. The top-left
BSDF preview remains the sphere; there is no separate Texture view option.

The editor labels Raw behavior as **Continuous UVs** with the infinity icon,
matching the bottom UV preview controls. The stored project keyword remains
`tiling raw`. The other material choices are UV 0…1 · Repeat and UV 0…1 ·
Seamless. Tiling Settings opens a material-local popover with UV mode, period
and edge-blend controls, independent of sidebar visibility and tile selection.
Only the title/empty header area selects or drags the material; toolbar controls
do not replace the current tile or library preview selection.
