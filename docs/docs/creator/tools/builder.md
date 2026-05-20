---
title: "Builder Tool"
sidebar_position: 7
---

The **Builder Tool** (keyboard shortcut **`B`**) bakes reusable [Builder Graph](/docs/builder_graph) assets into editable 3D Geometry Objects.

Use this page for the editor workflow. Use the [Builder Graph](/docs/builder_graph) chapter for the `.buildergraph` language, hosts, details, cuts, materials, examples, and CLI preview workflow.

## What It Does

The Builder Tool lets you:

- browse project builder assets
- create new builder assets
- tune exposed builder parameters in the lower dock sidebar
- open the Builder script editor
- click in the 3D scene to place and bake the selected builder asset immediately

Builder assets can be props, structures, wall details, and reusable assemblies. Baking a build creates ordinary editable geometry instead of leaving a procedural generator attached to the scene.

## Picker Workflow

When the Builder Tool is active, the lower picker area shows the **Builder Picker** instead of the Tile Picker.

From the picker you can:

- select a builder asset
- edit exposed `param` values in the TOML sidebar
- create a new asset with **New**
- open an asset by double-clicking it, pressing **Return**, or maximizing the editor
- click in the 3D scene to bake the selected asset at the clicked point
- use the normal geometry tools after baking to move, texture, cut, delete, or reshape the generated parts

Single-click selects a builder asset. The currently selected asset is used when you click in the 3D scene.

## Host Targets

Each Builder Graph declares one host target. The Builder Tool uses the clicked surface position and orientation as the placement host. Wall clicks face the asset out from the wall; floor clicks place it upright on the floor.

- `host = sector;`
  - bakes as a floor or platform-oriented asset
  - useful for platforms, floor props, surface relief, recesses, and freestanding sector details

- `host = linedef;`
  - bakes as a wall or span-oriented asset
  - useful for walls, rails, fences, pilasters, and long span-based details

- `host = vertex;`
  - bakes as a point-mounted asset
  - useful for point-mounted props such as wall torches, lanterns, posts, and campfires

Direct 3D clicks do not require a preselected sector, linedef, or vertex.

## Builder Script Editor

Opening a builder asset shows the Builder script editor.

The lower Builder dock contains:

- the Builder Picker
- a TOML parameter sidebar for exposed `param` values

The maximized Builder script editor contains:

- a text editor for the `.buildergraph` script
- a live 3D preview
- syntax highlighting for Builder keywords and identifiers

The editor is intended for fast iteration:

- tune exposed template parameters
- change script dimensions when needed
- check host orientation
- inspect material slots
- inspect surface details
- verify wall side and growth direction

The lower-dock TOML sidebar is generated from `param` declarations in the selected script:

```txt
param radius = 0.14;
param spacing = 2.0;
param placement = attached;
param broken_chance = 0.0;
param seed = 1.0;
```

Editing the sidebar updates those `param ... = ...;` lines. Use this for tuning a selected template. Choose a different template when the structure changes, such as switching from masonry relief to a cut-out column opening.

The Builder dock has a **Treasury** tab for published Builder Graph packages. It downloads the indexed package list only when the Treasury tab is opened. Starter templates such as tables, wall lights, campfires, masonry, column structures, and farmhouse shells live there. Treasury items can be baked directly, or installed into the project with **Install** if the script should become a permanent editable project asset.

For the script language itself, see [Builder Graph](/docs/builder_graph).

## HUD Slots

Builder Graphs can expose material and item slots. Baked Geometry Objects currently preserve the source graph and material slot name as object metadata, while the geometry itself is immediately editable through the normal 3D tools.

The Builder Graph language has two slot types:

- **Material slots**
  - name parts such as `TOP`, `LEGS`, `COLUMN`, or `TRIM`

- **Item slots**
  - name anchors or surfaces for future child-asset workflows

For example, a table builder can expose:

- `TOP` and `LEGS` as material slots
- `TOP` as an item surface slot for child props placed on the tabletop

## After Baking

After baking, the generated parts are selected as Geometry Objects. Use the normal 3D Object, Face, Edge, and Vertex tools to edit them.

The current bake path supports Builder Graph box and cylinder primitives. Surface-only details, cuts, child item slots, and procedural organic details remain Builder Graph concepts, but they are not baked into editable Geometry Objects yet.

## Testing A Builder

Typical workflow:

1. Create or select a Builder Graph asset.
2. Edit the `.buildergraph` script and watch the preview.
3. Activate the Builder Tool.
4. Click in the 3D scene where the asset should be placed.
5. Edit, texture, duplicate, or reshape the selected Geometry Objects.
6. Reapply after changing the script if you want a fresh baked version.

## Presets And Examples

The project includes several starting points:

- **Table**
- **Wall Torch**
- **Wall Lantern**
- **Campfire**
- **Surface Freestanding Columns**
- **Wall Columns**
- **Wall Masonry**
- **Wall Columns Masonry**
- **Wall Column Opening**

See [Builder Graph](/docs/builder_graph) for script examples and language details.

## Tips

- Use the Builder Tool for reusable placed structures and editable baked props.
- Prefer focused templates over one giant script with many unrelated controls.
- Use the parameter sidebar for tuning values such as radius, spacing, damage chance, and seed.
- Keep scripts generic and expose named slots instead of hardcoding materials.
- Use `tile_alias` in scripts to get useful previews before assigning exact material tiles.
- Use `host = sector` for floor/sector details.
- Use `host = linedef` for wall-span details.
- Use `host = vertex` for point-mounted objects.

## Related Pages

- [Builder Graph](/docs/builder_graph)
- [Tools Overview](/docs/creator/tools/overview)
- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Working With Tiles](/docs/building_maps/working_with_tiles)
