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

## `blocking`

*Item-only attribute.*

If set to `true`, the item blocks movement based on its radius.

```toml
blocking = true
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

## `mode`

*Character-only attribute.*

The current mode of the entity. On startup of characters this is set to **"active"**, [took_damage](server_commands#took_damage) changes this to **"dead"** when the [health attribute](/docs/configuration/game#health) is below or equal to 0. Dead characters do not receive events. Healers can set the mode attribute to **"active"** again.

```python
set_attr("mode", "active")
```

---

## `monetary`

*Item-only attribute.*

If `true`, the item is considered money. It is not picked up normally, but its worth is added to the wallet.

```toml
monetary = true
```

---

## `name`

*General attribute (applies to both characters and items).*

Name of the character or item. Can override the template name.

```toml
name = "Golden Key"
```

---

## `player`

*Character-only attribute.*

Marks the character as a player-controlled character that receives input events.

```toml
player = true
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

```toml
tile_id = "abc123"
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
