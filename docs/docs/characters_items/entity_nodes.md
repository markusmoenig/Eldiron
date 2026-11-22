---
title: "Entity Nodes"
sidebar_position: 3
---

Open **Entity Nodes** under a character or item in the project tree. This graph defines its initial configuration. **Behavior Nodes** define its reactions to events.

The left panel lists branches. The **Entity** branch starts with an Entity node containing race, class and initial level. Connect its right output to configuration nodes in a left-to-right strip, for example **Entity → Appearance → Inventory → Attribute Override**. Only nodes reachable from the branch root contribute settings. Disconnected nodes have no effect.

The separate **Input Mapping** branch starts with an Input Mapping node. Connect individual **Input Binding** nodes in a strip, one node per key. Open Entity Nodes and Behavior Nodes through their respective project-tree entries.

## Configuration nodes

| Node | Settings |
| --- | --- |
| Entity | Ruleset race, class, and initial level. Empty race/class and level 0 inherit defaults. |
| Animal | Species from the active ruleset. Supplies its race and class, including the body, stats and remains binding. Leave Entity race/class empty. |
| Set Tile | Assign a sprite tile using a scrollable grid of previews. Type to search aliases; no UUID entry is needed. Legacy module tiles are excluded. |
| Appearance | Avatar, 2D size, visibility, body build and height, face style, hair style and hair color. Face and hair choices depend on the resolved model. |
| Equipment | Starting worn/held items. Drag project item headers onto the node; item names are shown and stable references are saved. |
| Collision | Radius and blocking. |
| Item State | Initial On/Off state, Openable, Locked and Required Key. MZ State Switch outputs and footprints follow the same state in iso and 3D. |
| Inventory | Slot count and starting wealth. |
| Player | Whether the character is available for player selection. It does not spawn the character. |
| Input Mapping | Starts the input branch. |
| Input Binding | One key and client command, with an optional custom command. |
| Attribute Override | One named, typed value: text, integer, number, boolean, or list. |
| Light | Color, strength, range, and height offset. |
| Ruleset Item | A ruleset item path. Its properties follow the current ruleset; connected configuration nodes supply explicit overrides. |

The Entity node reports configuration errors and indicates whether the graph is valid.

For an interactive MZ item and keyed-door walkthrough, see [Doors and MZ Item States](doors).

Race and class choices come from the active ruleset. Ruleset stats, abilities, permissions, progression, and starting equipment do not need duplicate configuration nodes. Add overrides only for exceptions. Character defaults come from `attributes.defaults` and the selected race and class; custom item defaults come from `item_defaults`. Matching defaults are omitted during import.

Entity stays limited to race, class and level. Other authored settings use separate configuration nodes.

For an animal, use **+ → Add Animal**. It opens **Entity Nodes** with
**Entity → Animal** already connected. Choose its species on Animal, then drag the character
template onto the map as usual. Animals have the same **Behavior Nodes** as
NPCs: add routines, Random Walk, event reactions or other behavior there.
Selecting a species does not create or replace a behavior graph.

Choice controls open compact, searchable, scrollable pickers. Inline text commits on Return. Edits use the global Undo/Redo system.

## Equipment and visual appearance

Connect **Equipment** to Entity to override the class outfit. A drop replaces an
item in the same slot; invalid class or two-handed combinations report an error.
Omit Equipment to inherit the class loadout. An empty list equips nothing.
Names and ruleset IDs may also be entered manually, but are not required for
drag-and-drop authoring.

The character's **Appearance** tree entry previews the resolved outfit in 2D
and 3D. MZ bodies, clothing materials and attached models follow the ruleset
items you equip. See [Appearance](appearance) and [Rigging](rigging) before
editing the underlying artwork or skeleton.

In [Wayfarer’s Rest](../creator/wayfarers_rest), open **Rowan the Innkeeper →
Entity Nodes**. His **Appearance** node uses the shared Human model with the
Stout build, height 1.73 m, Bearded face, Receding hair and hair color
`#9e9a91`. A height of 0 inherits the model’s default. His **Equipment**
node selects the supplied Innkeeper Apron, Wool Trousers and Leather Shoes.
Compare Bram the Hunter, whose Hunting Cloak occupies the **Back** slot.

## Templates and instances

Template and instance configuration graphs are separate. An instance graph supplies only its overrides; an empty instance graph inherits the template.

At spawn, template settings are applied first, then instance overrides. Ruleset defaults and race/class progression fill values that were not explicitly supplied. Behavior **Startup** runs afterward, template first and instance second.

Creator’s top toolbar has a **Skip Character Setup** toggle (fast-forward icon beside the input-routing button). Enable it before Play to spawn the player using Entity Node defaults and open the configured play screen directly. The toggle remains selected across editor restarts of play; standalone games keep their character setup screen.

Configuration is resolved before spawning. Editing it does not reset the state of characters already running in the game. Behavior nodes handle runtime changes.

Invalid or unfinished graph edits remain editable. The last valid generated configuration is retained until the error is corrected. Duplicate writes to the same attribute or key within one graph are errors; an instance may override a template value.

## Player input

Each Input Binding node defines one key. Select a command such as `control.forward`, `intent.use`, `rules.basic_attack`, or `ui.actions`. Use **Custom Command** for other supported commands, including screen/game commands and custom intents.

The client resolves these bindings before behavior startup, preserving movement, targeting, local interface commands, and screen-button shortcut hints. Instance bindings override matching template keys and retain the others. Creator's **Game / Shortcuts** editor controls editor shortcuts separately.

See [Input Mapping](input_mapping) for command meanings, [Attributes](attributes) for runtime attribute names, and [Behavior Nodes](behavior_nodes) for event-driven logic.
