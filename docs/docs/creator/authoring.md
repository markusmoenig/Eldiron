---
title: "Authoring"
sidebar_position: 3
---

<div style={{position: 'relative', paddingBottom: '56.25%', height: 0, overflow: 'hidden', marginBottom: '1rem'}}>
  <iframe
    src="https://www.youtube.com/embed/YLHWBTcqfps"
    title="Eldiron Authoring"
    style={{position: 'absolute', top: 0, left: 0, width: '100%', height: '100%'}}
    frameBorder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
    referrerPolicy="strict-origin-when-cross-origin"
    allowFullScreen>
  </iframe>
</div>

Authoring in **Eldiron Creator** adds narrative and descriptive metadata to the world. It is used by the terminal client already, and the same data can also drive room descriptions, sector enter text, and other presentation in normal 2D and 3D games.

## Authoring Dock

The tool strip contains an **Authoring** toggle.

When it is enabled, contexts that would normally show the **Tiles** dock show the **Authoring** dock instead.

This makes Authoring a persistent mode:

- geometry and selection tools still work normally
- tile-backed contexts switch from **Tiles** to **Authoring**
- clicking Authoring opens its dock from any tool context; the sidebar **Console** remains independently available

## What You Can Edit

The Authoring dock edits player-facing TOML metadata for:

- selected sectors
- selected linedefs
- selected 3D Geometry Objects
- selected linked Prefab instances
- placed characters and items, selected on the map or in the region tree
- selected character templates
- selected item templates

Important:

- sectors, linedefs, and Geometry Objects are authored from the current region selection
- linked Prefab instances edit their own authoring; blank descriptions inherit the shared asset description
- placed characters and items keep their own authoring, without requiring an extra global template
- character and item templates supply reusable defaults; local descriptions override those defaults
- character and item configuration belongs in **Entity Nodes**, and gameplay logic in **Behavior Nodes**

So the split is:

- `Authoring`: descriptive and presentation text
- `Entity Nodes` and `Behavior Nodes`: configuration, input mappings, rules-based actions, and gameplay logic

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
- `look` for characters and items in 2D, 3D, and text play when no behavior response supplies a description

## Author a placed door or character

1. Enable the **Authoring** toggle in the tool strip.
2. Use **Object** mode for a placed item, or the **Entity** tool for a character or item on the map,
   or select its entry beneath the current region in the project tree.
3. Edit the displayed TOML in the **Authoring** dock. The dock title identifies
   the selected placement. You can switch tools while keeping Authoring enabled.
4. Play, choose **Look**, and click the object. Its description appears above it.

The authoring is stored with that placement, saved in the project and copied when
that placement is duplicated. It does not add a door to the global Items list or
change the MZ recipe. Undo/Redo applies separately to each authored target.
Blank local descriptions inherit template defaults. A placement can override one
state and leave other states inherited. The template remains useful for shared
content, but a new template is not required just to describe a particular door.

```toml
title = "Inn Door"
description = "Weathered oak planks bound with iron bands."

[state.off]
description = "The oak door is shut. Its iron handle is worn smooth."

[state.on]
description = "The door stands open, revealing the quiet inn beyond."
```

World objects do **not** reveal descriptions or names on hover. Inventory,
equipment and toolbar UI retain their tooltips. See [Look and World Text](../characters_items/world_text)
for reading, dismissing and paging descriptions in 2D, iso and 3D.

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

## Optional AI Text Input

The terminal client can use the same authored rooms, exits and intents with optional
AI interpretation. See [AI for Text Play](../clients/text_ai.md) for account setup,
provider options and cancellation. No extra authoring mode is needed.
