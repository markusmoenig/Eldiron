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
