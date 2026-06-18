---
title: "Player Input"
sidebar_position: 6.5
---

This page explains how **player input** works in Eldiron.

At a high level, player input is routed through commands:

- **control.\***: direct movement or turning commands such as `control.forward`, `control.left`, `control.right`, and `control.backward`
- **intent.\***: programmable interaction modes such as `intent.use`, `intent.attack`, `intent.look`, `intent.take`, or `intent.drop`
- **rules.\***: ruleset actions such as `rules.basic_attack`, `rules.minor_heal`, or `rules.gather_wood`
- **screen.\***: screen flow commands such as `screen.goto.Title` or `screen.goto.Play`
- **game.\***: game flow commands such as `game.start` or `game.start_class.Warrior`
- **ui.\***: user-interface commands such as `ui.inventory` for future action bars and panels

Keyboard input is configured in character data via [Input Mapping](input_mapping).  
UI buttons on screens can also trigger the same actions and intents. Button fields such as `action = "forward"` and `intent = "attack"` are read as `command = "control.forward"` and `command = "intent.attack"` when projects are loaded. Use `intent = ""` or `command = "intent."` for a Walk button which clears active targeting commands.

Rules commands are also rules-aware on the UI side. A button assigned to `rules.minor_heal` can show the action name, description, costs, reagent requirements, and cooldown state from the active ruleset. If that command is cooling down, the button is dimmed and receives a cooldown overlay.

The terminal roguelike client uses the same active player `[input]` table. In an
interactive terminal it reads raw keypresses, so movement keys act immediately
without pressing Return. When stdin/stdout are not terminals, it falls back to
line input so scripted tests can still pipe commands.

## Control Commands

Control commands are immediate movement-style commands.

Examples:

- `forward`
- `left`
- `right`
- `backward`
- `strafe_left`
- `strafe_right`

They are sent as runtime `EntityAction` values and are interpreted based on the current player camera mode:

- **2D / Isometric**: directional movement
- **2D Grid**: cardinal one-tile movement with smooth interpolation
- **First-Person**: forward/backward movement plus left/right turning, with optional strafing via `strafe_left` / `strafe_right`
- **First-Person Grid**: one-tile forward/backward/strafe movement with smooth interpolation, plus 90-degree left/right turning

If no intent is active, pressing an action key simply moves or turns the player.

The input mapping mode is controlled at runtime with [`set_player_camera`](server_commands#set_player_camera).
This affects how actions are interpreted, but it does not change the visual render camera by itself.

## Intents

Intents describe **what the player wants to do**, not how they move.

Common intents:

- `use`
- `attack`
- `look`
- `take`
- `drop`
- `spell`

An intent can be selected by:

- a keyboard mapping like `intent(use)`
- a spell shortcut like `spell(Fireball)`
- a screen button with an `intent` attribute

Once selected, the intent is stored on the player and used for the next interaction.

## 2D, Isometric, And First-Person Behavior

Intents behave differently in 2D and 3D.

### 2D

In 2D-style play, an intent is usually **one-shot**:

1. select an intent like `use` or `attack`
2. press a directional action like `forward`
3. the engine looks in that direction and sends the matching `intent` event

If no valid target is found, the engine sends the localized fallback `{system.cant_do_that}`.

This applies to both:

- `2d`
- `2d_grid`

You can opt into click-targeted 2D intent behavior with:

```toml
[game]
click_intents_2d = true
```

With that enabled, 2D behaves more like 3D:

1. select an intent like `use`, `attack`, or `spell`
2. move the cursor over an entity or item
3. click the target to apply that intent
4. the selected intent stays active until you switch it

Movement keys still walk normally. Intent hover / clicked cursors from screen buttons also apply in 2D when this mode is enabled.

### Recommended Rules-Based 2D Setup

For a rules-driven top-down game, the recommended starting point is:

```toml
[game]
auto_walk_2d = true
click_intents_2d = false
```

This gives the player mouse click-to-walk in Walk mode while keeping keyboard intents one-shot. A typical player input map then keeps movement, targeting, and rules actions separate:

```toml
[input]
w = "control.forward"
a = "control.left"
s = "control.backward"
d = "control.right"
t = "command(rules.basic_attack)"
u = "intent(use)"
l = "intent(look)"
```

For the screen action bar, use a Walk/default button plus rules command buttons:

```toml
[ui]
role = "button"
command = "intent."

# Another button:
[ui]
role = "button"
command = "rules.basic_attack"
```

`command = "intent."` selects Walk mode and clears active targeting commands. Intent and rules command buttons can resolve their icons through the active ruleset's `[icons]` catalog. Rules command buttons also get their name, description, cooldown, reagent/cost status, disabled state, and shortcut hint from the active ruleset and the active player's `[input]` table.

Class-driven action bars can bind buttons to command slots instead of hardcoding one command per screen:

```toml
[ui]
role = "button"
command_slot = "main.0"

[ui]
role = "button"
command_slot = "main.1"
```

Command slots resolve through the active player. A player attribute such as `command_slot_main_0 = "rules.minor_heal"` can override a slot; otherwise Eldiron reads the active ruleset class, for example `[classes.Cleric.action_bar] main = ["rules.basic_attack", "rules.minor_heal", "rules.holy_light", "rules.gather_herbs", "rules.craft_blessed_herb"]`. This keeps fixed world intents like Walk, Look, and Use separate from class actions.

Screen flow buttons use the same command field:

```toml
[ui]
role = "button"
command = "screen.goto.Play"

[ui]
role = "button"
command = "game.start"
```

`game.start` creates the configured player template using generic screen UI state. Start screens usually bind class buttons to `start.class` and text input to `start.name`:

```toml
[ui]
role = "button"
bind = "start.class"
value = "Warrior"
selection = "single"

[ui]
role = "input"
bind = "start.name"
text = "Eldiron"
```

Text widgets on the start screen can preview those choices with placeholders such as `{START.CLASS}`, `{START.CLASS_ROLE}`, `{START.CLASS_ATTRIBUTES}`, `{START.CLASS_ABILITIES}`, and `{START.CLASS_EQUIPMENT}`. The class details come from the active ruleset.

`game.start_class.<Class>` starts immediately with the requested class. If `[game].play_screen` is set, Eldiron switches to that screen after starting.

For a readable action bar overlay, place a `role = "deco"` widget behind the buttons and give it `layer = -1`. Negative-layer deco widgets draw below screen-rendered command icons, so a semi-transparent background can dim the game without dimming the icons.

### Isometric / First-Person

In isometric and first-person play, intents behave more like a **persistent interaction mode**:

1. select an intent like `use`, `attack`, or `spell`
2. move the cursor over an entity or item
3. click the target to apply that intent

The active intent can also change the cursor if the corresponding button widget defines intent cursor tiles.

This applies to:

- `iso`
- `firstp`
- `firstp_grid`

## Camera Input Modes

The current player input mode can be:

- `2d`: freeform cardinal movement
- `2d_grid`: smooth grid-based cardinal movement, one tile / world unit per action
- `iso`: same movement semantics as `2d`, usually paired with an isometric render camera
- `iso_grid`: alias of `2d_grid`, usually paired with an isometric render camera
- `firstp`: freeform first-person movement and turning
- `firstp_grid`: smooth grid-based first-person movement, one tile / world unit per step and 90-degree turns

## Intent Events

When an intent is triggered successfully, the engine sends an [`intent`](events#intent) event.

That event is sent to:

- the player character
- the clicked target entity, if the target is a character
- the clicked target item, if the target is an item

The event payload includes:

- `intent`
- `entity_id`
- `item_id`
- `distance`

This lets either side handle the interaction.

Examples:

- the player handles `attack` and calls `attack()`
- an item handles `use` and toggles itself
- a character handles `talk` and opens dialogue

## Built-In Shortcuts

Some common intents have built-in convenience behavior before or alongside script handling.

Examples include:

- `look`
- `use`
- `take` / `pickup`
- `drop`
- `spell:<template>`

Character and item attributes such as `on_look`, `on_use`, and `on_drop` can provide shortcuts for common cases.

## Spells

Spell shortcuts are encoded as intent payloads of the form:

```text
spell:Fireball
```

Keyboard mapping:

```toml
[input]
f = "command(intent.spell:Fireball)"
```

Screen button mapping:

```toml
command = "intent.spell:Fireball"
```

In both cases the runtime treats this as a spell intent and routes it through the normal intent system.

## Where To Configure What

- Keyboard mappings: [Input Mapping](input_mapping)
- Intent event handling: [Events](events#intent)
- Action/intention buttons on screens: [Screen Widgets](/docs/screens/widgets)
- Character and item shortcut attributes: [Attributes](attributes)
