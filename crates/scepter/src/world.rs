//! Map collaboration protocol. Coordinates are world X/Y/Z; 2D grids use X/Z.
use crate::{RegionRef, TileSelector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapTargets {
    #[serde(default)]
    pub sectors: Vec<u32>,
    #[serde(default)]
    pub geometry: Vec<String>,
    #[serde(default)]
    pub characters: Vec<String>,
    #[serde(default)]
    pub items: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapTransform {
    pub region: RegionRef,
    pub targets: MapTargets,
    #[serde(default)]
    pub translation: [f32; 3],
    #[serde(default)]
    pub pivot: [f32; 3],
    #[serde(default)]
    pub rotation_y: f32,
    #[serde(default = "unit_scale")]
    pub scale: [f32; 3],
}
fn unit_scale() -> [f32; 3] {
    [1.; 3]
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapDelete {
    pub region: RegionRef,
    pub targets: MapTargets,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EraseCells {
    pub region: RegionRef,
    pub cells: Vec<[i32; 2]>,
    #[serde(default)]
    pub layer: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorCreate {
    pub region: RegionRef,
    pub name: String,
    pub polygon: Vec<[f32; 2]>,
    #[serde(default)]
    pub floor: Option<TileSelector>,
    #[serde(default)]
    pub ceiling: Option<TileSelector>,
    #[serde(default)]
    pub height: Option<f32>,
    #[serde(default)]
    pub layer: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryCreate {
    pub region: RegionRef,
    pub id: String,
    pub name: String,
    /// box, cylinder, floor or ceiling.
    pub shape: String,
    pub min: [f32; 3],
    pub max: [f32; 3],
    #[serde(default)]
    pub material: Option<TileSelector>,
    #[serde(default)]
    pub segments: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityPlace {
    pub region: RegionRef,
    pub id: String,
    pub kind: String,
    pub template: String,
    pub position: [f32; 3],
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityUpdate {
    pub region: RegionRef,
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    #[serde(default)]
    pub orientation: Option<[f32; 2]>,
    #[serde(default)]
    pub name: Option<String>,
    /// Instance TOML, synchronized with the entity configuration graph.
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorSelection {
    #[serde(default)]
    pub geometry_vertices: Vec<(String, usize)>,
    #[serde(default)]
    pub region: Option<RegionRef>,
    pub targets: MapTargets,
    #[serde(default)]
    pub vertices: Vec<u32>,
    #[serde(default)]
    pub linedefs: Vec<u32>,
    /// [object UUID, face index] entries.
    #[serde(default)]
    pub faces: Vec<(String, usize)>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorFocus {
    #[serde(default)]
    pub region: Option<RegionRef>,
    /// 2d, iso, orbit or firstp.
    #[serde(default)]
    pub view: Option<String>,
    pub center: [f32; 3],
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    #[serde(default)]
    pub scale: Option<f32>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChangeQuery {
    #[serde(default)]
    pub after: u64,
    #[serde(default)]
    pub stream: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorUpdate {
    pub region: RegionRef,
    pub id: u32,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub floor_height: Option<f32>,
    #[serde(default)]
    pub height: Option<f32>,
    #[serde(default)]
    pub layer: Option<u8>,
}
