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
