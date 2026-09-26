---
title: "Characters and Items"
sidebar_position: 1
---

Characters and items are reusable templates. Place an instance in a region to give it a position and, when needed, its own behavior.

- **Behavior Nodes** describe what happens in response to events.
- **Entity Nodes** define initial configuration, including identity, appearance and player input.
- **Rules** define shared gameplay such as actions, relationships, combat costs and cooldowns.

Start with [Behavior Nodes](behavior_nodes), then use the [node reference](node_reference) and [examples](behavior_examples).

## Templates and instances

A template graph and an instance graph are independent. Both receive events. **Startup runs on the template first, then on the instance**, before normal routines begin. An instance graph does not replace a matching template event. Use the template for shared behavior and the instance for placement-specific setup; avoid duplicating the same action in both.

Each graph has its own execution state and live updates. Both act on the same character or item, so actions that change its movement, target or attributes affect that shared object.

## Player characters

Connect the **Entity** node’s right output to a **Player** node’s left input and enable **Available as Player**. Configure keys with [Input Mapping](input_mapping) and screen buttons with [Player Input](player_input).

Ruleset actions such as `rules.basic_attack` execute through the ruleset directly. They do not require a Player attack graph. Player graphs handle reactions such as entering areas, death, or quest events. Player Camera configures the camera after the character is created; it does not create the player or bypass character selection.

See [Entity Nodes](entity_nodes) for configuration and [Attributes](attributes) for runtime attribute names.
