//! Native visibility consumes CPU line-of-sight coverage as pixel paint commands.
use super::*;

pub(super) fn paint(
    map: &Map,
    assets: &rusterix::Assets,
    size: (usize, usize),
    settings: &rusterix::render_settings::RenderSettings,
) -> Option<PaintOp> {
    if settings.visibility_range_2d <= 0. || settings.visibility_alpha_2d <= 0. {
        return None;
    }
    let visibility = rusterix::client::visibility::VisibilityView2D::editor(
        map,
        (size.0 as u32, size.1 as u32),
        settings.visibility_range_2d,
    );
    let mapmini = map.as_mini(&assets.blocking_tiles());
    let mut rects = Vec::new();
    visibility.visit_hidden_rects(&mapmini, |[x, y, width, height]| {
        rects.push(Rect::new(x as f32, y as f32, width as f32, height as f32));
    });
    let background = settings
        .background_color_2d
        .map(|v| (v.clamp(0., 1.) * 255.).round() / 255.);
    Some(PaintOp::PixelRects {
        rects: rects.into(),
        color: [
            background[0],
            background[1],
            background[2],
            settings.visibility_alpha_2d,
        ],
    })
}

#[cfg(test)]
impl Editor {
    pub(crate) fn set_native_visibility_for_test(&mut self) -> (String, f32, Vec2<f32>) {
        let original = self.documents.project.config.clone();
        let map = self
            .documents
            .project
            .get_map_mut(&self.server_ctx.shared)
            .unwrap();
        let pose = (map.grid_size, map.offset);
        map.grid_size = 43.5;
        map.offset += Vec2::new(0.25, -0.5);
        let mut config = original.parse::<toml_edit::DocumentMut>().unwrap();
        config["viewport"]["visibility_range_2d"] = toml_edit::value(3.25);
        config["viewport"]["visibility_alpha_2d"] = toml_edit::value(0.37);
        self.documents.project.config = config.to_string();
        let mut bridge = self.server_ctx.reference.write();
        bridge.assets.config = self.documents.project.config.clone();
        bridge
            .scene_handler
            .sync_base_render_settings(&self.documents.project.config);
        (original, pose.0, pose.1)
    }
    pub(crate) fn restore_native_visibility_for_test(
        &mut self,
        original: (String, f32, Vec2<f32>),
    ) {
        self.documents.project.config = original.0;
        let map = self
            .documents
            .project
            .get_map_mut(&self.server_ctx.shared)
            .unwrap();
        map.grid_size = original.1;
        map.offset = original.2;
        let mut bridge = self.server_ctx.reference.write();
        bridge.assets.config = self.documents.project.config.clone();
        bridge
            .scene_handler
            .sync_base_render_settings(&self.documents.project.config);
    }
    pub(crate) fn native_visibility_plans_for_test(&self) -> (PaintList, PaintList, (u32, u32)) {
        let world = &self.native_viewport.world;
        let list = &world.published.as_ref().unwrap().1;
        let binding = world.binding.as_ref().unwrap();
        let mut base = PaintList::default();
        base.ops.push(PaintOp::DisplayCompositing(
            theframework::paint::DisplayCompositing::LegacyOpaque,
        ));
        base.ops.push(list.ops[0].clone());
        let mut masked = base.clone();
        assert!(matches!(&list.ops[1], PaintOp::PixelRects { rects, .. } if !rects.is_empty()));
        masked.ops.push(list.ops[1].clone());
        (base, masked, (binding.size.0 as u32, binding.size.1 as u32))
    }
    pub(crate) fn native_visibility_reference_for_test(&self, pixels: &mut [u8], size: (u32, u32)) {
        let bridge = self.server_ctx.reference.read();
        let map = self.documents.project.get_map(&self.server_ctx.shared).unwrap();
        assert_eq!(map.camera, MapCamera::TwoD);
        let gpu = self.native_viewport.world.gpu.as_ref().unwrap();
        let settings = &gpu.scenes.scene(gpu.view).unwrap().settings;
        let visibility = rusterix::client::visibility::VisibilityView2D::editor(
            map,
            size,
            settings.visibility_range_2d,
        );
        let mini = map.as_mini(&bridge.assets.blocking_tiles());
        rusterix::client::apply_2d_visibility_mask(
            pixels,
            size.0 as usize,
            size.1 as usize,
            &mini,
            visibility.grid_size,
            visibility.top_left,
            visibility.player_pos,
            visibility.range,
            settings.visibility_alpha_2d,
            settings
                .background_color_2d
                .map(|v| (v.clamp(0., 1.) * 255.).round() as u8),
        );
    }
}
