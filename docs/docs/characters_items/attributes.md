---
title: "Attributes"
sidebar_position: 5
---

This chapter lists all supported **attributes** for characters and items in Eldiron.  

Attributes can be applied to characters, items, or both.

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

Use this for things like:

- text targeting by class, for example `attack warrior`
- future class-based rules, progression, or bonuses

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

## `geo_targets`

*Item-only attribute.*

List of linedef names to attach this item's geometry to when equipped.  
Used only when automatic matching by `slot` is insufficient.

```toml
geo_targets = ["left_shoulder", "right_shoulder"]
```

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

## `facing`

*Character-only attribute.*

Initial facing direction on spawn.  
Supported values: `"front"`, `"back"`, `"left"`, `"right"`  
(also accepts `"north"`, `"south"`, `"west"`, `"east"`).

```toml
facing = "right"
```

---

## `start_items`

*Character-only attribute.*

List of item template names to add to the character inventory on spawn (not equipped).

```toml
start_items = ["Sword", "Potion"]
```

---

## `start_equipped_items`

*Character-only attribute.*

List of item template names to add and auto-equip on spawn.  
Items must define a valid `slot` attribute.

```toml
start_equipped_items = ["Shield", "Helmet"]
```

---

## `mode`

*Character-only attribute.*

The current mode of the entity. On startup of characters this is set to **"active"**. When the [health attribute](/docs/configuration/game#health) drops to zero or below, the server changes it to **"dead"** automatically. Dead characters do not receive events. Healers can set the mode attribute to **"active"** again.

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

## `damage_kind`

*Item-only attribute.*

Damage kind used by `attack()` when this item is the active weapon.
Default: `physical`.

```toml
damage_kind = "fire"
```

This maps weapon attacks to the matching `combat.kinds.<kind>` rule path, for example:

- `damage_kind = "physical"` -> `combat.kinds.physical`
- `damage_kind = "fire"` -> `combat.kinds.fire`
- `damage_kind = "ice"` -> `combat.kinds.ice`

See [Rules](../rules) for the full rules format and combat kind configuration.

---

## Spell Attributes

These are *item-only attributes* used by spell templates/items (`is_spell = true`).

## `is_spell`

Marks an item as a spell runtime object.

```toml
is_spell = true
```

## `spell_mode`

Spell simulation mode.
Default: `projectile`.

```toml
spell_mode = "projectile"
```

Currently supported: `projectile`.

## `spell_effect`

Default effect applied when the spell hits.
Default: `damage`.

```toml
spell_effect = "damage"
# or:
# spell_effect = "heal"
```

## `spell_kind`

Damage kind used for combat rules, combat messages, and combat audio.
Default: `spell`.

```toml
spell_kind = "fire"
```

This maps the spell to the matching `combat.kinds.<kind>` rule path, for example:

- `spell_kind = "spell"` -> `combat.kinds.spell`
- `spell_kind = "fire"` -> `combat.kinds.fire`
- `spell_kind = "ice"` -> `combat.kinds.ice`

See [Rules](../rules) for the full rules format and combat kind configuration.

## `spell_target_filter`

Target filtering used by spell hit checks.
Default: `any`.

```toml
spell_target_filter = "enemy"
```

Supported values: `enemy`, `ally`, `self`, `any`.

You can also use a numeric attribute expression on the target, for example:

```toml
spell_target_filter = "ALIGNMENT < 0"
```

Supported operators in expressions: `<`, `<=`, `>`, `>=`, `==`, `!=`.

## `spell_amount`

Effect magnitude (damage/heal amount).
Default: `1`.

```toml
spell_amount = 3
```

## `spell_speed`

Projectile movement speed.
Default: `6.0`.

```toml
spell_speed = 6.0
```

## `spell_cast_time`

Cast wind-up time in real-time seconds.
While casting, the spell is held in front of the caster before it starts moving.
Default: `0.0` (instant start).

```toml
spell_cast_time = 0.6
```

## `spell_cooldown`

Cooldown in real-time seconds (per caster, per spell template).
Default: `0.0` (no cooldown).

```toml
spell_cooldown = 1.5
```

## `spell_cast_offset`

Distance in map units to hold the spell in front of the caster during cast wind-up.
Default: `0.6`.

```toml
spell_cast_offset = 0.6
```

## `spell_cast_height`

Height used while the spell is in cast wind-up (preview state).
Default: `0.5`.

```toml
spell_cast_height = 0.5
```

## `spell_flight_height`

Height used while the projectile is traveling.
Default: `0.5`.

```toml
spell_flight_height = 0.5
```

## `spell_max_range`

Maximum travel distance before expire (`0` = unlimited).
Default: `0.0` (unlimited).

```toml
spell_max_range = 12.0
```

## `spell_lifetime`

Maximum lifetime in seconds.
Default: `3.0`.

```toml
spell_lifetime = 3.0
```

## `spell_radius`

Hit radius for projectile collision checks.
Default: `0.4`.

```toml
spell_radius = 0.4
```

## `effect_id`

Optional impact tile source.
Accepts a tile UUID, tile alias, or palette index.
If set, projectile spells switch to this tile on hit before despawn.

```toml
effect_id = "c4323247-0b92-4bf6-b303-643d8350f794"
# effect_id = "fire_impact"
# effect_id = "7"
```

## `effect_duration`

How long the impact visual (from `effect_id`) remains visible, in real-time seconds.
Default: `0.25`.

```toml
effect_duration = 0.25
```

## `effect_height`

Optional impact height override.
If not set, impact keeps the projectile's current height.

```toml
effect_height = 0.7
```

## `on_cast`

Optional message sent to the caster when the spell is successfully cast.

```toml
on_cast = "You cast a fireball"
```

## `reagents`

List of required reagents. Duplicates represent quantity.

```toml
reagents = ["Ginseng", "Ginseng", "Mandrake"]
```

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

Use this for things like:

- text targeting by race, for example `look orc`
- future race-based rules, progression, or bonuses
- separating gameplay identity from the visible display name

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

*Player character input mapping (top-level table in character data).*

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
