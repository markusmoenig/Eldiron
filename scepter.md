# Eldiron Scepter

Eldiron Scepter is a proposed automation and remote-control layer for Eldiron
Creator. The goal is to make Creator programmable by AI assistants, scripts,
command-line tools, platform automation systems, and advanced users without
forcing those clients to edit `.eldiron` project files directly.

Scepter should make Eldiron feel more like a programmable worldbuilding IDE:
the app remains the source of truth for project state, undo, validation,
preview rendering, and saving, while external or in-app automation can request
well-defined authoring operations.

## Core Idea

Instead of exposing raw project internals as the main interface, Scepter should
provide a stable command layer:

- discover project structure, regions, tiles, assets, characters, and items
- inspect tile visuals and metadata
- paint 2D regions
- construct and decorate 3D spaces
- create named gameplay sectors
- place items, characters, props, lights, and triggers
- generate or modify Eldrin scripts
- generate procedural tiles
- preview, validate, undo, and apply changes

AI chat is one client of this system, not the only purpose. Scepter should also
serve scripting users, procedural-generation tools, test automation, build
pipelines, and future integrations.

## Naming

Suggested naming family:

- **Eldiron Scepter**: the automation and control system
- **Scepter Lorebook**: live command reference, schemas, examples, and help
- **Scepter Rituals**: saved multi-step workflows or macros
- **Scepter Sigils**: permissions, capabilities, or automation tokens
- **Eldiron Treasury**: existing/future asset and library system

Avoid `Codex` for the help catalog to prevent confusion with OpenAI naming.

## Architecture

Scepter should be built around a platform-neutral automation core:

```text
Automation Core
  Rust command structs
  validation
  undo integration
  project dirty state
  preview/apply flow
  stable JSON schemas

Adapters
  Eldrin automation bindings
  local HTTP/WebSocket JSON-RPC
  CLI: eldironctl or eldiron-scepter
  macOS AppleScript / Shortcuts bridge
  AI chat connector
  future plugin/tool integrations
```

The important design rule is that the **core commands** are the truth. JSON,
Eldrin, AppleScript, and AI chat are different ways to call the same command
registry.

## Why Not Direct `.eldiron` Editing?

Direct project editing is technically possible because `.eldiron` files are
serialized data, but it should not be the main automation workflow.

2D regions are not stored as a simple tile grid. They contain vertices,
linedefs, sectors, sector properties, layers, tile sources, region-local item
instances, region-local character instances, and runtime/editor metadata.

Direct editing would require external tools to maintain:

- sector and linedef consistency
- numeric IDs and UUIDs
- tile IDs and aliases
- generated layers
- item and character template references
- undo and dirty-state behavior
- live editor synchronization

Scepter lets Creator own those details.

Direct `.eldiron` import/export can still exist as a fallback or batch tool,
but live authoring should go through Creator.

## Intermediate Plans

AI should usually work in a semantic intermediate format before applying edits.
This keeps the AI thinking in gameplay and composition terms rather than raw
map internals.

Example region plan:

```json
{
  "region": "Goblin Hideout",
  "ops": [
    {
      "paint_rect": {
        "tile": { "kind": "floor", "style": "stone" },
        "rect": [4, 4, 20, 12]
      }
    },
    {
      "paint_outline": {
        "tile": { "kind": "wall", "style": "stone" },
        "rect": [4, 4, 20, 12]
      }
    },
    {
      "sector": {
        "name": "Guard Room",
        "polygon": [[4, 4], [24, 4], [24, 16], [4, 16]]
      }
    },
    {
      "place_character": {
        "template": "Goblin Guard",
        "at": [10, 8]
      }
    },
    {
      "place_item": {
        "template": "Iron Key",
        "at": [21, 13]
      }
    }
  ]
}
```

Creator can validate this, resolve tiles/templates, preview it, and then apply
it as an undoable change set.

## Visual Tile Control

Tags and aliases are useful, but AI should also be able to inspect visuals.
Scepter should expose both semantic and visual discovery:

- list tiles by role, alias, procedural metadata, and style
- render tile thumbnails
- render contact sheets
- inspect tile metadata
- preview tiles in context
- render region previews after draft edits

This enables a loop like:

1. AI asks for available `stone wall` tiles.
2. Creator returns metadata and a contact sheet.
3. AI chooses a tile visually.
4. AI paints a small draft.
5. Creator renders a region preview.
6. AI adjusts based on visual result.
7. User accepts the change.

## Tileset Import Automation

Tilesets are a strong Scepter use case because Creator already knows which
parts of a tileset were imported, while AI or scripts can help classify,
name, group, and tag the remaining cells.

Useful commands:

```text
tileset.list
tileset.inspect
tileset.grid_detect
tileset.list_unimported
tileset.import_tile
tileset.import_anim
tileset.import_multi
tileset.import_batch
tile.set_meta
tile_group.create
```

Example batch:

```json
{
  "command": "tileset.import_batch",
  "params": {
    "tileset": "dungeon_a.png",
    "grid_size": [32, 32],
    "imports": [
      {
        "kind": "tile",
        "cell": [4, 2],
        "meta": {
          "alias": "crypt_floor_cracked",
          "role": "Dungeon",
          "procedural_kind": "floor",
          "procedural_style": "crypt",
          "blocking": false
        }
      },
      {
        "kind": "multi",
        "rect": [8, 2, 2, 2],
        "name": "crypt_pillar_cluster",
        "tags": ["crypt", "pillar"]
      }
    ]
  }
}
```

This allows a loop like:

1. AI asks Creator for tilesets and imported coverage.
2. Creator returns visual sheets, grid data, and unimported cells.
3. AI proposes names, roles, blocking flags, procedural metadata, and groups.
4. Creator previews the import batch.
5. User accepts, edits, or rejects the batch.

## Scepter Lorebook

Scepter should include a built-in self-description system. This is important
for AI, scripts, external tools, and documentation.

Suggested commands:

```text
scepter.help
scepter.list_commands
scepter.describe_command
scepter.describe_schema
scepter.examples
scepter.capabilities
```

Example `describe_command` response:

```json
{
  "name": "region.paint_rect",
  "description": "Paint a rectangular area in a 2D region using a tile source.",
  "params": {
    "region": "region id or name",
    "tile": "tile id, alias, or tag query",
    "rect": "[x, y, width, height]",
    "layer": "optional generated layer name"
  },
  "previewable": true,
  "undoable": true,
  "examples": [
    {
      "region": "Harbor",
      "tile": { "alias": "stone_floor_dark" },
      "rect": [4, 4, 12, 8]
    }
  ]
}
```

The Lorebook should be generated from the actual command registry so the docs
do not drift from implementation.

The first local HTTP adapter exposes:

```text
GET  /scepter/ping
GET  /scepter/lorebook
POST /scepter/command
GET  /scepter/project
GET  /scepter/region
GET  /scepter/region/summary
GET  /scepter/tiles
```

`POST /scepter/command` accepts the same JSON shape used by the Rust
`ScepterCommand` enum, for example:

```json
{ "command": "tile.list", "params": {} }
```

Read commands can return live Creator state immediately. Write commands can be
listed in the Lorebook before they are executable, so clients can start using
the stable vocabulary while implementation fills in command by command.

Currently executable command groups include project/region/tile reads, PNG
region previews, per-cell 2D painting, Eldrin script reads/replacements, and
character/item attribute reads/patches.

## Eldrin And Scepter

Eldrin should be a first-class way to use Scepter, but it should not be the only
API surface.

Recommended split:

- **Runtime Eldrin**: character, item, world, and region behavior while the game
  runs
- **Creator Eldrin**: editor automation, batch authoring, content generation,
  macros, and Scepter commands

Creator automation scripts could look like:

```text
region("Harbor").paint_rect("stone_floor_dark", 4, 4, 12, 8)
region("Harbor").paint_outline("stone_wall", 4, 4, 12, 8)
region("Harbor").create_sector("Guard Room", [[4,4], [16,4], [16,12], [4,12]])
region("Harbor").place_character("Goblin", 9, 8)
preview()
```

This gives Eldiron users a native scripting experience while keeping the
underlying command model usable from JSON, CLI, AppleScript, AI tools, and other
languages.

Runtime scripts should not automatically get full editor permissions. Creator
automation scripts should run in an explicit authoring context with appropriate
permissions and undo behavior.

## macOS Automation

AppleScript and Apple events still exist on macOS and can be used as a platform
adapter. Scepter should not be based on AppleScript internally, but Creator can
expose AppleScript-friendly commands on macOS.

Example:

```applescript
tell application "Eldiron Creator"
  paint rect {4, 4, 12, 8} in region "Harbor" using tile "stone_floor_dark"
end tell
```

The same operation should also be available through JSON-RPC, CLI, and Eldrin.

## Procedural Tile Generation

Scepter should allow AI and scripts to create new tiles, not only choose from
existing ones.

Possible commands:

```text
tile.create_bitmap
tile.create_from_rgba
tile.create_from_palette
tile.create_from_tilegraph
tile.set_meta
tile.set_alias
tile.set_procedural_meta
tile.preview
tile.add_to_picker
tile.contact_sheet
```

AI workflow:

1. Generate a small tile image, preferably palette-aware.
2. Submit it to Creator.
3. Set alias, role, blocking state, style, and procedural kind.
4. Render a contact sheet or in-map preview.
5. Iterate or apply.

## Script Generation

Scepter should support AI-assisted Eldrin generation for characters, items,
regions, and world scripts.

Useful commands:

```text
script.get
script.patch
script.replace
script.validate
script.compile
script.explain_errors
script.examples
script.diff
```

The first executable script implementation supports these targets:

- `world`
- `region`
- `character`
- `item`

Character and item targets refer to project templates by default. Supplying a
`region` selects a placed region instance instead:

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

For now, `script.patch` applies a full replacement source and records the edit
in Creator's undo stack. Parser-backed Eldrin diagnostics are still planned.

Safe workflow:

1. AI asks for the current target script.
2. AI proposes a patch or replacement.
3. Scepter validates and compiles the script.
4. Creator shows a diff and any errors.
5. User accepts.
6. Creator applies the change with undo/history.

AI should not blindly overwrite scripts.

## Character And Item Attributes

Characters and items store authoring/runtime attributes as TOML in their
`[attributes]` table. Scepter exposes these without requiring callers to edit
the whole `.eldiron` file.

Useful commands:

```text
attributes.get
attributes.patch
```

Example template edit:

```json
{
  "command": "attributes.patch",
  "params": {
    "target": { "kind": "item", "name": "Sign" },
    "values": {
      "visible": true,
      "blocking": false,
      "description": "A weathered signpost."
    },
    "validate": true
  }
}
```

Example placed-instance edit:

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
      "dialogue_role": "lookout"
    },
    "remove": ["temporary_note"]
  }
}
```

These edits are undoable and refresh the runtime map content. This is the
foundation for AI-assisted NPC setup, item behavior, quests, and generated
scripts.

## 2D Region Commands

The first useful Scepter milestone should focus on 2D, because it can deliver a
strong demo quickly.

Possible 2D commands:

```text
project.describe
region.list
region.summary
region.snapshot
region.render_preview
region.paint_rect
region.paint_outline
region.paint_cells
region.erase_generated
region.create_sector
region.rename_sector
region.place_item
region.place_character
region.move_instance
region.delete_instance
region.validate
project.apply_plan
project.undo
project.redo
```

This is enough for AI or scripts to co-design a playable 2D area with rooms,
paths, NPCs, items, doors, and named gameplay sectors.

For day-to-day AI map understanding, `region.summary` is the preferred read
command. It returns compact, derived information instead of a raw map dump:

- bounds, counts, and layer distribution
- dominant sector tile roles and blocking/walkable counts
- sector and linedef source usage with resolved tile metadata
- procedural sector kinds
- named sectors with bbox, center, source, and optional data
- placed characters and items
- optional ASCII layout for small/medium maps

For reading and safely extending existing 2D maps, `region.snapshot` should
return a normalized view rather than raw sector data alone:

- region and map identifiers
- vertices with coordinates and properties
- linedefs with start/end vertices, sector ownership, wall/source properties
- sectors with polygon points, bbox, center, area, layer, and properties
- all `PixelSource` fields resolved into source kind plus tile/group IDs
- resolved tile summaries for tile IDs, including alias, role, blocking,
  frame count, visual size, and procedural metadata
- tile override and blend override entries as explicit cell arrays

This lets AI answer questions like: “which sectors use this floor tile,”
“which walls block movement visually,” “where can I expand the room,” and “what
tile style should I continue with” without reverse-engineering `.eldiron`
internals.

Current 2D map authoring notes:

- 2D coordinates use an origin where x increases right, negative y is up, and
  positive y is down.
- `source` is the primary current sector surface tile/material source.
- `ceiling_source` is not part of current 2D map authoring; it is only kept for
  screen/button selected-state usage.
- existing terrain-related properties should be treated as legacy/deprecated
  and should not drive new Scepter commands until the replacement terrain
  system is designed.

## 3D Construction

Scepter can also handle 3D construction, but it should probably come after the
first 2D automation milestone.

The main rule for 3D is to avoid exposing raw mesh editing as the first API.
Start with high-level construction commands:

```text
geometry.create_box
geometry.create_floor
geometry.create_wall
geometry.create_room
geometry.cut_door
geometry.cut_window
geometry.paint_face
geometry.paint_object
geometry.set_solid
geometry.set_area
geometry.place_prop
geometry.place_builder_asset
dungeon.paint_cell
dungeon.build
preview.render_3d
camera.set
geometry.inspect
collision.preview
```

Example:

```json
[
  {
    "command": "geometry.create_room",
    "region": "Dungeon",
    "name": "Crypt",
    "rect": [0, 0, 8, 6],
    "height": 3,
    "wall_tile": "stone_wall",
    "floor_tile": "stone_floor"
  },
  {
    "command": "geometry.cut_door",
    "object": "Crypt.north_wall",
    "at": [4, 0],
    "width": 1.5,
    "height": 2.2
  },
  {
    "command": "geometry.place_builder_asset",
    "asset": "Wall Torch",
    "at": [2, 0, 0.2],
    "on": "Crypt.west_wall"
  }
]
```

For 3D, visual feedback is critical. Scepter should be able to render 3D
previews, inspect selected geometry, validate collision, and expose camera
controls.

Terrain commands are intentionally deferred because the current terrain system
is deprecated and will be replaced.

## Preview And Apply

Scepter should support a preview/apply split:

```text
plan -> validate -> preview -> revise -> apply
```

This is useful for AI and non-AI automation alike. It lets the user see a draft
before committing it.

Generated changes should be grouped into undoable change sets. Where possible,
they should carry metadata such as:

- generator name
- source prompt or script
- generated layer
- timestamp
- command list
- whether the result is editable or regenerated

## Safety And Permissions

Scepter should be powerful, but not ambiently dangerous.

Recommended safety concepts:

- explicit automation mode
- per-command capabilities
- preview-only commands
- undoable changes by default
- user confirmation for destructive operations
- generated layers for AI/script output
- script validation before apply
- project dirty state managed by Creator
- optional Sigils for external client permissions

## First Write Command

The first executable 2D write commands are `region.paint_cells` and
`region.paint_rect`.

`region.paint_cells` is the preferred primitive. It paints exact 1x1 grid
cells using a resolved Creator tile id. By default it first clears drawable
tile sectors that overlap each target cell, then creates one new sector per
cell. This avoids the bad "large transparent rectangle on top" failure mode.

`region.paint_rect` is convenience syntax only. Internally it expands the
rectangle into individual cells and uses the same replace-before-paint path.

Tiles can be selected by exact id, alias, or role metadata, so an AI or script
can first read the region and tile catalog, then apply a structurally correct
change.

Example:

```json
{
  "command": "region.paint_rect",
  "params": {
    "region": { "name": "Harbor" },
    "rect": [-2, -8, 4, 3],
    "tile": { "role": "Road" },
    "layer": "1",
    "replace_existing": true,
    "select": false
  }
}
```

This currently applies directly to the open Creator project through the local
Scepter endpoint and is added to Creator undo as one map edit per command. It
should still be paired with preview, confirmation, and generated-layer removal
commands for larger generated plans.

## Visual Preview

`region.render_preview` returns a rendered top-down image for a region or
region bounds. The response includes a base64 encoded PNG image plus the map
bounds used for the render.

Example:

```json
{
  "command": "region.render_preview",
  "params": {
    "region": { "name": "Harbor" },
    "bounds": [-12, -24, 20, 16],
    "zoom": 2
  }
}
```

This makes the intended AI loop possible:

1. Read `region.summary` and `tile.list`.
2. Apply small `region.paint_cells` batches.
3. Fetch `region.render_preview` for the changed bounds.
4. Visually inspect the rendered image.
5. Iterate or undo before saving.

## Development Path

Suggested implementation stages:

1. **Internal command registry**
   Define Rust command structs, results, errors, validation, undo grouping, and
   generated Lorebook metadata.

2. **2D plan importer**
   Support paint rect, paint outline, create sector, place item, place
   character, preview, apply, and undo.

3. **Tile visual discovery**
   Add tile listing, metadata queries, thumbnails, and contact sheets.

4. **Local API adapter**
   Add JSON-RPC or WebSocket for external tools and AI clients.

5. **Creator Eldrin bindings**
   Let users call Scepter commands from Eldrin automation scripts.

6. **Script generation support**
   Add get, patch, validate, compile, diff, and apply for Eldrin scripts.

7. **Procedural tile generation**
   Add tile image creation, metadata assignment, preview, and tile picker
   insertion.

8. **3D construction**
   Add high-level room/object/face/prop commands with 3D previews.

9. **New terrain system**
   Add terrain commands only after the replacement terrain model exists.

10. **Platform adapters**
   Add CLI and macOS AppleScript/Shortcuts bridges.

## Why This Is Worth It

Scepter could become a defining Eldiron feature. It would let Creator be used
as an interactive worldbuilding environment rather than a closed editor.

It supports:

- AI-assisted region design
- classic scripting and automation users
- batch generation
- procedural tooling
- repeatable tests
- external asset workflows
- future plugin ecosystems

The most compelling version is not "generate a random dungeon." It is:

> Co-design a playable region with the editor, visually and structurally, while
> keeping the user in control.
