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

### Documentation

- Added Organic Tool documentation covering the new brush-based workflow, dock layout, toolbar controls, and 3D brush preview behavior.
- Added configuration docs for the new renderer style and stylized post-processing settings, including their runtime `world.post.*` / `region.post.*` override fields.

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
- Changed `take_damage` to receive final incoming damage after rules are applied, while the server commits damage automatically after the event returns.
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
- Updated scripting and event docs to reflect the new `take_damage` behavior, damage kinds, and `source_item_id`.

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
