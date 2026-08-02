---
title: "Avatars and Wearables"
sidebar_position: 6
---

Procedural Recipes enhance Eldiron's existing frame-based Avatar system. They do not replace Avatar animations, perspectives, marker frames, equipment anchors, or gameplay equipment rules.

An equipped item can project a reusable Material recipe through semantic marker channels. This adds patterns and surface variation while retaining the original Avatar silhouette and shape shading.

## Wearable item binding

The item binding uses ordinary equipment attributes, independent of whether the item is edited in Creator, a ruleset, or a text project:

```toml
[attributes]
slot = "torso"
avatar_channels = ["torso"]
appearance_recipe = "wardrobe/quilted-cloth"
appearance_space = "channel"
appearance_tiling = [2.0, 3.0]
appearance_seed = 419
```

Creator will expose these bindings through item and equipment controls. The table above is also the data form used by current ruleset and Eldiron Source items.

| Attribute | Purpose |
| --- | --- |
| `avatar_channels` | Marker masks changed by the wearable. |
| `appearance_recipe` | Material alias relative to the project recipe catalog. |
| `appearance_space` | Consumer projection grouping: `avatar`, `channel`, or `part`. |
| `appearance_tiling` | Positive horizontal and vertical recipe frequency. |
| `appearance_seed` | Stable per-item variation seed. |

Supported marker channel names are `skin_light`, `skin_dark`, `torso`, `arms`, `legs`, `hair`, `eyes`, `hands`, and `feet`.

Normally clothes should target only their garment channels. Recoloring skin channels with a shirt material will also recolor the face and hands.

## Projection spaces

- `avatar` uses one mapping over the combined selected-channel bounds.
- `channel` evaluates each selected semantic channel independently.
- `part` is accepted for forward compatibility and currently behaves like per-channel projection.

The adapter derives a direction-aware mapping from each rendered marker mask. Left- and right-facing perspectives receive mirrored coordinate mappings where appropriate, allowing repeating bands, cloth, scales, or wear to remain coherent across all eight Avatar directions.

:::caution

Current projection is automatic and bounding-box based. It is suitable for repeating or broad patterns, but it is not yet authored garment UV mapping. Exact heraldry, readable runes, buckles, and asymmetric emblems may distort or shift between perspectives.

:::

## Rendering order

The runtime:

1. builds the normal Avatar frame and semantic marker channels;
2. finds the selected channel coverage;
3. evaluates the material over the selected projection space;
4. composites it through the marker mask while retaining Avatar shape shading;
5. applies procedural headgear and normal equipment presentation;
6. caches the result using the Avatar, equipment, recipe signatures, and appearance settings.

Changing the recipe source, material alias, channels, projection settings, tiling, or seed invalidates the relevant cached appearance.

## Eight-direction limitations and future work

The present adapter makes procedural repeating materials useful in all eight directions without requiring eight separate recipe variants. Fully stable hero-quality equipment will eventually benefit from canonical body-part UVs or authored guides that correspond across directions and animation frames.

That later projection work belongs to the Avatar adapter. The Material recipe should continue to receive coordinates and evaluate appearance without knowing which race, animation, or perspective supplied them.

See [SDF Shapes and Headgear](./sdf_shapes.md) for equipment that adds a new silhouette instead of recoloring an existing marker channel.
