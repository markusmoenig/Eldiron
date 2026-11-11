# Unending Night: a native MZ art-quality study

This is an authored visual benchmark for the original Ultima Online atmosphere:
quiet earthy ground, readable silhouettes, textured masonry and wood, mature
vegetation, everyday props and a furnished building. It tests actual Eldiron
rendering and movement. Every surface and mesh is an editable MZ source.

The playable project is **`test_projects/UnendingNight.eldiron`**. Its inn region
is still named Wayfarer's Rest. The project includes the former Cellar example's
responsive modular bottom action bar: Preferences, Equipment, Inventory and
Spellbook on the left, ten assignable action slots in the centre, and Look/Use
on the right. Edit the **Toolbar** sector on the **Game** screen to change its
layout and panel styling. Its mode icons are flat, palette-bound MZ recipes in
the **Unending Night / UI** Tile Picker collection; Actions artwork is included
for adding a generic command catalogue. The obsolete Cellar world is not included.
Only the meaningful **Meadow** and **Inn** areas are named for navigation. Wall,
roof, prop and planting placements remain unnamed sectors, so they do not fill
the region tree with gameplay areas. Their art-study labels live in placement
metadata, and each source part remains identifiable in the Tile Picker.

The player uses the normal **Characters → Player** template and a placed region
instance. Its **Human** race and **Warrior** class inherit the official ruleset's
avatar, animation, equipment and character defaults, just like the starters.
Change these through the character's setup nodes. There is no custom character
tile or Set Tile override in this project; use the Characters tree to place
characters, rather than painting character artwork with Rect.

The **Inn Door** is also an ordinary Item. Its single MZ recipe contains a
planked Closed leaf, a hinged Open row and a State Switch. The bottom toolbar
provides **Open/Close, Use, Look** through official intents. The door starts
unlocked; its Item State node can enable a lock using the supplied **Inn Key**.
See the [door/state tutorial](../../../../../docs/docs/characters_items/doors.md)
for keys, behavior nodes and the shared iso/3D collision workflow. The focused
installer/review helper is `unending_night_door`; its outputs belong in
`generated/mz-item-states/`.

## Palette and references

**Guildhall 256**, created by **Eric Johnson**:
<https://lospec.com/palette-list/guildhall-256>

`guildhall-256.hex` is the unmodified Lospec palette download. The project assigns
all 256 distinct swatches, while retaining approximate color roles in the original
64 material slots so the existing player materials stay coherent. The materials
emphasize olive/forest greens, warm greys, oak browns and parchment; blue cloth and
lanterns are accents.

The dated original-release reference is Anders Hedström's October 1997 gallery:
<https://www.alhem.net/uo/> and
<https://www.alhem.net/uo/uo0001.html> (10 October 1997).
The user's supplied house/encampment screenshots informed the small-scale detail.
Reference images are stored only in the ignored local review directory.

## Build and open

From the repository root on a native development machine:

```sh
cargo run -p eldiron-shared --example mz_iso_project -- generated/mz-iso2d-native
cargo test -p eldiron-creator --release --lib build_uo_art_study -- --ignored --nocapture
cargo build --release -p eldiron-creator
./target/release/eldiron-creator test_projects/UnendingNight.eldiron
```

The first command creates the seed when it is not already available.
The builder uses `generated/mz-iso2d-native/MZIso2D.eldiron` as the playable
character/screen seed. It creates a separate region and asset catalog; the seed
project is left intact. Its layout is in
`creator/src/procedural_material/woodland_demo/uo_art.rs`.

The output directory contains the project, courtyard/interior/wide renders,
and a live game render. All generated projects, baked frames and review captures
remain under ignored `generated/uo-art-study/`.

The builder above creates an isolated art-study scratch project. The maintained
Unending Night project in `test_projects/` is the explicit exception kept for
version control. Capture/update helpers prefer that maintained project. HUD and
panel review images stay under ignored `generated/unending-night/`.

## What to try in Eldiron

The **Wayfarer's Rest / MZ parts** collection contains the source assets. Materials
and meshes can be edited from the Tile Picker; select a mesh and use **Edit mesh**
to open construction. Stone bays, window bays, doorway, floorboards and roof
modules are separate reusable parts. Props include a well, fence, barrels, crates,
benches, table, hearth, lantern, grass tuft and mature oak.

The roof modules include continuous timber eave boards. **Inn / Gable end**
finishes each exposed end of the roof: an extruded plaster panel, tie beam,
upright timber framing and sloping verge boards covering the shingle edges.
Place it at a roof end at the same base height as the roof modules; rotate the
far end by 180 degrees. Interior roof modules stay open so the attic has no
repeated dividing panels. The demo's two gables follow the roof's indoor cutaway.
All of these details remain ordinary editable MZ rows and use the existing
plaster and oak materials from the Tile Picker.

Walk north through the inn doorway. The floor supports the character at Y=0.09;
the roof and near walls disappear while indoors. The courtyard and path remain
walkable around blocking props. Outside again, the roof returns.

Materials use Color Ramp, scoped Cell Data, Grid, White/Perlin Noise, Circle,
Distance Profile and Scatter. Leaf-covered loft lobes supply canopy volume;
alpha-cut leaf stamps break up their surface. Canopies use one leaf material with
natural colour variation; there are no directionally darkened leaf materials or
painted contact-shadow parts. The ground uses continuous world-coordinate pages
and MZ Auto road compositions.

## Dynamic lighting

MZ caches store unlit colour, surface normals, local height, roughness, metallic
and emission. The native iso renderer supplies those surfaces to a deferred **SceneVM GPU**
pass, which shares the existing device, queue and scene lights. It shades each
frame from
Eldiron's existing Render Settings, sun simulation and SceneHandler lights. The
editor uses its preview time; play uses server time. Moving a light or changing
time never recompiles a material or regenerates a sprite.

The door and table lanterns have ordinary map Point lights, editable with the
Light tool. The hearth uses a live MZ Particle Effect, including its light,
colour ramp and flame parameters; edit those in its mesh recipe. Flames are
instanced GPU billboards driven by the existing particle simulator. Lantern glass has an emissive MZ material. Its emission keeps
the glass visible; the map light illuminates nearby surfaces. The same map lights
are available to the other cameras. The lantern has an open lower frame so its
point source can illuminate the ground. Its four rails and nearby walls retain
real triangle shadows; a solid tray immediately below the emitter would project
a much larger rectangular umbra onto the floor.

GPU shadows rasterize the actual MZ triangles with their repeating UVs and
alpha masks, together with the shared terrain triangles. Hills cast shadows
and sculpting updates them. The sun uses an orthographic depth map; Point
lights use six depth views. Both use SceneVM's existing device and queue.
Geometry uploads reuse buffers, and unchanged terrain triangles stay cached.
Hidden cutaway roofs continue blocking outdoor light. Window bays have real
openings between their frame members, with no opaque black insert.

The CPU reference and older unsynchronized MZ caches retain column shadows.
Ordinary PNG sprites remain sprites: prelit artwork keeps its original pixels;
optional world tint can receive the new shadows without inventing a 3D caster.
Bitmap-only stamps have no mesh volume for casting shadows. Up to 32 Point
lights cast shadows; additional lights still illuminate the scene.

The demo sets `[simulation] night_ambient_strength = 0.12`, fading the ambient
floor down at dusk while preserving its daytime setting and local-light strength.
Fine grass blades use continuous world-cell identities, and the oak palettes
retain their original greens.

The builder also captures morning, midday, evening and night without rebaking
between times.

## Verification and current art limits

The builder validates recipes, source links and all 256 palette entries, checks
save/reopen, and walks the live player through the doorway in both directions across the courtyard and up/down the winding hill trail. Review the actual PNGs after changing recipes.

This study deliberately uses the current general MZ operations. It exposes the
remaining art work clearly: richer character animation/detail and a broader
range of plants. The current pass adds three oak silhouettes, bluebells, ferns,
timber-and-plaster bays and a softly edged earth path over a wooded hill.
The art target is the atmosphere and readability of the original release.

The exercise also found a core baking gap: isometric meshes previously ignored
Material Texture resolution and baked at 64 pixels. Mesh bakes now honor the
recipe resolution (16–512), distinguish source/resolution/snapshot cache entries,
and reuse the shared palette-aware material cache. Static mesh cache version 14
invalidates earlier derived frames. A regression test covers resolution and
embedded source changes. Interior cutaways now remove opted-in sprites entirely
so translucent roof patterns cannot obscure furnishings or actors.

## GPU profiling

```sh
cargo run --release -p eldiron-shared --features graphics --example mz_iso_profile -- test_projects/UnendingNight.eldiron --gpu --size 1280x900 --walking
```

The timed frames include tile assembly, geometry and light upload, triangle shadow maps,
live MZ particle simulation and billboard drawing, and readback into the UI
buffer. Five warm-up frames are excluded. `--lit` runs the CPU reference lighting
pass; omitting both flags measures the unlit canvas. Generated output stays in
`generated/`. Use the Rect HUD's **Rotate -90 / Rotate +90** controls or **, / .**
to orient a part before placement.

Use `--hour 18.5` for dusk, `--hour 23` for night, and `--no-shadows` for a comparison.
The benchmark reports mean, median and p95 after warmup; it measures the scene
renderer, including CPU assembly and GPU readback, rather than total editor FPS.

## Soft lantern shadows

The renderer defaults to `[render] point_shadow_softness = 0.075`, a world-unit source
radius for GPU point-light shadows. Blocker-distance filtering softens the
lantern frame's projected edges without blurring the scene or material pixels.
Door/table lantern intensities are 1.65/1.35, with ranges 4.5/3.6 and 0.08 flicker.
The editable MZ fixture and actual mesh/terrain occlusion remain in use.

The filter follows the three-stage PCSS approach (blocker search, penumbra
estimate, shadow filtering), adapted to the point-light depth views and receiving
surface planes: <https://developer.nvidia.com/gpugems/gpugems3/part-ii-light-and-shadows/chapter-8-summed-area-variance-shadow-maps>.

## Live terrain shape examples

The maintained UnendingNight project includes **Terrain Plateau**, **Terrain Hill**
and five graded linedefs under a continuous **MZ Composite** path east of
the inn. Edit their Shape Terrain inspector controls in 2D/iso; the generated
height field is shared with 3D, walking, picking and shadows. Floor/tile height
remains independent. The camera starts wider in daylight, and its 3D anchor faces
the examples.

```sh
cargo run -p eldiron-shared --release --example shape_terrain -- \
  test_projects/UnendingNight.eldiron generated/unending-night/shape-terrain --review
```

This helper adds the editable examples only once, verifies both directions of
the graded route, compares real 3D chunk vertices with the shared surface, and
profiles chunk generation. `--review` captures native iso and 3D views and times
3D VM rendering/readback. Captures, backup and logs remain in ignored generated/.
The intentionally maintained `.eldiron` project is the only generated example
artifact intended for version control.


### Inspect the hill and path materials

The existing terrain composition uses **Inn / grass** as the base, weathered
rock on Terrain Plateau and Terrain Hill, a wide grassy verge, and a narrower
earth trail. Moving its guide geometry updates the coverage, while sculpting
keeps material coordinates attached to the terrain.

Rect **Compose** offers tile shapes, soft/noisy edges and material layers.
There are no map-area/path modes or geometry-binding buttons in Compose.
Edit terrain heights through the linedef/sector **Shape Terrain** settings.
The hill sectors and five approach linedefs own their `[terrain.material]`
settings. Select an example with Sector/Linedef, open Edit Sector/Edit Linedef,
and change `source`, `width`/`inset`, `soft_edge`, or `noisy_edge` in TOML.
Path settings also include `verge_source` (grass) and `verge_width`; clear the
source strings to remove material coverage while keeping terrain heights.
The terrain result blends over existing paint in 2D, iso and 3D. Derived MZ
outputs are hidden from the picker, regenerated from geometry settings, and
saved as caches with the project. Edit the original MZ sources in the picker.

Under **Advanced mixing**, the rock material is separately editable using
**Edit material recipe**;
**Back to composite** returns to the selected layer. Its Roughness row maps
facets to **0.34–0.54**, with zero metallic. Iso terrain passes those MZ material
channels to live GPU lighting, giving subtle sun/local-light highlights rather
than painted reflections. Guildhall palette bindings remain in place.

To refresh the maintained example with the normal compositor and native MZ baker:

```sh
cargo test -p eldiron-creator --lib update_unending_night_terrain_art -- --ignored --nocapture
```

The helper verifies persisted trail continuity and live-light material data
after reopening, and captures day/evening GPU views under ignored
`generated/unending-night/terrain-art/`. Only the maintained project is saved
after successful verification; captures, backups and standalone caches stay ignored.
