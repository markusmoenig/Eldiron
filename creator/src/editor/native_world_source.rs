//! Application-owned immutable static geometry for the main authoring view.
//! Camera/layout changes keep this CPU publication; device replacement does not
//! restart its jobs. Reference previews retain their separate migration worker.
use super::*;

#[derive(Clone)]
struct Key {
    document: Uuid,
    runtime: Option<Uuid>,
    context: ProjectContext,
    map: Uuid,
    changed: u32,
    camera: MapCamera,
    three_d: bool,
    config: String,
    assets: AssetKey,
}
impl Key {
    fn same_source(&self, other: &Self) -> bool {
        self.document == other.document
            && self.runtime == other.runtime
            && self.context == other.context
            && self.map == other.map
            && self.changed == other.changed
            && self.camera == other.camera
            && self.three_d == other.three_d
            && self.config == other.config
            && self.same_catalog(&other.assets)
    }
    fn same_catalog(&self, assets: &AssetKey) -> bool {
        // Animated/equipment avatar cache publications do not change the
        // manager's static tile/prop/palette inputs. GPU input scopes still
        // track that revision separately through the complete view binding.
        self.assets.source == assets.source
            && self.assets.prefabs == assets.prefabs
            && self.assets.same_atlas(assets)
    }
}
pub(super) struct Publication {
    key: Key,
    source: Option<crate::scene_source::Source>,
    empty: Arc<SceneBuildSnapshot>,
}
impl Publication {
    pub(super) fn matches(&self, binding: &Binding, map: &Map) -> bool {
        self.key.document == binding.document
            && self.key.runtime == binding.runtime
            && self.key.context == binding.context
            && self.key.map == map.id
            && self.key.changed == map.changed
            && self.key.camera == map.camera
            && self.key.three_d == (binding.mode != EditorViewMode::D2)
            && self.key.config == binding.geometry_config
            && self.key.same_catalog(&binding.assets)
    }
    pub(super) fn ready(&self) -> bool {
        self.source.as_ref().is_none_or(|source| source.ready())
    }
    pub(super) fn snapshot(&self) -> Arc<SceneBuildSnapshot> {
        self.source
            .as_ref()
            .map_or_else(|| self.empty.clone(), |source| source.snapshot())
    }
    pub(super) fn billboards(&self) -> FxHashMap<scenevm::GeoId, rusterix::BillboardMetadata> {
        self.source
            .as_ref()
            .map(|source| source.billboards())
            .unwrap_or_default()
    }
}
impl Editor {
    pub(super) fn native_world_asset_key(&self, assets: &rusterix::Assets) -> AssetKey {
        AssetKey {
            source: assets.visual_source_revision,
            avatars: assets.avatars.revision(),
            tiles: assets.tiles_revision,
            prefabs: assets.block_props_revision,
            config: assets.config.clone(),
            palette: assets.palette.clone(),
            material: self.documents.project.procedural_materials.revision(),
            appearance: self.documents.project.procedural_appearances.revision(),
            geometry: self.procedural_appearance_revision,
        }
    }
    pub(super) fn native_world_geometry_config(&self) -> String {
        if self.server_ctx.editor_view_mode == EditorViewMode::D2 || !self.server_ctx.pc.is_region()
        {
            return String::new();
        }
        self.server_ctx
            .pc
            .id()
            .and_then(|id| self.documents.project.get_region(&id))
            .map(|region| region.config.clone())
            .unwrap_or_default()
    }
    pub(crate) fn advance_native_world_source(&mut self) -> bool {
        if !self.native_world_owns_viewport() {
            return self.native_viewport.world.source.take().is_some();
        }
        let Some(document) = self.documents.active_id() else {
            return false;
        };
        let Some(map) = self.documents.project.get_map(&self.server_ctx) else {
            return false;
        };
        let bridge = RUSTERIX.read().unwrap();
        let assets = self
            .play_session
            .as_ref()
            .map(|play| play.assets(&bridge.assets))
            .unwrap_or(&bridge.assets);
        let key = Key {
            document,
            runtime: self
                .play_session
                .as_ref()
                .map(|play| play.pump().session_id()),
            context: self.server_ctx.pc,
            map: map.id,
            changed: map.changed,
            camera: map.camera,
            three_d: self.server_ctx.editor_view_mode != EditorViewMode::D2,
            config: self.native_world_geometry_config(),
            assets: self.native_world_asset_key(assets),
        };
        let publication = &mut self.native_viewport.world.source;
        let changed = publication
            .as_ref()
            .is_none_or(|old| !old.key.same_source(&key));
        if changed {
            let source = if !key.three_d && map.camera == MapCamera::TwoDIso {
                // Isometric geometry has a direct GPU producer, with no hidden
                // chunk-builder dependency just to make its view ready.
                None
            } else {
                let mut projection = map.clone();
                if key.three_d && key.context.is_region() {
                    crate::utils::filter_iso_placements_for_3d(
                        &mut projection,
                        &self.documents.project.tiles,
                    );
                    crate::utils::apply_region_config(&mut projection, key.config.clone());
                }
                Some(crate::scene_source::Source::for_editor(
                    crate::scene_source::SourceKey {
                        document,
                        map: map.id,
                        source: map.changed as u64,
                        assets: assets.visual_source_revision,
                    },
                    projection,
                    assets,
                ))
            };
            *publication = Some(Publication {
                key,
                source,
                empty: rusterix::scene_build_index::SceneBuildIndex::default().snapshot(),
            });
        }
        changed
            | publication
                .as_mut()
                .and_then(|publication| publication.source.as_mut())
                .is_some_and(|source| source.advance())
    }
}
