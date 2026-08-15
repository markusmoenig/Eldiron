---
title: "Block Tool"
sidebar_position: 7
---

The **Block Tool** (keyboard shortcut **`B`**) builds 3D regions from modular block stamps.

Use it for fast dungeon, house, corridor, room, and blockout construction that produces editable 3D Geometry Objects.

## What It Does

The Block Tool lets you:

- choose block stamps from the **Blocks** dock
- place practical building pieces such as floors, walls, corners, doorways, stairs, ceilings, columns, and solid blocks
- preview each block with a 3D-rendered icon
- stamp one block with a click
- drag line or rectangle strokes of repeated blocks
- replace or erase existing block instances
- rotate blocks in 90-degree steps
- stack blocks on different grid levels
- make component-aware height and width adjustments before stamping
- stamp clean or damaged geometry variants

Placed blocks become ordinary editable 3D Geometry Objects. You can continue editing them with the Object, Face, Edge, and Vertex tools.

## Blocks Dock

When the Block Tool is active, the lower dock switches to **Blocks**.

The left side shows the block library as rendered isometric previews. The right side is a compact guide for the selected block and the active placement state:

- **Block**: the selected block stamp
- **Size**: the grid footprint and whether the block reacts to height, width, both, or neither
- **State**: current cell size, stack level, rotation, and clean/damaged mode
- **Shape**: remembered height and width expansion values
- **Mouse / Keys / Resize**: the main placement shortcuts

The toolbar above the block library contains:

- **Place / Replace / Erase**: choose the edit operation
- **Clean / Damaged**: choose whether newly stamped blocks are intact or deterministically damaged
- **Line / Rect**: choose the drag stroke shape

The operation controls are left-aligned. The **Clean / Damaged** and **Line / Rect** controls are grouped on the right.

## Placement Grid

The Block Tool uses its own block grid in 3D views. While the tool is active, this grid replaces the normal edit grid for block placement.

Placement and preview are projected onto the active block-grid plane. This keeps stamps aligned even when the camera is zoomed in or the cursor is over existing geometry.

Use the stack shortcuts to move the active block grid up and down.

## Block Stamps

The current starter library includes:

- **Floor Slab**
- **Floor + Wall**
- **Floor + Wall + Ceiling**
- **Floor + Corner**
- **Floor + Doorway**
- **Stairs**
- **Wall**
- **Doorway**
- **Ceiling Slab**
- **Full Block**
- **Large Block**
- **Column**
- **Plain Column**

Composite stamps are intended as the main workflow. For example, **Floor + Wall** places a walkable floor tile and a wall on one cell edge in a single action. Rotation changes which side of the cell receives the wall.

Columns are stamped as editable faceted cylinder geometry, not rectangular posts. **Column** includes base and cap pieces, while **Plain Column** is a clean single shaft.

## Place, Replace, And Erase

The operation buttons control what happens when you click or drag:

- **Place**: add the selected block stamp
- **Replace**: remove existing block instances on the affected cells, then place the selected block stamp
- **Erase**: remove existing block instances on the affected cells

Replace and Erase operate on whole block instances. A doorway, corner, or floor+wall stamp may contain multiple Geometry Objects, but it is removed as one block instance.

## Line And Rectangle Strokes

Click places one block stamp.

Drag with **Line** selected to stamp along a straight grid line between the drag start and end cells.

Drag with **Rect** selected to stamp a filled rectangular area.

The 3D overlay previews the whole pending stroke before mouse-up. Erase strokes are shown with red cell outlines.

## Clean And Damaged Stamps

Use **Clean / Damaged** to choose whether newly stamped geometry should be intact or chipped.

When **Damaged** is active, the Block Tool applies deterministic damage while stamping. The damage is baked into the placed Geometry Objects and stored with a seed, so undo, redo, save/load, and copy/paste do not regenerate a different result.

Damage only affects newly stamped blocks. Existing blocks are not changed when you toggle the mode.

## Component-Aware Sizing

The Block Tool remembers height and width settings. These settings are applied intelligently per component:

- floors ignore height changes
- walls, posts, columns, ceilings, and lintels react to height changes
- width-aware pieces expand by tile increments
- widened doorways grow the opening without thickening the side posts

This means **make higher** affects wall-like pieces but not floor slabs. **Make wider** affects the useful span of pieces such as walls, floors, ceilings, stairs, and doorway openings.

## Shortcuts

When the 3D view has focus:

- **B**: activate the Block Tool
- **R**: rotate the selected block 90 degrees
- **E**: toggle Place / Erase
- **D**: toggle Clean / Damaged stamping
- **[**: move the block grid one level down
- **]**: move the block grid one level up
- **h**: make height-aware components one tile higher
- **Shift + H**: make height-aware components one tile lower
- **w**: make width-aware components one tile wider on each side
- **Shift + W**: make width-aware components one tile narrower

## After Stamping

Blocks are baked as editable Geometry Objects. After stamping, use the direct 3D tools to refine them:

- [Object Tool](object): move, resize, duplicate, delete, and assign sources to whole objects
- [Sector / Face Tool](sector): edit faces and assign tile/material sources
- [Linedef / Edge Tool](linedef): edit edges and draw surface details
- [Vertex Tool](vertex): edit vertices

## Tips

- Use **Floor + Wall** and **Floor + Corner** for room outlines.
- Use **Floor + Doorway** where corridors or rooms connect.
- Use **Rect** strokes with Floor Slab for fast room floors.
- Use **Replace** to correct a run of wall cells without erasing manually first.
- Use **Erase** to remove block instances cleanly, especially multi-piece doorways and corners.
- Use the height shortcuts before stamping taller walls; floors in composite stamps stay thin.

## Related Pages

- [Tools Overview](overview)
- [Object Tool](object)
- [Creating 3D Maps: Geometry](/docs/building_maps/creating_3d_maps)
