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
- **Shader Language** — Includes [Rusteria](https://crates.io/crates/rusteria), a procedural shader language for texture and material generation

## License

Licensed under the MIT License.
