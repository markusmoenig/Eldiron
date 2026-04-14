---
title: "Authoring"
sidebar_position: 3
---

<div style="position:relative;padding-bottom:56.25%;height:0;overflow:hidden;margin-bottom:1rem;">
  <iframe
    src="https://www.youtube.com/embed/YLHWBTcqfps"
    title="Eldiron Authoring"
    style="position:absolute;top:0;left:0;width:100%;height:100%;"
    frameborder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
    referrerpolicy="strict-origin-when-cross-origin"
    allowfullscreen>
  </iframe>
</div>

Authoring in **Eldiron Creator** adds narrative and descriptive metadata to the world. It is used by the terminal client already, and the same data can also drive room descriptions, sector enter text, and other presentation in normal 2D and 3D games.

## Authoring Dock

The tool strip contains an **Authoring** toggle.

When it is enabled, contexts that would normally show the **Tiles** dock show the **Authoring** dock instead.

This makes Authoring a persistent mode:

- geometry and selection tools still work normally
- tile-backed contexts switch from **Tiles** to **Authoring**
- other docks like **Data**, **Code**, **Visual Code**, or **Console** are unaffected

## What You Can Edit

The Authoring dock edits player-facing TOML metadata for:

- selected sectors
- selected linedefs
- selected character templates
- selected item templates

Important:

- sectors and linedefs are authored from the current region selection
- characters and items are authored on their templates, not on placed instances
- gameplay/mechanical TOML still belongs in the normal `Data` dock

So the split is:

- `Authoring`: descriptive and presentation text
- `Data`: stats, flags, input, rules-related values, and other mechanics

## Minimal Format

The current starter template is:

```toml
title = ""
description = """
"""
```

This is shown automatically for empty selections so the expected format is always visible.

Use:

- `title` for the player-facing name of the place, connection, entity, or item
- `description` for the longer descriptive text

`title` is optional for `look`. If only `description` is present, that still works.

Examples:

```toml
title = "Your ship"
description = """
The familiar deck of your faithful ship creaks softly beneath your feet. It has carried you through many adventures, and still feels more like home than any harbor.
"""
```

```toml
title = "Crossroads"
description = """
A small crossroads of worn earth and scattered stones, marking the meeting point between harbor, home, and garden.
"""
```

## How It Is Used

Right now the authoring metadata is already used by:

- text-style terminal room titles and descriptions
- text-style exit and room presentation
- authored sector description messages in regular gameplay
- `look` for characters and items in 2D, 3D, and text play when no explicit `on_look` message is present

## Character And Item Authoring

Character and item templates can define descriptive fallbacks used by `look`.

Characters support mode-based overrides:

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

Items support state-based overrides:

```toml
title = "Torch"
description = """
A simple wall torch.
"""

[state.off]
description = """
An unlit torch is fixed to the wall.
"""
on_use = "You light the torch."

[state.on]
description = """
A lit torch flickers warmly against the stone wall.
"""
on_use = "You extinguish the torch."
```

Resolution order is:

1. matching `mode.<value>` for characters
2. matching `state.<value>` for items
3. fallback to the top-level `description`

So `mode.*` and `state.*` are optional overrides, not required fields.

For items, `state.*` can also carry simple use text like `on_use`.
That authored `on_use` message is used as a fallback in text, 2D, and 3D play when no explicit item `on_use` behavior overrides it.

For sectors, you can also add:

```toml
show_in_2d = false
show_in_3d = false
```

to suppress automatic sector description messages per view mode.

## Global Authoring Settings

Global authoring and text-presentation behavior lives under **Game / Authoring**.

That configuration controls things like:

- startup welcome text
- startup room/description behavior
- exit presentation style
- terminal colors
- auto-attack behavior for text-style clients
- sector description message policy

See [Authoring Configuration](../configuration/authoring) for the full reference.
