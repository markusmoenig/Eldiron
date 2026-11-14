//! Immutable current-paint source keys, independent of camera and residency.
//! Building this index shares the authored planes; it never packs a full atlas,
//! replays authoring history, or initializes a renderer. GPU residency is derived
//! separately so indirect rays can distinguish absent keys from unloaded pages.
use super::Builder;
use crate::iso_paint::{ISO_PAINT_BAKE_VERSION, ISO_PAINT_BAKED_CHUNK_SIZE, IsoPaintLayer};
use scenevm::{GeoId, Raster3DSurfacePaintEntry};
use std::collections::HashSet;
use theframework_core::SharedBytes;

const SIZE: u32 = ISO_PAINT_BAKED_CHUNK_SIZE as u32;
const PLANE_BYTES: usize = SIZE as usize * SIZE as usize * 4;

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
    entries: Vec<Raster3DSurfacePaintEntry>,
    sources_by_slot: Vec<Option<usize>>,
    alpha_geo_ids: Vec<GeoId>,
}
impl PaintPageIndex {
    /// `table_byte_limit` bounds the complete GPU lookup payload before any
    /// index allocation; callers supply their storage-binding/memory policy.
    /// This is a metadata bound, not a bound on shared authored pixel storage.
    /// Reserve auxiliary headers/other payloads separately before supplying
    /// a device storage-binding limit here.
    pub fn from_layer(layer: &IsoPaintLayer, table_byte_limit: u64) -> Result<Self, String> {
        let current = layer.visible && layer.baked_version == ISO_PAINT_BAKE_VERSION;
        let count = if current { layer.baked_chunks.len() } else { 0 };
        let table_len = count
            .checked_mul(2)
            .and_then(usize::checked_next_power_of_two)
            .filter(|length| *length <= u32::MAX as usize)
            .ok_or("Paint source index exceeds its addressable range")?
            .max(2);
        let table_bytes = table_len as u64 * size_of::<Raster3DSurfacePaintEntry>() as u64;
        if table_bytes > table_byte_limit {
            return Err("Paint source index exceeds its metadata byte limit".into());
        }
        let mut pages = Vec::new();
        pages.try_reserve_exact(count).map_err(|e| e.to_string())?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(table_len)
            .map_err(|e| e.to_string())?;
        entries.resize(
            table_len,
            Raster3DSurfacePaintEntry {
                geo: [0; 4],
                uv_origin: [0; 2],
                uv_size: [0; 2],
                atlas_rect: [0; 4],
            },
        );
        let mut sources_by_slot = Vec::new();
        sources_by_slot
            .try_reserve_exact(table_len)
            .map_err(|e| e.to_string())?;
        sources_by_slot.resize(table_len, None);
        if current {
            for chunk in layer.baked_chunks.values() {
                if chunk.color_rgba.len() != PLANE_BYTES || chunk.material_rgba.len() != PLANE_BYTES
                {
                    return Err("Paint source page has invalid plane dimensions".into());
                }
                if chunk
                    .origin
                    .iter()
                    .any(|value| value.rem_euclid(SIZE as i32) != 0)
                {
                    return Err("Paint source page has an unaligned UV origin".into());
                }
                let mut slot = Builder::surface_paint_entry_hash(chunk.paint_geo, chunk.origin)
                    as usize
                    & (table_len - 1);
                while entries[slot].uv_size != [0; 2] {
                    if entries[slot].geo == chunk.paint_geo
                        && entries[slot].uv_origin == chunk.origin
                    {
                        return Err("Paint source contains duplicate UV keys".into());
                    }
                    slot = (slot + 1) & (table_len - 1);
                }
                entries[slot] = Raster3DSurfacePaintEntry {
                    geo: chunk.paint_geo,
                    uv_origin: chunk.origin,
                    uv_size: [SIZE; 2],
                    atlas_rect: [0; 4],
                };
                sources_by_slot[slot] = Some(pages.len());
                pages.push(PaintPageSource {
                    geo: chunk.paint_geo,
                    uv_origin: chunk.origin,
                    revision: chunk.revision,
                    color: chunk.color_rgba.clone(),
                    material: chunk.material_rgba.clone(),
                });
            }
        }
        Ok(Self {
            pages,
            entries,
            sources_by_slot,
            alpha_geo_ids: if current {
                Builder::surface_paint_alpha_geo_ids(layer)
            } else {
                Vec::new()
            },
        })
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
    pub fn alpha_geo_ids(&self) -> &[GeoId] {
        &self.alpha_geo_ids
    }
    pub fn page(&self, slot: u32) -> Option<&PaintPageSource> {
        let source = self.sources_by_slot.get(slot as usize)?.as_ref()?;
        self.pages.get(*source)
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
        let mut entries = self.entries.clone();
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
