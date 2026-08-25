---
title: "Palette Tool"
sidebar_position: 11
---

The **Palette Tool** (keyboard shortcut **`P`**) switches Eldiron into **art palette editing mode**.

Eldiron keeps two palette concepts separate:

- **Ruleset Palette**: fixed colors owned by the active ruleset. These indices
  are used by ruleset avatars, UI color defaults, and generated missing-art
  fallbacks. Artist-authored item icon PNGs are full-color RGBA and are not
  recolored through this palette.
- **Art Palette**: the editable 256-slot project palette used for tiles, pixel drawing, tile graphs, palette-index geometry sources, and 3D Paint.

The Palette Tool edits the **Art Palette**. It does not edit the Ruleset Palette.

## What It Is For

Use the Palette Tool to work with Art Palette slots:

- select palette entries
- create new entries
- clone entries
- delete entries
- swap entries by drag and drop
- edit the selected entry color
- edit the selected entry material and finish
- apply the selected Art Palette entry to geometry

New projects start with an Art Palette seeded from the Endesga 64 palette, leaving the rest of the 256 slots available for project-specific colors.

## Palette Dock

When Palette mode is active, the lower dock shows the **Palette** dock.

The dock contains:

- a fixed, non-editable **Ruleset Palette** view
- an editable **Art Palette** board
- a material inspector for the selected Art Palette entry
- toolbar buttons for:
  - `New`
  - `Clone`
  - `Apply Color`
  - `Clear`

`New` and `Clone` append at the end of the currently used Art Palette range so existing indices are not disturbed.

## Material Properties

Each Art Palette entry stores both a color and high-level material metadata:

- `Material`
- `Finish`

These values are resolved through Eldiron's material library when a surface uses a palette-index source. Palette entries do not store raw PBR sliders in project files.

Changing an Art Palette entry's material or finish updates existing surfaces that use that palette index; you do not need to reapply the color.

## Loading Palettes

The **Load Palette ...** action loads colors into the Art Palette. It does not overwrite the Ruleset Palette.

Use this when you want to switch an art workflow to another color set while keeping rules-owned indices stable.

## Minimap Color Picking

While Palette mode is active, the minimap switches to **palette color picking** instead of normal map navigation.

You can:

- click to sample a color
- drag to preview color changes continuously
- release the mouse to commit the final change

Drag sampling creates a single undo step on mouse release.

## HUD and Apply Workflow

Palette mode still uses the normal geometry selection and HUD slot system.

That means you can select geometry and apply the current Art Palette entry as its source.

`Apply Color` belongs to the Palette dock, not to the Tile Picker.

Opening the Palette tree item in the sidebar selects palette content, but the Palette Tool is the editing mode that keeps the Palette dock active while you work in the map.

## Related

- [Overview](/docs/creator/tools/overview)
- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Project Tree](/docs/creator/project_tree)
