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
- future scripting, plugin, or platform automation adapters

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
GET  /scepter/job?request_id=...
DELETE /scepter/job?request_id=...
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

## Shared editing and request status

Read `GET /scepter/project` before editing. Its `session` identifies the open
tab and its `revision` identifies authoring state. Every mutation, including
undo/redo, script edits, action execution and tool selection, requires these
fields plus a unique `request_id` at the top level:

```json
{
  "request_id": "road-edit-001",
  "session": "<session from project.describe>",
  "revision": "<revision from project.describe>",
  "command": "project.transaction",
  "params": { "operations": [] }
}
```

The empty operation list above is a placeholder; a transaction needs 1–128
operations. The other command examples below show command/parameter bodies;
add the three envelope fields when sending a mutation. Missing context returns
HTTP 428. A changed revision or another active tab returns HTTP 409 without
applying the edit. Responses include the resulting session and revision.
Camera movement, selection and completed MZ bakes do not invalidate context;
undo/redo and authored material, palette, bitmap and map edits do.

A repeated request ID with the same payload returns the original job/result.
Reusing that ID with a changed payload returns HTTP 409. Mutation IDs are
retained for the service lifetime. Read results may be reclaimed after 60
seconds. Long requests return HTTP 202 (`queued` or `running`), rather than
reporting failure while an edit is still pending:

```text
GET /scepter/job?request_id=road-edit-001
DELETE /scepter/job?request_id=road-edit-001
```

DELETE returns HTTP 200 when it cancels queued work; cancelled requests subsequently report HTTP 409 with
`status: "cancelled"`. A running edit cannot be cancelled halfway through.
Poll it before deciding on another edit. Handler failures have top-level
`ok: false` and a non-success HTTP status.

The loopback service allows 16 simultaneous connections, 16 KiB headers, 8 MiB
request bodies and 16 MiB results. Requests have a five-second read deadline.
Retained requests/results have a 64 MiB serialized-data budget and 2048 entries;
capacity errors are explicit. Restarting Creator begins new session IDs.

## MZ materials and map compositions

- `material.list`: project palette indices, material IDs, composition bindings
  and source definitions.
- `material.get`, with `{ "id": "UUID" }`: the editable MZ bundle, composition
  definition and optional surface binding.
- `material.validate`, with `{ "bundle": "..." }`: validate a whole PBR bundle
  against the project palette and return the normalized bundle.
- `composition.get`, with `{ "id": "UUID" }`: editable layer stack and binding.
- `surface.list`, with `{ "region": { "id": "UUID" } }`: sectors, linedefs,
  geometry objects/faces, walls, lights and composition bindings.

Edits use `project.transaction` with `operations: [{ "op": "...", "params": {} }]`:

| Operation | Parameters |
| --- | --- |
| `material_put` | `id`, `bundle`; root document ID must match the tile UUID |
| `composition_put` | `id`, `composition`; creates or replaces a layer stack |
| `composition_copy` | new `id`, existing `source`; copies the editable stack |
| `surface_apply` | `region`, `surface`, `material`; returns the placement material ID |
| `paint_cells` | same parameters as `region.paint_cells` |
| `paint_rect` | same parameters as `region.paint_rect` |

Material bundles are the same versioned documents used by the procedural
editor. Color parameters persist project palette-index bindings, and linked
Image / Tile Sources refresh through the existing editor service. The supported
outputs are Color, Roughness, Metallic, Opacity and Emission; BSDF is excluded.

A composition contains `base` (material/tile UUID), `layers` and
`tiling: { "mode": "raw", "period": [1,1], "blend": 0.15 }`. `raw` uses
continuous world UVs; `repeat` and `seamless` use bounded repetition. Every layer
has its own UUID and material source; shapes include `Split`, `Road`, `Crossing`,
`AutoRoad`, `Corner`, `Strips`, `Blend`, `Mask`, `Height`, `Overlay` and `UvPosition`.
Layers retain the editor's `position`, `angle`, `feather`, `amount`, `wobble`,
`wobble_scale`, `mask`, `blend`, `overlay_level`, `uv_direction`, `uv_levels`
and `extra_materials` fields. Start from `composition.get` to preserve these
fields when editing. Sources can be MZ materials, ordinary bitmap tiles or
other compositions; cycles and missing sources are rejected.

Surface selectors use the existing externally tagged editor representation:

```json
{
  "op": "surface_apply",
  "params": {
    "region": { "id": "<region UUID>" },
    "surface": { "GeometryFace": { "object": "<object UUID>", "face": 0 } },
    "material": "<material or composition UUID>"
  }
}
```

Other selectors include `Sector: {id, ceiling}`, `Linedef: {id, row}` (rows 1–4),
`RectTile: {sector}`, `SectorTile: {sector, cell}`, `GeometryTile: {object, face,
cell}`, `TerrainTile: {cell}`. Use `surface.list` to
inspect the stable target IDs. Applying creates a placement-owned composition
and UV cache while retaining shared source materials. Editing that returned
composition ID adjusts that surface; applying a copied stack elsewhere avoids
rebuilding the road by hand.

Transactions stage every edit, validate referenced assets and regenerate world
UV coverage/dependent compositions before publication. Failure discards all
staged changes. A successful transaction saves as normal project data and adds
one undo step. `dry_run: true` validates and reports results without publishing
or changing undo. A transaction permits at most 4096 painted cells and 32
layers per composition. Action scripts remain ordinary action sequences; they
are not this atomic transaction API.

## Working together in the map editor

`editor.context` reads the active region, map, tool, viewport dimensions, cursor,
camera and selection, plus the change-feed `stream` and `cursor_sequence`.
Reading context leaves the user's view alone. `editor.select` and `editor.focus`
change that shared view explicitly, require the mutation envelope, and add no
undo record:

```json
{
  "command": "editor.select",
  "params": {
    "region": { "name": "Harbor" },
    "targets": { "geometry": ["<object UUID>"] },
    "faces": [["<object UUID>", 0]]
  }
}
```

`targets` can contain `sectors` (integer IDs), `geometry`, `characters`
and `items` (UUID lists). Selection also accepts `vertices`, `linedefs`, `faces`,
`geometry_vertices` (`[object UUID, vertex index]` pairs). Only one character or item can be the active entity. IDs are checked
before changing selection. Select a tool separately through `tool.select` when
running a contextual action such as `face.extrude`.

```json
{
  "command": "editor.focus",
  "params": {
    "region": { "name": "Harbor" },
    "view": "iso",
    "center": [8, 0, 6],
    "scale": 12
  }
}
```

Focus views are `2d`, `iso`, `orbit` and `firstp`; optional `position` sets the
3D camera anchor. This uses Creator's normal view switching. For a preview of
the shared camera, send `region.render_preview` with `editor_view: true` and
optional `size`. It requires the active region map and captures the actual
viewport camera, including fractional 2D zoom, isometric angles and perspective
FOV/pitch. Its response reports the effective camera. Ordinary explicit-camera
previews remain independent of the editor view.

Poll `project.changes` to follow the person working beside you:

```json
{
  "command": "project.changes",
  "params": { "stream": "<stream UUID>", "after": 12, "limit": 64 }
}
```

Keep the returned `stream` and `next` cursor. `events` reports coalesced content,
view/selection and session changes. `has_more` means another page is available.
When `reset_required` is true, reread context and relevant map/material snapshots
and resume from `next`. The in-memory feed retains 256 events and allows 1–128
per response. While a client is active, Creator samples changes every 200 ms;
tracking becomes idle after two minutes without a context/feed read. This is a
bounded synchronization feed, not a record of every mouse movement.

## Transactional map and entity authoring

The same `project.transaction` can combine materials/compositions, map geometry
and entity instances. Creator prepares it on a worker, then rechecks the session
and revision before publishing. Concurrent human edits produce HTTP 409 and
leave the live project intact. Only one transaction prepares at a time; another
gets a busy conflict. Pending work uses the normal job polling protocol. Camera
and selection changes made during preparation are preserved, as are completed
bakes of unchanged materials. Publication and undo refresh only affected project
tree rows.

All 3D points are `[x,y,z]`, with Y up. 2D polygons/cells use `[x,z]`. Angles are
degrees. Region references use `{ "id": "UUID" }` or `{ "name": "unique name" }`.
Material selectors use a unique tile `id`, `alias`, or matching role/metadata.
New geometry, wall and entity operations require a caller-generated UUID in
`id`; results return generated face/span/sector IDs for subsequent edits.

| Operation | Parameters |
| --- | --- |
| `sector_create` | `region`, `name`, `polygon: [[x,z],…]`; optional `floor`, `ceiling`, `height`, `layer` |
| `sector_update` | `region`, integer `id`; optional `name`, `floor_height`, `height`, `layer` |
| `erase_cells` | `region`, `cells: [[x,z],…]`; optional integer `layer` |
| `map_transform` | `region`, `targets`; optional `translation`, `pivot`, `rotation_y`, `scale` |
| `map_copy` | same transform fields; returns fresh IDs for the copies |
| `map_delete` | `region`, `targets` |
| `geometry_create` | `region`, `id`, `name`, `shape` (`box`, `cylinder`, `floor`, `ceiling`), `min`, `max`; optional `material`, cylinder `segments` |
| `entity_place` | `region`, `id`, `kind` (`character`/`item`), `template`, `position`; optional `name`, instance TOML `data`, `active` |
| `entity_update` | `region`, `id`, `kind`; optional `name`, `position`, character `orientation: [x,z]`, instance TOML `data`, `active` |

Erasing part of a larger floor stores per-cell overrides and preserves the rest
of the floor and other layers. Sector moves detach shared boundaries instead of
dragging neighboring sectors. Copying preserves geometry effects, surface compositions and instance configuration. Placement-owned
composition IDs are copied; source materials remain shared. Surfaces with per-cell overrides support whole-cell translations,
quarter turns and unit horizontal scale. Sector floor-height edits reject shared
vertices; detach/move the sector first when changing its level independently.

`entity.templates` lists existing project/ruleset characters and items with IDs
and setup TOML. `region.entities` reads placed instances. `template` resolves a
unique UUID, name or ruleset identifier/path. Placement retains the template's
MZ appearance, rig, clothing, equipment and independent on/off appearance states.
Instance `data` is TOML synchronized with the normal entity configuration graph;
`active` sets its normal attribute. Scepter does not create replacement character
or item assets just to place them. Item orientation is not supported by the
current instance model and is rejected explicitly.

For example, create a room and a lit ruleset torch in one undo step:

```json
{
  "command": "project.transaction",
  "params": {
    "operations": [
      {
        "op": "geometry_create",
        "params": {
          "region": { "name": "Harbor" },
          "id": "<new object UUID>",
          "name": "Workshop wall",
          "shape": "box",
          "min": [4, 0, 4], "max": [10, 3, 4.25],
          "material": { "alias": "Walls" }
        }
      },
      {
        "op": "entity_place",
        "params": {
          "region": { "name": "Harbor" }, "id": "<new instance UUID>",
          "kind": "item", "template": "Torch",
          "position": [6, 0, 3], "active": true
        }
      }
    ]
  }
}
```

Use `dry_run: true` first when exploring a larger edit. Coordinates must be
finite and within ±1,000,000; primitive extents are at most 4096, polygons
have at most 256 points, cylinders at most 128 segments, and instance TOML
at most 64 KiB. A transaction
allows 128 operations and a combined 4096 painted/erased cells.

The native acceptance test exercises geometry, a composed road, a ruleset
character and active torch in copies of Hideout2D-MZ and 3D Starter, with 2D/3D
previews, save/reopen and one-step undo/redo. Run it on a machine with a native GPU:

```bash
cargo test -p eldiron-creator --lib scepter_world_workflow_in_hideout_and_3d_starter -- --ignored --test-threads=1 --nocapture
```

## Lorebook

The catalog reports `implemented` for every command. Planned commands return
HTTP 501. MZ material and composition editing, atomic transactions and engine
previews are implemented. Map selection/cameras, primitives, sectors,
transforms and ruleset entity placement are implemented. Direct appearance/rig
and arbitrary node-graph authoring remain follow-up work.

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

For a sequence, Scepter can run a small editor-automation script:

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
    "view": "2d",
    "size": [768, 768]
  }
}
```

Previews use the actual SceneVM renderer and the same MZ GPU bake worker as
Creator, including surface UV pages, animation frames, packed PBR channels,
project palette and global pixelization. They render an owned project snapshot
without changing the user's map selection or camera. The offscreen renderer and
bounded material cache are reused between requests.

Use `view: "2d"`, `"iso"` or `"3d"`; `size: [768, 512]` sets output dimensions
(each axis 64–1024). Bounds use `[x, z, width, depth]`, with positive extents up
to 256 units. Optional `camera` contains `center: [x,y,z]`, `position: [x,y,z]`
(for perspective), `scale`, `azimuth`, `elevation` (for isometric), and `fov`,
`pitch` (for perspective). `zoom` is pixels per map unit in
2D. `animation_frame` selects an engine animation tick. Lighting uses the
snapshot's project time. PNG data is at `preview.png_base64`; `preview.ready`
is true only after GPU bakes and scene chunk preparation finish. The returned
session/revision identifies the captured authoring state.

```json
{
  "command": "region.render_preview",
  "params": {
    "region": { "name": "Harbor" },
    "view": "iso",
    "size": [768, 512],
    "camera": { "center": [0, 0, 0], "scale": 12 },
    "animation_frame": 0
  }
}
```

A preview taking more than two seconds returns HTTP 202 and a `request_id`.
Poll its job endpoint for the finished PNG. Only one preview renders at a time;
a simultaneous preview gets an explicit busy error. The project continues to
be editable while the background preview runs.

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

`replace_existing` defaults to `true`. Existing drawable content in the target
cells/layer is replaced. Partial overlaps with larger floors use cell overrides
so neighboring terrain is preserved.

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
tileset.inspect
tileset.list_unimported
tileset.import_batch
```

These commands are meant to help automate tileset import, tagging, grouping,
blocking flags, and procedural metadata.

## Stored Script Source

These APIs edit source fields, not Behavior Node graphs. Use the Nodes dock to author runtime behavior.

Scepter can read and replace stored script source for:

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
records the change in Creator's undo stack. Parser-backed source diagnostics
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

Scepter edits mark the project dirty and use Creator undo. A complete
`project.transaction` is one undo step. Supply a fresh request ID and the current
session/revision for undo or redo too.

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
