# Eldiron character generator

Experimental standalone Rust library and CLI for generating humanoids and equipment
from plain-text appearance definitions. No dependency on Creator, Rusterix,
SceneVM, or gameplay rules in its default build. The optional `eldiron` feature
provides explicit ruleset, Avatar and SceneVM adapters. The catalog is a proposed
appearance format, not automatic runtime dispatch from ruleset declarations.

## Run

From the workspace root:

```sh
cargo run -p eldiron-character-generator -- \
  crates/character_generator/examples/villager.toml \
  villager target/character-generator/villager
```

Open `target/character-generator/villager/preview.html` in a browser. It works
offline without a server and offers animation/direction controls and hand-anchor
overlays. The preview shows baked sprites. Use a glTF viewer to inspect the 3D
export independently. Optional CLI flags: `--size 96 --frames 8`.

## Creation style

Style belongs to the generator/app settings, separately from character definitions
and gameplay rulesets. The same catalog can produce any of these presets:

| Preset | Geometry | Default shading |
| --- | --- | --- |
| `blocky` | Box forms and four-sided limbs | Flat |
| `stylized` | Shaped torso/head and eight-sided rounded limbs | Flat |
| `natural` | Narrower head, shaped body and twelve-sided limbs | Smooth |

Pass `--style natural` to the CLI. Optional overrides are `--segments 16`
(4–24; blocky requires 4), `--shading flat` or `smooth`, and `--head-scale 1.1`
(0.75–1.35). Defaults preserve the original blocky output. These options apply
to both 3D meshes and baked 2D frames, using the same skeleton and equipment
attachments. Natural is an early low-poly form; realistic anatomy and detailed
faces are still future work.

Library callers use `catalog.generate_with_options(id, &GenerationOptions {
style: StylePreset::Natural, ..Default::default() })`. `GenerationOptions` is
serializable for a separate app/project art profile. Character and ruleset
appearance tables do not accept style settings. Output metadata records the
resolved style for inspection.

For a synchronized comparison, generate the same character into
`<output>/styles/blocky`, `stylized`, and `natural` with the corresponding
`--style` values, then run:

```sh
python3 crates/character_generator/examples/compare_styles.py <output>
```

Open `<output>/style-comparison.html` to review all three together.

Generated outputs:

- `character.glb`: glTF 2.0 binary mesh, materials, 15-joint skeleton, inverse-bind
  matrices, weighted vertices, named hand sockets, and idle, walk, cast, death, sit, use, attack and parry clips.
- `atlas.png`: transparent orthographic color atlas, frames across columns;
  eight direction rows per motion, in `CharacterAsset.motions` order.
- `channels.png`: unshaded grayscale semantic IDs matching visible atlas pixels.
- `atlas.json`: frame rectangles, times, clip durations, pixel scale, ground
  points, loop flags, and projected hand sockets.
- `preview.html`: offline 2D animation review tool.
- `viewer3d.html`: offline WebGL mesh and animation viewer with orbit, pan, zoom,
  head close-up, light direction, ground grid, repeat and time scrubbing.

All frames share a fixed camera elevation of 10 degrees, framing and pixel scale.
Directions are front, front_right, right, back_right, back, back_left, left,
front_left. Character forward is +Z, up is +Y, and units are meters.
Positive X is the character's left. `main_hand` is the right hand; `off_hand`
is the left. Atlas socket and ground coordinates are frame-local pixels from
the top-left and can be fractional. The ground point stays fixed while the
root joint bobs. Looping clips exclude the duplicate loop endpoint in the atlas;
actions include their exact final pose.
GLB clips include both endpoints and sample poses at 32 intervals per cycle.

## Text catalog v1

See `examples/villager.toml` for a complete catalog. `version = 1` is required.

- `characters.<id>` requires `body = "humanoid"`, `height` (1–2.5 meters),
  `build` (0–1), and `skin` / `hair` colors (`#RRGGBB`).
- Optional `outfit` references shirt/trousers/boots item IDs. Optional
  `equipment` references sword item IDs. Both default to empty lists.
- `items.<id>` requires `generator` and `color`. Supported generators are
  `shirt`, `trousers`, `boots`, `helmet`, `sword`, `shield`, and `spike`.
- Hand-held swords, shields and spikes go in `equipment` and require `attachment = "main_hand"` or `"off_hand"`. For swords, optional
  `blade_length` is 0.1–1.5 meters (default 0.65), and `blade_width` is
  0.01–0.2 meters (default 0.045).

Helmets go in `outfit`, fit the generated head (including head-scale changes),
and replace its hair cap. Shields are beveled oval plates with a boss and grip.
Spikes are pointed hand-held weapons. Optional `size = [x, y, z]` dimensions
(0.01–1.5 meters) default to `[0.42, 0.55, 0.055]` for shields and
`[0.06, 0.06, 0.65]` for spikes; spike length runs along +Z. Each hand holds one
item. See `examples/guard.toml` for sword/shield and spike/shield characters.
Held items remain rigidly skinned to their hand; helmets follow the head.

Unknown fields, unsupported generators, unresolved IDs, duplicate clothing or
hand slots, and invalid dimensions/colors produce errors. No randomness or
external assets are involved, so identical definitions produce identical output.
Colors are authored in sRGB, converted to linear for GLB, and converted back to
sRGB after simple diffuse/ambient lighting in the CPU baker.

## Head appearance

Optional `[characters.<id>.head]` values describe physical traits; style remains
in `GenerationOptions`. Defaults are 1.0 for all proportions and `#222631` for
`eye_color`:

```toml
[characters.villager.head]
jaw = 0.8
cheekbones = 0.85
nose_length = 1.3
nose_width = 0.8
eye_spacing = 0.9
ear_size = 1.0
eye_color = "#384966"
```

Jaw accepts 0.7–1.3; cheekbones 0.8–1.2; nose length/width and ear size 0.6–1.4;
eye spacing 0.75–1.25. All must be finite. The jaw and cheek controls deform the
head surface, while eyes/brows/mouth and the scalp cap fit the result. Eye whites,
coloured irises, pupils and ears remain static features. `examples/heads.toml`
contains round, angular and broad variants. Head traits preserve the rig and
apply to both 2D and 3D outputs.

## 3D inspection

Open `viewer3d.html` directly offline or through a local server. It embeds the
mesh, texture bytes and 33 sampled poses per clip; it needs WebGL, but no external
scripts, asset downloads or network connection. Drag to orbit, Shift-drag to pan,
and scroll to zoom. Front/Side and Head close-up/Upper body/Full body reset the camera.
Upper body is useful for checking shoulder shape while scrubbing cast and attack.
The Time slider pauses and scrubs; Repeat actions is on by default.
Rendering switches between Smooth diffuse lighting, Dithered four-level material
ramps with a fixed Bayer pattern, and Grainy ramps with deterministic noise in
bind-position coordinates. Grain follows the animated surface; it has no
frame-random seed. Pixel size sets the live canvas resolution from 1× to 6× CSS
pixels with nearest scaling and no multisample antialiasing. Dithered/3× is the
default. Material detail is enabled by default and adds cool-shadow/warm-highlight
material ramps, sparse cloth weave, swept hair streaks, and leather creases.
Patterns use bind positions so they follow the rig without UV seams. Where
WebGL derivatives are supported, fine stripes fade as they become subpixel to
reduce aliasing. Toggle Material detail to compare against the plain materials.
These effects belong to the viewer shader; GLB materials and baked atlases retain
their existing textures. Isometric uses an orthographic camera at a 45-degree yaw and 35.26-degree
elevation; Front/Side return to perspective. These are live shader experiments,
not a path tracer or baked sprite playback. Fine grain may alias during motion.

The viewer uses GPU skinning with the same vertices, weights and bind positions
as GLB export. It interpolates sampled world poses, so very small differences
from the analytical pose evaluator are possible between samples. Rendering uses
simple diffuse lighting and approximate sRGB conversion, without shadows or full
metallic/roughness shading. Use GLB in a full renderer for material parity. The
viewer is generated from the asset directly, not a separate glTF importer.
`export_viewer(&asset, path)` is also available to library callers.

## Library boundary

```rust,no_run
use character_generator::{Catalog, BakeOptions, export_glb, bake_atlas};
# fn main() -> character_generator::Result<()> {
let source = std::fs::read_to_string("catalog.toml")?;
let catalog = Catalog::parse(&source)?;
let asset = catalog.generate("villager")?;
std::fs::create_dir_all("generated")?;
export_glb(&asset, "generated/character.glb")?;
bake_atlas(&asset, BakeOptions::default(), "generated")?;
# Ok(()) }
```

`CharacterAsset` exposes primitives, materials, joints, motions and sockets.
`local_pose`, `world_pose`, `bind_pose`, and `posed_vertex` allow a consumer to
evaluate animation without a file round trip. Parent joints precede children.
The generated rest skeleton has translation-only joints; the skin evaluator
and GLB inverse binds currently rely on this invariant. Public asset structs
are an experimental inspection/adapter API, not a validated arbitrary-mesh input.

The ruleset adapter resolves appearance declarations into this catalog. The
generator does not depend on inventory or combat logic. Optional SceneVM and
Avatar adapters translate the generated data to actual engine types. Runtime
inventory changes still need an explicit consumer to regenerate appearance.

## Current scope and limitations

This first milestone verifies generation, rigging, export and baking. Blocky uses
box geometry and tapered rectangular limbs. Stylized and natural use shaped,
rounded ring profiles with continuous side UVs. Chest, arms, hips, legs, neck and
head form a connected closed surface across material boundaries. Shoulder ports
join the arm rings; the pelvis splits along a shared crotch seam into both thighs.
There are no internal caps at these joins. Shared skin weights keep the surface
closed during movement, and smooth shading crosses garment material boundaries.
The neck keeps a rounded cross-section through the collar and jaw; jaw-width
traits shape the jaw above the throat attachment. Chest and pelvis profiles have
distinct anterior and posterior depths, with a shallow rear midline instead of
a pointed pelvis ridge, and the head sits forward of the spine.
Hip connections match actual polar angles about each thigh axis instead of
assuming equal vertex spacing. A smaller nose is sculpted
into the head surface rather than added as a separate primitive.
Clothing substitutes covered body surfaces and adds basic boot shafts; it is not
a separate removable cloth simulation mesh. Hands, feet, ears and equipment are
still separate parts; the complete character is not one anatomical surface.
Eyes, eyebrows and mouth are static marks fitted to the actual front head surface;
helmet brims clear the eyebrows. The neutral mouth is a small tapered lip mark;
the sprite baker gives it a minimum 1.5-pixel projected thickness and a matching
depth bias to retain visibility at low resolutions. Hair follows a varying
irregular hairline around the forehead, temples and nape, with asymmetrical
clump volume and adjustable color, and is
hidden under helmets. The sword blade is rectangular.

These are procedural demonstration clips, with planted-foot leg IK for attack/parry/cast/use, but without terrain-aware locomotion, blending,
prop interaction or ragdoll physics. There are no weapon-specific combat variants, skirts, capes,
armor or facial animation yet. Blocky UVs are assigned per quad; its limb ring
segments repeat the pattern. Rounded profiles still have seams and separate caps.
Semantic masks have coarse
garment/body IDs. The optional Avatar export uses final colored frames, not
recolorable marker templates.
The CPU baker uses a depth buffer and simple lighting, without anti-aliasing,
shadows or PBR parity with SceneVM. Equipping changes appearance at generation
time; runtime swapping and caching belong to a later adapter.

Next: improve anatomy, garment seams and movement; verify visual
rendering through SceneVM and the Creator Avatar importer before adding runtime
selection, equipment-change regeneration and caching. The adapters are available
for direct use; Creator and game clients do not automatically select them yet.

## Motion playback

Idle (2 seconds) and walk (1 second) loop. Cast (1.4 seconds) raises both hands
and returns to rest; use (1 second) reaches/works with the main hand and returns
to rest. Sit (0.9 seconds) transitions into a held seated pose, with feet at the
standing ground height. Death (1.2 seconds) falls backward and holds its final
pose. No seat, spell effects, interactive target or dropped equipment is generated.
Equipment continues to follow its attachment joint.

`local_pose` wraps loop times and clamps action times to their start/end poses.
`Motion::looping()` exposes this distinction. `atlas.json` records `durations`
and `looping` per motion; glTF animation `extras.loop` carries the same flag.
Consumers must implement looping/holding because glTF has no standard loop flag.
Preview dropdowns include every generated motion; Restart replays an action.
Repeat actions is enabled by default for continuous review; turn it off to
inspect single playback and final-pose holds.

Attack (1 second) winds up, strikes downward with the main hand and recovers.
Parry (0.85 seconds) raises the weapon and off-hand shield, recoils, and returns
to rest. Both play once. They are visual clips; hit detection, damage and timed
block windows belong to the consumer. These actions include hip lowering, weight
shifts, torso twist and knee flexion. A two-bone leg solve keeps the support feet level while the hips move, using a
forward knee pole. Attack lifts and advances the right foot about 12% of body
height, plants it during the strike, and lifts it back during recovery. The left
foot stays planted; other actions keep both feet planted. The solve targets flat
rest ground, not scene terrain; there are no obstacle contacts yet. The current strike is generic, with no
weapon-specific thrust or combo variants. The preview Frame slider pauses and
scrubs every clip for inspection.

The Avatar adapter exports all eight named clips. Eldiron's current Avatar schema
has no loop/hold field, so its runtime consumer must honor the accompanying atlas
metadata to clamp death/sit instead of cycling them. The standalone previews and
SceneVM pose evaluator already honor the generator's playback behavior.

## Verification

```sh
cargo test -p eldiron-character-generator
```

Tests cover invalid definitions, bind-pose reconstruction, weighted vertices,
weapon/hand motion, looping clips, GLB container/accessor consistency,
inverse-bind matrices, all atlas views, deterministic output, mask/alpha
alignment, frame clipping and allocation limits. Style checks cover rig/material
compatibility, detail/shading/head-scale overrides and invalid options. Generated files belong in
`target/`, not in source control.

## Material recipes

An item can name a material alias with `material = "linen"`. Define it in
`[materials.linen]` with an inline `source` string containing exactly one
existing `Material` recipe, optional `size` (4–256, default 64), `seed` (default
0), and `tiling` (positive values up to 64, default `[1, 1]`).
`examples/villager.toml` demonstrates woven cloth. The material multiplies the
item's authored color in linear space, so neutral cloth can be reused with
multiple tints. The sword blade/guard uses its material; the grip retains its
separate leather material.

`palette = ["#909090", "#ffffff"]` can supply recipe palette colors. Without
an explicit palette, a standalone material uses its authored nearest/exact
color declarations. The resolved-ruleset adapter instead supplies the ruleset's
palette when none is explicitly given. Palette-dependent recipes retain their
usual `nearest` and `Strict` semantics; use `BaseOnly` for continuous shading.

The GLB embeds albedo and glTF metallic/roughness PNGs. The CPU baker samples
the same albedo with interpolated mesh UVs; metallic/roughness affect GLB and
SceneVM presentation, not its simple baked lighting. Material recipes are
sampled once at time zero. Opaque, non-emissive surfaces are required; transparent
or emissive surfaces produce errors. Recipe normal/height fields are currently
unused by this adapter; normal-map and animated-material support remain future
work. Both consumers use nearest texture sampling without mipmaps.

## Ruleset and Avatar adapters

```sh
cargo run -p eldiron-character-generator --features eldiron -- \
  crates/character_generator/examples/ruleset-overlay.toml \
  villager target/character-generator/ruleset-villager --ruleset --avatar
```

`--ruleset` resolves the input as an overlay on the bundled official ruleset,
checks its existing validation report, and adapts these extension tables:

- `[procedural_characters.<id>]`: the same fields as `characters.<id>`.
- `[procedural_materials.<alias>]`: the same fields as `materials.<alias>`.
- `[items.<group>.<id>.appearance]`: the same fields as `items.<id>`.

The example extends the existing `linen_shirt`, `wool_trousers`, `leather_shoes`
and `training_sword` templates. Their gameplay fields stay in the ruleset.
Unrelated items without appearance declarations are ignored; duplicate item IDs
across groups are rejected. The adapter uses explicit appearance references,
not guessed geometry from item names or gameplay categories. `catalog_from_table`
can adapt any already-resolved table without the feature; `catalog_from_resolved`
accepts Eldiron's actual `ResolvedRuleset` with `eldiron` enabled.

`--avatar` writes `avatar/character.eldiron_avatar` using Rusterix's actual
serialized Avatar type, plus a separate body-only atlas and metadata. Generated
hand-held items (including shields and spikes) are excluded from these frames so Eldiron can draw equipped item visuals
at the per-frame hand anchors. Clothing and helmets remain baked. These final-color frames
preserve recipe shading and do not provide runtime marker recoloring; regenerate
when appearance changes. Reserved marker RGBs are nudged by one byte to avoid
accidental recoloring by the existing Avatar builder. Ground points remain in
`atlas.json` because Avatar has no matching field.

`--tick-rate 4` controls Avatar animation speed relative to the consuming
client's animation counter. Four ticks/second matches the client's default
250 ms game tick; pass your project's actual rate if different. Coarse counter
cadence can skip baked frames. Reproducible UUIDs identify the Avatar and clips.

## SceneVM adapter

With `eldiron` enabled, `SceneVmCharacter::new(&asset)` converts the generated
materials to SceneVM atlas tiles. `register_materials(&shared_atlas)` registers
them once. `objects(geo_id, origin, yaw, motion, time)` skins the character on
the CPU and returns real `DynamicObject::mesh` objects grouped by material.
Vertex positions include the world-space origin, because the current SceneVM
mesh renderer does not apply `DynamicObject.center` to mesh vertices. Normals
and UVs follow the same mesh used for baking and GLB export.

Submit the objects using `Atom::AddDynamic` through the existing per-frame
scene assembly. The consumer owns clearing/replacing prior dynamics along with
its other dynamic objects; do not clear the whole scene separately for each
character. Material IDs include generated content so changed appearance cannot
reuse stale atlas tiles. Release unused tiles through the normal atlas lifecycle.
This is a direct API adapter, not a hook already installed in the game clients.

The feature currently enables Rusterix's graphics support because its existing
headless build references a graphics-only Widget. The default generator remains
independent of Rusterix/SceneVM and requires no GPU.

Integration checks:

```sh
cargo test -p eldiron-character-generator --features eldiron
cargo clippy -p eldiron-character-generator --all-targets --all-features --no-deps -- -D warnings
```

Tests additionally resolve the official ruleset overlay, deserialize generated
Avatars and build their frames through Rusterix, check that weapons are excluded,
and register materials/submit posed meshes to SceneVM's actual VM. These checks
verify schema and command compatibility, not GPU-rendered visual parity.


## Anatomy and gait review

`examples/anatomy.toml` is an untextured, unequipped mannequin for checking the
base surface in front and side views. Generate it with `--style natural`.
The walk uses two-bone leg IK with a level stance foot and a forward knee pole.
Swing feet clear the floor; pelvis height follows the stance leg. Stance feet move
backward for in-place playback: match character travel to `0.4 * height` per
second for this fixed one-second walk when moving it through a scene. Terrain
contact, stride-speed adaptation and transitions between clips remain future work.
Shoulder pivots are closer to the chest, with a small outward upper-arm rest
slope. Skeleton joint names, hierarchy and hand sockets remain compatible.
