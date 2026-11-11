use crate::editor::{DOCKMANAGER, RUSTERIX};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::material_library::MaterialDefinition;
use std::collections::HashMap;

const ISO_PAINT_MIN_BRUSH_SIZE: f32 = 0.05;
const ISO_PAINT_MAX_PAINT_BRUSH_SIZE: f32 = 16.0;
const ISO_PAINT_MAX_STAMP_BRUSH_SIZE: f32 = 8.0;
const ISO_PAINT_PATTERN_KIND: &str = "3D Paint Pattern Kind";
const ISO_PAINT_PATTERN_SCALE: &str = "3D Paint Pattern Scale";
const ISO_PAINT_MORTAR: &str = "3D Paint Mortar";
const ISO_PAINT_PATTERN_DETAIL: &str = "3D Paint Pattern Detail";
const ISO_PAINT_PATTERN_VARIATION: &str = "3D Paint Pattern Variation";

#[derive(Clone)]
struct IsoPaintSelectionProjection {
    owner: IsoPaintOwner,
    origin_world: Vec3<f32>,
    normal: Vec3<f32>,
    axis_u: Vec3<f32>,
    axis_v: Vec3<f32>,
    origin_surface: Vec2<f32>,
    plane_distance: f32,
    camera: scenevm::Camera3D,
    viewport: Vec2<f32>,
}

impl IsoPaintSelectionProjection {
    fn surface_at(&self, screen: Vec2<i32>) -> Option<Vec2<f32>> {
        let ndc_x = screen.x as f32 / self.viewport.x * 2.0 - 1.0;
        let ndc_y = 1.0 - screen.y as f32 / self.viewport.y * 2.0;
        let aspect = self.viewport.x / self.viewport.y;
        let (ray_origin, ray_direction) = match self.camera.kind {
            scenevm::CameraKind::OrthoIso => (
                self.camera.pos
                    + self.camera.right * (ndc_x * self.camera.ortho_half_h * aspect)
                    + self.camera.up * (ndc_y * self.camera.ortho_half_h),
                self.camera.forward,
            ),
            scenevm::CameraKind::OrbitPersp | scenevm::CameraKind::FirstPersonPersp => {
                let tan_half = (self.camera.vfov_deg.to_radians() * 0.5).tan();
                (
                    self.camera.pos,
                    self.camera.forward
                        + self.camera.right * (ndc_x * tan_half * aspect)
                        + self.camera.up * (ndc_y * tan_half),
                )
            }
        };
        let denominator = ray_direction.dot(self.normal);
        if !denominator.is_finite() || denominator.abs() <= 1.0e-6 {
            return None;
        }
        let distance = (self.plane_distance - ray_origin.dot(self.normal)) / denominator;
        let world = ray_origin + ray_direction * distance;
        (distance.is_finite() && world.x.is_finite() && world.y.is_finite() && world.z.is_finite())
            .then(|| self.surface_coord(world))
    }

    fn surface_coord(&self, world: Vec3<f32>) -> Vec2<f32> {
        Vec2::new(world.dot(self.axis_u), world.dot(self.axis_v))
    }

    fn world_to_screen(&self, world: Vec3<f32>) -> Option<[i32; 2]> {
        let relative = world - self.camera.pos;
        let aspect = self.viewport.x / self.viewport.y;
        let (ndc_x, ndc_y) = match self.camera.kind {
            scenevm::CameraKind::OrthoIso => (
                relative.dot(self.camera.right) / (self.camera.ortho_half_h * aspect),
                relative.dot(self.camera.up) / self.camera.ortho_half_h,
            ),
            scenevm::CameraKind::OrbitPersp | scenevm::CameraKind::FirstPersonPersp => {
                let depth = relative.dot(self.camera.forward);
                if depth <= self.camera.near || depth >= self.camera.far {
                    return None;
                }
                let tan_half = (self.camera.vfov_deg.to_radians() * 0.5).tan();
                (
                    relative.dot(self.camera.right) / (depth * tan_half * aspect),
                    relative.dot(self.camera.up) / (depth * tan_half),
                )
            }
        };
        if !ndc_x.is_finite() || !ndc_y.is_finite() {
            return None;
        }
        Some([
            ((ndc_x + 1.0) * 0.5 * self.viewport.x).round() as i32,
            ((1.0 - ndc_y) * 0.5 * self.viewport.y).round() as i32,
        ])
    }

    fn outline(&self, bounds: [f32; 4]) -> Option<[[i32; 2]; 4]> {
        let corners = [
            Vec2::new(bounds[0], bounds[1]),
            Vec2::new(bounds[2], bounds[1]),
            Vec2::new(bounds[2], bounds[3]),
            Vec2::new(bounds[0], bounds[3]),
        ];
        let mut outline = [[0; 2]; 4];
        for (index, corner) in corners.into_iter().enumerate() {
            let world =
                self.axis_u * corner.x + self.axis_v * corner.y + self.normal * self.plane_distance;
            outline[index] = self.world_to_screen(world)?;
        }
        Some(outline)
    }
}

pub struct IsoPaintTool {
    overlay: super::paint_overlay::PaintOverlay,
    id: TheId,
    painting: bool,
    previous_dock: Option<String>,
    active_stroke: Option<Uuid>,
    last_stamp_screen: Option<[i32; 2]>,
    stamp_clip_geo: Option<[u32; 4]>,
    stroke_before: Option<IsoPaintLayer>,
    stroke_changed: bool,
    paint_surface_cache: Option<scenevm::PaintSurfaceBuffer>,
    selection_drag_start: Option<Vec2<i32>>,
    selection_projection: Option<IsoPaintSelectionProjection>,
    selection_move_bounds: Option<[f32; 4]>,
    selection_grid_step: f32,
    terrain_hover: Option<Vec2<f32>>,
    terrain_radius: f32,
    terrain_selection_start: Option<Vec2<f32>>,
    terrain_stamp_last: Option<Vec2<f32>>,
}

impl IsoPaintTool {
    fn terrain_stamp_dabs(
        &mut self,
        map: &Map,
        layer: &mut IsoPaintLayer,
        point: IsoPaintPoint,
    ) -> bool {
        let Some(uv) = point.surface_uv else {
            return false;
        };
        let target = Vec2::<f32>::from(uv);
        let spacing =
            (layer.active_size * 9. * (1.55 - layer.active_stamp_density.clamp(0., 1.) * 0.9)
                / 32.)
                .max(1. / 32.);
        let erase_radius = (layer.active_size * 5. / 32.).max(0.1);
        let spacing = if layer.active_operation == "erase" {
            spacing.min(erase_radius * 0.5)
        } else {
            spacing
        };
        let mut positions = Vec::new();
        if let Some(last) = self.terrain_stamp_last {
            let distance = (target - last).magnitude();
            let count = (distance / spacing).floor() as usize;
            for i in 1..=count {
                positions.push(last + (target - last) * (i as f32 * spacing / distance));
            }
        } else {
            positions.push(target);
        }
        let surface = rusterix::chunkbuilder::terrain_surface::TerrainSurface::cached(map);
        let mut changed = false;
        for at in positions {
            self.terrain_stamp_last = Some(at);
            if layer.active_operation == "erase" {
                let kind = layer.active_brush.clone();
                changed |= layer.erase_terrain_stamps_near(at.into_array(), erase_radius, &kind);
                continue;
            }
            if layer.active_selection().is_some_and(|s| {
                !s.contains(
                    rusterix::isometric::paint::TERRAIN_PAINT_GEO,
                    [(at.x * 32.).round() as i32, (at.y * 32.).round() as i32],
                )
            }) {
                continue;
            }
            let height = if let Some(surface) = &surface {
                let Some(height) = surface.height_at(at) else {
                    continue;
                };
                height
            } else {
                if !rusterix::isometric::flat_terrain_contains(map, at) {
                    continue;
                }
                0.
            };
            let mut point = point.clone();
            point.surface_uv = Some(at.into_array());
            point.world = Some([at.x, height, at.y]);
            // Native procedural artwork uses a fixed world density, independent
            // of the editor zoom. Keep the 3D billboard scale compatible too.
            point.camera_scale = point.viewport_size.map(|s| s[1] as f32 / 64.);
            layer.add_stamp(point);
            changed = true;
        }
        changed
    }

    fn terrain_paint_point(
        map: &Map,
        viewport: [i32; 2],
        coord: Vec2<i32>,
    ) -> Option<IsoPaintPoint> {
        let projection = rusterix::isometric::Projection::editor(
            map,
            Vec2::new(viewport[0] as f32, viewport[1] as f32),
        );
        let world = rusterix::isometric::terrain_point(map, projection, coord.map(|v| v as f32))?;
        Some(
            IsoPaintPoint::new(
                [coord.x, coord.y],
                Some(world),
                Some(IsoPaintOwner::Terrain { x: 0, z: 0 }),
            )
            .with_surface_uv(Some(Vec2::new(world.x, world.z)))
            .with_paint_geo(Some(rusterix::isometric::paint::TERRAIN_PAINT_GEO))
            .with_surface_normal(Some(Vec3::unit_y()))
            .with_viewport_size(Some(viewport)),
        )
    }

    fn terrain_map_event(
        &mut self,
        event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        region: &mut Region,
        server: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let viewport = Self::paint_viewport_size(ui, server)?;
        if !server.pc.is_prefab() && !self.overlay.is_active() {
            Self::sync_live_paint_settings(ui, &mut region.iso_paint);
        }
        self.terrain_radius = if Self::is_stamp_mode(&region.iso_paint) {
            (region.iso_paint.active_size * 5. / 32.).max(0.1)
        } else {
            (region.iso_paint.active_size * 2.).round().clamp(1., 96.) / 32.
        };
        let coord = match &event {
            MapClicked(p) | MapDragged(p) | MapHover(p) | MapUp(p) => Some(*p),
            _ => None,
        };
        let point = coord.and_then(|p| Self::terrain_paint_point(&region.map, viewport, p));
        if let Some(coord) = coord {
            server.iso_paint_hover_screen = Some(coord);
            self.terrain_hover = point.as_ref().and_then(|p| p.surface_uv).map(Vec2::from);
        }
        match event {
            MapClicked(_) => {
                self.reset_stroke();
                let point = point?;
                if region.iso_paint.active_operation == "pick" {
                    Self::pick_paint_color(&mut region.iso_paint, &point, ctx);
                    return None;
                }
                self.stroke_before = Some(region.iso_paint.clone());
                self.painting = true;
                if region.iso_paint.active_operation == "select" {
                    self.terrain_selection_start = point.surface_uv.map(Vec2::from);
                } else if Self::is_stamp_mode(&region.iso_paint) {
                    self.stroke_changed =
                        self.terrain_stamp_dabs(&region.map, &mut region.iso_paint, point);
                } else {
                    self.active_stroke = Some(region.iso_paint.begin_stroke(point));
                    self.stroke_changed = true;
                }
            }
            MapDragged(_) | MapUp(_) if self.painting => {
                if let Some(start) = self.terrain_selection_start {
                    if let Some(end) = point.as_ref().and_then(|p| p.surface_uv).map(Vec2::from) {
                        let min = start.map2(end, f32::min);
                        let max = start.map2(end, f32::max);
                        let p = rusterix::isometric::Projection::editor(
                            &region.map,
                            Vec2::new(viewport[0] as f32, viewport[1] as f32),
                        );
                        let surface =
                            rusterix::chunkbuilder::terrain_surface::TerrainSurface::cached(
                                &region.map,
                            );
                        server.iso_paint_selection_rect = Some(
                            [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)].map(|q| {
                                p.screen(
                                    q,
                                    surface.as_ref().and_then(|s| s.height_at(q)).unwrap_or(0.),
                                )
                                .map(|v| v.round() as i32)
                                .into_array()
                            }),
                        );
                        if matches!(event, MapUp(_)) {
                            if ((max.x - min.x) * (max.y - min.y) * 1024.) > 1_048_576. {
                                self.reset_stroke();
                                server.iso_paint_selection_rect = None;
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Select a smaller terrain area (up to 32 × 32 tiles).".into(),
                                ));
                                Self::request_paint_redraw(ctx);
                                return None;
                            }
                            let owner = IsoPaintOwner::Terrain { x: 0, z: 0 };
                            let mut selection = IsoPaintSelection::new(owner.clone());
                            for z in (min.y * 32.).floor() as i32..=(max.y * 32.).ceil() as i32 {
                                for x in (min.x * 32.).floor() as i32..=(max.x * 32.).ceil() as i32
                                {
                                    let q = Vec2::new(x as f32 / 32., z as f32 / 32.);
                                    if surface.as_ref().is_none_or(|s| s.contains(q)) {
                                        selection.set_pixel(
                                            owner.clone(),
                                            rusterix::isometric::paint::TERRAIN_PAINT_GEO,
                                            [x, z],
                                        );
                                    }
                                }
                            }
                            if !selection.is_empty() {
                                region.iso_paint.set_active_selection(selection);
                                self.stroke_changed = true;
                            }
                        }
                    }
                } else if Self::is_stamp_mode(&region.iso_paint) {
                    if let Some(point) = point {
                        self.stroke_changed |=
                            self.terrain_stamp_dabs(&region.map, &mut region.iso_paint, point);
                    }
                } else if let Some(id) = self.active_stroke {
                    if let Some(point) = point {
                        self.stroke_changed |= region.iso_paint.append_point(id, point);
                    }
                }
                if matches!(event, MapUp(_)) {
                    let undo = self.stroke_changed.then(|| {
                        ProjectUndoAtom::RegionPaintEdit(
                            ProjectContext::Region(region.id),
                            region.id,
                            Box::new(self.stroke_before.take().unwrap()),
                            Box::new(region.iso_paint.clone()),
                        )
                    });
                    self.reset_stroke();
                    server.iso_paint_selection_rect = None;
                    Self::request_paint_redraw(ctx);
                    return undo;
                }
            }
            MapEscape => {
                if let Some(before) = self.stroke_before.take() {
                    region.iso_paint = before;
                }
                self.reset_stroke();
                self.terrain_hover = None;
                server.iso_paint_selection_rect = None;
            }
            _ => {}
        }
        Self::request_paint_redraw(ctx);
        None
    }

    fn active_size_max(layer: &IsoPaintLayer) -> f32 {
        if Self::is_stamp_mode(layer) {
            ISO_PAINT_MAX_STAMP_BRUSH_SIZE
        } else {
            ISO_PAINT_MAX_PAINT_BRUSH_SIZE
        }
    }

    fn neutral_material_palette(project: &Project) -> (u16, [u8; 4]) {
        let target = [132i32, 132i32, 128i32];
        let mut best: Option<(u16, [u8; 4], i32)> = None;
        for (index, entry) in project.palette.colors.iter().enumerate() {
            let Some(color) = entry.as_ref() else {
                continue;
            };
            let mut color = color.to_u8_array();
            color[3] = 255;
            let dr = color[0] as i32 - target[0];
            let dg = color[1] as i32 - target[1];
            let db = color[2] as i32 - target[2];
            let score = dr * dr + dg * dg + db * db;
            if best.map_or(true, |(_, _, best_score)| score < best_score) {
                best = Some((index as u16, color, score));
            }
        }
        best.map(|(index, color, _)| (index, color))
            .unwrap_or((6, [132, 132, 128, 255]))
    }

    fn material_color_needs_gray(index: Option<u16>, color: [u8; 4]) -> bool {
        let average = (color[0] as u16 + color[1] as u16 + color[2] as u16) / 3;
        index == Some(0) || average < 58 || (color[0] > 150 && color[1] < 90 && color[2] < 90)
    }

    fn ensure_initial_material_settings(layer: &mut IsoPaintLayer, neutral: (u16, [u8; 4])) {
        let active_brush = layer.active_brush.as_str();
        let active_index = layer.active_palette_indices.first().copied();
        let active_color = layer
            .active_palette_colors
            .first()
            .copied()
            .unwrap_or(layer.active_color);
        let needs_material_seed = active_brush.is_empty()
            || active_brush == "screen"
            || (active_brush == "material"
                && (layer.active_palette_colors.is_empty()
                    || Self::material_color_needs_gray(active_index, active_color)));

        if !needs_material_seed {
            return;
        }

        let (palette_index, color) = neutral;
        let size = if layer.active_size <= 1.001 {
            8.0
        } else {
            layer.active_size
        };
        let opacity = if layer.active_opacity <= 0.0 {
            1.0
        } else {
            layer.active_opacity
        };
        let material_id = MaterialDefinition::from_preset_finish("default", "natural").id();
        layer.set_active_settings(
            "draw",
            "material",
            "solid",
            "default",
            "natural",
            material_id,
            "coat",
            "surface",
            color,
            vec![palette_index],
            vec![color],
            layer.active_pattern_kind.clone(),
            layer.active_pattern_scale,
            layer.active_pattern_mortar,
            layer.active_pattern_detail,
            layer.active_pattern_variation,
            layer.active_stamp_density,
            layer.active_stamp_size_jitter,
            layer.active_stamp_rotation_jitter,
            "wildflowers",
            size,
            opacity,
        );
    }

    fn is_stamp_mode(layer: &IsoPaintLayer) -> bool {
        (layer.active_source.is_some() || matches!(
            layer.active_brush.as_str(),
            "grass"
                | "rubble"
                | "leaves"
                | "flowers"
                | "vines"
                | "roots"
                | "bushes"
                | "tree"
                | "candles"
                | "footprints"
                | "mud"
        )) && layer.active_material_mode == "stamp"
    }

    fn stamp_label(layer: &IsoPaintLayer) -> &'static str {
        match layer.active_brush.as_str() {
            "rubble" => "rubble",
            "leaves" => "leaves",
            "flowers" => "flowers",
            "vines" => "vines",
            "roots" => "roots",
            "bushes" => "bushes",
            "tree" => "tree",
            "candles" => "candles",
            "footprints" => "footprints",
            "mud" => "mud",
            _ => "grass",
        }
    }

    fn should_place_stamp(
        last: Option<[i32; 2]>,
        coord: Vec2<i32>,
        size: f32,
        density: f32,
    ) -> bool {
        let Some(last) = last else {
            return true;
        };
        let density = density.clamp(0.0, 1.0);
        let spacing_scale = 1.55 - density * 0.9;
        let spacing = (size * 9.0 * spacing_scale).round().clamp(5.0, 42.0) as i32;
        let dx = coord.x - last[0];
        let dy = coord.y - last[1];
        dx * dx + dy * dy >= spacing * spacing
    }

    fn stamp_clip_geo(layer: &IsoPaintLayer, point: &IsoPaintPoint) -> Option<[u32; 4]> {
        (layer.active_clip == "surface")
            .then_some(point.paint_geo)
            .flatten()
    }

    fn stamp_point_matches_clip(point: &IsoPaintPoint, clip_geo: Option<[u32; 4]>) -> bool {
        clip_geo.is_none_or(|clip_geo| point.paint_geo == Some(clip_geo))
    }

    fn apply_stamp_at(
        layer: &mut IsoPaintLayer,
        point: IsoPaintPoint,
        clip_geo: Option<[u32; 4]>,
    ) -> bool {
        if !Self::stamp_point_matches_clip(&point, clip_geo) {
            return false;
        }
        if layer.active_operation == "erase" {
            let active_brush = layer.active_brush.clone();
            layer.erase_stamps_near_owner_kind(
                point.screen,
                layer.active_size,
                point.owner.as_ref(),
                Some(active_brush.as_str()),
            )
        } else if layer.active_operation == "draw" {
            layer.add_stamp(point);
            true
        } else {
            false
        }
    }

    fn pick_paint_color(layer: &mut IsoPaintLayer, point: &IsoPaintPoint, ctx: &mut TheContext) {
        let (Some(uv), Some(geo)) = (point.surface_uv, point.paint_geo) else { return; };
        let assets = RUSTERIX.read().unwrap();
        layer.refresh_sources(&assets.assets);
        layer.ensure_baked_paint_current();
        let at = uv.map(|v| (v * 32.).floor() as i32);
        let width = shared::iso_paint::ISO_PAINT_BAKED_CHUNK_SIZE;
        let color = layer.baked_chunks.values().find_map(|chunk| {
            let x = at[0] - chunk.origin[0];
            let y = at[1] - chunk.origin[1];
            if chunk.paint_geo != geo || x < 0 || y < 0 || x >= width || y >= width { return None; }
            let index = ((y * width + x) * 4) as usize;
            let rgba: [u8;4] = chunk.color_rgba[index..index+4].try_into().ok()?;
            (rgba[3] > 0).then_some(rgba)
        });
        let Some(color) = color else { return; };
        let (index, mut swatch) = assets.assets.palette.colors.iter().enumerate()
            .filter_map(|(i,c)|c.as_ref().map(|c|(i,c.to_u8_array())))
            .min_by_key(|(_,c)|(0..3).map(|i|(c[i] as i32-color[i] as i32).pow(2)).sum::<i32>())
            .unwrap_or((0,color));
        swatch[3] = 255;
        layer.active_source = None;
        layer.active_brush = "material".into();
        layer.active_brush_shape = "solid".into();
        layer.active_material_mode = "coat".into();
        layer.active_color = swatch;
        layer.active_palette_colors = vec![swatch];
        layer.active_palette_indices = vec![index as u16];
        ctx.ui.send(TheEvent::Custom(TheId::named("Paint Picker Refresh"), TheValue::Empty));
    }

    fn sync_live_paint_settings(ui: &mut TheUI, layer: &mut IsoPaintLayer) {
        if layer.active_source.is_some() { return; }
        if let Some(opacity) = ui
            .get_widget_value("3D Paint Tool Opacity")
            .and_then(|value| value.to_f32())
        {
            layer.active_opacity = opacity.clamp(0.0, 1.0);
        }
        if let Some(TheValue::Int(index)) = ui.get_widget_value("3D Paint Material Mode") {
            layer.active_material_mode = match index {
                1 => "replace".to_string(),
                2 => "stamp".to_string(),
                _ => "coat".to_string(),
            };
        }
        if let Some(TheValue::Int(index)) = ui.get_widget_value(ISO_PAINT_PATTERN_KIND) {
            layer.active_pattern_kind = match index {
                0 => "tile".to_string(),
                2 => "arch".to_string(),
                _ => "brick".to_string(),
            };
        }
        if let Some(pattern_scale) = ui
            .get_widget_value(ISO_PAINT_PATTERN_SCALE)
            .and_then(|value| value.to_f32())
        {
            layer.active_pattern_scale = pattern_scale.clamp(0.25, 4.0);
        }
        if let Some(mortar) = ui
            .get_widget_value(ISO_PAINT_MORTAR)
            .and_then(|value| value.to_f32())
        {
            layer.active_pattern_mortar = mortar.clamp(0.0, 0.4);
        }
        if let Some(detail) = ui
            .get_widget_value(ISO_PAINT_PATTERN_DETAIL)
            .and_then(|value| value.to_f32())
        {
            layer.active_pattern_detail = detail.clamp(0.0, 1.0);
        }
        if let Some(variation) = ui
            .get_widget_value(ISO_PAINT_PATTERN_VARIATION)
            .and_then(|value| value.to_f32())
        {
            layer.active_pattern_variation = variation.clamp(0.0, 1.0);
        }
        if let Some(size) = ui
            .get_widget_value("3D Paint Tool Size")
            .and_then(|value| value.to_f32())
        {
            layer.active_size = size.clamp(ISO_PAINT_MIN_BRUSH_SIZE, Self::active_size_max(layer));
        }
        if let Some(size_jitter) = ui
            .get_widget_value("3D Paint Stamp Size Jitter")
            .and_then(|value| value.to_f32())
        {
            layer.active_stamp_size_jitter = size_jitter.clamp(0.0, 1.0);
        }
        if let Some(rotation_jitter) = ui
            .get_widget_value("3D Paint Stamp Rotation Jitter")
            .and_then(|value| value.to_f32())
        {
            layer.active_stamp_rotation_jitter = rotation_jitter.clamp(0.0, 1.0);
        }
    }

    fn hit_status(server_ctx: &ServerContext) -> String {
        if server_ctx.geo_hit.is_some() {
            fl!("status_iso_paint_hit")
        } else if server_ctx.hover_cursor_3d.is_some() {
            fl!("status_iso_paint_ground")
        } else {
            fl!("status_iso_paint_active")
        }
    }

    fn owner_from_geo_id(geo_id: scenevm::GeoId) -> IsoPaintOwner {
        match geo_id {
            scenevm::GeoId::Unknown(id) => IsoPaintOwner::Unknown(id),
            scenevm::GeoId::Vertex(id) => IsoPaintOwner::Vertex(id),
            scenevm::GeoId::Linedef(id) => IsoPaintOwner::Linedef(id),
            scenevm::GeoId::Sector(id) => IsoPaintOwner::Sector(id),
            scenevm::GeoId::Character(id) => IsoPaintOwner::Character(id),
            scenevm::GeoId::Item(id) => IsoPaintOwner::Item(id),
            scenevm::GeoId::Light(id) => IsoPaintOwner::Light(id),
            scenevm::GeoId::ItemLight(id) => IsoPaintOwner::ItemLight(id),
            scenevm::GeoId::Triangle(id) => IsoPaintOwner::Triangle(id),
            scenevm::GeoId::Terrain(x, z) => IsoPaintOwner::Terrain { x, z },
            scenevm::GeoId::GeometryObject(id) => IsoPaintOwner::GeometryObject(id),
            scenevm::GeoId::Hole(sector_id, hole_id) => IsoPaintOwner::Hole { sector_id, hole_id },
            scenevm::GeoId::Gizmo(id) => IsoPaintOwner::Gizmo(id),
        }
    }

    fn paint_point(
        coord: Vec2<i32>,
        server_ctx: &ServerContext,
        viewport_size: Option<[i32; 2]>,
    ) -> IsoPaintPoint {
        // Read the dedicated paint coordinate emitted by the actual rasterized triangle. It is
        // intentionally separate from the material texture UV, which may repeat across tiles.
        let (raster_surface, brush_transform) = viewport_size
            .and_then(|[width, height]| {
                if width <= 0 || height <= 0 {
                    return None;
                }
                RUSTERIX.read().ok().and_then(|rusterix| {
                    let screen_uv = [
                        coord.x as f32 / width as f32,
                        coord.y as f32 / height as f32,
                    ];
                    let surface = rusterix.scene_handler.vm.pick_paint_surface_at_uv(
                        width as u32,
                        height as u32,
                        screen_uv,
                    )?;
                    let brush_transform = rusterix.scene_handler.vm.paint_surface_brush_transform(
                        width as u32,
                        height as u32,
                        screen_uv,
                        &surface,
                    );
                    Some((surface, brush_transform))
                })
            })
            .map_or((None, None), |(surface, transform)| {
                (Some(surface), transform)
            });
        let owner = raster_surface
            .as_ref()
            .map(|surface| Self::owner_from_geo_id(surface.geo_id))
            .or_else(|| server_ctx.geo_hit.map(Self::owner_from_geo_id));
        let world = if server_ctx.geo_hit.is_some() {
            Some(server_ctx.geo_hit_pos)
        } else {
            server_ctx.hover_cursor_3d
        };
        let surface_uv = raster_surface
            .map(|surface| Vec2::new(surface.uv[0], surface.uv[1]))
            .or_else(|| {
                server_ctx.hover_surface.as_ref().and_then(|surface| {
                    server_ctx
                        .hover_surface_hit_pos
                        .map(|pos| surface.world_to_uv(pos))
                })
            });
        let surface_normal = raster_surface
            .map(|surface| Vec3::new(surface.normal[0], surface.normal[1], surface.normal[2]))
            .or(server_ctx.hover_surface_normal)
            .or_else(|| {
                server_ctx
                    .hover_surface
                    .as_ref()
                    .map(|surface| surface.plane.normal)
            });
        let camera_scale = RUSTERIX
            .read()
            .ok()
            .map(|rusterix| rusterix.client.camera_d3.scale());
        IsoPaintPoint::new([coord.x, coord.y], world, owner)
            .with_surface_uv(surface_uv)
            .with_paint_geo(raster_surface.map(|surface| surface.paint_geo))
            .with_surface_normal(surface_normal)
            .with_camera_scale(camera_scale)
            .with_viewport_size(viewport_size)
            .with_brush_transform(brush_transform)
    }

    fn capture_paint_surface(
        viewport_size: Option<[i32; 2]>,
    ) -> Option<scenevm::PaintSurfaceBuffer> {
        let [width, height] = viewport_size?;
        if width <= 0 || height <= 0 {
            return None;
        }
        let mut rusterix = RUSTERIX.write().ok()?;
        rusterix
            .scene_handler
            .vm
            .paint_surface_buffer_gpu(width as u32, height as u32)
            .or_else(|| {
                Some(
                    rusterix
                        .scene_handler
                        .vm
                        .paint_surface_buffer(width as u32, height as u32),
                )
            })
    }

    fn surface_pixel_derivative(
        buffer: &scenevm::PaintSurfaceBuffer,
        x: i32,
        y: i32,
        paint_geo: [u32; 4],
        horizontal: bool,
    ) -> Option<[f32; 2]> {
        let center = buffer.pixel(x, y)?;
        let matching = |px: i32, py: i32| {
            buffer
                .pixel(px, py)
                .filter(|pixel| pixel.valid && pixel.paint_geo == paint_geo)
        };
        let mut negative = None;
        let mut positive = None;
        for distance in 1..=4 {
            if negative.is_none() {
                let (px, py) = if horizontal {
                    (x - distance, y)
                } else {
                    (x, y - distance)
                };
                negative = matching(px, py).map(|pixel| (distance as f32, pixel));
            }
            if positive.is_none() {
                let (px, py) = if horizontal {
                    (x + distance, y)
                } else {
                    (x, y + distance)
                };
                positive = matching(px, py).map(|pixel| (distance as f32, pixel));
            }
            if negative.is_some() && positive.is_some() {
                break;
            }
        }
        match (negative, positive) {
            (Some((negative_distance, negative)), Some((positive_distance, positive))) => {
                let distance = negative_distance + positive_distance;
                Some([
                    (positive.uv[0] - negative.uv[0]) / distance,
                    (positive.uv[1] - negative.uv[1]) / distance,
                ])
            }
            (Some((distance, negative)), None) => Some([
                (center.uv[0] - negative.uv[0]) / distance,
                (center.uv[1] - negative.uv[1]) / distance,
            ]),
            (None, Some((distance, positive))) => Some([
                (positive.uv[0] - center.uv[0]) / distance,
                (positive.uv[1] - center.uv[1]) / distance,
            ]),
            (None, None) => None,
        }
    }

    fn screen_space_transform(
        buffer: &scenevm::PaintSurfaceBuffer,
        x: i32,
        y: i32,
        paint_geo: [u32; 4],
    ) -> Option<[f32; 4]> {
        if let (Some(dx), Some(dy)) = (
            Self::surface_pixel_derivative(buffer, x, y, paint_geo, true),
            Self::surface_pixel_derivative(buffer, x, y, paint_geo, false),
        ) {
            let transform = [dx[0], dy[0], dx[1], dy[1]];
            let determinant = transform[0] * transform[3] - transform[1] * transform[2];
            if transform.iter().all(|value| value.is_finite())
                && determinant.is_finite()
                && determinant.abs() > 1.0e-10
            {
                return Some(transform);
            }
        }

        // Narrow mortar strips and bevel faces often have no matching pixel directly above or
        // beside the sample. Fit the screen-to-UV transform from every nearby pixel on that face
        // instead of dropping the face from the brush entirely.
        let center = buffer.pixel(x, y)?;
        let mut xx = 0.0_f32;
        let mut xy = 0.0_f32;
        let mut yy = 0.0_f32;
        let mut ux = 0.0_f32;
        let mut uy = 0.0_f32;
        let mut vx = 0.0_f32;
        let mut vy = 0.0_f32;
        let mut samples = 0_u32;
        for sample_y in y - 12..=y + 12 {
            for sample_x in x - 12..=x + 12 {
                let Some(pixel) = buffer
                    .pixel(sample_x, sample_y)
                    .filter(|pixel| pixel.valid && pixel.paint_geo == paint_geo)
                else {
                    continue;
                };
                let sx = (sample_x - x) as f32;
                let sy = (sample_y - y) as f32;
                if sx == 0.0 && sy == 0.0 {
                    continue;
                }
                let du = pixel.uv[0] - center.uv[0];
                let dv = pixel.uv[1] - center.uv[1];
                xx += sx * sx;
                xy += sx * sy;
                yy += sy * sy;
                ux += sx * du;
                uy += sy * du;
                vx += sx * dv;
                vy += sy * dv;
                samples += 1;
            }
        }
        let determinant = xx * yy - xy * xy;
        if samples < 3 || !determinant.is_finite() || determinant.abs() <= 1.0e-8 {
            return None;
        }
        let transform = [
            (ux * yy - uy * xy) / determinant,
            (uy * xx - ux * xy) / determinant,
            (vx * yy - vy * xy) / determinant,
            (vy * xx - vx * xy) / determinant,
        ];
        transform
            .iter()
            .all(|value| value.is_finite())
            .then_some(transform)
    }

    fn projected_paint_points(
        &self,
        coord: Vec2<i32>,
        layer: &IsoPaintLayer,
        viewport_size: Option<[i32; 2]>,
    ) -> Vec<IsoPaintPoint> {
        let Some(buffer) = self.paint_surface_cache.as_ref() else {
            return Vec::new();
        };
        let radius = (layer.active_size * 2.0).round().clamp(1.0, 96.0) as i32;
        let radius_squared = radius * radius;
        let mut faces: HashMap<[u32; 4], (i32, i32, i32, scenevm::PaintSurfacePixel)> =
            HashMap::new();
        for y in coord.y - radius..=coord.y + radius {
            for x in coord.x - radius..=coord.x + radius {
                let dx = x - coord.x;
                let dy = y - coord.y;
                let distance_squared = dx * dx + dy * dy;
                if distance_squared > radius_squared {
                    continue;
                }
                let Some(pixel) = buffer.pixel(x, y).copied().filter(|pixel| pixel.valid) else {
                    continue;
                };
                let entry = faces
                    .entry(pixel.paint_geo)
                    .or_insert((distance_squared, x, y, pixel));
                if distance_squared < entry.0 {
                    *entry = (distance_squared, x, y, pixel);
                }
            }
        }
        let mut faces: Vec<_> = faces.into_values().collect();
        faces.sort_by_key(|entry| entry.0);
        let clip_owner = faces
            .first()
            .map(|entry| Self::owner_from_geo_id(entry.3.geo_id));
        let camera_scale = RUSTERIX
            .read()
            .ok()
            .map(|rusterix| rusterix.client.camera_d3.scale());
        faces
            .into_iter()
            .filter(|(_, _, _, pixel)| {
                !matches!(layer.active_clip.as_str(), "surface" | "object")
                    || clip_owner.as_ref().is_none_or(|owner| {
                        owner.same_paint_object(&Self::owner_from_geo_id(pixel.geo_id))
                    })
            })
            .filter_map(|(_, x, y, pixel)| {
                let transform = Self::screen_space_transform(buffer, x, y, pixel.paint_geo)?;
                Some(
                    IsoPaintPoint::new(
                        [coord.x, coord.y],
                        Some(Vec3::from(pixel.world)),
                        Some(Self::owner_from_geo_id(pixel.geo_id)),
                    )
                    .with_surface_uv(Some(Vec2::from(pixel.uv)))
                    .with_paint_geo(Some(pixel.paint_geo))
                    .with_surface_normal(Some(Vec3::from(pixel.normal)))
                    .with_camera_scale(camera_scale)
                    .with_viewport_size(viewport_size)
                    .with_screen_space_brush(
                        Some(transform),
                        [(x - coord.x) as f32, (y - coord.y) as f32],
                    ),
                )
            })
            .collect()
    }

    fn selection_projection(
        buffer: &scenevm::PaintSurfaceBuffer,
        start: Vec2<i32>,
        _map: &Map,
    ) -> Option<IsoPaintSelectionProjection> {
        let mut clicked = None;
        let mut clicked_distance = i32::MAX;
        for y in start.y - 3..=start.y + 3 {
            for x in start.x - 3..=start.x + 3 {
                let Some(pixel) = buffer.pixel(x, y).copied().filter(|pixel| pixel.valid) else {
                    continue;
                };
                let dx = x - start.x;
                let dy = y - start.y;
                let distance = dx * dx + dy * dy;
                if distance < clicked_distance {
                    clicked_distance = distance;
                    clicked = Some(pixel);
                }
            }
        }
        let clicked = clicked?;
        let owner = Self::owner_from_geo_id(clicked.geo_id);
        let anchor = clicked;

        let normal: Vec3<f32> = Vec3::from(anchor.normal).try_normalized()?;
        let origin_world = Vec3::from(anchor.world);
        let world_up = Vec3::<f32>::unit_y();
        let (axis_u, axis_v) = if normal.y.abs() < 0.9 {
            let axis_v = (world_up - normal * world_up.dot(normal)).try_normalized()?;
            (axis_v.cross(normal).try_normalized()?, axis_v)
        } else {
            let world_right = Vec3::unit_x();
            let axis_u = (world_right - normal * world_right.dot(normal))
                .try_normalized()
                .or_else(|| {
                    let world_forward = Vec3::unit_z();
                    (world_forward - normal * world_forward.dot(normal)).try_normalized()
                })?;
            (axis_u, normal.cross(axis_u).try_normalized()?)
        };
        let viewport = Vec2::new(buffer.width.max(1) as f32, buffer.height.max(1) as f32);
        // Use the exact camera that produced the surface buffer. Reconstructing it from the
        // editor camera can differ in its resolved center and shifts the projected outline away
        // from the pixels that will be committed on release.
        let camera = RUSTERIX.read().ok()?.scene_handler.vm.vm.camera3d;
        Some(IsoPaintSelectionProjection {
            owner,
            origin_world,
            normal,
            axis_u,
            axis_v,
            origin_surface: Vec2::new(origin_world.dot(axis_u), origin_world.dot(axis_v)),
            plane_distance: origin_world.dot(normal),
            camera,
            viewport,
        })
    }

    fn selection_hit_pixel(
        buffer: &scenevm::PaintSurfaceBuffer,
        coord: Vec2<i32>,
        selection: &IsoPaintSelection,
    ) -> Option<scenevm::PaintSurfacePixel> {
        let mut hit = None;
        let mut best_distance = i32::MAX;
        for y in coord.y - 3..=coord.y + 3 {
            for x in coord.x - 3..=coord.x + 3 {
                let Some(pixel) = buffer.pixel(x, y).copied().filter(|pixel| pixel.valid) else {
                    continue;
                };
                let uv_pixel = [
                    (pixel.uv[0] * ISO_PAINT_BAKED_PIXELS_PER_UV).round() as i32,
                    (pixel.uv[1] * ISO_PAINT_BAKED_PIXELS_PER_UV).round() as i32,
                ];
                if !selection.contains(pixel.paint_geo, uv_pixel) {
                    continue;
                }
                let dx = x - coord.x;
                let dy = y - coord.y;
                let distance = dx * dx + dy * dy;
                if distance < best_distance {
                    best_distance = distance;
                    hit = Some(pixel);
                }
            }
        }
        hit
    }

    fn active_selection_bounds(
        buffer: &scenevm::PaintSurfaceBuffer,
        projection: &IsoPaintSelectionProjection,
        selection: &IsoPaintSelection,
    ) -> Option<[f32; 4]> {
        let mut min = Vec2::broadcast(f32::INFINITY);
        let mut max = Vec2::broadcast(f32::NEG_INFINITY);
        let mut found = false;
        for pixel in &buffer.pixels {
            if !pixel.valid
                || !projection
                    .owner
                    .same_paint_object(&Self::owner_from_geo_id(pixel.geo_id))
            {
                continue;
            }
            let relative = Vec3::from(pixel.world) - projection.origin_world;
            if relative.dot(projection.normal).abs() > 0.75 {
                continue;
            }
            let Some(pixel_normal) = Vec3::from(pixel.normal).try_normalized() else {
                continue;
            };
            let uv_pixel = [
                (pixel.uv[0] * ISO_PAINT_BAKED_PIXELS_PER_UV).round() as i32,
                (pixel.uv[1] * ISO_PAINT_BAKED_PIXELS_PER_UV).round() as i32,
            ];
            if pixel_normal.dot(projection.normal).abs() < 0.55
                || !selection.contains(pixel.paint_geo, uv_pixel)
            {
                continue;
            }
            let surface = projection.surface_coord(Vec3::from(pixel.world));
            min.x = min.x.min(surface.x);
            min.y = min.y.min(surface.y);
            max.x = max.x.max(surface.x);
            max.y = max.y.max(surface.y);
            found = true;
        }
        found.then_some([min.x, min.y, max.x, max.y])
    }

    fn snapped_selection_bounds(&self, end: Vec2<i32>) -> Option<[f32; 4]> {
        let projection = self.selection_projection.as_ref()?;
        let step = self.selection_grid_step.max(0.0001);
        let snap = |value: f32| (value / step).round() * step;
        if let Some(bounds) = self.selection_move_bounds {
            let delta = projection.surface_at(end)? - projection.origin_surface;
            let delta = Vec2::new(snap(delta.x), snap(delta.y));
            Some([
                bounds[0] + delta.x,
                bounds[1] + delta.y,
                bounds[2] + delta.x,
                bounds[3] + delta.y,
            ])
        } else {
            let start = Vec2::new(
                snap(projection.origin_surface.x),
                snap(projection.origin_surface.y),
            );
            let end = projection.surface_at(end)?;
            let end = Vec2::new(snap(end.x), snap(end.y));
            Some([
                start.x.min(end.x),
                start.y.min(end.y),
                start.x.max(end.x),
                start.y.max(end.y),
            ])
        }
    }

    fn rectangle_selection(&self, bounds: [f32; 4]) -> Option<IsoPaintSelection> {
        let buffer = self.paint_surface_cache.as_ref()?;
        let projection = self.selection_projection.as_ref()?;
        let mut selection = IsoPaintSelection::new(projection.owner.clone());
        let mut extent_by_face: HashMap<[u32; 4], (i32, i32)> = HashMap::new();
        for (index, pixel) in buffer.pixels.iter().enumerate() {
            if !pixel.valid
                || !projection
                    .owner
                    .same_paint_object(&Self::owner_from_geo_id(pixel.geo_id))
            {
                continue;
            }
            let relative = Vec3::from(pixel.world) - projection.origin_world;
            if relative.dot(projection.normal).abs() > 0.75 {
                continue;
            }
            let Some(pixel_normal) = Vec3::from(pixel.normal).try_normalized() else {
                continue;
            };
            let surface = projection.surface_coord(Vec3::from(pixel.world));
            if pixel_normal.dot(projection.normal).abs() < 0.55
                || surface.x < bounds[0]
                || surface.x > bounds[2]
                || surface.y < bounds[1]
                || surface.y > bounds[3]
            {
                continue;
            }
            let x = index as i32 % buffer.width as i32;
            let y = index as i32 / buffer.width as i32;
            let (extent_x, extent_y) =
                *extent_by_face.entry(pixel.paint_geo).or_insert_with(|| {
                    Self::screen_space_transform(buffer, x, y, pixel.paint_geo).map_or(
                        (0, 0),
                        |transform| {
                            (
                                (((transform[0].abs() + transform[1].abs())
                                    * ISO_PAINT_BAKED_PIXELS_PER_UV
                                    * 0.55)
                                    .ceil() as i32)
                                    .clamp(0, 8),
                                (((transform[2].abs() + transform[3].abs())
                                    * ISO_PAINT_BAKED_PIXELS_PER_UV
                                    * 0.55)
                                    .ceil() as i32)
                                    .clamp(0, 8),
                            )
                        },
                    )
                });
            let center = [
                (pixel.uv[0] * ISO_PAINT_BAKED_PIXELS_PER_UV).round() as i32,
                (pixel.uv[1] * ISO_PAINT_BAKED_PIXELS_PER_UV).round() as i32,
            ];
            for uv_y in center[1] - extent_y..=center[1] + extent_y {
                for uv_x in center[0] - extent_x..=center[0] + extent_x {
                    selection.set_pixel(
                        Self::owner_from_geo_id(pixel.geo_id),
                        pixel.paint_geo,
                        [uv_x, uv_y],
                    );
                }
            }
        }
        (!selection.is_empty()).then_some(selection)
    }

    fn paint_viewport_size(ui: &mut TheUI, server_ctx: &ServerContext) -> Option<[i32; 2]> {
        let view_name = if server_ctx.pc.is_prefab() {
            "PrefabView"
        } else {
            "PolyView"
        };
        ui.get_render_view(view_name).map(|render_view| {
            let dim = *render_view.dim();
            [dim.width, dim.height]
        })
    }

    fn request_paint_redraw(ctx: &mut TheContext) {
        ctx.ui.redraw_all = true;
    }

    fn reset_stroke(&mut self) {
        self.painting = false;
        self.active_stroke = None;
        self.last_stamp_screen = None;
        self.stamp_clip_geo = None;
        self.stroke_before = None;
        self.stroke_changed = false;
        self.paint_surface_cache = None;
        self.selection_drag_start = None;
        self.selection_projection = None;
        self.selection_move_bounds = None;
        self.terrain_selection_start = None;
        self.terrain_stamp_last = None;
    }
}

impl Tool for IsoPaintTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            overlay: Default::default(),
            id: TheId::named("3D Paint Tool"),
            painting: false,
            previous_dock: None,
            active_stroke: None,
            last_stamp_screen: None,
            stamp_clip_geo: None,
            stroke_before: None,
            stroke_changed: false,
            paint_surface_cache: None,
            selection_drag_start: None,
            selection_projection: None,
            selection_move_bounds: None,
            selection_grid_step: 1.0,
            terrain_hover: None,
            terrain_radius: 0.5,
            terrain_selection_start: None,
            terrain_stamp_last: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_iso_paint")
    }

    fn icon_name(&self) -> String {
        "paint-brush".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('I')
    }

    fn handle_event(&mut self,event:&TheEvent,ui:&mut TheUI,ctx:&mut TheContext,project:&mut Project,server:&mut ServerContext)->bool {
        let changed=super::paint_picker::event(event,ui,ctx,project,server);
        if changed {self.overlay.sync(project,server);}
        changed
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                self.reset_stroke();
                self.terrain_hover = None;
                server_ctx.curr_map_tool_type = MapToolType::IsoPaint;
                // 3D Paint operates in every 3D camera.  Preserve the artist's current orbit
                // or first-person view; entering from 2D defaults to the familiar iso view.
                let terrain_iso = !server_ctx.pc.is_prefab()
                    && project
                        .get_map(server_ctx)
                        .is_some_and(|m| m.camera == MapCamera::TwoDIso)
                    && server_ctx.editor_view_mode == EditorViewMode::D2;
                if server_ctx.editor_view_mode == EditorViewMode::D2 && !terrain_iso {
                    server_ctx.editor_view_mode = EditorViewMode::Iso;
                }
                server_ctx.geometry_edit_mode = GeometryEditMode::Geometry;
                server_ctx.hover_cursor = None;
                server_ctx.iso_paint_hover_screen = None;
                server_ctx.iso_paint_selection_rect = None;

                let neutral_material = Self::neutral_material_palette(project);
                if let ProjectContext::Prefab(asset_id) = server_ctx.pc {
                    if let Some(map) = project.prefab_editor_map.as_mut() {
                        for object in &mut map.geometry_objects {
                            object.ensure_face_paint_data();
                        }
                        map.clear_selection();
                        map.clear_temp();
                    }
                    let paint = project.block_prop_paint.entry(asset_id).or_default();
                    Self::ensure_initial_material_settings(paint, neutral_material);
                    if matches!(paint.active_brush.as_str(), "material" | "brick")
                        && paint.active_size <= 1.001
                    {
                        paint.active_size = 8.0;
                    }
                } else if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    if server_ctx.editor_view_mode == EditorViewMode::Iso {
                        region.map.camera = MapCamera::ThreeDIso;
                    }
                    // Persist face identity and object-local paint coordinates before the first
                    // stroke. Rendering has a legacy fallback, but new paint must never depend
                    // on the object's current world transform.
                    for object in &mut region.map.geometry_objects {
                        object.ensure_face_paint_data();
                    }
                    region.map.clear_selection();
                    region.map.clear_temp();
                    Self::ensure_initial_material_settings(&mut region.iso_paint, neutral_material);
                    if matches!(region.iso_paint.active_brush.as_str(), "material" | "brick")
                        && region.iso_paint.active_size <= 1.001
                    {
                        region.iso_paint.active_size = 8.0;
                    }
                }

                super::paint_picker::activate(ui,ctx,project,server_ctx);
                self.overlay.sync(project,server_ctx);

                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
                RUSTERIX.write().unwrap().set_overlay_dirty();
                ctx.ui.redraw_all = true;
                true
            }
            DeActivate => {
                super::paint_picker::deactivate(ui, ctx);
                self.reset_stroke();
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                server_ctx.iso_paint_hover_screen = None;
                server_ctx.iso_paint_selection_rect = None;
                RUSTERIX.write().unwrap().set_overlay_dirty();
                ctx.ui.redraw_all = true;
                if !server_ctx.pc.is_prefab()
                    && DOCKMANAGER.read().unwrap().dock == "Paint"
                    && let Some(prev) = self.previous_dock.take()
                {
                    DOCKMANAGER
                        .write()
                        .unwrap()
                        .set_dock(prev, ui, ctx, project, server_ctx);
                }
                true
            }
            _ => false,
        }
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server: &mut ServerContext,
        _assets: &rusterix::Assets,
    ) {
        if server.editor_view_mode != EditorViewMode::D2 || map.camera != MapCamera::TwoDIso {
            self.overlay.draw(buffer,ctx);
            return;
        }
        let projection = rusterix::isometric::Projection::editor(
            map,
            Vec2::new(buffer.dim().width as f32, buffer.dim().height as f32),
        );
        if let Some(center) = self.terrain_hover {
            let points = super::terrain::TerrainTool::outline(map, center, self.terrain_radius);
            for pair in points.windows(2) {
                rusterix::isometric::line(
                    buffer,
                    projection.screen(Vec2::new(pair[0].x, pair[0].z), pair[0].y),
                    projection.screen(Vec2::new(pair[1].x, pair[1].z), pair[1].y),
                    [235, 205, 135, 240],
                );
            }
        }
        if let (Some(start), Some(end)) = (self.terrain_selection_start, self.terrain_hover) {
            let surface = rusterix::chunkbuilder::terrain_surface::TerrainSurface::cached(map);
            let corners = [
                start,
                Vec2::new(end.x, start.y),
                end,
                Vec2::new(start.x, end.y),
            ];
            for i in 0..4 {
                let a = corners[i];
                let b = corners[(i + 1) % 4];
                let steps = ((b - a).magnitude() * 8.).ceil().clamp(1., 2048.) as usize;
                let project = |q| {
                    projection.screen(
                        q,
                        surface.as_ref().and_then(|s| s.height_at(q)).unwrap_or(0.),
                    )
                };
                let mut previous = project(a);
                for k in 1..=steps {
                    let next = project(a + (b - a) * (k as f32 / steps as f32));
                    rusterix::isometric::line(buffer, previous, next, [135, 205, 235, 240]);
                    previous = next;
                }
            }
        }
        self.overlay.draw(buffer,ctx);
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match map_event {
            MapClicked(_) => {
                self.painting = true;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    Self::hit_status(server_ctx),
                ));
            }
            MapDragged(_) => {
                if self.painting {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        Self::hit_status(server_ctx),
                    ));
                }
            }
            MapHover(_) => {
                if !self.painting {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        Self::hit_status(server_ctx),
                    ));
                }
            }
            MapUp(_) => {
                self.painting = false;
                server_ctx.iso_paint_hover_screen = None;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
            }
            MapEscape => {
                self.painting = false;
                server_ctx.iso_paint_hover_screen = None;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
            }
            _ => {}
        }

        None
    }

    fn region_map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        region: &mut Region,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if self.overlay.map_event(&map_event,ui,ctx,server_ctx) {
            self.terrain_hover=None; server_ctx.iso_paint_hover_screen=None;
            return None;
        }
        if region.map.camera == MapCamera::TwoDIso
            && server_ctx.editor_view_mode == EditorViewMode::D2
        {
            return self.terrain_map_event(map_event, ui, ctx, region, server_ctx);
        }
        match map_event {
            MapClicked(coord) => {
                let viewport_size = Self::paint_viewport_size(ui, server_ctx);
                server_ctx.iso_paint_hover_screen = Some(coord);
                if !server_ctx.pc.is_prefab() && !self.overlay.is_active() {
                    Self::sync_live_paint_settings(ui, &mut region.iso_paint);
                }
                if region.iso_paint.active_operation == "pick" {
                    let point = Self::paint_point(coord, server_ctx, viewport_size);
                    Self::pick_paint_color(&mut region.iso_paint, &point, ctx);
                    return None;
                }
                self.painting = true;
                self.stroke_before = Some(region.iso_paint.clone());
                if region.iso_paint.active_operation == "select" {
                    self.paint_surface_cache = Self::capture_paint_surface(viewport_size);
                    let move_hit = self.paint_surface_cache.as_ref().and_then(|buffer| {
                        region.iso_paint.active_selection().and_then(|selection| {
                            Self::selection_hit_pixel(buffer, coord, selection)
                        })
                    });
                    self.selection_projection = self
                        .paint_surface_cache
                        .as_ref()
                        .and_then(|buffer| Self::selection_projection(buffer, coord, &region.map));
                    self.selection_grid_step =
                        ServerContext::edit_grid_step(region.map.subdivisions).max(0.01);
                    let selection_grid_step = self.selection_grid_step;
                    self.selection_move_bounds = self
                        .paint_surface_cache
                        .as_ref()
                        .zip(self.selection_projection.as_ref())
                        .and_then(|(buffer, projection)| {
                            region.iso_paint.active_selection().and_then(|selection| {
                                move_hit.and_then(|_| {
                                    Self::active_selection_bounds(buffer, projection, selection)
                                })
                            })
                        })
                        .map(|bounds| {
                            bounds.map(|value| {
                                (value / selection_grid_step).round() * selection_grid_step
                            })
                        });
                    self.selection_drag_start = self.selection_projection.as_ref().map(|_| coord);
                    self.active_stroke = None;
                    server_ctx.iso_paint_selection_rect = self
                        .snapped_selection_bounds(coord)
                        .and_then(|bounds| self.selection_projection.as_ref()?.outline(bounds));
                    Self::request_paint_redraw(ctx);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        if self.selection_move_bounds.is_some() {
                            fl!("status_iso_paint_selection_move")
                        } else {
                            fl!("status_iso_paint_selection_drag")
                        },
                    ));
                    return None;
                }
                if Self::is_stamp_mode(&region.iso_paint) {
                    let point = Self::paint_point(coord, server_ctx, viewport_size);
                    self.stamp_clip_geo = Self::stamp_clip_geo(&region.iso_paint, &point);
                    let clip_geo = self.stamp_clip_geo;
                    let changed = Self::apply_stamp_at(&mut region.iso_paint, point, clip_geo);
                    self.active_stroke = None;
                    self.last_stamp_screen = Some([coord.x, coord.y]);
                    self.stroke_changed = changed;
                    Self::request_paint_redraw(ctx);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        format!(
                            "{} {} stamp",
                            Self::hit_status(server_ctx),
                            Self::stamp_label(&region.iso_paint)
                        ),
                    ));
                    return None;
                }
                self.paint_surface_cache = Self::capture_paint_surface(viewport_size);
                let mut projected_points =
                    self.projected_paint_points(coord, &region.iso_paint, viewport_size);
                let point = if projected_points.is_empty() {
                    Self::paint_point(coord, server_ctx, viewport_size)
                } else {
                    projected_points.remove(0)
                };
                let stroke_id = region.iso_paint.begin_stroke(point);
                if region
                    .iso_paint
                    .append_projected_points(stroke_id, projected_points)
                {
                    self.stroke_changed = true;
                }
                let (stroke_opacity, stroke_material_mode) = region
                    .iso_paint
                    .chunks
                    .values()
                    .flat_map(|chunk| chunk.strokes.iter())
                    .find(|stroke| stroke.id == stroke_id)
                    .map(|stroke| (stroke.opacity, stroke.material_mode.clone()))
                    .unwrap_or((
                        region.iso_paint.active_opacity,
                        region.iso_paint.active_material_mode.clone(),
                    ));
                self.active_stroke = Some(stroke_id);
                self.stroke_changed = true;
                Self::request_paint_redraw(ctx);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!(
                        "{} opacity {:.3} mode {}",
                        Self::hit_status(server_ctx),
                        stroke_opacity,
                        stroke_material_mode
                    ),
                ));
            }
            MapDragged(coord) => {
                let viewport_size = Self::paint_viewport_size(ui, server_ctx);
                server_ctx.iso_paint_hover_screen = Some(coord);
                if self.painting
                    && region.iso_paint.active_operation == "select"
                    && self.selection_drag_start.is_some()
                {
                    server_ctx.iso_paint_selection_rect = self
                        .snapped_selection_bounds(coord)
                        .and_then(|bounds| self.selection_projection.as_ref()?.outline(bounds));
                    Self::request_paint_redraw(ctx);
                    return None;
                }
                if self.painting
                    && Self::is_stamp_mode(&region.iso_paint)
                    && Self::should_place_stamp(
                        self.last_stamp_screen,
                        coord,
                        region.iso_paint.active_size,
                        region.iso_paint.active_stamp_density,
                    )
                {
                    let point = Self::paint_point(coord, server_ctx, viewport_size);
                    let changed =
                        Self::apply_stamp_at(&mut region.iso_paint, point, self.stamp_clip_geo);
                    if changed {
                        self.last_stamp_screen = Some([coord.x, coord.y]);
                    }
                    self.stroke_changed |= changed;
                    Self::request_paint_redraw(ctx);
                    return None;
                }
                if self.painting
                    && let Some(stroke_id) = self.active_stroke
                {
                    let projected_points =
                        self.projected_paint_points(coord, &region.iso_paint, viewport_size);
                    let changed = if projected_points.is_empty() {
                        let point = Self::paint_point(coord, server_ctx, viewport_size);
                        region.iso_paint.append_point(stroke_id, point)
                    } else {
                        region
                            .iso_paint
                            .append_projected_points(stroke_id, projected_points)
                    };
                    if changed {
                        self.stroke_changed = true;
                    }
                    Self::request_paint_redraw(ctx);
                }
            }
            MapHover(coord) => {
                server_ctx.iso_paint_hover_screen = Some(coord);
                Self::request_paint_redraw(ctx);
            }
            MapUp(coord) => {
                let viewport_size = Self::paint_viewport_size(ui, server_ctx);
                server_ctx.iso_paint_hover_screen = Some(coord);
                if self.painting
                    && region.iso_paint.active_operation == "select"
                    && self.selection_drag_start.is_some()
                {
                    let moved_without_delta = self.selection_move_bounds.is_some()
                        && self.selection_drag_start == Some(coord);
                    if !moved_without_delta {
                        if let Some(selection) = self
                            .snapped_selection_bounds(coord)
                            .and_then(|bounds| self.rectangle_selection(bounds))
                        {
                            region.iso_paint.set_active_selection(selection);
                            self.stroke_changed = true;
                        }
                    }
                    let undo_atom = if self.stroke_changed {
                        self.stroke_before.take().map(|old_paint| {
                            ProjectUndoAtom::RegionPaintEdit(
                                ProjectContext::Region(region.id),
                                region.id,
                                Box::new(old_paint),
                                Box::new(region.iso_paint.clone()),
                            )
                        })
                    } else {
                        None
                    };
                    server_ctx.iso_paint_selection_rect = None;
                    self.reset_stroke();
                    Self::request_paint_redraw(ctx);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("status_iso_paint_selection_ready"),
                    ));
                    return undo_atom;
                }
                if self.painting
                    && Self::is_stamp_mode(&region.iso_paint)
                    && Self::should_place_stamp(
                        self.last_stamp_screen,
                        coord,
                        region.iso_paint.active_size,
                        region.iso_paint.active_stamp_density,
                    )
                {
                    let point = Self::paint_point(coord, server_ctx, viewport_size);
                    let changed =
                        Self::apply_stamp_at(&mut region.iso_paint, point, self.stamp_clip_geo);
                    self.stroke_changed |= changed;
                } else if self.painting
                    && let Some(stroke_id) = self.active_stroke
                {
                    let projected_points =
                        self.projected_paint_points(coord, &region.iso_paint, viewport_size);
                    let changed = if projected_points.is_empty() {
                        let point = Self::paint_point(coord, server_ctx, viewport_size);
                        region.iso_paint.append_point(stroke_id, point)
                    } else {
                        region
                            .iso_paint
                            .append_projected_points(stroke_id, projected_points)
                    };
                    if changed {
                        self.stroke_changed = true;
                    }
                    region.iso_paint.mark_stroke_for_screen_commit(stroke_id);
                }

                let undo_atom = if self.stroke_changed {
                    self.stroke_before.take().map(|old_paint| {
                        ProjectUndoAtom::RegionPaintEdit(
                            ProjectContext::Region(region.id),
                            region.id,
                            Box::new(old_paint),
                            Box::new(region.iso_paint.clone()),
                        )
                    })
                } else {
                    None
                };

                self.reset_stroke();
                Self::request_paint_redraw(ctx);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
                return undo_atom;
            }
            MapEscape => {
                server_ctx.iso_paint_hover_screen = None;
                server_ctx.iso_paint_selection_rect = None;
                if let Some(old_paint) = self.stroke_before.take() {
                    region.iso_paint = old_paint;
                }
                self.reset_stroke();
                Self::request_paint_redraw(ctx);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
            }
            _ => {}
        }

        None
    }

    fn prefab_map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        asset_id: Uuid,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        // Reuse the mature Region paint gesture implementation while storing
        // the resulting layer on the Prefab source instead of a region.
        let mut proxy = Region::default();
        proxy.map.subdivisions = project
            .prefab_editor_map
            .as_ref()
            .map(|map| map.subdivisions)
            .unwrap_or(1.0);
        proxy.iso_paint = project
            .block_prop_paint
            .shift_remove(&asset_id)
            .unwrap_or_default();
        let undo = self.region_map_event(map_event, ui, ctx, &mut proxy, server_ctx);
        project.block_prop_paint.insert(asset_id, proxy.iso_paint);
        match undo {
            Some(ProjectUndoAtom::RegionPaintEdit(_, _, old, new)) => {
                Some(ProjectUndoAtom::PrefabPaintEdit(
                    ProjectContext::Prefab(asset_id),
                    asset_id,
                    old,
                    new,
                ))
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod terrain_tests {
    use super::*;
    use rusterix::chunkbuilder::{
        terrain_sculpt::{SculptMode, TerrainSculpt},
        terrain_surface::TerrainSurface,
    };

    #[test]
    #[ignore = "native editor activation, undo and game rendering require Metal"]
    fn iso_terrain_stamps_native_sculpt_erase_undo_and_play() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../generated");
        let mut project = shared::project_io::decode_project(
            &std::fs::read(root.join("mz-iso2d-terrain-playground/MZTerrainPlayground.eldiron"))
                .unwrap(),
        )
        .unwrap();
        project.name = "MZ Organic Terrain Stamps / Steam Lords".into();
        let mut server = ServerContext::default();
        server.curr_region = project.regions[0].id;
        server.pc = ProjectContext::Region(server.curr_region);
        server.editor_view_mode = EditorViewMode::D2;
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1000, 700, 1.);
        let mut view = TheRenderView::new(TheId::named("PolyView"));
        view.set_dim(TheDim::sized(1000, 700), &mut ctx);
        ui.canvas.set_widget(view);
        RUSTERIX.write().unwrap().set_tiles_for_maps(
            project.tiles.clone(),
            false,
            [&project.regions[0].map],
        );
        let mut tool = IsoPaintTool::new();
        tool.tool_event(Activate, &mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(project.regions[0].map.camera, MapCamera::TwoDIso);
        assert_eq!(server.editor_view_mode, EditorViewMode::D2);
        let center = Vec2::new(28.5, 16.5);
        let size = Vec2::new(1000., 700.);
        project.regions[0].map.grid_size = 46.;
        server.center_map_at_grid_pos(size, center, &mut project.regions[0].map);
        let click = |map: &Map, at: Vec2<f32>| {
            let h = TerrainSurface::from_map(map)
                .unwrap()
                .height_at(at)
                .unwrap();
            rusterix::isometric::Projection::editor(map, size)
                .screen(at, h)
                .map(|v| v.round() as i32)
        };
        let stamps = |region: &Region| {
            region
                .iso_paint
                .chunks
                .values()
                .flat_map(|c| c.stamps.iter().cloned())
                .collect::<Vec<_>>()
        };
        for (kind, offset, color) in [
            ("grass", -1.2, [79, 119, 84, 255]),
            ("flowers", 0., [192, 209, 204, 255]),
            ("rubble", 1.2, [124, 148, 161, 255]),
        ] {
            let paint = &mut project.regions[0].iso_paint;
            paint.active_source = None; // This fixture verifies the original native generator path.
            paint.active_brush = kind.into();
            paint.active_operation = "draw".into();
            paint.active_material_mode = "stamp".into();
            paint.active_size = 1.8;
            paint.active_opacity = 1.;
            paint.active_stamp_size_jitter = 0.2;
            paint.active_stamp_density = 0.7;
            paint.active_color = color;
            paint.active_palette_colors = vec![
                color,
                [161, 159, 124, 255],
                [79, 119, 84, 255],
                [124, 148, 161, 255],
            ];
            ui.set_widget_value("3D Paint Material Mode", &mut ctx, TheValue::Int(2));
            ui.set_widget_value("3D Paint Tool Size", &mut ctx, TheValue::Float(1.8));
            let a = click(&project.regions[0].map, center + Vec2::new(offset, -0.7));
            let b = click(&project.regions[0].map, center + Vec2::new(offset, 1.));
            tool.region_map_event(
                MapClicked(a),
                &mut ui,
                &mut ctx,
                &mut project.regions[0],
                &mut server,
            );
            tool.region_map_event(
                MapDragged(b),
                &mut ui,
                &mut ctx,
                &mut project.regions[0],
                &mut server,
            );
            assert!(
                tool.region_map_event(
                    MapUp(b),
                    &mut ui,
                    &mut ctx,
                    &mut project.regions[0],
                    &mut server
                )
                .is_some()
            );
        }
        let original = stamps(&project.regions[0]);
        for kind in ["grass", "flowers", "rubble"] {
            assert!(
                original.iter().filter(|s| s.kind == kind).count() >= 3,
                "drag should interpolate {kind} stamps"
            );
        }
        assert!(project.regions[0].iso_paint.baked_chunks.is_empty());
        // Erase after changing both camera offset and zoom: old screen positions
        // must not determine which stamp is removed.
        let target = original
            .iter()
            .find(|s| s.kind == "flowers")
            .unwrap()
            .surface_uv
            .unwrap();
        project.regions[0].map.grid_size = 64.;
        server.center_map_at_grid_pos(size, Vec2::<f32>::from(target), &mut project.regions[0].map);
        let at = click(&project.regions[0].map, Vec2::from(target));
        project.regions[0].iso_paint.active_brush = "flowers".into();
        project.regions[0].iso_paint.active_operation = "erase".into();
        tool.region_map_event(
            MapClicked(at),
            &mut ui,
            &mut ctx,
            &mut project.regions[0],
            &mut server,
        );
        let undo = tool
            .region_map_event(
                MapUp(at),
                &mut ui,
                &mut ctx,
                &mut project.regions[0],
                &mut server,
            )
            .unwrap();
        let erased = stamps(&project.regions[0]);
        assert!(erased.len() < original.len());
        assert_eq!(
            erased.iter().filter(|s| s.kind != "flowers").count(),
            original.iter().filter(|s| s.kind != "flowers").count()
        );
        undo.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(stamps(&project.regions[0]), original);
        undo.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(stamps(&project.regions[0]), erased);
        undo.undo(&mut project, &mut ui, &mut ctx, &mut server);
        project.regions[0].iso_paint.active_operation = "draw".into();
        tool.region_map_event(
            MapClicked(at),
            &mut ui,
            &mut ctx,
            &mut project.regions[0],
            &mut server,
        );
        tool.region_map_event(
            MapEscape,
            &mut ui,
            &mut ctx,
            &mut project.regions[0],
            &mut server,
        );
        assert_eq!(stamps(&project.regions[0]), original);
        let out = root.join("iso-organic-stamps");
        std::fs::create_dir_all(&out).unwrap();
        let mut assets = rusterix::Assets::default();
        assets.set_tiles(project.tiles.clone());
        let mut cache = rusterix::isometric::GroundCache::default();
        server.center_map_at_grid_pos(size, center, &mut project.regions[0].map);
        for (name, mode, amount) in [
            ("before", None, 0.),
            ("raised", Some(SculptMode::Raise), 0.8),
            ("smoothed", Some(SculptMode::Smooth), 0.5),
            ("lowered", Some(SculptMode::Lower), 0.3),
        ] {
            let region = &mut project.regions[0];
            if let Some(mode) = mode {
                TerrainSculpt::dab(&mut region.map, center, 2.5, amount, mode);
            }
            assert_eq!(
                stamps(region),
                original,
                "sculpting must not rewrite stamp sources"
            );
            shared::iso_paint_terrain::sync(&mut assets, region.map.id, &mut region.iso_paint);
            assert_eq!(
                assets.terrain_paint[&region.map.id].stamps.len(),
                original.len()
            );
            let mut buffer = TheRGBABuffer::new(TheDim::sized(1000, 700));
            rusterix::isometric::draw_scene_cached(
                &region.map,
                &assets,
                &mut buffer,
                rusterix::isometric::Projection::editor(&region.map, size),
                0.,
                None,
                &Default::default(),
                Some(&mut cache),
            );
            std::fs::write(out.join(format!("{name}.png")), buffer.to_png().unwrap()).unwrap();
        }
        let spawn = center + Vec2::new(-0.5, 1.);
        let h = TerrainSurface::from_map(&project.regions[0].map)
            .unwrap()
            .height_at(spawn)
            .unwrap();
        for character in project.regions[0].characters.values_mut() {
            character.position = Vec3::new(spawn.x, h, spawn.y);
        }
        let bytes = shared::project_io::encode_project(&project).unwrap();
        let mut reopened = shared::project_io::decode_project(&bytes).unwrap();
        assert_eq!(stamps(&reopened.regions[0]), original);
        let mut runtime = rusterix::Rusterix::new_without_audio();
        runtime.set_tiles_for_maps(reopened.tiles.clone(), false, [&reopened.regions[0].map]);
        shared::rusterix_utils::setup_client(&mut runtime, &mut reopened);
        let mut game = rusterix::client::widget::game::GameWidget::new();
        game.set_camera_mode(rusterix::PlayerCamera::D2Iso);
        game.grid_size = 64.;
        game.player_pos = center;
        game.buffer = TheRGBABuffer::new(TheDim::sized(1000, 700));
        game.graphical_draw(
            &reopened.regions[0].map,
            &reopened.time,
            0,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        std::fs::write(out.join("playing.png"), game.buffer.to_png().unwrap()).unwrap();
        let with = game.buffer.pixels().to_vec();
        runtime
            .assets
            .terrain_paint
            .remove(&reopened.regions[0].map.id);
        game.graphical_draw(
            &reopened.regions[0].map,
            &reopened.time,
            0,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        assert!(
            with.chunks_exact(4)
                .zip(game.buffer.pixels().chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count()
                > 50
        );
        std::fs::write(out.join("OrganicTerrain.eldiron"), bytes).unwrap();
    }

    #[test]
    #[ignore = "native Paint tool activation, undo and rendering require Metal"]
    fn iso_terrain_paint_native_stroke_sculpt_undo_and_reopen() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../generated");
        let mut project = shared::project_io::decode_project(
            &std::fs::read(root.join("mz-iso2d-terrain-playground/MZTerrainPlayground.eldiron"))
                .unwrap(),
        )
        .unwrap();
        let mut server = ServerContext::default();
        server.curr_region = project.regions[0].id;
        server.pc = ProjectContext::Region(server.curr_region);
        server.editor_view_mode = EditorViewMode::D2;
        let size = Vec2::new(1000., 700.);
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1000, 700, 1.);
        let mut view = TheRenderView::new(TheId::named("PolyView"));
        view.set_dim(TheDim::sized(1000, 700), &mut ctx);
        ui.canvas.set_widget(view);
        RUSTERIX.write().unwrap().set_tiles_for_maps(
            project.tiles.clone(),
            false,
            [&project.regions[0].map],
        );
        let mut tool = IsoPaintTool::new();
        tool.tool_event(Activate, &mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(server.editor_view_mode, EditorViewMode::D2);
        assert_eq!(project.regions[0].map.camera, MapCamera::TwoDIso);
        let center = Vec2::new(28.5, 16.5);
        server.center_map_at_grid_pos(size, center, &mut project.regions[0].map);
        let paint = &mut project.regions[0].iso_paint;
        paint.active_size = 8.;
        paint.active_operation = "draw".into();
        paint.active_source = None; // Native paint regression; picker linkage has its own coverage.
        paint.active_brush = "material".into();
        paint.active_material_mode = "coat".into();
        paint.active_opacity = 1.;
        paint.active_color = [161, 159, 124, 255];
        paint.active_palette_indices = vec![21];
        paint.active_palette_colors = vec![paint.active_color];
        let click = |map: &Map, at: Vec2<f32>| {
            let h = TerrainSurface::from_map(map)
                .unwrap()
                .height_at(at)
                .unwrap();
            rusterix::isometric::Projection::editor(map, size)
                .screen(at, h)
                .map(|v| v.round() as i32)
        };
        let start = click(&project.regions[0].map, center);
        let end = click(&project.regions[0].map, center + Vec2::new(1., 0.));
        tool.region_map_event(
            MapClicked(start),
            &mut ui,
            &mut ctx,
            &mut project.regions[0],
            &mut server,
        );
        tool.region_map_event(
            MapDragged(end),
            &mut ui,
            &mut ctx,
            &mut project.regions[0],
            &mut server,
        );
        let undo = tool
            .region_map_event(
                MapUp(end),
                &mut ui,
                &mut ctx,
                &mut project.regions[0],
                &mut server,
            )
            .unwrap();
        let painted = project.regions[0].iso_paint.baked_chunks.clone();
        assert!(!painted.is_empty());
        let point = project.regions[0]
            .iso_paint
            .chunks
            .values()
            .flat_map(|c| &c.strokes)
            .next()
            .unwrap()
            .points[0]
            .clone();
        assert!((Vec2::<f32>::from(point.surface_uv.unwrap()) - center).magnitude() < 0.05);
        undo.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert!(project.regions[0].iso_paint.baked_chunks.is_empty());
        undo.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.regions[0].iso_paint.baked_chunks, painted);
        let out = root.join("iso-terrain-paint");
        std::fs::create_dir_all(&out).unwrap();
        let mut assets = rusterix::Assets::default();
        assets.set_tiles(project.tiles.clone());
        let mut cache = rusterix::isometric::GroundCache::default();
        for (name, sculpt) in [("painted", false), ("sculpted", true)] {
            let region = &mut project.regions[0];
            if sculpt {
                assert!(TerrainSculpt::dab(
                    &mut region.map,
                    center,
                    2.5,
                    0.8,
                    SculptMode::Raise
                ));
            }
            assert_eq!(region.iso_paint.baked_chunks, painted);
            shared::iso_paint_terrain::sync(&mut assets, region.map.id, &mut region.iso_paint);
            let mut buffer = TheRGBABuffer::new(TheDim::sized(1000, 700));
            rusterix::isometric::draw_scene_cached(
                &region.map,
                &assets,
                &mut buffer,
                rusterix::isometric::Projection::editor(&region.map, size),
                0.,
                None,
                &Default::default(),
                Some(&mut cache),
            );
            let p = click(&region.map, center);
            let at = (p.y as usize * 1000 + p.x as usize) * 4;
            assert!(
                buffer.pixels()[at] > 110,
                "paint should remain on the hill after sculpting"
            );
            tool.region_map_event(MapHover(p), &mut ui, &mut ctx, region, &mut server);
            tool.draw_hud(&mut buffer, &mut region.map, &mut ctx, &mut server, &assets);
            std::fs::write(out.join(format!("{name}.png")), buffer.to_png().unwrap()).unwrap();
        }
        // Escape cancels a new stroke without touching the previous paint.
        let at = click(&project.regions[0].map, center);
        tool.region_map_event(
            MapClicked(at),
            &mut ui,
            &mut ctx,
            &mut project.regions[0],
            &mut server,
        );
        tool.region_map_event(
            MapEscape,
            &mut ui,
            &mut ctx,
            &mut project.regions[0],
            &mut server,
        );
        assert_eq!(project.regions[0].iso_paint.baked_chunks, painted);
        // Terrain selections clip/fill in the same world coordinates.
        let before_selection = project.regions[0].iso_paint.clone();
        project.regions[0].iso_paint.active_operation = "select".into();
        let a = click(&project.regions[0].map, center - 0.2);
        let b = click(&project.regions[0].map, center + 0.2);
        tool.region_map_event(
            MapClicked(a),
            &mut ui,
            &mut ctx,
            &mut project.regions[0],
            &mut server,
        );
        assert!(
            tool.region_map_event(
                MapUp(b),
                &mut ui,
                &mut ctx,
                &mut project.regions[0],
                &mut server
            )
            .is_some()
        );
        assert!(
            project.regions[0]
                .iso_paint
                .active_selection()
                .is_some_and(|s| s.contains(
                    rusterix::isometric::paint::TERRAIN_PAINT_GEO,
                    [(center.x * 32.) as i32, (center.y * 32.) as i32]
                ))
        );
        project.regions[0].iso_paint.active_operation = "draw".into();
        project.regions[0].iso_paint.active_color = [60, 80, 160, 255];
        project.regions[0].iso_paint.active_palette_colors = vec![[60, 80, 160, 255]];
        assert!(project.regions[0].iso_paint.fill_active_selection());
        project.regions[0].iso_paint = before_selection;
        let bytes = shared::project_io::encode_project(&project).unwrap();
        let mut reopened = shared::project_io::decode_project(&bytes).unwrap();
        assert_eq!(reopened.regions[0].iso_paint.baked_chunks, painted);
        let mut runtime = rusterix::Rusterix::new_without_audio();
        runtime.set_tiles_for_maps(reopened.tiles.clone(), false, [&reopened.regions[0].map]);
        shared::rusterix_utils::setup_client(&mut runtime, &mut reopened);
        assert!(
            runtime
                .assets
                .terrain_paint
                .contains_key(&reopened.regions[0].map.id)
        );
        let mut game = rusterix::client::widget::game::GameWidget::new();
        game.set_camera_mode(rusterix::PlayerCamera::D2Iso);
        game.grid_size = 40.;
        game.player_pos = center;
        game.buffer = TheRGBABuffer::new(TheDim::sized(1000, 700));
        game.graphical_draw(
            &reopened.regions[0].map,
            &reopened.time,
            0,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        let h = TerrainSurface::from_map(&reopened.regions[0].map)
            .unwrap()
            .height_at(center)
            .unwrap();
        let p = game.isometric_projection.unwrap().screen(center, h);
        let at = (p.y.round() as usize * 1000 + p.x.round() as usize) * 4;
        assert!(
            game.buffer.pixels()[at] > 110,
            "saved paint must appear during play"
        );
        std::fs::write(out.join("playing.png"), game.buffer.to_png().unwrap()).unwrap();
        std::fs::write(out.join("PaintedTerrain.eldiron"), bytes).unwrap();
    }
}
