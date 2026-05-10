---
title: "Overview"
sidebar_position: 1
---

This chapter and its sub-sections describe the tools available in **Eldiron Creator**.

## Map Tools Specifics

Some tools are specifically designed for **map editing** and display a common **HUD**. These include:
- **Object Tool**
- **Vertex Tool**
- **Linedef / Edge Tool**
- **Sector / Face Tool**
- **Rect Tool**

The tool strip also contains **mode toggles** below the main map tools:

- **Authoring**
- **Text Play**
- **Palette Tool**

### Terminology

- **Geometry Objects**: Editable 3D objects made from vertices and faces *(edited via the Object Tool)*.
- **Vertices**: Points that define the corners of 2D map geometry or 3D objects *(edited via the Vertex Tool)*.
- **Linedefs / Edges**: Lines that connect vertices in 2D, edges on 3D objects, or face-local 3D surface lines for detail/cutout actions *(edited via the Linedef / Edge Tool)*.
- **Sectors**: Areas enclosed by edges in 2D, or faces on 3D objects *(edited via the Sector / Face Tool)*.

### Navigation

You can navigate the map using:
- **Trackpad**: Swipe to move around.
- **Arrow keys**: Move the view in any direction.
- **Mini-map**: Click on the mini-map in the **Region** section to jump to a location.
- **Mouse wheel / Trackpad + Ctrl (Mac: Command)**: Zoom in and out.

### HUD Overview

![HUD](/img/docs/hud.png)

- The **upper-left corner** of the HUD shows the **current map position**. In 3D views it falls back to the selected geometry position when the cursor is not hovering geometry.
- The numbers **1, 2, ..., 0** represent **grid snap steps**:
  - **1** = one world unit
  - **2** = half a world unit
  - **0** = one tenth of a world unit
- In 3D views, the HUD keeps these shortcut labels compact and shows the active snap step beside them.
- Smaller snap steps help create **detailed** maps, while larger snap steps are useful for **general layouts**.
- In 3D Object mode, the HUD also contains `MOVE / SIZE` controls for the active object gizmo.

### Keyboard Shortcuts

When the **map view** has focus, you can use **number keys (1-0)** on your keyboard to quickly switch between grid snap steps, instead of clicking on the HUD.

In 3D geometry views you can also use:

- `G`: Object Tool
- `V`: Vertex Tool
- `L`: Linedef / Edge Tool
- `E`: Sector / face tool
- `M`: Move object gizmo
- `S`: Size object gizmo

For direct 3D geometry editing shortcuts, see [Object Tool](object).

### Tile Icons And Geometry Modes

The **icons in the upper-right corner** of the HUD are **tool-specific** and allow you to **assign tiles** to the selected geometry using the **Apply** and **Remove** buttons.

With the [Palette Tool](palette), the lower picker area shows the **Palette** dock instead:

- a palette board for selecting and reordering palette entries
- a material inspector for roughness, metallic, opacity, and emissive
- `Apply Color` and `Clear` actions for palette-based assignment

## Direct 3D Geometry Editing

The direct 3D path uses editable geometry objects as the main construction model.

In 3D views:

- **Object Tool** selects whole 3D geometry objects.
- **Vertex Tool** selects object vertices.
- **Linedef / Edge Tool** selects object edges and draws face-local surface lines for ridges, grooves, and cutouts.
- **Sector / Face Tool** selects object faces.

This keeps the familiar 2D tool vocabulary while changing the 3D behavior to direct object, face, edge, and vertex editing.

## Authoring Mode

The tool strip also contains an **Authoring** toggle. When enabled, contexts that would normally show the **Tiles** dock show the **Authoring** dock instead.

Authoring mode lets you enter **TOML metadata** for selected sectors, linedefs, entity instances, and item instances.

For the full workflow and metadata format, see [Authoring](../authoring).

## Palette Mode

The **Palette Tool** is another bottom-row mode toggle. When enabled, contexts that would normally show the **Tiles** dock keep the **Palette** dock visible instead.

Palette mode is used for:

- editing project palette entries
- changing palette material properties
- applying palette-index sources to geometry

For the full workflow, see [Palette Tool](palette).
