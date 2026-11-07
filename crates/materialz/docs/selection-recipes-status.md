# Selection-driven MZ geometry: status and handoff

Recorded 2026-11-07.

## Decision

Develop the sandbox game in 2D first. Prioritize a playable gather/craft/trade
loop, ruleset development and MZ tile artwork. Retain the generic selection-driven
3D recipe work as an experimental feature; defer additional construction recipes
and expansion of this bridge until the 2D game establishes a concrete need.

This decision concerns the new selection/Prefab construction experiment. Existing
MZ character, equipment, material and icon workflows also serve 2D development
and remain useful to the game.

## What exists

- MZ **Geometry Input** accepts a named host mesh and retains a portable preview.
- **Fit To Geometry** fits selected axes and aligns anchors with optional padding.
- Creator's generic **Apply MZ Recipe** and **Edit MZ Recipe** actions work with
  selected objects/faces in a region or the isolated Prefab editor.
- A Prop part can retain an MZ binding as its source and normal Geometry Objects
  as its derived rendering/collision cache.
- Bindings retain input references and a source snapshot. Regeneration preserves
  object and face identities for edits that keep topology, plus paint identities
  and explicit face material overrides. Make Prefab Unique remaps the references.
- Saving, reopening and server startup synchronize recipe-backed geometry. Invalid
  or missing inputs retain the last good mesh; a missing Prop mesh cache can be
  rebuilt from its source.
- The first example is a pitched roof fitted to an authored timber frame. Its
  construction and rise/color controls are MZ data, not a roof-specific generator.

The earlier full-cottage generator/action was removed. It is not part of the
current implementation or the intended asset-authoring approach.

## Entry points

| Area | Source |
| --- | --- |
| Portable operations and evaluator | [geometry/mod.rs](../src/geometry/mod.rs) |
| Roof recipe | [pitched-roof.mzgeometry.json](../examples/geometry/pitched-roof.mzgeometry.json) |
| Selection binding, cache and regeneration | [geometry_recipe.rs](../../shared/src/geometry_recipe.rs) |
| Prop geometry source | [block_prop.rs](../../rusterix/src/map/block_prop.rs) |
| Generic editor actions | [mz_recipe.rs](../../../creator/src/actions/mz_recipe.rs) |
| Current usage guide | [MZ Recipes on Geometry](../../../docs/docs/building_maps/mz_geometry_recipes.md) |
| Reproducible example generator | [mz_selection_recipe.rs](../../shared/examples/mz_selection_recipe.rs) |

## Validation recorded at implementation

35 focused tests passed across the MZ geometry/group/input suites, shared recipe
tests, Creator integration/action registration and existing Prop runtime tests.
Coverage included rotated and face-selected inputs, resizing, moving the input
and output together, material/paint identity retention, save/reopen, disposable
cache reconstruction, part pivots, behavior, support references and Make Prefab
Unique. Native runtime captures verified fitting before and after widening a frame.
This records that implementation check, not a full-suite or platform guarantee.

Reproduce the relevant checks from the repository root:

```sh
cargo test -p denrim-materials --features integration --test geometry --test geometry_groups --test geometry_inputs --offline
cargo test -p eldiron-shared --lib geometry_recipe::tests --offline
cargo test -p eldiron-creator --lib actions::mz_recipe::tests --offline
cargo test -p eldiron-creator --lib actionlist::tests::every_registered_action_has_unique_stable_command_metadata --offline
cargo test -p rusterix --lib map::block_prop::tests --offline
cargo run -p eldiron-shared --example mz_selection_recipe -- /tmp/mz-selection
```

The last command needs a native GPU for PNG captures. Set
`ELDIRON_RECIPE_NO_RENDER=1` to export the project and recipe without rendering.
Generated projects, exported recipe copies and preview PNGs are local artifacts;
keep them in `/tmp/mz-selection` or the ignored `generated/mz-selection` directory.
They are not required repository fixtures. The canonical roof recipe is the MZ
source linked above.

## Limits and conditions for resuming

The roof fits rectangular bounds in a horizontal frame. Arbitrary roof footprints,
recipe-generated input chains and animated rig/effect outputs are not supported
by this construction bridge. Topology changes can invalidate face references.
Regeneration can replace direct mesh edits and output-only movements; permanent
offsets belong in the recipe. Shared recipe edits affect all bindings to that asset.
Broader performance and platform coverage are not established by this prototype.

Resume with one concrete building-component requirement when the 2D game needs
a 3D presentation. Build on the shared recipe mechanism and test preservation of
Prop behavior, pivots, surfaces and paint when changing it.

A future **Build 3D from Map** action is a separate design, not implemented here.
During 2D development, keep stable tile/object identities and structural metadata
(type, elevation, blocking and openings) separate from artwork. Such an action
could map those definitions to MZ recipes; it would not reconstruct geometry from
tile pixels or guarantee automatic conversion of arbitrary maps.
