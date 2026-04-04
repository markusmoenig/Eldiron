---
title: "Palette Tool"
sidebar_position: 8
---

The **Palette Tool** (keyboard shortcut **`P`**) switches Eldiron into **palette editing mode**.

Unlike the main map tools, Palette is a **mode toggle** at the bottom of the tool strip. While it is active, Eldiron keeps the **Palette** dock open instead of switching back to the **Tile Picker** when you select geometry.

## What It Is For

Use the Palette Tool to work with the project palette as a board of fixed slots:

- select palette entries
- create new entries
- clone entries
- delete entries
- swap entries by drag and drop
- edit the selected entry color and material values
- apply the selected palette entry to geometry and Builder material slots

## Palette Dock

When Palette mode is active, the lower dock shows the **Palette** dock.

The dock contains:

- a palette board on the left
- a material inspector on the right
- toolbar buttons for:
  - `New`
  - `Clone`
  - `Apply Color`
  - `Clear`

`New` and `Clone` always append at the end of the currently used palette range so existing indices are not disturbed.

## Material Properties

Each palette entry stores both a color and a small set of material properties:

- `Roughness`
- `Metallic`
- `Opacity`
- `Emissive`

These values are used when a palette entry is applied through `PaletteIndex` sources, for example on sectors or Builder material slots.

## Minimap Color Picking

While Palette mode is active, the minimap switches to **palette color picking** instead of normal map navigation.

You can:

- click to sample a color
- drag to preview color changes continuously
- release the mouse to commit the final change

Drag sampling creates a single undo step on mouse release.

## HUD and Apply Workflow

Palette mode still uses the normal geometry selection and HUD slot system.

That means you can:

- select a sector and apply the current palette entry as its source
- select a Builder host and use the HUD material slots to apply the current palette entry to a Builder material slot

`Apply Color` belongs to the Palette dock, not to the Tile Picker.

## Global Palette vs Palette Dock

There are two palette views in Eldiron:

- the **global Palette** tree item in the sidebar
- the **Palette dock** used by Palette mode

The global palette is primarily for selection and overview.
The Palette dock is the editing surface used for:

- drag-and-drop swapping
- `New`
- `Clone`
- `Delete`
- material editing

Opening the global Palette tree item does **not** enable Palette mode by itself.

## Related

- [Overview](/docs/creator/tools/overview)
- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Project Tree](/docs/creator/project_tree)
