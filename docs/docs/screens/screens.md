---
title: "Screens"
sidebar_position: 1
---

**Screens** are special maps that define the **visible area** of your game, i.e. your game UI.

Design screens with the same map tools you already use:

- **Linedef / Edge Tool** and **Sector / Face Tool**: carve areas, name sectors to turn them into widgets.
- **Rect Tool**: add background decorations.

![Screen Widgets](screens_widgets.png)

Each **sector** shows up as an item in the project tree; selecting it opens the widget editor where you can set the **role** and configure widget **attributes** (see [widgets](widgets)).

## Screen Settings and Responsive Layout

Select **Settings** below a Screen in the project tree to edit its screen-level TOML. Screens use the legacy fixed canvas unless they explicitly enable responsive layout:

```toml
[layout]
mode = "responsive"
```

On a responsive screen, an existing `role = "game"` widget automatically fills the current client window. Widget roles are lowercase identifiers, so use `role = "game"`, not `role = "Game"`. Eldiron does not create a game widget automatically: a responsive screen without one is a valid UI-only screen.

For a ruleset-driven bottom UI, add one widget with `role = "action_bar"` and anchor it with `anchor = "bottom_center"`. The widget generates its configured buttons as a group and keeps the complete bar centered when the window is resized. Its TOML can also configure built-in panels such as `[ui.equipment]`, `[ui.inventory]`, `[ui.spellbook]`, and `[ui.preferences]`, so an ordinary gameplay screen does not need separate popup widgets. Add a `role = "equipment"`, `role = "inventory"`, or `role = "spellbook"` sector only when that screen should override the corresponding default panel rectangle or presentation. See [Action Bar Widgets](./widgets#action-bar-widgets).

Reusable `role = "tab_bar"` and `role = "dropdown"` widgets provide binding-driven selections for inventory filters, sorting, and custom panels. They use Eldiron's embedded Roboto Bold font by default and can select a project font explicitly. See [Tab Bar and Dropdown Widgets](./widgets#tab-bar-and-dropdown-widgets).

Other widgets can anchor their authored size to the runtime window:

```toml
[layout]
anchor = "bottom_center"
x = 0
y = -20
```

Supported anchors are `top_left`, `top_center`, `top_right`, `center_left`, `center`, `center_right`, `bottom_left`, `bottom_center`, and `bottom_right`. The optional `x` and `y` values are offsets from the anchor. Without an anchor, the authored widget rectangle is retained.

## Eldiron Source Screens

Eldiron Source projects can define the same screen widgets in `.els` files.
The source form compiles into the normal Eldiron screen data, so Creator and all
clients use the same widget roles and configuration.

```text
Screen "play" {
  name = "Dungeon Play"

  widget "Game" {
    role = "game"
    x = 0
    y = 96
    width = 720
    height = 504

    data {
      [ui]
      role = "game"
      grid_size = 40

      [camera]
      type = "firstp"
    }
  }

  widget "Leader Profile" {
    role = "profile"
    x = 8
    y = 8
    width = 222
    height = 80

    data {
      [ui]
      role = "profile"
      party = "leader"
      image_size = 64
      stats_layout = "side"

      [[ui.stats]]
      stat = "HP"
      max_stat = "MAX_HP"
      height = 14
      fill_color = "#d63a3a"
      background_color = "#250909"
      border_color = "#6c3030"
      border_size = 1
    }
  }
}
```

- **Screen** defines one screen map.
- **widget** defines a rectangular widget by name, role, position, and size.
- **data** contains the same TOML widget configuration used by Creator widgets.
- Coordinates and sizes are screen pixels. Match them to the project
  `[viewport]` dimensions in `eldiron.toml`.
