---
title: "Events"
sidebar_position: 9
---

This chapter lists all available **events** that can be received by characters and items in Eldiron.

Events are categorized as:

- **System Events** – sent by the engine to the `event()` handler

---

## System Events

### `active`

*Item-only event.*

- **Value**: active state *(bool)*
- **Description**: Called when the state of the item has changed and directly after item creation. This event allows the item to sync its visuals with the current state, for example a torch may use [set_emit_light](server_commands/#set_emit_light) to adjust it's light emission.

---

### `arrived`

*Character-only event.*

- **Value**: destination sector name *(string)*
- **Description**: Send by [goto](server_commands#goto) when the character arrives at the destination.

---

### `bumped_into_entity`

- **Value**: `entity_id` *(int)*
- **Description**: Triggered when this entity bumps into another entity.

---

### `bumped_into_item`

- **Value**: `item_id` *(int)*
- **Description**: Triggered when this entity bumps into an item.

---

### `bumped_by_entity`

- **Value**: `entity_id` *(int)*
- **Description**: Triggered when another entity collides with this entity or item.

---

### `closed_in`

- **Value**: `entity_id` *(int)*
- **Description**: Send when an NPC closed in within the radius of the target entity. Send by the [close_in](server_commands#close_in) command.

---

### `dead`

- **Value**: `entity_id` *(int)*
- **Description**: Send when another entity kills this character. The *entity_id* of the killer is passed in the value. This is useful for sending messages and taking the next steps. A player character could for example [teleport](server_commands#teleport) to a graveyard or healer.

---

### `entered`

- **Value**: `sector_name` *(string)*
- **Description**: Triggered when the character has entered a named sector. Useful for traps or teleports.

---

### `engagement_over`

- **Value**: reason *(string)*
- **Description**: Triggered when an engine-owned [follow_attack](server_commands#follow_attack) engagement ends.

Current reasons:

- `lost`
- `too_far`

This currently happens when the target:

- no longer exists
- is no longer a valid living/visible target
- moves beyond the current chase leash

The chase leash is currently:

- `max(proximity_tracking_distance, 1.5) + 1.0`
- fallback if no proximity tracking was set: `max(5.0, 1.5) + 1.0 = 6.0`

So in practice, if you call `set_proximity_tracking(true, 4)`, `follow_attack` will currently break when the target gets beyond `5.0`.

This event is useful for clearing target state and returning an NPC to idle behavior such as `random_walk_in_sector(...)` or `goto(...)`.

---

### `intent`

- **Value**: `dict`  
  `{ intent (string), entity_id (int), item_id (int), distance (float)}`
- **Description**: Triggered when the player triggers an intent towards another entity or item. Either via a movement based keyboard shortcut or by clicking on the target entity or item.
  - When the target is an item, the event is send to the target item **and** to the originating player entity as the action may be handled by either of them depending on the context, for example a torch would lit itself when used, or a character may take an item.
  - When the target is another character, the event is send to both, the originating character and the target entity. For example on an `attack` intent the originating player may call [deal_damage](server_commands#deal_damage) to the given `entity_id`, or the target may want to respond when talked to.

---

### `level_up`

Sent to a character after `gain_xp(...)` causes it to reach a new level.

`value` is the new level number.

---

### `kill`

- **Value**: `entity_id` *(int)*
- **Description**: Send when this entity kills another character. The *entity_id* of the target is passed in the value. This is useful for sending messages and for NPCs to reset what they are doing.

---

### `left`

- **Value**: `sector_name` *(string)*
- **Description**: Triggered when the character has left a named sector.

---

### `proximity_warning`

- **Value**: `entity_ids` *(array)*
- **Description**: Called when proximity tracking was enabled via [set_proximity_tracking](server_commands#set_proximity_tracking) and other entities are in radius. Useful for NPCs to interact with other characters (attack, talk, heal, etc.).

---

### `startup`

- **Value**: *(None)*
- **Description**: Called when the entity or item is created. This is a common place to start a background NPC sequence via [run_sequence](server_commands#run_sequence).

---

### `time`

- **Value**: `hour` *(int, 0..23)*
- **Description**: Triggered for all characters and items whenever in-game time reaches a full hour (`MM == 00`). The value contains the current 24-hour hour value.

This event is the current scheduling hook for NPC routines. A common pattern is:

- `08:00` -> `run_sequence("go_to_work")`
- `18:00` -> `run_sequence("go_home")`

See [NPC Sequences](npc_sequences) for the bigger event + sequence model.

---

### `take_damage`

- **Value**: `amount` *(int)*
- **Description**: Triggered by the `deal_damage()` command after the server has applied the project-wide damage formula.  
  `amount` is the final incoming damage, `from_id` contains the attacker id, `damage_kind` contains the kind such as `physical`, `spell`, or `fire`, and `source_item_id` contains the weapon or spell item when available.
  The server applies this final damage automatically after the event returns.
  If the target has [autodamage](attributes#autodamage) enabled, this event is not triggered.

---

Player key input is configured via [Input Mapping](input_mapping), not via script events.
