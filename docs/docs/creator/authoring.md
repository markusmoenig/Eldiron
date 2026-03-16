---
title: "Authoring"
sidebar_position: 3
---

Authoring in **Eldiron Creator** adds narrative and descriptive metadata to the world. It is used by the terminal client already, and the same data can also drive room descriptions, sector enter text, and other presentation in normal 2D and 3D games.

## Authoring Dock

The tool strip contains an **Authoring** toggle.

When it is enabled, contexts that would normally show the **Tiles** dock show the **Authoring** dock instead.

This makes Authoring a persistent mode:

- geometry and selection tools still work normally
- tile-backed contexts switch from **Tiles** to **Authoring**
- other docks like **Data**, **Code**, **Visual Code**, or **Console** are unaffected

## What You Can Edit

The Authoring dock currently edits TOML metadata for selected:

- sectors
- linedefs
- entity instances
- item instances

This metadata is stored with the selected object and is intended for player-facing text.

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
