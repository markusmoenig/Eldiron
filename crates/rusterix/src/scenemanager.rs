use crate::{
    Assets, BBox, Batch3D, Chunk, ChunkBuilder, D2ChunkBuilder, D3ChunkBuilder, Map, PixelSource,
    TerrainChunk, Texture, Tile, Value, ValueContainer,
};
use scenevm::Chunk as VMChunk;
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
pub enum SceneManagerCmd {
    SetTileList(Vec<Tile>, FxHashMap<Uuid, u16>),
    SetPalette(ThePalette, Vec<[f32; 4]>),
    SetMap(Map),
    UpdateMap(Map),
    SetBuilder2D(Option<Box<dyn ChunkBuilder>>),
    SetFocusChunk(Option<(i32, i32)>),
    ReplaceDirty(Vec<(i32, i32)>),
    AddDirty(Vec<(i32, i32)>),
    SetDirtyTerrainChunks(Vec<TerrainChunk>),
    SetTerrainModifierState(bool),
    Quit,
}

// #[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum SceneManagerResult {
    Startup,
    Clear,
    Chunk(VMChunk, i32, i32, Vec<crate::BillboardMetadata>),
    ProcessedHeights(Vec2<i32>, FxHashMap<(i32, i32), f32>),
    UpdatedBatch3D((i32, i32), Batch3D),
    Quit,
}

/// WASM-compatible scene manager that processes chunks incrementally without threads
pub struct SceneManager {
    // Internal state (no channels needed)
    assets: Assets,
    map: Map,
    terrain_modifiers: bool,
    chunk_size: i32,

    dirty: FxHashSet<(i32, i32)>,
    all: FxHashSet<(i32, i32)>,
    terrain_modifiers_update: FxHashSet<(i32, i32)>,
    total_chunks: i32,
    focus_chunk: Option<(i32, i32)>,

    chunk_builder_d2: Option<Box<dyn ChunkBuilder>>,
    chunk_builder_d3: Option<Box<dyn ChunkBuilder>>,
    apply_preview_filters: bool,

    // Results queue
    results: Vec<SceneManagerResult>,

    // Processing state
    processing_final_update: bool,
    final_update_iter: std::vec::IntoIter<(i32, i32)>,
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneManager {
    fn palette_index_tile_uuid(index: u16) -> Uuid {
        Uuid::from_u128(0x50414C455454455F0000000000000000u128 | index as u128)
    }

    fn ensure_palette_tile(&mut self, index: u16) {
        let Some(Some(col)) = self.assets.palette.colors.get(index as usize) else {
            return;
        };
        let tile_id = Self::palette_index_tile_uuid(index);
        let tile = Tile::from_texture(Texture::from_color(col.to_u8_array()));
        let mut tile = tile;
        tile.id = tile_id;
        let [roughness, metallic, opacity, emissive] = self
            .assets
            .palette_materials
            .get(index as usize)
            .copied()
            .unwrap_or([0.5, 0.0, 1.0, 0.0]);
        for texture in &mut tile.textures {
            texture.set_materials_all(roughness, metallic, opacity, emissive);
        }

        self.assets.tiles.insert(tile_id, tile.clone());
        if let Some(&ti) = self.assets.tile_indices.get(&tile_id) {
            self.assets.tile_list[ti as usize] = tile;
        } else {
            let ti = self.assets.tile_list.len() as u16;
            self.assets.tile_indices.insert(tile_id, ti);
            self.assets.tile_list.push(tile);
        }
    }

    fn ensure_palette_tiles_from_props(&mut self, props: &ValueContainer) {
        for v in props.values() {
            if let Value::Source(PixelSource::PaletteIndex(index)) = v {
                self.ensure_palette_tile(*index);
            }
        }
    }

    fn ensure_palette_tiles_for_map(&mut self) {
        let map = self.map.clone();
        for sector in &map.sectors {
            self.ensure_palette_tiles_from_props(&sector.properties);
        }
        for linedef in &map.linedefs {
            self.ensure_palette_tiles_from_props(&linedef.properties);
        }
        for vertex in &map.vertices {
            self.ensure_palette_tiles_from_props(&vertex.properties);
        }
        for entity in &map.entities {
            self.ensure_palette_tiles_from_props(&entity.attributes);
        }
        for item in &map.items {
            self.ensure_palette_tiles_from_props(&item.attributes);
        }
        for profile in map.profiles.values() {
            for sector in &profile.sectors {
                self.ensure_palette_tiles_from_props(&sector.properties);
            }
            for linedef in &profile.linedefs {
                self.ensure_palette_tiles_from_props(&linedef.properties);
            }
            for vertex in &profile.vertices {
                self.ensure_palette_tiles_from_props(&vertex.properties);
            }
        }
    }

    pub fn new() -> Self {
        Self {
            assets: Assets::default(),
            map: Map::default(),
            terrain_modifiers: true,
            chunk_size: 32,

            dirty: FxHashSet::default(),
            all: FxHashSet::default(),
            terrain_modifiers_update: FxHashSet::default(),
            total_chunks: 0,
            focus_chunk: None,

            chunk_builder_d2: Some(Box::new(D2ChunkBuilder::new())),
            chunk_builder_d3: Some(Box::new(D3ChunkBuilder::new())),
            apply_preview_filters: false,

            results: Vec::new(),

            processing_final_update: false,
            final_update_iter: Vec::new().into_iter(),
        }
    }

    /// Check for a result (pop from queue)
    pub fn receive(&mut self) -> Option<SceneManagerResult> {
        if !self.results.is_empty() {
            Some(self.results.remove(0))
        } else {
            None
        }
    }

    /// Send a command (process immediately, no channels needed)
    pub fn send(&mut self, cmd: SceneManagerCmd) {
        match cmd {
            SceneManagerCmd::SetTileList(tiles, indices) => {
                self.assets.tile_list = tiles;
                self.assets.tile_indices = indices;
                self.dirty = Self::generate_chunk_coords(&self.map.bbox(), self.chunk_size);
                self.all = self.dirty.clone();
            }
            SceneManagerCmd::SetPalette(palette, palette_materials) => {
                self.assets.palette = palette;
                self.assets.palette_materials = palette_materials;
                self.ensure_palette_tiles_for_map();
                self.dirty = Self::generate_chunk_coords(&self.map.bbox(), self.chunk_size);
                self.all = self.dirty.clone();
            }
            SceneManagerCmd::SetBuilder2D(builder) => {
                self.chunk_builder_d2 = builder;
                self.dirty = Self::generate_chunk_coords(&self.map.bbox(), self.chunk_size);
                self.all = self.dirty.clone();
            }
            SceneManagerCmd::SetFocusChunk(chunk) => {
                self.focus_chunk = chunk;
            }
            SceneManagerCmd::ReplaceDirty(dirty_chunks) => {
                self.dirty.clear();
                self.all.clear();
                for d in dirty_chunks {
                    self.dirty.insert(d);
                    self.all.insert(d);
                }
                self.total_chunks = self.dirty.len() as i32;
            }
            SceneManagerCmd::SetMap(mut new_map) => {
                if !self.apply_preview_filters {
                    new_map.properties.remove("preview_hide");
                }
                if self.map.id != new_map.id {
                    self.results.push(SceneManagerResult::Clear);
                }
                let builder_sector_count = new_map
                    .sectors
                    .iter()
                    .filter(|sector| {
                        !sector
                            .properties
                            .get_str_default("builder_graph_data", String::new())
                            .trim()
                            .is_empty()
                    })
                    .count();
                self.map = new_map;
                self.ensure_palette_tiles_for_map();
                self.chunk_size = self.map.terrain.chunk_size.max(1);
                let mut bbox = self.map.bbox();
                if let Some(tbbox) = self.map.terrain.compute_bounds() {
                    bbox.expand_bbox(tbbox);
                }
                println!(
                    "SceneManagerCmd::SetMap(Min: {}, Max: {}, BuilderSectors: {})",
                    bbox.min, bbox.max, builder_sector_count
                );
                self.dirty = Self::generate_chunk_coords(&bbox, self.chunk_size);
                self.all = self.dirty.clone();
                self.total_chunks = self.dirty.len() as i32;
            }
            SceneManagerCmd::UpdateMap(mut new_map) => {
                if !self.apply_preview_filters {
                    new_map.properties.remove("preview_hide");
                }
                // Keep current dirty set; caller controls incremental invalidation via AddDirty.
                self.map = new_map;
                self.ensure_palette_tiles_for_map();
                self.chunk_size = self.map.terrain.chunk_size.max(1);
            }
            SceneManagerCmd::AddDirty(dirty_chunks) => {
                for d in dirty_chunks {
                    self.dirty.insert(d);
                    self.all.insert(d);
                }
            }
            SceneManagerCmd::SetDirtyTerrainChunks(dirty_chunks) => {
                for chunk in dirty_chunks {
                    let coord = (chunk.origin.x, chunk.origin.y);
                    let local = self.map.terrain.get_chunk_coords(coord.0, coord.1);
                    self.map.terrain.chunks.insert(local, chunk);
                    self.dirty.insert(coord);
                    self.all.insert(coord);
                    if !self.terrain_modifiers {
                        self.terrain_modifiers_update.insert(coord);
                    }
                }
            }
            SceneManagerCmd::SetTerrainModifierState(state) => {
                if state && !self.terrain_modifiers {
                    // Update all the chunks we created w/o modifiers
                    for d in &self.terrain_modifiers_update {
                        self.dirty.insert(*d);
                        self.all.insert(*d);
                    }
                }
                self.terrain_modifiers = state;
                self.terrain_modifiers_update.clear();
            }
            SceneManagerCmd::Quit => {
                self.results.push(SceneManagerResult::Quit);
            }
        }
    }

    pub fn set_tile_list(&mut self, tiles: Vec<Tile>, tile_indices: FxHashMap<Uuid, u16>) {
        self.send(SceneManagerCmd::SetTileList(tiles, tile_indices));
    }

    pub fn set_palette(&mut self, palette: ThePalette, palette_materials: Vec<[f32; 4]>) {
        self.send(SceneManagerCmd::SetPalette(palette, palette_materials));
    }

    pub fn set_builder_2d(&mut self, builder: Option<Box<dyn ChunkBuilder>>) {
        self.send(SceneManagerCmd::SetBuilder2D(builder));
    }

    pub fn set_map(&mut self, map: Map) {
        self.send(SceneManagerCmd::SetMap(map));
    }

    pub fn update_map(&mut self, map: Map) {
        self.send(SceneManagerCmd::UpdateMap(map));
    }

    pub fn set_apply_preview_filters(&mut self, apply: bool) {
        self.apply_preview_filters = apply;
        if !apply {
            self.map.properties.remove("preview_hide");
        }
    }

    pub fn add_dirty(&mut self, dirty: Vec<(i32, i32)>) {
        self.send(SceneManagerCmd::AddDirty(dirty));
    }

    pub fn set_focus_chunk(&mut self, chunk: Option<(i32, i32)>) {
        self.send(SceneManagerCmd::SetFocusChunk(chunk));
    }

    pub fn replace_dirty(&mut self, dirty: Vec<(i32, i32)>) {
        self.send(SceneManagerCmd::ReplaceDirty(dirty));
    }

    pub fn set_dirty_terrain_chunks(&mut self, dirty: Vec<TerrainChunk>) {
        self.send(SceneManagerCmd::SetDirtyTerrainChunks(dirty));
    }

    pub fn set_terrain_modifier_state(&mut self, state: bool) {
        self.send(SceneManagerCmd::SetTerrainModifierState(state));
    }

    pub fn startup(&mut self) {
        self.results.push(SceneManagerResult::Startup);
    }

    /// Process one chunk per call. Call this from your main loop/update function.
    /// Returns true if there's more work to do, false if idle.
    pub fn tick(&mut self) -> bool {
        // If we're doing final terrain mesh updates
        if self.processing_final_update {
            if let Some(coord) = self.final_update_iter.next() {
                let local = self.map.terrain.get_chunk_coords(coord.0, coord.1);
                if self.map.terrain.chunks.contains_key(&local) {
                    if let Some(ch) = self.map.terrain.chunks.get(&local).cloned() {
                        let batch = ch.build_mesh(&self.map.terrain);
                        if !batch.vertices.is_empty() {
                            self.results
                                .push(SceneManagerResult::UpdatedBatch3D(coord, batch));
                        }
                    }
                }
                return true; // More final updates to process
            } else {
                // Done with final updates
                self.processing_final_update = false;
                return false;
            }
        }

        // Process one dirty chunk
        let next_coord = if let Some(focus) = self.focus_chunk {
            self.dirty
                .iter()
                .min_by_key(|(x, y)| {
                    let dx = *x - focus.0;
                    let dy = *y - focus.1;
                    dx * dx + dy * dy
                })
                .copied()
        } else {
            self.dirty.iter().next().copied()
        };

        if let Some(coord) = next_coord {
            self.dirty.remove(&coord);

            let mut chunk = Chunk::new(Vec2::new(coord.0, coord.1), self.chunk_size);
            let mut vmchunk = VMChunk::new(Vec2::new(coord.0, coord.1), self.chunk_size);

            if let Some(cb_d2) = &mut self.chunk_builder_d2 {
                cb_d2.build(&self.map, &self.assets, &mut chunk, &mut vmchunk);
            }

            if let Some(cb_d3) = &mut self.chunk_builder_d3 {
                cb_d3.build(&self.map, &self.assets, &mut chunk, &mut vmchunk);
            }

            // Send the chunk with billboards
            let billboards = chunk.billboards.clone();
            self.results.push(SceneManagerResult::Chunk(
                vmchunk,
                self.dirty.len() as i32,
                self.total_chunks,
                billboards,
            ));

            // Check if we just finished all dirty chunks
            if self.dirty.is_empty() {
                // Start final terrain mesh update phase
                let all_coords: Vec<(i32, i32)> = self.all.iter().copied().collect();
                self.final_update_iter = all_coords.into_iter();
                self.processing_final_update = true;
            }

            true // More work to do
        } else {
            false // Idle
        }
    }

    /// Process multiple chunks at once (useful for batch processing)
    /// Returns the number of chunks processed
    pub fn tick_batch(&mut self, max_chunks: usize) -> usize {
        let mut processed = 0;
        for _ in 0..max_chunks {
            if !self.tick() {
                break;
            }
            processed += 1;
        }
        processed
    }

    /// Returns all chunks which cover the given bounding box.
    fn generate_chunk_coords(bbox: &BBox, chunk_size: i32) -> FxHashSet<(i32, i32)> {
        let min_x = (bbox.min.x / chunk_size as f32).floor() as i32;
        let min_y = (bbox.min.y / chunk_size as f32).floor() as i32;
        let max_x = (bbox.max.x / chunk_size as f32).ceil() as i32;
        let max_y = (bbox.max.y / chunk_size as f32).ceil() as i32;

        let mut coords = FxHashSet::default();
        for y in min_y..max_y {
            for x in min_x..max_x {
                coords.insert((x * chunk_size, y * chunk_size));
            }
        }
        coords
    }

    /// Check if the manager is currently processing chunks
    pub fn is_busy(&self) -> bool {
        !self.dirty.is_empty() || self.processing_final_update
    }

    /// Get the number of chunks remaining to process
    pub fn remaining_chunks(&self) -> usize {
        self.dirty.len()
    }
}
