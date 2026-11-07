---
title: "MZ Recipes on Geometry"
sidebar_position: 9
---

MZ recipes can use selected map geometry as input and generate a fitted component.
The same mechanism works in a region and inside the isolated Prefab editor. The
construction belongs to the MZ document; the editor has one generic application
action for all recipes.

**Status: experimental.** The selection/Prefab bridge and rectangular roof recipe
are implemented and have focused test and native-preview coverage. Further
construction work is deferred while the sandbox prototype prioritizes 2D
gameplay, rulesets and procedural tile artwork. This is not an automatic 2D-map
to 3D conversion system.

## Apply a recipe

1. Select objects or faces in a 3D region or Prefab source.
2. Run **Apply MZ Recipe** in the **Procedural** action group.
3. Choose a recipe and apply it. **Pitched roof** is the first bundled example.
4. Select the generated object and run **Edit MZ Recipe** to open its MZ rows.
   Select the **Pitched roof** Group tile to edit its published roof-rise and
   palette controls. Its **Fit To Geometry** tile controls overhang and alignment.
5. Return to the map. Input edits and shared recipe edits regenerate the output.

The selected input geometry remains in place. For the roof, select the complete
frame or its horizontal footprint face. Object selection supplies complete
meshes; explicit face selection supplies only those faces. A horizontal edge in
the input establishes the local X axis, allowing a rectangular frame to rotate
with its roof. The roof fits the input's local rectangular bounds and top height;
it does not infer an arbitrary roof from an irregular polygon.

Reapplying to an existing generated object updates its existing binding rather
than making another copy. Recipe edits are shared by bindings that reference the
same project asset. Duplicate/import a recipe asset to create an independent style.
Regeneration refits the output to its input. Put permanent geometric offsets in
the recipe; direct mesh edits or output-only movements can be replaced by regeneration.

## Author another recipe

Import an `.mzgeometry.json` model through the MZ Geometry workspace's **Asset →
Import model** command. Documents containing a `Geometry Input` named `selection`
appear in **Apply MZ Recipe**. All other construction and controls use ordinary
MZ geometry rows and groups.

Two generic operations support selection-driven recipes:

- **Geometry Input** receives a named host-supplied mesh. Its embedded preview
  document makes the recipe usable in standalone MZ previews as well.
- **Fit To Geometry** fits the current row's mesh to another mesh row. Each axis
  can be fitted independently, with source/target anchors and padding. A roof can
  fit X/Z while retaining its authored vertical thickness and ridge height.

The roof's slopes, rise deformation and palette controls live entirely in
`crates/materialz/examples/geometry/pitched-roof.mzgeometry.json`.
There is no roof-specific evaluator or roof-specific editor action.

## Props and regeneration

Applying a recipe inside a Prefab creates an **MZ-backed part**. Its source keeps
the recipe reference, portable source snapshot and input references. Normal
Geometry Objects are derived rendering/collision caches. Existing authored parts,
part hierarchy, pivots, support surfaces and behavior remain in the Prefab system.

Generated objects and triangle faces retain their identities for parameter edits
that preserve topology. Paint identities and explicit face material overrides
are preserved while the corresponding faces remain; topology edits can invalidate
surface references. Make Prefab Unique
remaps recipe input references along with object and face IDs.

Deleting an input or making a recipe invalid retains the last good generated mesh.
Saving, reopening and server startup synchronize recipe-backed geometry. Runtime
clients consume the derived meshes through the existing geometry/Prefab paths.
Selection recipes currently produce static geometry; animated rigs/effects use
the existing character/item appearance workflow. Input chains through other
recipe-generated geometry are not supported in this first version.

## Example and verification

```sh
cargo run -p eldiron-shared --example mz_selection_recipe -- /tmp/mz-selection
```

The example writes a project containing a timber frame with a fitted roof and a
recipe-backed Prefab, the portable roof recipe, and two native runtime captures.
The second capture widens the input frame and regenerates the same roof object.
Set `ELDIRON_RECIPE_NO_RENDER=1` to export without creating a GPU renderer.
Generated projects and captures are local artifacts, not repository fixtures.
Use `/tmp/mz-selection` or the ignored `generated/mz-selection` directory for them.
