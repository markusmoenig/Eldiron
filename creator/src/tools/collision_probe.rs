use crate::{editor::RUSTERIX, prelude::*};
use MapEvent::*;
use rusterix::{
    Assets, CollisionProbeStepKind, CollisionWorld,
    chunkbuilder::{ChunkBuilder, d3chunkbuilder::D3ChunkBuilder},
};
use theframework::prelude::*;

pub struct CollisionProbeTool {
    id: TheId,
}

impl CollisionProbeTool {
    const RADIUS: f32 = 0.49;
    const MAX_STEP_HEIGHT: f32 = 1.0;

    fn screen_to_map(ui: &mut TheUI, map: &Map, coord: Vec2<i32>) -> Option<Vec2<f32>> {
        ui.get_render_view("PolyView").map(|render_view| {
            let dim = *render_view.dim();
            let grid_space_pos = Vec2::new(coord.x as f32, coord.y as f32)
                - Vec2::new(dim.width as f32, dim.height as f32) / 2.0
                - Vec2::new(map.offset.x, -map.offset.y);
            grid_space_pos / map.grid_size
        })
    }

    fn map_to_screen(map: &Map, dim: TheDim, pos: Vec2<f32>) -> Vec2<i32> {
        let screen = pos * map.grid_size
            + Vec2::new(dim.width as f32, dim.height as f32) / 2.0
            + Vec2::new(map.offset.x, -map.offset.y);
        screen.map(|v| v.round() as i32)
    }

    fn pointer_position(
        ui: &mut TheUI,
        server_ctx: &ServerContext,
        map: &Map,
        coord: Vec2<i32>,
        plane_y: Option<f32>,
    ) -> Option<Vec3<f32>> {
        if server_ctx.editor_view_mode != EditorViewMode::D2 {
            if let Some(y) = plane_y
                && let Some(hit) = Self::ray_plane_position(server_ctx, y)
                && Self::point_inside_probe_bounds(map, hit)
            {
                return Some(hit);
            }

            if let Some(hit) = server_ctx.hover_cursor_3d {
                return Some(hit);
            }
            if server_ctx.geo_hit.is_some() {
                return Some(server_ctx.geo_hit_pos);
            }
        }

        let pos = Self::screen_to_map(ui, map, coord)?;
        Some(Vec3::new(pos.x, 0.0, pos.y))
    }

    fn point_inside_probe_bounds(map: &Map, point: Vec3<f32>) -> bool {
        if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
            return false;
        }
        let bbox = map.bbox();
        let pad = 10.0;
        point.x >= bbox.min.x - pad
            && point.x <= bbox.max.x + pad
            && point.z >= bbox.min.y - pad
            && point.z <= bbox.max.y + pad
    }

    fn ray_plane_position(server_ctx: &ServerContext, y: f32) -> Option<Vec3<f32>> {
        let origin = server_ctx.hover_ray_origin_3d?;
        let dir = server_ctx.hover_ray_dir_3d?;
        if dir.y.abs() <= 1e-6 {
            return None;
        }
        let t = (y - origin.y) / dir.y;
        if !t.is_finite() || !(0.0..=250.0).contains(&t) {
            return None;
        }
        Some(origin + dir * t)
    }

    fn build_collision_world(map: &Map) -> CollisionWorld {
        let assets = RUSTERIX.read().unwrap().assets.clone();
        let mut world = CollisionWorld::default();
        let mut chunk_builder = D3ChunkBuilder::new();
        let chunk_size = 10;

        if map.vertices.is_empty() && map.geometry_objects.is_empty() {
            return world;
        }

        let bbox = map.bbox();
        let min_chunk = Vec2::new(
            (bbox.min.x / chunk_size as f32).floor() as i32,
            (bbox.min.y / chunk_size as f32).floor() as i32,
        );
        let max_chunk = Vec2::new(
            (bbox.max.x / chunk_size as f32).floor() as i32,
            (bbox.max.y / chunk_size as f32).floor() as i32,
        );

        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let chunk_origin = Vec2::new(cx, cy);
                let collision =
                    chunk_builder.build_collision(map, &assets, chunk_origin, chunk_size);
                world.update_chunk(chunk_origin, collision);
            }
        }

        world
    }

    fn update_probe_from_start(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &Map,
        server_ctx: &mut ServerContext,
        start: Vec3<f32>,
        coord: Vec2<i32>,
    ) -> Option<Vec3<f32>> {
        let Some(target) = Self::pointer_position(ui, server_ctx, map, coord, Some(start.y)) else {
            return None;
        };

        let world = Self::build_collision_world(map);
        let start_2d = Vec2::new(start.x, start.z);
        let target_2d = Vec2::new(target.x, target.z);
        let reference_y = world
            .get_floor_height_nearest(start_2d, start.y)
            .unwrap_or(start.y);
        let result = world.probe_path_direct(
            start_2d,
            target_2d,
            Self::RADIUS,
            Self::MAX_STEP_HEIGHT,
            reference_y,
        );

        server_ctx.collision_probe_target = Some(Vec3::new(target.x, reference_y, target.z));
        let target_y = result
            .steps
            .last()
            .map(|step| step.to.y)
            .unwrap_or(reference_y);
        server_ctx.collision_probe_result = Some(result);
        RUSTERIX.write().unwrap().set_overlay_dirty();
        ctx.ui.redraw_all = true;
        Some(Vec3::new(target.x, target_y, target.z))
    }

    fn update_probe(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &Map,
        server_ctx: &mut ServerContext,
        coord: Vec2<i32>,
    ) -> Option<Vec3<f32>> {
        let start = server_ctx
            .collision_probe_points
            .last()
            .copied()
            .or(server_ctx.collision_probe_start)?;
        server_ctx.collision_probe_start = Some(start);
        self.update_probe_from_start(ui, ctx, map, server_ctx, start, coord)
    }

    fn clear_probe(server_ctx: &mut ServerContext) {
        server_ctx.collision_probe_start = None;
        server_ctx.collision_probe_target = None;
        server_ctx.collision_probe_result = None;
        server_ctx.collision_probe_results.clear();
        server_ctx.collision_probe_points.clear();
        server_ctx.collision_probe_dragging = false;
    }

    fn finish_active_polyline(server_ctx: &mut ServerContext) {
        server_ctx.collision_probe_start = None;
        server_ctx.collision_probe_target = None;
        server_ctx.collision_probe_result = None;
        server_ctx.collision_probe_points.clear();
        server_ctx.collision_probe_dragging = false;
    }

    fn draw_marker(buffer: &mut TheRGBABuffer, pos: Vec2<i32>, color: [u8; 4]) {
        buffer.draw_line(pos.x - 3, pos.y, pos.x + 3, pos.y, color);
        buffer.draw_line(pos.x, pos.y - 3, pos.x, pos.y + 3, color);
    }

    fn draw_contrast_line(buffer: &mut TheRGBABuffer, a: Vec2<i32>, b: Vec2<i32>, color: [u8; 4]) {
        let shadow = [5, 8, 12, color[3].saturating_sub(25)];
        for ox in -2..=2 {
            for oy in -2..=2 {
                if ox == 0 && oy == 0 {
                    continue;
                }
                if ox * ox + oy * oy <= 5 {
                    buffer.draw_line(a.x + ox, a.y + oy, b.x + ox, b.y + oy, shadow);
                }
            }
        }
        buffer.draw_line(a.x, a.y, b.x, b.y, color);
        buffer.draw_line(a.x + 1, a.y, b.x + 1, b.y, color);
        buffer.draw_line(a.x, a.y + 1, b.x, b.y + 1, color);
    }

    fn offset_screen_segment(a: Vec2<i32>, b: Vec2<i32>, amount: f32) -> (Vec2<i32>, Vec2<i32>) {
        let delta = Vec2::new((b.x - a.x) as f32, (b.y - a.y) as f32);
        let len = delta.magnitude();
        if len <= 0.001 {
            return (a, b);
        }
        let n = Vec2::new(-delta.y / len, delta.x / len) * amount;
        let o = n.map(|v| v.round() as i32);
        (a + o, b + o)
    }

    fn goto_path_points(result: &rusterix::CollisionProbeResult) -> Vec<Vec3<f32>> {
        if result.goto_path.len() >= 2 {
            result.goto_path.clone()
        } else if result.goto_path_found || result.arrived {
            let fallback_end = result
                .steps
                .last()
                .map(|step| step.to)
                .unwrap_or_else(|| Vec3::new(result.target.x, result.start.y, result.target.y));
            vec![result.start, fallback_end]
        } else {
            Vec::new()
        }
    }

    fn project_3d_to_screen(point: Vec3<f32>, dim: TheDim) -> Option<Vec2<i32>> {
        if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
            return None;
        }
        let rusterix = RUSTERIX.read().ok()?;
        let view = rusterix.client.camera_d3.view_matrix();
        let proj = rusterix
            .client
            .camera_d3
            .projection_matrix(dim.width as f32, dim.height as f32);
        let clip = proj * view * Vec4::new(point.x, point.y, point.z, 1.0);
        if clip.w <= 0.0 || !clip.w.is_finite() {
            return None;
        }
        let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
        if !ndc.x.is_finite()
            || !ndc.y.is_finite()
            || !ndc.z.is_finite()
            || ndc.z < -1.0
            || ndc.z > 1.0
        {
            return None;
        }
        Some(Vec2::new(
            ((ndc.x * 0.5 + 0.5) * dim.width as f32).round() as i32,
            ((1.0 - (ndc.y * 0.5 + 0.5)) * dim.height as f32).round() as i32,
        ))
    }

    fn draw_goto_route_3d_screen(
        buffer: &mut TheRGBABuffer,
        dim: TheDim,
        result: &rusterix::CollisionProbeResult,
        faded: bool,
    ) {
        let color = [244, 114, 255, if faded { 190 } else { 255 }];
        let points = Self::goto_path_points(result);
        for segment in points.windows(2) {
            let Some(a) = Self::project_3d_to_screen(segment[0] + Vec3::new(0.0, 0.18, 0.0), dim)
            else {
                continue;
            };
            let Some(b) = Self::project_3d_to_screen(segment[1] + Vec3::new(0.0, 0.18, 0.0), dim)
            else {
                continue;
            };
            if (Vec2::new((b.x - a.x) as f32, (b.y - a.y) as f32)).magnitude() > 1600.0 {
                continue;
            }
            let (a, b) = Self::offset_screen_segment(a, b, 7.0);
            Self::draw_contrast_line(buffer, a, b, color);
        }
    }

    fn draw_result_2d(
        buffer: &mut TheRGBABuffer,
        map: &Map,
        dim: TheDim,
        result: &rusterix::CollisionProbeResult,
        faded: bool,
    ) {
        let alpha = if faded { 150 } else { 235 };
        let green = [74, 222, 128, alpha];
        let yellow = [250, 204, 21, alpha];
        let orange = [251, 146, 60, alpha];
        let magenta = [244, 114, 255, if faded { 135 } else { 235 }];
        let blue = [96, 165, 250, if faded { 125 } else { 210 }];
        let red = [248, 113, 113, alpha];
        let gray = [203, 213, 225, if faded { 105 } else { 185 }];

        for step in &result.steps {
            let color = match step.kind {
                CollisionProbeStepKind::Walk => green,
                CollisionProbeStepKind::StepUp | CollisionProbeStepKind::StepDown => yellow,
                CollisionProbeStepKind::Contact => orange,
                CollisionProbeStepKind::Blocked | CollisionProbeStepKind::NoFloor => red,
            };
            let a = Self::map_to_screen(map, dim, Vec2::new(step.from.x, step.from.z));
            let b = Self::map_to_screen(map, dim, Vec2::new(step.to.x, step.to.z));
            Self::draw_contrast_line(buffer, a, b, color);

            for sample in &step.support_samples {
                let sample_pos = Self::map_to_screen(map, dim, sample.position);
                Self::draw_marker(
                    buffer,
                    sample_pos,
                    if sample.reachable { blue } else { gray },
                );
            }

            if let Some(blocker) = step.blocker {
                let a = Self::map_to_screen(map, dim, blocker.start);
                let b = Self::map_to_screen(map, dim, blocker.end);
                Self::draw_contrast_line(
                    buffer,
                    a,
                    b,
                    if step.kind == CollisionProbeStepKind::Contact {
                        orange
                    } else {
                        red
                    },
                );
            }
        }

        let goto_path = Self::goto_path_points(result);
        for segment in goto_path.windows(2) {
            let a = Self::map_to_screen(map, dim, Vec2::new(segment[0].x, segment[0].z));
            let b = Self::map_to_screen(map, dim, Vec2::new(segment[1].x, segment[1].z));
            let (a, b) = Self::offset_screen_segment(a, b, 6.0);
            Self::draw_contrast_line(buffer, a, b, magenta);
        }

        let start = Self::map_to_screen(map, dim, Vec2::new(result.start.x, result.start.z));
        Self::draw_marker(buffer, start, green);
        let target = Self::map_to_screen(map, dim, result.target);
        Self::draw_marker(buffer, target, if result.arrived { green } else { red });
    }

    fn draw_legend(
        buffer: &mut TheRGBABuffer,
        ctx: &TheContext,
        result: &rusterix::CollisionProbeResult,
    ) {
        let dim = *buffer.dim();
        let stride = buffer.stride();
        let panel = [24, 24, 26, 220];
        let text = [226, 232, 240, 255];
        let bg = [0, 0, 0, 0];
        let result_text = if result.goto_path_unstable {
            fl!("collision_probe_goto_unstable")
        } else if result.arrived {
            fl!("collision_probe_arrived")
        } else if result.goto_path_found {
            fl!("collision_probe_goto_ok")
        } else {
            fl!("collision_probe_blocked")
        };
        let label = format!("{} | {}", fl!("collision_probe_legend"), result_text);
        let max_width = (dim.width as i32 - 16).max(0);
        let width = (label.len() as i32 * 7).clamp(0, max_width).max(0) as usize;
        ctx.draw.rect(
            buffer.pixels_mut(),
            &(8, dim.height as usize - 28, width, 20),
            stride,
            &panel,
        );
        ctx.draw.text(
            buffer.pixels_mut(),
            &(12, dim.height as usize - 25),
            stride,
            &label,
            TheFontSettings {
                size: 12.0,
                ..Default::default()
            },
            &text,
            &bg,
        );
    }
}

impl Tool for CollisionProbeTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Collision Probe Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_collision_probe")
    }

    fn icon_name(&self) -> String {
        "path".into()
    }

    fn accel(&self) -> Option<char> {
        Some('C')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            ToolEvent::Activate => {
                server_ctx.curr_map_tool_type = MapToolType::CollisionProbe;
                Self::clear_probe(server_ctx);
                ctx.ui.redraw_all = true;
                true
            }
            ToolEvent::DeActivate => {
                Self::clear_probe(server_ctx);
                RUSTERIX.write().unwrap().set_overlay_dirty();
                ctx.ui.redraw_all = true;
                true
            }
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match map_event {
            MapClicked(coord) => {
                if server_ctx.collision_probe_points.is_empty() {
                    if let Some(start) = Self::pointer_position(ui, server_ctx, map, coord, None) {
                        let world = Self::build_collision_world(map);
                        let start_2d = Vec2::new(start.x, start.z);
                        let reference_y = world
                            .get_floor_height_nearest(start_2d, start.y)
                            .unwrap_or(start.y);
                        let start = Vec3::new(start.x, reference_y, start.z);
                        server_ctx.collision_probe_start = Some(start);
                        server_ctx.collision_probe_target = None;
                        server_ctx.collision_probe_result = None;
                        server_ctx.collision_probe_points.push(start);
                        server_ctx.collision_probe_dragging = true;
                        RUSTERIX.write().unwrap().set_overlay_dirty();
                        ctx.ui.redraw_all = true;
                    }
                } else if let Some(target) = self.update_probe(ui, ctx, map, server_ctx, coord) {
                    if let Some(result) = server_ctx.collision_probe_result.clone()
                        && result.steps.len() > 0
                    {
                        server_ctx.collision_probe_results.push(result);
                    }
                    server_ctx.collision_probe_points.push(target);
                    server_ctx.collision_probe_start = Some(target);
                    server_ctx.collision_probe_target = None;
                    server_ctx.collision_probe_result = None;
                    server_ctx.collision_probe_dragging = true;
                    RUSTERIX.write().unwrap().set_overlay_dirty();
                    ctx.ui.redraw_all = true;
                }
            }
            MapHover(coord) => {
                if !server_ctx.collision_probe_points.is_empty() {
                    self.update_probe(ui, ctx, map, server_ctx, coord);
                }
            }
            MapDragged(coord) => {
                if !server_ctx.collision_probe_points.is_empty() {
                    self.update_probe(ui, ctx, map, server_ctx, coord);
                }
            }
            MapEscape => {
                if server_ctx.collision_probe_points.is_empty() {
                    Self::clear_probe(server_ctx);
                } else {
                    Self::finish_active_polyline(server_ctx);
                }
                RUSTERIX.write().unwrap().set_overlay_dirty();
                ctx.ui.redraw_all = true;
            }
            _ => {}
        }

        None
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        _assets: &Assets,
    ) {
        let active_result = server_ctx.collision_probe_result.as_ref();
        let latest_result = active_result.or_else(|| server_ctx.collision_probe_results.last());
        let Some(latest_result) = latest_result else {
            return;
        };

        if server_ctx.editor_view_mode != EditorViewMode::D2 {
            let dim = *buffer.dim();
            Self::draw_goto_route_3d_screen(buffer, dim, latest_result, false);
            Self::draw_legend(buffer, _ctx, latest_result);
            return;
        }

        let dim = *buffer.dim();
        for result in &server_ctx.collision_probe_results {
            Self::draw_result_2d(buffer, map, dim, result, false);
        }
        if let Some(result) = active_result {
            Self::draw_result_2d(buffer, map, dim, result, false);
        }
        Self::draw_legend(buffer, _ctx, latest_result);
    }
}
