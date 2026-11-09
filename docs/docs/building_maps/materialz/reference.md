---
title: Reference, examples and troubleshooting
sidebar_position: 7
---

## Find the right control

| Task | Where to work |
| --- | --- |
| Place an existing mesh | Tile Picker, compact mesh sidebar, Rect tool |
| Open full mesh construction | **Edit mesh** |
| Return to placement | **Back to map** |
| Create a part from a template | Meshes → **New mesh** / **Browse recipes…** |
| Make an independent variation | Browser → **Project**, then create a copy |
| Change the shared asset | Select its Group and edit published controls |
| Change what a control drives | **Publish controls…** → Recipe parameter / Recipe input |
| Calculate a dependent measurement | Publisher → **Formula** |
| Edit a shared project material | Tile Picker or linked material opening control in the mesh editor |
| Edit inline surface values | Mesh recipe → **PBR Material** |
| Define ground-contact geometry | Mesh recipe → **Contour** + **Footprint** |
| Define attachment locations | Mesh recipe → **Connector** |
| Update placement collision from the recipe | **Follow recipe shape** |
| Join nearby mesh parts | **Snap connectors** |
| Remove a placement's parent connection | Select it → **Detach connection** |
| Sculpt a hill | **Terrain** tool |
| Color a terrain surface | Terrain-aware **Paint** workflow |

## Common problems

### I changed a construction value and nothing happened

A published parameter may override that literal value during evaluation. Select the outer Group and change the published control. Inspect the binding if the wrong input is driven. Formula controls are calculated from their dependencies and cannot be changed directly while using a recipe.

### All my walls changed together

They use the same recipe. Use a project recipe copy for an independent variation. A shared material edit similarly affects all recipes that reference that material.

### My wall blocks a rectangle larger than its base

Enable Follow recipe shape and provide a ground-contact footprint. Without one, the fallback is the mesh's bounds. For a tree, use the trunk contour rather than the canopy bounds. Keep Walkable top off for ordinary blocking props.

### My doorway is blocked

Check that its footprint has separate jamb regions and that the lintel has no ground footprint. A single outer rectangle still fills the doorway even if its rendered image is transparent there. Test the clearance with the player's collision radius, not only a point at the opening's center.

### A hole does not show through the mesh

Footprints affect collision and support. They do not cut visible mesh triangles. Build an actual opening in the geometry and add the corresponding footprint hole.

### Bricks stretch or restart at every part

Check both the source material and the mesh's Material Texture projection. Ground materials can use continuous world X/Z coordinates; custom mesh UVs retain their authored layout. Continuous texturing cannot smooth a stepped mesh or merge separate modeled slopes.

### The preview shows grayscale noise instead of my material

It may be displaying the selected operation or a single channel. Choose the material output and Color channel to inspect the final color.

### I cannot find the brick material in a mesh

Select its Material Texture operation. A project material reference opens a shared source that also appears in the Tile Picker. A PBR Material operation is an inline surface with palette color, roughness and metallic controls; it does not have a separate project material tile.

### Parts do not snap

Check Snap connectors and Follow recipe shape. Both parts need static connector caches, matching family keys and opposing directions. Move the tentative placement closer: the search radius is 0.8 world units in three dimensions. Raise Top height when attaching a roof to a wall top. An already occupied connector is unavailable.

### A connected part stopped following

Check that its parent still exists, the recipe still exposes the same connector IDs and families, and Follow recipe shape is enabled. Missing connectors and cycles freeze affected links. Detach and re-place the part to establish a new connection. Editing a closed room's dimensions may leave its closing seam open because the links form a parent forest, not a closed-loop solver.

### A recipe error leaves the previous picture in place

The last valid render and placement cache are retained so an invalid edit does not destroy the scene. Read the recipe error, fix it and allow the next bake to finish. Check formula ranges, contour validity, duplicate bindings and missing upstream references.

## Lighting an isometric scene

MZ recipes define surface colour and shape. Eldiron's sun simulation and local
lights provide illumination in the map preview and game. The iso cache retains
unlit colours, normals, heights and material properties. A deferred SceneVM GPU
pass shares the existing device and scene lights, and evaluates lighting and
mesh shadow maps in the editor and game; changing a light or time
of day updates the scene without regenerating the assets.

The sun's morning, midday, evening and night colours come from the project's
`[simulation]` settings, together with its changing direction. Editor preview
time and game time drive the same lighting. Local Point lights supply their own
colour, intensity, range and flicker.

The relevant Game settings are:

```toml
[render]
ambient_strength = 0.24
point_shadow_softness = 0.075

[simulation]
enabled = true
night_ambient_strength = 0.12
```

`point_shadow_softness` defaults to `0.075`. It is an effective source radius in
world units: larger values soften point-light shadow edges more, especially when
the blocking object is far from the receiving surface. Set it to zero for sharp
point lights. Filtering follows surface geometry and does not blur material
pixels or the whole scene. This setting currently applies to GPU 2D iso lighting.

`night_ambient_strength` is optional and omitted by default. The example uses
`0.12` for darker nights while retaining the daytime ambient strength. It fades
at dusk/dawn and leaves local lights and sun colours alone. Ambient fill colour
is independent of simulated sky colour.

See [Game Configuration: 2D Isometric Lighting](../../configuration/game.md#2d-isometric-lighting)
for the full controls, defaults, a complete example and transition times.

An emissive material makes a lamp's glass visible; a local Point light illuminates
its surroundings. Opaque frame parts still cast shadows. Keep a path open beneath
a hanging lantern: a solid tray very close below the source can project a large
rectangular shadow onto the ground. The Wayfarer's Rest recipe has an open lower
frame, reduced light intensity and soft point shadows for a gradual pool of light.

GPU cast shadows use the actual MZ triangles, their repeating UVs and alpha
masks. The shared terrain triangles cast shadows too, including newly sculpted
hills. Sun shadows use a stabilized orthographic depth map; local Point lights
use six depth views so furniture, walls and hills can block light in every
direction. Openings and disconnected parts stay open, without filling their
bounding boxes. Hidden cutaway roofs continue to block outside light.

The maps follow the shadow enable and strength settings and update with the
sun, local lights, recipe changes, placement and terrain sculpting. Source
triangles and sampled terrain are cached; changing the time does not rebake MZ
materials or sprites. The CPU fallback and older unsynchronized mesh caches
retain height-column shadows. Up to 32 local Point lights cast GPU shadows;
additional local lights still illuminate the scene.

Ordinary bitmap tiles remain sprites. **Preserve original artwork** retains
their supplied shading and colours, including beside dynamically lit MZ meshes.
**World lighting tint** lets a sprite receive live light and mesh/terrain shadows
with its flat surface normal. Neither mode invents a 3D caster from a PNG.
Bitmap-only organic stamps likewise do not become opaque mesh volumes.

MZ **Particle Effect** operations on placed iso meshes are live attachments.
The existing particle simulator advances them in world XYZ, and SceneVM draws
instanced billboards with geometry occlusion. Flame size, rate, colours and
light output remain editable in the mesh recipe. An open window needs an actual
geometry opening; an opaque dark panel still blocks the view and light.

Avoid painting a fixed directional shadow into a tree or ground material: it
cannot follow the sun. Natural pigment, grain and leaf colour variation belong
in the MZ material. See the editable scene sources in
`crates/shared/examples/iso_2d/uo_art/` for a lantern, leafy tree and day/night study.

## Worked source examples

All of these are editable sources in the repository. The mesh browser uses the same recipe files.

| Example | Source |
| --- | --- |
| Ground starter | `STARTER` in `creator/src/procedural_material.rs` |
| Continuous brick material | `crates/materialz/examples/continuous-bricks.material` |
| Wall and corner | `creator/embedded/meshes/recipes/wall-segment.mzgeometry.json`, `wall-corner.mzgeometry.json` |
| Paired roof with derived dimensions | `creator/embedded/meshes/recipes/gable-roof.mzgeometry.json`, `roof-slope.mzgeometry.json` |
| Fence with count, spacing and derived length | `creator/embedded/meshes/recipes/fence.mzgeometry.json` |
| Doorway, posts and holed walkway | `creator/embedded/meshes/recipes/doorway.mzgeometry.json`, `repeated-posts.mzgeometry.json`, `ring-walkway.mzgeometry.json` |

These source paths are for repository users; creating a built-in mesh through the browser does not require editing JSON. Downloadable copies are provided below for studying the format. They are source documents, not a new asset-import UI:

- [Continuous bricks material source](/examples/materialz/continuous-bricks.material)
- [Wall segment recipe](/examples/materialz/wall-segment.mzgeometry.json)
- [Gable roof recipe](/examples/materialz/gable-roof.mzgeometry.json)
- [Fence recipe](/examples/materialz/fence.mzgeometry.json)
- [Doorway recipe](/examples/materialz/doorway.mzgeometry.json)
- [Ring walkway recipe](/examples/materialz/ring-walkway.mzgeometry.json)

## Reproduce the playground

From a native development checkout with the terrain example available under `generated/`:

```sh
cargo test -p eldiron-creator --lib build_iso_published_controls_playground -- --ignored --nocapture
```

The builder first looks for `generated/iso-organic-stamps/OrganicTerrain.eldiron`, then the earlier terrain playground. It writes `generated/mz-iso2d-recipe-playground/MZRecipePlayground.eldiron` and review captures. It leaves the input project intact. The recipe playground adds reusable room parts, a roof, fences, separate posts, a holed walkway and a connector assembly exercise.

This is a developer reproduction harness, not a prerequisite for the tutorials. In a normal project, create the same assets through the browser and assemble them with the editor tools.

## Format and evaluation details

For operation-level source documentation, see `crates/materialz/docs/geometry-prototype.md`. Geometry documents use stable row and operation IDs. References point to earlier rows. Groups retain embedded construction documents, controls, bindings and formulas. Generated geometry, directional art and placement caches can be rebuilt from those sources and their dependencies.

Static Repeat accepts 1–128 copies within the geometry vertex budget. Footprints allow 128 regions, 256 rings and 4096 points. Connectors allow 256 entries per mesh. Prefer a modest number of purposeful parts and connectors over repeating invisible metadata on every decorative detail.

The older [Procedural Materials guide](../procedural_materials) contains additional material composition, image source and preview details. [MZ Recipes on Geometry](../mz_geometry_recipes) describes applying a recipe to selected 3D geometry; that is a separate workflow from placing iso parts.

## Rebuild the procedural art study

The Wayfarer's Rest scene combines a timber-and-plaster cottage, transparent
window openings, live hearth particles, three oak recipes, planted borders and
a winding MZ earth trail over shared sculptable terrain. Its materials use
Guildhall 256 by Eric Johnson from Lospec. Colour variation describes the
surface; the sun, lanterns and hearth supply the illumination.

With the local `generated/mz-iso2d-native/MZIso2D.eldiron` experiment available:

```sh
cargo test -p eldiron-creator --release --lib build_uo_art_study -- --ignored --nocapture
```

This writes the playable `generated/uo-art-study/WayfarersRest.eldiron` and
same-scene day, dusk, night, interior and hill captures. It also walks a player
through the doorway and up/down the hill. Generated projects, captures and
performance logs stay under the ignored `generated/` directory.

To measure the warmed renderer, including GPU uploads and readback:

```sh
cargo run -p eldiron-shared --release --example mz_iso_profile -- \
  generated/uo-art-study/WayfarersRest.eldiron --gpu --size 1280x900 --walking
```

Use `--hour 18.5` or `--hour 23` for dusk and night, and `--no-shadows` to isolate
the shadow cost. The report gives median and 95th-percentile milliseconds after
five warmup frames. These are scene-renderer timings, rather than total editor
frame times.
