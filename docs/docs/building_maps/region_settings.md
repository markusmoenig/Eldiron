---
title: "Region Settings"
sidebar_position: 5
---

Region Settings are stored as TOML and control region-level editor/runtime behavior.

Runtime logic for a region is edited separately from this TOML file:

- **Region / Visual Scripting**
- **Region / Eldrin Scripting**

Use the region scripts for dynamic runtime overrides such as local fog, palette remap, or post changes. Use **Region Settings** for authored/static map settings.

## Terrain

Enable terrain rendering for the region and define the default terrain tile:

```toml
[terrain]
enabled = true
tile_id = "27826750-a9e7-4346-994b-fb318b238452"
```

- `enabled`: turns terrain on/off for the region.
- `tile_id`: default tile used for terrain rendering.
  - accepts UUID, tile alias, or palette index.
  - examples: `tile_id = "27826750-a9e7-4346-994b-fb318b238452"`, `tile_id = "grass_default"`, `tile_id = 2`, `tile_id = "2"`.

## Procedural

The `[procedural]` section drives the **Build Procedural** action. The first generator is `connected_rooms`, which creates a deterministic 2D room-and-corridor dungeon from the region seed and the procedural tile metadata authored with **Edit Tile Meta**.

For a full workflow guide, see [Procedural Map Generation](/docs/building_maps/procedural_generation).

```toml
[procedural]
enabled = true
generator = "connected_rooms"
mode = "2d"
seed = 12345
style = "stone"
width = 32
height = 32
room_count = 6
room_min_size = 6
room_max_size = 10
door_placement = "both"
door_randomness = 1.0

[procedural.items.door]
names = ["Door"]
weights = [1]

[procedural.characters.skeleton]
names = ["Skeleton"]
weights = [1]
percentage = 35
```

- `enabled`: if `false`, **Build Procedural** does nothing.
- `generator`: currently supports `connected_rooms`.
- `mode`: `2d` builds tile-map geometry. `3d` is reserved for the later Dungeon Tool-backed generator.
- `seed`: makes the generated layout deterministic. Reusing the same seed and assets recreates the same dungeon.
- `style`: selects tiles whose **Edit Tile Meta** procedural style matches this value. If no matching style exists, the generator can fall back to any procedural tile of the required kind.
- `width` / `height`: generated grid size. Values are clamped to the supported editor range.
- `room_count`: target number of rooms in the connected path.
- `room_min_size` / `room_max_size`: room size range in tiles.
- `door_placement`: `entrances`, `exits`, or `both`.
- `door_randomness`: probability from `0.0` to `1.0` after `door_placement` filtering. If no door is placed, the connection remains passable floor.

These settings can also be changed from scripts through `region.procedural.*` context paths before calling `build_procedural()`. `build_procedural(0)` maintains `region.procedural.run` internally to advance to the next deterministic layout. `region.procedural.rooms` is accepted as an alias for `region.procedural.room_count`, and character `percent` is accepted as an alias for `percentage`.

Tiles are selected from tile metadata:

```toml
[procedural]
style = "stone"
kind = "floor"
weight = 1
```

Supported tile `kind` values are `floor`, `wall`, `entrance`, and `exit`. The first room receives an `entrance` marker tile at its center and the last room receives an `exit` marker tile at its center. If no dedicated marker tile is available, floor tiles are used instead.

Gameplay objects are generated as item or character instances, not tile kinds. Use `[procedural.items.<kind>]` for weighted item templates such as doors, traps, potions, or treasure, and `[procedural.characters.<kind>]` for weighted character templates.

Weighted choices can be authored compactly:

```toml
[procedural.items.door]
names = ["Wooden Door", "Iron Door"]
weights = [4, 1]
```

Or as explicit choices:

```toml
[procedural.characters.monster]
chance = 0.4
choices = [
  { name = "Skeleton", weight = 3 },
  { name = "Orc", weight = 1 },
]
```

Character spawn probability accepts `percentage = 40` or `chance = 0.4`. Generated characters are placed in room centers and skip the entrance and exit rooms.

Running **Build Procedural** clears previously generated procedural sectors, generated procedural items, and generated procedural characters before rebuilding. This lets you change tiles, item templates, character templates, or the seed and regenerate the map without manually deleting old generated content.

## Preview

You can hide sector geometry in the editor preview by name pattern:

```toml
[preview]
hide = ["KeepRoof*"]
```

- `*` is a prefix wildcard.
- `KeepRoof*` matches names like `KeepRoof`, `KeepRoofA`, `KeepRoof_Upper`.

This is useful in isometric editing when you want to hide roof sectors while working on interiors.
These preview filters are editor-only and are not applied in the in-game runtime view.
