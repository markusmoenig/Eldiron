# Procedural material core

This is Eldiron's self-contained copy of the MaterialZ Rust core. It is a normal
workspace member at `crates/materialz`; the package/import names remain
`denrim-materials` / `denrim_materials`. Creator enables its `wgpu` feature.
Building Eldiron does not require a MaterialZ checkout or a MaterialZ Git remote.
Normal third-party Cargo dependencies still use the workspace lockfile.

The copy includes the Rust library and CLI, shader sources and attribution,
schemas, C header, Rust tests and their fixtures, material recipes, the WGPU
example, core documentation, and library/channel generators. The Swift application,
Apple renderer package, Xcode project, generated previews, build artifacts and
upstream Git metadata are excluded. Swift files under `tests/` are small optional
Metal test harnesses, not an application dependency; the default tests do not
require Swift or a GPU.

## Imported snapshot

Imported on 2026-10-29 from the current MaterialZ working tree, based on commit
`6ed7d95ae21e23b6d7a9a04a856d56c10e6d33e6`. That checkout contained uncommitted
core changes required by Eldiron, so this copy deliberately includes them.

- [UPSTREAM.toml](UPSTREAM.toml) records the base revision and local adjustments.
- [UPSTREAM_FILES.json](UPSTREAM_FILES.json) records SHA-256 hashes of both the
  original working-tree files and the initially vendored files.
- [UPSTREAM_STATUS.txt](UPSTREAM_STATUS.txt) records the source working-tree status
  for the imported areas at copy time, including deleted legacy sources.

At import time, two adjustments were confined to support files: the example audit
tested only the included Rust recipes, and `scripts/generate-bsdf.py` generated Rust
channel definitions without modifying the excluded Apple renderer. Engine and
shader files were otherwise unchanged in that initial snapshot. Eldiron's later
material, image-source, geometry, rig and animation work lives in this copy. The
hashes remain an import record; subsequent changes are tracked in Eldiron's Git history.

Updates from standalone MaterialZ are explicit source imports. Preserve Eldiron
changes and refresh the provenance records when importing another snapshot;
there is no automatic synchronization or external build-time source lookup.

## Checks

Run from the Eldiron repository root:

```sh
cargo check --workspace --offline
cargo test -p denrim-materials --features wgpu --offline
cargo test -p eldiron-creator --lib procedural_material --offline
```

GPU checks require an available adapter:

```sh
cargo test -p denrim-materials --features wgpu --test wgpu_runtime --offline -- --ignored
cargo test -p eldiron-creator --lib procedural_material --offline -- --ignored
```

The explicitly ignored macOS Metal tests additionally require Swift/Xcode tools.
Test-generated reports and native harnesses live in the ignored `work/` directory.
The `generate-library.py`, `generate-bsdf.py` and `port-library.py` scripts operate
on this copy; builds use checked-in generated sources and do not run generators.

## Documentation and attribution

For current Creator usage, start with [Procedural Materials](../../docs/docs/building_maps/procedural_materials.md),
[Appearance](../../docs/docs/characters_items/appearance.md) and
[Rigging](../../docs/docs/characters_items/rigging.md). These document the native
sidebar/minimap and full geometry viewport, map composition, ruleset item states,
palette bindings and settings. The production renderer bakes materials; there is
no live per-pixel GPU material VM.

The [Eldiron geometry prototype](docs/geometry-prototype.md) implements the first
editable geometry rows and sword experiment, including its current limits.

The [unified appearance concept](docs/unified-appearance-concept.md) records the
original proposal and subsequent implemented geometry, rig, animation,
attachment and icon milestones. Its dated sections separate measured native
behavior from remaining platform, live-game and performance acceptance work.

Start with the [Eldiron integration API](docs/eldiron-integration.md),
[document format](docs/format.md), [tile language](docs/tile-language.md), and
[architecture](docs/architecture.md). These are upstream documentation snapshots;
references to the standalone Apple app describe components not included here.

Preserve [LICENSE](LICENSE), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md),
[shader provenance](shaders/PROVENANCE.md), and the source-specific notices in
`shaders/original/`. The upstream third-party notice also discusses the excluded
Apple renderer; that renderer is not part of this import.

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

These animation and Grid additions were back-integrated into the original local
MaterialZ checkout on 2026-10-31, including Swift preview controls and native tests.
