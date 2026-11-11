---
title: "Paint Tool"
sidebar_position: 12
---

The **Paint Tool** (shortcut **`I`**) paints on terrain in **2D Iso** and on
geometry in 3D isometric, orbit and first-person views. Organic stamps add grass,
flowers, rubble and other details anchored to those surfaces.

## Choose artwork in the Tile Picker

Activate Paint to open the normal Tile Picker. On first use, Eldiron adds two
collections to your project:

- **Paint Brushes**: brick, moss, grass, dirt, cracks, puddles and the other existing procedural patterns.
- **Organic Stamps**: the existing generated grass, rubble, leaves, flowers,
  vines, roots, bushes, trees, candles, footprints and mud, plus flower variants.

These collections stay in the project when you switch tools, so your recipe edits
remain available. Paint does not add a separate library of gray material swatches;
edit the brush recipe's material settings instead.

These are editable **MZ recipes**, using the original Paint generators. Their
source settings and palette bindings are saved inside the recipe. Generated
images are embedded caches; you do not need separate image files.

Click a material or brush to paint. Click an organic stamp to scatter it.
Selecting an entry keeps the map open. Other bitmap tiles and MZ materials in
the Tile Picker can also be used as surface paint.

## Overlay controls

The Paint panel floats over the editor, like the Rect tool controls. It does not
reduce the map viewport. Clicking or dragging its controls never paints the map
behind them.

Use these controls for ordinary painting:

- **Paint / Scatter** chooses continuous surface strokes or individual stamps.
  Scatter requires an entry from **Organic Stamps**.
- **Erase** removes paint or stamps. Stamp erasure targets the selected stamp kind.
- **Pick** samples painted ground colors.
- **Select** creates a surface selection. **Fill** paints inside it and
  **Clear Sel.** removes the constraint.
- **Coat / Replace**, when painting, blends over the existing surface or replaces
  its covered color and material.
- **Size / Opacity** controls coverage or stamp placement size and transparency.
  Use the minus/plus buttons, or click a value to type an exact number.
- **Colors** opens the project palette when you click a swatch. The color slots
  belong to the selected brush or stamp recipe.
- **Material / Finish** chooses the familiar Paint material family and finish:
  stone, dirt, wood, metal, foliage, water, emissive and the other existing families,
  with natural, matte, polished or wet finishes.
- **Brush shape** chooses solid, soft, dirt, speckle, jagged, scratch or wash.
- **Density / Size jitter / Rotation**, when scattering, controls spacing and
  variation between placed stamps. Plants, trees, rocks and candles stay upright;
  whole-image rotation is offered only for flat details such as leaves, mud,
  roots and footprints.
- **Settings** contains surface clipping and, for bricks, pattern type, scale,
  mortar, detail and color variation. Flat stamps also expose rotation here.
- The eye toggles layer visibility.
- **Advanced** explicitly opens the full MZ editor.

Click for one stamp, or drag to scatter. Selections constrain strokes, stamp
placement and erasure. Paint edits support undo and redo.

## Edit colors and materials directly

1. Select a brush or stamp in the Tile Picker.
2. Click one of its **Colors** swatches and select a project palette entry.
3. Choose a **Material** and **Finish**, or change the brush shape.
4. Keep painting in the map. No formula editing is required.

These edits update the same MZ recipe, its picker preview and existing linked
strokes or stamps. They support recipe undo/redo. Size, opacity and scatter
variation remain placement controls, so changing a color does not move or resize
existing artwork.

Stock Paint recipes use the original procedural brush and stamp generators at
the actual brush or placement size. Their picker images are previews; brushes
do not paint repeated thumbnail patches, and stamps do not enlarge a small
thumbnail to replace their native detail.

Use **Advanced** to extend the MZ graph or edit its source parameters. Custom
graph outputs remain authoritative. Ordinary palette and material choices are
available directly in the overlay.

## Terrain in 2D Iso

Paint leaves the **2D Iso** camera active. Pointer picking and the brush outline
follow hills. Paint and stamps keep their world X/Z coordinates when you raise,
lower or smooth terrain, while their height follows the current surface. They
work in the editor and during play, and survive saving and reopening.

Stamps sort with characters, trees and buildings. They are decorative details;
they do not add collision. Use placed MZ meshes when a prop needs its own
walk-blocking footprint.

Use the Rect tool's Compose mode for connected paths and shape-based material
mixing. Paint adds freehand detail over that result. Painting upright prop
surfaces uses the 3D views.

## Materials and lighting

Stock brushes retain the original Paint material families and finishes as well
as their generated color and masks. Custom MZ materials carry their baked material
channels. **Coat** blends these over the underlying material; **Replace** uses
the paint's coverage to replace it. Terrain paint and isometric stamps receive
scene lighting. In 3D, organic stamp billboards use the existing stamp lighting
pass and honor emissive material intensity; their surface material overlay is
separate from the billboard's color.

## Related

- [MZ materials and meshes](/docs/building_maps/procedural_materials)
- [Palette Tool](/docs/creator/tools/palette)
- [Creating 3D Maps: Geometry](/docs/building_maps/creating_3d_maps)
