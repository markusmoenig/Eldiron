---
title: "Attributes"
sidebar_position: 5
---

This chapter lists supported **attributes** for characters and items in Eldiron.

Attributes can be applied to characters, items, or both.

This page covers individual attributes such as `tile_id`, `radius`, or `timeout`.
Gameplay rules such as weapon damage, spell cooldowns, intent ranges, class
permissions, armor categories, and progression belong in the effective ruleset:
the official ruleset plus **Game / Rules** project overrides.

Some character configuration in the **Attributes** editor also uses top-level TOML tables instead of single attributes, for example:

- [NPC Sequences](/docs/characters_items/npc_sequences) via `behavior.sequences`
- [Input Mapping](/docs/characters_items/input_mapping) via `[input]`

---

## Attributes

## `active`

*Item-only attribute.*

Represents the active state of an item (on or off). When this attribute is changed, an [active event](events#active) is automatically send to the item to allow the item to sync its visual state.

```toml
active = true
```

---

## `autodamage`

*Character-only attribute.*

If set to `true`, incoming damage is applied directly by the server and the [take_damage](events#take_damage) event is skipped.
Use this for simple NPCs/targets that should not run custom damage scripts.
On lethal damage, the server also applies death state automatically (`mode = "dead"` and `visible = false`).

```toml
autodamage = true
```

---

## `autodrop`

*Character-only attribute.*

If set to `true`, the character automatically drops all inventory and equipped items to the floor on death.

```toml
autodrop = true
```

---

## `blocking`

*Item-only attribute.*

If set to `true`, the item blocks movement based on its radius.

```toml
blocking = true
```

---

## `class`

*Character-only attribute.*

Optional gameplay role or archetype for a character.

The class name selects a class definition from the effective ruleset. Class
rules, progression, abilities, equipment permissions, and cooldowns should be
defined in the ruleset, not repeated on the character.

```toml
class = "Warrior"
```

---

## `color`

*Item-only attribute.*

Hex color code that overrides geometry color when item is equipped.

```toml
color = "#ff0000"
```

---

## `color_targets`

*Item-only attribute.*

List of geometry node names whose color should be overridden when the item is equipped.

```toml
color_targets = ["left_leg", "right_leg"]
```

---

## `facing`

*Character-only attribute.*

Initial facing direction on spawn.  
Supported values: `"front"`, `"back"`, `"left"`, `"right"`  
(also accepts `"north"`, `"south"`, `"west"`, `"east"`).

```toml
facing = "right"
```

---

## `geo_targets`

*Item-only attribute.*

List of linedef names to attach this item's geometry to when equipped.  
Used only when automatic matching by `slot` is insufficient.

```toml
geo_targets = ["left_shoulder", "right_shoulder"]
```

---

## `hold_speed`

*Character-only attribute.*

Sustained movement speed used for held grid movement input after the first tile.
This applies to grid-based player movement such as `2d_grid` and `firstp_grid`.
If omitted, it falls back to `speed`.

```toml
hold_speed = 3.0
```

Example:

```toml
speed = 8.0
hold_speed = 3.0
```

This gives a fast first tile while keeping held movement smooth and continuous.

---

## `inventory_slots`

*Character-only attribute.*

Number of inventory slots the character has. If not specified, defaults to 0.

```toml
inventory_slots = 8
```

---

## `party_index`

*Character-only attribute.*

Optional UI/runtime party order for party-bound screen widgets.
Lower numbers come first, so `0` is typically the leader.

```toml
party_index = 0
```

---

## `party_role`

*Character-only attribute.*

Optional named role used by party-bound screen widgets.
This is useful for stable bindings such as `"leader"`, `"tank"`, or `"healer"`.

```toml
party_role = "leader"
```

---

## `portrait_tile_id`

*Character-only attribute.*

Static portrait tile used by screen button widgets with `portrait = true`.
This is separate from runtime `avatar` rendering and is intended for classic portrait/head UI graphics.

You can store it the same way other tile-based attributes are authored: as a tile UUID, a tile alias, or a palette index.

```toml
portrait_tile_id = "01234567-89ab-cdef-0123-456789abcdef"
portrait_tile_id = "hero_portrait"
portrait_tile_id = "2"
```

---

## `start_items`

*Character-only attribute.*

List of item template names to add to the character inventory on spawn (not equipped).
When omitted, the active ruleset class can provide its default inventory.

```toml
start_items = ["Sword", "Potion"]
```

---

## `start_equipped_items`

*Character-only attribute.*

List of item template names to add and auto-equip on spawn.  
Items must define a valid `slot` attribute.
When omitted, the active ruleset class can provide its default equipped weapons,
armor, and clothing.

```toml
start_equipped_items = ["Shield", "Helmet"]
```

---

## `mode`

*Character-only attribute.*

The current mode of the entity. On startup of characters this is set to **"active"**. When the ruleset health attribute drops to zero or below, the server changes it to **"dead"** automatically. Dead characters do not receive events. Healers can set the mode attribute to **"active"** again. If health is still zero, `set_attr("mode", "active")` restores it from `MAX_<health>` or `MAX_HP`.

```python
set_attr("mode", "active")
```

---

## `route`

*Character-only attribute.*

Patrol route definition used by [patrol](server_commands#patrol).
Each entry is a linedef `name` that belongs to the patrol path.

```toml
route = ["GuardRouteA", "GuardRouteB"]
```

You can also use a single route name:

```toml
route = "GuardRouteA"
```

---

## `route_mode`

*Character-only attribute.*

Controls route traversal mode for [patrol](server_commands#patrol).
Default is `"loop"`.

```toml
route_mode = "loop"
```

Supported values:

- `loop`: restart from the first point after the last point.
- `pingpong`: reverse direction at the route ends.

---

## `timeout`

*Character-only attribute.*

Generic NPC interaction timeout used by scripts when temporarily interrupting background behavior such as [NPC sequences](npc_sequences).

```toml
timeout = 10
```

Typical use:

- pause a sequence during `talk`
- wait for the player to finish interacting
- resume the sequence when the interaction ends or times out

This attribute is authoring data for your scripts. It does not automatically pause or resume anything by itself.

NPC background workflows themselves are defined separately in the character **Attributes** editor under `behavior.sequences`. See [NPC Sequences](npc_sequences).

---

## Spell Attributes

Spell gameplay is ruleset data.

Define spell mode, costs, cooldowns, target rules, range, damage or healing,
visuals, audio, reagents, and messages in the effective ruleset. Use
**Game / Rules** when this project needs to change the official spell rules.
Project item data should only carry concrete asset/state information for a
materialized spell item.

---

## `monetary`

*Item-only attribute.*

If `true`, the item is considered money. It is not picked up normally, but its worth is added to the wallet.

```toml
monetary = true
```

---

## `on_look`

*General attribute (applies to both characters and items).*

Shortcut for the `look` intent.

If set and the player uses `look` on the character or item, this text is sent as a system message directly, without requiring script code.

```toml
on_look = "You see a sword."
```

---

## `on_pickup`

*Item-only attribute.*

Shortcut for pickup/take behavior on items.
`on_take` is supported as an alias.

If set and the player uses `pickup`/`take` on the item:
- `"pickup"` (or `"take"`) performs default pickup logic (same as calling `take`).
- Any other text is sent as a system message.

```toml
on_pickup = "pickup"
# alias:
# on_take = "pickup"
# or:
# on_pickup = "It's stuck in the stone."
```

---

## `on_use`

*Item-only attribute.*

Shortcut for the `use` intent on items.

If set and the player uses `use` on the item, this text is sent as a system message directly, without requiring item script code.

```toml
on_use = "You cannot use that."
```

---

## `on_drop`

*Item-only attribute.*

Shortcut for the `drop` intent on items.

If set and the player uses `drop` on the item (including inventory/equipped item clicks):
- If empty or `"drop"`, default drop logic runs (drops item to the ground).
- `"You cannot drop that"` sends that message and prevents dropping.
- Any other text sends the message and still drops the item.

```toml
on_drop = "You dropped a sword."
# or:
# on_drop = "You cannot drop that."
```

---

## `name`

*General attribute (applies to both characters and items).*

Name of the character or item. Can override the template name.

```toml
name = "Golden Key"
```

---

## `race`

*Character-only attribute.*

Optional race/species identity for a character.

The race name selects a race definition from the effective ruleset. Race
relations, reputation defaults, progression, bonuses, and visual defaults should
be defined in the ruleset.

```toml
race = "Orc"
```

---

## `player`

*Character-only attribute.*

Marks the character as a player-controlled character.

```toml
player = true
```

---

## `input`

*Player character input mapping (top-level table in the character **Attributes** editor).*

Maps keys to player actions/intents. See [Input Mapping](input_mapping) for syntax and supported commands.

```toml
player = true

[input]
w = "action(forward)"
a = "action(left)"
s = "action(backward)"
d = "action(right)"
u = "intent(use)"
```

---

## `size`

*Character-only attribute.*

Billboard height/width scale for characters rendered as billboards in 3D views.  
Default is `2.0` if no size attribute is set.

```toml
size = 2.0
```

---

## `size_2d`

*Character-only attribute.*

2D avatar scale for characters rendered from an `avatar` in 2D views.  
Default is `1.0` if no `size_2d` attribute is set.

Unlike `size`, this only affects 2D avatar rendering. It does not change gameplay position, grid alignment, collision, or pathing.

```toml
size_2d = 1.25
```

---

## `speed`

*Character-only attribute.*

Movement speed multiplier used by server-driven character movement such as `goto()`, `random_walk()`, `patrol()`, and grid tile traversal.
Higher values make each tile or movement step complete faster.
Default: `1.0`.

```toml
speed = 1.0
```

Example:

```toml
speed = 8.0
```

---

## `billboard_alignment`

*Item-only attribute.*

Controls how item sprites are aligned in 3D when rendered as billboards.

Supported values:
- `"upright"` (default): camera-facing upright billboard
- `"floor"`: ground-aligned billboard (lies flat on the floor)

Aliases accepted for floor alignment: `"ground"`, `"flat"`.

```toml
billboard_alignment = "floor"
```

---

## `radius`

*General attribute (applies to both characters and items).*

Collision radius of the character or item. Default is `0.5`.

```toml
radius = 0.3
```

---

## `slot`

*Item-only attribute.*

Slot name the item occupies when equipped (e.g. `"legs"`, `"head"`).

```toml
slot = "legs"
```

---

## `static`

*Item-only attribute.*

If `true`, the item is static and cannot be picked up (e.g. doors, campfires).

```toml
static = true
```

---

## `tile_id`

*General attribute (applies to both characters and items).*

Tile ID for the visual representation. Use the tile picker to find valid IDs.

Accepted forms:

- tile UUID
- tile alias
- palette index

Examples:

```toml
tile_id = "03160f57-90e3-4455-a16e-f0b8edfaa415"
tile_id = "player_tile"
tile_id = 2
tile_id = "2"
```

---

## `visible`

*General attribute (applies to both characters and items).*

Whether the character or item is visible in the world.

```toml
visible = false
```

---

## `wealth`

*Character-only attribute.*

Inital wealth of the character in base currency.

```toml
wealth = 2
```

---

## `worth`

*Item-only attribute.*

Trade value of the item in base currency.

```toml
worth = 2
```

---

## Emitting Light

Both entities and items can emit light by configuring the `light` group in their data tool.

Light emittance can be set on / off via the [set_emit_light](server_commands#set_emit_light) command.

```toml
[light]
color = "#ffffff" # Light Color
strength = 5.0      # Strength of the Light
range = 3.0         # Range of the light
flicker = 0.4       # Amount of light flickering
```

---

## Billboard Gate/Door Animation

*Item-only attributes; only valid for items linked to Gate/Door sectors that use billboard sprites.*

### `animation`

Billboard animation type applied when the sector opens/closes. Valid values: `"fade"`, `"up"`, `"down"`, `"left"`, `"right"`.

```toml
animation = "fade"
```

---

### `animation_duration`

Duration of the animation in seconds.

```toml
animation_duration = 1.0
```

---

### `animation_clock`

Timing mode for the animation: `"smooth"` (time-based) or `"tick"` (game-tick based).

```toml
animation_clock = "smooth"
```
