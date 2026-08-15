---
title: "Eldiron Source"
sidebar_position: 1
slug: /source-projects
---

# Eldiron Source

**Eldiron Source** is the text-first way to create Eldiron games. Instead of editing a single project in Eldiron Creator, you keep the game's configuration, maps, characters, items, screens, scripts, and procedural recipes in files that work well with a text editor and version control.

The `eldiron-source` command compiles this source folder into a regular `.eldiron` project. The resulting project can be played by the same Eldiron clients as a project created visually.

```text
eldiron.toml + .els files + assets
                 |
          eldiron-source build
                 |
                 v
          build/game.eldiron
```

The source files are the source of truth. Treat the generated `.eldiron` file as build output: rebuilding replaces it.

## When to use it

Eldiron Source is useful when you prefer:

- readable text files and normal version-control diffs;
- repeatable builds;
- large maps expressed as text grids;
- reusable procedural recipes stored beside the game source;
- editing game content with a code editor or automation tools.

Eldiron Creator remains the visual authoring environment. Both workflows produce the same project format, but a Source project is intended to be maintained from its source files rather than by editing its generated output.

## In this chapter

- [Getting Started](./getting_started.md) covers downloads and the `new`, `build`, `play`, and `watch` commands.
- [Project Structure](./project_structure.md) explains `eldiron.toml`, source directories, assets, and build output.
- [Source Files](./source_files.md) introduces characters, items, regions, terrain, screens, and embedded scripts.

For a complete example, see `source_projects/stonefall-dungeon` in the Eldiron repository.
