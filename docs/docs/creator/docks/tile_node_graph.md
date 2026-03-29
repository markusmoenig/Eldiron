---
title: "Tile Node Graph"
sidebar_position: 5
---

The **Tile Node Graph** editor is Eldiron’s procedural tile authoring system.

It opens when:

- a **node group** is selected in the tile picker
- and you use [Edit / Maximize](/docs/creator/actions/#edit--maximize)

A node group is a first-class tile source asset. It is not just one loose image. A node group defines:

- output group size such as `1x1`, `2x2`, or `3x3`
- tile pixel size per cell
- the procedural graph state
- the generated output tiles

## Workflow

The current graph direction is height-first:

1. **Layout nodes** generate structural fields such as `Height`, `Center`, and `Cell Id`
2. **Height shaping** nodes remap and sculpt that field
3. **Color** nodes map the final field to palette colors
4. **Output** writes color, height, and material values

This keeps the system modular and allows several layout families to feed the same downstream pipeline.

The graph editor also supports **modular layering**. A node graph can import another graph as a reusable layer and combine it with masks or other fields. This makes it possible to build results such as:

- a rock base plus soil overlay
- grass or moss on top of stone
- several reusable material layers mixed in one top-level graph

## Layout Nodes

Current layout families include:

- **Voronoi**: irregular stones and rough organic partitions
- **Bricks & Tiles**: aligned or staggered man-made layouts
- **Disc**: scattered circular or blob-like detail/layout fields
- **Box Divide**: subdivided patterned layouts useful for pavers and floors

These nodes generally expose:

- `Height`
- `Center`
- `Cell Id`

and can be warped or shaped further.

## Output Node

The output node holds graph-level settings such as:

- graph name
- output group size
- tile pixel size
- palette source
- fallback roughness, metallic, opacity, and emissive values

It also receives the final graph outputs:

- `Color`
- `Height`
- optional material channels

`Height` is especially important because Eldiron uses it to generate procedural normals for node-group tiles.

## Importing Layers

Reusable graph layering is done with two pieces:

- **Import Layer**: evaluates another node graph as a node inside the current graph
- **Layer Input**: exposes named field inputs from the imported graph to the parent graph

This lets you build one graph as a reusable material layer and combine it inside another graph.

Typical pattern:

1. Create a reusable graph such as `Stones`
2. Create a second reusable graph such as `Soil Overlay`
3. In a top-level graph, add **Import Layer** nodes for both
4. Feed masks or fields into any exposed **Layer Input** terminals
5. Blend or combine their outputs into the final `Output`

### Resolution Rules

In Eldiron, imported layers are resolved by node-graph name. The resolver is forgiving and accepts:

- exact graph names such as `Layered Stones`
- file-style names such as `layered_stones`
- slug-like names such as `layered-stones`

In the standalone `tilegraph` CLI, imported layers are resolved relative to the current `.tilegraph` file, so a graph can import another graph file from the same folder.

### Imported Layer Outputs

Imported layers expose the same output contract as the main graph output node:

- `Color`
- `Height`
- `Roughness`
- `Metallic`
- `Opacity`
- `Emissive`

This makes them suitable as reusable material layers, not just closed grayscale masks.

## Palette Source

Node graphs can use one of two palette modes:

- **Local**: the graph uses its own embedded palette
- **Project**: the graph uses the current project palette

New graphs default to **Local**, which makes them portable and shareable.

The **Graph** menu also includes **Map To Project Palette**, which remaps existing palette-index usage to the nearest colors in the current project palette.

## Previewing

The graph editor supports:

- small per-node previews
- a tiled background preview of the current selected/output result
- preview opacity control from the **Graph** menu

Node previews can be collapsed if the graph becomes too crowded.

## Applying Node Groups

Generated node groups appear in the tile picker like other grouped tile sources.

They can be:

- selected from the tile picker
- previewed like grouped content
- applied to supported surfaces and geometry just like other tile sources

At runtime, Eldiron uses the generated tiles of the node group, so the node graph becomes a reusable procedural tile source inside the project.

## Example Layering Use

The `tilegraph` crate examples include a simple layered setup:

- `stones.tilegraph`: base stone layer
- `soil_overlay.tilegraph`: overlay layer with a `Layer Input` mask
- `layered_stones.tilegraph`: top-level graph that imports and combines both

That example shows the intended direction of the system: reusable graphs stay separate and the top-level graph composes them.

## Related Pages

- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Pixel Tile Editor](/docs/creator/docks/pixel_tile_editor)
