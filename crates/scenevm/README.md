# SceneVM

SceneVM is a GPU-based rendering engine built on [wgpu](https://wgpu.rs/), powering the visual pipeline of [Eldiron](https://github.com/markusmoenig/Eldiron). It supersedes the earlier software rasterizer in [Rusterix](https://crates.io/crates/rusterix).

## Overview

SceneVM provides a layer-based renderer with configurable compute shaders that translates 2D and 3D geometry into GPU render passes. It handles texture atlasing, lighting, camera management, and dynamic objects, all driven by WGSL shaders.

## Key Features

- **wgpu Rendering** — Cross-platform GPU rendering via wgpu, supporting Metal, Vulkan, DX12, and WebGPU
- **2D and 3D Geometry** — Poly2D and Poly3D primitives with texture mapping and material support
- **Texture Atlas** — Automatic atlas packing and GPU-side atlas tables for efficient batched rendering
- **Lighting** — Point, directional, and ambient lights with GPU-computed shading
- **Camera System** — Perspective and orthographic cameras with first-person and top-down modes
- **Dynamic Objects** — Animated and interactive objects with repeat modes and blending
- **BVH Acceleration** — Scene-wide bounding volume hierarchy for efficient GPU ray queries
- **UI System** — Optional built-in UI layer (behind the `ui` feature flag)
- **WASM Support** — Runs in the browser via WebGPU

## License

Licensed under the MIT License.
