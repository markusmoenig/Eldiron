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
- **Terrain Tool**
- **3D Props Tool**
- **Paint Tool**

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

Navigation depends on the active camera:

- **Physical mouse on every platform**: the existing wheel, right-drag, modifier-drag, and arrow-key controls remain unchanged.
- **macOS trackpad in Orbit**: two-finger scroll pans, pinch zooms, and primary click-drag on empty space or the ViewCube orbits. `Command` + two-finger scroll is a zoom fallback.
- **macOS trackpad in Iso**: two-finger scroll pans and pinch zooms.
- **Linux Wayland touchpad**: two-finger scroll pans in Orbit and Iso; hold `Ctrl` while scrolling to zoom. `Ctrl` + scroll also zooms the 2D editor, profile editor, and node canvas. Native Linux pinch gestures are not delivered by the current winit 0.30 backend.

- **First Person**: `WASD` moves whenever the viewport is focused; hold right mouse to look or press `Space` to toggle touchpad look. First Person does not pan.
- **ViewCube in Orbit**: click a visible face to align the camera to that axis.
- **Mini-map**: click the mini-map in the **Region** section to jump to a location.

For touchpad zoom, click the canvas to focus the window, hold `Ctrl`, and move
two fingers **up or down** on the touchpad while the pointer stays over the
canvas. This is a scroll gesture, without holding a mouse button. Horizontal
scroll alone does not change zoom.

If Ctrl + vertical scrolling still pans or does nothing, builds with input
diagnostics can be launched from a terminal using
`ELDIRON_INPUT_DEBUG=1 ./eldiron-creator 2>input.log` (or substitute the AppImage
filename). Test ordinary scrolling and Ctrl + vertical scrolling in both the
editor and node canvas. The log records modifier flags, scroll deltas, and the
hovered widget, without recording typed text. Logging is off by default.

See [Camera Actions](../actions#camera-actions) for the complete per-camera controls.

### HUD Overview

![Current map HUD showing grid snap steps and a tile assignment slot.](/img/creator/map-hud.png)

- The **upper-left corner** of the HUD shows the **current map position**. In 3D views it falls back to the selected geometry position when the cursor is not hovering geometry and displays enough decimal precision for the smallest grid snap steps.
- In 2D, 2D Iso and 3D geometry tools, the numbers **1 ... 6** represent **grid snap steps**:
  - **1** = one world unit
  - **2** = half a world unit
  - **3** = one quarter of a world unit
  - **4** = one eighth of a world unit
  - **5** = one sixteenth of a world unit
  - **6** = one thirty-second of a world unit
- In 3D views, the HUD keeps these shortcut labels compact and shows the active snap step beside them. The **, / .** shortcuts step through the same snap subdivisions.
- Smaller snap steps help create **detailed** maps, while larger snap steps are useful for **general layouts**.
- In 3D Object mode, the HUD also contains `MOVE / SIZE` controls for the active object gizmo.

### Keyboard Shortcuts

When the **map view** has focus, you can use **number keys (1-6)** on your keyboard to quickly switch between grid snap steps, instead of clicking on the HUD.

In 3D geometry views you can also use:

- `O`: Object Tool
- `V`: Vertex Tool
- `E`: Linedef / Edge Tool
- `F`: Sector / Face Tool
- `B`: one-shot Box Select in Orbit mode; 3D Props Tool in Iso and First Person
- `M`: Move object gizmo
- `S`: Size object gizmo

These 3D tool shortcuts are resolved contextually, so in-tool commands such as vertex fill (**F**), edge-loop selection (**L**), object rotation (**R**), and tile application (**T**) keep working when their matching geometry is selected. Defaults can be changed from **Game / Shortcuts**.

The **Game / Shortcuts** document stores overrides by stable action id:

```toml
[shortcuts]
"selection.box" = "B"
"tool.object" = "O"
"tool.vertex" = "V"
"tool.edge" = "E"
"tool.face" = "F"
```

For direct 3D geometry editing shortcuts, see [Object Tool](object).

### Tile Icons And Geometry Modes

The **icons in the upper-right corner** of the HUD are **tool-specific** and allow you to **assign tiles** to the selected geometry using the **Apply** and **Remove** buttons.

With the [Palette Tool](palette), the lower picker area shows the **Palette** dock instead:

- an editable project palette board
- Lospec/local palette switching with a before/after preview
- material and finish selectors for project palette entries
- `Apply Color` and `Clear` actions for palette-based assignment

World materials, MZ appearances and UI use this one project palette. For world
tiles, **Make Proc** opens the [procedural material editor](/docs/building_maps/procedural_materials);
the Rect tool's Compose mode edits layered material shapes on the map.

With the [Paint Tool](iso_paint), **Paint Brushes** and **Organic Stamps** are editable MZ recipes in the Tile Picker. Brush operations, palette colors, material finishes and pattern settings live in the floating viewport overlay.

With the [3D Props Tool](blocks), the lower area shows the **Prefabs** browser: rendered previews for built-in stamps, linked project Props and static **MZ Meshes**. The floating viewport overlay contains placement operations, rotation, grid/size settings and explicit editing controls. Selecting a thumbnail keeps the map open.

## Direct 3D Geometry Editing

The direct 3D path uses editable geometry objects as the main construction model.

In 3D views:

- **Object Tool** selects whole 3D geometry objects.
- **Vertex Tool** selects object vertices.
- **Linedef / Edge Tool** selects object edges and draws face-local surface lines for ridges, grooves, and cutouts.
- **Sector / Face Tool** selects object faces.

This keeps the familiar 2D tool vocabulary while changing the 3D behavior to direct object, face, edge, and vertex editing.

## Terrain Brush

The **Terrain** hill icon selects the landscape brush. Choose **Raise**,
**Lower** or **Smooth** and drag across the map, or hold to build up the effect.
Use the panel's minus/plus buttons for **Radius** and **Strength**, or **[ / ]**
for radius. Hold **Shift** to lower temporarily. Lower stops at height zero.
**Esc** cancels the current stroke; undo/redo restores entire strokes.

The brush follows the shared terrain in top-down, 2D Iso and 3D views, including
existing hills made with control vertices. MZ ground materials and painted paths
follow the surface; buildings, stairs and other rigid parts retain their authored
geometry. Stonefall uses ordinary editable geometry.

## Authoring Mode

The tool strip also contains an **Authoring** toggle. Clicking it opens the **Authoring** dock; while enabled, contexts that would normally show **Tiles** show **Authoring** instead.

Authoring mode lets you enter descriptive **TOML metadata** for sectors, linedefs, geometry objects, linked prefabs, and character and item templates. Gameplay configuration and logic use Entity and Behavior Nodes.

For the full workflow and metadata format, see [Authoring](../authoring).

## Text Play

The **Text Play** toggle offers typed gameplay commands in the Game view, or in a dock while editing a running region. Input intents come from the character's Entity Nodes, including instance overrides. Prompt answers appear as numbered choices; enter the number and press Return, or enter **0** to cancel. Expired choices and choices outside conversation range cannot be selected.

Authored sectors and connections provide room descriptions and named exits. Scenes with only Geometry Objects show the region name and nearby characters and items instead; inventory, stats and targeted commands still work. This proximity view does not infer room boundaries or named exits from walls. Add authored sectors and connections for room-to-room text navigation.

## Palette Mode

The **Palette Tool** is another bottom-row mode toggle. When enabled, contexts that would normally show the **Tiles** dock keep the **Palette** dock visible instead.

Palette mode is used for:

- editing project palette entries
- changing palette material and finish metadata
- applying palette-index sources to geometry

For the full workflow, see [Palette Tool](palette).

## Prefabs And Block Building

The **3D Props Tool** browses and places reusable linked project Prefabs and also retains the built-in construction-stamp workflow for rooms, corridors, dungeon layouts, houses, and blockouts.

Project Prefabs are UUID-backed shared sources: editing the source updates linked instances, while each placement keeps its own transform and runtime state. Built-in construction stamps still use the dedicated block grid, click/line/rectangle placement, whole-instance replace/erase, clean or damaged variants, rotation, stack levels, and component-aware sizing.

Static MZ mesh recipes can also be placed directly from **MZ Meshes**, without a Prefab wrapper. Placement, source editing and published controls are in the viewport overlay; the lower dock is the asset browser. Use the Tile Picker in 2D and 2D Iso.

For the full workflow, see [3D Props Tool](blocks).

## Paint Mode

The **Paint Tool** paints terrain in **2D Iso** and surfaces on 3D region geometry.
In 2D Iso, strokes and their brush outline follow the hills. Raise, lower or smooth
the terrain and the paint remains anchored to the same ground coordinates. Paint
appears during play and supports erase, selections, fill, undo and save/reopen.
Grass, flowers and rocks/rubble also work as organic **Stamp** brushes in 2D Iso:
their positions stay fixed while their height follows sculpted terrain. They draw
alongside characters and props. Surface material responses use the 3D workflow.

For the full workflow, see [Paint Tool](iso_paint).
