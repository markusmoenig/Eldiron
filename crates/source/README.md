# Eldiron Source

`eldiron-source` is the source-first compiler and project tool for
[Eldiron](https://eldiron.com) games.

An Eldiron Source project keeps its game configuration, maps, characters,
items, screens, scripts, assets, and procedural recipes in text-friendly source
files. The tool compiles those files into a regular `.eldiron` project that can
be run by the standard Eldiron clients.

```text
eldiron.toml + .els files + assets
                 |
          eldiron-source build
                 |
                 v
          build/game.eldiron
```

The source folder is the source of truth. The generated `.eldiron` file is build
output and is replaced by the next build.

## Installation

Install from [crates.io](https://crates.io/crates/eldiron-source):

```bash
cargo install eldiron-source
```

## Quick start

```bash
eldiron-source new my-game
eldiron-source build my-game
eldiron-source play my-game
```

Use `watch` during development to rebuild after source or asset changes:

```bash
eldiron-source watch my-game
```

The main commands are:

- `new` — scaffold a Source project;
- `build` — compile it into a `.eldiron` project;
- `play` — build and start the configured graphical or terminal client;
- `watch` — rebuild when project files change.

Run `eldiron-source help <command>` for command-specific options.

The full guide, project layout, and `.els` examples are in the
[Eldiron Source documentation](https://eldiron.com/docs/source-projects).
The Eldiron repository also includes the complete
`source_projects/stonefall-dungeon` example.

## License

Licensed under the MIT License.
