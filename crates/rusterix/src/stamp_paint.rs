//! Immutable CPU stamp artwork and world/UV anchors. This derived asset has no
//! renderer, authoring history, window or serialized-project ownership.
use crate::Tile;
use std::sync::Arc;
use theframework_core::Uuid;

#[derive(Clone, PartialEq)]
pub struct StampPaint {
    pub stamps: Vec<StampRecipe>,
}
#[derive(Clone, PartialEq)]
pub struct StampRecipe {
    pub id: Uuid,
    pub kind: String,
    pub variant: String,
    pub world: Option<[f32; 3]>,
    pub surface_uv: Option<[f32; 2]>,
    pub paint_geo: Option<[u32; 4]>,
    pub size: f32,
    pub camera_scale: Option<f32>,
    pub viewport_size: Option<[i32; 2]>,
    pub rotation: f32,
    pub variation: u32,
    pub material_id: u8,
    pub material_override: bool,
    pub color: [u8; 4],
    pub palette_colors: Vec<[u8; 4]>,
    pub opacity: f32,
    pub source_image: Option<Arc<Tile>>,
    pub source_material_pass: bool,
}
