# Eldiron v0.9.13

## Improvements

### Creator

- Added the first official Eldiron Ruleset documentation and ruleset direction, covering the move toward unified race, class, item, combat, progression, and visual defaults for v0.91.
- Improved **Edit Geometry** action parameters by grouping metadata and transform values into separate TOML sections for easier editing.
- Improved scripted 3D Geometry Object areas so `set_attr("visible", ...)` updates the backing object visibility, while `set_attr("blocking", ...)` updates object solidity and rebuilds runtime collision/navigation.
- Improved hidden 3D Geometry Object handling so hidden objects are still available to the scene and can be revealed later through script.
- Fixed rotated **Create Pattern** repeats so the pattern fills the selected face in rotated pattern space instead of collapsing into a small central area.
- Fixed **Create Pattern** guide mode and minimap previews on vertical/transformed faces, including clipped tile guide lines on triangular/sloped face boundaries and shared pattern fitting across coplanar subdivided faces.
- Fixed 3D face texture offset and rotation controls so arrow-key nudging and the **Edit Texture** action follow each face's UV winding instead of moving mirrored faces in the wrong direction.
- Reintroduced the **Builder Tool** as an instant click-to-bake workflow that turns Builder Graph box and cylinder primitives into editable 3D Geometry Objects instead of procedural scene-time host data.
- Reworked **Surface Noise** to blend materials in the 3D shader from stable world-space noise instead of adding render tessellation or UV-dependent distortion.

### Game

- Added Messages widget options for press-to-continue overflow pauses, explicit script pauses, timed pauses, input blocking during pauses, and mouse-wheel scrollback.
- Improved mesh collision for direct 3D Geometry Object cutouts so actors can move through wall openings without colliding against stale hidden geometry.
- Improved mesh floor movement on narrow bridges and stepped geometry by sampling reachable floor support across the actor radius instead of relying only on the center point.
- Improved mesh `goto` movement on stairs by trying stair-aware direct floor stepping before falling back to navgrid pathing.
- Improved first-person stair traversal by smoothing the visual camera height while keeping collision and actor position tied to the real floor height.
- Fixed dynamic entity collision so actors on different vertical levels no longer block each other in XZ when their height ranges do not overlap.

### Rules

- Added headless rules interaction tests covering scripted character attacks, `damaged` event payloads, NPC retaliation, and attack cooldown blocking.
- Added headless death and loot regression coverage for lethal attacks, `death` / `kill` event delivery, item drops, and dead-target attack blocking.
- Added headless spell regression coverage for ruleset spell damage, `damaged` payloads, MP costs, spell cooldowns, and healing caps.
- Added negative spell regressions for invalid targets, not enough MP, and lethal spell casts firing `death` / `kill` once.
- Added official ruleset regressions for Warrior defaults/loadout, Cleric spell unlocks, Human/Orc hostility, official weapon damage, and official spell costs/cooldowns.
- Added terminal ruleset validation and summary commands for the bundled official ruleset and resolved project rules overrides.
- Added a terminal character rules inspector that resolves race, class, level, attributes, unlocks, loadout, spell details, and combat rolls from the official ruleset.
- Added a terminal item rules inspector for weapons, armor, and clothing, including damage rolls, attributes, category rules, and visual/avatar metadata.
- Added the first Ranger ruleset slice with bow-based DEX damage, a hunting bow, wooden arrows, ranged range validation, and terminal/rules regressions.
- Added an official training spear and made bow attacks consume matching stackable ammunition from inventory.
- Updated official item visuals with a wooden training sword, palette-driven axe, mace, spear, bow, shield, and arrow masks, and item descriptions for look/text paths.
- Made attack intent range rules-owned so the same attack icon uses melee range for melee weapons and bow range for Rangers, even in projects with old character intent-distance data.
- Fixed 2D directional attack targeting so ranged weapons scan along the chosen direction to weapon range instead of only checking the adjacent tile.
- Fixed hostility resolution for placed race-named character templates, so attacking an Orc instance can use Orc/Human race relations even if the instance has no explicit race override.

---

# Eldiron v0.9.12

## Improvements

### Creator

- Added a Geometry Object minimap path for 3D region editing, drawing a top-down XZ wire projection with selection highlights for objects, faces, edges, vertices, and surface-detail lines.
- Extended 3D Geometry Objects with area metadata so named objects can act as gameplay areas for sector-style script destinations such as `goto`, `teleport`, and `random_walk_in_sector`, spawn object-linked items, and fade with **Hide in Iso**.
- Improved **Create Box** in 3D edge mode so a selected floor edge can attach a wall box to the face below it, with wall thickness taken from the current grid step.
- Added **Cut Profile** for selected 3D Geometry Objects, starting with centered crenellation/battlement cuts across the full object.
- Added **Cut Stairs** for selected 3D Geometry Object faces, deriving a stair profile from one top face and one adjacent side face while keeping the result as a single object.
- Added **Game / Shortcuts** for overriding editor shortcut bindings by stable action ID, with the 3D Object, Vertex, Edge, and Face tools defaulting to `O` / `V` / `E` / `F` while preserving in-tool commands such as vertex fill, edge-loop selection, object rotation, and tile application.
- Restored the surface-detail drawing workflow so switching from a selected face to the Linedef / Edge Tool enters detail mode and lets clicks add surface points on that face.
- Added **Create Pattern** for selected 3D faces, with guide mode for editable surface-line stamps and relief mode for generated raised pattern geometry. Patterns include discs, triangles, quads, lines, regular or interleaved tile grids, irregular rounded cobbles, and alternating comma-separated sequences, with **PATTERN** and **BACKGROUND** HUD material slots for relief generation.
- Improved action parameter editing so current Create Pattern TOML values are preserved across undo/refresh cycles, and added a modular minimap preview overlay hook used by Create Pattern to show the current pattern outline before applying.
- Added **Create Face** for closed 3D surface-detail loops, creating a new selectable coplanar face without cutting through the host object so drawn footprints can be extruded into new geometry.
- Added a rounded profile option to **Create Ridge** and **Create Groove** for softer surface-line details such as vines, roots, cables, and organic wall carving.
- Added arrow-key texture editing for selected textured 3D faces: arrows adjust offset, Shift+Left/Right rotates, and Ctrl/Cmd+arrows scales.
- Fixed surface-detail `L` expansion so selecting one point or segment of a stamped or drawn guide loop can select the whole connected surface-detail component.
- Fixed leaving surface-detail mode so switching back to Face, Vertex, or Object mode returns to normal geometry selection instead of keeping detail-mode picking active.
- Replaced the old Organic paint tool path with a **Surface Noise** action for selected 3D faces; the action exposes a `NOISE` HUD material slot for tile/color assignment and clearing, and noise is stored per face/evaluated from object/world-space coordinates so adjoining noisy faces can stay continuous around corners.

## Bug Fixes

### Game

- Fixed the isometric game camera follow target so climbing stairs or elevated geometry keeps the player centered instead of drifting with height changes.

---

# Eldiron v0.9.11

## Improvements

### Creator

- Improved 3D grid snapping to use power-of-two edit subdivisions from `1` through `1/32`, matching practical mapping increments for precise mesh work.
- Updated the 3D grid HUD and shortcut slots so `1` through `6` select `1`, `1/2`, `1/4`, `1/8`, `1/16`, and `1/32` snap steps.
- Changed the 3D `,` / `.` shortcuts to step through edit snap subdivisions instead of changing the 2D pixel-grid zoom.
- Improved 2D grid-subdivision button and fallback shortcut handling so the same `1` through `6` snap slots are used consistently across Vertex, Linedef, and Sector tools.
- Improved 3D vertex dragging so moved vertices snap onto absolute grid positions instead of preserving their previous off-grid offset.
- Improved 3D vertex/edge auto-merge so it also runs after gizmo-based vertex movement.
- Improved 3D edge splitting so splitting a selected edge on quad geometry performs a connected quad loop-cut instead of leaving larger unsplit faces.
- Improved 3D edge splitting on triangle and odd-polygon faces so selected-edge splits divide the touched face instead of only inserting an extra midpoint into the same polygon.
- Improved 3D Face Subdivide so neighboring faces share the new boundary midpoint vertices, preventing detached T-junctions around subdivided faces.
- Improved 3D cutouts on split faces so surface-detail loops that span multiple coplanar face pieces rebuild the full coplanar surface instead of cutting only the small host quad.
- Improved 3D gizmo and vertex/surface marker sizing so handles scale from camera distance instead of selected object size, keeping handles closer to large objects and less overwhelming on tiny details.
- Improved the 3D HUD coordinate readout to show three decimal places normally, four decimal places at `1/16`, and five decimal places only at the `1/32` grid step.
- Improved the 3D HUD coordinate readout so selected Geometry Object, face, edge, vertex, and surface-detail positions stay locked while hovering or dragging over other 3D objects.
- Improved 3D Object Tool multi-selection movement so dragging a selected object or its move gizmo moves the selected objects together.
- Improved 3D geometry dragging on small grid steps so free vertex, edge, face, and object movement uses a stable drag plane, and gizmo movement can skip across multiple snap points to keep up with the cursor.
- Improved 3D Object Tool rotation so `R` rotates selected Geometry Objects around Y, while `Shift+R` rotates around Z for standing objects on end.
- Added a 3D-only HUD length readout for selected Geometry Object edges, selected surface-detail segments, and the active surface-line drawing segment.
- Added 3D rectangle selection for Geometry Objects, faces, edges, and vertices, with Shift adding to the selection and Alt/Option removing from it.
- Improved 3D character and item placement so entity drops and moves can choose and render on the floor below overhead Geometry Object roofs instead of snapping onto the highest overlapping surface.
- Improved mesh character movement so tiny seams between adjacent walkable Geometry Object floors do not block traversal in either direction.
- Improved 2D/isometric movement input so simultaneous cardinal keys stay cardinal-only, using the most recently pressed direction instead of emitting diagonal movement actions.
- Improved 3D Edit Face Texture selection handling so explicit face selections edit only those faces, while object selections still edit all faces when no individual faces are selected.
- Improved 3D Edit Face Texture live updates so UV offset, scale, and rotation changes refresh the editor scene immediately without needing to start or stop the game server.
- Improved 3D face painting so applying or clearing tile and palette sources respects explicit face selections before falling back to whole-object application.
- Extended Edit Vertex to single selected 3D Geometry Object vertices, allowing exact world-coordinate edits for precise vertex placement.
- Added editor-only 3D preview actions for toggling post-processing and lighting while editing without changing project render settings.

## Bug Fixes

### Creator

- Fixed object-mode `R` / `Shift+R` rotation being intercepted by the Rect Tool shortcut before selected Geometry Objects could rotate.
- Fixed the grid-subdivision HUD buttons painting over the bottom HUD separator line.
- Fixed duplicated Geometry Object selections so duplication is undoable and leaves only the duplicated objects selected at object level, giving multi-object duplicates one group-move selection instead of stale sub-object selections.

### Renderer

- Fixed transparent water/glass volumes hiding opaque geometry inside or behind them, such as bridge elements disappearing inside water areas.

---

# Eldiron v0.9.10

## Improvements

### Creator

- Changed the Game Tool shortcut from `A` to `K` so right-mouse 3D camera movement no longer conflicts with `WASD` navigation.
- Improved right-mouse 3D camera movement by grabbing the cursor and feeding raw mouse-motion deltas into the editing cameras while dragging.
- Extended the same captured right-mouse camera movement path to the Xcode / macOS FFI build.
- Improved direct 3D Create Box behavior so newly-created boxes switch to the Object Tool immediately and remain selected for object-level editing.
- Improved 3D tool switching so stale object, face, edge, vertex, and surface-detail selections no longer keep driving shortcuts after changing tools.
- Improved 3D tool switching so selected objects convert into all faces, edges, or vertices when switching to the Face, Edge, or Vertex tools, and selected faces convert into editable vertices when switching to Vertex.
- Improved 3D Face Fill so selected vertices are ordered around their plane before creating the face, avoiding twisted faces from unlucky selection order.
- Improved 3D Edge Tool feedback so newly-created split vertices are visible immediately after splitting selected edges.
- Improved 3D vertex `X` splitting so two selected non-neighboring vertices on one face split that face along the diagonal instead of leaving an invalid polygon.
- Added 3D vertex/edge `M` merging so selected geometry vertices or edges collapse to their center and affected faces are rebuilt.
- Added 3D vertex/edge auto-merge while dragging, so moved vertices collapse into existing vertices when they land on the same grid position.
- Improved 3D topology cleanup after edge split, merge, fill, drag, and vertex delete operations by dropping degenerate faces, triangulating concave/non-planar faces, and refreshing affected UVs.
- Improved 3D grid HUD feedback by keeping the compact `1` ... `0` shortcut slots and showing the active snap step beside them.
- Improved the 3D grid overlay so visible subdivision lines now match the active `1` ... `0` snap step used by geometry movement, resizing, extrusion, duplication, and surface-detail editing.
- Improved 3D Face Subdivide so all newly-created child faces stay selected, making repeated subdivision work immediately.
- Shortened the 3D object-selection status text so the footer focuses on hidden HUD mode shortcuts instead of repeating visible action shortcuts.
- Fixed the 3D `,` / `.` grid-size shortcuts so they redraw immediately and stay clamped to valid grid sizes.
- Fixed camera shortcuts such as `Cmd/Ctrl+3` so they no longer also trigger the plain grid snap shortcuts.
- Improved the 3D HUD coordinate readout so selected geometry continues to show a useful position even when the cursor is no longer hovering the object.
- Improved the 3D HUD coordinate readout so it falls back to the ground plane when no object is hovered.
- Improved 3D object gizmo sizing so handles stay closer to large selected objects instead of scaling far off-screen.
- Added object-mode `R` / `Shift+R` rotation for 90-degree vertical-axis turns on selected Geometry Objects.
- Added object-wide `T` tile/color application for selected Geometry Objects, matching the face-level material shortcut while applying the current tile source to every face.
- Added per-face 3D texture placement controls for selected faces or selected Geometry Objects, including UV offset, scale, and rotation.
- Improved 3D face texture editing so offset, scale, and rotation parameter changes update the viewport immediately.
- Added direct Geometry Object visibility, solidity, and group-label properties as the foundation for helper geometry, trigger/water volumes, and grouped level pieces.
- Improved mesh collision for direct 3D geometry so walkable faces sample their actual face plane instead of an averaged height, with more robust floor lookup around chunk boundaries.
- Removed legacy terrain/procedural parameter groups from Edit Vertex, Edit Linedef, and Edit Sector; these 2D actions now expose only current map-edit metadata.

## Bug Fixes

### Creator

- Fixed the 3D HUD position readout so the displayed `Y` value uses the actual hovered surface height instead of snapping to the edit grid, avoiding flicker between floor levels.
- Fixed mesh collision movement so actors can step onto low raised Geometry Object floor edges within the allowed step height instead of being blocked by the cap's vertical side.

### Server

- Fixed character lifecycle ordering so the `startup` event is sent before the initial `entered` event for loaded and spawned characters.

---

# Eldiron v0.9.9

## New Features

### Creator

- Added the new direct 3D geometry editing path for Regions, centered on editable **Geometry Objects** instead of the older generated/procedural 3D authoring workflow.
- New projects and newly-created Regions now start with a centered starter box so 3D editing has an immediate object to select, resize, subdivide, paint, and cut.
- Added 3D object, face, edge, and vertex editing through the existing tool vocabulary:
  - **Object Tool** selects and moves whole Geometry Objects.
  - **Sector / Face Tool** selects and edits geometry faces.
  - **Linedef / Edge Tool** selects geometry edges and draws face-local surface lines.
  - **Vertex Tool** selects and edits geometry vertices.
- Added direct 3D face actions for creating and editing blockout geometry, including Create Box, Face Extrude, Face Subdivide, Face Inset, Face Merge, Face Delete, and Cut Opening.
- Added face-local surface detail drawing with persistent point/segment data, including line previews, Escape-to-finish behavior, cube-style point handles, selectable segments, and closed-loop support.
- Added **Create Ridge** and **Create Groove** actions that convert selected surface lines into persistent raised or recessed geometry, with box and triangle stroke profiles and inherited host-surface material sources.
- Added polygon cutouts from closed surface-line loops, allowing window/door-style openings through Geometry Objects without relying on hardcoded arch/window actions.
- Added direct 3D tile and color application for Geometry Objects, including object-wide material assignment, face material assignment, Rect Tool face painting, palette-color sources, and support for tile/color/tilegraph/nodegraph sources through the same tile-source path.
- Added 3D Rect Tool hover previews and live painting on geometry faces, with nudged render geometry to avoid z-fighting against the host surface.
- Added 3D editing HUD updates for the current selection, including localized status text that shows the available object, face, edge, vertex, and surface-detail shortcuts.
- Added dedicated 3D HUD slots for object **MOVE / SIZE** modes instead of reusing tile/material icon slots.
- Added 3D camera shortcut and control cleanup for Iso, Orbit, and FirstP editing views, including arrow-key target panning in 3D editing cameras.
- Added FirstP fly navigation for the Creator: `Space` toggles fly mode, pointer position controls looking/turning, `WASD` moves through the level, and `Escape` exits back to normal editing.
- Added live Rect Tool painting previews in 2D and the new direct 3D path so tile/color strokes appear while dragging instead of only after mouse release.
- Added mesh-collision feeding from direct 3D Geometry Objects, so edited floors, walls, and cutouts participate in 3D movement collision instead of relying on the old procedural 3D path.
<!--- Added the first 2D procedural dungeon builder via the new **Build Procedural** action and `[procedural]` region settings, starting with the `connected_rooms` generator.-->
- Added procedural tile metadata in **Edit Tile Meta**, allowing tiles to be tagged by style, kind, and weight for generated maps.
- Added procedural generation support for entrance/exit marker tiles, weighted item spawns such as doors, and weighted character spawns such as dungeon monsters.
- Added support for endless 2D roguelike-style procedural loops, where scripts can scale `region.procedural.*` settings, rebuild the current dungeon, and place the player at the regenerated entrance.

### Scripting

- Added `world_event(event, value)` and the matching visual scripting **World Event** block, allowing characters, items, and region scripts to delegate orchestration to the World script's `event(event, value)` handler.
- Added `teleport_entity(entity_id, sector, region)` and the matching visual scripting **Teleport Entity** block so World scripts can move a player or NPC passed in through a world event.
- Added `build_procedural(seed)` and the matching visual scripting **Build Procedural** block so World scripts can regenerate 2D `connected_rooms` procedural regions during play.
- Added script access to live region procedural settings through context variables such as `region.procedural.room_count` and `region.procedural.characters.skeleton.percentage`, enabling roguelike difficulty scaling before regeneration.

## Optimizations

### Creator

- Improved direct 3D editor responsiveness by coalescing redundant hover/drag events, processing only the newest queued geometry drag, and throttling expensive overlay refreshes.
- Improved Rect Tool responsiveness by using dirty chunk updates and avoiding full geometry overlay rebuilds during common paint drags.
- Improved tile-paint commits so tile-only strokes dirty affected scene chunks instead of forcing a full scene-manager rebuild.

### Client

- Improved wgpu client UI sharpness by switching the RGBA overlay sampler to nearest filtering.

## Documentation

- Rewrote the 3D editing documentation around the new direct Geometry Object workflow, including object, face, edge, vertex, surface-line, ridge/groove, cutout, Rect painting, camera, and shortcut behavior.
- Renamed the 3D-facing tool documentation to match the new shared terminology, including **Sector / Face Tool** and **Linedef / Edge Tool**.
- Added documentation for the new 2D procedural dungeon workflow, including **Build Procedural**, `[procedural]` region settings, procedural tile metadata, door/item generation, character generation, and regeneration behavior.

## Bug Fixes

### Creator

- Fixed Rect Tool previews so they remain visible when editing geometry is hidden, without forcing the rest of the editing geometry overlay back on.
- Fixed the direct 3D Rect Tool paint preview so the preview rectangle follows drag painting again while still using throttled overlay updates.
- Fixed 3D Rect Tool side-face painting so painted tiles are visible without z-fighting against the selected Geometry Object face.
- Fixed face editing so moving and resizing faces preserves visible UV/checker material feedback after the edit commits.
- Fixed direct 3D selection transitions so switching into face editing with no selected face no longer leaves a stale broken gizmo/object-move state.
- Fixed cutout generation so closed surface-line loops create real through-openings instead of capped recesses.
- Fixed renamed/older 3D starter projects with Rect-painted geometry faces failing to load by accepting both legacy face tile-cell maps and the new vectorized tuple-key format.
- Fixed 3D entity and item editor previews so dragged or placed instances snap to edited geometry floor height instead of flickering between world zero and the floor.

---

# Eldiron v0.9.8

## New Features

### Creator

- Restored Creator self-update support for Windows and Linux release builds, matching the GitHub release asset names generated by the release workflow.
- Added macOS update detection that shows the same `Update to v...` button when a newer GitHub release exists, opening the Eldiron releases page instead of trying to replace the signed app bundle in place.

### Client

- Added `--render-debug` timing output for the wgpu client and SceneVM renderer to diagnose adapter/backend selection, frame preparation, draw, overlay, and raster-stage timings.

## Bug Fixes

### Creator

- Fixed self-update release detection so newer versions are compared in the correct direction and the latest release is selected reliably.

### Renderer

- Fixed severe SceneVM Raster 3D CPU spikes in dense scenes with dynamic billboards and particles by caching static geometry separately and updating only dynamic tails instead of rebuilding the full mesh every frame.
- Fixed SceneVM Raster 2D dynamic billboard rendering so static 2D geometry is cached separately and no longer rebuilt every frame when avatars, items, or particles are present.
- Reduced Raster 3D CPU usage by caching static visible / opaque / transparent / particle index splits and reprocessing only dynamic geometry when the camera and static visibility are unchanged.
- Improved 3D hover-picking performance by using cached transformed raster geometry with per-triangle GeoId metadata instead of re-walking and transforming chunk polygons during mouse movement.

---

# Eldiron v0.9.7

## New Features

### Creator

- Added the new brush-based **Organic Tool** workflow for painting surface detail directly onto map surfaces, replacing the older graph-driven organic authoring direction.
- Added a dedicated Organic dock with a live brush preview, visual brush-shape presets, and compact controls for `Base`, `Border`, `Noise`, `Brush Size`, `Border Size`, `Noise Amount`, and `Opacity`.
- Added Organic toolbar controls for `Free / Locked`, `Clear`, and `Active / Deactive`, plus a 3D brush-footprint preview for organic painting in place of the generic hover marker.
- Added renderer style and post-processing controls for a less shiny retro RPG look, including `style = "clean" | "retro" | "grimy"` and post controls for `grit`, `posterize`, `palette_bias`, `shadow_lift`, and `edge_soften`.
- Applied the stylized post-processing controls to Raster 2D as well as Raster 3D, so 2D and 3D views can share the same authored color treatment.

### Server

- Added the `ensure_active` NPC sequence step so scripted routes can enforce stateful interactions such as opening a door only if needed and closing it again afterward.
- Added `hold_speed` for grid-based held movement, so characters can use a fast first-tile `speed` while keeping sustained held movement smooth and continuous.
- Added configurable `[game]` simulation pacing with `simulation_mode = "realtime" | "turn_based" | "hybrid"` and `turn_timeout_ms`, so projects can choose between continuous simulation, fully player-driven turns, or Ultima-style idle turn stepping.
- Added `multiple_choice(entity, prompt, choice_attribute)` for script-defined choice menus using labels authored as character attributes. Selecting an option sends both `{choice_attribute}` and `{choice_attribute}:{index}` back to the offering character.
- Added the new `dialog(entity, node)` system for TOML-authored nested dialogs, including node transitions, continue-style choices, choice events, simple `if` / `unless` conditions, localization/substitution support, and visual scripting support.

### Documentation

- Added Organic Tool documentation covering the new brush-based workflow, dock layout, toolbar controls, and 3D brush preview behavior.
- Added configuration docs for the new renderer style and stylized post-processing settings, including their runtime `world.post.*` / `region.post.*` override fields.
- Clarified the `say(...)` command documentation to show both the default one-argument form and the optional category-color form.
- Moved new `say(...)` presentation documentation from global game config to game widget `[say]` settings.
- Added documentation for script-defined multiple-choice menus and TOML-authored nested dialogs, including localization-key usage and message-widget filtering.

## Bug Fixes

### Creator

- Fixed Organic tool undo / redo and related dock-state restore behavior so brush painting no longer bounces the UI back into the tile picker workflow during restore.
- Improved Organic brush editing so presets are shape-only, thumbnails reuse the current brush colors, and noise is exposed as an actual adjustable paint contribution instead of only a preview accent.
- Fixed project render settings sync so loading a project and editing project settings now apply the same renderer/post/daylight defaults instead of changing the visible sun-shadow state after the first settings edit.

### Renderer

- Softened Raster 3D sun-shadow sampling and increased default shadow biasing in the shader to reduce hard shadow acne seams.
- Fixed two-sided 3D lighting so sun shadows and direct lights use light-facing normals instead of camera-facing normals, preventing surfaces from changing lit state when the iso camera moves.
- Added exact compile-time layout checks for the Raster 3D uniform block to guard against backend-sensitive WGSL/Rust alignment mismatches.

### Server

- Fixed grid-based player movement so the character `speed` attribute now actually affects direct tile stepping, instead of being hardcoded to `1.0`.
- Fixed Eldrin compilation of `say("Text")` so the optional category parameter is truly optional, matching the runtime handler and documentation.

### Client

- Fixed 2D game widgets so `say(...)` speech bubbles render above characters and items like they already do in 3D views.
- Game widgets now prefer their own `[say]` section for speech bubble duration, text colors, and background styling, with legacy global `[say]` config kept as a fallback.
- Messages widgets can now use `handles = [...]` to split regular messages, dialogs, script multiple-choice menus, and inventory offers across different widget placements.
- Messages widgets can optionally draw sender portraits from `portrait_tile_id` with `portrait = true`, `portrait_size`, and `portrait_gap`.
- Fixed bottom-up Messages widget rendering for very long wrapped messages so the newest/lower lines remain visible instead of the message being culled when its first lines overflow above the widget.

---

# Eldiron v0.9.6

## New Features

### Server

- Added the first NPC sequence system for character background workflows, with TOML-authored `behavior.sequences` and step-based `goto`, `use`, and `wait` actions.
- Added sequence runtime script commands `run_sequence(...)`, `pause_sequence()`, `resume_sequence()`, and `cancel_sequence()` so event handlers can coordinate background behavior with reactive NPC logic such as talk, combat, and time-based routines.
- Added timeout- and distance-aware `offer_inventory()` sessions for vendors, driven by the seller `timeout` attribute and `[intent_distance]` setup, with automatic `goodbye` closeout when the buyer leaves range or the session expires.

## Bug Fixes

- Reduced 3D mouse-hover sampling in the Creator and graphical clients to 5 times per second and accelerated static geometry hover picking with `rayon`, improving dense-scene mouse-move performance on projects like `Village3D`.
- Improved the screen Messages widget layout by adding explicit spacing between wrapped message blocks, making consecutive messages easier to distinguish.
- Fixed Messages widget multiple-choice / `offer_inventory()` layout so item names and prices are rendered as separate left/right columns again instead of collapsing into one inline text run.
- Fixed `offer_inventory()` session handling so shop interactions now end with `goodbye` when the buyer times out or moves too far away, instead of allowing stale purchases from an invalid distance.
- Fixed `random_walk()` in 2D so it now uses the same tile-centered pathing behavior as `random_walk_in_sector()`, keeping characters aligned to `.5 / .5` tile centers instead of drifting toward tile boundaries.
- Fixed 2D `random_walk()` / pathfinding hangs by rejecting blocked destination tiles before running unbounded A* searches.
- Fixed 2D `random_walk()` target picking in tight spaces so NPCs prefer nearby walkable tile centers before falling back to continuous random points.
- Fixed 2D blocked-tile movement so actors spawned inside blocking content can step out of their starting tile instead of getting trapped permanently.
- Fixed `goto()` in 2D so low-speed movement no longer stalls from overly aggressive straight-line progress checks while routing around blockers.
- Fixed high-speed 2D `goto()` / path-following so reaching an intermediate path tile no longer incorrectly marks the full destination as arrived.
- Fixed `goto()` so blocking items and entities now stop movement instead of being ignored by the runtime path step.
- Fixed grid-aligned 2D `goto()` so NPCs now stay grid-aligned when blocked by dynamic obstacles such as closed doors.
- Fixed grid `goto()` so temporary blocking by another actor no longer cancels the route permanently and NPCs resume once the blocker moves away.

---

# Eldiron v0.9.5

## New Features

### Creator

- Added tile aliases for single-tile sources via **Edit Tile Meta**, with alias-aware tile lookup across Creator inputs and runtime tile-source usage such as `tile_id` and `set_tile()`.
- Added `Alt/Opt` tile picking to the **Rect Tool**, so clicked 2D/3D rect sources can be sampled from the map and revealed in the tile picker.

### Documentation

- Updated the docs to consistently describe tile sources as supporting UUIDs, tile aliases, and palette indices, including `tile_id` usage, `set_tile()`, and the **Edit Tile Meta** alias workflow.

### Bug Fixes

- Fixed debug-mode Play startup so existing Eldrin `source_debug` scripts are no longer silently regenerated from visual scripts unless the debug source is actually empty.
- Fixed `set_tile()` for entity scripts so player / character tile changes now update through the active server host path and propagate as appearance changes to clients.
- Fixed standalone client palette-index geometry so clients now initialize `rusterix` with the project palette and palette materials before building runtime tiles, matching Creator behavior.
- Fixed 2D movement collision so unnamed internal rect-tool seams no longer behave like blocking walls, while arbitrary sector shapes and real boundary linedefs still block correctly.
- Fixed 2D pathing so tile-based movement targets tile centers instead of tile corners, preventing NPCs from drifting onto `.0` tile boundaries instead of `.5` centers.
- Fixed 2D direct movement so blocked tiles are still respected even when collision comes from tile occupancy rather than linedef walls.
- Fixed `2d_grid` step movement from off-center start positions so forward / backward movement recenters onto the grid instead of falsely treating lateral correction as blocked movement.
- Fixed sector `left` events so leaving a sector no longer routes through the wrong internal event name and is now delivered correctly to Eldrin scripts.
- Fixed Windows mouse-move stutter in dense 3D scenes like `Village3D` by throttling expensive 3D hover-picking in both the Creator and the graphical clients to the game’s 30 FPS cadence instead of recomputing picks on every raw cursor event.

---

# Eldiron v0.9.4

## New Features

### Creator

### Framework

- Added the new `pixels` Winit backend as the default accelerated path, including wasm/Chrome surface-format negotiation and an explicit `softbuffer` fallback feature for platforms or builds that need it.

### Bug Fixes

- Fixed standalone client game-widget mouse release handling so 3D item click intents and world-item drag-and-drop into inventory/equipment slots are no longer dropped on `touch_up()`.

---

# Eldiron v0.9.3

## New Features

### Creator

- Improved the **Remap Tile** action with `remap_all` support, multiple remap modes, and better palette-reduction handling for aggressive color-count changes.
- Added configurable 2D viewport and screen background colors via `background_color_2d` and `screen_background`.
- Improved 2D avatar rendering stability and added `size_2d` for character avatars, allowing 2D-only avatar scaling without affecting the 3D `size` billboard scale.
- Added **World** and **Region** scripting entries to the project tree, plus runtime `world.*` / `region.*` render and post overrides for palette remap, fog, background, and related visual settings.
- Added automatic item drag-and-drop between world, inventory/equipment slots, and terrain in both 2D and 3D game views.
- Added the first expandable party-bound UI layer for screens, including `party` widget bindings, portrait button support via `portrait_tile_id`, and party-aware inventory/equipment slot interaction without breaking existing player-bound screens.
- Added configurable 2D visibility / LOS masking via `visibility_range_2d` and `visibility_alpha_2d`, including runtime `world.render.*` / `region.render.*` control and tile-blocked sight using `MapMini`.
- Added optional click-based 2D intents via `click_intents_2d`, optional 2D terrain auto-walk via `auto_walk_2d`, and optional 2D target highlighting via `target_rect_color`.

### Bug Fixes

- Fixed Entity Tool delete handling so character/item deletion no longer hangs from the keyboard path.
- Fixed tile metadata blocking changes so tile status, live tile caches, and runtime blocking update immediately without requiring a save/restart.
- Inverted 2D editor arrow-key panning on Windows and Linux so the cursor keys move the view in the expected direction, while keeping macOS behavior unchanged.
- Added `Home` / `End` keyboard support to text editing widgets, including line start/end navigation and `Shift` selection in both single-line and multi-line editors.
- Fixed `Extrude Linedef` so it no longer duplicates the base vertices / bottom edge of the source linedef during extrusion.
- Fixed Messages widget word wrapping so wrapped lines no longer intermittently show `...` from the text truncation path.
- Fixed Creator Debug Log auto-opening so general scripting warnings/errors, including runtime setup script failures, are surfaced more reliably instead of only some startup cases.
- Fixed 2D `random_walk_in_sector` target picking so NPCs no longer jitter against sector borders from repeatedly choosing borderline-invalid walk destinations.
- Added the Eldiron Creator version number to the window title bar.

### Documentation

- Added docs for `background_color_2d`, `screen_background`, and the new `size_2d` character attribute.
- Added docs for the new world/region scripting tree items and the `world.render.*`, `region.render.*`, `world.post.*`, and `region.post.*` runtime override paths.
- Added docs for party-bound screen widgets, portrait buttons, and the new `party_index`, `party_role`, and `portrait_tile_id` character attributes.
- Added docs for `click_intents_2d`, `auto_walk_2d`, `target_rect_color`, and 2D intent cursor support on screen widgets.

---

# Eldiron v0.9.2

## New Features

### Server

- Added a new high-level `attack()` combat command that uses progression-based base damage, weapon damage kinds, and the shared combat rules pipeline.
- Expanded combat rules to support both `outgoing_damage` and `incoming_damage`, allowing attacks to be shaped before and after defense.
- Added a new progression rules layer with configurable `level` / `experience` attribute names from Game Settings.
- Added `progression.damage` so combat base damage can be derived from progression formulas instead of being hardcoded in scripts.
- Added `gain_xp(...)`, automatic kill XP via `progression.xp.kill`, and `level_up` events when XP thresholds are crossed.
- Added rules-driven progression messages for XP gain and leveling up, localized through the locale system.
- Added `race` and `class` character metadata, including rule-layer support through `races.<race>` and `classes.<class>` for progression and combat formulas.

### Creator

- Added the first release of the new text-based **Builder Graph** workflow, including a script editor with live preview and reusable Builder assets.
- Added Builder presets such as **Table**, **Wall Torch**, **Wall Lantern**, and **Campfire**, plus improved Builder picker previews and undo-aware deletion from the Builder board.
- Added stable 3D wall and floor point placement for vertex-hosted Builder props, including inside/outside wall placement for wall-mounted assets like torches and lanterns.
- Reworked the Tile Node Graph FX workflow with explicit **Particle Spawn**, **Particle Motion**, **Particle Render**, and **Light Emitter** nodes, plus a one-click **Particle Template** starter setup.
- Added live particle minimap previews, particle/light tile thumbnails, ramp-based particle coloring, optional **Flame Base**, and runtime propagation of Tile Graph particle updates into the 3D editor preview.
- Exposed `attack` and `gain_xp` in CodeGridFX.
- Added config-aware `PLAYER.LEVEL`, `PLAYER.EXP`, and `PLAYER.EXPERIENCE` support for text widgets and localized text.
- Added config-aware `PLAYER.FUNDS` support and a configurable `stats` text command driven from **Game / Authoring**.
- Added tab-stop support for text widgets via `\\t` and `tab_width`, making aligned HUD/status layouts easier to build.
- Added a persistent Authoring mode and dock for entering room, connection, entity, and item metadata as TOML, including starter templates and undo / redo support.
- Added a new **Palette Tool** mode with its own **Palette** dock, dock-only palette board editing, `Apply Color`, minimap color picking, and per-entry material properties for roughness, metallic, opacity, and emissive.
- Added **Game / Authoring** for global startup text, exit presentation, terminal behavior, and text-style color configuration.
- Added embedded text-style play directly inside the Creator Game Tool, including colored transcript output, command input, inventory display, and shared text-session behavior with the terminal client.
- Added a starter project browser for new projects, driven by a repo manifest and offering built-in 2D and 3D starter templates.
- Creator now persists and restores the region camera mode used for editing, so saved 3D projects reopen in their 3D view.
- Added a new **Filter Geometry** editor action with `All` / `Dungeon` modes plus `Dungeon No Ceiling`, making it easier to texture Dungeon Tool geometry in crowded 3D regions by hiding terrain, non-dungeon generated features, and unrelated 3D editing overlays.
- Added `GEOM / DETAIL` modes to the 3D editor HUD, separating world-geometry editing from direct surface-profile editing.
- `Vertex`, `Linedef`, and `Sector` now support direct 3D detail editing on clicked wall, floor, and ceiling surfaces without switching into a separate editing-surface mode.
- Removed the old `Set Editing Surface` workflow in favor of staying in the normal 3D view while editing profiles.
- Added **Build Room** for generating room floor, wall, ceiling, and optional front-lip geometry from a selected wall sector.
- Moved `Apply Tile` / `Clear Tile` from the action list into the **Tile Picker** dock as dedicated buttons.
- Added action-aware tile assignment in Region geometry, so Tile Picker apply/clear can target HUD material slots exposed by actions such as **Build Room**.
- Improved `TheTextView` and `TheListItem` with richer text/styling support used by the new text-play and starter-browser UI.

### Documentation

- Added Builder Tool documentation for the new `.buildergraph` workflow, including a full **Wall Torch** example and host-target guidance.
- Added Palette Tool documentation covering Palette mode, the Palette dock workflow, minimap color picking, and palette material assignment.
- Updated the Tile Node Graph docs to cover the new particle/light authoring workflow, particle template setup, output connections, and **Flame Base**.
- Expanded the Rules docs with progression, leveling flow, automatic kill XP, and progression message examples.
- Updated command, event, widget, and localization docs to cover `attack()`, `gain_xp()`, `level_up`, XP/level placeholders, and text-widget tab alignment.
- Added new documentation pages for **Features**, **Eldiron Architecture**, and **Authoring**, plus a stronger top-level **Getting Started** guide.

### Text-Style Play

- Added a first text-style terminal client that can load and play `.eldiron` projects without graphical assets.
- Added authored room titles, descriptions, exits, entities, items, and corpse presentation to the terminal client.
- Added terminal command aliases, realtime ticking, text exit navigation, and configured player-intent commands like `attack orc`.
- Added text targeting by character `name`, `race`, and `class`, plus a configurable `stats` command for inspecting player attributes in text gameplay.
- Added terminal presentation controls through **Game / Authoring**, including exit styles, message colors, character color rules, and auto-attack behavior.
- Added shared text session/gameplay logic used by both the terminal client and the embedded Creator text-play mode.
- Added text inventory display, nearby-attacker handling, corpse/item room fallback near sector boundaries, and drop/fall cues for newly dropped items.

---

# Eldiron v0.9.1

## New Features

### Creator

- Added a new **Game / Rules** TOML document with its own project tree item, data editor, and undo / redo integration.
- Added a new **Game / Locales** TOML document for project localization, separated from game settings.
- Added a new **Game / Audio FX** TOML document for generated sound effects, including in-editor preview via the `Play` button.
- Extended the `Game` tree so **Settings**, **Rules**, **Locales**, and **Audio FX** are distinct TOML entries, while **Debug Log** and **Console** remain separate runtime/output items.
- Added a new **Game / Console** dock for inspecting and navigating live runtime state, including root, character, and item focus plus interactive console commands.
- Improved CodeGridFX debugging with live execution highlighting for event headers, cells, and rows.
- Added persistent last result / error feedback in CodeGridFX so one-shot calls remain visible after execution.
- Added `if` condition feedback in CodeGridFX, including `True` / `False` values and a muted not-taken highlight.
- Added inline variable value mirroring in CodeGridFX so assignment targets show their current value after execution.
- Added hover help for function calls directly inside the visual scripting graph.
- Added drag-copy for existing CodeGridFX cells, including dependency subtree copying, validation, and visual drop feedback.
- Improved the CodeGridFX editing field styling by restoring framed text entry widgets.
- Added 3D editor billboard previews for region entities and items, so placed instances are visible in 3D even without full runtime geometry.
- Improved 3D Sector tool selection so sectors without direct geometry can still be selected from the world hit position.

### Server

- Added project-wide combat rules with per-kind overrides for incoming damage.
- Added the `damaged` event for ruleset damage reactions. It receives final incoming damage after rules are applied, while the server commits damage automatically after the event returns.
- Integrated damage kinds across combat, including physical, spell, and custom kinds such as fire.
- Added automatic combat messages driven by rules and localized through the new locale system.
- Added automatic combat audio driven by rules, including per-kind overrides and support for generated Audio FX.
- Added per-weapon and per-spell audio overrides via item attributes such as `attack_fx` and `hit_fx`.
- Added configurable combat message categories in rules.
- Added `locale = "auto"` support to resolve the active language from the system locale.
- Added support for localized custom `message(...)` strings with named parameters and shortcut resolvers like `self.*`, `attacker.*`, and `target.*`.
- Added world time resolver keys for text and messages: `WORLD.HOUR`, `WORLD.MINUTE`, `WORLD.TIME`, `WORLD.TIME_12`, and `WORLD.TIME_24`.
- Hid legacy `took_damage` from the normal visual scripting surface while keeping runtime compatibility for older content.

### Audio

- Added generated built-in Audio FX definitions such as `attack`, `hit`, `door_open`, `fire_cast`, and other reusable small effects.
- Made generated Audio FX available through the normal audio playback path, so `play_audio(...)` works for both assets and generated effects.
- Added rules-driven combat audio that can trigger generated effects like `attack` and `hit` automatically.

### Documentation

- Added dedicated documentation for **Rules** and **Localization**.
- Updated the visual scripting docs to cover realtime debugging, persistent values, condition feedback, hover help, and drag-copy.
- Updated audio docs to cover `Game / Audio FX`, combat audio integration, and Audio FX parameters.
- Updated scripting and event docs to reflect the new `damaged` event, damage kinds, and `source_item_id`.

---

# Eldiron v0.9.0

## New Features

### Creator

- New **Avatar** based character *skinning system*.
- New *Duplicate* tool for vertices, lines, sectors.
- New paint tools: *Selection, Paint Bucket, Eraser, Picker*.
- New 3D and 2D renderer using rasterization for faster performance.
- Better terrain creation tools.
- Switched action parameters to be TOML based.
- Procedural geometry actions: *Create Fence*, *Create Palisade, Create Stairs, Create Campfire*.
- First procedural geometry props using the *Create Props* action.
- Terrain and avatars are now drawn in both 3D and 2D.
- Added audio support to Eldion (WAV + OGG) and added the corresponding host scripting commands ("play_audio", "clear_audio", set_audio_bus_volume").
- Support for spells!
- Added patrol (walks along linedefs).
- Eldiron Creator now supports multiple projects.
- New `say()` script command which shows messages above the emitting entity.
- New `time` event which reports full in-game hours.

---

# Eldiron v0.8.100

## New Features

### Creator

- New terrain system:
  - Region settings turn terrain on / off and set a default tile_id.
  - Vertices control height and terrain smoothness and can be associated with a billboard tile (of configurable size).
  - Sectors can either exclude (cut out) terrain (for houses etc) or create ridges of varying height, width and steepness.
  - Rect tool now paints on terrain.

- "Edit Sector" action can now apply tags to sectors.
- Geometry can now be made visible / invisible without rebuilding the BVH. This lays the foundation to be able to hide roofs and other geometry in-game on the fly.
- 'Shift' + Click in the vertex tool now adds vertices in both 2D and 3D.
- Rectangle selection mode now works with all 3D cameras (previously only worked in 2D).
- New 3D Gizmo for moving vertices / linedefs and sectors along the current plane in 3D camera modes (previously only worked in 2D).
- "Automatic" mode for Actions. Apply actions automatically when clicked (or a parameter changes).
- New Azimuth / Elevation settings for the Isometric camera.
- Palette now has its own entry in the project root
  - **Clear Palette** action
  - **Import Palette...** action
  - **Remap Tile** action (Remaps a tile to the current palette)
- Pressing Cmd / Ctrl during dragging of **sectors** moves all embedded sectors with the sector. Useful for moving complete houses etc (in 2D).
- Sector creation with the linedef tool: By default manual mode is used which only closes polygons based on the click history. When Cmd / Ctrl is pressed auto mode is used which tries to close polygons automatically (also allows closing of old polygons). Auto mode however fails when operating in a grid (Rect tool).
- New **Debug Log** project tree item. Shows debug output from the server when running.
- New **Entity Tool** to move and delete entity and item instances on the map.
- New **Game Input** switch to send keyboard events directly to the running game from within the editor.
- New **Apply Tile** action now has a repate mode setting (repeat / scale).

- **Items** can now be associated to Door / Gates (i.e. profile cutouts), controlling blocking states, visibility and more. The item name can be entered in the sector settings.
- Door / Gate billboards can now be animated when visibility changes, with scrolling up /down, left / right or fading in / out.

- Switched from Python to my own scripting language: **Eldrin**. Old projects need to be tweaked to work again.

### Renderer

- Support for **vertex-weighted texture blending**.
- Fixed a bug where transparent billboards would prevent proper ambient occlusion / shadow tracing.

### Client

---

# Eldiron Creator v0.8.90

## New Features

### Creator

- New UI concept: A project tree view now drives the content shown in the dock window (instead of Tools). This also drives a new Undo / Redo system, each dock window has its own undo / redo stack.
- Maximizing the tile picker now reveals the new tile editor. Edit tiles directly in Eldiron and see them update the region in realtime.
- The action list has been expanded and is now central to nearly all functionality inside Eldiron. A lot of new actions were added covering all aspects of Eldiron.
- The Rect tool now works in 3D, draw tiles directly on surfaces.
- New 3D editing actions include creation of gates and doors within surface holes, material settings for tiles and more.
- Now localization system and initial Chinese, Taiwanese, Spanish and German translations for the Creator.
- Added a minimal starter project for new projects.

### Client

- Switched from software based rendering to GPU based rendering for 2D and 3D. 3D is utilizing PBR shading and ray tracing.
- A new collision system for extruded surfaces allowing opening / closing passages.

## Bug Fixes

- A DPI setting > 1.0 could crash Windows and Linux machines.
- In 2D mode drawing blocking tiles would not make them blocking.
- And many others ...

---

# Eldiron Creator v0.8.80

## New Features

### Creator

- New visual real time shading language for materials and more.
- New 3D editing functionality. 3D views are now integrated into the editing workflow.
- New "Action" system. Apply actions based on geometry and UI selections.
- New default 256 color palette.
- The project format changed a bit. If your project does not show tiles anymore, load it into a text editor and replace "floor_source" with "source".

### CI

- Build clients for all platforms at release.

## Bug Fixes

- Correctly refresh screen and tilemap lists after loading a new project.

---

# Eldiron Creator v0.8.70

## New Features

### Creator

- Character and item classes can now be renamed via their context menus.
- New grid based node editor for coding entity and item behavior in the code tool. **The main new feature for this release.**

### Server

- Removed `get_entity_attr` and `get_item_attr` and replaced it with `get_attr_of` which works for both entities and items.

## Bug Fixes

- Fixed deletion of character and item instances in the map (the map would not immediately update sometimes).

---

# Eldiron Creator v0.8.60

## New Features

### Creator

- The `Data Tool` now supports direct sector selections in the map. Making it easier to select and edit widgets who are mostly data driven.
- Button widgets have new capabilities
  - **active** - Boolean, switch if the widget is active by default
  - **show** - String array of widgets to show when clicked
  - **hide** - String array of widgets to hide when clicked
  - **deactivate** - String array of button widgets to deactivate when clicked
- New `inventory_index` attribute for button widgets to display the inventory item at the given index.
- Intent based actions now also work on items in the inventory (when an intent is active and an inventory button is clicked).
- Material node graphs can now be created for screen widgets, allowing procedural borders and content for screen widgets.

### Client

- Messages widgets now support some new config strings: `multiple_choice` the color for multiple choice items (like inventory sales) and `column_width` to define the maximum size of item columns for multiple choice items.
- New localisation and text formatting system, the server may now generate strings like **"{you_bought} {I:{}.name, article=indef, case=lower}"** which gets automatically resolved by the client. Characters and items also can send strings like this now, allowing for powerful in-game text formatting and localization.

### Server

- New `drop` function to drop a specific item with the given id.
- Refactored some code to make sure all actions / intent are executed correctly on items on the map **and** on items in inventories.
- **Major refactoring of the server instancing code. Removes the ref_thread_local dependency and enables rayon parallelism, which in turn finally enables web deployment**.
- New `wealth` attribute for entities which defines the initial characters wealth in the base currency.
- New multiple choice system, implemented right now for inventory sales which vendors can initiate via the new `offer_inventory` command after receiving an intent. `offer_inventory` takes two arguments, the target entity id and a filter string, if empty, i.e. "", all items are offered.
- block_events() now supports specific intents via "intent: attack", this allows for blocking specific intents for a given amount of time. Previously it was only possible to block all intents via "intent".

## Bug Fixes

- Rect tool content was not shown in the screen editor.
- Items in an inventory had a bug during creation which prevented them to be used with events later on.

---

# Eldiron Creator v0.8.50

## New Features

### Server

- New 'entered' and 'left' entity system events when entering / leaving named sectors.
- New 'teleport' command which either takes one argument (destination sector name in the same region) or two, the destination sector name and the name of the destination region (which will transfer the character to a different region). The entity will be transferred to the center of the destination sector.
- New `[light]` data attributes to enable light emission for entities and items.
- New 'set_emit_light(True / False)` cmd to enable / disable light emission for an entity or an item.
- New special `active` item attribute which specifies if the item is active or not. On startup (or if the attribute is changed) a new `active` event is send to the item which can than decide what to do based on the value, like enabling / disabling light emission for a torch.
- New `intent` system. Define the current player intent (like "talk", "use", "open" etc.) via the new `intent` parameter for button widgets. Server will send new `intent` events to both entities and items for direction based and click based item interations.
- New `health` config attribute which holds the name of the default entity health attribute, by default `HP`.
- New `mode` entity attribute which holds the current state string of the entity. Set to `active` on entity instantiation and `dead` when the health attribute is <= 0.
- New `death` event send to an entity when the health attribute is <= 0.
- New `id` command which returns the id of the current entity.
- New `took_damage` command (my_id, from_id, damage_amount). This command sends out messages and checks for death.
- New `goto` command (sector name, speed). Makes an NPC go to a sector. Sends `arrived` event on arrival with the name of the sector as value.
- New  `close_in` command (target id, target radius, speed). Makes an NPC close in (in weapon range given by the target radius) of the entity id with the given speed. Once the target is in range a `closed_in` event is send.
- New `kill` event send to the attacker when he kills his target. The value of the event is the id of the dead target.

### Client

- New `intent` command to invoke an intention via key shortcuts (same as actions).

### Creator

- Tileset tool: Preview icons now in the minimap.
- Tilepicker: Icons preview on hover in the minimap.

## Bug Fixes

- Make game widgets honor the global render graph.
- Info viewer did not show item values correctly.
- Changed `Data` tool shortcut from `A` to `D`.
- When adding tiles to the project the background renderer was not updated correctly.
- Adjust Undo / Redo state when switching regions.
