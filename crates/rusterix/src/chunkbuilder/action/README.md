# Surface Actions System

## Overview

The Surface Actions system provides a **trait-based architecture** for defining how sectors on surfaces should be rendered as 3D meshes. Instead of hardcoding mesh generation logic for each type of surface modification (holes, reliefs, recesses, etc.), surface actions **describe** the mesh geometry in an abstract, declarative way.

## Key Concepts

### What's Different?

**Before (Hardcoded):**
- Mesh generation for holes, reliefs, and recesses was hardcoded into the chunk builder
- Adding new surface effects required modifying complex meshing code
- Each action type had its own mesh generation logic duplicated throughout the codebase

**After (Trait-Based):**
- Surface actions **describe** what mesh should be generated using control points
- One unified mesh builder processes all action types
- Adding new effects is as simple as implementing a trait with descriptive logic
- No knowledge of sectors, chunks, or low-level meshing required

### Core Components

1. **`SurfaceAction` Trait** - Describes WHAT to generate (not HOW)
2. **`SectorMeshDescriptor`** - Specifies caps, sides, holes, and connection modes
3. **`ControlPoint`** - Defines positions in UV space + extrusion along surface normal
4. **`MeshTopology`** - Describes how control points connect (loops, filled regions, quad strips)
5. **`SurfaceMeshBuilder`** - Unified mesh generator that processes descriptors

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     SurfaceAction Trait                      │
│  describe_mesh() -> Returns SectorMeshDescriptor            │
└────────────────────────┬────────────────────────────────────┘
                         │
                         │ Implemented by
                         │
        ┌────────────────┼────────────────┐
        │                │                │
   ┌────▼────┐     ┌─────▼─────┐    ┌────▼────┐
   │  Hole   │     │  Relief   │    │ Recess  │
   │ Action  │     │  Action   │    │ Action  │
   └─────────┘     └───────────┘    └─────────┘
        │                │                │
        └────────────────┼────────────────┘
                         │
                         │ Processed by
                         │
              ┌──────────▼──────────┐
              │ SurfaceMeshBuilder  │
              │  build() -> Mesh    │
              └─────────────────────┘
```

## Control Points

A `ControlPoint` defines a position in space using:
- **UV coordinates** (x, z) - Position on the surface plane in local coordinates
- **Extrusion** (y) - Distance along the surface normal
  - `0.0` = on the front surface
  - `surface_thickness` = on the back surface  
  - Positive = outward (relief)
  - Negative = inward (recess)

```rust
ControlPoint {
    uv: Vec2::new(5.0, 10.0),  // Position on surface
    extrusion: 2.0,             // 2 units outward along normal
}
```

## Connection Modes

How the generated mesh connects to surrounding geometry:

```rust
pub enum ConnectionMode {
    Hard,                              // Sharp edge
    Smooth,                            // Blend normals
    Bevel { segments: u8, radius: f32 }, // Beveled transition
}
```

## Adding a New Surface Action

Let's walk through creating a new terrain-like action step by step.

### Example: Sloped Terrain

Let's create a terrain action that slopes from one edge to another.

```rust
use crate::chunkbuilder::surface_action::*;
use vek::Vec2;

pub struct SlopedTerrainAction {
    pub slope_direction: Vec2<f32>,  // Direction vector
    pub max_height: f32,             // Maximum elevation
}

impl SurfaceAction for SlopedTerrainAction {
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        _surface_thickness: f32,
        _properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if sector_uv.len() < 3 {
            return None;
        }

        // Find the bounding box of the sector
        let mut min = sector_uv[0];
        let mut max = sector_uv[0];
        
        for &uv in sector_uv.iter().skip(1) {
            min.x = min.x.min(uv.x);
            min.y = min.y.min(uv.y);
            max.x = max.x.max(uv.x);
            max.y = max.y.max(uv.y);
        }

        let size = max - min;
        let normalized_direction = self.slope_direction.normalized();

        // Create control points with varying extrusion based on position
        let mut cap_points = Vec::with_capacity(sector_uv.len());
        
        for &uv in sector_uv {
            // Calculate position relative to bounding box (0..1)
            let relative = (uv - min) / size;
            
            // Project onto slope direction to get height factor
            let height_factor = relative.dot(normalized_direction);
            let height_factor = height_factor.clamp(0.0, 1.0);
            
            // Calculate extrusion (height)
            let extrusion = self.max_height * height_factor;
            
            cap_points.push(ControlPoint { uv, extrusion });
        }

        // Create a filled region with smooth blending
        Some(SectorMeshDescriptor {
            is_hole: false,
            cap: Some(MeshTopology::FilledRegion {
                outer: cap_points,
                holes: vec![],
            }),
            sides: None, // Sides handled by smooth connection
            connection: ConnectionMode::Smooth,
        })
    }

    fn name(&self) -> &'static str {
        "SlopedTerrain"
    }
}
```

### Example: Wave Pattern

Here's a more advanced example that creates a wave pattern:

```rust
pub struct WaveAction {
    pub amplitude: f32,
    pub frequency: f32,
    pub direction: Vec2<f32>,
}

impl SurfaceAction for WaveAction {
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        _surface_thickness: f32,
        _properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if sector_uv.len() < 3 {
            return None;
        }

        let normalized_dir = self.direction.normalized();

        // Create wave pattern across the surface
        let cap_points: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| {
                // Project position onto wave direction
                let distance = uv.dot(normalized_dir);
                
                // Calculate wave height
                let wave_height = self.amplitude * 
                    (distance * self.frequency * std::f32::consts::TAU).sin();
                
                ControlPoint {
                    uv,
                    extrusion: wave_height,
                }
            })
            .collect();

        Some(SectorMeshDescriptor {
            is_hole: false,
            cap: Some(MeshTopology::FilledRegion {
                outer: cap_points,
                holes: vec![],
            }),
            sides: None,
            connection: ConnectionMode::Smooth, // Smooth for natural waves
        })
    }

    fn name(&self) -> &'static str {
        "Wave"
    }
}
```

### Example: Stepped Pyramid

Create a multi-level stepped structure:

```rust
pub struct SteppedPyramidAction {
    pub num_levels: u32,
    pub step_height: f32,
}

impl SurfaceAction for SteppedPyramidAction {
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        _surface_thickness: f32,
        _properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if sector_uv.len() < 3 || self.num_levels == 0 {
            return None;
        }

        // Find center of the sector
        let center = sector_uv
            .iter()
            .fold(Vec2::zero(), |acc, &uv| acc + uv) 
            / sector_uv.len() as f32;

        // Find maximum radius
        let max_radius = sector_uv
            .iter()
            .map(|&uv| (uv - center).magnitude())
            .fold(0.0f32, f32::max);

        // Create stepped heights
        let cap_points: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| {
                let distance = (uv - center).magnitude();
                let normalized_dist = distance / max_radius.max(1e-6);
                
                // Calculate which step we're on (outer = 0, center = num_levels)
                let step = ((1.0 - normalized_dist) * self.num_levels as f32).floor();
                let height = step * self.step_height;
                
                ControlPoint {
                    uv,
                    extrusion: height,
                }
            })
            .collect();

        Some(SectorMeshDescriptor {
            is_hole: false,
            cap: Some(MeshTopology::FilledRegion {
                outer: cap_points,
                holes: vec![],
            }),
            sides: None,
            connection: ConnectionMode::Hard, // Hard edges for distinct steps
        })
    }

    fn name(&self) -> &'static str {
        "SteppedPyramid"
    }
}
```

## Usage Pattern

### Step 1: Define Your Action

Implement the `SurfaceAction` trait with your custom logic.

### Step 2: Integrate with LoopOp (Optional)

If you want to use this action through the existing profile system:

```rust
// In surface.rs, extend LoopOp
pub enum LoopOp {
    None,
    Relief { height: f32 },
    Recess { depth: f32 },
    SlopedTerrain { direction: Vec2<f32>, height: f32 }, // New!
}

// Update the get_action() method
impl LoopOp {
    pub fn get_action(&self) -> Option<Box<dyn SurfaceAction>> {
        match self {
            // ... existing cases ...
            LoopOp::SlopedTerrain { direction, height } => {
                Some(Box::new(SlopedTerrainAction {
                    slope_direction: *direction,
                    max_height: *height,
                }))
            }
        }
    }
}
```

### Step 3: Use in Chunk Builder

```rust
// In d3chunkbuilder.rs
let action = loop_op.get_action()?;
let properties = loop_op.to_action_properties(target_side);

let descriptor = action.describe_mesh(
    &sector_uv,
    surface.extrusion.depth,
    &properties,
)?;

let mesh_builder = SurfaceMeshBuilder::new(&surface);
let meshes = mesh_builder.build(&descriptor);

// Process meshes...
```

## Benefits

### 1. **Simplicity**
- New effects require only descriptive logic
- No knowledge of triangulation, winding order, or UV mapping needed
- Focus on WHAT to generate, not HOW

### 2. **Consistency**
- All actions use the same mesh builder
- Uniform UV mapping and normal handling
- Consistent connection behavior

### 3. **Flexibility**
- Easy to add terrain, architectural, or organic effects
- Connection modes allow smooth or hard transitions
- Control points give precise geometric control

### 4. **Maintainability**
- Single source of truth for mesh generation
- Changes to meshing algorithm benefit all actions
- Clear separation of concerns

## Advanced Topics

### Creating Side Geometry

For actions that need explicit side walls (not just smooth blending):

```rust
let base_loop = sector_uv.iter().map(|&uv| ControlPoint {
    uv,
    extrusion: 0.0,
}).collect();

let top_loop = sector_uv.iter().map(|&uv| ControlPoint {
    uv,
    extrusion: height,
}).collect();

sides: Some(MeshTopology::QuadStrip {
    loop_a: base_loop,
    loop_b: top_loop,
})
```

### Working with Holes

To create an action with holes (like a donut):

```rust
cap: Some(MeshTopology::FilledRegion {
    outer: outer_points,
    holes: vec![inner_hole_points],
})
```

### Custom Connection Modes

For beveled edges:

```rust
connection: ConnectionMode::Bevel {
    segments: 4,      // Number of bevel segments
    radius: 0.5,      // Bevel radius
}
```

## Testing Your Action

Create a simple test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sloped_terrain() {
        let action = SlopedTerrainAction {
            slope_direction: Vec2::new(1.0, 0.0),
            max_height: 10.0,
        };

        let sector_uv = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];

        let descriptor = action.describe_mesh(
            &sector_uv,
            0.0,
            &ActionProperties::default(),
        );

        assert!(descriptor.is_some());
        
        let desc = descriptor.unwrap();
        assert!(!desc.is_hole);
        assert!(desc.cap.is_some());
    }
}
```

## Summary

The Surface Actions system transforms mesh generation from imperative (HOW to build) to declarative (WHAT to build). This makes it trivial to add new surface effects for:

- **Terrain**: hills, valleys, riverbeds, mountains
- **Architecture**: columns, arches, moldings, bevels  
- **Organic**: waves, bumps, depressions
- **Procedural**: noise-based, pattern-based effects

All without touching the core meshing infrastructure!
