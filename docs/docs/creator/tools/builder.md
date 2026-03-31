---
title: "Builder Tool"
sidebar_position: 6
---

The **Builder Tool** (keyboard shortcut **`B`**) lets you place reusable **builder graph assets** such as tables, fences, torch stands, and other assemblies onto map geometry.

Builder graphs are structural assets. They define:

- geometry
- named **material slots** such as `TOP` or `LEGS`
- named **item slots** such as a tabletop surface
- a required host target:
  - `Sector`
  - `Linedef`
  - `Vertex`

## Picker Workflow

When the Builder tool is active, the lower picker area shows the **Builder Picker** instead of the **Tile Picker**.

From there you can:

- browse project builder assets
- create new assets with **New**
- select a builder asset
- **Apply Build** to the selected hosts
- **Clear** the builder graph from the selected hosts

Single-click selects a builder asset. Double-click, **Return**, or maximize opens the Builder graph editor.

## Host Targets

Each builder graph declares its output target. That target decides what the tool applies to:

- **Sector** builders place assemblies on sectors, for example tables or platforms
- **Linedef** builders place assemblies along edges, for example fences, rails, or balconies
- **Vertex** builders place assemblies on points, for example posts or torch stands

Selecting a builder asset switches the map-edit host mode to the matching target automatically.

## HUD Slots

Builder hosts use the same upper-right HUD area as other map tools, but the icons represent **builder slots** instead of direct tile assignment.

There are two slot types:

- **Material slots**: assign visual tile sources to named parts such as `TOP` or `LEGS`
- **Item slots**: attach other builder assets to named anchors or surfaces such as a tabletop

For example, a table builder can expose:

- `TOP` and `LEGS` as material slots
- `TOP` as an item surface slot for child props placed on the tabletop

## Applying Materials

Use the **Tile Picker** with a Builder host selected to assign tiles to the currently selected builder **material slot**.

This keeps the builder graph reusable:

- the graph defines the slot names
- the placed instance decides which tile fills each slot

The same table graph can therefore be reused with different materials without duplicating the graph itself.

## Attaching Child Builders

Builder **item slots** can host other builder assets.

This is used for workflows such as:

- placing an object on a tabletop
- attaching content to a shelf
- mounting a child prop onto a stand

Point attachments use **item anchors**. Surface attachments use **item surfaces** such as a tabletop or shelf top.

## Tips

- Use Builder for reusable placed structures, not for painting terrain or assigning floor tiles.
- Keep the graph generic and expose named slots instead of hardcoding materials.
- Start with sector builders such as tables, then expand to linedef and vertex hosts.

## Related Pages

- [Overview](/docs/creator/tools/overview)
- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Working With Tiles](/docs/building_maps/working_with_tiles)
