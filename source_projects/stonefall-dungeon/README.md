# Stonefall Dungeon

Dungeon Master-style Eldiron Source sample project.

Build:

```sh
target/release/eldiron-source build source_projects/stonefall-dungeon
```

Play:

```sh
target/release/eldiron-source play source_projects/stonefall-dungeon
```

Run the graphical client with:

```sh
target/release/eldiron-client source_projects/stonefall-dungeon/build/stonefall-dungeon.eldiron
```

This project is intentionally source-first: regions, characters, screens, and
tile symbol mappings live in `.els` files and compile to a regular `.eldiron`
project.

## Procedural environment tiles

Stonefall's wall, floor, and ceiling live as height-first text recipes in
`recipes/`. Eldiron Source executes them with the project's art palette while
building and embeds their results as normal tiles. No generated PNGs or
edge-crossfade scripts are part of the asset pipeline. Recipe `coverage`
controls how many dungeon cells share one continuous texture footprint; the
primary wall uses coarse, perturbed masonry rather than a small regular
brick grid.

Reusable color graphs, Roughness/Metallic/Opacity/Emission data, and
micro-normal settings live in unified `Surface` blocks inside
multi-declaration `.recipe` files. Materials are independent of tile height;
tile recipes own geometry and any material masks. The three environment recipes
reference `dungeon.recipe`.

The terrain in `regions/dungeon.els` is a first-person dungeon with distinct
chambers, connecting galleries at least three cells wide, a trap hall, a crypt,
a fountain room, and an exit chamber. `main.els` keeps the map readable by assigning
single-character symbols to:

- the procedural irregular-stone wall;
- the procedural flagstone floor;
- the procedural rough-stone ceiling.

Tile declarations may use `blocking=true` or `blocking=false`, which lets
Stonefall use multiple wall and floor symbols without hard-coding their
collision behavior into the compiler. Walkable symbols may use
`ceiling=<tile>` to theme ceilings per chamber.
