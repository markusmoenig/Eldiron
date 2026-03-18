---
title: "Authoring Configuration"
sidebar_position: 2
---

You can configure text-adventure and authoring-related runtime behavior by selecting **Game -> Authoring** in the **project tree**.

This page documents:

- the global `Game / Authoring` TOML configuration
- per-sector metadata keys used by sector descriptions
- template authoring metadata used by `look` for characters and items

---

## Startup

Startup options are located in the `[startup]` section.

```toml
[startup]
welcome = ""
show = "room"
```

### `welcome`

- Optional text shown before any room or sector text in text-style clients.
- Useful for a game intro like `Welcome to my game.`
- Supports multi-line TOML strings:

```toml
welcome = """
Welcome to my game.
"""
```

### `show`

- Controls what text-style clients show automatically on startup.
- Supported values:
  - `"room"`: show the full room view, including exits.
  - `"description"`: show only the current sector description.
  - `"none"`: show nothing automatically.

`"room"` is the default because it includes exits and current room context.

---

## Colors

Terminal presentation colors are configured in the `[colors]` section.

```toml
[colors]
title = "cyan"
objects = "white"
items = "bright_magenta"
characters = "white"
corpses = "bright_black"

[[colors.character_rules]]
when = "ALIGNMENT < 0"
color = "red"

[[colors.character_rules]]
when = "ALIGNMENT > 0"
color = "bright_green"

[colors.message_categories]
success = "bright_green"
warning = "bright_yellow"
severe = "bright_red"
error = "bright_red"
system = "cyan"
multiple_choice = "bright_magenta"
```

These colors are currently used by text-style terminal clients.

### `title`

- Color for the room title.

### `objects`

- General fallback color for room contents.
- Used as a fallback for items, characters, and corpses if their more specific color is not set.

### `items`

- Color for visible items in the room.

### `characters`

- Default color for neutral characters.

### `corpses`

- Color for dead characters shown as corpses.

### `[[colors.character_rules]]`

- Optional ordered rules for character colors.
- Rules are checked from top to bottom.
- The first matching rule wins.
- `when` currently supports simple numeric comparisons such as:
  - `ALIGNMENT < 0`
  - `ALIGNMENT > 0`
  - `LEVEL >= 5`
- If no rule matches, the base `characters` color is used.

### `[colors.message_categories]`

- Optional terminal colors for message categories.
- These colors are used for incoming runtime messages in text-style clients.
- If not specified, terminal clients use built-in defaults.
- Useful keys include:
  - `success`
  - `warning`
  - `severe`
  - `error`
  - `system`
  - `multiple_choice`

Supported examples include:
- `"cyan"`
- `"yellow"`
- `"bright_cyan"`
- `"bright_white"`
- `"red"`

---

## Terminal Message Colors

Text-style terminal clients also colorize incoming messages by message category.

Default mapping:

- `success`: bright green
- `warning`: bright yellow
- `severe`: bright red
- `error`: bright red
- `system`: cyan
- `multiple_choice`: bright magenta

So if you want XP gain or level-up messages to appear green in the terminal client, use:

```toml
[progression.messages]
xp_category = "success"
level_up_category = "success"
```

---

## Sector Messages

Sector enter-message options are located in the `[sector_messages]` section.

```toml
[sector_messages]
mode_2d = "always"
mode_3d = "always"
cooldown_minutes_2d = 10
cooldown_minutes_3d = 10
show_on_startup = true
```

These rules control when authored sector descriptions are sent as normal `system` messages to players in 2D and 3D.

### `mode_2d`

- Controls sector description messages for players in **2D**.
- Supported values:
  - `"always"`: show every time the player enters the sector.
  - `"once"`: show only the first time for that player.
  - `"cooldown"`: show again only after the configured cooldown.
  - `"never"`: never show sector descriptions automatically in 2D.

### `mode_3d`

- Same as `mode_2d`, but for **3D** play (`iso` and `firstp`).

### `cooldown_minutes_2d`

- Used when `mode_2d = "cooldown"`.
- Defines how many **in-game minutes** must pass before the same sector description may be shown again in 2D.

### `cooldown_minutes_3d`

- Used when `mode_3d = "cooldown"`.
- Defines how many **in-game minutes** must pass before the same sector description may be shown again in 3D.

### `show_on_startup`

- If `true`, the player's starting sector may also show its description automatically.
- If `false`, only later sector entries may show automatic descriptions.

---

## Sector Metadata

Sectors can define authoring metadata in the **Authoring** dock.

Minimal format:

```toml
title = ""
description = """
"""
```

### `title`

- Player-facing display name of the sector.
- Used by text-style clients and room descriptions.

Example:

```toml
title = "Your ship"
```

### `description`

- Main descriptive text for the sector.
- Used in text-style clients and, depending on `sector_messages`, as an automatic enter message in normal gameplay.

Example:

```toml
description = """
The familiar deck of your faithful ship creaks softly beneath your feet.
"""
```

### `show_in_2d`

- Optional per-sector override.
- If set to `false`, the sector description is not shown automatically in 2D, even if global `sector_messages.mode_2d` would allow it.

Example:

```toml
show_in_2d = false
```

### `show_in_3d`

- Optional per-sector override.
- If set to `false`, the sector description is not shown automatically in 3D.

Example:

```toml
show_in_3d = false
```

This is useful for places like a `Crossroads` sector that should exist for navigation but should stay quiet in moment-to-moment 2D play.

---

## Character And Item Template Authoring

Character and item templates have separate **Authoring** metadata in addition to their normal `Data` TOML.

Use:

- the **Authoring** dock for descriptive/player-facing text
- the **Data** dock for mechanical attributes and gameplay configuration

### Character Templates

Minimal example:

```toml
title = "Guard"
description = """
A weary guard watches the road.
"""

[mode.active]
description = """
A weary guard watches the road.
"""

[mode.dead]
description = """
The guard lies motionless on the ground.
"""
```

Characters may define optional `mode.*` overrides. These are used by `look` based on the current runtime mode, for example:

- `mode.active`
- `mode.dead`

If the current `mode.*` entry is missing, `look` falls back to the top-level `description`.

### Item Templates

Minimal example:

```toml
title = "Torch"
description = """
A simple wall torch.
"""

[state.off]
description = """
An unlit torch is fixed to the wall.
"""

[state.on]
description = """
A lit torch flickers warmly against the stone wall.
"""
```

Items may define optional `state.*` overrides. These are used by `look` based on the current runtime state, for example:

- `state.off`
- `state.on`

If the current `state.*` entry is missing, `look` falls back to the top-level `description`.

### `title`

- Optional display name for the template.
- Useful for UI or future presentation systems.
- `look` itself only requires `description`.

### `description`

- Base fallback description used by `look`.
- Works even if no `mode.*` or `state.*` overrides are defined.

### Runtime Use

This template authoring metadata is currently used by `look` in:

- 2D gameplay
- 3D gameplay
- text gameplay

It is used when no explicit `on_look` message is defined for the target.

---

## Example

```toml
[startup]
welcome = """
Welcome to the Hideout Eldiron example!
"""
show = "room"

[sector_messages]
mode_2d = "cooldown"
mode_3d = "always"
cooldown_minutes_2d = 10
cooldown_minutes_3d = 10
show_on_startup = true
```

And for a specific sector:

```toml
title = "Crossroads"
show_in_2d = false
description = """
A small crossroads of worn earth and scattered stones, marking the meeting point between harbor, home, and garden.
"""
```
