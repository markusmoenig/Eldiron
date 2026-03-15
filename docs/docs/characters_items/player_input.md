---
title: "Player Input"
sidebar_position: 7.5
---

This page explains how **player input** works in Eldiron.

At a high level, player input is split into two concepts:

- **Actions**: direct movement or turning commands such as `forward`, `left`, `right`, and `backward`
- **Intents**: interaction modes such as `use`, `attack`, `look`, `take`, `drop`, or `spell`

Keyboard input is configured in character data via [Input Mapping](input_mapping).  
UI buttons on screens can also trigger the same actions and intents.

## Actions

Actions are immediate movement-style commands.

Examples:

- `forward`
- `left`
- `right`
- `backward`

They are sent as runtime `EntityAction` values and are interpreted based on the current player camera mode:

- **2D / Isometric**: directional movement
- **First-Person**: forward/backward movement plus left/right turning

If no intent is active, pressing an action key simply moves or turns the player.

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

If no valid target is found, the engine may send a localized fallback message such as `nothing_to_use` or `nothing_to_attack`.

### Isometric / First-Person

In isometric and first-person play, intents behave more like a **persistent interaction mode**:

1. select an intent like `use`, `attack`, or `spell`
2. move the cursor over an entity or item
3. click the target to apply that intent

The active intent can also change the cursor if the corresponding button widget defines intent cursor tiles.

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

- the player handles `attack` and calls `deal_damage(...)`
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
f = "spell(Fireball)"
```

Screen button mapping:

```toml
intent = "spell"
spell = "Fireball"
```

In both cases the runtime treats this as a spell intent and routes it through the normal intent system.

## Where To Configure What

- Keyboard mappings: [Input Mapping](input_mapping)
- Intent event handling: [Events](events#intent)
- Action/intention buttons on screens: [Screen Widgets](/docs/screens/widgets)
- Character and item shortcut attributes: [Attributes](attributes)
