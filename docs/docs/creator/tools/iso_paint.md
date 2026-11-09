---
title: "Paint Tool"
sidebar_position: 12
---

The **Paint Tool** (keyboard shortcut **`I`**) paints directly onto terrain in **2D Iso** and onto region geometry in 3D views.

Use it when the 3D model provides the playable structure, collision, hit testing, and lighting, but you want a more organic hand-authored surface treatment in isometric, orbit, or first-person views.

## What It Paints

3D Paint stores a persistent paint layer on the region. Strokes use stable local surface coordinates that are independent of the surface material's tiled UVs. They render with the geometry instead of becoming camera-dependent editor marks or repeating once per texture tile.

It is intended for:

- moss, grass, dirt, and road breakup
- cracks and ruin detail
- generated brick or tile patterns
- generated arch/trim-like pattern strokes for gates and openings
- surface stamps such as grass, rubble, leaves, flowers, vines, roots, bushes, trees, candles, footprints, and mud
- puddle and wet-look details
- color-only touchups
- material/finish overlays on existing geometry

Paint works in **2D Iso** and in every 3D camera: isometric, orbit, and first-person.

## Terrain in 2D Iso

Select **Paint** while the **2D Iso** camera is active. The camera stays in 2D Iso;
the brush outline and pointer picking follow the visible terrain, including hills.
Choose a brush, size, opacity and project palette colors, then click or drag.
**Erase** reveals the original MZ ground. **Pick** samples a palette color from
painted ground. **Select** defines an area in ground X/Z coordinates for clipped
strokes or **Fill**; **Clear Sel.** removes that constraint.

Paint stays anchored to world X/Z coordinates when you raise, lower or smooth the
landscape with the Terrain tool. It appears in the editor and during play, beneath
characters, trees and buildings. Undo/redo and saving/reopening preserve the layer.
Use Rect Paint/Compose for MZ materials and connected paths alongside these strokes.

For organic detail, choose **Grass**, **Flowers**, or **Rocks / Rubble** and set
material mode to **Stamp**. Click for one stamp or drag to scatter a row. Density
controls spacing; size and rotation jitter vary the generated artwork. These
decorative stamps share the draw order of characters, trees and buildings.
Their ground X/Z coordinates stay fixed while height follows the current terrain,
including raise, lower and smooth edits. Erase targets the selected stamp kind
and keeps working after panning or zooming. Selections constrain placement and
erasure. Stamps support undo/redo, save/reopen and play.

Other organic stamp presets use the same terrain placement. Painting the surfaces
of upright MZ props remains a 3D workflow. Material finish and lighting responses
described below apply to 3D rendering; 2D Iso displays the generated color art.

## Toolbar

The Paint toolbar contains:

- **Draw / Erase / Pick / Select**: paint, remove paint, sample settings, or drag a surface-aligned rectangular selection over visible 3D surfaces
- **Fill**: fill the active selection with the current brush, including generated and organic patterns
- **Clear Sel.**: remove the active selection without removing its paint
- **Visible**: show or hide the authored 3D Paint layer in the editor render
- **Clear All**: remove the current region's 3D Paint layer with undo support
- **No Clip / Surface**: continue across visible 3D surfaces or constrain the whole stroke to the surface where it started

In 3D, choose **Select**, then drag over a wall, floor, or other surface to create a rectangular selection.
The selection follows that surface and snaps to the current grid. Drag inside an existing selection
to move it. While a selection is active, **Draw** and **Erase** affect only the selected area, **Fill**
applies the current brush throughout it, and **Clear Sel.** removes the selection without removing
its paint. Selection and painting changes support undo.

## Brush Presets

The preset strip shows visual brush thumbnails. Selecting a preset updates its saved settings.

Current preset families include:

- **Material Paint**
- **Brick Pattern**
- **Moss**
- **Crack**
- **Grass Detail**
- **Rubble**
- **Leaves**
- **Flowers**
- **Vines**
- **Roots**
- **Bushes**
- **Tree**
- **Candles**
- **Footprints**
- **Mud**
- **Puddle**
- **Dirt / Color Touchup**

Each preset can keep its own size, opacity, material, finish, brush shape, and palette colors.

## Brush Editor

The left side of the dock previews the selected brush and its recipe layers. The brush shape strip selects the stroke mask, such as solid, soft, dirt, speckle, jagged, scratch, or wash.

The right detail view exposes the editable settings for the selected preset:

- size
- opacity
- one or more project palette color slots
- material mode
- pattern settings when the selected preset uses generated patterns
- stamp density, size jitter, and rotation jitter when the selected preset is in stamp mode

Brush colors come from the **project palette**. A multi-color brush exposes multiple `Color` slots so terrain, moss, grass, and pattern brushes can use several related colors instead of a single flat swatch.

## Materials and Modes

The material row selects the material family and finish used by the brush. 3D Paint uses the same high-level material library as tiles and palette entries, including families such as stone, dirt, foliage, water, glass, mirror, emissive, fabric, plastic, skin, bone, and wax.

Material mode controls how the stroke interacts with the underlying surface:

- **Coat**: blend paint and its material response over the existing surface while keeping the original surface visible underneath.
- **Replace**: replace the covered surface color and material. Opacity controls the replacement alpha, including transparent replacement at zero opacity.
- **Stamp**: place generated scene-aware stamp details instead of a continuous stroke.

Stamp brushes honor the currently selected material family and finish. They also write material pixels for their generated shape, so a candle can be emissive, mud can stay wet, foliage can remain foliage, and custom user material choices are preserved per stamp.

## Pattern Brushes

Pattern brushes use surface hit data to generate aligned patterns, such as tiles or staggered bricks, into the paint layer.

Pattern options include:

- tile, brick, or arch mode
- pattern scale
- mortar width
- generated detail
- color variation

The generated pattern is still paint data, so it can be authored quickly without modeling every brick or tile as separate geometry.

Pattern scale controls the generated tile/brick size independently from the brush size. Brush size controls how much area the stroke covers; pattern scale controls how large the repeated pattern is inside that area.

## Stamp Brushes

Stamp mode places individual generated details at the painted hit point. Stamps store a stable surface anchor, owning surface, selected material ID, palette colors, size, opacity, rotation, and variation seed.

This lets stamps:

- stay tied to the scene at a constant world size instead of scaling with the camera
- sort and occlude against scene geometry in the 3D render path
- repaint their material contribution into the material overlay
- erase by nearby stamp, surface clip, and active stamp kind
- remain attached to the same surface in every 3D camera

Drag painting uses the stamp density setting to space repeated stamps. Size jitter and rotation jitter add variation without changing the active brush size.

## Rendering

3D Paint renders in both the editor viewport and the game rendering path when the paint layer is visible.

The render path combines:

- base 3D geometry
- tile and palette-index sources
- high-level material families and finishes
- the authored 3D Paint layer
- generated surface stamps
- stamp material overlays
- the material-aware 3D renderer/post treatment

This keeps the editable 3D structure clean while allowing a more organic, painterly result in every 3D camera.

## Related

- [Palette Tool](/docs/creator/tools/palette)
- [Creating 3D Maps: Geometry](/docs/building_maps/creating_3d_maps)
- [Actions](/docs/creator/actions)
