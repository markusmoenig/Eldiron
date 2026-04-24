---
title: "Organic Tool"
sidebar_position: 7
---

The **Organic Tool** (keyboard shortcut **`O`**) paints organic surface detail directly onto map surfaces.

Use it for:

- moss and grime
- mud and stains
- edge breakup and noisy buildup
- painted surface accents on floors, walls, ceilings, and supported terrain targets

The current Organic tool is a **brush-based painter**. It no longer uses the older node-graph workflow.

## Organic Dock

When the Organic tool is active, the lower picker area shows the **Organic** dock instead of the Tile Picker.

The dock is split into three parts:

- **Brush Preview** on the left
- **Brush Presets** in the center
- **Brush Settings** on the right

### Brush Preview

The left preview shows the current brush using the active brush colors and outline.

This is a fast visual check for:

- fill shape
- border thickness
- noise breakup
- current color balance

### Brush Presets

The center column contains visual brush-shape presets.

These presets change the **brush shape behavior only**. They do **not** replace your current brush colors. All preset thumbnails reuse the current `Base`, `Border`, and `Noise` colors so you can judge the shape without losing the active palette setup.

### Brush Settings

The right sidebar contains the main brush settings:

- **Base**: main paint color
- **Border**: outline color used around the brush
- **Noise**: noisy breakup color used inside the brush
- **Brush Size**: overall brush radius
- **Border Size**: thickness of the outer outline
- **Noise Amount**: amount of noisy breakup inside the brush
- **Opacity**: paint strength

The Organic tool is meant to stay compact. The main workflow is selecting a brush shape, choosing the three colors, and adjusting only a few obvious paint controls.

## Toolbar Controls

The Organic toolbar contains:

- **Free / Locked**
- **Clear**
- **Active / Deactive**

### Free / Locked

- **Free** lets you paint on any valid hovered surface.
- **Locked** restricts painting to the current selection, typically the selected sector or active surface.

This is useful when you want to avoid spilling detail onto nearby geometry in dense 3D scenes.

### Clear

`Clear` removes organic paint.

- In **Free** mode it clears all organic paint in the current map.
- In **Locked** mode it clears only the currently locked target.

`Clear` is undoable.

### Active / Deactive

This toggle turns the organic paint layer rendering on or off.

Use it to compare:

- the base surface without painted detail
- the final result with the organic layer enabled

The toggle affects rendering only. It does not delete the painted data.

## 3D Brush Preview

In 3D editing, Organic mode replaces the normal yellow hover marker with a brush-footprint preview.

That preview shows:

- the current brush radius
- the border ring
- the surface position where the stroke will land

This makes it easier to judge placement before committing a stroke.

## Painting Workflow

A simple workflow looks like this:

1. Switch to the **Organic Tool**.
2. Choose a brush-shape preset.
3. Pick `Base`, `Border`, and `Noise` colors.
4. Set `Brush Size`, `Border Size`, `Noise Amount`, and `Opacity`.
5. Use **Free** or **Locked** depending on whether you want broad painting or target-restricted painting.
6. Paint directly in the 3D scene.
7. Toggle **Active / Deactive** to compare the result.

## Undo / Redo

Organic brush painting supports undo and redo as normal map edits.

This includes:

- brush strokes
- `Clear`
- render-layer visibility changes where supported by the current session state

## Notes

- Organic painting is intended for **surface detail**, not for placing trees, bushes, or other large procedural scene objects.
- Builder remains the better tool for reusable placed assemblies and future hybrid mesh / billboard content.
- Organic paint is rendered as a surface layer. It does not generate separate organic geometry.

## Related Pages

- [Overview](/docs/creator/tools/overview)
- [Palette Tool](palette)
- [Builder Tool](builder)
