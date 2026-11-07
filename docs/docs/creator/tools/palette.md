---
title: "Palette Tool"
sidebar_position: 11
---

The **Palette Tool** (`P`) edits the project's single 256-slot palette. Tiles,
procedural materials, avatars, item fallback colors, game UI and palette-painted
geometry all read this palette. New projects start with **Endesga 64**, leaving
other slots available for project colors. Ruleset refreshes preserve project edits.

Each slot carries a color plus a **Material** and **Finish**. For example, two
slots can use the same gray while one represents metal and another represents
stone. Palette switching preserves both slot indices and these material settings.

## Switching palettes

In the Palette dock, choose **Lospec…** and enter a palette name such as
`Endesga 32` or `Sweetie 16`. You can also paste a Lospec palette-list URL.
Eldiron downloads the palette in the background and opens a before/after preview.
Lospec identifies palettes by the slug in their URL; paste that URL when a
palette's display name differs from its slug.

**Switch Palette…** imports a local HEX, Paint.NET TXT, GIMP GPL or JASC PAL file.
Both paths support 1–256 distinct colors and use the same matching workflow:

- existing slot colors are matched by perceptual similarity;
- authored procedural shading ramps and default color families keep ordered tones
  where the target palette permits it;
- the preview reports ramps that lose distinct stops;
- accepting makes one undoable project change; cancel leaves the project untouched.

An imported palette is a target color set. Existing slot identities remain stable,
so surfaces and recipes retain their assignments. Duplicate colors are valid.
Missing target swatches are added to empty slots where space permits. The
**Imported colors…** menu retains the complete set; choosing one assigns that
color to the currently selected slot, including when the board is full.

## Artwork and rendering

Procedural recipes retain palette indices and rebuild their baked textures after
palette edits. Bundled ruleset bitmap item icons and world tile artwork are palette-fitted
from their original images on refresh, preserving alpha. Switching repeatedly never recolors
an already quantized copy. Project-owned full-color bitmap overrides retain their
own colors. The official item library already supplies editable MZ icon recipes,
body/garment materials and world models using these same project palette slots.
PNG artwork remains supported for other icons and explicit bitmap overrides.

The switch preview shows palette slots, cached material artwork, item icons, an
avatar and game UI color samples. Cached material previews approximate the
rebaked procedural result.

## Drawing and materials

Use **New**, **Clone**, **Apply Color** and **Clear** in the Palette dock to manage
slots and assign them to geometry. New and Clone append to the used range so
existing indices are preserved. Changing a slot's color, material or finish
updates surfaces that already reference it.

In Palette mode, the minimap provides palette color picking: click to sample,
drag to preview, and release to commit. Prefab geometry uses the same palette.
The editor's own UI theme is configured separately.
