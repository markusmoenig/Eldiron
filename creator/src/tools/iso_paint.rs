use crate::editor::{DOCKMANAGER, RUSTERIX};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::material_library::MaterialDefinition;

const ISO_PAINT_MIN_BRUSH_SIZE: f32 = 0.05;
const ISO_PAINT_MAX_PAINT_BRUSH_SIZE: f32 = 16.0;
const ISO_PAINT_MAX_STAMP_BRUSH_SIZE: f32 = 8.0;
const ISO_PAINT_PATTERN_KIND: &str = "3D Paint Pattern Kind";
const ISO_PAINT_PATTERN_SCALE: &str = "3D Paint Pattern Scale";
const ISO_PAINT_MORTAR: &str = "3D Paint Mortar";
const ISO_PAINT_PATTERN_DETAIL: &str = "3D Paint Pattern Detail";
const ISO_PAINT_PATTERN_VARIATION: &str = "3D Paint Pattern Variation";

pub struct IsoPaintTool {
    id: TheId,
    painting: bool,
    previous_dock: Option<String>,
    active_stroke: Option<Uuid>,
    last_stamp_screen: Option<[i32; 2]>,
    stamp_clip_geo: Option<[u32; 4]>,
    stroke_before: Option<IsoPaintLayer>,
    stroke_changed: bool,
}

impl IsoPaintTool {
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
        for (index, entry) in project.art_palette.colors.iter().enumerate() {
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
        matches!(
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
        ) && layer.active_material_mode == "stamp"
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

    fn sync_live_paint_settings(ui: &mut TheUI, layer: &mut IsoPaintLayer) {
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
    }
}

impl Tool for IsoPaintTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("3D Paint Tool"),
            painting: false,
            previous_dock: None,
            active_stroke: None,
            last_stamp_screen: None,
            stamp_clip_geo: None,
            stroke_before: None,
            stroke_changed: false,
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
                server_ctx.curr_map_tool_type = MapToolType::IsoPaint;
                // 3D Paint operates in every 3D camera.  Preserve the artist's current orbit
                // or first-person view; entering from 2D defaults to the familiar iso view.
                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    server_ctx.editor_view_mode = EditorViewMode::Iso;
                }
                server_ctx.geometry_edit_mode = GeometryEditMode::Geometry;
                server_ctx.hover_cursor = None;
                server_ctx.iso_paint_hover_screen = None;

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

                if !server_ctx.pc.is_prefab() {
                    let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
                    if current_dock != "3D Paint" {
                        self.previous_dock = if current_dock.is_empty() {
                            None
                        } else {
                            Some(current_dock)
                        };
                    }
                    DOCKMANAGER.write().unwrap().set_dock(
                        "3D Paint".into(),
                        ui,
                        ctx,
                        project,
                        server_ctx,
                    );
                }

                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
                RUSTERIX.write().unwrap().set_overlay_dirty();
                ctx.ui.redraw_all = true;
                true
            }
            DeActivate => {
                self.reset_stroke();
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                server_ctx.iso_paint_hover_screen = None;
                if !server_ctx.pc.is_prefab()
                    && DOCKMANAGER.read().unwrap().dock == "3D Paint"
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
        match map_event {
            MapClicked(coord) => {
                let viewport_size = Self::paint_viewport_size(ui, server_ctx);
                server_ctx.iso_paint_hover_screen = Some(coord);
                if !server_ctx.pc.is_prefab() {
                    Self::sync_live_paint_settings(ui, &mut region.iso_paint);
                }
                self.painting = true;
                self.stroke_before = Some(region.iso_paint.clone());
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
                let point = Self::paint_point(coord, server_ctx, viewport_size);
                let stroke_id = region.iso_paint.begin_stroke(point);
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
                    let point = Self::paint_point(coord, server_ctx, viewport_size);
                    if region.iso_paint.append_point(stroke_id, point) {
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
                    let point = Self::paint_point(coord, server_ctx, viewport_size);
                    if region.iso_paint.append_point(stroke_id, point) {
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
