---
title: "Architecture and Roadmap"
sidebar_position: 8
---

Procedural Recipes are intended to become a consumer-neutral procedural kernel. The current Tile, Material, and SDF document types are the compatibility base for that evolution.

## Architectural boundary

```text
Recipe core
  parsing and validation
  deterministic fields and patterns
  colors and surface materials
  SDF coverage
  consumer-neutral render surfaces
              |
              +-- Tile / project adapter
              +-- Avatar wearable adapter
              +-- Avatar headgear adapter
              +-- future UI adapter
              +-- future paint and stamp adapters
              +-- future geometry adapter
```

The core must not depend on Avatar, UI, map, ruleset, or renderer-specific entity types. A consumer supplies dimensions, coordinate mapping, seed, time, palette, and any semantic masks. It then converts the typed result into its own representation.

## Ownership

- A recipe owns procedural visual logic and named outputs.
- A Material owns reusable surface appearance.
- An SDF owns reusable coverage or silhouette.
- A consumer binding owns context such as equipment channels, map placement, widget state, or target dimensions.
- A ruleset or item owns gameplay meaning.

For example, a helmet item owns its head slot and armor value, its SDF owns the silhouette, its Material owns the hammered-metal appearance, and the Avatar adapter owns fitting those assets to each frame.

## Stability policy

New work should preserve existing Tile and Material documents. Shared primitives should be generalized underneath those documents rather than replacing their syntax prematurely.

When a new consumer is added:

1. define the small typed output contract it needs;
2. define the coordinate and seed context it supplies;
3. reuse existing recipe outputs where possible;
4. keep consumer-specific metadata outside the recipe;
5. add deterministic previews and cache signatures;
6. document whether the binding is stable or experimental.

## Planned directions

These are design goals, not currently supported authoring contracts:

- richer shared SDF primitives, transforms, repetition, and smooth operations;
- canonical Avatar part coordinates and optional authored UV guides;
- recipe-driven buttons, panels, borders, and icons with UI state inputs;
- procedural brushes and stamps using shared coverage and materials;
- particle-emitter outputs;
- consumer-neutral geometry output with extrusion and placement adapters;
- complete recipe-authored Avatar parts while retaining current atlases;
- a visual Recipe editor backed by the same canonical text format.

The guiding constraint is that a feature should strengthen the shared procedural vocabulary. It should not create another isolated procedural subsystem that only one Eldiron tool can understand.
