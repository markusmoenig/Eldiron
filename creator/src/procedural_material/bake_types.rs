//! Addressed CPU asset results shared by native and cooperative browser bakes.
use super::PreviewSelection;
pub(super) struct Job {
    pub pixels_per_tile: u32,
    pub tile: theframework::prelude::Uuid,
    pub revision: u64,
    pub bundle: String,
    pub continuous: bool,
    pub selection: Option<PreviewSelection>,
    pub preview_aspect: f32,
}
pub(super) struct Completed {
    pub pixels_per_tile: u32,
    pub tile: theframework::prelude::Uuid,
    pub bundle: String,
    pub revision: u64,
    pub frame: Result<Frame, String>,
}
pub(super) struct Frame {
    pub size: usize,
    pub animation_images: Vec<[Vec<u8>; 5]>,
    pub animation_selections: Vec<Vec<u8>>,
    pub images: [Vec<u8>; 5],
    pub tile: rusterix::Texture,
    pub warnings: Vec<String>,
    pub selection: Option<(PreviewSelection, Vec<u8>)>,
    pub continuous: bool,
    /// Unit-domain RGBA thumbnails for every authored operation, keyed by its
    /// stable identity. They do not depend on minimap selection or output wires.
    pub thumbnails: Vec<(String, Vec<u8>)>,
    pub surface_pages: Option<std::sync::Arc<SurfacePages>>,
}

pub(super) struct SurfacePages {
    pub layout: rusterix::TileUvPages,
    pub textures: Vec<rusterix::Texture>,
}
