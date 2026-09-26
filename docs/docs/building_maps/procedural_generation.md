---
title: "Procedural Map Generation"
sidebar_position: 6
---

Procedural map generation creates map geometry from region settings and tile metadata. The first generator is `connected_rooms`, a seed-based 2D dungeon generator for room-and-corridor layouts.

Use this workflow when you want a generated dungeon that can be rebuilt after changing tiles, item templates, character templates, or the seed.

## Workflow

1. Tag the tiles the generator may use with **Edit Tile Meta**.
2. Add a `[procedural]` section to the region settings.
3. Switch to the 2D editor view and run **Build Procedural** from the action list.
4. Adjust tiles, spawn rules, or the seed and run **Build Procedural** again.

Running **Build Procedural** rebuilds the procedural region as an authored generator output: existing geometry, items, and non-player characters in that region are cleared before the new layout is created. Keep persistent handcrafted content in another region, and rebuild only the generated region.

## Tile Metadata

Select a tile in the tile picker and use **Edit Tile Meta** to assign procedural metadata:

```toml
[procedural]
style = "stone"
kind = "floor"
weight = 1
```

- `style`: groups tiles into a visual set, such as `stone`, `cave`, or `crypt`.
- `kind`: describes what the tile is used for.
- `weight`: controls how often this tile is chosen relative to other tiles of the same style and kind.

Supported `kind` values for `connected_rooms` are:

- `floor`: room and corridor floor tiles.
- `wall`: wall tiles around generated floor areas.
- `entrance`: marker tile for the start endpoint.
- `exit`: marker tile for the end endpoint.

Use `none` for tiles that should not be selected by procedural generation.

Doors, traps, potions, treasure, and monsters should not be tile kinds. Generate them as item or character instances from region settings instead.

## Region Settings

The generator is configured in the current region's settings:

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
- `mode`: `2d` builds tile-map geometry. `3d` is reserved for a future direct geometry generator.
- `seed`: makes the generated layout deterministic.
- `style`: selects tiles whose procedural style matches this value.
- `width` / `height`: generated grid size.
- `room_count`: target number of connected rooms.
- `room_min_size` / `room_max_size`: room size range in tiles.
- `door_placement`: `entrances`, `exits`, or `both`.
- `door_randomness`: probability from `0.0` to `1.0` after `door_placement` filtering.

If a matching tile style is not available, the generator can fall back to any procedural tile of the required kind. If no dedicated `entrance` or `exit` tile is available, floor tiles are used for those markers.

The same settings can be read or changed at runtime with `region.procedural.*` context paths before calling `build_procedural()`:

- `region.procedural.seed`
- `region.procedural.width`
- `region.procedural.height`
- `region.procedural.room_count`
- `region.procedural.room_min_size`
- `region.procedural.room_max_size`
- `region.procedural.door_placement`
- `region.procedural.door_randomness`
- `region.procedural.characters.<kind>.chance`
- `region.procedural.characters.<kind>.percentage`

`region.procedural.run` is maintained by `build_procedural(0)`. Each `0` rebuild increments it and derives a new deterministic seed from the region's configured `seed`. Use a positive seed argument if you want to rebuild from an exact seed instead.

For compatibility, `region.procedural.rooms` is accepted as an alias for `region.procedural.room_count`, and `percent` is accepted as an alias for character `percentage`.

## Connected Rooms

`connected_rooms` creates a single connected path from the entrance room to the exit room. Rooms are standalone areas connected by corridors, rather than one large merged maze.

The endpoint marker tiles are named `entrance` and `exit`. These named sectors can be used by scripts, teleport targets, or entered/left events.

Door placement is controlled by `door_placement`:

- `entrances`: only place doors on incoming room sides.
- `exits`: only place doors on outgoing room sides.
- `both`: place doors on both sides of room connections.

`door_randomness` decides whether an eligible door is actually placed. If no door is placed at a connection, that connection remains passable floor.

## Items

Items are generated from `[procedural.items.<kind>]` tables. Door generation currently uses the `door` kind:

```toml
[procedural.items.door]
names = ["Wooden Door", "Iron Door"]
weights = [4, 1]
```

The generator creates item instances from the named item templates. The item template controls its tile, blocking behavior, script, and interactions.

You can also write weighted choices explicitly:

```toml
[procedural.items.door]
choices = [
  { name = "Wooden Door", weight = 4 },
  { name = "Iron Door", weight = 1 },
]
```

## Characters

Characters are generated from `[procedural.characters.<kind>]` tables:

```toml
[procedural.characters.monster]
chance = 0.4
choices = [
  { name = "Skeleton", weight = 3 },
  { name = "Orc", weight = 1 },
]
```

Character spawn probability accepts either:

- `percentage = 40`
- `chance = 0.4`

Generated characters are placed in room centers and skip the entrance and exit rooms.

## Related

- [Build Procedural action](/docs/creator/actions/#build-procedural)
- [Edit Tile Meta action](/docs/creator/actions/#edit-tile-meta)
- [Region Settings](/docs/building_maps/region_settings/#procedural)
