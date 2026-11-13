//! Immutable CPU display assets for baked 3D paint, independent of any GPU.
#[derive(Clone)]
pub struct SurfacePaint {
    pub key: u64,
    pub width: u32,
    pub height: u32,
    pub color_rgba: Vec<u8>,
    pub material_rgba: Vec<u8>,
    pub entries: Vec<scenevm::Raster3DSurfacePaintEntry>,
    pub alpha_geo_ids: Vec<scenevm::GeoId>,
}
