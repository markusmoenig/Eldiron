# MZ screen material integration

Audit and fixes: 2026-11-06. The six rendering failures found by the initial audit
are fixed. The former failing probes now serve as regressions.

## Current behavior

| Consumer | Behavior |
| --- | --- |
| Screen base and foreground geometry | Baked Art/PBR color and opacity, animation, repeat/stretch, continuous spatial pages and refreshed material pixels. |
| Buttons | Live material references. Default/active visual state is selected independently of the material's animation phase. Rebakes do not recreate widgets or reset UI state. |
| Deco panels | Assigned material references are retained, animated and refreshed. Material-backed panels have transparent backgrounds unless a background color is explicitly configured. |
| Action-bar artwork | Frame, slot and slot-frame tile references resolve current asset pixels and temporal phases; nine-slice borders remain supported. |
| Stat, inventory and equipment artwork | Current baked pixels and temporal frames. A spatial page is no longer mistaken for a temporal frame in explicit item-icon tile references. |
| Explicit profile artwork and command icons | Resolve current material pixels and the selected temporal phase. Generated character/item appearance paths retain their existing behavior. |

MZ recipes remain authoring assets, with normal `TileId` baked runtime tiles.
Screen artwork is flat, using color and opacity; it does not introduce PBR lighting
into UI panels. Roughness/metallic/emission remain available for world rendering.

## Alpha and triangle coverage

The CPU rasterizer now retains straight RGBA in transparent intermediate buffers.
The final buffer composition applies source-over alpha once, including correct
output alpha for overlapping translucent layers. Widget texture layers use the
same convention. Explicitly opaque raster targets still blend against an opaque
background. Half-open triangle edge coverage gives a shared diagonal to exactly
one triangle, removing seams in uniformly translucent rectangles.

The regressions cover a half-transparent color over an opaque background, a
transparent pass, two overlapping layers and final compositing over the viewport.

## Continuous coordinates

Screen geometry uses authored screen-grid coordinates for continuous material
sampling. `Tile::sample_2d` separates spatial page selection from temporal phase,
including negative origins, multi-unit pages and self-contained repeat periods.
Stretched artwork uses a representative tile; repeated artwork samples the screen
surface. The live widget artwork resolver supports stretch, repeat and nine-slice.

Creator includes screen maps when computing material bake coverage and when
invalidating coverage after geometry edits. Region/world coordinate behavior and
GPU material sampling remain unchanged.

## Portable screen files

Screen export writes an `eldiron.screen` version 1 package containing the screen,
referenced baked tiles, editable MZ recipes, composition dependencies, linked
project image tiles and source palette. Embedded MZ document/image data travels
with its recipe. Whole-project saving remains unchanged.

Import prepares a complete project change before applying it:

- Fresh asset/screen/map IDs prevent collisions. Map references, TOML UUIDs, MZ
  document identities, image links and composition references are remapped.
- Duplicate screen names receive a suffix so runtime name lookup stays unique.
- Palette bindings map to the nearest colors in the receiving project's existing
  palette and their resolved MZ constants update immediately. Imported baked
  pixels are remapped too, retaining alpha. Existing project assets and palette
  slots remain intact.
- The import is one undoable operation. Undo removes the imported assets and
  restores the previous editor context; redo restores the same imported IDs.
- New recipe IDs have no old bake keys, allowing Creator to regenerate them using
  the receiving project's current settings.

This package carries screen artwork dependencies. Commands, gameplay logic and
navigation to other screens remain owned by the receiving project. The previous
bare-screen JSON export is replaced rather than migrated.

## Validation

`crates/rusterix/src/client/tests/screen_materials.rs` evaluates actual MZ Art/PBR
Color rows with project-palette bindings and tests the runtime consumers. CPU
cases cover color/opacity, palette rebakes, animation, page selection, alpha,
triangle coverage, stat tiles, inventory icons and command icons. Native cases
exercise `Client::init_screen`, visual-state/animation independence and deco ticks.
They are ignored in the ordinary suite only because constructing `SceneHandler`
requires a native GPU adapter.

```sh
cargo test -p rusterix --lib screen_material_tests --offline -j 2
cargo test -p rusterix --lib screen_material_tests --offline -j 2 -- --ignored --test-threads=1
cargo test -p rusterix --lib material_tests --offline -j 2
cargo test -p eldiron-shared screen_package::tests --offline -j 2
cargo test -p eldiron-creator screen_only_material_bakes --offline -j 2
cargo test -p eldiron-creator screen_import_undo_redo --offline -j 2 -- --ignored
```

Additional regressions cover explicit profile frames, screen-only bake extents,
portable image links, asset collisions, palette constants and import undo/redo.
Browser/mobile interaction and GPU/CPU pixel parity are not established by these
CPU screen tests.

The targeted checks passed: 29 CPU/native regressions across screen consumers,
straight-alpha composition, avatar-signature invalidation, portable screen assets,
screen bake coverage, import undo/redo and GPU pipeline reuse. The workspace and
test targets compile. The GPU consistency test now compares matching centered UV
coordinates rather than comparing the unit tile to a different surface corner.

Hideout-MZ's 2D preservation benchmark also passed: palette slots, editable recipes,
bake keys, tile pixels and picker positions were unchanged. Stationary/pan/zoom
frames averaged 25.27/25.32/25.12 ms (p95 at most 26.07 ms). The first ten redraws
took 1.35 s. These measurements use the same stopped-editor native setup as the
3D validation optimization documented in the unified appearance concept.

The subsequent running-game pass adds bounded raster/composite caches, retained
transparent viewport holes and glyph reuse. In-place artwork edits, palette
rebakes, animation frames, page metadata, geometry and viewport changes invalidate
cached screen output. A source-over regression compares sparse composition with
the ordinary blend for every destination alpha value. HUD equipment substitutions
also reuse parsed rules without freezing live rules or equipment edits. The
running-game benchmark now verifies an active viewport and player; its results
and remaining 30 FPS gap are documented in the unified appearance concept.
