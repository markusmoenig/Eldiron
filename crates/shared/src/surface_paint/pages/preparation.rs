//! Cooperative frozen-key and material classification. One advance has bounded
//! table initialization, original-page scans and instance-owner work. No GPU or
//! editable history is retained, and no partially classified index is exposed.
use super::*;
use crate::iso_paint::{IsoPaintBakedChunk, IsoPaintOwner};

pub struct PaintPageIndexBuilder {
    index: PaintPageIndex,
    table_len: usize,
    chunks: crate::shared_index_map::SharedIndexMap<String, IsoPaintBakedChunk>,
    chunk_cursor: usize,
    owners: crate::shared_vec::SharedVec<IsoPaintOwner>,
    owner_cursor: usize,
    alpha_seen: rustc_hash::FxHashSet<GeoId>,
    has_alpha: bool,
    remaining: IsoPaintLayer,
    failure: Option<String>,
}
impl PaintPageIndexBuilder {
    /// Takes frozen source ownership. Original planes are moved/shared, never
    /// copied, and the returned residual layer contains only non-UV inputs.
    pub fn new(mut layer: IsoPaintLayer, table_byte_limit: u64) -> Result<Self, String> {
        let current = layer.visible && layer.baked_version == ISO_PAINT_BAKE_VERSION;
        let chunks = std::mem::take(&mut layer.baked_chunks);
        let owners = std::mem::take(&mut layer.surface_instance_owners);
        let chunks = if current { chunks } else { Default::default() };
        let owners = if current { owners } else { Default::default() };
        let count = chunks.len();
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
        let mut index = PaintPageIndex {
            pages: Vec::new(),
            #[cfg(feature = "graphics")]
            generated: Default::default(),
            entries: Arc::new(Vec::new()),
            sources_by_slot: Vec::new(),
            alpha_geo_ids: Arc::new(Vec::new()),
            alpha_owners: Arc::new(rustc_hash::FxHashSet::default()),
        };
        index
            .pages
            .try_reserve_exact(count)
            .map_err(|e| e.to_string())?;
        Arc::get_mut(&mut index.entries)
            .unwrap()
            .try_reserve_exact(table_len)
            .map_err(|e| e.to_string())?;
        index
            .sources_by_slot
            .try_reserve_exact(table_len)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            index,
            table_len,
            chunks,
            chunk_cursor: 0,
            owners,
            owner_cursor: 0,
            alpha_seen: rustc_hash::FxHashSet::default(),
            has_alpha: false,
            remaining: layer,
            failure: None,
        })
    }
    /// At most `items` 64x64 material scans/instance owners, preceded by at most
    /// `items * 64` table slots. Zero does no work. True permits `finish`.
    pub fn advance(&mut self, items: usize) -> Result<bool, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let result = self.advance_inner(items);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn advance_inner(&mut self, items: usize) -> Result<bool, String> {
        let end = self
            .index
            .entries
            .len()
            .saturating_add(items.saturating_mul(64))
            .min(self.table_len);
        Arc::get_mut(&mut self.index.entries).unwrap().resize(
            end,
            Raster3DSurfacePaintEntry {
                geo: [0; 4],
                uv_origin: [0; 2],
                uv_size: [0; 2],
                atlas_rect: [0; 4],
            },
        );
        self.index.sources_by_slot.resize(end, None);
        if end < self.table_len {
            return Ok(false);
        }
        for _ in 0..items {
            if let Some((_, chunk)) = self.chunks.get_index(self.chunk_cursor) {
                let chunk = chunk.clone();
                self.chunk_cursor += 1;
                self.insert(chunk)?;
            } else if !self.has_alpha {
                // Original pages prove there is no alpha pass to expand.
                self.owner_cursor = self.owners.len();
                break;
            } else if let Some(owner) = self.owners.get(self.owner_cursor).cloned() {
                self.owner_cursor += 1;
                self.insert_alpha(Builder::iso_paint_owner_geo_id(&owner));
            } else {
                break;
            }
        }
        Ok(self.chunk_cursor == self.chunks.len() && self.owner_cursor == self.owners.len())
    }
    fn insert(&mut self, chunk: IsoPaintBakedChunk) -> Result<(), String> {
        if chunk.color_rgba.len() != PLANE_BYTES || chunk.material_rgba.len() != PLANE_BYTES {
            return Err("Paint source page has invalid plane dimensions".into());
        }
        if chunk
            .origin
            .iter()
            .any(|value| value.rem_euclid(SIZE as i32) != 0)
        {
            return Err("Paint source page has an unaligned UV origin".into());
        }
        let mut slot = Builder::surface_paint_entry_hash(chunk.paint_geo, chunk.origin) as usize
            & (self.table_len - 1);
        while self.index.entries[slot].uv_size != [0; 2] {
            if self.index.entries[slot].geo == chunk.paint_geo
                && self.index.entries[slot].uv_origin == chunk.origin
            {
                return Err("Paint source contains duplicate UV keys".into());
            }
            slot = (slot + 1) & (self.table_len - 1);
        }
        if Builder::surface_paint_chunk_needs_alpha(&chunk) {
            self.has_alpha = true;
            self.insert_alpha(Builder::iso_paint_owner_geo_id(&chunk.owner));
        }
        Arc::get_mut(&mut self.index.entries).unwrap()[slot] = Raster3DSurfacePaintEntry {
            geo: chunk.paint_geo,
            uv_origin: chunk.origin,
            uv_size: [SIZE; 2],
            atlas_rect: [0; 4],
        };
        self.index.sources_by_slot[slot] = Some(self.index.pages.len());
        self.index.pages.push(PaintPageSource {
            geo: chunk.paint_geo,
            uv_origin: chunk.origin,
            revision: chunk.revision,
            color: chunk.color_rgba,
            material: chunk.material_rgba,
        });
        Ok(())
    }
    fn insert_alpha(&mut self, geo: GeoId) {
        if self.alpha_seen.insert(geo) {
            Arc::get_mut(&mut self.index.alpha_geo_ids)
                .unwrap()
                .push(geo);
        }
    }
    pub fn finish(mut self) -> Result<(PaintPageIndex, IsoPaintLayer), String> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if self.index.entries.len() != self.table_len
            || self.chunk_cursor != self.chunks.len()
            || self.owner_cursor != self.owners.len()
        {
            return Err("Paint source preparation is incomplete".into());
        }
        self.index.alpha_owners = Arc::new(self.alpha_seen);
        Ok((self.index, self.remaining))
    }
}
