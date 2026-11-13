//! Ephemeral native paste projection. Only the ordinary authoring dispatcher
//! may commit it; this source never enters Documents, undo, or serialized data.
use super::*;
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq)]
pub(super) struct Stamp {
    clipboard: u64,
    target: Option<[u32; 2]>,
}
impl Stamp {
    fn target(server: &ServerContext) -> Option<Vec2<f32>> {
        if server.editor_view_mode == EditorViewMode::D2 {
            server.hover_cursor
        } else {
            server
                .hover_cursor_3d
                .or_else(|| server.geo_hit.map(|_| server.geo_hit_pos))
                .map(|point| Vec2::new(point.x, point.z))
        }
    }
    #[cfg(test)]
    pub(super) fn has_target(&self) -> bool {
        self.target.is_some()
    }
    pub(super) fn current(server: &ServerContext) -> Option<Self> {
        let clipboard = server.paste_clipboard.as_ref()?;
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_vec(clipboard)
            .expect("serializable clipboard")
            .hash(&mut hash);
        Some(Self {
            clipboard: hash.finish(),
            target: Self::target(server).map(|p| [p.x.to_bits(), p.y.to_bits()]),
        })
    }
}
#[derive(Clone, PartialEq)]
struct Key {
    document: Uuid,
    mode: EditorViewMode,
    custom: bool,
    context: ProjectContext,
    runtime: Option<Uuid>,
    config: String,
    authored: SourceStamp,
    stamp: Stamp,
    assets: AssetKey,
}
pub(super) struct Preview {
    key: Key,
    pub map: Arc<Map>,
    source: Option<crate::scene_source::Source>,
    empty: Arc<SceneBuildSnapshot>,
    pub ignored: Arc<FxHashSet<scenevm::GeoId>>,
}
impl Preview {
    pub(super) fn matches(&self, binding: &Binding, authored: &Map) -> bool {
        self.key.document == binding.document
            && self.key.mode == binding.mode
            && self.key.custom == binding.custom.is_some()
            && self.key.context == binding.context
            && self.key.runtime == binding.runtime
            && self.key.config == binding.geometry_config
            && self.key.assets == binding.assets
            && self.key.authored == SourceStamp::new(authored)
            && binding.paste.as_ref() == Some(&self.key.stamp)
    }
    pub(super) fn ready(&self) -> bool {
        self.source.as_ref().is_none_or(|source| source.ready())
    }
    pub(super) fn snapshot(&self) -> Option<Arc<SceneBuildSnapshot>> {
        self.source
            .as_ref()
            .map(|source| source.snapshot())
            .or_else(|| self.key.custom.then(|| self.empty.clone()))
    }
    pub(super) fn billboards(&self) -> FxHashMap<scenevm::GeoId, rusterix::BillboardMetadata> {
        self.source
            .as_ref()
            .map(|source| source.billboards())
            .unwrap_or_default()
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
        let Some(authored) = scope::map(&self.documents.project, &self.server_ctx) else {
            return false;
        };
        let bridge = self.server_ctx.reference.read();
        let assets = self
            .play_session
            .as_ref()
            .map(|play| play.assets(&bridge.assets))
            .unwrap_or(&bridge.assets);
        let key = Key {
            document: self.documents.active_id().expect("paste document"),
            mode: self.server_ctx.editor_view_mode,
            custom: custom::Description::current(authored, &self.server_ctx).is_some(),
            context: self.server_ctx.pc,
            runtime: self
                .play_session
                .as_ref()
                .map(|play| play.pump().session_id()),
            config: self.native_world_geometry_config(),
            authored: SourceStamp::new(authored),
            stamp,
            assets: self.native_world_asset_key(assets),
        };
        let preview = &self.native_viewport.world.paste;
        let changed = preview.as_ref().is_none_or(|old| old.key != key);
        if changed {
            rusterix::chunkbuilder::terrain_surface::TerrainSurface::invalidate_cached(authored.id);
            let mut map = authored.clone();
            if let Some(point) = key
                .stamp
                .target
                .map(|point| Vec2::new(f32::from_bits(point[0]), f32::from_bits(point[1])))
            {
                map.paste_at_position(self.server_ctx.paste_clipboard.as_ref().unwrap(), point);
            }
            let mut ignored = FxHashSet::default();
            ignored.extend(
                map.vertices[authored.vertices.len()..]
                    .iter()
                    .map(|v| scenevm::GeoId::Vertex(v.id)),
            );
            ignored.extend(
                map.linedefs[authored.linedefs.len()..]
                    .iter()
                    .map(|v| scenevm::GeoId::Linedef(v.id)),
            );
            ignored.extend(
                map.sectors[authored.sectors.len()..]
                    .iter()
                    .map(|v| scenevm::GeoId::Sector(v.id)),
            );
            ignored.extend(
                map.geometry_objects[authored.geometry_objects.len()..]
                    .iter()
                    .map(|v| scenevm::GeoId::GeometryObject(v.id)),
            );
            // Dedicated custom/isometric producers bypass CPU publication. A
            // 3D editor camera still needs its ordinary filtered scene source.
            let mut geometry = map.clone();
            self.project_native_world_geometry(&mut geometry);
            let source = (!key.custom
                && (key.mode != EditorViewMode::D2 || map.camera != MapCamera::TwoDIso))
                .then(|| {
                    let source_key = crate::scene_source::SourceKey {
                        document: key.document,
                        map: map.id,
                        source: key.stamp.clipboard,
                        assets: key.assets.source,
                    };
                    if key.mode == EditorViewMode::D2 {
                        crate::scene_source::Source::new(source_key, geometry, assets)
                    } else {
                        crate::scene_source::Source::for_editor(source_key, geometry, assets)
                    }
                });
            self.native_viewport.world.paste = Some(Preview {
                key,
                map: Arc::new(map),
                source,
                ignored: Arc::new(ignored),
                empty: rusterix::scene_build_index::SceneBuildIndex::default().snapshot(),
            });
        }
        changed
            | self
                .native_viewport
                .world
                .paste
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
        if !focused
            || ui.dialog.is_some()
            || ui.popover.is_some()
            || ui.context_menu.is_some()
            || ctx.ui.overlay.is_some()
        {
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
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        let current_camera = self
            .native_world_camera()
            .map(|camera| CameraStamp::new(camera.as_ref(), binding.size));
        let old_target = Stamp::target(&self.server_ctx);
        if binding.mode == EditorViewMode::D2 {
            self.server_ctx.hover_cursor = Some(self.server_ctx.local_to_map_grid(
                Vec2::new(binding.size.0 as f32, binding.size.1 as f32),
                Vec2::new(at.x as f32, at.y as f32),
                map,
                map.subdivisions,
            ));
            self.server_ctx.hover_cursor_3d = None;
            self.server_ctx.hover_ray_origin_3d = None;
            self.server_ctx.hover_ray_dir_3d = None;
        } else {
            let world = &mut self.native_viewport.world;
            let Some(frame) = world.presented.as_ref().filter(|frame| {
                frame.binding.capture_compatible(binding)
                    && frame.stamp == SourceStamp::new(map)
                    && frame.camera == current_camera
            }) else {
                ctx.ui.redraw_all = true;
                return true;
            };
            let ignored = frame.paste_ignored.clone();
            let Some(pick) = world.gpu.as_mut().and_then(|gpu| {
                gpu.scenes.pick_editor_surface_filtered(
                    frame.output,
                    [
                        at.x as f32 / binding.size.0 as f32,
                        at.y as f32 / binding.size.1 as f32,
                    ],
                    |geo| {
                        ignored
                            .as_ref()
                            .is_none_or(|ignored| accepts_surface(ignored, geo))
                    },
                )
            }) else {
                ctx.ui.redraw_all = true;
                return true;
            };
            let Some(position) = surface_target(&pick) else {
                return true;
            };
            self.server_ctx.hover_cursor = None;
            self.server_ctx.hover_cursor_3d = Some(position);
            self.server_ctx.hover_ray_origin_3d = pick.ray.map(|(origin, _)| origin);
            self.server_ctx.hover_ray_dir_3d = pick.ray.map(|(_, direction)| direction);
        }
        self.server_ctx.hover = (None, None, None);
        self.server_ctx.geo_hit = None;
        self.server_ctx.hover_surface = None;
        self.server_ctx.hover_surface_hit_pos = None;
        self.server_ctx.hover_surface_normal = None;
        let moved = old_target != Stamp::target(&self.server_ctx);
        ctx.ui.redraw_all |= moved;
        if !matches!(input, MapEvent::MapClicked(_)) {
            return true;
        }
        // A click at a newly moved target first publishes that preview. It
        // cannot commit a shape that has never appeared in the displayed view.
        let world = &self.native_viewport.world;
        let accepted = !moved
            && world.presented.as_ref().is_some_and(|frame| {
                frame.binding == *binding && world.frame_is_fresh(frame, map, &current_camera)
            });
        ctx.ui.redraw_all |= !accepted;
        !accepted
    }
}

fn accepts_surface(ignored: &FxHashSet<scenevm::GeoId>, geo: scenevm::GeoId) -> bool {
    use scenevm::GeoId;
    !ignored.contains(&geo)
        && match geo {
            GeoId::Hole(sector, _) => !ignored.contains(&GeoId::Sector(sector)),
            _ => true,
        }
}
fn surface_target(pick: &eldiron_presentation::EditorSurfacePick) -> Option<Vec3<f32>> {
    let point = if let Some((_, point, _, _)) = pick.base {
        point
    } else {
        let (origin, direction) = pick.ray?;
        if direction.y.abs() < 1e-6 {
            return None;
        }
        let distance = -origin.y / direction.y;
        if !distance.is_finite() || distance <= 0. {
            return None;
        }
        origin + direction * distance
    };
    if [point.x, point.y, point.z].iter().any(|v| !v.is_finite()) {
        return None;
    }
    let depth = (point - pick.camera.pos).dot(pick.camera.forward);
    (depth.is_finite() && depth >= pick.camera.near && depth <= pick.camera.far).then_some(point)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paste_surface_target_uses_displayed_surface_or_bounded_forward_ground() {
        let mut pick = eldiron_presentation::EditorSurfacePick {
            camera: scenevm::Camera3D::builder()
                .with_pos(Vec3::new(0., 4., 5.))
                .with_basis(Vec3::new(0., -1., -1.), Vec3::unit_y()),
            ray: Some((Vec3::new(0., 4., 5.), Vec3::new(0., -1., -1.).normalized())),
            base: None,
        };
        let ground = surface_target(&pick).unwrap();
        assert!((ground - Vec3::new(0., 0., 1.)).magnitude() < 1e-5);
        let surface = Vec3::new(0., 2., 3.);
        pick.base = Some((scenevm::GeoId::Sector(1), surface, 2., Vec3::unit_y()));
        assert_eq!(surface_target(&pick), Some(surface));
        pick.base = None;
        for direction in [
            Vec3::unit_x(),
            Vec3::unit_y(),
            Vec3::new(f32::NAN, -1., -1.),
        ] {
            pick.ray = Some((pick.camera.pos, direction));
            assert!(surface_target(&pick).is_none());
        }
        pick.ray = Some((pick.camera.pos, Vec3::new(0., -1e-5, -1.)));
        assert!(
            surface_target(&pick).is_none(),
            "horizon targets beyond far clip cannot expand the source"
        );
        pick.base = Some((
            scenevm::GeoId::Sector(1),
            Vec3::new(0., 6., 7.),
            2.,
            Vec3::unit_y(),
        ));
        assert!(
            surface_target(&pick).is_none(),
            "behind-camera hits cannot become paste targets"
        );
    }
}
