---
title: "Paint Tool"
sidebar_position: 12
---

The **Paint Tool** (shortcut **`I`**) paints on terrain in **2D Iso** and on
geometry in 3D isometric, orbit and first-person views. Organic stamps add grass,
flowers, rubble and other details anchored to those surfaces.

## Choose artwork in the Tile Picker

Activate Paint to open the normal Tile Picker. On first use, Eldiron adds three
collections to your project:

- **Paint materials**: the existing 15 material families in their four finishes.
- **Paint brushes**: brick, moss, grass, dirt, cracks, puddles and the other existing procedural patterns.
- **Organic stamps**: the existing generated grass, rubble, leaves, flowers,
  vines, roots, bushes, trees, candles, footprints and mud, plus flower variants.

These are editable **MZ recipes**, using the original Paint generators. Their
source settings and palette bindings are saved inside the recipe. Generated
images are embedded caches; you do not need separate image files.

Click a material or brush to paint. Click an organic stamp to scatter it.
Selecting an entry keeps the map open. Other bitmap tiles and MZ materials in
the Tile Picker can also be used as surface paint.

## Toolbar above the map

The compact toolbar contains the application settings:

- **Paint / Scatter** chooses continuous surface strokes or individual stamps.
  Scatter requires an entry from **Organic stamps**.
- **Erase** removes paint or stamps. Stamp erasure targets the selected stamp kind.
- **Pick** samples painted ground colors.
- **Select** creates a surface selection. **Fill** paints inside it and
  **Clear Sel.** removes the constraint.
- **Coat / Replace**, when painting, blends over the existing surface or replaces
  its covered color and material.
- **Size / Opacity** controls coverage or stamp placement size and transparency.
- **Density / Size jitter / Rotation**, when scattering, controls spacing and
  variation between placed stamps.
- **Edit recipe** opens the selected MZ material explicitly.

Click for one stamp, or drag to scatter. Selections constrain strokes, stamp
placement and erasure. Paint edits support undo and redo.

## Edit a Paint recipe

1. Select a material or stamp in the Tile Picker.
2. Click **Edit recipe** above the map.
3. Select its **Paint Source** or **Stamp Source** operation.
4. Edit the variation seed and palette color slots. Paint sources expose pattern
   scale; brick also exposes detail and variation. Stamp sources expose artwork size.
5. Use the normal MZ material controls for roughness, metallic response, opacity
   and emission, or extend the graph with other MZ operations.
6. Return to the map to keep painting.

The source generator remains in the MZ recipe. A recipe edit updates the tile and
existing strokes or stamps that reference it; it does not relocate them. The
four embedded source frames provide stable variations rather than an animated
thumbnail. Reopening Paint preserves your recipe edits.

Toolbar **Size** changes placement. **Artwork size** in the recipe regenerates
the stamp source; it does not change the saved placement size of existing stamps.

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

Surface paint carries the selected recipe's baked material channels as well as
its color. **Coat** blends these over the underlying material; **Replace** uses
the paint's coverage to replace it. Terrain paint and isometric stamps receive
scene lighting. In 3D, organic stamp billboards use the existing stamp lighting
pass; their surface material overlay is separate from the billboard's color.

## Related

- [MZ materials and meshes](/docs/building_maps/procedural_materials)
- [Palette Tool](/docs/creator/tools/palette)
- [Creating 3D Maps: Geometry](/docs/building_maps/creating_3d_maps)
