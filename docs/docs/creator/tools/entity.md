---
title: "Entity Tool"
sidebar_position: 8
---

The **Entity Tool** (keyboard shortcut **'Y'**) allows you to **select, move, and delete character and item instances** in the map.

Click to select, click-drag to move and the **delete** key for deleting the currently selected character or item instance.

In 3D editor views, dropped and moved instances snap to the geometry floor under the cursor. When the cursor hits overhead geometry such as a roof awning, placement prefers the floor below that surface so entities can be placed and rendered under cover.

For selected **character instances**, you can rotate facing in 90° steps with:

- **Q**: rotate left
- **E**: rotate right

## Authoring

With **Authoring** mode enabled, the lower dock shows the Authoring editor instead of the tile picker for selected character and item instances.

Use:

```toml
title = ""
description = """
"""
```

This is useful for descriptive text, inspect text, and text-adventure style presentation.
