# Curated source provenance

Original repository: https://github.com/markusmoenig/material-z-vg
Source directory: `sources/`, retrieved 2026-10-26/27.

`upstream-manifest.json` records filenames, upstream Git blob SHA-1 hashes,
sizes and import status for all 133 source files. `original/` preserves their
algorithms, parameter declarations, author credits and license statements.
Obsolete `ParamReferences` documentation links are removed. New engine code's
MIT license does not replace pre-existing source terms.

The active catalog contains 98 sources: 14 base/native registrations and 84
library registrations. Noise cleanup removes redundant executable ports and old
names, without an Advanced collection. `library.json` is the active manifest;
`authored` entries are maintained Tile Language families and are not overwritten
by the historical GLSL importer. That importer only rebuilds still-active legacy
entries. Original GLSL remains solely for attribution and algorithm reference.
The four `matmixer_*` shaders are complete material operations in `src/material_mix.rs`.

Credits and source-specific terms remain in each original file, including Brian
Sharpe's Wombat, Inigo Quilez, Stefan Gustavson and the other credited authors.
Brian's Perlin Noise derives from:
https://github.com/BrianSharpe/Wombat/blob/master/Perlin2D.glsl
Its attribution applies to the CPU and Metal derivatives here as well.

## Explicit adaptations

- `tiles/*.tile` is executable MaterialZ Tile Language, not embedded GLSL.
  Typed expressions and bounded loops feed the same CPU and Metal backends.
  `scripts/port-library.py` regenerates the retained legacy ports from the cleaned
  originals, respecting the active manifest; `scripts/generate-library.py` regenerates their Rust registration.
- GLSL vector/matrix operations and branch control are lowered mechanically.
  Loop bodies stay bounded loops; the graph budget includes loop bounds and
  resampled upstream calls. Unbounded recursion is unsupported.
- CPU and Metal use matching float32 sine/cosine polynomials, dot accumulation
  order and disabled fused contraction so sine-based hashes agree across backends.
  This specifies portable numerical behavior; it does not promise historical
  WebGL bit identity. No noise algorithm is replaced with a platform noise call.
- GLSL modulus and unsigned hash conversion have explicit matching semantics.
  Dynamic near-zero divisors use signed epsilon guards.
- Screen-space derivatives become finite differences at the preview pixel width.
  Cut-and-Project's fixed-size arrays/records are flattened into typed values.
- Legacy function inputs remain callable. Tilers carry cell dimensions and IDs
  upstream; Disc uses them for per-cell sizing/variation. Standalone Disc retains
  the fallback box `(1,1)` and ID zero.
- 3D sources expose a Z Slice parameter for the 2D material preview; time-based
  sources expose Time Offset. Extra legacy mixer inputs become vector parameters
  that can also receive connected row values. Original list parameters become
  menus, and RGB color parameters retain RGBA editing controls.
- Material mixers apply to all modern BSDF surface fields, extending the old
  Mask shader's partial field list. Mask with Blend off explicitly sets zero
  opacity for intermediate masks instead of an implicit negative-bump sentinel.
  Unconnected material inputs use standard surface defaults.

The retained library ports have CPU sampling and Metal compilation coverage;
actual-device comparisons cover the curated noise modes and library algorithms. These validate the new backends against each other, not against
captured historical WebGL rendering fixtures. The original Perlin additionally
has an independent curated-reference regression check.

Periodic Profile and Compare/Select are native Tile Language sources authored
for this engine. Their declarations and algorithms are maintained directly in
`tiles/periodic_profile.tile` and `tiles/compare_select.tile`; they have no legacy
GLSL import. Profile filtering uses analytic one-dimensional box integrals with
a locally estimated input footprint.

Cell Data, Directional Grain and Height to Normal are native Tile Language
sources authored for this engine. Cell Data and grain use the existing shared
`hash33` primitive; shared math and hash provenance remains as documented above.
No reference textures or third-party shader source were imported for the wicker.

## Curated noise families

Organic Noise and Organic fBm retain Dave Hoskins' cubic value noise and exact
matching frequency/amplitude/gain/clamp response. The fractal evaluates its noise
internally instead of exposing the legacy auxiliary-input pairing protocol.
Frozen pre-cleanup programs in tests/fixtures verify both outputs exactly.
Perlin and Simplex combine Brian Sharpe's 2D/3D algorithms behind a Dimensions
control. Voronoi now uses the shared integer hash for reproducible seeded sites, complete
bounded searches for F1/F2 in three metrics, and Euclidean bisector planes for
true border distance in 2D/3D. IQ's original cellular implementations remain
archived as provenance for the earlier port.
White Noise uses the existing integer hash with explicit UV cell density and seed.
Spots, Gabor and Wave retain their selected algorithms. General fBm combines the
former octave accumulators with consistent normalization and simultaneous UV/3D
sampling; Warp combines the offset operations with optional vector displacement.
Author credits and original source terms remain applicable to these derivatives.

## General-purpose core pass

Grid, Scatter, Partition, Coordinates, Compose Channels, Levels, Curve, Distance
Profile, Color Ramp and Vector Math are maintained Tile Language sources authored
for MaterialZ, with no legacy
GLSL import. Normal Combine follows the reoriented-normal construction from
Stephen Hill and Colin Barré-Brisebois; shared math/hash primitives retain the
credits above. Grid keeps full RGBA and separate instance/group keys. Mix
consolidates the old color/mixer interpolation controls. Directional Blur is now
a centered premultiplied RGBA Gaussian, rather than the imported green-only filter.
Retired executable ports are removed; historical originals remain archived.

Polygon's maintained implementation is now an authored shortest-edge-distance and
ray-crossing construction, with seeded radius/angle variation. Its legacy source
remains archived. Height Gradient and Vector Filter are authored Tile Language
sources using centered derivatives and bounded vector-trail sampling, respectively.
They reuse the shared math/hash primitives credited above.
