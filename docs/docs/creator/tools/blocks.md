---
title: "Prefab Tool"
sidebar_position: 7
---

The **Prefab Tool** browses and places reusable linked Prefabs and retains Eldiron's modular construction-stamp workflow.

Outside Orbit mode, its 3D keyboard shortcut is **`B`**. In Orbit, `B` is reserved for one-shot Box Select, so activate the Prefab Tool from the left toolbar.

## What It Does

The Prefab Tool lets you:

- choose project Prefabs and built-in construction stamps from the **Prefabs** dock
- place project Prefabs as lightweight linked instances
- place practical building pieces such as floors, walls, corners, doorways, stairs, ceilings, columns, and solid blocks
- preview each block with a 3D-rendered icon
- stamp one block with a click
- drag line or rectangle strokes of repeated blocks
- replace or erase existing block instances
- rotate blocks in 90-degree steps
- stack blocks on different grid levels
- make component-aware height and width adjustments before stamping
- stamp clean or damaged geometry variants

Project Prefab placements remain linked to their shared UUID-backed source. Built-in construction stamps currently become ordinary editable Geometry Objects; you can continue editing them with the Object, Face, Edge, and Vertex tools.

## Reusable Prefabs

Create reusable assets from selected Geometry Objects in Object mode:

- **Create Linked Prefab** creates the source and replaces the selection with a linked instance.
- **Create Prefab Copy** creates the source while retaining the original geometry.
- **Update Prefab Source** replaces a source from selected Geometry Objects without changing its UUID or existing links.
- **Make Prefab Unique** gives one selected instance a copied source UUID.
- **Unpack Prefab** converts linked instances back into ordinary Geometry Objects.

Maximize a selected project Prefab in the dock to enter its isolated editor. It reuses the normal Object, Face, Edge, Vertex, camera, undo, and 3D Paint paths while hiding unrelated region content. The lower panel provides a hierarchical Prefab / part / Geometry Object tree, part parenting and assignment, pivots, Prefab naming, and Door configuration and preview. Changes write back to the same source UUID, so every linked instance updates.

### Parts, Paint, And Doors

The tree is the source of truth for the Prefab hierarchy. Select a part to rename or reparent it, assign selected Geometry Objects, or derive its pivot from the current object, face, edge, or vertex selection. **Create Part** and **Remove Part** sit together in the Prefab toolbar; removing a part keeps its geometry by moving that geometry back to the root.

Choose the shared **3D Paint Tool** in the isolated editor to replace the lower panel with the same Paint UI used by the region editor. Prefab paint is stored in asset-local coordinates and renders on every linked instance.

For a Door or Gate, put all moving Geometry Objects in one part, set that part's hinge pivot, then use **Make Door / Gate**. The Door component targets the whole part: clicking any visible geometry belonging to it can resolve the interaction, so individual opening faces do not need to be marked. Configure swing or slide motion, single or split leaves, open angle or slide distance, and interaction distance. **Preview Door** tests the authored open state without running the game. At runtime, Open/Close uses the contextual rules interaction, while normal Prefab authoring can still provide Look text.

## Prefabs Dock

When the Prefab Tool is active, the lower dock switches to **Prefabs**.

The dock combines project Prefabs with the built-in block library. Its rendered previews select the asset to place, while the compact state area describes the current construction placement settings:

- **Block**: the selected block stamp
- **Size**: the grid footprint and whether the block reacts to height, width, both, or neither
- **State**: current cell size, stack level, rotation, and clean/damaged mode
- **Shape**: remembered height and width expansion values
- **Mouse / Keys / Resize**: the main placement shortcuts

For built-in construction stamps, the placement toolbar contains:

- **Place / Replace / Erase**: choose the edit operation
- **Clean / Damaged**: choose whether newly stamped blocks are intact or deterministically damaged
- **Line / Rect**: choose the drag stroke shape

The operation controls are left-aligned. The **Clean / Damaged** and **Line / Rect** controls are grouped on the right.

## Placement Grid

The Prefab Tool uses its own block grid for construction stamps in 3D views. While placing these stamps, this grid replaces the normal edit grid.

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

When **Damaged** is active, the Prefab Tool applies deterministic damage while stamping. The damage is baked into the placed Geometry Objects and stored with a seed, so undo, redo, save/load, and copy/paste do not regenerate a different result.

Damage only affects newly stamped blocks. Existing blocks are not changed when you toggle the mode.

## Component-Aware Sizing

The Prefab Tool remembers height and width settings for built-in construction stamps. These settings are applied intelligently per component:

- floors ignore height changes
- walls, posts, columns, ceilings, and lintels react to height changes
- width-aware pieces expand by tile increments
- widened doorways grow the opening without thickening the side posts

This means **make higher** affects wall-like pieces but not floor slabs. **Make wider** affects the useful span of pieces such as walls, floors, ceilings, stairs, and doorway openings.

## Shortcuts

When the 3D view has focus:

- **B**: activate the Prefab Tool in Iso and First Person; in Orbit, arm Box Select instead
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

Built-in construction stamps are baked as editable Geometry Objects. After stamping, use the direct 3D tools to refine them. Project Prefabs should instead be edited through their isolated shared-source editor unless you first use **Unpack Prefab**.

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
