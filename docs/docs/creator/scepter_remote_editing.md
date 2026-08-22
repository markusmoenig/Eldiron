---
title: "Scepter: Remote Editing"
sidebar_position: 4
---

**Eldiron Scepter** is Creator's local automation and remote editing system. It
lets tools outside the normal graphical UI ask Creator to inspect, preview, and
change the current project through structured commands.

The important idea is that Scepter does **not** make external tools edit
`.eldiron` files directly. Creator remains the source of truth for project
state, undo, validation, previews, dirty state, and future save behavior.
External clients send authoring commands; Creator applies them as normal editor
operations.

Scepter is meant for:

- AI assistants that help build regions, tiles, characters, items, and scripts
- user scripts and command-line tools
- procedural generation workflows
- testing and automation
- future Eldrin, plugin, or platform automation adapters

## Local API

The first Scepter adapter is a local JSON API exposed by Creator:

```text
http://127.0.0.1:37687
```

The API is local to the machine. It is intended for tools running beside
Creator, not for remote network access.

Useful endpoints:

```text
GET  /scepter/ping
GET  /scepter/lorebook
GET  /scepter/project
GET  /scepter/region
GET  /scepter/region/summary
GET  /scepter/tiles
POST /scepter/command
```

`/scepter/command` accepts the stable Scepter command shape:

```json
{
  "command": "region.summary",
  "params": {
    "region": { "name": "Harbor" }
  }
}
```

## Lorebook

Scepter includes a live command reference called the **Scepter Lorebook**. It is
intended to be readable by humans and by automation clients.

Use it to discover:

- command names
- parameters
- examples
- capabilities
- whether a command is previewable
- whether a command is undoable

Fetch it with:

```bash
curl http://127.0.0.1:37687/scepter/lorebook
```

Or ask for a command list:

```json
{
  "command": "scepter.list_commands"
}
```

The Lorebook is the source of truth for the Scepter protocol: command names,
parameter schemas, capabilities, and examples. It is not a frozen copy of every
action or tool installed in Creator. Use `action.list` and `tool.list` for that
live runtime catalog, including contextual availability. The hand-written
documentation explains concepts and workflows; automation clients should
discover details from these machine-readable sources.

## Creator Actions And Tools

Creator distinguishes actions from tools:

- an **action** is a contextual operation such as changing camera, extruding a
  face, or starting a bake
- a **tool** is a long-lived interaction mode such as Geometry, Vertex,
  Linedef, Face, or 3D Paint

Creator's interactive [Console](console) uses these same registries. Its short
commands are intended for people at the editor, while Scepter provides the
structured JSON boundary for external clients.

The sidebar's visible action labels are localized and grouped, for example
**Camera: 3D Iso Camera**. Automation uses stable, non-localized IDs such as
`camera.isometric`; it must not depend on the visible label.

List all actions, or only the currently applicable Face actions:

```json
{
  "command": "action.list",
  "params": {
    "group": "face",
    "applicable_only": true
  }
}
```

Run an action through its normal applicability, parameter, undo, and project
hooks:

```json
{
  "command": "action.run",
  "params": {
    "id": "face.extrude",
    "parameters_toml": "amount = 2"
  }
}
```

Action parameters use the same TOML representation as the Actions sidebar.
Omit `parameters_toml`, or send an empty string, for an action without
parameters.

List or select interaction tools:

```json
{
  "command": "tool.list",
  "params": {
    "include_hidden": false
  }
}
```

```json
{
  "command": "tool.select",
  "params": {
    "id": "tool.geometry"
  }
}
```

`tool.select` uses the same deactivate/activate lifecycle as selecting a tool
in the UI. An unavailable action or tool is rejected instead of bypassing the
current editor context.

For a sequence, Scepter can run a small Eldrin editor-automation script:

```json
{
  "command": "action.run_script",
  "params": {
    "source": "editor_tool(\"tool.geometry\");\neditor_action(\"camera.isometric\", \"\");\neditor_action(\"face.extrude\", \"amount = 2\");"
  }
}
```

The operations execute in source order on Creator's UI thread. Each successful
action keeps its normal undo entry, and execution stops at the first error. The
script is therefore an ordered convenience layer, not an atomic transaction.

The relevant Scepter capabilities are `action_read`, `action_execute`,
`tool_read`, and `tool_select`. Commands that change project content also
declare `project_write`.

## Project And Region Reads

Clients should start by reading the project instead of guessing names and IDs:

```bash
curl http://127.0.0.1:37687/scepter/project
```

This returns the project path, dirty state, current region, region list, and
top-level character/item templates.

For 2D maps, prefer `region.summary` first. It returns compact map information
that is easier for humans and AIs to reason about:

```json
{
  "command": "region.summary",
  "params": {
    "region": { "name": "Harbor" },
    "include_ascii": true
  }
}
```

Use `region.snapshot` when a client needs lower-level structure such as
vertices, linedefs, sectors, tile sources, placed characters, and placed items:

```json
{
  "command": "region.snapshot",
  "params": {
    "region": { "name": "Harbor" },
    "include_tiles": true
  }
}
```

2D coordinate notes:

- x increases to the right
- negative y is up
- positive y is down
- `source` is the current primary sector tile/material source
- `ceiling_source` is legacy for screen/button selected-state usage and should
  not drive 2D map authoring

## Preview Loop

Remote editing should usually follow this loop:

```text
read -> plan -> apply small change -> render preview -> inspect -> revise
```

Scepter can render a compressed PNG preview of a region:

```json
{
  "command": "region.render_preview",
  "params": {
    "region": { "name": "Harbor" },
    "bounds": [-14, -24, 35, 36],
    "zoom": 2
  }
}
```

The response contains a base64 PNG image and metadata such as width, height,
bounds, and cell size. This is useful for AI clients because they can visually
inspect their own edits instead of relying only on tile tags.

## 2D Painting

2D map painting is done through tile/grid commands. The current implementation
creates per-cell sectors, so generated edits remain inspectable and undoable.

Paint individual cells:

```json
{
  "command": "region.paint_cells",
  "params": {
    "region": { "name": "Harbor" },
    "tile": { "alias": "stone_floor_dark" },
    "cells": [[4, -8], [5, -8], [6, -8]],
    "replace_existing": true
  }
}
```

Paint a rectangle:

```json
{
  "command": "region.paint_rect",
  "params": {
    "region": { "name": "Harbor" },
    "tile": { "role": "Road" },
    "rect": [4, -10, 8, 3],
    "replace_existing": true
  }
}
```

`replace_existing` defaults to `true`. This clears existing drawable sectors
that overlap the target cells before painting, which avoids stacking new tiles
over old ones.

## Tiles

Tags and roles help automation choose tiles, but visual control is also
important. Clients can list tile metadata and later use preview/contact sheet
commands to choose tiles by appearance.

```json
{
  "command": "tile.list",
  "params": {
    "role": "Road"
  }
}
```

Useful tile authoring command groups planned for Scepter include:

```text
tile.create_from_rgba
tile.set_meta
tile_group.create
tileset.inspect
tileset.list_unimported
tileset.import_batch
```

These commands are meant to help automate tileset import, tagging, grouping,
blocking flags, and procedural metadata.

## Scripts

Scepter can read and replace Eldrin source for:

- world scripts
- region scripts
- character templates
- item templates
- placed character instances
- placed item instances

Read a script:

```json
{
  "command": "script.get",
  "params": {
    "target": {
      "kind": "item",
      "name": "Sign"
    }
  }
}
```

Read a placed character instance script:

```json
{
  "command": "script.get",
  "params": {
    "target": {
      "kind": "character",
      "region": { "name": "Harbor" },
      "name": "Old Smuggler"
    }
  }
}
```

Patch a script:

```json
{
  "command": "script.patch",
  "params": {
    "target": {
      "kind": "item",
      "name": "Sign"
    },
    "patch": "on examine {\n    say(\"The sign points toward the harbor.\")\n}",
    "validate": true
  }
}
```

The first executable version of `script.patch` replaces the full source and
records the change in Creator's undo stack. Parser-backed Eldrin diagnostics
are planned for a later pass.

## Attributes

Characters and items store gameplay attributes as TOML under an `[attributes]`
table. Scepter can read and patch that table without making clients rewrite the
whole project file.

Read attributes from a character template:

```json
{
  "command": "attributes.get",
  "params": {
    "target": {
      "kind": "character",
      "name": "Orc"
    }
  }
}
```

Patch attributes on a placed character instance:

```json
{
  "command": "attributes.patch",
  "params": {
    "target": {
      "kind": "character",
      "region": { "name": "Harbor" },
      "name": "Harbor Lookout"
    },
    "values": {
      "faction": "dock_watch",
      "dialogue_role": "lookout",
      "visible": true,
      "radius": 0.5
    },
    "remove": ["temporary_note"],
    "validate": true
  }
}
```

JSON values are converted to TOML values. Use `remove` to delete keys.

## Undo And Redo

Scepter edits are intended to behave like Creator edits. They mark the project
dirty and are grouped into undoable operations where possible.

```json
{
  "command": "project.undo"
}
```

```json
{
  "command": "project.redo"
}
```

This is especially important for AI-assisted workflows: a client can try a
small edit, render a preview, and undo or revise if the result is not right.

## Design Guidelines For Clients

Remote clients should follow these rules:

- read the project and current region before changing anything
- prefer semantic selectors such as names, aliases, roles, and tags
- use `region.summary` before `region.snapshot` when possible
- apply small batches instead of huge blind edits
- render previews after visual changes
- use `replace_existing: true` for 2D map painting unless layering is intended
- patch scripts and attributes only after reading the current value
- use undo instead of saving test edits

Scepter is powerful because it makes Creator programmable. It should still act
like an editor: inspect first, change carefully, preview often, and keep the
user in control.
