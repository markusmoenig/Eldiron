---
title: "Procedural Map Generation"
sidebar_position: 6
---

Procedural map generation creates map geometry from region settings and tile metadata. The first generator is `connected_rooms`, a seed-based 2D dungeon generator for room-and-corridor layouts.

Use this workflow when you want a generated dungeon that can be rebuilt after changing tiles, item templates, character templates, or the seed.

## Workflow

1. Tag the tiles the generator may use with **Edit Tile Meta**.
2. Add a `[procedural]` section to the region settings.
3. Run **Build Procedural** from the action list.
4. Adjust tiles, spawn rules, or the seed and run **Build Procedural** again.

Running **Build Procedural** rebuilds the procedural region as an authored generator output: existing geometry, items, and non-player characters in that region are cleared before the new layout is created. Keep persistent handcrafted content in another region, or spawn it again from script after the rebuild.

At runtime, scripts can call `build_procedural(0)` to advance the procedural run and rebuild the current region. Scripts can also read or change live region settings before rebuilding by using context variables such as `region.procedural.room_count` or `region.procedural.characters.skeleton.percentage`.

```eldrin
let depth = region.dungeon.depth + 1;
region.dungeon.depth = depth;
region.procedural.room_count = 6 + depth;
region.procedural.characters.skeleton.percentage = 25 + depth * 6;
world_event("dungeon_exit", id());
```

The world script can then rebuild the region and place the player at the new entrance:

```eldrin
fn event(event, value) {
    if event == "dungeon_exit" {
        build_procedural(0);
        teleport_entity(value, "entrance", "");
    }
}
```

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
- `mode`: `2d` builds tile-map geometry. `3d` is reserved for a later Dungeon Tool-backed generator.
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

## Endless Roguelike Loop

A simple endless roguelike loop can use one procedural dungeon region and rebuild that same region whenever the player reaches the exit.

Recommended setup:

- Add an `entrance` tile kind and an `exit` tile kind so the generator can create named sectors for spawning and progression.
- Give the player an `entered` event that reacts to `exit`.
- Store progression in region context values, for example `region.dungeon.depth`.
- Update procedural settings from the player script, then raise a world event.
- Let the world script call `build_procedural(0)` and `teleport_entity(player_id, "entrance", "")`.

Example player `entered` event:

```eldrin
if event == "entered" {
    if value == "exit" {
        let depth = region.dungeon.depth + 1;
        region.dungeon.depth = depth;
        region.procedural.room_count = 6 + depth;
        region.procedural.characters.skeleton.percentage = 25 + depth * 6;
        world_event("dungeon_exit", id());
    }
}
```

Example world event:

```eldrin
fn event(event, value) {
    if event == "dungeon_exit" {
        build_procedural(0);
        teleport_entity(value, "entrance", "");
    }
}
```

Passing `0` to `build_procedural` advances the procedural run so the next rebuild uses a different deterministic layout derived from the region seed. Passing a positive seed rebuilds from that exact seed instead.

## Related

- [Build Procedural action](/docs/creator/actions/#build-procedural)
- [Edit Tile Meta action](/docs/creator/actions/#edit-tile-meta)
- [Region Settings](/docs/building_maps/region_settings/#procedural)
- [Server Commands](/docs/characters_items/server_commands/#build_procedural)
