# Surface Action Properties Guide

This document describes all the properties you can set on sectors to control how holes, reliefs, and recesses are rendered.

## Overview

Properties can be set on two types of sectors:
1. **Host Sector** - The main surface sector
2. **Profile Sector** - Sectors in the profile map that define surface features

Profile sector properties take precedence over host sector properties.

The property system uses **unified names** that work for all action types (holes, reliefs, recesses, etc.).

---

## Core Operation Properties

### `profile_op` (Profile Sector Only)
**Type:** Integer  
**Default:** 0  
**Description:** Defines what operation this profile sector performs.

**Values:**
- `0` = None (Hole/cutout)
- `1` = Relief (raised)
- `2` = Recess (inset)

**Example:**
```rust
profile_sector.properties.set("profile_op", 1); // Relief
```

### `profile_amount` (Profile Sector) **[RECOMMENDED]**
**Type:** Float  
**Default:** 0.0  
**Description:** **Unified property** for height/depth that works with ALL action types. Positive values mean "how much" the action applies (height for reliefs, depth for recesses).

**Example:**
```rust
// Relief - amount is height
profile_sector.properties.set("profile_op", 1);
profile_sector.properties.set("profile_amount", 2.5); // 2.5 units high

// Recess - amount is depth
profile_sector.properties.set("profile_op", 2);
profile_sector.properties.set("profile_amount", 1.0); // 1 unit deep
```

### `profile_height` (Profile Sector) **[DEPRECATED - use profile_amount]**
**Type:** Float  
**Default:** 0.0  
**Description:** Legacy property for relief height. **Use `profile_amount` instead** for new code.

**Backward Compatibility:** Still works for reliefs, but `profile_amount` takes precedence if both are set.

### `profile_depth` (Profile Sector) **[DEPRECATED - use profile_amount]**
**Type:** Float  
**Default:** 0.0  
**Description:** Legacy property for recess depth. **Use `profile_amount` instead** for new code.

**Backward Compatibility:** Still works for recesses, but `profile_amount` takes precedence if both are set.

### `profile_target` (Profile Sector)
**Type:** Integer  
**Default:** 0  
**Description:** Which side of the surface to apply the feature to.

**Values:**
- `0` = Front cap (default)
- `1` = Back cap

**Example:**
```rust
// Create relief on the back side
profile_sector.properties.set("profile_target", 1);
```

### `connection_mode` (Profile Sector)
**Type:** Integer  
**Default:** 0 (Hard)  
**Description:** How the mesh edges connect to the surrounding surface.

**Values:**
- `0` = Hard (sharp edges, no blending)
- `1` = Smooth (blend normals with surrounding surface)
- `2` = Bevel (beveled transition edge)

**Example:**
```rust
// Hard edges (default)
profile_sector.properties.set("connection_mode", 0);

// Smooth blending
profile_sector.properties.set("connection_mode", 1);

// Beveled edges
profile_sector.properties.set("connection_mode", 2);
profile_sector.properties.set("bevel_segments", 4);  // Number of segments
profile_sector.properties.set("bevel_radius", 0.5);  // Bevel radius in world units
```

### `bevel_segments` (Profile Sector, used with connection_mode = 2)
**Type:** Integer  
**Default:** 4  
**Description:** Number of segments in the bevel transition.

**Example:**
```rust
profile_sector.properties.set("bevel_segments", 6);  // Smoother bevel
```

### `bevel_radius` (Profile Sector, used with connection_mode = 2)
**Type:** Float  
**Default:** 0.5  
**Description:** Radius of the bevel in world units.

**Example:**
```rust
profile_sector.properties.set("bevel_radius", 0.25);  // Smaller bevel
```

---

## Unified Material/Texture Properties

### Property Names (Unified for All Actions)

Instead of having separate properties for each action type (like `relief_source`, `recess_source`, etc.), the system now uses **unified names** that work for everything:

- **`cap_source`** - Texture for the top/bottom surface (cap) of ANY feature
- **`jamb_source`** - Texture for the sides/walls of ANY feature
- **`source`** - Generic fallback texture

This means the same property names work for reliefs, recesses, holes, or any future action type!

### Material Lookup Chain

When rendering a feature, the system looks for materials in this order:

#### For Caps (Top/Bottom Surfaces)
1. Profile sector: `cap_source`
2. Profile sector: `source`
3. Host sector: `cap_source`
4. Host sector: `source`

#### For Jambs/Sides (Walls)
1. Profile sector: `jamb_source`
2. Profile sector: `source`
3. Host sector: `jamb_source`
4. Host sector: `side_source` (backward compatibility)
5. Host sector: `source`

### Setting Tile Sources

**Type:** Value::Source(PixelSource)  
**Description:** Defines which tile/texture to use.

**Example - Relief:**
```rust
// Set the cap (top surface) texture
profile_sector.properties.set(
    "cap_source",
    Value::Source(PixelSource::TileId(gold_tile_uuid))
);

// Set the jamb (wall) texture
profile_sector.properties.set(
    "jamb_source",
    Value::Source(PixelSource::TileId(marble_tile_uuid))
);
```

**Example - Recess:**
```rust
// Set the cap (floor of pocket) texture
profile_sector.properties.set(
    "cap_source",
    Value::Source(PixelSource::TileId(wood_tile_uuid))
);

// Set the jamb (walls of pocket) texture
profile_sector.properties.set(
    "jamb_source",
    Value::Source(PixelSource::TileId(brick_tile_uuid))
);
```

**Example - Hole:**
```rust
// Holes don't have caps, only jambs (tube interior)
profile_sector.properties.set(
    "jamb_source",
    Value::Source(PixelSource::TileId(metal_tile_uuid))
);
```

**Example - Global Fallbacks (Host Sector):**
```rust
// Main surface texture
host_sector.properties.set(
    "source",
    Value::Source(PixelSource::TileId(main_tile_uuid))
);

// All feature caps default to this if not specified
host_sector.properties.set(
    "cap_source",
    Value::Source(PixelSource::TileId(cap_tile_uuid))
);

// All feature jambs/sides default to this
host_sector.properties.set(
    "jamb_source",  // or "side_source" for backward compatibility
    Value::Source(PixelSource::TileId(side_tile_uuid))
);
```

---

## Texture Tiling Properties

### Base Surface Tiling

**`tile_mode`** (Host Sector)  
**Type:** Integer  
**Default:** 1  
**Description:** How textures are mapped to the surface.

**Values:**
- `0` = Fit (stretch texture to fit the surface, 0..1 UV)
- `1` = Tile/Repeat (use world-space coordinates with texture_scale)

**`texture_scale_x`** (Host Sector)  
**Type:** Float  
**Default:** 1.0  
**Description:** Horizontal texture scale in world units (when tile_mode = 1).

**`texture_scale_y`** (Host Sector)  
**Type:** Float  
**Default:** 1.0  
**Description:** Vertical texture scale in world units (when tile_mode = 1).

**Example:**
```rust
host_sector.properties.set("tile_mode", 1);           // Tile mode
host_sector.properties.set("texture_scale_x", 2.0);   // Repeat every 2 units
host_sector.properties.set("texture_scale_y", 2.0);
```

### Side/Wall Tiling

**`side_tile_mode`** (Host Sector)  
**Type:** Integer  
**Default:** Inherits from `tile_mode`  
**Description:** How textures are mapped to side walls.

**`side_texture_scale_x`** (Host Sector)  
**Type:** Float  
**Default:** Inherits from `texture_scale_x`  
**Description:** Horizontal texture scale for sides (U = perimeter distance).

**`side_texture_scale_y`** (Host Sector)  
**Type:** Float  
**Default:** Inherits from `texture_scale_y`  
**Description:** Vertical texture scale for sides (V = depth/height).

**Example:**
```rust
// Make walls tile differently than floors
host_sector.properties.set("side_tile_mode", 1);
host_sector.properties.set("side_texture_scale_x", 1.0);
host_sector.properties.set("side_texture_scale_y", 3.0); // Stretch vertically
```

### Per-Feature Jamb Tiling Overrides

You can override jamb texture scaling on a per-feature basis using unified property names:

**`jamb_texture_scale_x`** (Profile Sector)  
**Type:** Float  
**Default:** Inherits from `side_texture_scale_x`  
**Description:** Horizontal texture scale for this feature's walls.

**`jamb_texture_scale_y`** (Profile Sector)  
**Type:** Float  
**Default:** Varies by action type (see below)  
**Description:** Vertical texture scale for this feature's walls.

**Note:** For reliefs, the default inherits from `texture_scale_y` (cap scale) so the texture aligns with the top surface. For recesses and holes, it inherits from `side_texture_scale_y`.

**Example:**
```rust
// Make this specific feature's walls use different tiling
profile_sector.properties.set("jamb_texture_scale_x", 0.5);
profile_sector.properties.set("jamb_texture_scale_y", 2.0);
```

---

## Complete Examples

### Example 1: Simple Relief with Custom Textures

```rust
// Profile sector (the relief shape)
profile_sector.properties.set("profile_op", 1);                    // Relief
profile_sector.properties.set("profile_amount", 1.5);              // 1.5 units high
profile_sector.properties.set("profile_target", 0);                // On front
profile_sector.properties.set(
    "cap_source",                                                  // Top surface
    Value::Source(PixelSource::TileId(gold_tile_uuid))
);
profile_sector.properties.set(
    "jamb_source",                                                 // Side walls
    Value::Source(PixelSource::TileId(marble_tile_uuid))
);

// Host sector (main surface)
host_sector.properties.set("tile_mode", 1);
host_sector.properties.set("texture_scale_x", 2.0);
host_sector.properties.set("texture_scale_y", 2.0);
```

### Example 2: Recess with Unified Properties

```rust
// Profile sector
profile_sector.properties.set("profile_op", 2);                    // Recess
profile_sector.properties.set("profile_amount", 0.5);              // 0.5 units deep
profile_sector.properties.set("profile_target", 0);                // Into front
profile_sector.properties.set(
    "cap_source",                                                  // Floor of recess
    Value::Source(PixelSource::TileId(wood_tile_uuid))
);
profile_sector.properties.set(
    "jamb_source",                                                 // Walls of recess
    Value::Source(PixelSource::TileId(brick_tile_uuid))
);
```

### Example 3: Through-Hole

```rust
// Profile sector
profile_sector.properties.set("profile_op", 0);                    // Hole
profile_sector.properties.set(
    "jamb_source",                                                 // Tube interior
    Value::Source(PixelSource::TileId(metal_tile_uuid))
);

// Texture tiling for the tube
profile_sector.properties.set("jamb_texture_scale_x", 1.0);       // Around perimeter
profile_sector.properties.set("jamb_texture_scale_y", 2.0);       // Along depth
```

### Example 4: Multiple Features with Global Defaults

```rust
// Host sector - sets defaults for ALL features
host_sector.properties.set(
    "source",                                                      // Main surface
    Value::Source(PixelSource::TileId(concrete_tile_uuid))
);
host_sector.properties.set(
    "cap_source",                                                  // All caps default
    Value::Source(PixelSource::TileId(stone_tile_uuid))
);
host_sector.properties.set(
    "jamb_source",                                                 // All walls default
    Value::Source(PixelSource::TileId(brick_tile_uuid))
);

// Profile 1 - relief using defaults
profile1.properties.set("profile_op", 1);                         // Relief
profile1.properties.set("profile_amount", 1.0);
// Will use host's cap_source and jamb_source

// Profile 2 - recess using defaults
profile2.properties.set("profile_op", 2);                         // Recess
profile2.properties.set("profile_amount", 0.5);
// Will use host's cap_source and jamb_source

// Profile 3 - custom materials override defaults
profile3.properties.set("profile_op", 1);                         // Relief
profile3.properties.set("profile_amount", 2.0);
profile3.properties.set(
    "cap_source",                                                 // Custom cap
    Value::Source(PixelSource::TileId(gold_tile_uuid))
);
profile3.properties.set(
    "jamb_source",                                                // Custom walls
    Value::Source(PixelSource::TileId(marble_tile_uuid))
);
```

### Example 5: Using ConnectionMode for Different Edge Styles

```rust
// Sharp-edged relief (architectural detail)
profile1.properties.set("profile_op", 1);                         // Relief
profile1.properties.set("profile_amount", 0.5);
profile1.properties.set("connection_mode", 0);                    // Hard edges
profile1.properties.set(
    "cap_source",
    Value::Source(PixelSource::TileId(stone_tile_uuid))
);

// Smooth relief (organic shape)
profile2.properties.set("profile_op", 1);                         // Relief
profile2.properties.set("profile_amount", 3.0);
profile2.properties.set("connection_mode", 1);                    // Smooth blending
profile2.properties.set(
    "cap_source",
    Value::Source(PixelSource::TileId(grass_tile_uuid))
);

// Beveled architectural detail
profile3.properties.set("profile_op", 1);                         // Relief
profile3.properties.set("profile_amount", 1.0);
profile3.properties.set("connection_mode", 2);                    // Bevel
profile3.properties.set("bevel_segments", 8);                     // Smooth bevel
profile3.properties.set("bevel_radius", 0.25);                    // Small radius
profile3.properties.set(
    "cap_source",
    Value::Source(PixelSource::TileId(marble_tile_uuid))
);
```

---

## Backward Compatibility

The system maintains backward compatibility with the old naming:

- **`side_source`** on host sector → works as fallback for `jamb_source`

This means existing code using `side_source` will continue to work!

---

## Property Lookup Priority

When the renderer looks for a property, it follows this priority:

### For Caps
1. Profile sector: `cap_source`
2. Profile sector: `source`
3. Host sector: `cap_source`
4. Host sector: `source`

### For Jambs/Sides
1. Profile sector: `jamb_source`
2. Profile sector: `source`
3. Host sector: `jamb_source`
4. Host sector: `side_source` (backward compatibility)
5. Host sector: `source`

This allows you to:
- Set **global defaults** on the host sector
- **Override specific features** on profile sectors
- **Mix and match** materials efficiently
- **Future-proof** your code for new action types

---

## Summary Table

| Property | Where | Type | Default | Purpose |
|----------|-------|------|---------|---------|
| **Operation** |
| `profile_op` | Profile | Int | 0 | 0=Hole, 1=Relief, 2=Recess |
| `profile_amount` | Profile | Float | 0.0 | Unified height/depth for ANY action |
| `profile_height` | Profile | Float | 0.0 | DEPRECATED - use profile_amount |
| `profile_depth` | Profile | Float | 0.0 | DEPRECATED - use profile_amount |
| `profile_target` | Profile | Int | 0 | 0=Front, 1=Back |
| `connection_mode` | Profile | Int | 0 | 0=Hard, 1=Smooth, 2=Bevel |
| `bevel_segments` | Profile | Int | 4 | Bevel segment count |
| `bevel_radius` | Profile | Float | 0.5 | Bevel radius in world units |
| **Materials (Unified)** |
| `source` | Both | Source | - | Generic fallback texture |
| `cap_source` | Both | Source | - | Cap/top surface texture for ANY action |
| `jamb_source` | Both | Source | - | Jamb/wall texture for ANY action |
| `side_source` | Host | Source | - | Legacy fallback for jamb_source |
| **Tiling** |
| `tile_mode` | Host | Int | 1 | 0=Fit, 1=Tile |
| `texture_scale_x` | Host | Float | 1.0 | Horizontal tiling scale |
| `texture_scale_y` | Host | Float | 1.0 | Vertical tiling scale |
| `side_tile_mode` | Host | Int | ← tile_mode | Side wall tiling mode |
| `side_texture_scale_x` | Host | Float | ← texture_scale_x | Side wall U scale |
| `side_texture_scale_y` | Host | Float | ← texture_scale_y | Side wall V scale |
| `jamb_texture_scale_x` | Profile | Float | ← side_texture_scale_x | Jamb U scale override |
| `jamb_texture_scale_y` | Profile | Float | varies | Jamb V scale override |

**Legend:** ← indicates "inherits from"

---

## Benefits of Unified Properties

✅ **Simpler** - Only 2 material properties to remember: `cap_source` and `jamb_source`  
✅ **Consistent** - Same properties work for all action types  
✅ **Future-proof** - New actions work automatically  
✅ **Flexible** - Still allows per-feature customization  
✅ **Clean** - No need for action-specific property names  
✅ **Backward compatible** - Old `side_source` still works
