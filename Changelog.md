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
