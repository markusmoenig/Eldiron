---
title: "Input Mapping"
sidebar_position: 7
---

For the overall input model, see [Player Input](player_input).

Player input is mapped in character data via a top-level `[input]` table.

```toml
player = true

[input]
w = "control.forward"
a = "control.left"
s = "control.backward"
d = "control.right"
u = "intent.use"
l = "intent.look"
t = "rules.basic_attack"
k = "rules.take"
f = "intent.spell:Fireball"
tab = "ui.actions"
```

Key names are matched case-insensitively.

---

## Commands

New mappings should use the same command namespaces as screen buttons:

- `control.<action>`
- `intent.<name>`
- `rules.<action_id>`
- `screen.<command>`
- `game.<command>`
- `ui.<command>`

The older wrapper forms remain accepted for existing projects:

- `action(<type>)`
- `intent(<name>)`
- `spell(<template>)`
- `command(<namespaced-command>)`
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

`intent.<name>` selects a generic world interaction mode such as `use` or
`look`. The wrapper form `intent(<name>)` remains supported.

Executable ruleset actions use `rules.<action_id>`. For example, bind the
official attack and pickup actions as `rules.basic_attack` and `rules.take`
rather than treating their action ids as generic intents. This lets command-slot
buttons resolve the matching shortcut, requirements, range, cooldown, and
presentation from the active ruleset.

`ui.actions` opens the reusable ruleset action catalogue. Its actions can be
dragged onto screen buttons with `command_slot = "main.0"` and similar slot ids,
or assigned with the panel's **Assign** mode. The resulting player override is
persistent; the class action bar remains the fallback for slots without an
override.

`ui.spellbook` opens the complete active-class ability catalogue in the configurable
player-facing book. Action `kind` values organize entries into Combat, Spells, and
Utility; they do not exclude non-magical abilities. The book uses the same ruleset
icons, availability checks, and quick-slot assignment path.

```toml
[actions.basic_attack]
target = "hostile_or_neutral_entity"
range = "weapon"
cooldown = 1.0
```

Behavior:

- `target` checks the target kind and disposition from the ruleset
- `allowed_target_kinds` limits an intent to entities or items
- `distance` sets a fixed range or a structured range source
- `deny_message` is sent if the rule blocks the intent
- `cooldown` blocks the specific intent for the ruleset-defined duration

For UI-driven intents, you can also use [button widgets](/docs/screens/widgets#button-widgets).
If a button command matches a key in the active player's `[input]` table, its hover tooltip shows the shortcut.

`ui.actions` is a local interface command that toggles the ruleset-driven
Actions panel. It does not send an action to the server by itself; selecting a
panel entry submits the corresponding `rules.<action_id>` command.

For how intents behave in 2D vs 3D and how they become `intent` events, see [Player Input](player_input).

## Spell Shortcuts

`spell(<template>)` is still accepted and maps to `intent.spell:<template>`.
New mappings can use the command form directly.

Example:

```toml
f = "intent.spell:Fireball"
```

This activates the button with:

```toml
command = "intent.spell:Fireball"
```

## Invocation Sequences

An optional ruleset invocation sequence uses:

```toml
command = "intent.invoke:words_of_power:LO VI"
```

The server resolves the scheme and phrase to a normal ruleset action. The same
action can still be selected directly through `rules.<action_id>` or presented
with an icon. Invocation tokens are therefore an optional input vocabulary,
not a separate spell implementation.
