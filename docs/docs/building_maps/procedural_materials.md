---
title: "Procedural Materials"
sidebar_position: 1.5
---

Eldiron includes the MaterialZ (MZ) row editor for procedural textures. A material
is an editable recipe, its project-palette bindings and its baked render data.
Use the same sources for 2D map tiles, 3D floors and walls, and screen artwork.
Ordinary bitmap and animated tiles remain supported.

## Make a procedural tile

1. Open the **Tile Picker** and select a tile or a vacant board cell.
2. Click **Make Proc** to create a procedural material for that selection.
3. The **Procedural Materials** sidebar tab opens automatically when a procedural
   tile is selected. The map remains available for editing.
4. Drag a source from **Sources** into an outlined empty slot. Dropping into the
   first slot of the empty row creates another row.
5. Select an operation to edit its controls in **Props**. Numeric controls support
   both sliding and manual entry. Use the source category menu and search field
   to find noise, shapes, operators, colors and image sources.

Recipes, palette bindings and changes to their settings are saved in the project
and participate in Undo/Redo. The resulting tile can be painted with the
[Rect tool](../creator/tools/rect) or assigned with **Apply Tile**.

## Rows and outputs

Operations run **left to right within a row**, and rows run **top to bottom**.
Place a producer above a row that consumes it. A noise row can feed a later mixer;
you do not need another copy of the noise in each consuming row.

The **Row Input** tab controls how the selected row receives upstream values.
**Props** edits the selected operation or material constants; **Tile** contains
source/tile settings; **Sources** is the draggable operation library.

The PBR outputs are **Color, Roughness, Metallic, Opacity and Emission**, in that
order. Each can connect to a row, or use its constant when disconnected. Height
and Normal are not exposed as material outputs in this Eldiron workflow.

Connections belong to rows. Removing an operation does not disconnect the row's
existing consumers. Selected-row connections are highlighted; the others remain
dimly visible. Click a connection to reveal its origin row and select its first
operation. Click a connected input terminal to disconnect it.

Choose authored colors through the **project palette**, including Color tiles,
ramps and emission colors. See [Palette Tool](../creator/tools/palette) for
switching palettes without replacing recipe references.

## Minimap preview

The material preview occupies the **minimap**, below the sidebar. Its icon buttons
choose a PBR channel and either the **material output** or the **selected
operation/source**. Selecting an entry in Sources can preview it before adding it.
The preview fills the pane so tile boundaries and repeating patterns are visible.
Hover an icon for its meaning.

The **Continuous**, **Repeat** and **Seamless** buttons also change the saved
material tiling setting. They are not merely preview display modes: changes
propagate to placements on the map.

| Mode | Use |
| --- | --- |
| Continuous | Sample across the surface's coordinates, producing variation across neighboring tiles or a large floor/wall. |
| Repeat | Wrap one authored pattern period. Placements share the repeating tile rather than storing another copy for each placement. |
| Seamless | Blend shifted pattern copies near the period's boundaries to join repeating edges. |

Continuous coordinates on a top-down surface follow world X/Z. Other surfaces
use their surface projection, so a wall's axes need not match a floor's. Use
consistent scale and orientation on adjoining surfaces when their patterns must
join.

The current renderer **bakes textures and surface pages**. Continuous variation
does not mean evaluating a material graph for every pixel at game runtime. Large
continuous surfaces can require more bake coverage and loading work than Repeat.

## Compose directly on the map

Material authoring edits a reusable source; composition edits how sources cover
a particular placement or surface.

1. Paint a base tile, for example grass, with the Rect tool.
2. Switch the Rect tool from **Paint** to **Compose**, then click the painted
   tile in the map. In 3D, target the painted surface cell.
3. Choose **Background**, add a layer, then choose **Add material**, for example
   a road. Bitmap tiles work as inputs too.
4. Pick a thumbnail from the shape grid: halves, diagonals, curved corners,
   edge strips, horizontal/vertical roads, crossings, **Auto road**, or **Blend**.
5. Adjust the selected shape. **Soft edge** blends its boundary; **Wobble** adds
   an uneven edge. Width, position and angle depend on the shape.

Each layer combines its material with the result below it. Add more layers for
road markings, dirt, borders or decoration, and reorder them with **Earlier** /
**Later**. Advanced controls expose masks and additional mixing modes. Whole
floor, ceiling and wall-surface compositions remain available where those
surface targets are selected.

Use **Save copy to Tilepicker** to turn the current composition into a reusable
tile. Paint that copy elsewhere instead of rebuilding the same road composition.
Auto road derives its connections from adjacent compatible placements; the other
shape presets retain their authored orientation.

## Images and animation

Drag **Image / Tile Source** from the Image category into a row to use an imported
image or an existing project tile. Select it and choose the source in **Props**.
Linked tiles preserve their alpha and animation/material data and refresh when
their bitmap changes. Use repeating image edges for textures or transparent
edges for stamps. Connect its color and alpha to Color and Opacity, or combine
it with other layers using Alpha Over. Imported bitmap colors are not
automatically converted into palette-index recipe controls.

Animated recipes can use **Loop Flow** for moving ripples or flowing patterns.
Animation is baked into frames and played by the tile animation system; editing
an animation does not introduce a live material VM. Check both preview and game
playback, since tile animation is paced through the project's animation tick.

## Resolution, transparency and gameplay

**Game → Settings → Procedural Materials → Pixels per tile** controls world
material baking globally. The default is **16**; **0** uses smooth evaluation
with 64-pixel storage. See [Game Configuration](../configuration/game#tile-and-appearance-settings)
for this setting, tile backgrounds and independent character/item pixelization.
Item icon recipes use their own 32×32 output, separate from world tile density.

A material's appearance does not assign gameplay semantics. Set **Role**,
**Blocking** and **Gameplay Tags** through tile metadata where needed: water or
rock can block movement, while a painted road need not. Equipment rules and item
state changes likewise remain gameplay data, not material nodes.
