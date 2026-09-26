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
| Appearance | Avatar, tile, 2D size, and visibility. Visibility is independent of life/activity state. |
| Collision | Radius and blocking. |
| Inventory | Slot count and starting wealth. |
| Player | Whether the character is available for player selection. It does not spawn the character. |
| Input Mapping | Starts the input branch. |
| Input Binding | One key and client command, with an optional custom command. |
| Attribute Override | One named, typed value: text, integer, number, boolean, or list. |
| Light | Color, strength, range, and height offset. |
| Ruleset Item | A ruleset item path. Its properties follow the current ruleset; connected configuration nodes supply explicit overrides. |

The Entity node reports configuration errors and indicates whether the graph is valid.

Race and class choices come from the active ruleset. Ruleset stats, abilities, permissions, progression, and starting equipment do not need duplicate configuration nodes. Add overrides only for exceptions.

Choice controls open compact, searchable, scrollable pickers. Inline text commits on Return. Edits use the global Undo/Redo system.

## Templates and instances

Template and instance configuration graphs are separate. An instance graph supplies only its overrides; an empty instance graph inherits the template.

At spawn, template settings are applied first, then instance overrides. Ruleset defaults and race/class progression fill values that were not explicitly supplied. Behavior **Startup** runs afterward, template first and instance second.

Configuration is resolved before spawning. Editing it does not reset the state of characters already running in the game. Behavior nodes handle runtime changes.

Invalid or unfinished graph edits remain editable. The last valid generated configuration is retained until the error is corrected. Duplicate writes to the same attribute or key within one graph are errors; an instance may override a template value.

## Player input

Each Input Binding node defines one key. Select a command such as `control.forward`, `intent.use`, `rules.basic_attack`, or `ui.actions`. Use **Custom Command** for other supported commands, including screen/game commands and custom intents.

The client resolves these bindings before behavior startup, preserving movement, targeting, local interface commands, and screen-button shortcut hints. Instance bindings override matching template keys and retain the others. Creator's **Game / Shortcuts** editor controls editor shortcuts separately.

See [Input Mapping](input_mapping) for command meanings, [Attributes](attributes) for runtime attribute names, and [Behavior Nodes](behavior_nodes) for event-driven logic.
