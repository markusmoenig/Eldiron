# BuilderGraph Design Guide

This guide describes the long-term direction for BuilderGraph as Eldiron's shared procedural authoring system. The goal is to avoid adding one hardcoded editor action for every specific feature, while still keeping the workflow accessible through templates and parameters.

## Core Direction

BuilderGraph should become the single source of truth for procedural geometry and procedural decoration.

It should cover:

- Freestanding props and assemblies.
- Hosted sector, linedef, and vertex geometry.
- Surface details on walls, floors, and ceilings.
- Terrain and organic scatter.
- Static billboard vegetation and decoration.
- Dynamic billboards where interaction or animation requires them.

The authored source should stay script-based and use the `.buildergraph` file extension. A node graph can still be a future editing or visualization layer, but the durable asset format should be a builder script.

## Mental Model

Builder scripts generate an assembly against a host.

```text
Builder script + Host context -> Assembly IR -> Preview / Chunk builder / Renderer
```

The host defines where and how the script is evaluated. The assembly IR defines what was generated.

Example host targets:

```text
object
sector
linedef
vertex
surface
terrain
```

The same script language should support all of them. A "DetailGraph" is not a separate engine. It is a BuilderGraph script using a surface host.

## Template-First UX

Most users should not write scripts.

Normal workflow:

```text
Select wall/floor/terrain region
Open Treasury
Pick a template
Adjust exposed parameters
Apply
```

Builder scripts should expose public parameters:

```text
param arch_count int auto range 1 16
param arch_width float 1.2 range 0.5 4.0
param column_radius float 0.08 range 0.02 0.3
param cut_openings bool true
param trim_material material
param seed int 0
```

The UI renders these as sliders, toggles, material slots, selectors, seed fields, and similar controls.

Treasury templates become the product layer:

- Wall Details: arches, columns, trims, windows, alcoves, battlements.
- Floor Details: borders, mosaics, raised platforms, curbs, steps.
- Vegetation: grass patches, bushes, reeds, tree clusters, vines.
- Props: barrels, crates, signs, lamps, fences.
- Dungeon Dressing: torches, cracks, rubble, chains, banners.

## Host Contracts

The first major design task is to define clear host contracts. Scripts should consume host data through stable names instead of knowing editor internals.

### Common Host Data

```text
host.kind
host.id
host.seed
host.bounds
host.material
host.material_slots
```

### Surface Host

Used for wall, floor, ceiling, and profile-map detail regions.

```text
host.surface.origin
host.surface.u_axis
host.surface.v_axis
host.surface.normal
host.surface.thickness
host.surface.side
host.surface.loop
host.bounds.min_u
host.bounds.max_u
host.bounds.min_v
host.bounds.max_v
host.width
host.height
```

Surface scripts should generate in UVW space:

```text
u = surface horizontal/local axis
v = surface vertical/local axis
w = surface normal offset
```

This lets the same detail script run on any wall orientation.

### Linedef Host

Used for fences, wall spans, rows, railings, and similar linear structures.

```text
host.start
host.end
host.center
host.along
host.outward
host.up
host.length
host.height
host.width
```

### Sector Host

Used for sector-local props, floor details, roofs, platforms, and room dressing.

```text
host.loop
host.bounds
host.center
host.floor_y
host.ceiling_y
host.width
host.depth
host.height
```

### Terrain Host

Used for vegetation and organic scatter.

```text
host.region.loop
host.terrain.height(x, z)
host.terrain.normal(x, z)
host.density
host.seed
```

## Assembly IR

Builder scripts should output an intermediate assembly representation. The CLI, preview renderer, chunk builder, and future editor tools should all consume this same IR.

Suggested assembly entries:

```text
MeshPrimitive
LoftPrimitive
CutMask
StaticBillboardBatch
DynamicBillboard
Light
Anchor
MaterialSlot
Warning
```

This avoids coupling scripts directly to renderer objects.

## Surface Details

Profile maps should remain the primary placement/editing surface for local details. They answer:

- Where on the surface does the detail happen?
- What region, loop, or span is selected?
- Which side does it target?
- Which material overrides apply?

Builder scripts answer:

- What geometry is generated?
- Does it overlay, cut, or replace the host?
- How does it repeat or fit?
- Which materials and variants does it use?

Surface detail modes:

```text
overlay    // Add geometry on top of host surface.
cut        // Subtract a region from the host surface.
replace    // Replace host patch with generated geometry.
cut_overlay // Cut opening and add generated frame/detail.
```

Existing actions should gradually become thin template applications:

```text
Relief    -> surface detail script: extrude selected loop
Recess    -> surface detail script: inset selected loop
Billboard -> surface detail script: cut + billboard plane
Window    -> surface detail script: cut + frame + glass
Fence     -> linedef builder script
Arch      -> surface detail script
```

## Architectural Transitions

UO-style architecture depends on transition geometry. Stacking a cylinder on a square block is not enough.

Builder scripts need primitives for shape transitions:

```text
loft rect(w, d) to circle(r) height h
loft rect(w, d) to ngon(sides, r) height h
beveled_box w d h bevel
cylinder radius h sides
lathe profile around_y
profile_extrude shape height
```

Important uses:

- Square base to round column shaft.
- Round shaft to square capital.
- Octagonal block to cylinder.
- Decorative column bases.
- Column capitals.
- Arch impost blocks.
- Trim and cornice profiles.

`lathe` is especially important for columns. A column can be built from side-profile revolutions plus a shaft:

```text
lathe base_profile
cylinder shaft
lathe capital_profile
```

`loft` should generate a mesh between two closed rings. Internally, rings can be resampled to compatible vertex counts and joined with quad strips.

## Procedural Variation

Organic and repeated procedural content needs deterministic variation.

Builder scripts should support:

```text
seed
random(min, max)
choice(...)
jitter_position
jitter_scale
jitter_rotation
variant_tile
variant_material
noise
density_mask
```

Randomness must be stable. Use deterministic seeds derived from stable inputs:

```text
world_seed + graph_id + host_id + instance_index
```

This ensures vegetation and decorations do not reshuffle after reloads or chunk rebuilds.

## Static And Dynamic Billboards

Current dynamic billboard rebuilding is fine for interactive or animated objects, but it is not a long-term fit for thousands of grass, bush, tree, or ivy billboards.

Split billboards into two categories.

### Dynamic Billboards

Use for:

- Characters.
- Particles.
- Animated doors and gates.
- Interactive items whose state changes often.

These can continue to use the dynamic path.

### Static Billboards

Use for:

- Grass.
- Bushes.
- Trees.
- Reeds.
- Ivy.
- Non-interactive dressing.

Static billboards should be generated during chunk build and uploaded as static instance data.

Suggested static billboard batch:

```text
StaticBillboardBatch
  tile_or_atlas_id
  positions
  sizes
  rotations
  tints
  variants
  normals
  facing_mode
```

Facing modes:

```text
camera_facing
axial_y
fixed_cross
ground_aligned
mesh_proxy_lod
```

The renderer may still orient static billboards toward the camera, but the instance data should not be rebuilt every frame.

## Script Features To Prioritize

Minimal architectural set:

```text
box
cylinder
loft
lathe
beveled_box
repeat_u
repeat_v
fit_to_bounds
cut_rect
cut_arch
material_slot
host_material
```

Minimal organic set:

```text
scatter
billboard
random
choice
jitter
terrain_height
terrain_normal
density
```

Minimal template set:

```text
param
range
default
material parameter
bool parameter
seed parameter
preview host
```

## BuilderGraph CLI

The buildergraph CLI should become the main test and debug environment.

Current CLI behavior is minimal: parse/evaluate a script or graph, print primitives/anchors, and save a PNG preview.

Target CLI commands:

```text
buildergraph check file.buildergraph
buildergraph inspect file.buildergraph
buildergraph eval file.buildergraph --host surface --host-json host.json --out assembly.json
buildergraph render file.buildergraph --host surface --png out.png --size 512
buildergraph export-obj file.buildergraph --host surface --obj out.obj
buildergraph snapshot tests/builders/*.buildergraph
```

Fake host shortcuts:

```text
--host wall --width 6 --height 3 --thickness 0.3
--host floor --width 6 --depth 6
--host linedef --length 8 --height 2
--host terrain --size 16 --seed 42
```

Debug output should include:

```text
mesh count
triangle count
billboard count
cut mask count
material slots
anchors
bounds
warnings
assembly hash
```

Warnings should catch:

- Invalid profiles.
- Zero-size primitives.
- Overlapping or out-of-bounds cuts.
- Non-manifold lofts.
- Missing materials.
- Unsupported host requirements.

## Testing Strategy

Builder scripts need deterministic tests.

Recommended test layers:

1. Parser tests.
2. Host contract validation tests.
3. Assembly JSON snapshot tests.
4. Geometry invariant tests.
5. Preview PNG snapshots for representative templates.
6. OBJ export for manual inspection.

Geometry invariants:

```text
no NaN vertices
valid triangle indices
non-empty bounds
expected primitive counts
expected material slots
stable assembly hash for fixed seed
```

## Milestones

### Milestone A: Host Contract And Assembly IR

- Define host structs for surface, linedef, sector, vertex, and terrain.
- Define assembly IR.
- Update builder script evaluation to output IR.
- Add assembly JSON export in CLI.

### Milestone B: Surface Detail Basics

- Add surface host target.
- Emit boxes/cylinders in UVW space.
- Support overlay mode.
- Add preview host for wall and floor.

### Milestone C: Cut And Replace

- Add cut masks to assembly IR.
- Let surface detail scripts cut host surfaces.
- Migrate simple hole/window behavior toward script-generated cut + geometry.

### Milestone D: Architectural Primitives

- Add loft.
- Add lathe.
- Add beveled box.
- Add repeat/fit helpers.
- Build first UO-style templates: wall arcade, column pair, trim band.

### Milestone E: Script Parameters And UI

- Add public script params.
- Render params in the editor UI.
- Support material slots.
- Store per-placement parameter overrides.

### Milestone F: Static Billboard System

- Add static billboard batch IR.
- Add chunk build path for static billboards.
- Add renderer support for static billboard batches.
- Add vegetation templates.

### Milestone G: Treasury Packs

- Package curated templates.
- Add previews.
- Add categories.
- Add template metadata and descriptions.

## Design Constraints

- Prefer one script language over one action per feature.
- Keep host contracts explicit and stable.
- Keep procedural output deterministic.
- Keep templates accessible through parameters.
- Avoid coupling builder scripts directly to renderer internals.
- Keep current actions working during migration.
- Use the CLI for fast iteration before editor integration.

## Open Questions

- Final syntax details for builder scripts.
- Exact public parameter syntax.
- How much of profile-map editing should be exposed as "detail placement" in UI.
- Whether graph/node visualization should round-trip to script or remain a read-only view.
- How static billboard batches should be represented in SceneVM.
- How cut masks should interact with existing surface profile loop classification.
- How Treasury templates should declare compatible host types.
