---
title: "Entity Tool"
sidebar_position: 9
---

The **Entity Tool** (keyboard shortcut **'Y'**) allows you to **select, move, and delete character and item instances** in the map.

Click to select, click-drag to move, and press **Delete** to remove the
selected character or item instance. In **2D Iso**, drag the visible sprite
and release to commit its new map placement. Use Undo to restore the old position.

In 3D editor views, dropped and moved instances snap to the geometry floor under the cursor. When the cursor hits overhead geometry such as a roof awning, placement prefers the floor below that surface so entities can be placed and rendered under cover.

For selected **character instances**, you can rotate facing in 90° steps with:

- **Q**: rotate left
- **E**: rotate right

## Behavior Overview

Click a character or placed item with the Entity Tool to open a scrollable
summary of its routines and event responses over the map. Close it with **×**
or **Esc**. It works while editing and during Play when the editor has input.
Switch out of **Game Input** to inspect an entity during Play. Try selecting
Mara or the North Gate Bonfire in [Wayfarer’s Rest](../wayfarers_rest).

## Authoring

With **Authoring** mode enabled, the lower dock shows the Authoring editor instead of the tile picker for selected character and item instances.

Use:

```toml
title = ""
description = """
"""
```

This is useful for descriptive text, inspect text, and text-adventure style presentation.
