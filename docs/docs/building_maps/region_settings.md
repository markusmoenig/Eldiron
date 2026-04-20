---
title: "Region Settings"
sidebar_position: 5
---

Region Settings are stored as TOML and control region-level editor/runtime behavior.

Runtime logic for a region is edited separately from this TOML file:

- **Region / Visual Scripting**
- **Region / Eldrin Scripting**

Use the region scripts for dynamic runtime overrides such as local fog, palette remap, or post changes. Use **Region Settings** for authored/static map settings.

## Terrain

Enable terrain rendering for the region and define the default terrain tile:

```toml
[terrain]
enabled = true
tile_id = "27826750-a9e7-4346-994b-fb318b238452"
```

- `enabled`: turns terrain on/off for the region.
- `tile_id`: default tile used for terrain rendering.
  - accepts UUID, tile alias, or palette index.
  - examples: `tile_id = "27826750-a9e7-4346-994b-fb318b238452"`, `tile_id = "grass_default"`, `tile_id = 2`, `tile_id = "2"`.

## Preview

You can hide sector geometry in the editor preview by name pattern:

```toml
[preview]
hide = ["KeepRoof*"]
```

- `*` is a prefix wildcard.
- `KeepRoof*` matches names like `KeepRoof`, `KeepRoofA`, `KeepRoof_Upper`.

This is useful in isometric editing when you want to hide roof sectors while working on interiors.
These preview filters are editor-only and are not applied in the in-game runtime view.
