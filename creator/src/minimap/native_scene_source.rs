//! Incremental CPU geometry for a parent overview whose editor is showing a
//! different map. Never borrow that editor's chunk worker or display VM.
use super::*;
use rusterix::{SceneManager, SceneManagerResult, scene_build_index::SceneBuildIndex};

#[derive(Clone, Copy, PartialEq)]
pub(super) struct SourceKey {
    pub document: Uuid,
    pub map: Uuid,
    pub source: u64,
    pub assets: u64,
}
pub(super) struct Source {
    pub key: SourceKey,
    manager: SceneManager,
    index: SceneBuildIndex,
}
impl Source {
    pub fn new(key: SourceKey, map: Map, assets: &rusterix::Assets) -> Self {
        let mut manager = SceneManager::new();
        manager.set_tile_list(assets.tile_list.clone(), assets.tile_indices.clone());
        manager.set_block_props(assets.block_props.clone());
        manager.set_palette(
            assets.palette.clone(),
            assets.palette_materials.clone(),
            assets.palette_material_ids.clone(),
        );
        manager.set_map(map);
        Self {
            key,
            manager,
            index: SceneBuildIndex::default(),
        }
    }
    /// Called by application update, with bounded work per update. Preparation
    /// only borrows the completed immutable publication.
    pub fn advance(&mut self) -> bool {
        if self.manager.is_busy() {
            rusterix::chunkbuilder::terrain_surface::TerrainSurface::invalidate_cached(
                self.key.map,
            );
        }
        let worked = self.manager.tick_batch(8) > 0;
        while let Some(result) = self.manager.receive() {
            match result {
                SceneManagerResult::Clear => self.index.clear(),
                SceneManagerResult::Chunk(chunk, ..) => self.index.index_chunk(&chunk),
                _ => (),
            }
        }
        worked
    }
    pub fn ready(&self) -> bool {
        !self.manager.is_busy()
    }
    pub fn snapshot(&self) -> Arc<SceneBuildSnapshot> {
        self.index.snapshot()
    }
}
