//! Editable tile compositions. Material UUIDs reference shared picker assets;
//! generated material bundles and baked pixels are derived caches.
use serde::{Deserialize, Serialize};
use theframework::prelude::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MaterialComposition {
    pub base: Uuid,
    pub layers: Vec<CompositionLayer>,
    /// Authored output tiling survives regeneration of the derived material bundle.
    pub tiling: CompositionTiling,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CompositionTiling {
    pub mode: String,
    pub period: [f32; 2],
    pub blend: f32,
}
impl Default for CompositionTiling {
    fn default() -> Self {
        Self {
            mode: "raw".into(),
            period: [1.; 2],
            blend: 0.15,
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CompositionLayer {
    pub id: Uuid,
    pub material: Uuid,
    /// Map geometry supplies a built-in area/path shape; no mask asset is needed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<CompositionGeometry>,
    pub shape: CompositionShape,
    /// Split position in tile units, with 0.5 through the tile centre.
    pub position: f32,
    pub angle: f32,
    /// Full transition width in tile units; zero is a hard edge.
    pub feather: f32,
    pub amount: f32,
    /// World-coherent SDF edge displacement in tile units.
    pub wobble: f32,
    pub wobble_scale: f32,
    /// Optional row mask. A missing row mask uses `amount` as a constant mask.
    pub mask: Option<CompositionMask>,
    pub blend: bool,
    pub overlay_level: f32,
    /// Tile-local UV Position uses X (0) or Y (1), before source UV transforms.
    pub uv_direction: usize,
    pub uv_levels: [f32; 3],
    /// The third and fourth UV Position inputs; the preceding result is input 1.
    pub extra_materials: [Option<Uuid>; 2],
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CompositionMask {
    pub material: Uuid,
    /// Stable material and row identities inside the referenced source bundle.
    pub owner: String,
    pub row: String,
    pub components: String,
    pub mapping: Option<[f32; 2]>,
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionShape {
    Split,
    Blend,
    Mask,
    Height,
    Overlay,
    UvPosition,
    /// A quarter-circle anchored to a tile corner; angle selects the corner.
    Corner,
    /// Matching strips on opposite edges; angle selects horizontal/vertical.
    Strips,
    Road,
    Crossing,
    /// Join matching road layers on neighbouring painted map cells.
    AutoRoad,
    /// Continuous world-space stroke along chosen map linedefs.
    MapPath,
    /// Filled map sectors with a soft, noisy boundary.
    MapArea,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CompositionGeometry {
    pub map: Uuid,
    pub linedefs: Vec<u32>,
    pub sectors: Vec<u32>,
}
fn default_wobble_scale() -> f32 {
    2.
}
fn default_blend() -> bool {
    true
}
fn default_levels() -> [f32; 3] {
    [0.25, 0.5, 0.75]
}
impl CompositionLayer {
    pub fn split(material: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            material,
            geometry: None,
            shape: CompositionShape::Split,
            position: 0.5,
            angle: 0.,
            feather: 0.,
            amount: 0.5,
            wobble: 0.,
            wobble_scale: default_wobble_scale(),
            mask: None,
            blend: default_blend(),
            overlay_level: 0.,
            uv_direction: 0,
            uv_levels: default_levels(),
            extra_materials: [None; 2],
        }
    }
}

/// Ownership of a composition edited directly on a map. Generated tiles are
/// internal render caches, not entries in the source-material picker.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MapMaterialBinding {
    pub map: Uuid,
    pub surface: CompositionSurface,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CompositionSurface {
    /// A unit tile painted as its own sector by the 2D Rect tool.
    RectTile {
        sector: u32,
    },
    /// Rect paint coordinates on a larger surface or terrain.
    SectorTile {
        sector: u32,
        cell: [i32; 2],
    },
    GeometryTile {
        object: Uuid,
        face: usize,
        cell: [i32; 2],
    },
    TerrainTile {
        cell: [i32; 2],
    },
    Sector {
        id: u32,
        ceiling: bool,
    },
    Linedef {
        id: u32,
        row: u8,
    },
    GeometryFace {
        object: Uuid,
        face: usize,
    },
}
