# Todo List for v1

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
