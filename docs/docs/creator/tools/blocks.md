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

Project Prefabs are Eldiron's reusable 3D assets. One source asset owns its Geometry Objects, part hierarchy, pivots, paint, support surfaces, and behavior. Regions contain lightweight instances that refer to the source through its stable UUID.

This distinction is important:

| Source asset | Linked instance |
| --- | --- |
| Shared geometry, materials, paint, parts, pivots, support surfaces, and behavior | World position, rotation, scale, and runtime state for one placement |
| Edited in the isolated Prefab editor | Selected and positioned in a region |
| One stable project UUID | Its own stable instance UUID plus the source UUID |
| A source edit updates every linked placement | Moving one instance does not move or edit the others |

Built-in construction stamps and project Prefabs share the Prefab Tool, but they have different results. A project Prefab is placed as a linked instance. A built-in floor, wall, stair, doorway, or similar stamp is baked into ordinary editable Geometry Objects.

### Create A Project Prefab

Prefab creation is an Object-mode operation in a normal 3D region:

1. Model the asset from one or more Geometry Objects.
2. Switch to the **Object Tool** and select every object that belongs to the asset.
3. In the action list, choose either **Create Linked Prefab** or **Create Prefab Copy**.
4. Eldiron creates a project asset with a stable UUID and selects it in the Prefabs dock.
5. Rename and refine the source in the isolated Prefab editor.

The creation actions differ as follows:

- **Create Linked Prefab** moves the selected content into a reusable source and replaces the region selection with one linked instance at the same location. Surface-clipped 3D Paint owned by the selected objects is moved into asset-local Prefab paint so it renders on every linked instance.
- **Create Prefab Copy** creates a reusable source but leaves the selected region objects untouched. Those original objects remain independent geometry and no longer follow the Prefab source.

The source is localized around the bottom center of the selected geometry. This gives the Prefab a practical placement origin while the replacement instance remains at the original world position.

New assets receive an automatic name such as `Prefab 1`. To rename one, open its isolated editor and edit the **Prefab** field in the lower inspector. Renaming changes only the display name; the UUID, instances, geometry, paint, and authored behavior remain linked.

### Place A Project Prefab

1. Activate the **Prefab Tool**.
2. Select the project Prefab in the **Prefabs** dock.
3. Move the pointer over the desired 3D placement surface.
4. Click to place a linked instance.

Place as many instances as needed. Every placement keeps an independent transform and runtime state, but all resolve the same source asset. Editing the source later updates them without recreating the placements.

To reposition an existing linked instance, select any visible part of it with the **Object Tool**, press **M** if necessary, and drag the move gizmo or drag the selected instance directly. Moving changes only that placement's world transform. The **Size** operation is intentionally unavailable for linked instances because geometry dimensions belong to the shared source; edit the source, make the instance unique, or unpack it when its shape must change independently.

### Open And Close The Isolated Editor

Select a project Prefab in the Prefabs dock and use the top-right **Edit / Maximize Dock** control (`Cmd/Ctrl + [`) to open it. Built-in construction stamps are not project source assets and cannot be opened this way.

The isolated editor:

- displays only the selected Prefab source and a neutral preview grid
- starts in Orbit view and frames the asset automatically
- reuses the normal Object, Face, Edge, Vertex, camera, ViewCube, gizmo, action, undo, and redo paths
- disables unrelated region content and editor post-processing while it is open
- writes changes back to the same Prefab UUID
- provides Prefab-specific hierarchy, pivot, Door, paint, tile, palette, and support-surface controls

Use **Restore Dock** (`Cmd/Ctrl + ]`) to return to the normal region layout. Eldiron restores the previous region and camera context and returns to the Prefab Tool.

Advanced source editing happens in this isolated editor. A linked instance in a region can be positioned, made unique, or unpacked, but its shared Geometry Objects are not edited directly in place.

### Editor Modes

The lower section follows the active tool so each workflow has one shared UI implementation:

- **Parts**: Object, Face, Edge, and Vertex tools show the Prefab hierarchy, part properties, support surfaces, and Prefab actions.
- **3D Paint**: the **3D Paint Tool** shows the same brush and layer UI used by the region editor, but writes asset-local Prefab paint.
- **Tiles**: the **Tile Picker Tool** shows the normal Tiles dock for applying tiles and materials to the Prefab's selected faces or objects.
- **Palette**: the **Palette Tool** shows the normal Art Palette editor and lets Prefab geometry use the same project palette workflow.

The contextual action list remains available beside the editor. Region-only actions, post-processing controls, and tools that do not apply to a Prefab source are hidden.

### Model The Source

Use the Object, Face, Edge, and Vertex tools exactly as in the normal 3D editor. Object editing shortcuts such as **M** for move and **S** for size operate on source geometry, and camera movement keeps the transform handles aligned to the current view.

The same contextual modeling actions are available where applicable. Common Prefab-building operations include:

- **Create Box** to attach a box fitted to the selected surface
- **Create Unit Box** to attach a centered `1 × 1 × 1` primitive without fitting its size
- duplicate selected Geometry Objects for repeated legs, slats, handles, or trim
- edit faces with extrude, inset, subdivide, delete, texture, and opening actions
- edit edges, contours, and vertices with the normal direct-geometry tools
- assign tiles, materials, Art Palette colors, and 3D Paint

See [Object Tool](object), [Sector / Face Tool](sector), [Linedef / Edge Tool](linedef), [Vertex Tool](vertex), and [Creator Actions](/docs/creator/actions) for the complete shared modeling operations.

### Prefab Hierarchy And Parts

The hierarchy tree is the source of truth for how the Prefab is assembled:

```text
Prefab
  Part
    Geometry Object
    Support Surface
    Child Part
```

A new directly authored Prefab begins with one part containing all selected Geometry Objects. Extra parts are useful when geometry needs its own name, parent, pivot, support surfaces, or motion. A static table can remain one part; a door leaf, drawer, lid, or lever normally needs a distinct moving part.

Selecting a part exposes these properties:

- **Prefab**: renames the complete reusable asset without changing its UUID.
- **Name**: renames the selected logical part without changing its stable identity.
- **Parent**: assigns another part as the parent. Child parts follow their parent's transformation and motion. Cyclic parenting is rejected.
- **Selected Objects**: moves the currently selected Geometry Objects into this part. Objects are reassigned, not duplicated.
- **Pivot**: displays the part's stored Prefab-local pivot. It is intentionally read-only; use **Set Pivot** to derive it from the current 3D selection.

The top Prefab toolbar provides:

- **Create Part**: create a new logical part from the currently selected Geometry Objects. The selected objects move into the new part.
- **Remove Part**: remove the selected logical part without deleting its Geometry Objects. The objects move to another remaining part. Support surfaces and behavior owned specifically by the removed part are removed, and a Prefab must always retain at least one part.
- **Set Pivot**: set the selected part's pivot to the center of the selected vertices, edges, faces, or objects.
- **Create Surface** and **Remove Surface**: create an item-placement area from selected faces or remove the selected definition.
- **Make Door / Gate** and **Preview Door**: configure and test standard Door behavior.

Clicking a Geometry Object in the tree selects it in the 3D view. Clicking a support surface selects all of its referenced faces, making its exact area visible with the normal Face selection overlay.

When a support surface is selected, **Surface Settings** appears in the inspector settings list and opens its anchored property popover.

### Set A Part Pivot

The pivot controls part motion. For a swinging door, it must lie on the hinge axis rather than at the center of the leaf.

1. Select the part in the hierarchy.
2. Switch to Vertex, Edge, Face, or Object mode.
3. Select the geometry whose center represents the desired pivot. For a door hinge, select the vertical hinge edge or its two vertices.
4. Click **Set Pivot**.

The stored Pivot field updates in Prefab-local coordinates. Rotating a Door part then keeps the hinge edge fixed while the rest of the leaf moves around it.

### Paint A Prefab

Choose the **3D Paint Tool** while the isolated editor is open. The lower section switches to the complete shared Paint UI, including its brush, layer, and palette controls.

Prefab paint is stored in source-local coordinates and is resolved against stable geometry identities. Painting the source therefore appears on every linked instance, including differently positioned or rotated placements. Returning to the Object, Face, Edge, or Vertex tool switches the lower section back to Parts without discarding the paint.

Tiles and materials are still face properties rather than 3D Paint. Use the Tile Picker or Palette Tool for those sources, and 3D Paint for brush-based surface detail.

### Create A Door Or Gate

A standard Door is behavior attached to a complete moving part. You do not select interaction faces: clicking any visible geometry in the controlled part can resolve the Door.

For a single swinging door:

1. Create or fit the door geometry. For a Wall Tool opening, select the opening and run **Create Fitted Geometry** directly. For ordinary irregular geometry, select one rim edge, press **C** to select its contour, and run **Create Fitted Geometry**; it derives the opposite contour from the connected reveal faces.
2. Convert the new object to a linked Prefab and open its isolated editor.
3. If the Prefab also contains a frame or other static geometry, select the leaf objects and click **Create Part** so only the leaf moves. A Prefab containing only the leaf can use its existing part.
4. Select the leaf's hinge edge or hinge vertices and click **Set Pivot**.
5. Set **Leaves** to **Single** and **Motion** to **Swing**.
6. Set **Open Angle**. Positive and negative angles open in opposite directions; the valid range is `-180` through `180`, excluding zero.
7. Set **Usage Distance**, then click **Make Door / Gate**.
8. Click **Preview Door** to toggle a non-destructive open/closed preview.

For a sliding door, choose **Slide** and set **Slide Distance**. Open Angle is ignored. Fitted geometry supplies its motion axis when available.

For a split door or gate, select exactly two fitted leaf Geometry Objects, choose **Split**, and click **Make Door / Gate**. Eldiron assigns or creates two moving parts, derives their outer pivots and motion direction from the fitted geometry, and binds both leaves to one shared open/closed state.

The Door settings are:

- **Leaves**: one moving part or two synchronized fitted leaves.
- **Motion**: swing around the part pivot or slide along the fitted motion axis.
- **Open Angle**: signed swing angle in degrees.
- **Slide Distance**: Prefab-local travel distance for Slide motion.
- **Usage Distance**: maximum horizontal distance from which the player may operate the Door.

**Preview Door** changes only the editor preview. It does not modify the Prefab's authored closed geometry and closes automatically when geometry editing resumes.

At runtime, the existing authoring and rules system remains in control. **Look** can target any visible part of any Prefab and show its authored description, whether or not the Prefab has a component. **Use**, **Open**, and **Close** first pass through contextual authoring/rules resolution and may then continue into the built-in Door behavior. This allows a Door to show Look text, reject a locked interaction through rules, or perform authored logic without requiring marked faces.

### Create A Support Surface

A support surface marks exact Prefab faces on which items may be placed, such as a tabletop, shelf, counter, altar, bed, or window ledge. It references existing faces without copying or changing their geometry.

To author one:

1. Open the Prefab's isolated editor and switch to the **Sector / Face Tool**.
2. Select one or more coplanar faces that form the usable area.
3. Ensure all selected faces belong to the same logical part. They may span several Geometry Objects within that part.
4. Click **Create Surface** in the Prefab toolbar.
5. Edit the surface in the anchored settings popover.

The new surface appears below its owning part in the hierarchy. Select it to restore its face selection and make the usable area visible. Click **Surface Settings** to reopen the popover, or **Remove Surface** to remove only the placement definition. Removing a surface never deletes or modifies its Geometry Objects.

The settings are:

- **Surface Name**: label shown in the hierarchy and available to later authoring or scripting tools.
- **Snap Spacing**: item-placement grid in Prefab-local units. Use `0` for continuous placement.
- **Allowed Tags**: comma-separated item tags. Leave empty to allow any item. Authored items automatically offer `placeable` and their item type in addition to explicit tags.
- **Capacity**: maximum number of items on the surface. Leave empty for no explicit limit.
- **Occupancy**:
  - **Reject Overlap** rejects an occupied snapped position.
  - **Allow Overlap** permits multiple items at the same position.
  - **Single Occupant** permits only one item on the entire surface.

**Create Surface** is enabled only when a valid face selection exists. **Surface Settings** and **Remove Surface** are enabled only while a surface is selected in the hierarchy. Creation is rejected if the faces are not coplanar or span more than one part.

### Place Items On A Support Surface

Support surfaces are used in the normal region editor, not in the isolated source editor:

1. Place a linked instance of the table, shelf, or other supporting Prefab.
2. Use the **Entity Tool** to drag an existing item, or drag a new item from the sidebar.
3. Move it over the authored support surface and drop it.

Eldiron snaps the position, validates allowed tags, capacity, and occupancy, and stores a relationship containing the Prefab instance UUID, support-surface UUID, and a surface-local transform. The item therefore follows the linked Prefab when the instance moves and is recalculated when the source or instance transform changes.

If placement is rejected, the status bar reports whether the item is not allowed, the surface is full, or the position is occupied.

### Update, Detach, Or Branch A Prefab

These contextual Object-mode actions operate in a normal 3D region:

- **Update Prefab Source** replaces the currently selected project Prefab's authored source with the selected Geometry Objects while preserving the asset UUID and all linked instances. This is a source rebuild: existing parts, support surfaces, interaction targets, components, and asset-local paint are cleared because their stable geometry references belonged to the old source. Use the isolated editor for ordinary incremental changes.
- **Make Prefab Unique** duplicates the source behind exactly one selected linked instance, creates a new UUID, and relinks only that instance. Later source edits no longer affect placements of the original asset.
- **Unpack Prefab** resolves selected linked instances into ordinary editable Geometry Objects and removes those placements' source links, support-surface placement relationships, and Prefab behavior. The reusable source asset and its other instances remain unchanged.

Use **Make Prefab Unique** when one placement should become a separately reusable variant. Use **Unpack Prefab** when the result should stop being a Prefab entirely.

### Example: Build A Table Prefab

1. Create the tabletop with **Create Box** or **Create Unit Box**, then size it.
2. Create one leg, duplicate it, and position the copies.
3. Select the tabletop and legs in Object mode and run **Create Linked Prefab**.
4. Open the selected Prefab in the isolated editor and rename it in the **Prefab** field.
5. Apply tiles, materials, palette colors, and 3D Paint as needed.
6. Switch to Face mode, select the top face or coplanar top faces, and click **Create Surface**.
7. Give the surface a useful name such as `Tabletop`, then configure snapping, tags, capacity, and occupancy.
8. Restore the normal editor and place additional linked table instances from the Prefabs dock.
9. Drag an item onto any table instance with the Entity Tool.

The table does not need separate parts unless a section must move independently or needs its own hierarchy, pivot, or semantic ownership.

### Current Boundaries

- Project Prefabs currently use directly authored Geometry Objects. Procedural Recipe-backed Prefab sources are planned but are not yet part of this editor workflow.
- Built-in construction stamps are editable baked geometry, not linked project Prefabs.
- Linked source geometry is edited in the isolated editor rather than directly inside a region.
- **Update Prefab Source** is a destructive source rebuild for Prefab-specific semantics; it is not the normal way to save incremental source edits.
- **Unpack Prefab** removes Prefab behavior and support relationships from those placements.
- Stable UUIDs make project assets suitable for future library, packaging, import/export, and database sharing, but those sharing workflows are not yet exposed by the Creator UI.

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
- [3D Paint Tool](iso_paint)
- [Palette Tool](palette)
- [Creator Actions](/docs/creator/actions)
- [Creator Authoring](/docs/creator/authoring)
- [Creating 3D Maps: Geometry](/docs/building_maps/creating_3d_maps)
