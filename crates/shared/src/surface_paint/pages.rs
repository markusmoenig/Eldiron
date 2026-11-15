//! Immutable current-paint source keys, independent of camera and residency.
//! Building this index shares the authored planes; it never packs a full atlas,
//! replays authoring history, or initializes a renderer. GPU residency is derived
//! separately so indirect rays can distinguish absent keys from unloaded pages.
use super::Builder;
mod preparation;
mod surface;
pub use surface::SurfacePaintIndexBuilder;
#[cfg(feature = "graphics")]
mod generated;
#[cfg(feature = "graphics")]
mod stamps;
#[cfg(feature = "graphics")]
pub use stamps::StampPageIndexBuilder;
use crate::iso_paint::{ISO_PAINT_BAKE_VERSION, ISO_PAINT_BAKED_CHUNK_SIZE, IsoPaintLayer};
pub use preparation::PaintPageIndexBuilder;
use scenevm::{GeoId, Raster3DSurfacePaintEntry};
use std::{collections::HashSet, sync::Arc};
use theframework_core::SharedBytes;

const SIZE: u32 = ISO_PAINT_BAKED_CHUNK_SIZE as u32;
const PLANE_BYTES: usize = SIZE as usize * SIZE as usize * 4;

#[derive(Clone)]
pub struct PaintPagePixels {
    pub color: SharedBytes,
    pub material: SharedBytes,
}

#[derive(Clone)]
pub struct PaintPageSource {
    pub geo: [u32; 4],
    pub uv_origin: [i32; 2],
    pub revision: u64,
    pub color: SharedBytes,
    pub material: SharedBytes,
}

/// Slot identity is stable for this frozen index, including while individual
/// pages are evicted. A fault receipt names this slot, never a resident slot.
pub struct PaintPageIndex {
    pages: Vec<PaintPageSource>,
    #[cfg(feature = "graphics")]
    generated: generated::GeneratedPages,
    entries: Arc<Vec<Raster3DSurfacePaintEntry>>,
    sources_by_slot: Vec<Option<usize>>,
    alpha_geo_ids: Arc<Vec<GeoId>>,
    alpha_owners: Arc<rustc_hash::FxHashSet<GeoId>>,
}
impl PaintPageIndex {
    /// `table_byte_limit` bounds the complete GPU lookup payload before any
    /// index allocation; callers supply their storage-binding/memory policy.
    /// This is a metadata bound, not a bound on shared authored pixel storage.
    /// Reserve auxiliary headers/other payloads separately before supplying
    /// a device storage-binding limit here.
    pub fn from_layer(layer: &IsoPaintLayer, table_byte_limit: u64) -> Result<Self, String> {
        let current = layer.visible && layer.baked_version == ISO_PAINT_BAKE_VERSION;
        let source = IsoPaintLayer {
            visible: layer.visible,
            baked_version: layer.baked_version,
            baked_chunks: if current {
                layer.baked_chunks.clone()
            } else {
                Default::default()
            },
            surface_instance_owners: if current {
                layer.surface_instance_owners.clone()
            } else {
                Vec::new()
            },
            ..Default::default()
        };
        let mut builder = PaintPageIndexBuilder::new(source, table_byte_limit)?;
        while !builder.advance(64)? {}
        Ok(builder.finish()?.0)
    }
    pub fn source_count(&self) -> usize {
        self.pages.len()
    }
    /// Logical original payload size. Shared planes are not allocated again by
    /// this index; aliases may make their unique retained byte count smaller.
    pub fn logical_source_bytes(&self) -> u64 {
        self.pages.len() as u64 * (PLANE_BYTES * 2) as u64
    }
    pub fn table_bytes(&self) -> u64 {
        (self.entries.len() * size_of::<Raster3DSurfacePaintEntry>()) as u64
    }
    pub fn unloaded_entries(&self) -> &[Raster3DSurfacePaintEntry] {
        &self.entries
    }
    /// Immutable published metadata backing, shared with derived residency owners.
    /// Builders retain exclusive mutation until finish; no second key-table copy.
    pub fn unloaded_entries_shared(&self) -> Arc<Vec<Raster3DSurfacePaintEntry>> {
        self.entries.clone()
    }
    /// Exact CPU lookup shared by every derived GPU/view owner.
    pub fn alpha_owners_shared(&self) -> Arc<rustc_hash::FxHashSet<GeoId>> {
        self.alpha_owners.clone()
    }
    pub fn alpha_geo_ids(&self) -> &[GeoId] {
        &self.alpha_geo_ids
    }
    pub fn page(&self, slot: u32) -> Option<&PaintPageSource> {
        let source = self.sources_by_slot.get(slot as usize)?.as_ref()?;
        self.pages.get(*source)
    }
    /// Metadata access alone does not materialize generated stamp pixels.
    /// Pending is a CPU source job, never an absent or transparent source page.
    pub fn poll_pixels(&self, slot: u32) -> Result<Option<PaintPagePixels>, String> {
        let source = *self.sources_by_slot.get(slot as usize).and_then(Option::as_ref)
            .ok_or("Absent paint source slot")?;
        #[cfg(feature = "graphics")]
        if self.generated.contains(source) { return self.generated.poll(source); }
        let page = &self.pages[source];
        if page.color.len() != PLANE_BYTES || page.material.len() != PLANE_BYTES {
            return Err("Authored source page has invalid plane dimensions".into());
        }
        Ok(Some(PaintPagePixels { color: page.color.clone(), material: page.material.clone() }))
    }
    pub fn slot(&self, geo: [u32; 4], origin: [i32; 2]) -> Option<u32> {
        let mut slot =
            Builder::surface_paint_entry_hash(geo, origin) as usize & (self.entries.len() - 1);
        for _ in 0..self.entries.len() {
            let entry = self.entries[slot];
            if entry.uv_size == [0; 2] {
                return None;
            }
            if entry.geo == geo && entry.uv_origin == origin {
                return Some(slot as u32);
            }
            slot = (slot + 1) & (self.entries.len() - 1);
        }
        None
    }
    /// Derive only the lookup metadata for selected resident pages. Origins are
    /// aligned to one 64x64 page and must be unique/in bounds. Original source
    /// slots, all authored keys, and the immutable source bytes remain intact.
    pub fn resident_entries(
        &self,
        atlas: [u32; 2],
        residents: &[(u32, [u32; 2])],
    ) -> Result<Vec<Raster3DSurfacePaintEntry>, String> {
        let mut entries = self.entries.as_ref().clone();
        let mut slots = HashSet::new();
        let mut origins = HashSet::new();
        for &(slot, origin) in residents {
            if self.page(slot).is_none() || !slots.insert(slot) {
                return Err("Invalid or duplicate resident paint source slot".into());
            }
            if !origin.iter().all(|value| *value % SIZE == 0)
                || !origins.insert(origin)
                || !(0..2).all(|axis| {
                    origin[axis]
                        .checked_add(SIZE)
                        .is_some_and(|end| end <= atlas[axis])
                })
            {
                return Err(
                    "Resident paint rectangle is unaligned, overlapping or out of bounds".into(),
                );
            }
            entries[slot as usize].atlas_rect = [origin[0], origin[1], SIZE, SIZE];
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests;
