---
title: "Input Mapping"
sidebar_position: 7
---

For the overall input model, see [Player Input](player_input).

Player input is mapped in character data via a top-level `[input]` table.

```toml
player = true

[input]
w = "action(forward)"
a = "action(left)"
s = "action(backward)"
d = "action(right)"
u = "intent(use)"
t = "intent(attack)"
k = "intent(take)"
f = "spell(Fireball)"
```

Key names are matched case-insensitively.

---

## Commands

Each entry value supports one of:

- `action(<type>)`
- `intent(<name>)`
- `spell(<template>)`
- bare action alias (`"forward"`, `"left"`, `"right"`, `"backward"`, `"strafe_left"`, `"strafe_right"`)

---

## Action Types

### `forward`

- **2D / Isometric**: Move the player north.
- **2D Grid**: Move the player one tile north with smooth interpolation.
- **First-Person**: Move the player forward in current facing direction.
- **First-Person Grid**: Move the player one tile forward in current facing direction with smooth interpolation.

### `left`

- **2D / Isometric**: Move the player west.
- **2D Grid**: Move the player one tile west with smooth interpolation.
- **First-Person**: Rotate left.
- **First-Person Grid**: Rotate left by 90 degrees.

### `right`

- **2D / Isometric**: Move the player east.
- **2D Grid**: Move the player one tile east with smooth interpolation.
- **First-Person**: Rotate right.
- **First-Person Grid**: Rotate right by 90 degrees.

### `backward`

- **2D / Isometric**: Move the player south.
- **2D Grid**: Move the player one tile south with smooth interpolation.
- **First-Person**: Move backward in current facing direction.
- **First-Person Grid**: Move the player one tile backward in current facing direction with smooth interpolation.

### `strafe_left`

- **First-Person**: Sidestep left without changing facing.
- **First-Person Grid**: Sidestep one tile left with smooth interpolation without changing facing.

### `strafe_right`

- **First-Person**: Sidestep right without changing facing.
- **First-Person Grid**: Sidestep one tile right with smooth interpolation without changing facing.

How these control commands are interpreted depends on the current runtime player input mode set by [`set_player_camera`](server_commands#set_player_camera):

- `2d`
- `2d_grid`
- `iso`
- `iso_grid`
- `firstp`
- `firstp_grid`

## Intents

`intent(<name>)` or `command(intent.<name>)` sets the player intent (for example `use`, `attack`, `take`).

Intent policy comes from the official ruleset. Character input should map keys
to intent names; ranges, target restrictions, cooldowns, and disposition checks
belong to the ruleset.

```toml
[intents.attack]
allowed_dispositions = ["hostile"]
deny_message = "{system.cant_do_that}"

[intents.attack.distance]
source = "weapon_range"
fallback = 1.5
```

Behavior:

- `allowed_dispositions` checks the target's disposition from the ruleset
- `allowed_target_kinds` limits an intent to entities or items
- `distance` sets a fixed range or a structured range source
- `deny_message` is sent if the rule blocks the intent
- `cooldown` blocks the specific intent for the ruleset-defined duration

For UI-driven intents, you can also use [button widgets](/docs/screens/widgets#button-widgets).
If a button command matches a key in the active player's `[input]` table, its hover tooltip shows the shortcut.

For how intents behave in 2D vs 3D and how they become `intent` events, see [Player Input](player_input).

## Spell Shortcuts

`spell(<template>)` is still accepted and maps to `intent.spell:<template>`.
New mappings can use the command form directly.

Example:

```toml
f = "command(intent.spell:Fireball)"
```

This activates the button with:

```toml
command = "intent.spell:Fireball"
```
