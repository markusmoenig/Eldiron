---
title: "Events"
sidebar_position: 6
---

This chapter lists all available **events** that can be received by characters and items in Eldiron.

Events are categorized as:

- **System Events** – sent by the engine to the `event()` handler
- **User Events** – sent to the `user_event()` handler (usually from player input)

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

### `intent`

- **Value**: `dict`  
  `{ intent (string), entity_id (int), item_id (int), distance (float)}`
- **Description**: Triggered when the player triggers an intent towards another entity or item. Either via a movement based keyboard shortcut or by clicking on the target entity or item.
  - When the target is an item, the event is send to the target item **and** to the originating player entity as the action may be handled by either of them depending on the context, for example a torch would lit itself when used, or a character may take an item.
  - When the target is another character, the event is send to both, the originating character and the target entity. For example on an `attack` intent the originating player may call [deal_damage](server_commands#deal_damage) to the given `entity_id`, or the target may want to respond when talked to.

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
- **Description**: Called when the entity or item is created.

---

### `take_damage`

- **Value**: `dict`
- **Description**: Triggered by the `deal_damage()` command.  
  The dictionary contains the damage payload sent by the attacker.

---

## User Events

### `key_down`

- **Value**: `string` (e.g. `"w"`, `"a"`, `"space"`)
- **Description**: Triggered when a key is pressed.

---

### `key_up`

- **Value**: `string` (e.g. `"w"`, `"a"`, `"space"`)
- **Description**: Triggered when a key is released.
