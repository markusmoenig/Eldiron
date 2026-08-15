---
title: "Source Files"
sidebar_position: 4
---

# Source Files

Eldiron Source files use the `.els` extension. A file can contain `Character`, `Item`, `Region`, and `Screen` declarations. Declaration names are stable IDs used by configuration, terrain symbols, scripts, and other content.

## Characters and items

A character has a quoted ID, display fields, TOML-compatible data, and an optional Eldrin script:

```text
Character "player" {
  name = "Player"
  glyph = "@"

  data {
    [attributes]
    player = true
    visible = true
    class = "Warrior"

    [input]
    w = "action(forward)"
    a = "action(left)"
  }

  script {
    fn event(event, value) {
      if event == "startup" {
        message(id(), "Welcome!", "info");
      }
    }
  }
}
```

Items use the same overall shape:

```text
Item "key" {
  name = "Cellar Key"
  glyph = "k"

  data {
    [attributes]
    visible = true
    takeable = true
  }
}
```

The contents of `data` are stored with the character or item. The contents of `script` use [Eldrin](../characters_items/eldrin_scripting_language.md), including the normal [events](../characters_items/events.md) and [server commands](../characters_items/server_commands.md).

## Regions and terrain

A region describes its default tiles and a text terrain grid:

```text
Region "cellar" {
  name = "Old Cellar"
  default = wall-stone
  floor = floor-stone
  ceiling = ceiling-stone
  ceiling_height = 3.0

  terrain """
  #########
  #.......#
  #...@...#
  #.......#
  #########
  """
}
```

Each character in the terrain is a symbol. `@` places the player. Character and item glyphs place their matching declarations. Other symbols are mapped to tiles.

Tile mappings can be declared in a top-level or region-local `tiles` block:

```text
tiles {
  "#" = wall-stone
  "." = floor-stone ceiling=ceiling-stone ceiling_height=3.0
}
```

Useful mapping options include `blocking`, `material`, `finish`, `ceiling`, `ceiling_height`, and `profile`. A region-local mapping overrides the same symbol for that region.

## Screens and widgets

Screens contain positioned widgets and use the same widget data understood by regular Eldiron projects:

```text
Screen "play" {
  name = "Play"

  widget "Message" {
    role = "text"
    x = 16
    y = 16
    width = 320
    height = 48

    data {
      [ui]
      role = "text"
      text = "Welcome to the cellar"
      color = "#ffffff"
    }
  }
}
```

See [Screens](../screens/screens.md) and [Widgets](../screens/widgets.mdx) for the available UI concepts.

## A larger example

The `source_projects/stonefall-dungeon` project in the repository demonstrates:

- a first-person 3D dungeon;
- a large terrain region with multiple characters, items, and screens;
- embedded gameplay scripts;
- procedural wall, floor, ceiling, equipment, and fixture recipes;
- project-local images and runtime configuration.

Build it from the Eldiron repository root with:

```bash
cargo run --release -p eldiron-source -- build source_projects/stonefall-dungeon
```
