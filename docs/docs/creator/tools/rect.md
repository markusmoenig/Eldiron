---
title: "Rect Tool"
sidebar_position: 6
---

The **Rect Tool** (keyboard shortcut **'R'**) allows you to **directly draw tiles** onto the map.

The Rect tool paints the selected tile into the map grid by creating or updating
rectangular placements. The source can be a bitmap or a procedural material.

### How It Works

- The **Rect Tool automatically creates the geometry** for a **sector** and **assigns the selected tile**.
- Click on the map to **draw**.
- Hold **Shift** while drawing to **erase content**.
- Hold **Alt/Opt** and click to **pick the tile source from the map** into the tile picker.
- Hold **Cmd/Ctrl** while drawing to paint **straight lines** (axis-constrained from the start cell).

The picker works in both **2D** and **3D** Rect workflows. When possible, the picked source is also revealed in the **Tiles** dock.

## Compose Mode

Switch the viewport's **Paint / Compose** control to **Compose**, then click a
painted cell to edit its material layers. Choose a background and an added
material, then select a shape thumbnail such as Road H, Road V, a curved corner,
crossing or Blend. **Soft edge** blends the boundary and **Wobble** varies it.
Bitmap and procedural sources can be combined; additional layers build on the
previous result. This works per tile in 2D and per painted surface cell in 3D.

**Save copy to Tilepicker** creates a reusable composition for further painting.
See [Procedural Materials](/docs/building_maps/procedural_materials#compose-directly-on-the-map).

## Painting in 2D and 3D

- **2D Camera**: The **Rect Tool is the primary tool** for drawing tiles.

- **3D Camera**: Draw tiles directly on 3D surfaces and terrain.

The **Rect Tool** is a fast and intuitive way to add **tiles, decorations, and details** to your maps.
