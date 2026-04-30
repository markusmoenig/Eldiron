---
title: "Builder Tool"
sidebar_position: 6
---

The **Builder Tool** (keyboard shortcut **`B`**) applies reusable [Builder Graph](/docs/builder_graph) assets to map geometry.

Use this page for the editor workflow. Use the [Builder Graph](/docs/builder_graph) chapter for the `.buildergraph` language, hosts, details, cuts, materials, examples, and CLI preview workflow.

## What It Does

The Builder Tool lets you:

- browse project builder assets
- create new builder assets
- open the Builder script editor
- apply the selected builder asset to selected map hosts
- clear builder data from selected hosts
- assign material and item slots through the HUD

Builder assets can be props, structures, wall details, surface details, or procedural assemblies.

## Picker Workflow

When the Builder Tool is active, the lower picker area shows the **Builder Picker** instead of the Tile Picker.

From the picker you can:

- select a builder asset
- create a new asset with **New**
- open an asset by double-clicking it, pressing **Return**, or maximizing the editor
- use **Apply Build** to apply the asset to selected hosts
- use **Clear** to remove builder data from selected hosts

Single-click selects a builder asset. The currently selected asset is used by **Apply Build**.

## Host Targets

Each Builder Graph declares one host target. The Builder Tool uses that target to decide which map selection type it applies to:

- `host = sector;`
  - applies to selected sectors
  - useful for platforms, floor props, surface relief, recesses, and freestanding sector details

- `host = linedef;`
  - applies to selected linedefs
  - useful for walls, rails, fences, pilasters, and long span-based details

- `host = vertex;`
  - applies to selected vertices
  - useful for point-mounted props such as wall torches, lanterns, posts, and campfires

Selecting a builder asset switches the map edit host mode to the matching target automatically.

## Builder Script Editor

Opening a builder asset shows the Builder script editor.

The editor contains:

- a text editor for the `.buildergraph` script
- a live 3D preview
- syntax highlighting for Builder keywords and identifiers

The editor is intended for fast iteration:

- change script dimensions
- check host orientation
- inspect material slots
- inspect surface details
- verify wall side and growth direction

For the script language itself, see [Builder Graph](/docs/builder_graph).

## HUD Slots

Builder hosts use the same upper-right HUD area as other map tools, but the icons represent builder slots.

There are two slot types:

- **Material slots**
  - assign visual tile sources to named parts such as `TOP`, `LEGS`, `COLUMN`, or `TRIM`

- **Item slots**
  - attach other builder assets to named anchors or surfaces

For example, a table builder can expose:

- `TOP` and `LEGS` as material slots
- `TOP` as an item surface slot for child props placed on the tabletop

## Applying Materials

Use the Tile Picker with a Builder host selected to assign tiles to the currently selected builder material slot.

This keeps the graph reusable:

- the graph defines slot names
- the placed instance decides which tile fills each slot

The same graph can be reused with different material assignments.

## Applying Child Builders

Builder item slots can host other builder assets.

This supports workflows such as:

- placing an object on a tabletop
- attaching content to a shelf
- mounting a child prop onto a stand
- adding effects or secondary props to an anchor

Point attachments use item anchors. Surface attachments use item surfaces.

## Testing A Builder

Typical workflow:

1. Create or select a Builder Graph asset.
2. Edit the `.buildergraph` script and watch the preview.
3. Select matching map hosts:
   - sectors for `host = sector`
   - linedefs for `host = linedef`
   - vertices for `host = vertex`
4. Press **Apply Build**.
5. Assign materials through the HUD if needed.
6. Rebuild or reapply after changing the script.

## Presets And Examples

The project includes several starting points:

- **Table**
- **Wall Torch**
- **Wall Lantern**
- **Campfire**
- **Surface Freestanding Columns**
- **Wall Columns**

See [Builder Graph](/docs/builder_graph) for script examples and language details.

## Tips

- Use the Builder Tool for reusable placed structures and procedural details.
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
