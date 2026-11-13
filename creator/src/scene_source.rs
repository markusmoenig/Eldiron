//! Bounded CPU geometry publication for auxiliary authoring views. Each owner
//! keeps its own source identity; no display VM or application worker is lent.
use crate::prelude::*;
use rusterix::scene_build_index::SceneBuildSnapshot;
use rusterix::{SceneManager, SceneManagerResult, scene_build_index::SceneBuildIndex};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct SourceKey {
    pub document: Uuid,
    pub map: Uuid,
    pub source: u64,
    pub assets: u64,
}
pub(crate) struct Source {
    pub key: SourceKey,
    manager: SceneManager,
    index: SceneBuildIndex,
    billboards: FxHashMap<(i32, i32), Vec<rusterix::BillboardMetadata>>,
}
impl Source {
    pub fn new(key: SourceKey, map: Map, assets: &rusterix::Assets) -> Self {
        Self::build(key, map, assets, false)
    }
    pub fn for_editor(key: SourceKey, map: Map, assets: &rusterix::Assets) -> Self {
        Self::build(key, map, assets, true)
    }
    fn build(key: SourceKey, map: Map, assets: &rusterix::Assets, preview_filters: bool) -> Self {
        let mut manager = SceneManager::new();
        manager.set_apply_preview_filters(preview_filters);
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
            billboards: Default::default(),
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
                SceneManagerResult::Clear => {
                    self.index.clear();
                    self.billboards.clear();
                }
                SceneManagerResult::Chunk(chunk, _, _, billboards) => {
                    self.billboards
                        .insert((chunk.origin.x, chunk.origin.y), billboards);
                    self.index.index_chunk(&chunk);
                }
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
    pub fn billboards(&self) -> FxHashMap<scenevm::GeoId, rusterix::BillboardMetadata> {
        self.billboards
            .values()
            .flatten()
            .map(|billboard| (billboard.geo_id, billboard.clone()))
            .collect()
    }
}
