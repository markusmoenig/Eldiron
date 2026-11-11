---
title: "Screens"
sidebar_position: 1
---

**Screens** are special maps that define the **visible area** of your game, i.e. your game UI.

Design screens with the same map tools you already use:

- **Linedef / Edge Tool** and **Sector / Face Tool**: carve areas, name sectors to turn them into widgets.
- **Rect Tool**: add background decorations.

![Current screen editor with widget outlines and TOML settings.](/img/creator/screen-editor.png)

Each **sector** shows up as an item in the project tree; selecting it opens the widget editor where you can set the **role** and configure widget **attributes** (see [widgets](widgets)).

New **2D**, **2D Iso** and **3D** projects include a responsive **Game** screen with the same bottom toolbar as UnendingNight: Preferences, Equipment, Inventory, Spellbook, ten action slots, Look and Use. Select **Screens → Game → Toolbar** to edit its layout, command groups and panel styling. The **Game UI** Tile Picker collection contains its nine-slice artwork, editable MZ command icons and default cursor tile. The cursor is assigned through `viewport.cursor_id` in the project configuration.

## Procedural screen artwork

Screen decoration, sector textures and tile-backed widget artwork can use
[procedural materials](../building_maps/procedural_materials), including MZ Art
and PBR Color/Opacity. UI artwork uses color and alpha; world lighting channels
do not turn a screen panel into a lit 3D surface. Recipe edits, palette changes
and animation frames update the artwork through the normal tile paths.

Continuous sources sample authored screen-grid coordinates. Repeat wraps the
pattern, while stretched widget artwork fits a representative tile to its target.
Widget artwork supports stretch, repeat and nine-slice where the widget exposes
those modes. Opacity is composited over the content below it; it is not replaced
by a black bitmap background.

Screen export includes referenced baked tiles, editable MZ recipes, composition
dependencies, linked image tiles and the source palette. Import remaps asset
identities and palette bindings into the receiving project in one undoable
change. Gameplay commands and links to other screens still need valid targets
in that project. See [Project Files](../creator/project_files#portable-screens).

## Screen Settings and Responsive Layout

Select **Settings** below a Screen in the project tree to edit its screen-level TOML. Screens use the legacy fixed canvas unless they explicitly enable responsive layout:

```toml
[layout]
mode = "responsive"
```

On a responsive screen, an existing `role = "game"` widget automatically fills the current client window. This also applies to the embedded **Game** tool in Creator: its available editor area is forwarded as the client surface size, including when docks open/close or the window is resized. The viewport grows to show more world at the current pixel scale, and the bottom toolbar follows its anchor. Widget roles are lowercase identifiers, so use `role = "game"`, not `role = "Game"`. Eldiron does not create a game widget automatically: a responsive screen without one is a valid UI-only screen.

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

For a fixed canvas, `[viewport]` in the project configuration still defines the
reference `width` and `height`. Set `upscale = "aspect"` there to scale that canvas
to fit with preserved aspect ratio, or `upscale = "none"` to keep it at native
size. Responsive layout instead adopts the available surface size; it does not
require increasing the fixed viewport dimensions to fill Creator's Game tool.

## Intent shortcuts and cursor feedback

New starter players include **U → Use**, **L → Look** and **O → Open/Close**.
UnendingNight includes the same bindings as editable **Input Binding** nodes in
the player's **Entity Nodes**. Change the key or command there; instance input
bindings can override their template's bindings. Classic directional 2D retains
its directional-intent behavior, while persistent-target iso/3D selects the
intent for the following object click.

While an intent is selected in the game view, a small **monochrome** mark beside
the normal pointer identifies the active tool. The pointer itself remains
unchanged. The mark keeps the existing intent icon's shape and details in black and white;
it disappears over UI controls and when no intent is selected.
Open/Close feedback follows the hovered door's current state. Toolbar artwork
keeps its normal colors. This also works with the direct 3D presentation overlay.
