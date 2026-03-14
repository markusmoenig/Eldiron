---
title: "Server Commands"
sidebar_position: 7
---

## Commands

This chapter lists all available scripting **commands** for Eldiron, used by characters and items.

For a complete audio workflow (assets + buses + command examples), see [Audio](../audio).

---

## `add_item`

*This command can only be used with characters.*

Creates a new item of the given class and adds it to the character’s inventory.  
Returns the item ID or `-1` on failure.

```eldrin
add_item("Torch")
```

---

## `block_events`

*This command can be used with both characters and items.*

Blocks the listed events from being sent to the character or item for a number of in-game minutes.

```eldrin
block_events(minutes, "event1", "event2", ...)
```

You can also block specific `intent` events.

```eldrin
block_events(2, "intent: attack") // Block attack intents for 2 in-game minutes.

block_events(2, "intent") // Block all intents for 2 in-game minutes.
```

---

## `close_in`

*This command can only be used with characters.*

Makes an NPC character close in within a given radius on a target character with the given speed.

Once it is in range, sends [closed_in](events#closed_in) events to the NPC.

```eldrin
close_in(entity_id, 4.0, 1.0) // Close in within 4.0 radius on the entity_id with 1.0 speed.
```

---

## `clear_audio`

*This command can be used with both characters and items.*

Stops currently playing audio.

- `clear_audio("bus")` clears one audio bus/layer (for example `"music"` or `"sfx"`).
- `clear_audio()` clears all buses.

```eldrin
clear_audio("music") // Stop only music layer.
clear_audio() // Stop all currently playing audio.
```

---

## `clear_target`

*This command can be used with both characters and items.*

Clears the current target.

```eldrin
clear_target()
```

Returns `true`.

See also: [set_target](#set_target), [target](#target), [has_target](#has_target), [deal_damage](#deal_damage).

---

## `deal_damage`

*This command can be used with both characters and items.*

Deals damage to an entity or item. The server first applies the project-wide **Game -> Rules** combat formula to calculate the final incoming damage, then sends a [take_damage](events#take_damage) event to the receiver with that final amount and the attacker id.

```eldrin
deal_damage(id, random(2, 5))
deal_damage(random(2, 5)) // Uses current target.
deal_damage(id, random(2, 5), "fire")
deal_damage(random(2, 5), "physical")
```

If called with one argument, `deal_damage(amount)` uses the current target (see [set_target](#set_target)).
If called with two arguments and the second argument is a string, `deal_damage(amount, kind)` uses the current target and sets the damage kind.
If no kind is supplied, `deal_damage` defaults to `physical`.

The damage kind is also used by:

- `Game / Rules` combat formula overrides under `combat.kinds.<kind>`
- automatic combat messages and combat audio in `Game / Rules`
- the `take_damage` event payload as `damage_kind`

If the target character has [autodamage](attributes#autodamage) set to `true`, damage is applied directly by the server and no [take_damage](events#take_damage) event is sent.

See also: [set_target](#set_target), [target](#target), [has_target](#has_target), [clear_target](#clear_target).

:::note
Characters and items can deal damage. But only characters can receive damage and actually die.
:::
---

## `debug`

*This command can be used with both characters and items.*

Sends a debug message to the log.

```eldrin
debug(arg1, arg2, ...)
```

---

## `drop`

*This command can only be used with characters.*

Drop a specific item from the character's inventory. The item is identified by its id and that value is mostly provided by `intent` messages acting on an item.

```eldrin
drop(value)
```

---

## `drop_items`

*This command can only be used with characters.*

Drops items from the character's inventory.  
If a `filter_string` is provided, only matching items are dropped, otherwise all items are dropped.

```eldrin
drop_items(filter_string)
```

---

## `entities_in_radius`

*This command can be used with both characters and items.*

Returns a list of nearby entity IDs within radius.

```eldrin
entities_in_radius()
```

---

## `equip`

*This command can only be used with characters.*

Equips an item from the character’s inventory to its slot.  
Returns `true` on success or `false` on failure.

```eldrin
equip(item_id)
```

---

## `get_attr`

*This command can be used with both characters and items.*

Gets an attribute of the current character or item.

```eldrin
get_attr("key")
```

---

## `get_attr_of`

*This command can be used with both characters and items.*

Gets an attribute from a specific entity or item.

```eldrin
get_attr_of(id, "key")
```

---

## `get_sector_name`

*This command can be used with both characters and items.*

Returns the name of the sector the character or item is currently in.

```eldrin
get_sector_name()
```

---

## `goto`

*This command can only be used with characters.*

The character will walk to the named destination sector with the given speed. It will utilize path-finding to avoid obstacles.

```eldrin
goto("Garden", 1.0)
```

---

## `has_target`

*This command can be used with both characters and items.*

Checks whether a valid current target is set.

```eldrin
if has_target() {
    deal_damage(3)
}
```

See also: [set_target](#set_target), [target](#target), [clear_target](#clear_target), [deal_damage](#deal_damage).

---

## `id`

*This command can only be used with characters.*

Returns the **id** of the current entity.

Valid entity IDs are always `> 0`.  
`0` is reserved as a sentinel value meaning "no entity / no target".

```eldrin
id()
```

---

## `inventory_items`

*This command can only be used with characters.*

Returns a list of item IDs in the character’s inventory.  
If a `filter_string` is provided, only matching items are returned.

```eldrin
inventory_items(filter_string)
```

---

## `inventory_items_of`

*This command can be used with both characters and items.*

Returns a list of item IDs in another entity’s inventory.  
If a `filter_string` is provided, only matching items are returned.

```eldrin
inventory_items_of(entity_id, filter_string)
```

---

## `message`

*This command can be used with both characters and items.*

Sends a message to a given character.  
An optional category can be used for UI coloring.

```eldrin
message(entity_id, "message", "optional_category")
```

---

## `say`

*This command can be used with both characters and items.*

Displays a speech bubble above the sender in the game world (2D and 3D).  
The optional category is used for color lookup via the game `[say]` configuration.

```eldrin
say("Hello there")
say("Watch out!", "warning")
```

Parameters:

- `message` (required): Text to show above the sender.
- `category` (optional): Color category key from `[say]` config (for example `warning`, `npc`, `quest`).

---

## `notify_in`

*This command can be used with both characters and items.*

Schedules an event to be sent after a given number of in-game minutes.

```eldrin
notify_in(minutes, "event_name")
```

---

## `offer_inventory`

*This command can only be used with characters.*

Offers the inventory to a given entity with an optional filter string. Mostly used by vendor NPCs who would offer their inventory when spoken to.

```eldrin
offer_inventory(entity, "") // Offer all inventory items to the given entity.
offer_inventory(entity, "Torch") // Offer only items named Torch.
```

---

## `patrol`

*This command can only be used with characters.*

Starts patrol behavior using the character route configuration from [route](attributes#route).

```eldrin
patrol()
patrol(wait_minutes)
patrol(wait_minutes, speed)
```

- `wait_minutes` default: `1.0`
- `speed` default: `1.0`

`route_mode` is read from [route_mode](attributes#route_mode) and defaults to `"loop"`.

See also: [route](attributes#route), [route_mode](attributes#route_mode), [goto](#goto), [random_walk_in_sector](#random_walk_in_sector).

---

## `play_audio`

*This command can be used with both characters and items.*

Plays an audio asset by name.

```eldrin
play_audio("door_open")
play_audio("battle_theme", "music", 0.8, true)
```

Parameters:

- `name` (required): Audio asset name.
- `bus` (optional): Audio bus/layer, default is `"sfx"`.
- `gain` (optional): Volume multiplier in range `0.0..4.0`, default is `1.0`.
- `looping` (optional): `true` loops the clip, `false` plays once (default).

Common buses are `music`, `sfx`, `ui`, `ambience`, and `voice`, but you can use custom bus names.

---

## `random_walk`

*This command can only be used with characters.*

Moves the character in a random direction for the given distance and speed.  
Sleeps after each move for a random time up to `max_sleep` in in-game minutes.

```eldrin
random_walk(distance, speed, max_sleep)
```

---

## `random_walk_in_sector`

*This command can only be used with characters.*

Similar to `random_walk`, but restricted to the current sector.

```eldrin
random_walk_in_sector(distance, speed, max_sleep)
```

---

## `set_attr`

*This command can be used with both characters and items.*

Sets an attribute on the current character or item.

```eldrin
set_attr("key", value)
```

---

## `set_audio_bus_volume`

*This command can be used with both characters and items.*

Sets the volume for one audio bus/layer.

```eldrin
set_audio_bus_volume("music", 0.5)
set_audio_bus_volume("sfx", 1.0)
```

`volume` is clamped to `0.0..4.0`.

---

## `set_emit_light`

*This command can be used with both characters and items.*

Enables / disables light emittance for entities and items. Items should do this as part of their state, see the [active event](events#active).

Light parameters need to be set up with the [light attributes](attributes#emitting-light).

```eldrin
fn event(event, value) {
    if event == "active" {
        set_emit_light(value);
    }
}
```

---

## `set_player_camera`

*This command can only be used with player characters.*

Defines how incoming player [actions](input_mapping#action-types) are translated into movement and camera behavior.  
This command **does not change the visual rendering camera**. This is done by the game widgets [camera setting](/docs/screens/widgets/#camera-section)
It only changes how player input is interpreted.

Current valid values are:

* `2d` — **forward**, **backward**, **left**, **right** move the character directly in the given world directions. This is the default.
* `iso` — Same movement behavior as `2d`, typically used with an isometric view.
* `firstp` — **forward** moves the player in the direction they are facing, **backward** moves opposite. **left** and **right** rotate the view instead of strafing.

```eldrin
set_player_camera("firstp");
```

---

## `set_proximity_tracking`

*This command can be used with both characters and items.*

Enables or disables proximity tracking for characters. If enabled, [proximity_warning](events#proximity_warning) events will be send for all nearby entities within the given radius.

Useful for NPCs (or even items) to interact with other characters (attack, talk, heal, etc.).

```eldrin
set_proximity_tracking(true, 4.0)
```

---

## `set_target`

*This command can be used with both characters and items.*

Sets the current target for the current character or item.

```eldrin
set_target(entity_id)
```

Returns `true` if the target exists and was set, otherwise `false`.

See also: [target](#target), [has_target](#has_target), [clear_target](#clear_target), [deal_damage](#deal_damage).

---

## `take`

*This command can only be used with characters.*

Takes an item from the region and adds it to the character’s inventory.  
Returns `True` on success or `False` if the inventory is full.

```eldrin
take(item_id)
```

---

## `target`

*This command can be used with both characters and items.*

Returns the current target entity id.

```eldrin
let id = target()
```

Returns `0` when no target is set.

See also: [set_target](#set_target), [has_target](#has_target), [clear_target](#clear_target), [deal_damage](#deal_damage).

---

## `teleport`

*This command can only be used with characters.*

Teleports the character to a named sector, the second parameter is optional and names the region to teleport to. If only the sector name is given `teleport` will search for the sector in the current region.

```eldrin
teleport("Entrance", "Deadly Dungeon")
```

---
