---
title: "Player Input"
sidebar_position: 6.5
---

This page explains how **player input** works in Eldiron.

At a high level, player input is routed through commands:

- **control.\***: direct movement or turning commands such as `control.forward`, `control.left`, `control.right`, and `control.backward`
- **intent.\***: programmable world interaction modes such as `intent.use`, `intent.look`, or `intent.drop`
- **rules.\***: ruleset actions such as `rules.basic_attack`, `rules.minor_heal`, or `rules.gather_wood`
- **screen.\***: screen flow commands such as `screen.goto.Title` or `screen.goto.Play`
- **game.\***: game flow commands such as `game.start` or `game.start_class.Warrior`
- **ui.\***: local user-interface commands such as `ui.actions`

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
- `look`
- `drop`
- `spell`

An intent can be selected by:

- a keyboard mapping like `intent.use`
- a spell shortcut like `spell(Fireball)`
- a screen button with an `intent` attribute

Ruleset-owned actions such as attacking and taking items should normally use
`rules.basic_attack` and `rules.take`. Legacy or script-driven games may still
use custom `attack` and `take` intents.

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
t = "rules.basic_attack"
u = "intent.use"
l = "intent.look"
tab = "ui.actions"
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

This separation remains part of the modern UI model. The Spellbook is a presentation and selection surface for `rules.*` abilities; it does not replace `intent.*`. Intents represent contextual interaction or targeting modes such as Walk/default, Look, Use, Drop, and choosing a world target. A spell selected from the Spellbook may enter that targeting layer, but the executable spell itself remains a rules action.

### Actions Panel

The five visible command slots are a quick-access bar, not the complete list of
commands known by a character. `ui.actions` toggles a generic Actions panel:

```toml
[input]
tab = "ui.actions"

# Or on a screen button:
[ui]
role = "button"
command = "ui.actions"
label = "Actions"
show_icon = false
```

The panel reads all commands in the active class's `[classes.<Class>.action_bar]`
table. It removes duplicates and groups rules actions from their `kind`:

- `spell` becomes **Spells**
- `gather`, `craft`, and `interaction` become **Utility**
- attacks, stances, and custom action kinds become **Combat**

Entries use the same rules-owned icons, tooltips, costs, reagent requirements,
cooldowns, and unlock checks as ordinary command-slot buttons. Selecting an
enabled entry highlights it and activates the normal rules action targeting
path; the panel stays open for subsequent choices until closed with its button,
close control, `Tab`, or Escape. This makes the generic panel suitable for
martial abilities and sandbox actions as well as magic. The Spellbook presents
the same complete ability catalogue through the configurable player-facing UI;
action kinds only organize its groups.

`ui.spellbook` opens the floating, draggable catalogue containing every rules action known by the active class. The active screen's action-bar widget may configure its built-in Spellbook under `[ui.spellbook]`, including its grid, tabs, details pane, fonts, colors, and artwork. A separate screen widget with `role = "spellbook"` overrides that default when a game needs its own rectangle or presentation. Both panels retain drag-to-slot behavior; set `show_assign = true` to expose the explicit Assign mode.

Drag a panel entry onto a `command_slot` button to replace that player's quick
slot. On touch or non-drag interfaces, use **Assign**, then click the action and
the destination slot. Eldiron persists the override on the player and validates
that the assigned command belongs to the active class action bar.

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

Text widgets on the start screen can preview those choices with placeholders such as `{START.CLASS}`, `{START.CLASS_ROLE}`, `{START.CLASS_ATTRIBUTES}`, `{START.CLASS_ABILITIES}`, `{START.CLASS_SPELLS}`, and `{START.CLASS_EQUIPMENT}`. The class details come from the active ruleset; abilities and spells use the class's starting `unlocks.level_1` entries and their authored catalogue names.

`game.start_class.<Class>` starts immediately with the requested class. If `[game].play_screen` is set, Eldiron switches to that screen after starting.

For a readable action bar overlay, place a `role = "deco"` widget behind the buttons and give it `layer = -1`. Negative-layer deco widgets draw below screen-rendered command icons, so a semi-transparent background can dim the game without dimming the icons.

### Isometric / First-Person

In isometric and first-person play, intents behave like a **persistent interaction mode** by default:

1. select an intent like `use`, `attack`, or `spell`
2. move the cursor over an entity or item
3. click the target to apply that intent

The selected mode remains active after targeting, so repeated attacks do not
require reselecting Attack. Choosing another targeting command or the
Walk/default button replaces it. The `persistent_intents` option only enables
the same persistence in 2D modes; it is not needed in 3D.

The active intent can also change the cursor if the corresponding button widget
defines intent cursor tiles.

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

## Ruleset Invocations

Rulesets may bind token sequences to ordinary actions. A game-specific screen
can present the tokens as words, runes, icons, or any other controls and submit
the assembled phrase:

```toml
command = "intent.invoke:words_of_power:{UI.spell.runes}"
```

For example, the official `LO VI` phrase resolves to `action:minor_heal`.
Resolution is server-side, and the action still owns its requirements, target,
costs, cooldown, and effects. Games that prefer direct spell or rules-action
buttons do not need to use invocation schemes.

When a Messages widget enables `command_input`, players can also type an
invocation directly:

```text
LO VI
SAR IR at skeleton
invoke words_of_power:YA FUL at orc
```

The text path resolves these through the same invocation catalogue. Self and
friendly-or-self actions infer the player when no target is written.

## Where To Configure What

- Keyboard mappings: [Input Mapping](input_mapping)
- Intent event handling: [Events](events#intent)
- Action/intention buttons on screens: [Screen Widgets](/docs/screens/widgets)
- Character and item shortcut attributes: [Attributes](attributes)
