//! CPU-owned speech survives GPU replacement. The world renderer supplies its
//! actual map/camera; no game widget or raster framebuffer is manufactured.
use super::*;
use eldiron_presentation::{GameFonts, SpeechOverlay, screen::ContentRevision};
use eldiron_runtime::{RegionEvents, RuntimeView};
use std::hash::{Hash, Hasher};

#[derive(Default)]
pub(super) struct WorldSpeech {
    overlay: SpeechOverlay,
    fonts: Option<GameFonts>,
}
impl WorldSpeech {
    #[cfg(any(target_arch = "wasm32", feature = "staticlib"))]
    pub(super) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "messages": self.overlay.messages().len(),
            "visible": self.overlay.visible_bounds().count(),
            "layouts": self.overlay.layouts_built(),
            "fonts": self.fonts.is_some(),
        })
    }
    pub(super) fn receive(&mut self, world: &RuntimeView<'_>, events: &RegionEvents) {
        self.overlay.receive(world, events);
    }
    pub(super) fn clear(&mut self) {
        self.overlay.clear();
    }
    pub(super) fn invalidate_geometry(&mut self) {
        self.overlay.invalidate_geometry();
    }
    pub(super) fn paint(
        &mut self,
        world: &RuntimeView<'_>,
        binding: &Binding,
        map: &Map,
        assets: &rusterix::Assets,
        camera: Option<&dyn D3Camera>,
        list: &mut PaintList,
    ) {
        self.overlay.invalidate_geometry();
        if world.state == rusterix::ServerState::Off {
            self.clear();
            return;
        }
        if world.map.is_none_or(|runtime| runtime.id != map.id)
            || self.overlay.messages().is_empty()
        {
            return;
        }
        // Font edits are still raw authored assets. A precise fingerprint is
        // used until documents publish a dedicated font-content revision.
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        for font in world.fonts() {
            (font.id, font.name, font.bytes).hash(&mut hash);
        }
        let fonts = self.fonts.get_or_insert_with(GameFonts::default);
        fonts.refresh(world, ContentRevision(hash.finish()));
        let size = Vec2::new(binding.size.0 as f32, binding.size.1 as f32);
        let isometric = rusterix::isometric::Projection::editor(map, size);
        let vp = camera.map(|camera| {
            camera.projection_matrix(size.x, size.y)
                * camera.view_matrix_for_surface(size.x, size.y)
        });
        self.overlay.prepare_projected(
            world,
            binding.view.uuid,
            Rect::new(0., 0., size.x, size.y),
            fonts,
            |message| {
                let position =
                    rusterix::world_text::anchor_position(&message.target.anchor(), map, assets)?;
                let point = if binding.mode == EditorViewMode::D2 {
                    if map.camera == MapCamera::TwoDIso {
                        isometric.screen(Vec2::new(position.x, position.z), position.y)
                    } else {
                        Vec2::new(
                            position.x * map.grid_size + map.offset.x + size.x * 0.5,
                            position.z * map.grid_size - map.offset.y + size.y * 0.5
                                - map.grid_size,
                        )
                    }
                } else {
                    let c = vp? * Vec4::new(position.x, position.y, position.z, 1.);
                    if c.w <= 0. || (c.z / c.w).abs() > 1. {
                        return None;
                    }
                    Vec2::new(
                        (c.x / c.w * 0.5 + 0.5) * size.x,
                        (0.5 - c.y / c.w * 0.5) * size.y,
                    )
                };
                (point.x >= 0. && point.y >= 0. && point.x <= size.x && point.y <= size.y)
                    .then_some([point.x, point.y])
            },
        );
        self.overlay.paint_view(binding.view.uuid, list);
    }
}

impl NativeWorld {
    pub(in crate::editor::native_views) fn receive_speech(
        &mut self,
        world: &RuntimeView<'_>,
        events: &RegionEvents,
    ) {
        self.speech.receive(world, events);
    }
    pub(in crate::editor::native_views) fn clear_speech(&mut self) {
        self.speech.clear();
    }
}

#[cfg(test)]
impl Editor {
    pub(crate) fn native_world_speech_for_test(&self) -> (usize, usize, u64, bool) {
        let speech = &self.native_viewport.world.speech;
        (
            speech.overlay.messages().len(),
            speech.overlay.visible_bounds().count(),
            speech.overlay.layouts_built(),
            speech.fonts.is_some(),
        )
    }
    pub(crate) fn native_world_speech_projection_for_test(
        &mut self,
        mode: EditorViewMode,
        camera: MapCamera,
        center: Vec3<f32>,
    ) -> (EditorViewMode, MapCamera, Vec3<f32>) {
        let region = self
            .documents
            .project
            .get_region_ctx_mut(&self.server_ctx.shared)
            .unwrap();
        let old = (
            self.server_ctx.shared.editor_view_mode,
            region.map.camera,
            region.editing_position_3d,
        );
        self.server_ctx.shared.editor_view_mode = mode;
        region.map.camera = camera;
        region.editing_position_3d = center;
        old
    }
    pub(crate) fn queue_native_world_speech_for_test(&mut self) {
        let play = self.play_session.as_mut().unwrap();
        let mut bridge = self.server_ctx.reference.write();
        let bridge = &mut *bridge;
        let parts = play.parts(&mut bridge.server, &mut bridge.assets);
        let map = &parts.project.regions[0].map;
        let region = parts.server.region_id_map[&map.id];
        let entity = map.entities.first().unwrap().id;
        parts.server.says.entry(region).or_default().push((
            Some(entity),
            None,
            "World speech e\u{301} مرحبا 東京".into(),
            "warning".into(),
        ));
    }
}
