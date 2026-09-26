# Stonefall standalone project conversion audit

Audit date: 2026-09-26.

Implementation status: the standalone file and matching starter are converted, with 20 Entity graphs and nine Behavior graphs. Flat walls are represented by 108 native spans replacing 208 baked wall bodies; niches and decorative details remain ordinary editable geometry. Party/support services and nodes are implemented. Inventory-owned item graphs now retain ownership and can update carried torches. Wall meshes regenerate from assemblies on project decode. Source tooling, source folders, prototype project and current source-workflow docs are removed; web/release paths and recipe fixtures are updated.

Static graph/port/metadata/localization checks and workspace metadata validation passed. Rust regression tests are authored but unrun. No builds or play tests performed. Gameplay, wall appearance and collision still need checking in a user-built Creator/client.

The original audit follows; paths under source_projects describe the pre-conversion baseline.

## Baseline

Use `source_projects/stonefall-dungeon/build/stonefall-dungeon.eldiron` as the conversion input. It is a legacy JSON project with seven character templates, 45 item templates, three screens, 22 tiles, 15 procedural recipes, one region, eight character instances and three item instances. It contains zero node graphs. All seven characters have behavior scripts; Torch and Exit Marker also have scripts.

`starters/projects/StonefallDungeon.eldiron` is another generated copy with different asset/object UUIDs. Do not combine the two projects. Replace the starter from the validated standalone project, or remove the duplicate entry if it is no longer wanted.

The output already embeds geometry, recipe sources, tile data, screens and settings. It can be retained without the source compiler. Preserve the start/play/inventory screens, first-person grid camera, party panels, renderer settings, hybrid simulation, mesh collision and initial player selection behavior. The current configuration includes `auto_create_player = true`; verify actual start-screen behavior before changing this flag.

## Behavior conversion

- Player: Startup camera, death/respawn feedback, kill feedback. Replace legacy attack/take key intents with the supported ruleset commands. Preserve talk/look input and screen shortcuts. Ruleset actions should execute directly rather than require equivalent attack scripts.
- Five enemy templates: Routine, Random Walk, Lookout, Engage, recovery to Routine, damage response, death loot and hit feedback. Preserve the archer/warden/brute distinctions and equipment. Translate script movement speed differences deliberately into entity/ruleset configuration rather than assume identical defaults.
- Alden: convert TOML dialogue to the current conversation nodes, recruitment with success/full-party outputs, and conditional healing of a damaged party member. Party joining exists in `region_host.rs` through `join_entity_party`, but the node registry/WorldServices does not yet expose a corresponding party node. Add a reusable party service and module instead of calling the scripting host. Healing uses the existing ruleset action; health eligibility must retain the living-and-at-most-half-health condition.
- Exit Marker: preserve look/use/bump handling, Bone Key possession checks, blocked/unlocked state and feedback. Use current inventory guards and attribute/message nodes; verify event subjects for each entry event. Do not introduce a new exit behavior: the existing script unlocks passage, rather than switching screens or completing a quest.
- Torch: inspect and retain startup/activation lighting behavior alongside current built-in item/ruleset handling; do not duplicate effects already handled by the runtime.
- Entity configuration: convert explicit character/item settings to Entity branches and player bindings to an Input Mapping branch. Preserve ruleset item links and genuine overrides; avoid freezing current rule-derived properties. Retain starting inventory/equipment and companion-specific attributes.

## Geometry conversion

The map contains 2,257 geometry objects: 2,256 in `eldiron-source` and one recipe-effect object. It has no authored wall assemblies; its single sector is not the room layout. Walls are generated boxes and subdivided decorative pieces. Floors, ceilings and transitions are also ordinary baked geometry.

The embedded 43-by-35 terrain grid and source symbol/recipe definitions provide the layout needed to reconstruct walkable boundaries. Ceiling heights are 2.25, 3.0 and 4.25. Convert these boundaries to connected WallAssembly nodes/spans with appropriate thickness, height overrides and original material sources. Check junctions, corridor clearances and grid alignment against the original occupancy, not merely visual similarity.

Do not relabel baked objects as wall-generated objects: the wall tool rebuilds geometry from assemblies and replaces objects bearing its generated-wall tag. Create real assemblies and remove only the obsolete wall bodies after replacement geometry is validated. Preserve floor and ceiling geometry initially, including height-transition pieces.

Niches, pillars and torches need explicit treatment. WallOpening describes an opening through a span, with no recess-depth field; a blind niche is not automatically equivalent. Preserve these as ordinary editable geometry, or add proper recess support if fully wall-driven niches are required. Preserve emitters, particles and procedural materials when wall faces/IDs change. Avoid retaining old wall bodies underneath replacement walls.

## Removal dependencies

The workspace lists `crates/source`, but the inspected app/client manifests do not depend on it. Removal still requires:

- Remove the workspace member and the crate's Cargo.lock package entry; prune exclusively unused lockfile dependencies carefully.
- Remove source-binary release matrix entries and dedicated Linux/Windows upload jobs in `.github/workflows/release.yml`.
- Update web-game staging and docs workflow paths to `test_projects/StonefallDungeon.eldiron`.
- Replace the starter copy consistently.
- Move the two Stonefall recipe fixtures referenced by `creator/src/recipe_utils.rs` before deleting source folders.
- Remove obsolete source-project documentation, navigation/links, prototype source assets and the source concept document. Historical release posts can remain historical.
- Review `terminal_screen::source_terrain`, which still reads `eldiron_source_terrain` from sectors. This is separate from the compiler dependency; preserve terminal functionality or move it to neutral map metadata rather than deleting the reader blindly.

## Recommended sequence and acceptance checks

1. Establish `test_projects/StonefallDungeon.eldiron` from the output and record baseline content/geometry.
2. Convert Entity and Behavior graphs, implement missing modular party operations, and validate startup, input, combat, loot, recruitment/healing and exit unlocking.
3. Replace wall bodies with actual editable wall assemblies while preserving room dimensions, floors, ceilings and decorative features. Check collision and lighting, plus wall edit/save/reopen behavior.
4. Update consumers, relocate recipe fixtures and remove the source tooling after the standalone project is validated.

Static/archive checks can run without building. Interactive acceptance needs a user-built Creator/client: character selection, dungeon entry, grid movement through every corridor, combat, torch lighting, Alden recruitment/healing, Bone Key acquisition and exit passage. Verify saving/reopening and continued editing with the Wall tool. Do not claim gameplay preservation from JSON validation alone.

## Completed Wall tool architecture conversion

Both Stonefall project copies now store all region architecture as Wall assemblies. The main masonry assembly includes 13 formerly recessed wall cells as solid spans. Twenty rectangular room/corridor sections carry 40 fitted floor/ceiling surfaces; full-height unframed openings on their boundary spans preserve open connections. The 592 floor cells and original ceiling heights are unchanged. Decorative ceiling beams, niche recesses and the decorative pillar were removed.

The wall sconce is a standard mounted prefab with authored fixture parts, particle emitter and light. No raw region geometry remains. Entity graphs, instances, screens, input and gameplay configuration are unchanged. Surface generation respects assembly texture scale, and full-height openings at span endpoints no longer generate solid junction posts.

Static JSON checks passed for identical project copies, surface coverage/heights, boundary references, prefab references and unchanged gameplay data. Regression tests were added but not run; no build was performed. Visual appearance, collision and Wall tool edit/save/reopen behavior still need checking in the user-built application.

Follow-up: corrected prefab geometry `group` fields from null to empty strings (GeometryObject requires a string). Both files also include 161 generated box cache objects for walls and surfaces, retaining assembly/span/surface references. Current loaders rebuild these from the editable assemblies; the cache provides geometry to existing loaders. The existing sync_project_items executable was used against a temporary copy without rebuilding.

Restored the 23 vertical ceiling height-transition panels as elevated Wall spans, with matching cached geometry. These were architectural closures, not removable decorative beams. Fixed client named-key release forwarding and ensured an Off action is not mistaken for the default blocked grid action. Extended the grid release regression test to cover translation and strafing as well as rotation; no build or tests run.
