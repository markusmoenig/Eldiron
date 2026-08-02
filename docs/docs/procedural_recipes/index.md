---
title: "Procedural Recipes"
sidebar_position: 1
slug: /procedural-recipes
---

Procedural Recipes are Eldiron's shared assets for deterministic procedural visuals. A recipe can describe height fields, patterns, colors, renderer materials, architectural features, or reusable 2D shape coverage. Its canonical representation is a small, human-readable `.recipe` document, regardless of whether it is edited visually in Eldiron Creator, as project text, or with an external editor.

The goal is one procedural foundation with small adapters for different consumers. Tiles, Avatar wearables, headgear, and future UI or geometry systems should reuse the same fields and materials instead of developing separate pattern languages.

```text
Recipe document
  fields, patterns, shapes, colors, materials
                    |
        consumer-specific adapter
                    |
       Tile / Avatar / future consumer
```

## Current support

| Area | Status | What recipes provide |
| --- | --- | --- |
| Tiles | Stable | Images, coordinated multi-tile coverage, height, surface materials, collision metadata, and niche geometry. |
| Materials | Stable | Reusable color, roughness, metallic, opacity, emission, and micro-normal definitions. |
| Avatar wearables | Initial support | Recipe materials projected through semantic Avatar marker channels in all eight directions. |
| Avatar headgear | Experimental | An SDF silhouette fitted to the head bounds and colored by a reusable material. |
| General SDF coverage | Experimental | Resolution-independent 2D primitives and boolean operations. |
| Brushes, stamps, UI, particles, and general geometry | Planned | Intended consumers of the shared core; no public binding contract yet. |
| Complete recipe-defined Avatars | Planned | A later extension; existing frame-based Avatars remain the current system. |

:::note

This chapter documents implemented behavior separately from planned architecture. Experimental SDF support is deliberately small and may evolve before v1.

:::

## Design principles

- Recipes are deterministic, bounded, and side-effect free.
- A reusable recipe describes procedural visual data, not gameplay behavior.
- Consumers own placement, dimensions, masks, transforms, and other context.
- Materials and shapes remain independent so one silhouette can use many finishes.
- Project palette choices remain available without coupling recipes to a specific ruleset.
- Existing Tile and Material recipes remain the compatibility base as new consumers are added.

## Authoring surfaces

Recipes are not tied to Eldiron Source. They belong to the project or ruleset recipe catalog and may be consumed by any compatible Eldiron system.

- **Eldiron Creator** is the intended primary visual authoring surface. Recipe assets will be editable from the project tree with live previews and a canonical text view as integration is completed.
- **Text projects** can store `.recipe` documents directly in their recipe catalog. Eldiron Source currently provides this workflow.
- **The command-line tool** validates and renders the same canonical documents, making it useful for previews, automation, and external-editor workflows.

The visual editor and text representation should describe the same recipe program. Moving between them must not create a separate Creator-only or Source-only format.

## In this chapter

- [Getting Started](./getting_started.md) creates and previews a first recipe.
- [Language Fundamentals](./language.md) explains documents, blocks, expressions, domains, and determinism.
- [Materials and Color](./materials.md) covers reusable surface appearance and palette policies.
- [Tiles and Geometry](./tiles_and_geometry.md) covers height-first tiles and placement-time architectural features.
- [Avatars and Wearables](./avatars.md) explains marker channels and eight-direction projection.
- [SDF Shapes and Headgear](./sdf_shapes.md) describes the experimental shape layer.
- [Architecture and Roadmap](./architecture_and_roadmap.md) records the universal-recipe direction without presenting planned features as available.

The crate README remains the exhaustive low-level language reference during this documentation migration. This chapter is the user-facing guide and will absorb that reference incrementally.
