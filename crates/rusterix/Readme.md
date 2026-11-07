# Rusterix

Rusterix is the core game engine powering [Eldiron](https://github.com/markusmoenig/Eldiron), an open-source creator for classic RPGs.

## Overview

Rusterix provides a complete game engine with a client-server architecture, a software rasterizer, and an integrated scripting VM.

## Key Features

- **Software Rasterizer** — Tile-based and polygon rendering with texture mapping, lighting, and distance fog. Now superseded by [SceneVM](https://crates.io/crates/scenevm) as the GPU-based renderer
- **Client-Server Architecture** — Separate server and client modules communicating via messages for clean separation of game logic and rendering
- **Entity System** — Entities with attributes, pathfinding, and behavior trees driven by a stack-based scripting VM
- **Map System** — Region-based maps with sectors, linedefs, and tile layers
- **Scripting** — Stack-based VM for entity behaviors and game logic, with host-callable functions for interacting with the game world
- **UI Widgets** — Built-in widget system for HUD elements, menus, and in-game UI
- **Procedural appearances** — MZ geometry/rig/motion evaluation for directional
  2D sprites and native SceneVM 3D meshes, with ruleset clothing, grips, item
  states and attached effects. `procedural_avatar` owns appearance playback;
  `ruleset_material` resolves palette-bound surface recipes. Screens and widgets
  consume the resulting live tile artwork, including alpha and animation.

See [Appearance](../../docs/docs/characters_items/appearance.md) and
[Procedural Materials](../../docs/docs/building_maps/procedural_materials.md).
The material core is embedded in the workspace; materials bake to textures/pages
rather than executing a live per-pixel material VM.

## License

Licensed under the MIT License.
