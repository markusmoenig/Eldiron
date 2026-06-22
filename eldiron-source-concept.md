# Eldiron Source Concept

Eldiron Source is a proposed source-first creation mode for Eldiron. Its crate
name would be `eldiron-source`.

Eldiron Creator is the visual editor. Eldiron Source would be the textual,
terminal-friendly authoring workflow for people who prefer programming, source
files, version control, and fast edit-compile-play loops.

The central idea is that Eldiron Source does not create a second game engine or
a second project model. It compiles source projects into normal `.eldiron` files
and reuses the existing Eldiron infrastructure, data model, scripting, project
settings, and runtime behavior.

## Goals

- Provide a textual game creation workflow for Eldiron.
- Compile `.els` source projects into regular `.eldiron` project files.
- Reuse Eldrin as the behavior language for characters, items, regions, and
  other scripted game objects.
- Allow games to be played immediately in the terminal after compilation.
- Keep `.eldiron` as the runtime source of truth.
- Make maps semantic, not graphical, so terminal, visual, and procedural
  rendering can all share the same gameplay data.
- Support both small single-file projects and larger folder-based projects.

## Project Structure

An Eldiron Source project should be a folder with a TOML project file at its
root. The TOML file defines project-level settings and the initial game entry
points, such as the start region and start screen.

Example:

```text
my-game/
  eldiron.toml
  main.els
  characters/
    player.els
    guard.els
  items/
    key.els
    lantern.els
  assets/
    fonts/
      ui.ttf
    audio/
      door.wav
  tiles/
    wall_stone.png
    floor_stone.png
  regions/
    cellar.els
    town.els
  scripts/
    shared.eldrin
  build/
    my-game.eldiron
```

Possible TOML shape:

```toml
[project]
name = "Forgotten Well"
version = "0.1.0"

[source]
main = "main.els"

[game]
start_region = "cellar"
start_screen = "terminal"
client_mode = "terminal"
terminal_mode = "roguelike"
simulation_mode = "hybrid"
turn_timeout_ms = 600
collision_mode = "tile"
player = "player"
auto_create_player = true

[viewport]
width = 80
height = 24
grid_size = 40
unit = "cell"
resize = "fit"

[build]
output = "build/forgotten-well.eldiron"
```

The TOML file is responsible for project configuration and boot settings. The
`.els` files are responsible for game content. Eldrin remains responsible for
behavior.

Projects should also have conventional asset folders. `assets/` is for general
project assets that should be copied into the compiled `.eldiron` file, such as
fonts, audio, and standalone images. `tiles/` is for PNG/JPEG image files that
should become Eldiron tile definitions. `images/` is accepted as an alias for
tile images while the project convention settles, but `tiles/` is the clearer
name for source projects.

At compile time:

```text
assets/**/*.ttf, *.otf       -> project font assets
assets/**/*.wav, *.ogg, ...  -> project audio assets
assets/**/*.png, *.jpg       -> project image assets
tiles/**/*.png, *.jpg        -> project tiles
images/**/*.png, *.jpg       -> project tiles
```

Imported asset and tile names are derived from their relative path without the
extension, so `tiles/dungeon/wall_stone.png` becomes the tile alias
`dungeon/wall_stone`.

```text
TOML   = project identity, build settings, start region, start screen
ELS    = maps, characters, items, regions, screens, semantic content
Eldrin = character, item, region, and gameplay behavior
```

## Compilation Model

The compiler would load the project TOML, then load the configured main `.els`
file. Additional content files can be discovered automatically from conventional
folders such as `characters/`, `items/`, and `regions/`.

```text
eldiron.toml
  -> load main.els
  -> auto-load content folders
  -> parse and validate source
  -> build Eldiron project data
  -> write .eldiron
```

Characters, items, regions, and other content may be defined inline in the main
`.els` file or split into separate files. This keeps tiny projects convenient
while allowing larger games to stay organized.

Compilation should produce good diagnostics, especially for map and reference
errors:

```text
unknown tile symbol 'X' in map 'cellar' at line 12, column 18
unknown character reference 'guard_captain' in region 'town'
```

## The `.els` Language

`.els` should be a declarative source format for Eldiron game content. It should
describe what exists in the game world, while Eldrin describes how things behave.

Illustrative example:

```text
Character "guard" {
  name = "Town Guard"
  glyph = "G"

  stats {
    health = 10
  }

  script {
    on_interact {
      say("Keep moving.");
    }
  }
}
```

The `script { ... }` block is Eldrin source embedded inside `.els`; it should not
become a second behavior language.

The same idea applies to items:

```text
Item "lantern" {
  name = "Lantern"
  glyph = "l"

  script {
    on_use {
      toggle_light(self);
    }
  }
}
```

## Semantic Maps

Embedded text maps are a core part of Eldiron Source, but they should represent
semantic gameplay data, not final graphics.

For example, `#` should not mean "draw a gray wall". It should mean something
like `terrain.wall.stone`, which can later be rendered by a terminal glyph,
visual tileset, procedural theme, or other renderer.

Illustrative example:

```text
Region "cellar" {
  default = terrain.void

  terrain """
  ################
  #@.....#.......#
  #..g...D...~...#
  #......#..~~~..#
  #......#.......#
  ################
  """
}
```

In this example, the glyphs come from the active ruleset's source conventions.
Projects should not have to redefine common symbols such as `#` for wall or `.`
for floor unless they want to override them.

The map should compile into Eldiron's existing region, tile, entity, and gameplay
data. Terminal rendering is only one possible presentation of that semantic map.
However, for conceptual terminal play, the same source symbols should also
provide the default textual presentation. That means `#` can both resolve to a
wall tile UUID and be the terminal glyph used to display that wall in the fast
source-play loop.

## Tile Identity

Eldiron tiles are associated with UUIDs. Eldiron Source should preserve that
model instead of generating anonymous tiles from map glyphs.

Map symbols such as `#`, `.`, or `~` should resolve through the active ruleset's
source symbol catalog. That catalog maps common glyphs to stable tile IDs,
character selectors, item selectors, spawn markers, terminal glyphs, terminal
colors, and other semantic concepts. This lets Eldiron Creator immediately
understand the compiled map because every semantic source tile maps to an
existing Eldiron tile definition with established behavior, metadata, and visual
presentation.

The project-level `tiles { ... }` block should be optional. It is for overrides,
local aliases, or project-specific symbols, not for redefining the basic source
vocabulary in every game.

Example:

```text
tiles {
  "#" = tile "00000000-0000-0000-0000-000000000101" // wall.stone
  "." = tile "00000000-0000-0000-0000-000000000102" // floor.damp
  "~" = tile "00000000-0000-0000-0000-000000000103" // water.shallow
}
```

For readability, `.els` should probably also support named aliases that are
resolved through the active ruleset or through project tile aliases:

```text
tiles {
  "#" = wall
  "." = floor
  "@" = floor
}
```

During compilation, those names resolve to concrete UUIDs. The generated
`.eldiron` file would contain normal Eldiron tile references, not source-only
concepts.

When a project has image tiles in `tiles/`, Eldiron Source resolves these values
by tile alias/name. For example, `tiles/wall.png` creates the alias `wall`, so
`"#" = wall` compiles to that tile's UUID. Nested tile paths also work:
`tiles/dungeon/wall.png` has the alias `dungeon/wall` and can be referenced by
the full alias or by the unique leaf name `wall`.

This gives Eldiron Source a clean authoring layer while keeping `.eldiron` and
Eldiron Creator grounded in the existing UUID-based tile model.

## Source Symbol Conventions

The active ruleset should provide a default source symbol convention so maps can
be written without boilerplate.

These symbols are both compile-time semantics and the default conceptual
terminal rendering vocabulary. The same `#` that means "wall" in source can be
drawn as `#` in terminal play unless the ruleset or project overrides the
terminal presentation.

Possible conventions:

- `#` = wall
- `.` = floor
- `~` = water
- `+` = closed door
- `/` = open door
- `@` = player spawn
- Uppercase letters = characters or character spawns
- Lowercase letters = items or item spawns
- Lowercase aliases can have ruleset meanings, such as `h` for herbs and `b`
  for blessed herbs

For example:

```text
Region "meadow" {
  terrain """
  ########
  #..h...#
  #..b.C.#
  #..@...#
  ########
  """
}
```

The compiler would resolve the symbols through the active ruleset. A project can
still override or extend the defaults when needed:

```text
symbols {
  "C" = character "captain" {
    terminal = "C"
    color = "yellow"
  }

  "h" = item "healing_herb" {
    terminal = "h"
    color = "green"
  }

  "b" = item "blessed_herb" {
    terminal = "b"
    color = "bright_green"
  }
}
```

This keeps small maps terse while still allowing explicit control for larger
projects.

## Map Editing

Plain rectangular text maps can be awkward to edit. Adding a column often means
editing every row by hand.

Eldiron Source should avoid requiring a full custom terminal text editor at the
start. Instead, the compiler and CLI can make normal editor workflows less
fragile.

Possible approaches:

- Accept ragged map rows and pad them with a configured default tile.
- Provide formatting commands that normalize map blocks.
- Provide map surgery commands for structural edits.
- Support stamps, patches, rectangles, and procedural placement to avoid huge
  fragile ASCII maps.

Example commands:

```sh
eldiron-source fmt
eldiron-source map insert-col regions/cellar.els cellar 12
eldiron-source map delete-col regions/cellar.els cellar 7
eldiron-source map insert-row regions/cellar.els cellar 4
```

Example compositional map source:

```text
Region "town" {
  size = 80x40
  fill = terrain.grass

  stamp "house_small" at 10, 8
  stamp "well" at 31, 14
  rect terrain.road.stone from 0,20 to 79,22

  terrain at 5,5 """
  #####
  #...#
  #...#
  #####
  """
}
```

This keeps source maps editable while still allowing precise hand-authored
layouts.

## CLI

`eldiron-source` should be a normal command-line tool with proper command help.
The first concrete commands are:

```sh
eldiron-source new my-game
eldiron-source build my-game
eldiron-source play my-game
eldiron-source watch my-game
eldiron-source help new
```

- `new` scaffolds an Eldiron Source project folder with `eldiron.toml`,
  `main.els`, and the conventional `characters/`, `items/`, `regions/`,
  `scripts/`, `assets/`, `tiles/`, and `build/` folders.
- `build` compiles the source project into the configured `.eldiron` output.
- `play` builds first, then launches the configured terminal, 2D, or 3D client.
- `watch` observes project sources and assets and rebuilds the `.eldiron` file
  after edits. Runtime reload can be layered on top later.

## Terminal Play

Instant terminal play is a major part of the appeal. The important rule is that
terminal play should consume generated `.eldiron` files, not `.els` directly.

```text
.els + eldiron.toml
  -> eldiron-source compile
  -> .eldiron
  -> terminal player
```

This avoids a second source of truth. If a game works in terminal preview, it is
working through the same compiled Eldiron project representation that Creator
and other runtimes can use.

A good authoring loop would be:

```sh
eldiron-source watch
```

The watch command currently:

```text
detect source changes
  -> compile .els project into .eldiron
  -> print compile errors when compilation fails
```

Later, a terminal or graphical runtime can add live reload of the compiled
`.eldiron` file.

## Terminal Runtime

Terminal rendering should live outside `eldiron-source`. The current codebase
already has `clients/client-terminal`, which can serve as the first runtime
target for source-generated `.eldiron` files.

Possible long-term crate split:

```text
eldiron-source    = .els project compiler and source tooling
eldiron-terminal  = terminal runtime that loads .eldiron files
eldiron-core      = shared project/game data model
eldiron-creator   = visual editor
```

In the current workspace, that maps roughly to:

```text
eldiron-source           = proposed new compiler crate
clients/client-terminal  = existing terminal client/runtime target
crates/shared            = existing shared project/game data model
creator                  = existing visual editor
```

The terminal runtime would translate Eldiron screens and widgets into terminal
areas and formatted text.

The terminal client should support explicit play modes for the same `.eldiron`
file:

```sh
eldiron-client-terminal game.eldiron --mode text
eldiron-client-terminal game.eldiron --mode roguelike
```

The CLI mode should override the project default in `[game].terminal_mode`.
Without either setting, the client should default to `text`.

Examples:

- Screen -> terminal frame/layout
- Widget -> rectangular terminal area
- Text widget -> wrapped/styled terminal text
- Choice/list widget -> selectable terminal list
- Inventory widget -> table, list, or grid
- Map widget -> roguelike viewport
- Dialog/message widget -> panel or overlay

This runtime would be useful beyond Eldiron Source. Eldiron Creator could also
offer "Preview in Terminal" for any `.eldiron` project.

Graphical source play should still compile to `.eldiron` first. For example,
`client_mode = "3d"` can build source text-map regions into Eldiron's generated
first-person 3D geometry, set the player to first-person grid input, and launch
the normal graphical client. `client_mode = "2d"` should use the graphical 2D
client path. Eldiron Source remains the compiler and source tooling layer, not a
second graphical runtime.

## Formatting and TUI Infrastructure

Because terminal play needs formatted output, Eldiron will eventually need
terminal formatting and layout tools.

Rust crates worth evaluating:

- `ratatui` for terminal layout, widgets, styling, buffers, and rendering.
- `crossterm` as the common terminal backend used by Ratatui.
- `tui-textarea` for optional multi-line terminal text input or future source
  editing support.

The first target should not be a full terminal source editor. The first target
should be a solid terminal runtime for `.eldiron` files and enough source tooling
to make `.els` maps pleasant to maintain.

## Suggested First Milestone

The first implementation milestone could be intentionally narrow:

1. Define `eldiron.toml`.
2. Parse a minimal `.els` file.
3. Support inline and folder-based characters, items, and regions.
4. Embed Eldrin scripts in character and item definitions.
5. Compile semantic maps into normal Eldiron region data.
6. Write a `.eldiron` file.
7. Load that `.eldiron` file in a terminal player.
8. Add runtime reload on top of `watch` for a compile-and-play workflow.

This would establish Eldiron Source as a real alternate authoring frontend while
keeping `.eldiron` as the shared project format.

## First Prototype Scope

The first prototype should prove one vertical slice:

```text
source project folder
  -> eldiron-source build
  -> normal .eldiron JSON project
  -> clients/client-terminal can load and play it
  -> Eldiron Creator can open it
```

This prototype should not try to cover the whole DSL. It should compile the
smallest possible source project into the existing `eldiron-shared::Project`
shape.

Minimal project:

```text
sample-source-game/
  eldiron.toml
  main.els
```

Minimal `eldiron.toml`:

```toml
[project]
name = "Source Prototype"
version = "0.1.0"

[source]
main = "main.els"

[game]
start_region = "cellar"
start_screen = "play"
terminal_mode = "roguelike"
simulation_mode = "hybrid"
turn_timeout_ms = 600
collision_mode = "tile"
auto_create_player = true
player = "player"

[viewport]
width = 80
height = 24
grid_size = 40
unit = "cell"
resize = "fit"

[build]
output = "build/source-prototype.eldiron"
```

Minimal `main.els`:

```text
Character "player" {
  name = "Player"
  glyph = "@"

  script {
  }
}

Region "cellar" {
  default = wall.stone

  terrain """
  #######
  #.....#
  #..@..#
  #.....#
  #######
  """
}
```

The prototype can rely on default source symbols, and projects can use
`tiles { ... }` blocks to map terrain glyphs to project tile aliases. More
general `symbols { ... }` blocks can come later for character, item, and spawn
overrides.

The generated `.eldiron` should be a regular serialized Eldiron project. The
current codebase already uses JSON serialization for `.eldiron` files, so the
prototype should write `serde_json` output for `eldiron-shared::Project`.

Prototype compiler responsibilities:

- Read `eldiron.toml`.
- Load `main.els`.
- Resolve default source symbols such as `#`, `.`, and `@` through the active
  ruleset.
- Create a `Project` with `config` containing `[game].start_region`,
  `[game].start_screen`, `[game].terminal_mode`, `[game].simulation_mode`,
  `[game].turn_timeout_ms`, `[game].collision_mode`, and
  `[game].auto_create_player`, plus a global `[viewport]`.
- Create one `Region` with a normal Eldiron `Map`.
- Place the player character template/instance so the terminal client can
  auto-create or find the local player.
- Write the `.eldiron` file.

Prototype non-goals:

- No full source formatter yet.
- No live runtime reload yet.
- No folder auto-discovery yet.
- No procedural placement yet.
- No custom TUI editor.
- No round-trip export from Creator back to `.els`.

The first acceptance test should be practical:

```sh
eldiron-source build sample-source-game
eldiron-client-terminal sample-source-game/build/source-prototype.eldiron
```

If the terminal client starts in the `cellar` region and Creator can open the
same `.eldiron` file, the prototype has proven the architecture.
