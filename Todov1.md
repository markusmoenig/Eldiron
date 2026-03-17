# Todo List for v1

## Text Adventure Support

This should not be treated as a separate niche mode, but as a second presentation layer over the same world data. Sector, linedef, item, and entity descriptions would be useful for full text adventures, but also for normal 2D and 3D games.

The first task here is architectural:

- Make a client trait to hide the wgpu dependencies for clients which do not use a graphics interface (terminal). This is the foundation for any text-only client.

Suggested implementation phases:

### Phase 1: Client Split

- Introduce a client trait / shared interface that is independent of wgpu.
- Move rendering-specific code behind the graphical client implementation.
- Keep input, message handling, world updates, and command dispatch in the shared layer.
- Make sure a terminal / text-only client can be built without pulling in wgpu.

### Phase 2: World Description Data

- Add a text adventure tool switch in the editor.
- When enabled, the dock should show text editing instead of the tile picker where appropriate.
- Sector selections should edit:
  - area title
  - area description
- Linedef selections should edit:
  - connection name
  - connection description
  - optional travel wording / direction hint
- Item and entity placement can stay the same, but items and entities should gain optional descriptive text fields.

### Phase 3: Text Presentation Layer

- Build a text adventure client view using the same project/world data.
- Show the current sector title and description.
- Show exits based on connected linedefs.
- Entities and items in a sector should be auto-added to that room description/presence list.
- Show visible items and entities with their descriptions.
- NPC characters going into or out of a room should be mentioned as interactive text events.
- Reuse the same region/map structure as 2D and 3D modes.

### Phase 4: Commands and Interaction

- Implement command parsing for text adventure based games.
- Text recognition should be based on the existing `ACTION` / `INTENT` system.
- Items, entities, and world interactions already describe which intents they support, so the parser should resolve player text into the same intent/action flow instead of inventing a separate interaction model.
- Start with a deterministic parser:
  - movement
  - look
  - examine
  - take
  - use
  - talk
- AI-assisted parsing can be an optional later layer, not the base implementation.

### Phase 5: Cross-Use in Normal Games

- Make sector and linedef descriptions reusable in normal 2D and 3D games.
- Use them for:
  - journals / logs
  - inspect / look interactions
  - tooltip or story text
  - optional fallback narration

This would give Eldiron a much broader scope without splitting the project into different engines.

## Better and more configurable Screen Widgets

Current screen widgets are aimed at in-game widgets, like buttons and text. But for signup, character setup we need more UI based widgets.

- Text lists
- Icon lists
- Text entry fields
- Drop down lists

Probably we will also need a general DnD system for items (drop spell from spell books into buton widget etc).

Also widgets for in game story-telling:

- Showing images
- Scrolling text

## Timeline Mode

RPGs are all abou telling a story. We need a timeline mode where characer actions are not script based but based on events on a timeline.

These can be played live during games as scriped stories.

## Raytracer

For screenshots and marketing we need a raytrace / pathtrace offline mode in the editor.

CPU or GPU ? GPU faster but CPU easier to tweak.

## Procedural FX for the Paint Tool

Create tiles procedurally

- Explosions
- Flames
- Smog

## Better Tile Picker

Right now tiles are displayed in the order they were added, but can be filtered by category.

On resizes and new tiles, tile structure looses cohersion, tiles which are similar get disconnected and tiles which fit together are disconnected to.

We would need a new widget which:

* Makes tiles sortable via Dnd
* Allows for tile-groups to stay in cohersive even on resize etc.
* Makes it easy for a future Eldiron Treasury system to integrate new tiles and tile-groups easily via a DB connection.
