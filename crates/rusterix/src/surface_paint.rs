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
impl SurfacePaint {
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0
            || self.height == 0
            || self.pages.is_empty()
            || self.width % PAGE_SIZE != 0
            || self.height % PAGE_SIZE != 0
            || self.entries.len() < 2
            || !self.entries.len().is_power_of_two()
        {
            return Err("Invalid surface paint page dimensions/table".into());
        }
        let mut origins = HashSet::new();
        let bytes = (PAGE_SIZE * PAGE_SIZE * 4) as usize;
        for page in &self.pages {
            if page.color_rgba.len() != bytes
                || page.material_rgba.len() != bytes
                || !origins.insert(page.atlas_origin)
                || !(0..2).all(|axis| {
                    page.atlas_origin[axis] % PAGE_SIZE == 0
                        && page.atlas_origin[axis]
                            .checked_add(PAGE_SIZE)
                            .is_some_and(|end| end <= [self.width, self.height][axis])
                })
            {
                return Err("Invalid, overlapping or out-of-bounds surface paint page".into());
            }
        }
        for entry in self.entries.iter().filter(|entry| entry.uv_size != [0; 2]) {
            if entry.uv_size != [PAGE_SIZE; 2]
                || entry.atlas_rect[2..] != [PAGE_SIZE; 2]
                || !origins.contains(&[entry.atlas_rect[0], entry.atlas_rect[1]])
            {
                return Err("Surface paint lookup refers to an absent page".into());
            }
        }
        Ok(())
    }
}
