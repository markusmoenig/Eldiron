---
title: "Server Commands"
sidebar_position: 10
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

## `attack`

*This command can only be used with characters.*

Attacks the current target using the project's progression and combat rules.

```eldrin
attack()
```

Behavior:

- uses the current target (see [set_target](#set_target))
- starts from `progression.damage`
- if `progression.damage` is not configured, falls back to the attacker's `DMG` attribute, then to `1`
- uses the current weapon's `damage_kind` when available, otherwise `physical`
- then applies `outgoing_damage` and `incoming_damage` from [Rules](../rules)

Use `attack()` for the normal combat path. Use [deal_damage](#deal_damage) when you need to send an explicit amount or kind.

---

## `close_in`

*This command can only be used with characters.*

Makes an NPC character close in within a given radius on a target character with the given speed.

Once it is in range, sends [closed_in](events#closed_in) events to the NPC.

```eldrin
close_in(entity_id, 4.0, 1.0) // Close in within 4.0 radius on the entity_id with 1.0 speed.
```

---

## `follow_attack`

*This command can only be used with characters.*

Starts an engine-owned melee engagement against a target character.

```eldrin
follow_attack(entity_id, 1.0)
```

Behavior:

- chases the target using the given movement speed
- attacks using the normal combat rules, weapon damage kind, and progression damage setup
- in non-realtime 2D modes, stays grid-aligned and does not use custom script-side `close_in` / `notify_in` attack loops
- if `speed < 1.0` in non-realtime 2D, it skips some turns and then moves a full tile instead of drifting off-grid
- keeps the engagement active only while the target still exists, is visible/alive, and stays close enough
- the chase leash currently comes from the attacker's [set_proximity_tracking](#set_proximity_tracking) radius
- exact formula: `max(proximity_tracking_distance, 1.5) + 1.0`
- if no proximity tracking radius is set, the fallback is `5.0`, so the default leash becomes `6.0`
- emits [engagement_over](events#engagement_over) when the target is gone, no longer valid, or moves beyond that leash distance

Use this for normal melee chase behavior. Prefer it over building custom combat loops from `proximity_warning`, `close_in`, `closed_in`, and `notify_in`.

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

## `set_tile`

*This command can be used with both characters and items.*

Changes the current visual tile source of the executing character or item.

Accepted forms:

- tile UUID
- tile alias
- palette index

```eldrin
set_tile("03160f57-90e3-4455-a16e-f0b8edfaa415")
set_tile("player_tile")
set_tile("2")
```

This updates the runtime `source` attribute directly.

---

## `deal_damage`

*This command can be used with both characters and items.*

Deals damage to an entity or item. The server first applies the project-wide [Rules](../rules) combat pipeline, then sends a [take_damage](events#take_damage) event to the receiver with the final amount and the attacker id.

```eldrin
deal_damage(id, random(2, 5))
deal_damage(random(2, 5)) // Uses current target.
deal_damage(id, random(2, 5), "fire")
deal_damage(random(2, 5), "physical")
```

If called with one argument, `deal_damage(amount)` uses the current target (see [set_target](#set_target)).
If called with two arguments and the second argument is a string, `deal_damage(amount, kind)` uses the current target and sets the damage kind.
If no kind is supplied, `deal_damage` defaults to `physical`.

`deal_damage(...)` is the lower-level escape hatch. For normal attacks against the current target, prefer [attack](#attack).

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

## `dialog`

*This command can only be used with characters.*

Opens a TOML-authored dialog node for a target entity. Dialogs use the same Messages widget and choice session pipeline as `multiple_choice()` and `offer_inventory()`.

```eldrin
dialog(entity, "greeting")
```

Dialog content is authored in the character attributes TOML:

```toml
[dialog]
start = "greeting"

[dialog.nodes.greeting]
text = "{dialog.guard.greeting}"
choices = [
  { label = "{dialog.guard.work}", next = "work", unless = "target.rat_quest_done" },
  { label = "{dialog.guard.thanks}", next = "thanks", if = "target.rat_quest_done" },
  { label = "{dialog.goodbye}", end = true },
]

[dialog.nodes.work]
text = "{dialog.guard.work_text}"
choices = [
  { label = "{dialog.accept}", event = "accept_rat_quest", end = true },
  { label = "{dialog.back}", next = "greeting" },
]
```

- `entity`: Target entity that should receive the dialog.
- `node`: Dialog node to open. Use the configured `start` node by passing `""`.
- `text`: Message text shown before the node choices. It uses the normal [message localization and substitution system](../localization).
- `choices`: Inline TOML array of choice tables.
- `label`: Choice text. It also supports localization keys.
- `next`: Opens another dialog node automatically after selection.
- `event`: Sends an event to the offering character after selection.
- `end`: Ends the dialog after selection.
- `if` / `unless`: Optional boolean conditions. Supported prefixes are `self.`, `target.` / `player.`, and `region.` / `world.`.

The selected choice event value contains the target entity id in `value.x`, the zero-based option index in `value.y`, and the selected label in `value.text`.

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

## `gain_xp`

*This command can only be used with characters.*

Adds experience to the current character.

```eldrin
gain_xp(25)
```

Behavior:

- adds to the attribute named by `game.experience`
- compares the new total against `progression.level.xp_for_level`
- raises the attribute named by `game.level` when thresholds are crossed
- sends a [level_up](events#level_up) event with the new level
- also uses the rules-driven progression message system when configured

The experience value is treated as a running total, not as "xp since last level".

Normal kill XP can also be awarded automatically through `progression.xp.kill`, so `gain_xp()` is mainly needed for quests, scripted rewards, trainers, and other custom sources.

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

## `run_sequence`

*This command can only be used with characters.*

Starts the named background sequence from step `0`.

Sequences are defined in the character **Attributes** editor under `behavior.sequences`.

```eldrin
run_sequence("go_to_work")
```

See [NPC Sequences](npc_sequences).

---

## `pause_sequence`

*This command can only be used with characters.*

Pauses the currently active background sequence.

```eldrin
pause_sequence()
```

This is usually called from a reactive event such as `talk`, `use`, or a custom interaction flow.

See [NPC Sequences](npc_sequences).

---

## `resume_sequence`

*This command can only be used with characters.*

Resumes the previously paused background sequence.

```eldrin
resume_sequence()
```

See [NPC Sequences](npc_sequences).

---

## `cancel_sequence`

*This command can only be used with characters.*

Cancels the active sequence and clears any paused sequence.

```eldrin
cancel_sequence()
```

Use this when the NPC should abandon its current background plan completely.

See [NPC Sequences](npc_sequences).

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
The one-parameter form is valid. The optional second parameter is a category
used for color lookup via the active game widget `[say]` configuration.

```eldrin
say("Hello there")              # Uses the default [say] color.
say("Watch out!", "warning")    # Uses the [say].warning color.
```

Parameters:

- `message` (required): Text to show above the sender.
- `category` (optional): Color category key from the game widget `[say]` config (for example `warning`, `npc`, `quest`).

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

The sale session stays valid only while the buyer remains close enough and within the vendor timeout window.

- Timeout comes from the seller's [timeout](attributes#timeout) attribute.
- Maximum distance currently follows the seller's top-level `[intent_distance]` table in the **Attributes** editor and falls back to `2.0`.
- If the buyer moves too far away or the timeout expires, the session ends and the seller receives a `goodbye` event.

---

## `multiple_choice`

*This command can only be used with characters.*

Offers script-defined choices to a target entity using the same Messages widget choice UI as `offer_inventory()`.

```eldrin
multiple_choice(entity, "Open the door?", "door_choices")
```

The third argument is the name of a character attribute containing the options.

```toml
[attributes]
door_choices = ["Yes", "No", "Maybe"]
```

Choice labels use the normal [message localization and substitution system](../localization), so they can also be authored as localization keys:

```toml
[attributes]
door_choices = ["{dialog.yes}", "{dialog.no}", "{dialog.maybe}"]
```

- `entity`: Target entity that should receive the choice menu.
- `prompt`: Message shown before the choices. Use `""` to skip the prompt.
- `choice_attribute`: Attribute on the offering character containing one or more choice labels.
- Selecting an option sends `{choice_attribute}` and `{choice_attribute}:{index}` back to the offering character, for example `door_choices` and `door_choices:0` for the first option.
- The event value contains the target entity id in `value.x`, the zero-based option index in `value.y`, and the selected label in `value.text`.
- `0` / cancel behaves like `offer_inventory()` and sends `goodbye` to the offering character.

The choice session uses the offering character's [timeout](attributes#timeout) attribute and the same distance validity behavior as `offer_inventory()`.

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

Defines how incoming player [actions](input_mapping#action-types) are translated into movement behavior.  
This command **does not change the visual rendering camera**. That is controlled by the game widget [camera setting](/docs/screens/widgets/#camera-section).
It only changes how player input is interpreted.

Current valid values are:

* `2d` — **forward**, **backward**, **left**, **right** move the character directly in the given world directions. This is the default.
* `2d_grid` — Like `2d`, but each movement action advances exactly one tile / world unit with smooth interpolation. Holding a direction repeats tile-by-tile.
* `iso` — Same movement behavior as `2d`, typically used with an isometric view.
* `iso_grid` — Alias of `2d_grid`, typically used with an isometric view.
* `firstp` — **forward** moves the player in the direction they are facing, **backward** moves opposite. **left** and **right** rotate instead of strafing. Optional `strafe_left` and `strafe_right` provide sidestepping.
* `firstp_grid` — Like `firstp`, but **forward** and **backward** move exactly one tile / world unit per action with smooth interpolation, while **left** and **right** rotate the facing by 90 degrees. Optional `strafe_left` and `strafe_right` sidestep by one tile without changing facing.

Typical combinations are:

* render camera `2d` with input mode `2d` or `2d_grid`
* render camera `iso` with input mode `iso` or `iso_grid`
* render camera `firstp` with input mode `firstp` or `firstp_grid`

The render camera and the input mode are separate systems. For example, `firstp_grid` reuses the normal first-person visual camera but changes input behavior to grid stepping.

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

## `teleport_entity`

*This command can be used from World scripts and other server scripts.*

Teleports a specific entity id to a named sector. The third parameter is optional and names the destination region. Use this from World scripts when a character or item delegates orchestration via `world_event(event, value)` and passes an entity id as the value.

```eldrin
teleport_entity(player_id, "entrance", "Dungeon")
```

---

## `build_procedural`

*This command can be used from World scripts and other server scripts.*

Rebuilds the current region from its `[procedural]` settings. It currently supports the 2D `connected_rooms` generator.

Pass a positive seed to rebuild with that exact seed. Pass `0` to advance the procedural run counter and derive the next seed from the region's configured seed.

```eldrin
build_procedural(0)
```

Region procedural settings are exposed through context variables. A script can read or write them before calling `build_procedural`.

```eldrin
region.procedural.room_count = 10
region.procedural.characters.skeleton.percentage = 55
build_procedural(0)
```

This is intended for roguelike loops where a World script handles a player reaching an exit, regenerates the current dungeon region, then moves the player to the new `entrance`.

```eldrin
fn event(event, value) {
    if event == "dungeon_exit" {
        build_procedural(0)
        teleport_entity(value, "entrance", "Dungeon")
    }
}
```

See also: [Procedural Map Generation](/docs/building_maps/procedural_generation).

---

## `world_event`

*This command can be used with characters, items, and region scripts.*

Queues an event for the World script. The World script receives it through its normal `event(event, value)` handler on the next script processing step.

Use this when local gameplay logic should delegate global orchestration, such as rebuilding a procedural dungeon, advancing a run, or moving a player between regions.

```eldrin
world_event("dungeon_exit", id())
```

World script:

```eldrin
fn event(event, value) {
    if event == "dungeon_exit" {
        build_procedural(0)
        teleport_entity(value, "entrance", "Dungeon")
    }
}
```

---
