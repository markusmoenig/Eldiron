---
title: "Input Mapping"
sidebar_position: 8
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
- bare action alias (`"forward"`, `"left"`, `"right"`, `"backward"`)

---

## Action Types

### `forward`

- **2D / Isometric**: Move the player north.
- **First-Person**: Move the player forward in current facing direction.

### `left`

- **2D / Isometric**: Move the player west.
- **First-Person**: Rotate left.

### `right`

- **2D / Isometric**: Move the player east.
- **First-Person**: Rotate right.

### `backward`

- **2D / Isometric**: Move the player south.
- **First-Person**: Move backward in current facing direction.

## Intents

`intent(<name>)` sets the player intent (for example `use`, `attack`, `take`).

You can define optional runtime distance limits for clicked intents in the same
character data with a top-level `[intent_distance]` table:

```toml
[intent_distance]
default = 2
attack = 3
take = 1.5
```

Behavior:

- if `[intent_distance]` is missing entirely, the engine uses `default = 2`
- `default` applies to all clicked intents unless overridden
- a matching intent key such as `attack` or `take` overrides `default`
- if the clicked target is farther away, the engine blocks the intent before it
  reaches scripts and sends `{system.too_far_away}`

You can also define optional intent rules globally in `Rules` and override them
per character template:

```toml
[intents.use]
allowed = "true"

[intents.take]
allowed = "distance <= 1.5"

[intents.attack]
allowed = "target.ALIGNMENT < 0"
deny_message = "{system.cant_do_that}"
cooldown = 2
```

Behavior:

- `allowed` is an expression evaluated before the `intent` event is sent
- `deny_message` is sent if `allowed` evaluates to false
- `cooldown` blocks the specific intent for the given number of in-game minutes
- per-character `[intents.<name>]` in character data override the global `Rules`
  entry for that intent

Supported variables currently include:

- `distance`
- `subject.<ATTR>` or `actor.<ATTR>`
- `target.<ATTR>`

For UI-driven intents, you can also use [button widgets](/docs/screens/screens_widgets#button-widgets).

For how intents behave in 2D vs 3D and how they become `intent` events, see [Player Input](player_input).

## Spell Shortcuts

`spell(<template>)` selects a specific spell intent button by template name.

Example:

```toml
f = "spell(Fireball)"
```

This activates the button with:

```toml
intent = "spell"
spell = "Fireball"
```
