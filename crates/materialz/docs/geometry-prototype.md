# Editable geometry rows in Eldiron

For the illustrated materials/mesh learning path, start with the
[MaterialZ tutorials](../../../docs/docs/building_maps/materialz/start-here.md).
For additional Creator usage, see [Procedural Materials](../../../docs/docs/building_maps/procedural_materials.md),
[Appearance](../../../docs/docs/characters_items/appearance.md) and
[Rigging](../../../docs/docs/characters_items/rigging.md). The dated concept and
milestone notes below retain their implementation/validation history.

The newer selection-driven construction/Prefab experiment is documented in
[Selection recipe status and handoff](selection-recipes-status.md). Its generic
bridge is retained; expansion is deferred while the sandbox prototype starts in 2D.

Status: experimental visual modelling and rigging slice, 2026-11-05. Implemented
in Eldiron's vendored MZ core and Creator. The standalone MaterialZ repository
is unchanged. The official humanoid also runs in the 2D and 3D scene paths described below.

## Trying the editor

Open **Procedural materials → Meshes**. **New mesh** or **New… → Browse
recipes…** opens the recipe browser. Search or filter building parts, nature,
props and primitives; project recipes can also be copied. Preview and adjust
published controls, name the mesh, then choose **Create mesh**. Bundled
starting points include box, wall segment, wall corner, doorway, roof slope,
gable roof, fence, repeated posts, ring walkway, rock, tree, barrel and crate. Each creates an independent, ordinary editable MZ document. No asset
has its own Creator action or special evaluator. **New…** also retains Empty rig,
Human and Sword for rigging and equipment work.

Selecting a mesh in the Tilepicker keeps the map available for placement. Use
**Edit mesh** to open the full editor. Rename, Duplicate and Delete use the
project undo history.

The MZ widget shows construction rows and their properties. The folder icon
opens a selected group; Back returns to its parent. A breadcrumb shows the
editing location. Published group controls provide quick dimension, proportion
and palette edits; the underlying construction remains editable.

Select a Group tile and choose **Asset… → Publish controls…** to open the visual
publisher. Add, remove and reorder controls; edit labels, sections, help text,
current values, neutral/default values and ranges. Choose independent width,
height or depth, scale/offset along an axis, or palette color. Scale, offset and
color can target the entire group or a named part. The preview updates as you
edit. Inside a Group, publishing edits its existing interface. At the root, without a
selected Group, it wraps the construction and suggests dimensions and colors. **Save controls** validates
the complete recipe and records one undo step; Cancel leaves the source unchanged.

Existing weighted, rig-aware and multi-modifier mappings are displayed as custom
mappings and preserved when editing their metadata. **Asset… → Advanced controls
(TOML)…** retains full access to these mappings, including multiple part targets.
**Recipe parameter** publishes typed construction inputs. Each `parameter`
modifier stores a target (group path, operation ID and input key), multiplier
and offset. Multiple inputs can share one control, e.g. box height and half-height
translation. Targets are whitelisted numeric fields, not arbitrary JSON pointers
or scripts. Parameter bindings apply to a temporary child document before
construction; existing FitAxis/Affine/Palette modifiers run afterwards. Duplicate
targets within a group, missing operations, invalid values and invalid geometry
fail evaluation without rewriting the source. Nested groups expose their published
numeric controls as inputs; groups without controls expose their construction.

Group controls optionally store an `expression` string alongside their existing
bindings. Numeric arithmetic, parentheses, `[control-id]` references (or plain
identifiers) and `sin_deg`, `cos_deg`, `tan_deg`, `sqrt`, `abs`, `min`, `max`,
`clamp`, `floor`, `ceil`, `round` are supported. References are scoped to the
current Group and resolved in dependency order on an evaluation copy, before
parameter bindings and post-mesh modifiers. Cycles, missing references,
non-finite values, invalid ranges and excessive expressions fail evaluation.
Limits: 64 controls, 1024 bytes/256 expression nodes, 32 parser nesting levels.
Expression controls are read-only at the published interface and cannot be
rebound by an enclosing Group. Controls with no modifiers are valid inputs or
calculated readouts. Omitted expressions preserve existing recipes unchanged.

The visual publisher includes a Formula field and searchable reference insertion.
Normal Group controls and the recipe browser display current calculated values
without allowing edits; reset changes editable inputs only. Gable roof and Fence
browser recipes demonstrate nested mitered roof panels and continuous rails,
using only portable MZ operations, formulas and bindings.

`Repeat` creates static mesh copies with fixed translation and rotation steps,
1–128 copies within the existing vertex/intermediate budgets. It preserves UVs
and material references, and uses deterministic copy/part IDs. It is available in
Sources, the visual publisher and the recipe browser's Repeated posts example.
The wall recipes bind box dimensions/position; the roof slope binds a profile,
extrusion depth and rotation. No special wall, roof or fence evaluator is needed.

`Footprint { row, holes = [], append = false }` attaches earlier Contour rows
to a static mesh. Coordinates mean mesh X/Z and no renderable parts are added.
`row` is the outer boundary; `holes` names its enclosed cutouts. With `append`,
this region is added to the incoming footprint; otherwise it replaces it. The
Sources inspector provides a contour selector, hole add/change/remove controls
and an Add region checkbox, all saved in the portable recipe.

`Mesh::footprint` contains a `footprint::Shape<[f32; 3]>`: a union of regions,
each an outer ring minus its own holes. Repeat transforms/copies the complete
shape; Assemble combines the shapes of its inputs. Transform, FitTo and
whole-group FitAxis/FitSize/Affine affect every point. A partial Select Parts
clears a whole-model annotation. Ring validation rejects self-intersections,
degeneracy and holes that touch/cross their outer boundary or overlap/nest other
holes. Separate regions can represent islands inside holes. Limits: 128 regions,
256 rings, 4096 points in total, 3–64 points per ring, 32 hole references per
Footprint operation. Footprint intermediates count toward the evaluation budget.
Animated footprint deformation remains unsupported.

The iso cache stores the compound shape per static sprite direction; legacy
simple contours and mesh bounds remain fallback formats. Each placement stays
one sector/one rendered instance. Its optional footprint topology stores indices
into the sector's edge list, so normal copy/paste and ID remapping preserve
regions/holes. Source updates rewire each ring independently and remove obsolete
geometry while retaining sector identity and mesh anchor. Shared map geometry is
preserved. Sector containment, triangulation, collision and walkable support all
use the same compound shape, without bridges between disconnected components
or a cap over a hole. Runtime point collision uses region union with per-region
holes; boundary collision and grid navigation use the individual closed rings.

The Wall corner and Tree examples retain their concave/trunk contours. Doorway,
Repeated posts and Ring walkway demonstrate assembly/repetition and an enclosed
opening, using the same published parameters for geometry and footprints.

`Connector { family, translation, rotation }` appends an invisible placement socket
at the operation's stable tile ID. Its local +Z axis is the outward direction;
rotation is XYZ Euler degrees, matching Transform. Families are case-sensitive
ASCII keys (1–64 characters: letters, digits, `_`, `-`, `.`, `/`). Position and
rotation are whitelisted numeric parameters, so published controls and formulas
bind directly to them. The visual Sources library exposes Connector; its Props
edit position/rotation and its source text edits the family key.

`Mesh::connectors` holds at most 256 validated IDs, family keys, positions and unit
directions. Transform/FitTo and whole-group modifiers move sockets with geometry.
Repeat and Assemble namespace copied connector identities deterministically.
Partial Select Parts clears model-level connectors, as it does footprints.
Connectors are static placement data, distinct from animated rig attachments.

Iso baking caches connectors per static direction and bumps the mesh signature
version when this cache format changes. The Rect mesh placement helper searches
compatible opposing, unoccupied sockets within 0.8 world units, using only baked
orientations. It stores a parent sector, local/remote connector IDs and relative
yaw in placement properties. Snapping is part of the normal MapEdit stroke undo.
The cache synchronization pass resolves this parent forest before regenerating
footprints. Derived parameter edits, height changes and root rotations propagate
to connected children; missing references or cycles freeze affected links.
**Detach connection** removes the incoming link while retaining placement and
footprint mode. Occupancy covers both endpoints of each saved link.

This is a parent forest, not a cyclic constraint solver: a room can close visually,
but its final seam is not an additional constraint. Resizing a closed room may
require rejoining its end. Connectors do not deform building geometry over terrain,
fit a roof to multiple walls, or attach animated actors. The operation remains
portable MZ data; the placement host currently is the 2D Iso Rect workflow.

The illustrated user guide starts at
`docs/docs/building_maps/materialz/start-here.md`. Its native screenshots omit the
AI header. Authored documentation captures are curated under
`docs/static/img/materialz`; temporary reviews/projects stay under `generated/`.

Independent finished-mesh dimensions still use `fit_axis`; existing `fit_size`
controls keep their uniform scaling behavior for character proportions.

**Asset… → Export model…** includes linked geometry snapshots and embedded MZ
surface recipes in a standalone JSON document. **Import model…** adds a separate
editable model with a new identity and normal undo. Shared project models remain
live dependencies during ordinary editing; exported snapshots can also be opened
without the original project. The document size limit is 4 MB.

Geometry opens a dedicated **full-size central viewport**, replacing the map,
map tools and bottom docks while it is active. Construction rows and properties
remain in the full-height sidebar. The viewport's mode selector exposes the actions needed for the current task.
The clip dropdown, play/pause buttons and phase field remain visible in every
mode, including Rig / joints. Surface-material editing keeps its existing
minimap preview.

Rigged models open directly in **Rig / joints**, with the joint tree selected and
bones/handles visible. View model is available when a plain mesh view is wanted.
The authoring modes are:

| Mode | Interaction |
| --- | --- |
| View model | Drag to orbit; the camera menu provides front, side and contour views. |
| Rig / joints | Click a joint to select it; drag in the camera plane. Change view to edit depth. Props also offers parent, translation and rest rotation. |
| Add joints | Click a parent, then click elsewhere to create a child and its bone profile. Alt-drag orbits instead of adding. |
| Profiles | Pick a part in Props. Drag ring centers or width/depth handles; right-click a handle to remove a ring, or elsewhere to insert a midpoint ring. At least two rings remain. |
| Pose | Choose a clip, scrub to a phase, and drag a joint to change rotation. Alt-drag changes the third rotation axis. A key is created at that phase. Props exposes exact track/key values. |
| Attachments | Drag an attachment socket or an item's Grip origin; Alt-drag rotates it. Props provides exact offsets and rotations. |

Scroll over the viewport to zoom in editing modes. Mirror
copies a joint branch and its bound profiles across the model's X plane. Add
Profile creates a profile on the selected bone (Shift creates a rigid joint detail,
useful for eyes or decorations); Delete removes the selected branch
or profile. The name field renames the selected joint or profile. New Clip
creates an editable clip. A drag commits one undo step; Escape cancels it.
**Back to map**, switching to Materials, or choosing another sidebar tab restores
the previous map/dock layout and minimap split without changing map selection,
camera, or the authored model. Returning to Geometry reopens the large workspace.
The New menu replaces the former row of preset buttons.

Selecting joint/profile modes on the human opens its Body group automatically.
The base body uses **two tiles**: an arbitrary joint tree and a collection
of bone-bound profiles, followed by the ruleset's surface assignments. It uses the same resources and tools as Empty rig.
Changing a rest joint reshapes the bound profiles; this also works for horizontal
spines and limbs. Reparenting preserves a joint's world pose and rejects cycles.

## Human, creatures and equipment

The [human recipe](../examples/geometry/human.mzgeometry.json) has seven main
rows and nine tiles: Body, Sword, Shield, Equipped human, Idle, Walk and Attack.
Its body has 17 joints and 29 profiles. The equipped output contains 34 mesh
parts and 9,696 triangle-list vertices. There is no Human geometry kernel or
character-generator dependency.

Body group controls expose height in metres and relative limb, torso, shoulder,
hip, chest and head proportions. Colors use the project palette. Group controls
resize geometry and rest skeleton together; hand attachments follow. Detailed
edits change the base recipe, before published controls are applied. Existing
published selectors name the original human parts; newly created branches need
explicit selector additions if they should participate in those controls.

A quadruped review is built from an empty document using the same add-joint,
profile and mirror functions used by the viewport. It has a horizontal spine,
four legs, ears, tail and an authored diagonal trot. It exercises arbitrary
branching rather than adding an animal-specific generator. The review exports
its standalone recipe to `/tmp/eldiron-visual-quadruped.mzgeometry.json`.

An item's Grip is its local attachment frame. Attach fits that frame to a named
joint plus an editable socket offset. The sword follows the right hand; the
shield follows the left. Removing a branch removes its profiles and motion
tracks and disables affected attachments while retaining their item recipes.

The official weapon library includes editable Training Sword, Hand Axe,
Round Shield, **Novice Mace, Hunting Bow and Training Spear** models. Open
**Geometry · Edit in MZ** from the item's project-browser entry. The top-level
group exposes dimension controls; open it to edit individual construction rows.
The ruleset binds each item to its shared geometry asset, so changes also reach
equipped appearances in 2D and 3D. Project overrides survive sync and saving.

The mace has six real extruded iron flanges. The bow has a curved lofted stave,
linen string, leather riser and nocked arrow. The spear has a tapered shaft,
binding and a four-sided leaf point. Each uses ordinary MZ operations and the
project palette. Their clean surface recipes use the same palette slots as
the corresponding icon components. No item-specific geometry kernel is needed.

Grip frames align them to the existing right-hand socket. Bow and spear grips
compensate for the sword socket's carry angle; the bow also turns outward so
its silhouette remains readable. Equipment stays rigidly attached through the
existing Idle, Walk and Attack clips on human, orc and skeleton. This model
pass retains those clips; bowstring deformation, arrow release and dedicated
archery/thrust animations are separate work.

To review all three weapons on all three races, plus normalized standalone
model views and their matching inventory icons:

```sh
cargo run -p eldiron-creator --example appearance_scene -- test_projects/3DStarter.eldiron /tmp/mz-weapons --default-palette --weapons
```

Use `Hideout2D.eldiron` for a populated 2D scene. `weapon-attachments.png`
compares headings and the attack midpoint, `weapon-models.png` compares model
angles, and `weapon-icons.png` contains the corresponding native 32px icons.

## Ruleset surfaces

### Orcs, skeletons and inventory icons

The official ruleset now supplies `orc` and `skeleton` MZ appearances alongside
`humanoid`. Race defaults resolve these names through the same runtime path;
there are no creature-specific geometry operations. Open their project assets
in **Procedural materials → Meshes** to edit proportions, profiles, joints,
clips and attachments. Their standalone recipes live in the ruleset's
`assets/geometry` folder. All three share the human equipment sockets and
Idle, Walk and Attack clips, with the same materials in 2D and 3D.
Edits to these project assets already update their corresponding race. The
separate **Use as human** action explicitly replaces the human default with
the selected model.

The orc uses a broad body, pointed ear profiles, a narrow hair crest and ivory
tusks. The skeleton uses separate skull/jaw/teeth, limb bones, a pelvic girdle
and six open rib hoops built with Contour, Revolve and Skin. Skull cavities
also have an editable MZ face surface. Anatomy colors use project palette
bindings (`orc-skin`, `bone`, `bone-skull`, `bone-shadow`).

The skeleton retains invisible human-semantic fitting envelopes so existing
Select Parts / Surface Offset garment recipes can dress it. These parts use
the zero-opacity `fitting-guide` material. Their geometry remains available
to clothing construction; fully transparent materials produce no runtime
draw calls or sprite rasterization. Bone profiles remain inside fitted shells.
New anatomy parts are included in the published body controls.

Every official item now has its own native **32×32 MZ icon**: weapons, armor,
clothing, containers, ammunition, reagents, materials, resource nodes and coins.
The 41 items have 42 authored states including the unlit torch. Use
**Icon: On → Edit in MZ** in an item's project-browser entry to edit it; the
torch also has **Icon: Off → Edit in MZ**. Their
standalone `assets/procedural_icons/<item>/icon.material` and `palette.json`
files contain stock shape/material rows, crisp alpha and project palette
indices. Inventory art is an independently editable composition, rather than
a camera render of the worn mesh.
This preserves deliberate inventory silhouettes; editing an icon does not
alter garment geometry or its surface graph.

The lit torch loops through four MZ frames at the ordinary server-tick cadence
(250ms by default); the unlit torch has one frame. The same palette-bound frames
feed inventory and the torch's existing world-state tile identities. Packed
roughness/metallic/opacity/emission maps persist with those tiles across saves.
The old bitmap item artwork is excluded from normal runtime texture loading;
action/intent artwork and user-supplied bitmap icons remain independent.

To export the complete inventory sheet and individual native pixels without
starting a game or opening a GPU window:

```sh
cargo run -p eldiron-creator --example ruleset_icons -- /tmp/mz-item-icons
```

For native scene and animation review, run:

```sh
cargo run -p eldiron-creator --example appearance_scene -- test_projects/Hideout2D.eldiron /tmp/mz-races-2d --default-palette --races
cargo run -p eldiron-creator --example appearance_scene -- test_projects/3DStarter.eldiron /tmp/mz-races-3d --default-palette --races
```

These review copies export `2d.png`, `3d.png`, a bare/equipped species/pose
sheet in `races.png`, and the actual 32px icon pixels in `icons.png`, without
rewriting project fixtures.

The official library lives in `crates/ruleset/rulesets/eldiron/v1/assets/materials`.
Each standalone `.material` has a companion `.palette.json` containing project
palette indices. Skin, hair, tunic, trousers, leather, eye white, iris, lips,
steel and wood use ordinary MZ tiles. Torso/sleeve and leg/waist variants keep
collars, cuffs and belts on their intended body regions. Clothing uses editable
Circle/Rounded Rectangle masks and Mix rows for seams, edging and fastenings,
with predominantly plain fabric. Skin and tiny facial regions stay clean;
hair uses combed strands. There are no authored RGB color ramps.
Derived linear color values in editable bundles are resolved caches of those
bindings. A fresh palette with fewer colors maps unavailable canonical slots
to valid project indices; ordinary palette switching then remaps those bindings.

In **Procedural materials → Materials**, use the **Ruleset material** dropdown
above the rows. It opens the same PBR editor and minimap used for tile materials.
Edits use the existing project undo history and are saved as project overrides;
loading a ruleset never replaces them. These library entries stay out of the map
Tilepicker. Their stable resource names are `ruleset:material/skin`,
`ruleset:material/tunic`, etc., so geometry refers to the library rather than
copying material graphs into each body part.

The ruleset humanoid assigns materials to all 29 body profiles, including eyes,
iris and lips. Its Body group retains the same joint/profile tools and published
proportion controls. Clothing/skin colors are edited in the material library.
Loft side and cap UVs are authored before posing and follow the skinned vertices.
Side UVs run from V=0 at the top to V=1 at the bottom, consistently in the
geometry preview, 2D sprite projection and 3D scene renderer;
**Texture → Mapping → Surface UVs** uses those UVs. Planar projection is also
available for flat objects and icon artwork.

The runtime bakes each distinct library asset/palette/resolution once into 64×64
color, opacity and packed roughness/metallic/emission maps. Baking uses the MZ
reference backend; the normal material editor retains its GPU preview. The geometry
preview shares these cached maps instead of compiling each body material again. Standard
Color, Mix and 2D Perlin tiles have parity-tested reference specializations;
edited algorithms still use the interpreter. Both 2D sprite projection and 3D
SceneVM meshes consume the exact same maps. RGB output is fitted to the project
palette; the two render paths retain their respective lighting. Shared material
content is deduplicated across body regions and characters. Editing a library
material or switching palette invalidates appearance caches without rebuilding
the world material pages.

Geometry assigns existing surface resources; this workspace does not author
textures. The shield is a simple extruded plate linked to
`ruleset:icon/round_shield/on`, so its artwork comes from the selected ruleset.
The model retains that resource identity rather than an appearance-owned material
copy. Texture editing remains in the ordinary material workflow.

The resource resolver supports the current bundled icon/animated-state resources.
Bitmap resources pass through MZ's Image / Tile Source and are fitted to the
project palette, matching the ruleset's runtime artwork. For state resources with
stable tile IDs, an associated project MZ bundle takes precedence. Moving the
whole ruleset catalogue to native MZ recipes remains later work; the geometry
reference can remain unchanged when that resolver is extended.

**Texture** assigns a resource and UV mapping to all parts or a selected
part. Resolution is 16–512; origin/span or Fit UV controls the mapping. UVs are
applied before posing. Resource changes, ruleset selection and palette edits
invalidate derived images without rewriting the model recipe.

GPU material results and texture-array bindings are cached. The mesh shader
samples color/opacity, roughness, metallic and emission maps with nearest
filtering; color and emission are sRGB. Alpha is cutout, not sorted translucency.
Planar UVs clamp. Animated mesh textures are not yet played back; live flames and
similar effects use the particle attachments described below. The world
material editor, animation and baking paths remain unchanged.

## Animation and rendering

Clips store duration, looping and joint tracks with normalized key phases,
local translations and rotations. Keys interpolate with quaternion slerp.
Adding a pose key between existing keys starts from the interpolated pose;
loop endpoints stay matched. Rest, Idle, Walk and Attack, play/pause and the
phase field stay visible in the viewport in every mode. Rig mode finds compatible
clips in parent groups automatically, so the human's Idle/Walk/Attack clips play
while Body construction remains open. Play, pause and scrubbing do not change
the authoring mode or saved model. Joint handles follow the animated pose.
Starting a bind-rig edit returns to Rest pose first; Pose mode continues to edit
animation keys. Switching between View, Rig and Pose retains the clip and phase. Skeleton and attachment axes
can be overlaid. Camera framing stays stable across a clip and during drags.

Preview skinning runs on the CPU and uploads posed vertices to one fixed GPU
shader using Eldiron's device and queue. Animation does not rebuild the graph or
compile a new shader per frame. Geometry requests 60 FPS independently of the
game's presentation/tick settings. Playback uses elapsed wall time and resets its
clock while the workspace is hidden.

One worker renders an in-flight request and retains only the latest pending
request. Completed camera/animation frames remain displayable when a newer frame
is requested; each carries its own projection and posed joints so handles and
picking stay aligned. Source, selection and viewport changes invalidate
incompatible results. Color/depth/readback targets and material bindings are
reused. Saved graph labels are cached and full documents are not parsed again
for every orbit frame. Texture source changes update cached MZ instances;
structural changes compile that material again. Mesh editing itself does not
compile shaders.

This remains a workflow experiment. Profiles are overlapping capped meshes,
not a watertight anatomical surface. There is no weight painting, IK, automatic
gait generation, retargeting, facial rig, root-motion extraction or terrain
contact solver. Walk/trot/attack clips are authored examples. Runtime avatar
replacement, gameplay equipping, animation events and inventory icon output are
still integration work. The preview lighting is an inspection light, not SceneVM
lighting parity.

## Core and persistence

The [portable geometry module](../src/geometry/mod.rs) owns the typed row document,
validation and mesh evaluation. It has no Creator, Rusterix, SceneVM or character
generator dependency. Resources include groups, joint trees, profile collections,
contours, extrusions, revolutions, lofts, material/texture assignment, skinning,
grips, attachments and motion clips. Traditional individual Joint/Skeleton/Skin
rows remain usable for custom construction.

Documents are standalone JSON source stored in `Project.procedural_appearances`.
Meshes and GPU buffers are derived. Geometry documents remain separate from the
surface parser; a unified appearance bundle is still future work. Stable IDs
connect rows, profiles, joints and motion tracks. References must target earlier
rows of compatible types. Validation rejects invalid trees, missing bindings,
invalid dimensions, folded deformations and resource-limit violations. Invalid
edits preserve the working recipe.

There are limits of 64 joints per editable tree, 128 profile parts, 32 rings per
profile, six nested group levels and a retained geometry budget. Contours support
3–64 points without holes. There is no general Boolean union, bevel, sweep or SDF
meshing implementation. Ranged numeric text edits support both dragging and typing.

## Verification

```sh
cargo test -p denrim-materials --features integration --test geometry --test human_geometry --test geometry_groups
cargo test -p eldiron-creator --lib geometry_editor -- --test-threads=1
cargo check --workspace --all-targets
```

Native GPU reviews export actual renderer output and workspace snapshots:

```sh
cargo test -p eldiron-creator --lib orbit_preview_benchmark -- --ignored --nocapture
cargo test -p eldiron-creator --lib full_viewport_gpu_review -- --ignored --nocapture
cargo test -p eldiron-creator --lib visual_authoring_gpu_review -- --ignored --nocapture
cargo test -p eldiron-creator --lib human_gpu_review -- --ignored --nocapture
```

The full viewport review writes `/tmp/eldiron-full-rigging-workspace.png` using
the actual native UI and GPU renderer. The visual authoring review writes `/tmp/eldiron-visual-*.png` (creature, rig, trot frames, textured
shield and workspace). The human review writes `/tmp/eldiron-mz-human-*.png` and
Body/Colors control reviews to `/tmp/eldiron-mz-group-*.png`.

The broader direction and acceptance gates remain in the
[unified appearance concept](unified-appearance-concept.md).

The continuous-orbit benchmark measures the actual Creator update/draw and GPU
worker path at a 60 Hz request rate, writing the presented FPS and UI timings to
`/tmp/eldiron-rig-orbit-performance.txt`. FPS is measured, not inferred from the
configured target.

## Rig editor interaction audit (November 2026)

Committed UI events are handled before the geometry workspace refreshes controls
and renders. Native dropdowns retain their hovered selection until mouse-up;
per-frame synchronization must not replace the selection of an open menu. This
also keeps queued row-editor commands from being discarded by a premature refresh.

Tool changes preserve the camera and joint/profile selection when the editing
scope is unchanged. Contour view is offered only for a selected Contour or Loft.
The skeleton overlay is an independent toggle; editing handles remain available
when the overlay is hidden. Group navigation still frames the newly opened model.

Play from Rest starts the first available clip. Empty rigs disable playback and
scrubbing; entering Pose does not create a clip—New Clip is the explicit action.
Generic inspector refreshes do not seek a paused animation back to its keyframe;
selecting a motion operation, track, or key does. Editing a rig/profile switches
to Rest so it changes the bind shape rather than silently authoring animation.
Joint/profile actions are disabled when the selected resource cannot support them,
including deleting or mirroring the skeleton root. Disabled native button groups
reject input. Name fields retain the user's active text while source data refreshes.

Regression coverage includes native dropdown mouse-down/drag/frame-update/mouse-up
sequences, overlay persistence, Play from Rest, empty-rig mode changes without
source mutations, and disabled transport input.

## Ruleset scene integration prototype

### Shared items and native ruleset icons

The training sword, round shield and hand axe now have standalone geometry in
`crates/ruleset/rulesets/eldiron/v1/assets/geometry`, and native 32×32 MZ icons
in `assets/procedural_icons`. Their colors use companion palette-index manifests.
The practice sword uses wood/leather; the shield separates matte painted face
from metal rim/boss/rivets in its PBR outputs. The default sword is about 0.88m
overall, with a 0.58m blade and 0.20m grip. The 0.56 × 0.62m shield uses a revolved
curved shell, a concave wooden back and rear hand grip; its curve depth is editable.
Its painted front projects the same MZ artwork used for the inventory icon. The
hand axe uses ordinary contour, extrusion, revolution and shared PBR surfaces,
with a palette-bound MZ icon and adjustable head/haft proportions. Other ruleset bitmap artwork continues
to work and can be moved into MZ individually.

Expand an item in the Project browser. **Edit in MZ** beside Icon: On/Off opens
that state's material editor. A bitmap is ported into Image / Tile Source,
including its animation frames. The portable image becomes the MZ graph's data;
the previous bitmap state is retained by project undo. Existing procedural
icons open their shared graph. **Geometry · Edit in MZ** opens the linked model,
or creates an editable box and grip when the item has none. Creating these
bindings is saved and undoable, and ruleset gameplay refreshes preserve them.

Item attributes bind `appearance_icon`, `appearance_icon_off`,
`appearance_geometry`, and optional `appearance_ground_flat`. Project MZ
overrides drive inventory/help icons and 2D ground sprites. In 3D, items with a
model use actual geometry at authored metre scale; flat ground placement is
available for weapons/shields. Explicit image/tile world sources still work.
Custom project bitmap icons continue to override inherited ruleset artwork.

Geometry Asset tiles reference another project model, with a portable dependency
snapshot. Hosts resolve the current shared asset; editing its source updates
instances. The Sources tab lists other project models for drag-and-drop reuse.
Open a linked tile with the folder button to edit its source. Cycles and excessive
dependency depth are rejected; in-use geometry cannot be deleted in the editor.
The humanoid references the shared sword/shield models. Hand attachments also
accept item-bound custom models with a published grip, in both directional 2D
character sprites and 3D skinned geometry. Unsupported items retain their ordinary
visual fallback instead of becoming an unrelated weapon.

Icon output retains its 32px resolution independently of map pixel density.
Geometry, icon, palette and surface edits invalidate the relevant appearance
caches and browser thumbnails; shared PBR maps are reused by the consumers.
Animation, attachments and inventory images remain separately authored outputs.
The 2026-11-05 integration pass adds **Model Render** in the material source
catalogue. Select its model in Props → Source, then adjust camera angle/elevation,
pose, clip, pixel resolution (16–256) and frame count (1–16). Its transparent,
palette-fitted image is an ordinary Image / Tile Source, so subsequent MZ rows
can process it. Connect its alpha to Opacity for a cutout icon. The image is
embedded in the saved material; source edits refresh it on a GPU worker, and a
saved content key avoids work on reopening. Missing sources retain their last
image. A model cannot texture itself with its rendered icon: feedback is rejected.

Character and item meshes now retain animated material frames instead of taking
only frame zero. The 2D sprite cache includes the selected texture frame; 3D uses
stable per-frame texture identities and uploads each once. The modelling viewport
also selects material frames while playing/scrubbing a clip. Playback uses the
material's explicit duration and the runtime clock, independently of skeletal
clip restarts. Static surfaces retain their existing cache behavior.

Runtime character transitions blend local joint transforms over 0.12 seconds
by default, preserving bone lengths, attachments and effects. Interrupting a blend
starts from its last displayed pose. In character TOML, `avatar_blend_time` changes
this interval (0 disables it), `avatar_stride` gives distance per walk cycle,
and `avatar_walk_rate` optionally overrides cadence derived from actual movement.
Attack clips retain the ruleset weapon duration.

Select a Motion tile and use **Asset… → Animation events…** for ordered TOML
markers (`[[events]]`, `time = 0.5`, `name = "release"`). Preview playback reports
crossed markers in the status bar. Runtime consumers can drain presentation
notifications once per entity through `drain_animation_events`; repeated render
passes do not duplicate them. Walk clips publish footstep markers, bow shots a
release marker, and spear thrusts an impact marker. These are presentation hooks;
they do not move damage or projectile authority out of the server ruleset.

Automatic terrain composition starts with the **Auto road** shape in the existing
map Compose grid. Use it on Rect-painted cells, then copy that composition with
the eyedropper on the same map surface. Matching road-material layers connect
into rounded ends, bends, T junctions and crossings, retaining width, soft edge
and world-coherent wobble. Painting, erasing and undo change the derived field,
not the authored layer stack. The first implementation supports up to 256 cells
per field; geometry faces and terrain/sector paint grids are separate domains.
Saved Tilepicker copies acquire neighbour context when painted. Use a separate
copy per map/paint grid; sharing one baked identity across different grids is
rejected rather than silently selecting the wrong junction. Removing the original
owner also removes its road arm; remaining copies supply the preview context.
Auto-road outputs use continuous world UVs, so their output tiling switch is
disabled. Source materials can still repeat independently.

Remaining extensions include general biome-edge rules, larger/chunked neighbour
fields, automatic cross-surface/cross-map road instances, nested model-render
dependencies, animation-marker bindings to presentation nodes, IK, retargeting,
and imported skinned meshes. These are not implied by the current controls.

### Ruleset clothing and armor

Linen shirt, wool trousers, leather shoes, padded armor, leather vest, chain shirt
and cleric vestments now define their worn MZ surfaces in the canonical rules
document. Twelve editable material recipes provide torso, sleeve, trouser/waist
and shoe variants. Construction includes seams, cuffs, diamond quilting, buckle
closures, woven trim and staggered open chain links. Every authored color is a
project palette binding; iron rings also carry metallic/roughness maps while
their underlying padding remains nonmetallic.

`appearance_worn_geometry` selects an editable garment geometry source. The seven
ruleset garments now each own an MZ document in `assets/geometry/garment_*.mzgeometry.json`.
A **Wearer body** input supplies the current character, **Select Parts** extracts
semantic mesh parts and **Surface Offset** builds the fitted shell. Garment rows
own their MZ texture bindings. The source snapshot also permits standalone preview.
Actor body edits are supplied at evaluation time; runtime fitting does not rewrite
the character or garment document.

Optional `appearance_materials` is an array of explicit `mesh-part=material-source`
bindings, for example `body-torso=ruleset:material/chainmail`. Sources may also be
project MZ material UUIDs. Part names belong to the geometry recipe, allowing new
garments and different creatures without a Rust garment-type enum. Bindings are
applied in sorted equipment-slot order; later slots win if they target the same
part. Surface-only bindings can use `appearance_padding` (0–0.03m); geometry-backed
garments set their offset in the MZ graph. Matching semantic parts replace their
body counterpart; additional garment parts join the same skeleton and preserve
the body's sockets. The current cleric recipe adds four thin lofted front/back
tabard panels, bound to hip/knee joints. These panels have independent profiles,
UVs and a palette-bound gold/blue hem material, and move with walking/attack clips.
This is skeletal deformation rather than cloth simulation.

The same compiled material/fit variant feeds directional 2D sprites and native
3D meshes. UVs, rig, skin weights and sockets stay intact. Framing is measured
before padding so equipping armor does not resize the character. Unequipping
returns to the authored body appearance, and torso/legs/feet are independent.
Runtime caches include worn bindings and resolved map contents; source/palette
edits and project undo refresh the appearance. Item data, MZ graphs and palette
indices survive normal project saving.

Expand a clothing or armor item in the Project browser and choose
**Worn geometry · Edit in MZ** opens the full geometry workspace. Its default
preview includes the wearer to judge fit, with the same garment motion clips.
The wearer input is supplied by the host; edit the character's own geometry to
change anatomy. **Worn · … · Edit in MZ** opens a shared surface. Torso, sleeve
and tabard materials have separate entries. Geometry and surface edits use normal
project save/undo. The geometry sources catalogue exposes the two new general
mesh operators; they also work for geometry beyond clothing.
The review harness accepts `--clothing` to compare the five torso outfits with
wool trousers and leather shoes in actual scenes, plus front/side/back sprites.

The official ruleset now owns `assets/humanoid.mzgeometry.json`, an editable
copy of the human recipe. Loading/creating a project seeds its project-owned
ruleset copy without replacing edits. Characters bound to `humanoid` (including
the bundled bitmap avatar's UUID) resolve to that recipe in both scene renderers.
An explicitly supplied project bitmap avatar remains authoritative. Orc and
skeleton bitmap avatars continue to work.

In the Geometry workspace select **Ruleset humanoid**, edit it, return to the map
and play. To try an existing model, select it and press **Use in game** beside New.
This copies the whole root recipe, including its groups, gear and clips, into the
ruleset humanoid slot and selects that copy. The action is saved and undoable;
subsequent edits and undo update the scene caches. Use isometric 3D to see the
player's body; first-person views retain their usual local-body suppression.

Flat dropped items face upwards (+Z in model space), including the shield’s
painted face.

2D uses cached, transparent 64×64 sprites generated from the same posed mesh in
eight directions. Sprites use a simple fixed light and nearest project-palette
colors, sampled at 16 animation frames per second. 3D submits CPU-skinned geometry
to SceneVM's native dynamic mesh path, using its ordinary depth, lights and material
atlas. Material tiles are deduplicated by content, not allocated per character or
per geometry edit. No material-specific shader is generated.

Idle, Walk and Attack use motion row IDs or labels. Idle/Walk durations are seconds;
Attack is fitted to the entity's `avatar_attack_time` window so gameplay does not
truncate a long authored windup. The render clock interpolates between game ticks and freezes when those ticks
stop. Attack serial changes restart the clip; non-looping clips hold their end.
Recipes can also be bound explicitly with `avatar = "<project appearance UUID>"`.

The humanoid exposes `main_hand` / `off_hand` attachments. Item-bound geometry is
used when available; otherwise matching sword/shield items use the bundled
attachment fallback. Other weapons are not replaced with a sword. Mesh scale/feet
alignment are taken from the body, so equipping gear does not resize the character.
Additional humanoid variants, per-entity marker skin/clothing overrides, and more
animation actions remain later work.

The scene review harness uses the actual GameWidget entity update and rendering
paths on an in-memory comparison group, leaving project files unchanged:

```sh
cargo run -p eldiron-creator --example appearance_scene -- test_projects/3DStarter.eldiron /tmp/mz-room
cargo run -p eldiron-creator --example appearance_scene -- test_projects/Hideout2D.eldiron /tmp/mz-hideout
```

It exports `2d.png`, `3d.png`, `directions.png` and `materials.png` (swatches follow
the material registry order, including sleeve and waistband variants).
Use the room's 3D image and Hideout's 2D image for the respective scene comparisons.
An optional third argument `--default-palette` reviews these old project fixtures
with the current Endesga 64 palette, without changing either project file.

Verification for this integration: 26 geometry-editor tests, two runtime appearance
tests, one ruleset binding/cache test, and eight project/item synchronization
regressions pass. Native scene reviews exercise the room in 3D and Hideout in 2D.
The standard workspace/all-targets check passes. A broader shared-library run also
exposed four independent failures in unchanged code: three construction-graph
connection-validation tests and the Stonefall wall-assembly fixture assertion.
A standalone no-default-features engine check currently fails in the existing
`batch` exports and rasterizer; the new appearance module's GPU entry points are
feature-gated. These unrelated failures are not changed by this prototype.

Ruleset prop coverage now includes standalone world recipes for all 41 official
items, plus the unlit torch state. Worn bags, charms and censers use ordinary
Asset/Grip/Attach rows consuming `geometry:input/wearer`; no item-specific geometry
operation was added. Merging worn parts preserves the layer's attachment frames
as well as skin weights. Rotated normals tolerate floating-point roundoff at the
unit component boundary. The `ruleset_models` Creator example exports all item
states next to their separately authored inventory icons and accessory poses.


Weapon animation uses the same generic geometry operations. **Attach** can mount
an already skinned item: its joint tree is grafted under the host socket, while
skin indices and nested attachment frames are remapped. **Assemble** and worn
part replacement merge compatible rig branches and reject conflicting joint
identities or a combined skeleton above 64 joints. This allows a flexible bow to
keep its own limb, string and arrow bones when equipped on any humanoid rig.

Motion keys have a discrete `visible` channel (shown as **Joint branch** in the
key inspector). Hidden joint branches clear fully bound mesh parts in the posed
copy while preserving part IDs and material order. The bind mesh is unchanged;
subsequent keys can restore visibility. Loop endpoints must agree on visibility.
Character motion and equipped item motion with the same clip ID combine by joint
ID on one normalized timeline. The native 3D mesh and the 2D sprite projection
sample this exact same composed pose.

The ruleset orc now has an independently sculpted profile baseline, neutral
proportion controls, mass/stoop controls and heavy authored motion. It retains
all 17 humanoid joint IDs and the semantic body part IDs used by fitted garments.
Clothing consumes the current wearer profiles and offsets their surfaces; held
and worn items continue to use the shared Grip/Attach operations. The orc change
introduces no character-specific geometry or animation evaluator.

## Particle Effect tiles (Eldiron)

Add **Particle Effect** from Sources → Effects after the mesh/Assemble tile it
belongs to. The tile preserves its mesh input and adds a named effect attachment.
Flame, Smoke and Sparks presets use Eldiron's existing particle simulator. Position,
rate, size, lifetime, speed, turbulence, light strength and range use the normal
ranged text controls. Four color chips bind to project palette indices. Palette
switching includes these ramps in its matching constraints.

The attachment position is expressed in the mesh's rest coordinates. An optional
Follow joint binds it to an existing skeleton joint. Attach, Assemble, transforms
and geometry groups preserve effects; an attached item follows its hand during
idle, walking and attacks. In the viewport's **Attachments** tool, drag the effect
marker to place its origin. Effects animate in the geometry viewport even without
a motion clip. The modelling viewport uses fixed inspection lighting; the item
Appearance preview and map show the attached point light.

The torch's On geometry has `flame_tip` after Torch assembly, replacing its static
flame meshes. Its Off geometry has no effect. Inventory icons retain their crafted
MZ animation; effect-bearing ground items use the live model appearance. Held and
ground effects render in 2D and 3D and extinguish with the item state. The emitter
follows the posed attachment, while existing particles remain in world space;
flames rise upwards even when the torch tilts. Each world placement owns simulation
state, while authored geometry and materials remain shared. Effect definitions
are saved with the geometry document; active particles are never serialized.

The human, orc and skeleton recipes include editable torch idle/walk clips that
hold the flame clear of the body. Item-owned clip selection also works for off-hand
tools, while the primary weapon retains its attack clip.

The ruleset's `tool` slot maps to the off hand. Its occupied-slot reservation prevents
a torch from sharing that hand with a shield or a two-handed weapon. No torch-only
particle simulator or authored RGB colors are involved.
