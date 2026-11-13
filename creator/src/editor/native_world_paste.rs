//! Ephemeral native 2D paste projection. Only the ordinary authoring dispatcher
//! may commit it; this source never enters Documents, undo, or serialized data.
use super::*;
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq)]
pub(super) struct Stamp {
    clipboard: u64,
    target: Option<[u32; 2]>,
}
impl Stamp {
    pub(super) fn current(server: &ServerContext) -> Option<Self> {
        let clipboard = server.paste_clipboard.as_ref()?;
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_vec(clipboard)
            .expect("serializable clipboard")
            .hash(&mut hash);
        Some(Self {
            clipboard: hash.finish(),
            target: server.hover_cursor.map(|p| [p.x.to_bits(), p.y.to_bits()]),
        })
    }
}
#[derive(Clone, PartialEq)]
struct Key {
    document: Uuid,
    authored: SourceStamp,
    stamp: Stamp,
    assets: u64,
}
pub(super) struct Preview {
    key: Key,
    pub map: Arc<Map>,
    source: Option<crate::scene_source::Source>,
}
impl Preview {
    pub(super) fn matches(&self, binding: &Binding, authored: &Map) -> bool {
        self.key.document == binding.document
            && self.key.authored == SourceStamp::new(authored)
            && binding.paste.as_ref() == Some(&self.key.stamp)
    }
    pub(super) fn ready(&self) -> bool {
        self.source.as_ref().is_none_or(|source| source.ready())
    }
    pub(super) fn snapshot(&self) -> Option<Arc<SceneBuildSnapshot>> {
        self.source.as_ref().map(|source| source.snapshot())
    }
    pub(super) fn billboards(&self) -> FxHashMap<scenevm::GeoId, rusterix::BillboardMetadata> {
        self.source.as_ref().map(|source| source.billboards()).unwrap_or_default()
    }
}
impl Editor {
    pub(crate) fn advance_native_world_paste(&mut self) -> bool {
        let stamp = self
            .native_world_owns_viewport()
            .then(|| Stamp::current(&self.server_ctx))
            .flatten();
        let Some(stamp) = stamp else {
            return self.native_viewport.world.paste.take().is_some();
        };
        let Some(authored) = self.documents.project.get_map(&self.server_ctx) else {
            return false;
        };
        let bridge = RUSTERIX.read().unwrap();
        let assets = self
            .play_session
            .as_ref()
            .map(|play| play.assets(&bridge.assets))
            .unwrap_or(&bridge.assets);
        let mut asset_key = std::collections::hash_map::DefaultHasher::new();
        self.native_minimap_asset_key(assets).hash(&mut asset_key);
        assets.visual_source_revision.hash(&mut asset_key);
        let key = Key {
            document: self.documents.active_id().expect("paste document"),
            authored: SourceStamp::new(authored),
            stamp,
            assets: asset_key.finish(),
        };
        let preview = &mut self.native_viewport.world.paste;
        let changed = preview.as_ref().is_none_or(|old| old.key != key);
        if changed {
            rusterix::chunkbuilder::terrain_surface::TerrainSurface::invalidate_cached(authored.id);
            let mut map = authored.clone();
            if let Some(point) = self.server_ctx.hover_cursor {
                map.paste_at_position(self.server_ctx.paste_clipboard.as_ref().unwrap(), point);
            }
            // Isometric sources are built directly by the shared GPU producer.
            // Ordinary 2D chunks get their own bounded CPU publication, leaving
            // the main authored geometry worker and its query source untouched.
            let source = (map.camera != MapCamera::TwoDIso).then(|| {
                crate::scene_source::Source::new(
                    crate::scene_source::SourceKey {
                        document: key.document,
                        map: map.id,
                        source: key.stamp.clipboard,
                        assets: key.assets,
                    },
                    map.clone(),
                    assets,
                )
            });
            *preview = Some(Preview {
                key,
                map: Arc::new(map),
                source,
            });
        }
        changed
            | preview
                .as_mut()
                .and_then(|preview| preview.source.as_mut())
                .is_some_and(|source| source.advance())
    }
    pub(super) fn handle_native_paste_input(
        &mut self,
        binding: &Binding,
        input: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        focused: bool,
    ) -> bool {
        if !focused || ui.dialog.is_some() || ui.popover.is_some() {
            return true;
        }
        if matches!(input, MapEvent::MapEscape) {
            return false;
        }
        let at = match input {
            MapEvent::MapHover(at) | MapEvent::MapDragged(at) | MapEvent::MapClicked(at) => at,
            _ => return true,
        };
        if at.y <= 20
            || at.x < 0
            || at.y < 0
            || at.x as usize >= binding.size.0
            || at.y as usize >= binding.size.1
        {
            return true;
        }
        let map = self.documents.project.get_map(&self.server_ctx).unwrap();
        let position = self.server_ctx.local_to_map_grid(
            Vec2::new(binding.size.0 as f32, binding.size.1 as f32),
            Vec2::new(at.x as f32, at.y as f32),
            map,
            map.subdivisions,
        );
        let moved = self.server_ctx.hover_cursor != Some(position);
        self.server_ctx.hover_cursor = Some(position);
        self.server_ctx.hover = (None, None, None);
        self.server_ctx.geo_hit = None;
        self.server_ctx.hover_ray_origin_3d = None;
        self.server_ctx.hover_ray_dir_3d = None;
        ctx.ui.redraw_all |= moved;
        if !matches!(input, MapEvent::MapClicked(_)) {
            return true;
        }
        // A click at a newly moved target first publishes that preview. It
        // cannot commit a shape that has never appeared in the displayed view.
        let world = &self.native_viewport.world;
        let accepted = !moved
            && world.presented.as_ref().is_some_and(|frame| {
                frame.binding == *binding && world.frame_is_fresh(frame, map, &None)
            });
        ctx.ui.redraw_all |= !accepted;
        !accepted
    }
}
