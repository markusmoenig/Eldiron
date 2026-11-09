# MZ 2D isometric experiment

## Published controls playground

```sh
cargo test -p eldiron-creator --lib build_iso_published_controls_playground -- --ignored --nocapture
```

Open `generated/mz-iso2d-recipe-playground/MZRecipePlayground.eldiron`.
This additive copy retains the terrain hills, winding MZ paths and, when the
organic-stamp project exists, its paint/stamp exercises. The original projects
are not overwritten. The player starts in the new yard at `(5.5, 26.5)`.
Walk north through the doorway to enter the room, or south through the fence gap.
The roof and near walls fade using the existing iso interior/cutaway behavior.

The **Published controls playground** Tilepicker collection contains:

- **Wall segments** and **Wall corners** from the browser recipes. Select a wall,
  choose **Edit mesh**, and select the outer Group. Try **Thickness** (called
  **Thickness** on the segment's Dimensions page), or change its height. All
  placements sharing that recipe update. Segments and corners are separate sources;
  give them matching thickness/height values when you want a uniform room.
- **Roof panels**, assembled from two editable Roof slope recipe groups per module.
  Change **Pitch** on the top Group: calculated panel length and mitered ridge
  cuts keep both halves joined at the fixed **Roof span**. **Panel length**,
  **Half run** and **Ridge rise** are read-only calculated values. Four placed
  modules cover the room. Open either child group to inspect its profile,
  extrusion, rotation and calculated ridge edge.
- **Fence (Repeat)** repeats the posts and calculates **Fence length = count ×
  spacing** for two continuous rails. Try count, spacing, post height and wood
  color; posts retain their authored width. The length readout updates automatically.
  These placements have **Follow recipe shape** enabled: their collision grows
  with the recipe, while each starting anchor stays fixed.
- **Room floor**, an editable Box recipe whose thickness extends below its
  walkable Y=0 top.
- **Tree trunk** uses a recipe-authored octagon matching the trunk base. Try
  **Trunk → Trunk width/depth**: both the trunk and collision contour update,
  while foliage stays unchanged. Two trees beside the room let you walk under
  their canopies. The room corners likewise use their actual concave L shapes.

- **Doorway** adds two separately blocking jambs and an overhead lintel at the
  room entrance. Width, height, thickness and depth stay in its recipe. The
  doorway remains open beneath the lintel because only the jambs have footprints.
- **Separate posts** sit left of the yard at `(0.5, 26.5)`. Try count and spacing;
  Repeat creates both the posts and their disconnected footprints. Walk between
  them at X=1.5 or X=3.5 with the default spacing.
- **Ring walkway** sits at `(2.5, 29.5)`, with its walkable top at Y=0.25. Walk
  across it from X=0.5 to X=4.5 at Z=29.5: the player steps up onto the rim, down
  into the open center, then back up and down. Opening width/depth update both
  its visible ring and footprint hole. The recipe is top-anchored; placement
  Top height controls the surface elevation.

Use **Asset → Publish controls** to inspect or change the actual input bindings
and formulas. **Insert control reference…** picks another control by label. The
roof and fence are also available as **Gable roof** and **Fence** in the recipe
browser. All dependencies and formulas are saved in their editable MZ sources.

The compact mesh placement sidebar includes **Follow recipe shape**. It uses an
MZ **Footprint** operation referencing an editable **Contour** row in X/Z. Open a
part with **Edit mesh**, enter its Group, and select **Footprint contour (X/Z)**
to inspect/edit the shape; **Publish controls** exposes the shared bindings.
Regions can have holes and there can be multiple disconnected regions. Each
ring has 3–64 points. Use the Footprint node’s **Add hole** selectors and **Add
region** checkbox; Repeat and Assemble propagate all regions automatically. Recipes without a Footprint
retain the rectangular mesh-bounds fallback. Disabling the placement setting
freezes the current polygon for manual editing. Shared map geometry remains
manual. Source edits, rotation, dragging, undo/reopen and topology changes retain
the mesh anchor and sector identity.

This updates each part independently. It does not move neighboring placements:
extending the two fences into their gap closes it in collision as well as art.
Pitch edits keep the roof span fixed. Runtime movement and grid navigation test
polygon boundaries/interiors; canopy areas and the corner notch remain open.

The builder verifies source links, recipe edits, changed iso bakes and source/cache
roundtrips. Its native runtime test walks through the doorway and fence gap in
both directions, walks into an open room corner, beneath a canopy and between
posts, and checks the player steps up/down at the ring and its empty center. `outside.png`, `inside.png`,
`edited-controls.png` and `playing-inside.png` are review captures. The generated
project and captures stay under ignored `generated/`; the reproducible layout
is in `creator/src/procedural_material/woodland_demo/recipe_playground.rs`.

## Terrain brush playground

Build the expanded native test scene with:

```sh
cargo test -p eldiron-creator --lib build_iso_terrain_playground -- --ignored --nocapture
```

Open `generated/mz-iso2d-terrain-playground/MZTerrainPlayground.eldiron`. This
separate variant retains the room, original hill and MZ stairs, and adds four
brush-sculpted landforms: East Ridge, High Meadow, South Knoll and a rolling
shoulder. A winding, connected MZ Auto road loop crosses the hills. Its grass
and earth use continuous world coordinates and editable MZ source recipes.

The player starts at the path junction near `(12.5, 17.5)`. Follow the loop east
to the high meadow, then south and back. The southwest spur ends in an open,
flat practice clearing at `(10.5, 24.5)`: use **Terrain → Raise / Lower / Smooth**
there to bend the existing path, then Play to walk across your changes. The
native harness walks the hill loop and clearing spur in both directions, checks
feet against the rendered surface, and verifies a raised practice clearing
remains walkable without changing its MZ paint.

The project and its previews stay under ignored `generated/`; authored layout
and generation code live in `creator/src/procedural_material/woodland_demo.rs`.

### Paint on the terrain

**Rect → Paint / Compose** already supports 2D Iso terrain: picking follows
the raised surface and MZ materials, blends and Auto road masks stay in world
coordinates. Use this workflow to change the playground's paths and ground.

The **Paint** tool (**I**) now stays in 2D Iso and paints on the shared terrain.
Its outline and picking follow the visible hills. Strokes are stored in stable
world X/Z paint coordinates, so sculpting moves the surface beneath them without
sliding or rewriting the paint. Painted color is part of the ground rendering,
before characters, trees and buildings; the game uses the same layer.
Draw/erase, palette color picking from painted ground, selection clipping/fill,
undo/redo and save/reopen are supported. Organic stamps also work in 2D Iso:
choose **Grass**, **Flowers**, or **Rocks / Rubble**, then **Stamp** mode. Click
or drag to scatter detail. Density and jitter vary spacing and artwork. X/Z
anchors stay fixed while height is sampled from the current terrain, so raise,
lower and smooth edits move the stamps with the hills. Erase uses world space
and the active stamp kind, including after panning and zooming. Upright prop
surface painting remains a 3D workflow.

The native organic stamp regression writes `generated/iso-organic-stamps/OrganicTerrain.eldiron`
and editor/game previews, all ignored by git. It scatters grass, flowers and rocks
on High Meadow and starts the player nearby:

```sh
cargo test -p eldiron-creator --lib iso_terrain_stamps_native -- --ignored --nocapture
```

The native regression writes a separate painted/sculpted project and previews
under ignored `generated/iso-terrain-paint/`:

```sh
cargo test -p eldiron-creator --lib iso_terrain_paint_native -- --ignored --nocapture
```

## Native Rect placement profiling

The full-editor benchmark can paint grass, change an Auto road placement, and
place a tree. It checks that each gesture commits a real change and reports
input and subsequent redraw timings separately:

```sh
ELDIRON_BENCH_PROJECT_PATH="$PWD/generated/iso-organic-stamps/OrganicTerrain.eldiron" \
ELDIRON_BENCH_RECT_PAINT=1 ELDIRON_PROFILE_EDITOR=1 \
cargo test -p eldiron-creator --lib benchmark_u5_creator_redraw -- --ignored --nocapture
```

The native iso editor renders directly from the map; it keeps SceneManager's
map and dirty chunks current for camera switches, but builds no unused 3D chunks
while editing in 2D Iso. Finished material bakes update CPU textures immediately;
GPU atlas upload waits until a GPU-backed view needs it. Terrain redraws use
independent row bands with the same per-pixel facet/depth order.

## Woodland art experiment

The darker variant uses **Steam Lords**, the 16-color palette by Slynyrd:
https://lospec.com/palette-list/steam-lords . The original swatches are in
`woodland/steam-lords.hex`; the project retains their attribution and assigns
them to the existing ruleset slots so mesh material references remain stable.

After generating the original native project below, build the separate art pass:

```sh
cargo test -p eldiron-creator --lib build_woodland_iso_demo -- --ignored --nocapture
```

This explicit native-Metal harness writes
`generated/mz-iso2d-woodland/MZWoodland.eldiron` and scene previews. It reads the
original project without overwriting it. Its authored recipes and layout builder
are in `creator/src/procedural_material/woodland_demo.rs` and `woodland/`.
No project, texture cache, or preview image belongs in git.

The project uses shared, palette-bound MZ materials for mossy grass, worn earth,
old masonry, stone, foliage, bark and banks. Trees, shrubs, stones and grass tufts
are editable MZ mesh recipes. The hill uses shared terrain controls with a
1.4-unit summit and a gentler shoulder, sampled at four subdivisions per tile.
The winding
path is an ordinary **Auto road** composition with **Uneven edge** and **Soft
edge**, derived from its painted placements; it remains editable in the Rect
tool's **Compose** mode. Connected world pages keep its masks continuous across
isometric tile boundaries and over the slope. The harness runs the actual game
server and checks click movement along the winding route in both directions,
keyboard ascent/descent, foot heights, the existing MZ stairs, and that reopening
the project retains its baked frames and material cache keys.

Grass uses **Continuous** world X/Z coordinates with cached page coverage, on
the flat map and the smooth terrain. The quieter, darker ground and brighter oak
foliage separate vegetation from the terrain. Materials remain flat swatches in
the picker and source editor; the placed outputs are projected. Terrain height
rules are shared with the other cameras; iso rendering, editor picking and
walking use the same sampled triangles. The stairs and terrace remain rigid MZ
construction parts.

To adjust the hill, select a named hill control with the Vertex tool and open
**Edit Vertex**. **Y** sets the peak height, **Terrain control** enables the
control, and **Influence radius** sets its reach. The terrain grid and Rect
painting follow the raised surface. Its path is a normal painted MZ composition,
so editing the source or its mask updates the hill's appearance without changing
its height. Generated projects and previews stay under ignored `generated/`.

## Native Eldiron project

```sh
cargo run -p eldiron-shared --example mz_iso_project -- generated/mz-iso2d-native
cargo run -p eldiron-creator -- generated/mz-iso2d-native/MZIso2D.eldiron
```

The project opens in the **2D Iso** editor camera. The editor shows the full
rendered scene, with its isometric grid and normal editing tools. X runs up-right
and Z down-right in the editor, game, MZ bakes, and picking. Ctrl/Cmd+6 selects the
camera; Ctrl/Cmd+2 returns to top-down. Pan and zoom keep the same world focus.

Use the Rect tool to place MZ parts. The **Rotate -90 / Rotate +90** HUD buttons
or **, / .** turn the brush through its directional views in 90° steps;
the brush shows the actual sprite before placement. Vertex/line/sector dragging,
box selection, entity placement, sampling, and snapping follow the iso projection.
The grid button controls editor guides; outlines are never baked onto the art.

Selecting a mesh tile keeps the map open and shows a compact **Meshes** panel on
right. **Edit mesh** explicitly opens the full modelling workspace; **Back to map**
returns to placement. The panel has no project-wide mesh selector. Use the
picker to select another mesh, or **New mesh** to create one.

The tilepicker's **MZ Materials** tab lists every project MZ material, including
shared sources used only by meshes and the ruleset material library. Existing
projects acquire named material tiles automatically, so the test project's
**Walls** source is visible without regenerating the project. These source tiles
are flat runtime caches at the material's own ID; projected outputs keep their
own IDs and source links. Ordinary ruleset item sprites stay outside map-tile tabs.

In a mesh recipe select its **Texture** operation, then **Edit material**. The
material opens in the right sidebar; **Back to mesh** restores the same recipe
and operation. Embedded portable materials are published as named project assets
and relinked, with publication and relinking covered by one undo operation.
Material choices use asset names. Referenced and ruleset materials cannot be
removed through the picker while they remain in use.

Ground output tiles still open the material recipe directly. Preset wall
orientations share one editable model source; explicit mesh editing keeps the
selected output orientation. Pixel editing routes source-linked tiles to MZ.
Recipe and linked-material edits automatically refresh bound outputs, the scene,
picker thumbnails and minimap. Undo/redo follows the same path, and late flat
preview results cannot replace projected outputs.

MZ material output automatically caches an isometric ground tile while preserving
its flat texture for material authoring, ordinary 2D maps, and live 3D surfaces.
Custom MZ model recipes automatically get source-linked tile outputs with four
views and a shared foot anchor. Models with a walk clip get eight directional
animated views. Editing a recipe, palette, dependency, or material refreshes the
output; undo, saving and reopening retain source bindings. The MZ model preview
has an **Isometric tile** view, selected by default in iso mode. Orbit/front/side
views remain available for constructing models.

The game camera uses `[camera] type = "2d_iso"`; an isometric region also selects
it for normal 2D game widgets/players. Live MZ characters use the existing outfit,
material, rig, and animation pipeline with the same isometric projection. All
props and characters share depth sorting. Source recipes live in
`procedural_materials` / `procedural_appearances`; `isometric_bakes` binds model
sources to stable tile IDs. Flat materials and projected caches remain distinct.
Scepter accepts `editor.focus` and `region.render_preview` with `view = "2d_iso"`.

Press Play to walk using clicks or WASD/arrows, including diagonal combinations.
The **Player Camera** node offers **2D Isometric** (`2d_iso`, smooth movement)
and **2D Isometric (Grid)** (`2d_iso_grid`, interpolated tile steps). W/Up follows
X up-right, D/Right follows Z down-right; combinations cover all eight directions.
Grid click paths also use eight directions and reject blocked diagonal corners.
Generate the grid variant with:

```sh
cargo run -p eldiron-shared --example mz_iso_project -- generated/mz-iso2d-grid --grid
```

Existing projects reuse compatible bundled material bakes on load. Prepared material
maps are reused across Play/setup passes; iso bake keys track referenced materials
and palette colors, so adding unrelated library entries or selecting a palette slot
does not rebake the scene. Ground rendering caches a padded view and scrolls it as
the camera moves; editing, zoom, material changes and animation invalidate it.
A reproducible CPU profile (including camera scrolling) is available with:

```sh
ELDIRON_PROFILE_STARTUP=1 cargo run -p eldiron-shared --example mz_iso_profile -- generated/mz-iso2d-native/MZIso2D.eldiron
```

Local debug measurements for the original smaller clearing: project decode/cache validation about
2.0 s, first avatar/material setup 2.1 s (previously 22–23 s), repeated setup
0.6 s (previously 8.4–8.6 s), and the scrolling CPU scene draw 5.6 ms/frame
(previously 27 ms/frame). These measure the named CPU stages; window presentation
and the complete editor startup are separate.
The expanded 22×18 height scene measured about 12.5 ms per scrolling CPU draw in
the same debug profile.

Walls are 2.4 units tall with a 2-unit-high entrance. Near walls fade inside the
room. The expanded clearing includes a two-tile-wide stone staircase to a
2-unit-high terrace and a three-tier grass hill. Both use reusable quarter-height
MZ blocks: `stone-step.mzgeometry.json` and `grass-step.mzgeometry.json`. Their
local origin is the walkable top; the geometry extends down by 0.25 units.
Placements supply the elevation, so one recipe serves every step and tier.

The compact mesh panel provides **Top height**, **Block height** (vertical stack
spacing), **Walkable top**, and **Fill below**. Select the MZ stone-step tile,
set its top height to 0.25, enable walkability and fill, and paint. Raise the
height for the next stair and paint the adjacent cell. At height 2, fill produces
eight placements of the same quarter-height recipe. `[` / `]` lower/raise the
editing plane by the block height while using the Rect tool.

**Pick surface height** arms the next map click to select a visible tile top.
The grid, brush, hover picking and box selection follow the active height.
Painting and erasing affect that level; repeated paint does not duplicate an
existing placement. Fill and the complete paint stroke use the normal undo/redo
path. The Sector tool's **Edit Sector** action exposes height and walkability for
existing placements. Step recipes use their local Y=0 as their walkable top;
match stack spacing to the recipe's thickness.

On an isometric map, a sector with `iso_walkable = true` supplies a solid walkable
top at `floor_height`. Lower fill blocks are visual placements only. Movement
updates character Y, rejects excessive rises/drops, and click paths use the
stairs. Defaults allow 0.35 units up and 0.5 down; keyboard movement can override
these with character attributes `max_step_height` / `max_drop_height`. Click
navigation currently uses the defaults. Picking and camera following account for
the raised top, and elevated sprites share depth sorting with characters.

From the starting room, the stone stairs are to the upper right (+X), beginning
at X=9, Z=3–5. W climbs them; S descends. The grass hill is to the lower right
(+Z), centered at X=6.5, Z=13.5. Click the terrace or hill top to test routing.
The terrace has solid sides: this tests height changes, not a floor that can also
be walked underneath. Overlapping storeys, bridges, and roof cutaways remain a
separate extension.

The generator checks native collision/pathfinding, save/reopen, actual server
controls through the entrance, ascent/descent on stairs and hill, click routes,
blocked ledges, height-aware picking, the editor draw path, and live MZ characters.
It writes `stairs.png`, `upper-terrace.png`, `hill.png`, and an editor overview.
Rendering requires GPU initialization; set
`ELDIRON_ISO_NO_RENDER=1` for generation/collision checks without a GPU.

All generated projects, sprites, screenshots and test binaries remain under the
ignored `/generated/` folder. Only authored sources belong in git.

## Original offline harness

An isolated sprite experiment: procedural MZ grass and paving, a closed 4×4 room
assembled from twelve straight wall tiles and four corners, one tree, and the
official MZ humanoid walking in eight directions. The original offline harness remains available for comparison.

From the repository root:

```sh
cargo run -p eldiron-shared --example mz_iso_2d -- generated/mz-iso2d
node --test crates/shared/examples/iso_2d/world.test.mjs
node crates/shared/examples/iso_2d/verify.mjs generated/mz-iso2d
# Optional sprite previews and alpha checks (requires Pillow):
python3 crates/shared/examples/iso_2d/preview.py generated/mz-iso2d
```

Open `generated/mz-iso2d/index.html` directly in a browser. It embeds all its assets
and runs offline. Auto walk patrols outside the closed room and around the tree.
Click to walk or use WASD/arrows; Space pauses. The overlap buttons place the
character behind/in front of obstacles or inside the room. Enable Footprints to
inspect anchors, routes, and collisions. There is deliberately no doorway.

## Reusable parts

`wall.mzgeometry.json` and `corner.mzgeometry.json` are editable MZ recipes, not a
whole-house generator. One straight recipe is baked in two orientations; one L
corner recipe is baked in four. Shared dimensions, a repeating masonry material,
consistent UV scale, and slightly overlapping coping avoid transparent cracks.
No per-tile outlines are added to these connecting sprites.

The Rust example evaluates the MZ recipes and official walk rig, then CPU-bakes
PNG sprites at one common 2:1 camera projection and light direction. Runtime
rendering uses Canvas 2D with ground anchors, painter sorting, footprint collision,
and A* movement. There are no runtime meshes or WebGL.

`world.test.mjs` covers projection, sorting, pathfinding, and the enclosed room.
`verify.mjs` executes the generated page scripts with a recording Canvas stub,
checking controls and 60 seconds of walking. `preview.py` composites those sprite
calls and samples all sixteen joins for opaque coverage. These are sprite-render
previews; DOM layout, Canvas shadows, and real browser input still need a manual
look. A passing coverage check does not guarantee imperceptible texture seams.

## Scope and limits

This offline harness tests whether reusable MZ parts can produce connected isometric
2D art. The native project above uses the new runtime path; existing projects keep
their saved camera mode.
The offline harness uses a single-level ground-anchor sort. The native project
adds raised solid terrain and an authored near-wall cutaway; larger props,
overhangs, and overlapping floors need more work. The MZ
character is a first bake of the existing 3D rig, not finished pixel art.

All generated PNGs, the atlas, manifest, playable HTML, draw-call recordings, and
preview GIFs belong under the already ignored `/generated/` directory. Only these
recipes, source files, and checks are intended for git. No generated files are
written to `test_projects`.

## Validation notes

The native generator verifies save/reopen, collision, pathfinding, game-widget
startup, and server-controlled entrance/exit. The targeted Rust tests cover
projection/picking, sprite order, directional frames, combined keys, and MZ cache
invalidation with last-good-bake retention. The workspace check passes with the
normal feature set. A separate `--no-default-features` check exposes existing
Rusterix feature-gating errors in batch exports and procedural avatar preparation;
this experiment does not repair that unrelated headless build configuration.

Raised terrain uses a contact compositor: a supporting top or its fill blocks
cannot paint over an actor standing on that column. Foreground props and higher
terrain retain their ordinary occlusion. This applies to cached MZ character tiles
and live avatar sprites without changing their foot anchors or movement heights.
The renderer tests cover tile interiors, shared edges, stacked fill, foreground
occlusion, and reuse of the contact buffer between frames. To inspect the native
MZ walk frames on the stairs and hill and time the scrolling contact renderer:

```sh
cargo run -p eldiron-shared --example mz_iso_contacts -- \
  generated/mz-iso2d-native/MZIso2D.eldiron generated/ui-review/iso-feet-contacts.png
```

This reads the saved project without modifying it; the resulting image stays ignored.


## Recipe connectors and illustrated tutorials

The published-controls playground now adds a second room east of the original
room (approximately X=12.5–18.5, Z=21.5–25.5). It is built through the same
`iso_editor::paint_cell_with_tiles` helper as the Rect tool: approximate grid
clicks snap 15 independent wall/corner/doorway parts into exact joins. The
builder checks the final closing seam. Its roofless layout is useful for
inspecting the doorway and walking around the walls.

Enable **Follow recipe shape** and **Snap connectors** for new placements.
Recipe edits move connector positions, connected child placements and then
collision footprints. Use **Detach connection** to freeze a placement's incoming
connection. Links form a parent forest; changing a closed room's dimensions can
open its unlinked closing seam. Roof, floor and fence recipes also expose
connectors. No asset-specific actions are introduced.

The extensive user guide is in `docs/docs/building_maps/materialz/`, with separate
material, mesh, controls, footprint and connector tutorials. Native screenshots
without the AI header are curated documentation assets under
`docs/static/img/materialz`. Generated projects, temporary captures, build logs
and rendered caches remain under the ignored `generated/` directory.
