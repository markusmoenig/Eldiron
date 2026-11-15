//! Immutable authored pages for baked 3D paint, independent of any GPU.
use std::{collections::HashSet, sync::Arc};
use theframework_core::SharedBytes;

pub const PAGE_SIZE: u32 = 64;

#[derive(Clone, PartialEq)]
pub struct SurfacePaintPage {
    pub atlas_origin: [u32; 2],
    pub color_rgba: SharedBytes,
    pub material_rgba: SharedBytes,
}

/// Full logical source description. Pages share authored bytes; consumers own
/// their GPU residency policy. No stitched CPU atlas is stored in the asset.
#[derive(Clone, PartialEq)]
pub struct SurfacePaint {
    pub key: u64,
    pub width: u32,
    pub height: u32,
    pub pages: Vec<SurfacePaintPage>,
    pub entries: Arc<[scenevm::Raster3DSurfacePaintEntry]>,
    pub alpha_geo_ids: Vec<scenevm::GeoId>,
}
mod validation;
pub use validation::SurfacePaintValidation;
