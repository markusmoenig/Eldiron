use crate::prelude::*;
use crate::{
    editor::RUSTERIX,
    hud::{Hud, HudMode},
};
use MapEvent::*;
use ToolEvent::*;
use rusterix::prelude::*;
use scenevm::GeoId;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WallInteractionMode {
    Build,
    Select,
    Opening,
    Brick,
    Surface,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WallBuildMode {
    Line,
    Ring,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WallStyleField {
    Masonry,
    AutoFloor,
    Height,
    Thickness,
    Curve,
    CurveSegments,
    BrickWidth,
    BrickHeight,
    MortarGap,
    Bevel,
    Irregularity,
    Damage,
    StoneVariation,
    FrameWidth,
    FrameDepth,
    ArchStones,
    SurfaceElevation,
    SurfaceThickness,
    SurfaceClearance,
}

struct HeldWallAdjustment {
    field: WallStyleField,
    delta: f32,
    pressed_at: Instant,
    last_repeat: Instant,
    previous: Option<Map>,
}

struct WallNodeDrag {
    assembly_id: Uuid,
    node_id: Uuid,
    pressed_at: Vec2<i32>,
    start_position: Vec3<f32>,
    connect_from: Option<Vec3<f32>>,
    previous: Map,
    changed: bool,
}

struct WallRingDrag {
    center: Vec3<f32>,
    assembly_id: Option<Uuid>,
    previous: Map,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WallOpeningHandle {
    Move,
    Left,
    Right,
    Bottom,
    Top,
    ArchSpring,
}

struct WallOpeningDrag {
    assembly_id: Uuid,
    span_id: Uuid,
    opening_id: Uuid,
    handle: WallOpeningHandle,
    start_coordinates: Vec2<f32>,
    original: WallOpening,
    previous: Map,
    changed: bool,
}

/// Persistent connected-wall placement and direct editing of its generated scene geometry.
/// Detailed masonry and per-brick editing layer onto the same source assemblies later.
pub struct WallTool {
    id: TheId,
    anchor: Option<Vec3<f32>>,
    hover: Option<Vec3<f32>>,
    hud: Hud,
    opening_armed: bool,
    opening_anchor: Option<(Uuid, Uuid, Vec2<f32>)>,
    opening_shape: WallOpeningShape,
    opening_surround: WallOpeningSurround,
    interaction_mode: WallInteractionMode,
    build_mode: WallBuildMode,
    build_style: WallStyle,
    build_auto_floor: bool,
    held_adjustment: Option<HeldWallAdjustment>,
    node_drag: Option<WallNodeDrag>,
    ring_drag: Option<WallRingDrag>,
    opening_drag: Option<WallOpeningDrag>,
    surface_elevation: f32,
    surface_thickness: f32,
    surface_clearance: f32,
}

impl WallTool {
    const PANEL_X: i32 = 12;
    const PANEL_Y: i32 = 30;
    const PANEL_WIDTH: i32 = 276;
    const PANEL_HEIGHT: i32 = 420;
    const PANEL_ROW_Y: i32 = 112;
    const PANEL_ROW_SPACING: i32 = 24;

    fn snap_distance(map: &Map) -> f32 {
        (ServerContext::edit_grid_step(map.subdivisions) * 0.6).max(0.025)
    }

    fn ray_plane_position(server_ctx: &ServerContext, y: f32) -> Option<Vec3<f32>> {
        let origin = server_ctx.hover_ray_origin_3d?;
        let direction = server_ctx.hover_ray_dir_3d?;
        if direction.y.abs() <= 1e-6 {
            return None;
        }
        let distance = (y - origin.y) / direction.y;
        (distance >= 0.0).then_some(origin + direction * distance)
    }

    fn raw_pointer_position(
        &self,
        ui: &mut TheUI,
        map: &Map,
        coord: Vec2<i32>,
        server_ctx: &ServerContext,
        plane_y: Option<f32>,
    ) -> Option<Vec3<f32>> {
        let raw = if server_ctx.editor_view_mode == EditorViewMode::D2 {
            let render_view = crate::utils::map_editor_render_view(ui, server_ctx)?;
            let dim = *render_view.dim();
            let point = server_ctx.local_to_map_grid(
                Vec2::new(dim.width as f32, dim.height as f32),
                coord.map(|value| value as f32),
                map,
                map.subdivisions,
            );
            Vec3::new(
                point.x,
                plane_y
                    .or(self.anchor.map(|anchor| anchor.y))
                    .unwrap_or(0.0),
                point.y,
            )
        } else if let Some(y) = plane_y.or(self.anchor.map(|anchor| anchor.y)) {
            Self::ray_plane_position(server_ctx, y)
                .or(server_ctx.hover_cursor_3d)
                .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))?
        } else {
            server_ctx
                .hover_cursor_3d
                .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))
                .or_else(|| Self::ray_plane_position(server_ctx, 0.0))?
        };
        Some(server_ctx.snap_world_point_for_edit(map, raw))
    }

    fn pointer_position(
        &self,
        ui: &mut TheUI,
        map: &Map,
        coord: Vec2<i32>,
        server_ctx: &ServerContext,
    ) -> Option<Vec3<f32>> {
        let snapped = self.raw_pointer_position(ui, map, coord, server_ctx, None)?;
        if let Some((assembly_id, node_id)) =
            map.nearest_wall_node(snapped, Self::snap_distance(map))
            && let Some(node) = map
                .wall_assembly(assembly_id)
                .and_then(|assembly| assembly.node(node_id))
        {
            return Some(node.position);
        }
        Some(snapped)
    }

    fn map_to_screen(map: &Map, dim: TheDim, point: Vec3<f32>) -> Vec2<i32> {
        let screen = Vec2::new(point.x, point.z) * map.grid_size
            + Vec2::new(dim.width as f32, dim.height as f32) / 2.0
            + Vec2::new(map.offset.x, -map.offset.y);
        screen.map(|value| value.round() as i32)
    }

    fn finish_run(&mut self, map: &mut Map) {
        self.node_drag = None;
        self.opening_drag = None;
        self.anchor = None;
        self.hover = None;
        map.curr_grid_pos_3d = None;
    }

    fn cancel_ring_drag(&mut self, map: &mut Map) {
        if let Some(drag) = self.ring_drag.take() {
            *map = drag.previous;
            map.rebuild_wall_geometry();
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
        }
    }

    fn cancel_opening(&mut self, map: &mut Map) {
        let had_preview = map.wall_opening_preview.take().is_some();
        self.opening_armed = false;
        self.opening_anchor = None;
        if had_preview {
            map.rebuild_wall_geometry();
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
        }
    }

    fn cancel_brick_preview(&mut self, map: &mut Map) {
        if map.wall_brick_preview.take().is_some() {
            map.rebuild_wall_geometry();
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
        }
    }

    fn cancel_surface_preview(&mut self, map: &mut Map) {
        if map.wall_surface_preview.take().is_some() {
            map.rebuild_wall_geometry();
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
        }
    }

    fn panel_rect() -> TheDim {
        TheDim::rect(
            Self::PANEL_X,
            Self::PANEL_Y,
            Self::PANEL_WIDTH,
            Self::PANEL_HEIGHT,
        )
    }

    fn panel_mode_rect(index: i32) -> TheDim {
        TheDim::rect(Self::PANEL_X + 10 + index * 51, Self::PANEL_Y + 34, 49, 26)
    }

    fn panel_build_mode_rect(index: i32) -> TheDim {
        TheDim::rect(
            Self::PANEL_X + 14 + index * 125,
            Self::PANEL_Y + 76,
            121,
            28,
        )
    }

    fn update_ring_preview(
        map: &mut Map,
        drag: &mut WallRingDrag,
        style: &WallStyle,
        auto_floor: bool,
        radius: f32,
    ) -> bool {
        let positions = [
            drag.center + Vec3::new(radius, 0.0, 0.0),
            drag.center + Vec3::new(0.0, 0.0, radius),
            drag.center + Vec3::new(-radius, 0.0, 0.0),
            drag.center + Vec3::new(0.0, 0.0, -radius),
        ];
        let assembly_id = if let Some(id) = drag.assembly_id {
            id
        } else {
            let mut assembly = WallAssembly::new(format!("Ring {}", map.wall_assemblies.len() + 1));
            assembly.style = style.clone();
            assembly.auto_floor = auto_floor;
            let nodes = positions.map(|position| assembly.add_node(position));
            for index in 0..4 {
                let Ok(span_id) = assembly.add_span(nodes[index], nodes[(index + 1) % 4]) else {
                    return false;
                };
                if let Some(span) = assembly.span_mut(span_id) {
                    span.curve_offset = -0.585_786_4 * radius;
                    span.curve_segments = 12;
                }
            }
            let id = assembly.id;
            map.wall_assemblies.push(assembly);
            drag.assembly_id = Some(id);
            id
        };

        let Some(assembly) = map.wall_assembly_mut(assembly_id) else {
            return false;
        };
        for (node, position) in assembly.nodes.iter_mut().zip(positions) {
            node.position = position;
        }
        for span in &mut assembly.spans {
            span.curve_offset = -0.585_786_4 * radius;
        }
        map.rebuild_wall_geometry();
        map.clear_selection();
        map.selected_wall_assembly = Some(assembly_id);
        if let Some((node_ids, span_ids)) = map.wall_assembly(assembly_id).map(|assembly| {
            (
                assembly
                    .nodes
                    .iter()
                    .map(|node| node.id)
                    .collect::<Vec<_>>(),
                assembly
                    .spans
                    .iter()
                    .map(|span| span.id)
                    .collect::<Vec<_>>(),
            )
        }) {
            map.selected_wall_nodes = node_ids;
            map.selected_wall_spans = span_ids;
        }
        true
    }

    fn panel_adjust_rect(&self, row: i32, plus: bool) -> TheDim {
        let row_offset = if self.interaction_mode == WallInteractionMode::Opening {
            2
        } else {
            0
        };
        TheDim::rect(
            Self::PANEL_X + if plus { 238 } else { 184 },
            Self::PANEL_Y + Self::PANEL_ROW_Y + (row + row_offset) * Self::PANEL_ROW_SPACING,
            28,
            20,
        )
    }

    fn panel_shape_rect(index: i32) -> TheDim {
        TheDim::rect(
            Self::PANEL_X + 116 + index * 76,
            Self::PANEL_Y + Self::PANEL_ROW_Y,
            70,
            24,
        )
    }

    fn panel_surround_rect(index: i32) -> TheDim {
        TheDim::rect(
            Self::PANEL_X + 78 + index * 62,
            Self::PANEL_Y + Self::PANEL_ROW_Y + Self::PANEL_ROW_SPACING,
            57,
            20,
        )
    }

    fn visible_style_fields(&self) -> &'static [(WallStyleField, &'static str)] {
        match self.interaction_mode {
            WallInteractionMode::Build => &[
                (WallStyleField::Masonry, "Masonry"),
                (WallStyleField::AutoFloor, "Create floor"),
                (WallStyleField::Height, "Height"),
                (WallStyleField::Thickness, "Thickness"),
                (WallStyleField::BrickWidth, "Stone width"),
                (WallStyleField::BrickHeight, "Course height"),
                (WallStyleField::MortarGap, "Mortar gap"),
                (WallStyleField::StoneVariation, "Stone variation"),
            ],
            WallInteractionMode::Select => &[
                (WallStyleField::Masonry, "Masonry"),
                (WallStyleField::AutoFloor, "Floor"),
                (WallStyleField::Height, "Height"),
                (WallStyleField::Thickness, "Thickness"),
                (WallStyleField::Curve, "Curve"),
                (WallStyleField::CurveSegments, "Curve detail"),
                (WallStyleField::MortarGap, "Mortar gap"),
                (WallStyleField::Bevel, "Bevel"),
                (WallStyleField::Irregularity, "Irregularity"),
                (WallStyleField::Damage, "Damage"),
                (WallStyleField::StoneVariation, "Stone variation"),
            ],
            WallInteractionMode::Brick => &[
                (WallStyleField::Bevel, "Bevel"),
                (WallStyleField::Irregularity, "Irregularity"),
                (WallStyleField::Damage, "Damage"),
                (WallStyleField::StoneVariation, "Stone variation"),
            ],
            WallInteractionMode::Opening => &[
                (WallStyleField::FrameWidth, "Surround width"),
                (WallStyleField::FrameDepth, "Relief depth"),
                (WallStyleField::ArchStones, "Arch stones"),
            ],
            WallInteractionMode::Surface => &[
                (WallStyleField::SurfaceElevation, "Elevation"),
                (WallStyleField::SurfaceThickness, "Thickness"),
                (WallStyleField::SurfaceClearance, "Clearance"),
            ],
        }
    }

    fn selected_wall_plane_coordinates(
        map: &Map,
        server_ctx: &ServerContext,
    ) -> Option<(Uuid, Uuid, Vec2<f32>)> {
        let assembly_id = map.selected_wall_assembly?;
        let span_id = *map.selected_wall_spans.first()?;
        if let Some(GeoId::GeometryObject(object_id)) = server_ctx.geo_hit
            && map.wall_source_for_geometry_object(object_id) == Some((assembly_id, span_id))
        {
            let assembly = map.wall_assembly(assembly_id)?;
            let span = assembly.span(span_id)?;
            let style = span.style_override.as_ref().unwrap_or(&assembly.style);
            let mut coordinates = assembly.span_coordinates(span_id, server_ctx.geo_hit_pos)?;
            coordinates.y = coordinates.y.clamp(0.0, style.height);
            return Some((
                assembly_id,
                span_id,
                Self::snap_opening_coordinates(map, coordinates),
            ));
        }
        let assembly = map.wall_assembly(assembly_id)?;
        let span = assembly.span(span_id)?;
        let start = assembly.node(span.start_node)?.position;
        let end = assembly.node(span.end_node)?.position;
        let horizontal = Vec3::new(end.x - start.x, 0.0, end.z - start.z);
        let normal = Vec3::new(-horizontal.z, 0.0, horizontal.x).try_normalized()?;
        let ray_origin = server_ctx.hover_ray_origin_3d?;
        let ray_direction = server_ctx.hover_ray_dir_3d?;
        let denominator = ray_direction.dot(normal);
        if denominator.abs() <= 1e-6 {
            return None;
        }
        let distance = (start - ray_origin).dot(normal) / denominator;
        if distance < 0.0 {
            return None;
        }
        let point = ray_origin + ray_direction * distance;
        let mut coordinates = assembly.span_coordinates(span_id, point)?;
        let height = span
            .style_override
            .as_ref()
            .unwrap_or(&assembly.style)
            .height;
        coordinates.y = coordinates.y.clamp(0.0, height);
        Some((
            assembly_id,
            span_id,
            Self::snap_opening_coordinates(map, coordinates),
        ))
    }

    fn opening_pointer_coordinates(
        &self,
        map: &Map,
        server_ctx: &ServerContext,
    ) -> Option<(Uuid, Uuid, Vec2<f32>)> {
        Self::selected_wall_plane_coordinates(map, server_ctx)
    }

    fn opening_handles(opening: &WallOpening) -> Vec<(WallOpeningHandle, Vec2<f32>)> {
        let left = opening.center - opening.width * 0.5;
        let right = opening.center + opening.width * 0.5;
        let top = opening.bottom + opening.height;
        let middle = opening.bottom + opening.height * 0.5;
        let mut handles = vec![
            (WallOpeningHandle::Left, Vec2::new(left, middle)),
            (WallOpeningHandle::Right, Vec2::new(right, middle)),
            (
                WallOpeningHandle::Bottom,
                Vec2::new(opening.center, opening.bottom),
            ),
            (WallOpeningHandle::Top, Vec2::new(opening.center, top)),
        ];
        if opening.shape == WallOpeningShape::Arch {
            handles.push((
                WallOpeningHandle::ArchSpring,
                Vec2::new(opening.center, top - opening.effective_arch_radius()),
            ));
        }
        handles
    }

    fn selected_opening_handle_at(map: &Map, coordinates: Vec2<f32>) -> Option<WallOpeningHandle> {
        let assembly_id = map.selected_wall_assembly?;
        let span_id = *map.selected_wall_spans.first()?;
        let opening_id = map.selected_wall_opening?;
        let opening = map
            .wall_assembly(assembly_id)?
            .opening(span_id, opening_id)?;
        let threshold = (ServerContext::edit_grid_step(map.subdivisions) * 0.45).max(0.08);
        Self::opening_handles(opening)
            .into_iter()
            .filter_map(|(handle, point)| {
                let distance = (point - coordinates).magnitude_squared();
                (distance <= threshold.powi(2)).then_some((handle, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(handle, _)| handle)
    }

    fn update_opening_drag(map: &mut Map, drag: &WallOpeningDrag, coordinates: Vec2<f32>) -> bool {
        let Some(assembly) = map.wall_assembly(drag.assembly_id) else {
            return false;
        };
        let Some(span) = assembly.span(drag.span_id) else {
            return false;
        };
        let style = span.style_override.as_ref().unwrap_or(&assembly.style);
        let wall_height = style.height;
        let Some(span_length) = assembly.span_length(drag.span_id) else {
            return false;
        };
        let delta = coordinates - drag.start_coordinates;
        let mut opening = drag.original.clone();
        let minimum_size = 0.05;
        if span_length < minimum_size || wall_height < minimum_size {
            return false;
        }
        opening.width = opening.width.min(span_length).max(minimum_size);
        opening.height = opening.height.min(wall_height).max(minimum_size);
        let original_left = drag.original.center - drag.original.width * 0.5;
        let original_right = drag.original.center + drag.original.width * 0.5;
        let original_top = drag.original.bottom + drag.original.height;
        match drag.handle {
            WallOpeningHandle::Move => {
                opening.center = (drag.original.center + delta.x)
                    .clamp(opening.width * 0.5, span_length - opening.width * 0.5);
                opening.bottom = (drag.original.bottom + delta.y)
                    .clamp(0.0, (wall_height - opening.height).max(0.0));
            }
            WallOpeningHandle::Left => {
                let left = (original_left + delta.x).clamp(0.0, original_right - minimum_size);
                opening.width = original_right - left;
                opening.center = (left + original_right) * 0.5;
            }
            WallOpeningHandle::Right => {
                let right =
                    (original_right + delta.x).clamp(original_left + minimum_size, span_length);
                opening.width = right - original_left;
                opening.center = (original_left + right) * 0.5;
            }
            WallOpeningHandle::Bottom => {
                opening.bottom =
                    (drag.original.bottom + delta.y).clamp(0.0, original_top - minimum_size);
                opening.height = original_top - opening.bottom;
            }
            WallOpeningHandle::Top => {
                let top = (original_top + delta.y)
                    .clamp(drag.original.bottom + minimum_size, wall_height);
                opening.height = top - drag.original.bottom;
            }
            WallOpeningHandle::ArchSpring => {
                let original_spring = original_top - drag.original.effective_arch_radius();
                opening.arch_radius = Some(
                    (original_top - (original_spring + delta.y))
                        .clamp(minimum_size, (opening.width * 0.5).min(opening.height)),
                );
            }
        }
        if let Some(radius) = opening.arch_radius {
            opening.arch_radius =
                Some(radius.clamp(minimum_size, (opening.width * 0.5).min(opening.height)));
        }
        let Some(target) = map
            .wall_assembly_mut(drag.assembly_id)
            .and_then(|assembly| assembly.opening_mut(drag.span_id, drag.opening_id))
        else {
            return false;
        };
        if *target == opening {
            return false;
        }
        *target = opening;
        true
    }

    fn brick_pointer(
        map: &Map,
        server_ctx: &ServerContext,
    ) -> Option<(Uuid, Uuid, WallBrickKey, Vec2<f32>)> {
        let assembly_id = map.selected_wall_assembly?;
        let span_id = *map.selected_wall_spans.first()?;
        let assembly = map.wall_assembly(assembly_id)?;
        if let Some(GeoId::GeometryObject(object_id)) = server_ctx.geo_hit
            && map.wall_source_for_geometry_object(object_id) == Some((assembly_id, span_id))
        {
            let coordinates = assembly.span_coordinates(span_id, server_ctx.geo_hit_pos)?;
            let key = assembly.brick_at(span_id, coordinates)?;
            return Some((assembly_id, span_id, key, coordinates));
        }
        let span = assembly.span(span_id)?;
        let start = assembly.node(span.start_node)?.position;
        let end = assembly.node(span.end_node)?.position;
        let horizontal = Vec3::new(end.x - start.x, 0.0, end.z - start.z);
        let normal = Vec3::new(-horizontal.z, 0.0, horizontal.x).try_normalized()?;
        let ray_origin = server_ctx.hover_ray_origin_3d?;
        let ray_direction = server_ctx.hover_ray_dir_3d?;
        let denominator = ray_direction.dot(normal);
        if denominator.abs() <= 1e-6 {
            return None;
        }
        let distance = (start - ray_origin).dot(normal) / denominator;
        if distance < 0.0 {
            return None;
        }
        let coordinates =
            assembly.span_coordinates(span_id, ray_origin + ray_direction * distance)?;
        let key = assembly.brick_at(span_id, coordinates)?;
        Some((assembly_id, span_id, key, coordinates))
    }

    fn snap_opening_coordinates(map: &Map, coordinates: Vec2<f32>) -> Vec2<f32> {
        let step = ServerContext::edit_grid_step(map.subdivisions).max(0.001);
        (coordinates / step).map(f32::round) * step
    }

    fn set_interaction_mode(
        &mut self,
        mode: WallInteractionMode,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        self.cancel_ring_drag(map);
        if mode == WallInteractionMode::Opening {
            if server_ctx.editor_view_mode == EditorViewMode::D2 {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Openings are edited directly on the wall in the 3D view.".to_string(),
                ));
                return;
            }
            if map.selected_wall_assembly.is_none() || map.selected_wall_spans.is_empty() {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Select a wall span before editing its openings.".to_string(),
                ));
                return;
            }
            self.cancel_brick_preview(map);
            self.finish_run(map);
            self.opening_armed = false;
            self.opening_anchor = None;
            map.wall_opening_preview = None;
            map.selected_wall_surface = None;
            self.interaction_mode = WallInteractionMode::Opening;
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                "Opening mode: click an opening to edit it, or click empty wall to create one."
                    .to_string(),
            ));
            ctx.ui.redraw_all = true;
            return;
        }
        if mode == WallInteractionMode::Surface {
            self.cancel_brick_preview(map);
            self.finish_run(map);
            self.opening_armed = false;
            self.opening_anchor = None;
            map.wall_opening_preview = None;
            self.interaction_mode = WallInteractionMode::Surface;
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                "Surface mode: click a bounded wall area to create or edit its fitted surface."
                    .to_string(),
            ));
            ctx.ui.redraw_all = true;
            return;
        }
        if mode == WallInteractionMode::Brick {
            if server_ctx.editor_view_mode == EditorViewMode::D2 {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Bricks are edited directly on the wall in the 3D view.".to_string(),
                ));
                return;
            }
            if map.selected_wall_assembly.is_none() || map.selected_wall_spans.is_empty() {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Select a wall span before editing its bricks.".to_string(),
                ));
                return;
            }
        }
        if self.opening_armed {
            self.cancel_opening(map);
        }
        if mode == WallInteractionMode::Build
            && let Some(assembly_id) = map.selected_wall_assembly
            && let Some(span_id) = map.selected_wall_spans.first().copied()
            && let Some(assembly) = map.wall_assembly(assembly_id)
            && let Some(span) = assembly.span(span_id)
        {
            self.build_style = span
                .style_override
                .clone()
                .unwrap_or_else(|| assembly.style.clone());
        }
        self.cancel_brick_preview(map);
        self.cancel_surface_preview(map);
        map.selected_wall_opening = None;
        map.selected_wall_surface = None;
        self.interaction_mode = mode;
        map.hovered_wall_span = None;
        self.finish_run(map);
        RUSTERIX.write().unwrap().set_overlay_dirty();
        let message = match mode {
            WallInteractionMode::Build => {
                "Build mode: click points to add walls, or drag an existing node to reshape its connected walls."
            }
            WallInteractionMode::Select => {
                "Select mode: click the visible wall span you want to modify."
            }
            WallInteractionMode::Brick => {
                "Brick mode: hover a brick for a live removal preview; click to remove or restore it."
            }
            WallInteractionMode::Opening | WallInteractionMode::Surface => unreachable!(),
        };
        ctx.ui
            .send(TheEvent::SetStatusText(TheId::empty(), message.to_string()));
        ctx.ui.redraw_all = true;
    }

    fn adjust_selected_span_style(
        &mut self,
        map: &mut Map,
        server_ctx: &ServerContext,
        field: WallStyleField,
        delta: f32,
    ) -> Option<ProjectUndoAtom> {
        if self.interaction_mode == WallInteractionMode::Build {
            if field == WallStyleField::AutoFloor {
                self.build_auto_floor = !self.build_auto_floor;
            } else {
                Self::adjust_style_value(&mut self.build_style, field, delta);
            }
            return None;
        }
        let assembly_id = map.selected_wall_assembly?;
        let previous = map.clone();
        if field == WallStyleField::AutoFloor {
            let assembly = map.wall_assembly_mut(assembly_id)?;
            assembly.auto_floor = !assembly.auto_floor;
            map.rebuild_wall_geometry();
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
            return Some(ProjectUndoAtom::MapEdit(
                server_ctx.pc,
                Box::new(previous),
                Box::new(map.clone()),
            ));
        }
        let span_ids = map.selected_wall_spans.clone();
        if span_ids.is_empty() {
            return None;
        }
        let assembly = map.wall_assembly_mut(assembly_id)?;
        let assembly_style = assembly.style.clone();
        let mut changed = false;
        for span_id in span_ids {
            let Some(span) = assembly.span_mut(span_id) else {
                continue;
            };
            let inherited = span
                .style_override
                .clone()
                .unwrap_or_else(|| assembly_style.clone());
            match field {
                WallStyleField::Curve => {
                    span.curve_offset = (span.curve_offset + delta).clamp(-100.0, 100.0)
                }
                WallStyleField::CurveSegments => {
                    span.curve_segments =
                        ((span.curve_segments as f32 + delta).round() as i32).clamp(2, 64) as u16
                }
                _ => Self::adjust_style_value(
                    span.style_override.get_or_insert(inherited),
                    field,
                    delta,
                ),
            }
            changed = true;
        }
        if !changed {
            return None;
        }
        map.rebuild_wall_geometry();
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.set_dirty();
        rusterix.set_overlay_dirty();
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(previous),
            Box::new(map.clone()),
        ))
    }

    fn adjust_wall_field(
        &mut self,
        map: &mut Map,
        server_ctx: &ServerContext,
        field: WallStyleField,
        delta: f32,
    ) -> Option<ProjectUndoAtom> {
        if self.interaction_mode == WallInteractionMode::Surface
            && matches!(
                field,
                WallStyleField::SurfaceElevation
                    | WallStyleField::SurfaceThickness
                    | WallStyleField::SurfaceClearance
            )
        {
            return self.adjust_area_surface(map, server_ctx, field, delta);
        }
        if self.interaction_mode != WallInteractionMode::Opening
            || map.selected_wall_opening.is_none()
            || !matches!(
                field,
                WallStyleField::FrameWidth
                    | WallStyleField::FrameDepth
                    | WallStyleField::ArchStones
            )
        {
            return self.adjust_selected_span_style(map, server_ctx, field, delta);
        }
        let assembly_id = map.selected_wall_assembly?;
        let span_id = *map.selected_wall_spans.first()?;
        let opening_id = map.selected_wall_opening?;
        let previous = map.clone();
        let (width, depth, stones) = {
            let assembly = map.wall_assembly(assembly_id)?;
            let span = assembly.span(span_id)?;
            let style = span.style_override.as_ref().unwrap_or(&assembly.style);
            let opening = assembly.opening(span_id, opening_id)?;
            (
                opening.frame.width(style),
                opening.frame.depth(style),
                opening.frame.arch_stones(style),
            )
        };
        let opening = map
            .wall_assembly_mut(assembly_id)?
            .opening_mut(span_id, opening_id)?;
        match field {
            WallStyleField::FrameWidth => {
                opening.frame.width = Some((width + delta).clamp(0.0, 2.0))
            }
            WallStyleField::FrameDepth => {
                opening.frame.depth = Some((depth + delta).clamp(0.0, 1.0))
            }
            WallStyleField::ArchStones => {
                opening.frame.arch_stones =
                    Some(((stones as f32 + delta).round() as i32).clamp(3, 32) as u16)
            }
            _ => return None,
        }
        map.rebuild_wall_geometry();
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.set_dirty();
        rusterix.set_overlay_dirty();
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(previous),
            Box::new(map.clone()),
        ))
    }

    fn adjust_area_surface(
        &mut self,
        map: &mut Map,
        server_ctx: &ServerContext,
        field: WallStyleField,
        delta: f32,
    ) -> Option<ProjectUndoAtom> {
        let previous = map.clone();
        let selected = map.selected_wall_assembly.zip(map.selected_wall_surface);
        let mut changed_persistent = false;
        if let Some((assembly_id, surface_id)) = selected
            && let Some(surface) = map
                .wall_assembly_mut(assembly_id)
                .and_then(|assembly| assembly.area_surface_mut(surface_id))
        {
            match field {
                WallStyleField::SurfaceElevation => {
                    surface.elevation = (surface.elevation + delta).clamp(-100.0, 100.0)
                }
                WallStyleField::SurfaceThickness => {
                    surface.thickness = (surface.thickness + delta).clamp(0.005, 10.0)
                }
                WallStyleField::SurfaceClearance => {
                    surface.clearance = (surface.clearance + delta).clamp(0.0, 10.0)
                }
                _ => return None,
            }
            self.surface_elevation = surface.elevation;
            self.surface_thickness = surface.thickness;
            self.surface_clearance = surface.clearance;
            changed_persistent = true;
        } else {
            match field {
                WallStyleField::SurfaceElevation => {
                    self.surface_elevation = (self.surface_elevation + delta).clamp(-100.0, 100.0)
                }
                WallStyleField::SurfaceThickness => {
                    self.surface_thickness = (self.surface_thickness + delta).clamp(0.005, 10.0)
                }
                WallStyleField::SurfaceClearance => {
                    self.surface_clearance = (self.surface_clearance + delta).clamp(0.0, 10.0)
                }
                _ => return None,
            }
            if let Some(preview) = map.wall_surface_preview.as_mut() {
                preview.surface.elevation = self.surface_elevation;
                preview.surface.thickness = self.surface_thickness;
                preview.surface.clearance = self.surface_clearance;
            }
        }
        if changed_persistent {
            map.rebuild_wall_geometry();
        } else if map.wall_surface_preview.is_some() {
            map.rebuild_wall_geometry_with_surface_preview();
        }
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.set_dirty();
        rusterix.set_overlay_dirty();
        changed_persistent.then(|| {
            ProjectUndoAtom::MapEdit(server_ctx.pc, Box::new(previous), Box::new(map.clone()))
        })
    }

    fn adjust_style_value(style: &mut WallStyle, field: WallStyleField, delta: f32) {
        match field {
            WallStyleField::Masonry => {
                style.masonry = style.masonry.offset(if delta >= 0.0 { 1 } else { -1 })
            }
            WallStyleField::AutoFloor => {}
            WallStyleField::Height => style.height = (style.height + delta).clamp(0.1, 100.0),
            WallStyleField::Thickness => {
                style.thickness = (style.thickness + delta).clamp(0.02, 10.0)
            }
            WallStyleField::BrickWidth => {
                style.brick_width = (style.brick_width + delta).clamp(0.05, 10.0)
            }
            WallStyleField::BrickHeight => {
                style.brick_height = (style.brick_height + delta).clamp(0.05, 10.0)
            }
            WallStyleField::MortarGap => {
                style.mortar_gap = (style.mortar_gap + delta).clamp(0.0, 1.0)
            }
            WallStyleField::Bevel => style.bevel = (style.bevel + delta).clamp(0.0, 1.0),
            WallStyleField::Irregularity => {
                style.irregularity = (style.irregularity + delta).clamp(0.0, 1.0)
            }
            WallStyleField::Damage => style.damage = (style.damage + delta).clamp(0.0, 1.0),
            WallStyleField::StoneVariation => {
                style.stone_variation = (style.stone_variation + delta).clamp(0.0, 1.0)
            }
            WallStyleField::FrameWidth => {
                style.frame_width = (style.frame_width + delta).clamp(0.0, 2.0)
            }
            WallStyleField::FrameDepth => {
                style.frame_depth = (style.frame_depth + delta).clamp(0.0, 1.0)
            }
            WallStyleField::ArchStones => {
                style.arch_stones =
                    ((style.arch_stones as f32 + delta).round() as i32).clamp(3, 32) as u16
            }
            WallStyleField::Curve | WallStyleField::CurveSegments => {}
            WallStyleField::SurfaceElevation
            | WallStyleField::SurfaceThickness
            | WallStyleField::SurfaceClearance => {}
        }
    }

    fn adjustment_amount(map: &Map, field: WallStyleField) -> f32 {
        let step = ServerContext::edit_grid_step(map.subdivisions).max(0.025);
        match field {
            WallStyleField::Masonry | WallStyleField::AutoFloor => 1.0,
            WallStyleField::Height
            | WallStyleField::BrickWidth
            | WallStyleField::BrickHeight
            | WallStyleField::Curve => step,
            WallStyleField::Thickness => step.min(0.25),
            WallStyleField::CurveSegments | WallStyleField::ArchStones => 1.0,
            WallStyleField::MortarGap
            | WallStyleField::Bevel
            | WallStyleField::FrameWidth
            | WallStyleField::FrameDepth => (step * 0.25).min(0.05),
            WallStyleField::Irregularity
            | WallStyleField::Damage
            | WallStyleField::StoneVariation => 0.05,
            WallStyleField::SurfaceElevation => 0.25,
            WallStyleField::SurfaceThickness | WallStyleField::SurfaceClearance => {
                (step * 0.25).min(0.05)
            }
        }
    }

    fn repeat_held_adjustment(
        &mut self,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        let now = Instant::now();
        let Some(held) = self.held_adjustment.as_ref() else {
            return false;
        };
        if now.duration_since(held.pressed_at) < Duration::from_millis(350)
            || now.duration_since(held.last_repeat) < Duration::from_millis(70)
        {
            return false;
        }
        let field = held.field;
        let delta = held.delta;
        if let Some(held) = self.held_adjustment.as_mut() {
            held.last_repeat = now;
        }
        let _ = self.adjust_wall_field(map, server_ctx, field, delta);
        ctx.ui.redraw_all = true;
        true
    }

    fn wall_span_at_pointer(
        map: &Map,
        server_ctx: &ServerContext,
        point: Vec3<f32>,
    ) -> Option<(Uuid, Uuid)> {
        if let Some(GeoId::GeometryObject(object_id)) = server_ctx.geo_hit
            && let Some(source) = Self::editable_wall_source_for_geometry_object(map, object_id)
        {
            return Some(source);
        }
        (server_ctx.editor_view_mode == EditorViewMode::D2)
            .then(|| map.nearest_wall_span(point, Self::snap_distance(map).max(0.2)))
            .flatten()
    }

    /// Generated floors retain their wall source IDs for materials and rebuilds,
    /// but clicking one must behave like clicking empty construction space. If
    /// floors participate in span hit-testing, every point in the room projects
    /// onto the floor's nominal source span and can select one of its endpoints.
    fn editable_wall_source_for_geometry_object(
        map: &Map,
        object_id: Uuid,
    ) -> Option<(Uuid, Uuid)> {
        let object = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)?;
        if object.properties.get_bool_default("wall_auto_floor", false) {
            return None;
        }
        map.wall_source_for_geometry_object(object_id)
    }

    fn wall_node_at_pointer(
        map: &Map,
        server_ctx: &ServerContext,
        point: Vec3<f32>,
    ) -> Option<(Uuid, Uuid, Option<Uuid>)> {
        let threshold = Self::snap_distance(map).max(0.16);
        if let Some(GeoId::GeometryObject(object_id)) = server_ctx.geo_hit
            && let Some((assembly_id, span_id)) =
                Self::editable_wall_source_for_geometry_object(map, object_id)
            && let Some(assembly) = map.wall_assembly(assembly_id)
            && let Some(span) = assembly.span(span_id)
            && let Some(length) = assembly.span_length(span_id)
            && let Some(coordinates) = assembly.span_coordinates(span_id, server_ctx.geo_hit_pos)
        {
            if coordinates.x <= threshold {
                return Some((assembly_id, span.start_node, Some(span_id)));
            }
            if length - coordinates.x <= threshold {
                return Some((assembly_id, span.end_node, Some(span_id)));
            }
        }
        map.nearest_wall_node(point, threshold)
            .map(|(assembly_id, node_id)| (assembly_id, node_id, None))
    }

    fn closest_span_endpoint(
        map: &Map,
        assembly_id: Uuid,
        span_id: Uuid,
        point: Vec3<f32>,
    ) -> Option<Vec3<f32>> {
        let assembly = map.wall_assembly(assembly_id)?;
        let span = assembly.span(span_id)?;
        let start = assembly.node(span.start_node)?.position;
        let end = assembly.node(span.end_node)?.position;
        Some(
            if (point - start).magnitude_squared() <= (point - end).magnitude_squared() {
                start
            } else {
                end
            },
        )
    }

    fn select_span(map: &mut Map, assembly_id: Uuid, span_id: Uuid, additive: bool) {
        let keep_existing = additive && map.selected_wall_assembly == Some(assembly_id);
        if !keep_existing {
            map.clear_selection();
            map.selected_wall_assembly = Some(assembly_id);
            map.selected_wall_spans.push(span_id);
        } else if let Some(index) = map
            .selected_wall_spans
            .iter()
            .position(|selected| *selected == span_id)
        {
            map.selected_wall_spans.remove(index);
        } else {
            map.selected_wall_spans.push(span_id);
        }
        map.selected_wall_opening = None;
        map.selected_wall_surface = None;
        let selected_nodes = map.wall_assembly(assembly_id).map(|assembly| {
            let mut nodes = Vec::new();
            for selected_span in &map.selected_wall_spans {
                if let Some(span) = assembly.span(*selected_span) {
                    nodes.extend([span.start_node, span.end_node]);
                }
            }
            nodes.sort_unstable();
            nodes.dedup();
            nodes
        });
        map.selected_wall_nodes = selected_nodes.unwrap_or_default();
        if map.selected_wall_spans.is_empty() {
            map.selected_wall_assembly = None;
        }
    }

    fn select_node(
        map: &mut Map,
        assembly_id: Uuid,
        node_id: Uuid,
        preferred_span: Option<Uuid>,
    ) -> Option<Vec3<f32>> {
        let (position, span_id) = {
            let assembly = map.wall_assembly(assembly_id)?;
            let position = assembly.node(node_id)?.position;
            let span_id = preferred_span
                .filter(|span_id| {
                    assembly
                        .span(*span_id)
                        .is_some_and(|span| span.start_node == node_id || span.end_node == node_id)
                })
                .or_else(|| assembly.connected_spans(node_id).next().map(|span| span.id));
            (position, span_id)
        };
        map.clear_selection();
        map.selected_wall_assembly = Some(assembly_id);
        map.selected_wall_nodes.push(node_id);
        if let Some(span_id) = span_id {
            map.selected_wall_spans.push(span_id);
        }
        Some(position)
    }

    fn select_area_surface(map: &mut Map, assembly_id: Uuid, surface_id: Uuid) -> bool {
        let Some(boundary) = map
            .wall_assembly(assembly_id)
            .and_then(|assembly| assembly.area_surface(surface_id))
            .map(|surface| surface.boundary.clone())
        else {
            return false;
        };
        map.clear_selection();
        map.selected_wall_assembly = Some(assembly_id);
        map.selected_wall_surface = Some(surface_id);
        map.selected_wall_spans = boundary.iter().map(|edge| edge.span_id).collect();
        map.selected_wall_spans.sort_unstable();
        map.selected_wall_spans.dedup();
        map.selected_wall_nodes = map
            .wall_assembly(assembly_id)
            .map(|assembly| {
                let mut nodes = boundary
                    .iter()
                    .filter_map(|edge| assembly.span(edge.span_id))
                    .flat_map(|span| [span.start_node, span.end_node])
                    .collect::<Vec<_>>();
                nodes.sort_unstable();
                nodes.dedup();
                nodes
            })
            .unwrap_or_default();
        true
    }

    fn surface_hit(map: &Map, server_ctx: &ServerContext) -> Option<(Uuid, Uuid)> {
        let GeoId::GeometryObject(object_id) = server_ctx.geo_hit? else {
            return None;
        };
        map.wall_area_surface_for_geometry_object(object_id)
    }

    fn update_surface_preview(&mut self, map: &mut Map, point: Vec3<f32>) {
        let mut resolved_map = map.clone();
        resolved_map.wall_surface_preview = None;
        let contacts = resolved_map.resolve_wall_endpoint_contacts(0.05);
        let Some((assembly_id, boundary)) = resolved_map.wall_surface_region_at(point) else {
            self.cancel_surface_preview(map);
            return;
        };
        if resolved_map
            .wall_assembly(assembly_id)
            .is_some_and(|assembly| {
                assembly
                    .area_surfaces
                    .iter()
                    .any(|surface| surface.boundary == boundary)
            })
        {
            self.cancel_surface_preview(map);
            return;
        }
        let unchanged = map.wall_surface_preview.as_ref().is_some_and(|preview| {
            preview.assembly_id == assembly_id
                && preview.surface.boundary == boundary
                && (preview.surface.elevation - self.surface_elevation).abs() <= 1e-6
                && (preview.surface.thickness - self.surface_thickness).abs() <= 1e-6
                && (preview.surface.clearance - self.surface_clearance).abs() <= 1e-6
        });
        if unchanged {
            return;
        }
        let source = resolved_map
            .wall_assembly(assembly_id)
            .map(WallAssembly::floor_pixel_source);
        let mut surface = WallAreaSurface::new(boundary);
        if let Some(preview) = map.wall_surface_preview.as_ref()
            && preview.assembly_id == assembly_id
            && preview.surface.boundary == surface.boundary
        {
            surface.id = preview.surface.id;
        }
        surface.elevation = self.surface_elevation;
        surface.thickness = self.surface_thickness;
        surface.clearance = self.surface_clearance;
        surface.source = source;
        map.wall_surface_preview = Some(WallAreaSurfacePreview {
            assembly_id,
            surface,
            wall_assemblies: (contacts > 0).then(|| resolved_map.wall_assemblies.clone()),
            block_prop_instances: (contacts > 0).then(|| resolved_map.block_prop_instances.clone()),
        });
        map.rebuild_wall_geometry_with_surface_preview();
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.set_dirty();
        rusterix.set_overlay_dirty();
    }

    fn place_span(
        &mut self,
        map: &mut Map,
        start: Vec3<f32>,
        end: Vec3<f32>,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut placed_style = self.build_style.clone();
        if let Some(assembly_id) = map.selected_wall_assembly
            && let Some(span_id) = map.selected_wall_spans.first().copied()
            && let Some(assembly) = map.wall_assembly(assembly_id)
            && let Some(span) = assembly.span(span_id)
        {
            let selected_style = span.style_override.as_ref().unwrap_or(&assembly.style);
            placed_style.stone_source = selected_style.stone_source.clone();
            placed_style.stone_variants = selected_style.stone_variants.clone();
            placed_style.mortar_source = selected_style.mortar_source.clone();
            placed_style.frame_source = selected_style.frame_source.clone();
            placed_style.stone_variation = selected_style.stone_variation;
        }
        let previous = map.clone();
        match map.connect_wall_points(start, end, Self::snap_distance(map)) {
            Ok((assembly_id, span_id, start_node, end_node)) => {
                if let Some(assembly) = map.wall_assembly_mut(assembly_id) {
                    if assembly.spans.len() == 1 {
                        assembly.auto_floor = self.build_auto_floor;
                    }
                    if let Some(span) = assembly.span_mut(span_id) {
                        span.style_override = Some(placed_style);
                    }
                }
                map.rebuild_wall_geometry();
                map.clear_selection();
                map.selected_wall_assembly = Some(assembly_id);
                map.selected_wall_spans.push(span_id);
                map.selected_wall_nodes.extend([start_node, end_node]);
                let continued_point = map
                    .wall_assembly(assembly_id)
                    .and_then(|assembly| assembly.node(end_node))
                    .map(|node| node.position)
                    .unwrap_or(end);
                self.anchor = Some(continued_point);
                self.hover = Some(continued_point);
                map.curr_grid_pos_3d = Some(continued_point);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Wall span added. Continue clicking, or press Escape to finish.".to_string(),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                Some(ProjectUndoAtom::MapEdit(
                    server_ctx.pc,
                    Box::new(previous),
                    Box::new(map.clone()),
                ))
            }
            Err(message) => {
                ctx.ui
                    .send(TheEvent::SetStatusText(TheId::empty(), message));
                None
            }
        }
    }

    fn draw_panel_button(
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: TheDim,
        label: &str,
        active: bool,
        enabled: bool,
    ) {
        let stride = buffer.stride();
        let fill = if !enabled {
            [38, 40, 44, 230]
        } else if active {
            [91, 70, 31, 255]
        } else {
            [54, 57, 63, 250]
        };
        let border = if active {
            [224, 184, 88, 255]
        } else {
            [92, 96, 106, 255]
        };
        let text = if enabled {
            [238, 239, 242, 255]
        } else {
            [105, 108, 116, 255]
        };
        ctx.draw
            .rect(buffer.pixels_mut(), &rect.to_buffer_utuple(), stride, &fill);
        buffer.draw_rect_outline(&rect, &border);
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &rect.to_buffer_utuple(),
            stride,
            label,
            TheFontSettings {
                size: 11.5,
                ..Default::default()
            },
            &text,
            TheHorizontalAlign::Center,
            TheVerticalAlign::Center,
        );
    }

    fn draw_wall_panel(
        &self,
        buffer: &mut TheRGBABuffer,
        map: &Map,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        let panel = Self::panel_rect();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &panel.to_buffer_utuple(),
            stride,
            &[27, 29, 33, 246],
        );
        buffer.draw_rect_outline(&panel, &[88, 92, 101, 255]);
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(
                (Self::PANEL_X + 12) as usize,
                (Self::PANEL_Y + 6) as usize,
                (Self::PANEL_WIDTH - 24) as usize,
                22,
            ),
            stride,
            "WALL",
            TheFontSettings {
                size: 14.0,
                ..Default::default()
            },
            &[241, 242, 245, 255],
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(
                (Self::PANEL_X + 64) as usize,
                (Self::PANEL_Y + 6) as usize,
                (Self::PANEL_WIDTH - 76) as usize,
                22,
            ),
            stride,
            if self.interaction_mode == WallInteractionMode::Surface {
                "HUD: SURFACE"
            } else {
                "HUD: STONE · VAR1 · VAR2 · MORTAR"
            },
            TheFontSettings {
                size: 10.5,
                ..Default::default()
            },
            &[143, 147, 156, 255],
            TheHorizontalAlign::Right,
            TheVerticalAlign::Center,
        );

        let opening_enabled = map.selected_wall_assembly.is_some()
            && server_ctx.editor_view_mode != EditorViewMode::D2;
        for (index, (mode, label)) in [
            (WallInteractionMode::Build, "BUILD"),
            (WallInteractionMode::Select, "SELECT"),
            (WallInteractionMode::Opening, "HOLE"),
            (WallInteractionMode::Brick, "BRICK"),
            (WallInteractionMode::Surface, "SURF"),
        ]
        .into_iter()
        .enumerate()
        {
            Self::draw_panel_button(
                buffer,
                ctx,
                Self::panel_mode_rect(index as i32),
                label,
                self.interaction_mode == mode,
                !matches!(
                    mode,
                    WallInteractionMode::Opening | WallInteractionMode::Brick
                ) || opening_enabled,
            );
        }

        let selection_rect = TheDim::rect(
            Self::PANEL_X + 10,
            Self::PANEL_Y + 68,
            Self::PANEL_WIDTH - 20,
            44,
        );
        ctx.draw.rect(
            buffer.pixels_mut(),
            &selection_rect.to_buffer_utuple(),
            stride,
            &[36, 39, 44, 250],
        );
        if self.interaction_mode == WallInteractionMode::Build {
            for (index, (mode, label)) in
                [(WallBuildMode::Line, "LINE"), (WallBuildMode::Ring, "RING")]
                    .into_iter()
                    .enumerate()
            {
                Self::draw_panel_button(
                    buffer,
                    ctx,
                    Self::panel_build_mode_rect(index as i32),
                    label,
                    self.build_mode == mode,
                    true,
                );
            }
        }
        let selected_opening = map.selected_wall_assembly.and_then(|assembly_id| {
            let span_id = *map.selected_wall_spans.first()?;
            let opening_id = map.selected_wall_opening?;
            let assembly = map.wall_assembly(assembly_id)?;
            let opening = assembly.opening(span_id, opening_id)?;
            let style = assembly
                .span(span_id)?
                .style_override
                .as_ref()
                .unwrap_or(&assembly.style);
            Some((opening, style))
        });
        let selected_surface = map.selected_wall_assembly.and_then(|assembly_id| {
            let surface_id = map.selected_wall_surface?;
            map.wall_assembly(assembly_id)?.area_surface(surface_id)
        });
        let selection = map
            .selected_wall_assembly
            .and_then(|assembly_id| {
                let assembly = map.wall_assembly(assembly_id)?;
                let span_id = *map.selected_wall_spans.first()?;
                let span = assembly.span(span_id)?;
                let style = if self.interaction_mode == WallInteractionMode::Build {
                    &self.build_style
                } else {
                    span.style_override.as_ref().unwrap_or(&assembly.style)
                };
                let selected_count = map.selected_wall_spans.len();
                Some((
                    if selected_count > 1 {
                        format!("{} spans", selected_count)
                    } else {
                        assembly.name.clone()
                    },
                    assembly.span_length(span_id)?,
                    span.curve_offset,
                    span.curve_segments,
                    style.height,
                    style.thickness,
                    style.masonry,
                    assembly.auto_floor,
                    style.brick_width,
                    style.brick_height,
                    style.mortar_gap,
                    style.bevel,
                    style.irregularity,
                    style.damage,
                    style.stone_variation,
                    selected_opening
                        .map(|(opening, style)| opening.frame.width(style))
                        .unwrap_or(style.frame_width),
                    selected_opening
                        .map(|(opening, style)| opening.frame.depth(style))
                        .unwrap_or(style.frame_depth),
                    selected_opening
                        .map(|(opening, style)| opening.frame.arch_stones(style))
                        .unwrap_or(style.arch_stones),
                    span.openings.len(),
                    span.removed_bricks.len(),
                ))
            })
            .or_else(|| {
                (self.interaction_mode == WallInteractionMode::Build).then(|| {
                    (
                        "New wall span".to_string(),
                        0.0,
                        0.0,
                        12_u16,
                        self.build_style.height,
                        self.build_style.thickness,
                        self.build_style.masonry,
                        self.build_auto_floor,
                        self.build_style.brick_width,
                        self.build_style.brick_height,
                        self.build_style.mortar_gap,
                        self.build_style.bevel,
                        self.build_style.irregularity,
                        self.build_style.damage,
                        self.build_style.stone_variation,
                        self.build_style.frame_width,
                        self.build_style.frame_depth,
                        self.build_style.arch_stones,
                        0,
                        0,
                    )
                })
            });
        let (selection_line, detail_line) = if let Some(surface) = selected_surface {
            (
                "Selected: Area surface".to_string(),
                format!(
                    "Elevation {:.2}  •  Thickness {:.2}  •  Clearance {:.3}",
                    surface.elevation, surface.thickness, surface.clearance
                ),
            )
        } else if let Some((opening, _)) = selected_opening {
            (
                format!("Selected: {:?} opening", opening.shape),
                format!(
                    "Width {:.2}  •  Height {:.2}  •  {}",
                    opening.width,
                    opening.height,
                    opening.frame.surround.label()
                ),
            )
        } else if let Some((
            name,
            length,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            openings,
            removed_bricks,
        )) = &selection
        {
            (
                format!("Selected: {name}"),
                format!("Length {length:.2}  •  Openings {openings}  •  Missing {removed_bricks}"),
            )
        } else {
            (
                "No wall span selected".to_string(),
                "Choose SELECT, then click the visible wall".to_string(),
            )
        };
        if self.interaction_mode != WallInteractionMode::Build {
            for (line, y, color, size) in [
                (
                    selection_line.as_str(),
                    Self::PANEL_Y + 72,
                    [233, 234, 238, 255],
                    12.5,
                ),
                (
                    detail_line.as_str(),
                    Self::PANEL_Y + 91,
                    [158, 162, 171, 255],
                    11.0,
                ),
            ] {
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &(
                        (Self::PANEL_X + 18) as usize,
                        y as usize,
                        (Self::PANEL_WIDTH - 36) as usize,
                        18,
                    ),
                    stride,
                    line,
                    TheFontSettings {
                        size,
                        ..Default::default()
                    },
                    &color,
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }
        }

        for (row, (field, label)) in self.visible_style_fields().iter().copied().enumerate() {
            let value = selection
                .as_ref()
                .map(
                    |(
                        _,
                        _,
                        curve,
                        curve_segments,
                        height,
                        thickness,
                        masonry,
                        auto_floor,
                        brick_width,
                        brick_height,
                        mortar_gap,
                        bevel,
                        irregularity,
                        damage,
                        stone_variation,
                        frame_width,
                        frame_depth,
                        arch_stones,
                        _,
                        _,
                    )| match field {
                        WallStyleField::Masonry => masonry.label().to_string(),
                        WallStyleField::AutoFloor => {
                            if *auto_floor { "On" } else { "Off" }.to_string()
                        }
                        WallStyleField::Height => format!("{height:.2}"),
                        WallStyleField::Thickness => format!("{thickness:.2}"),
                        WallStyleField::Curve => format!("{curve:+.2}"),
                        WallStyleField::CurveSegments => curve_segments.to_string(),
                        WallStyleField::BrickWidth => format!("{brick_width:.2}"),
                        WallStyleField::BrickHeight => format!("{brick_height:.2}"),
                        WallStyleField::MortarGap => format!("{mortar_gap:.3}"),
                        WallStyleField::Bevel => format!("{bevel:.3}"),
                        WallStyleField::Irregularity => format!("{irregularity:.2}"),
                        WallStyleField::Damage => format!("{damage:.2}"),
                        WallStyleField::StoneVariation => format!("{stone_variation:.2}"),
                        WallStyleField::FrameWidth => format!("{frame_width:.2}"),
                        WallStyleField::FrameDepth => format!("{frame_depth:.2}"),
                        WallStyleField::ArchStones => arch_stones.to_string(),
                        WallStyleField::SurfaceElevation => selected_surface
                            .map(|surface| format!("{:.2}", surface.elevation))
                            .unwrap_or_else(|| format!("{:.2}", self.surface_elevation)),
                        WallStyleField::SurfaceThickness => selected_surface
                            .map(|surface| format!("{:.2}", surface.thickness))
                            .unwrap_or_else(|| format!("{:.2}", self.surface_thickness)),
                        WallStyleField::SurfaceClearance => selected_surface
                            .map(|surface| format!("{:.3}", surface.clearance))
                            .unwrap_or_else(|| format!("{:.3}", self.surface_clearance)),
                    },
                )
                .unwrap_or_else(|| match field {
                    WallStyleField::SurfaceElevation => {
                        format!("{:.2}", self.surface_elevation)
                    }
                    WallStyleField::SurfaceThickness => {
                        format!("{:.2}", self.surface_thickness)
                    }
                    WallStyleField::SurfaceClearance => {
                        format!("{:.3}", self.surface_clearance)
                    }
                    _ => "—".to_string(),
                });
            let row_offset = if self.interaction_mode == WallInteractionMode::Opening {
                2
            } else {
                0
            };
            let y = Self::PANEL_Y
                + Self::PANEL_ROW_Y
                + (row as i32 + row_offset) * Self::PANEL_ROW_SPACING;
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &((Self::PANEL_X + 14) as usize, y as usize, 88, 20),
                stride,
                label,
                TheFontSettings {
                    size: 11.5,
                    ..Default::default()
                },
                &[182, 185, 192, 255],
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &((Self::PANEL_X + 98) as usize, y as usize, 76, 20),
                stride,
                &value,
                TheFontSettings {
                    size: 12.0,
                    ..Default::default()
                },
                &[239, 240, 243, 255],
                TheHorizontalAlign::Right,
                TheVerticalAlign::Center,
            );
            for plus in [false, true] {
                Self::draw_panel_button(
                    buffer,
                    ctx,
                    self.panel_adjust_rect(row as i32, plus),
                    if plus { "+" } else { "−" },
                    false,
                    selection.is_some(),
                );
            }
        }

        if self.interaction_mode == WallInteractionMode::Opening {
            let active_shape = selected_opening
                .map(|(opening, _)| opening.shape)
                .unwrap_or(self.opening_shape);
            let active_surround = selected_opening
                .map(|(opening, _)| opening.frame.surround)
                .unwrap_or(self.opening_surround);
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(
                    (Self::PANEL_X + 14) as usize,
                    (Self::PANEL_Y + Self::PANEL_ROW_Y) as usize,
                    98,
                    24,
                ),
                stride,
                "Opening shape",
                TheFontSettings {
                    size: 11.5,
                    ..Default::default()
                },
                &[182, 185, 192, 255],
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
            for (index, (shape, label)) in [
                (WallOpeningShape::Rectangular, "RECT"),
                (WallOpeningShape::Arch, "ARCH"),
            ]
            .into_iter()
            .enumerate()
            {
                Self::draw_panel_button(
                    buffer,
                    ctx,
                    Self::panel_shape_rect(index as i32),
                    label,
                    active_shape == shape,
                    opening_enabled,
                );
            }
            let surround_y = Self::PANEL_Y + Self::PANEL_ROW_Y + Self::PANEL_ROW_SPACING;
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &((Self::PANEL_X + 14) as usize, surround_y as usize, 62, 20),
                stride,
                "Surround",
                TheFontSettings {
                    size: 11.5,
                    ..Default::default()
                },
                &[182, 185, 192, 255],
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
            for (index, (surround, label)) in [
                (WallOpeningSurround::None, "NONE"),
                (WallOpeningSurround::Trim, "TRIM"),
                (WallOpeningSurround::Blocks, "BLOCKS"),
            ]
            .into_iter()
            .enumerate()
            {
                Self::draw_panel_button(
                    buffer,
                    ctx,
                    Self::panel_surround_rect(index as i32),
                    label,
                    active_surround == surround,
                    opening_enabled,
                );
            }
        }

        let guidance = match self.interaction_mode {
            WallInteractionMode::Build if self.build_mode == WallBuildMode::Ring => {
                "Drag center to radius  •  releases to Line"
            }
            WallInteractionMode::Build => "B  Build  •  Drag nodes  •  Esc finishes",
            WallInteractionMode::Select => "S  Click a wall span to edit it",
            WallInteractionMode::Opening if self.opening_anchor.is_some() => {
                "O  Move on wall, click opposite corner"
            }
            WallInteractionMode::Opening if map.selected_wall_opening.is_some() => {
                "Drag body / handles  •  Delete removes"
            }
            WallInteractionMode::Opening => "Click opening to edit, empty wall to create",
            WallInteractionMode::Brick => "R  Hover a brick; click to remove / restore",
            WallInteractionMode::Surface => "U  Click a bounded area  •  Delete removes",
        };
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(
                (Self::PANEL_X + 12) as usize,
                (Self::PANEL_Y + 396) as usize,
                (Self::PANEL_WIDTH - 24) as usize,
                17,
            ),
            stride,
            guidance,
            TheFontSettings {
                size: 10.5,
                ..Default::default()
            },
            &[143, 147, 156, 255],
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
    }
}

impl Tool for WallTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Wall Tool"),
            anchor: None,
            hover: None,
            hud: Hud::new(HudMode::Wall),
            opening_armed: false,
            opening_anchor: None,
            opening_shape: WallOpeningShape::Rectangular,
            opening_surround: WallOpeningSurround::Blocks,
            interaction_mode: WallInteractionMode::Build,
            build_mode: WallBuildMode::Line,
            build_style: WallStyle::default(),
            build_auto_floor: false,
            held_adjustment: None,
            node_drag: None,
            ring_drag: None,
            opening_drag: None,
            surface_elevation: 0.25,
            surface_thickness: 0.08,
            surface_clearance: 0.015,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Wall Tool — build, select, and edit connected walls from the Wall panel".to_string()
    }

    fn icon_name(&self) -> String {
        "line-segment".to_string()
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                self.held_adjustment = None;
                self.node_drag = None;
                self.ring_drag = None;
                self.opening_drag = None;
                self.build_mode = WallBuildMode::Line;
                server_ctx.curr_map_tool_type = MapToolType::Wall;
                server_ctx.hover_cursor = None;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    self.cancel_opening(map);
                    self.cancel_brick_preview(map);
                    self.cancel_surface_preview(map);
                    self.interaction_mode = WallInteractionMode::Build;
                    map.clear_selection();
                    map.curr_grid_pos_3d = None;
                }
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Wall Tool: use BUILD to place walls or SELECT to click and modify an existing span."
                        .to_string(),
                ));
                true
            }
            DeActivate => {
                self.held_adjustment = None;
                self.node_drag = None;
                self.opening_drag = None;
                self.build_mode = WallBuildMode::Line;
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    self.cancel_ring_drag(map);
                    self.finish_run(map);
                    self.cancel_opening(map);
                    self.cancel_brick_preview(map);
                    self.cancel_surface_preview(map);
                    map.selected_wall_opening = None;
                    map.selected_wall_surface = None;
                    map.hovered_wall_span = None;
                }
                true
            }
            _ => false,
        }
    }

    fn update(
        &mut self,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> bool {
        self.repeat_held_adjustment(map, ctx, server_ctx)
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
            MapKey(key) if matches!(key, '1'..='6') => {
                map.subdivisions = match key {
                    '1' => 1.0,
                    '2' => 2.0,
                    '3' => 4.0,
                    '4' => 8.0,
                    '5' => 16.0,
                    '6' => 32.0,
                    _ => map.subdivisions,
                };
                {
                    let mut rusterix = RUSTERIX.write().unwrap();
                    rusterix.set_dirty();
                    rusterix.set_overlay_dirty();
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Tool Changed"),
                    TheValue::Empty,
                ));
                ctx.ui.redraw_all = true;
                None
            }
            MapKey(key) if matches!(key, 'o' | 'O') => {
                self.set_interaction_mode(WallInteractionMode::Opening, map, ctx, server_ctx);
                None
            }
            MapKey(key) if matches!(key, 'b' | 'B') => {
                self.set_interaction_mode(WallInteractionMode::Build, map, ctx, server_ctx);
                None
            }
            MapKey(key) if matches!(key, 's' | 'S') => {
                self.set_interaction_mode(WallInteractionMode::Select, map, ctx, server_ctx);
                None
            }
            MapKey(key) if matches!(key, 'r' | 'R') => {
                self.set_interaction_mode(WallInteractionMode::Brick, map, ctx, server_ctx);
                None
            }
            MapKey(key) if matches!(key, 'u' | 'U') => {
                self.set_interaction_mode(WallInteractionMode::Surface, map, ctx, server_ctx);
                None
            }
            MapHover(coord) => {
                self.hud.hovered(coord.x, coord.y, map, ui, ctx, server_ctx);
                if Self::panel_rect().contains(coord) {
                    ctx.ui.redraw_all = true;
                    return None;
                }
                self.hover = self.pointer_position(ui, map, coord, server_ctx);
                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    server_ctx.hover_cursor = self.hover.map(|point| Vec2::new(point.x, point.z));
                } else {
                    server_ctx.hover_cursor_3d = self.hover;
                }
                map.curr_grid_pos_3d = self.anchor;
                let previous_hovered_span = map.hovered_wall_span;
                map.hovered_wall_span = if self.interaction_mode == WallInteractionMode::Select {
                    self.hover
                        .and_then(|point| Self::wall_span_at_pointer(map, server_ctx, point))
                } else {
                    None
                };
                if map.hovered_wall_span != previous_hovered_span {
                    RUSTERIX.write().unwrap().set_overlay_dirty();
                }
                if self.interaction_mode == WallInteractionMode::Surface {
                    let hit_existing = Self::surface_hit(map, server_ctx).is_some_and(
                        |(assembly_id, surface_id)| {
                            map.wall_assembly(assembly_id)
                                .and_then(|assembly| assembly.area_surface(surface_id))
                                .is_some()
                        },
                    );
                    let hit_preview = Self::surface_hit(map, server_ctx).is_some_and(
                        |(assembly_id, surface_id)| {
                            map.wall_surface_preview.as_ref().is_some_and(|preview| {
                                preview.assembly_id == assembly_id
                                    && preview.surface.id == surface_id
                            })
                        },
                    );
                    if hit_existing {
                        self.cancel_surface_preview(map);
                    } else if !hit_preview {
                        if let Some(point) = self.hover {
                            self.update_surface_preview(map, point);
                        } else {
                            self.cancel_surface_preview(map);
                        }
                    }
                }
                if self.interaction_mode == WallInteractionMode::Brick {
                    if let Some((assembly_id, span_id, key, _)) =
                        Self::brick_pointer(map, server_ctx)
                    {
                        let is_removed = map
                            .wall_assembly(assembly_id)
                            .and_then(|assembly| assembly.span(span_id))
                            .is_some_and(|span| span.removed_bricks.contains(&key));
                        let preview = WallBrickPreview {
                            assembly_id,
                            span_id,
                            key,
                            remove: !is_removed,
                        };
                        if map.wall_brick_preview != Some(preview) {
                            map.wall_brick_preview = Some(preview);
                            map.rebuild_wall_geometry_with_brick_preview();
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.set_dirty();
                            rusterix.set_overlay_dirty();
                        }
                    } else {
                        self.cancel_brick_preview(map);
                    }
                }
                if self.opening_armed
                    && let Some((assembly_id, span_id, coordinates)) =
                        self.opening_pointer_coordinates(map, server_ctx)
                {
                    let start = self
                        .opening_anchor
                        .map(|(_, _, start)| start)
                        .unwrap_or(coordinates);
                    map.wall_opening_preview = Some(WallOpeningPreview {
                        assembly_id,
                        span_id,
                        start,
                        end: coordinates,
                        shape: self.opening_shape,
                        surround: self.opening_surround,
                    });
                    if self.opening_anchor.is_some() {
                        map.rebuild_wall_geometry_with_opening_preview();
                        let mut rusterix = RUSTERIX.write().unwrap();
                        rusterix.set_dirty();
                        rusterix.set_overlay_dirty();
                    } else {
                        RUSTERIX.write().unwrap().set_overlay_dirty();
                    }
                }
                ctx.ui.redraw_all = true;
                None
            }
            MapClicked(coord) => {
                if Self::panel_rect().contains(coord) {
                    for (index, mode) in [
                        WallInteractionMode::Build,
                        WallInteractionMode::Select,
                        WallInteractionMode::Opening,
                        WallInteractionMode::Brick,
                        WallInteractionMode::Surface,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if Self::panel_mode_rect(index as i32).contains(coord) {
                            self.set_interaction_mode(mode, map, ctx, server_ctx);
                            return None;
                        }
                    }
                    if self.interaction_mode == WallInteractionMode::Build {
                        for (index, build_mode) in [WallBuildMode::Line, WallBuildMode::Ring]
                            .into_iter()
                            .enumerate()
                        {
                            if Self::panel_build_mode_rect(index as i32).contains(coord) {
                                self.cancel_ring_drag(map);
                                self.finish_run(map);
                                self.build_mode = build_mode;
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    match build_mode {
                                        WallBuildMode::Line => {
                                            "Line mode: click points to add connected walls."
                                        }
                                        WallBuildMode::Ring => {
                                            "Ring mode: click its center, then drag to set the radius."
                                        }
                                    }
                                    .to_string(),
                                ));
                                ctx.ui.redraw_all = true;
                                return None;
                            }
                        }
                    }
                    if map.selected_wall_assembly.is_some()
                        || self.interaction_mode == WallInteractionMode::Build
                        || self.interaction_mode == WallInteractionMode::Surface
                    {
                        for (row, (field, _)) in
                            self.visible_style_fields().iter().copied().enumerate()
                        {
                            for plus in [false, true] {
                                if self.panel_adjust_rect(row as i32, plus).contains(coord) {
                                    if self.opening_armed {
                                        self.cancel_opening(map);
                                    }
                                    self.cancel_brick_preview(map);
                                    let direction = if plus { 1.0 } else { -1.0 };
                                    let amount = Self::adjustment_amount(map, field);
                                    let delta = amount * direction;
                                    let previous =
                                        if self.interaction_mode == WallInteractionMode::Surface {
                                            map.selected_wall_surface.is_some().then(|| map.clone())
                                        } else {
                                            (self.interaction_mode != WallInteractionMode::Build)
                                                .then(|| map.clone())
                                        };
                                    let undo =
                                        self.adjust_wall_field(map, server_ctx, field, delta);
                                    ctx.ui.redraw_all = true;
                                    if matches!(
                                        field,
                                        WallStyleField::Masonry | WallStyleField::AutoFloor
                                    ) {
                                        return undo;
                                    }
                                    let now = Instant::now();
                                    self.held_adjustment = Some(HeldWallAdjustment {
                                        field,
                                        delta,
                                        pressed_at: now,
                                        last_repeat: now,
                                        previous,
                                    });
                                    return None;
                                }
                            }
                        }
                        if self.interaction_mode == WallInteractionMode::Opening {
                            for (index, shape) in
                                [WallOpeningShape::Rectangular, WallOpeningShape::Arch]
                                    .into_iter()
                                    .enumerate()
                            {
                                if Self::panel_shape_rect(index as i32).contains(coord) {
                                    if let (Some(assembly_id), Some(span_id), Some(opening_id)) = (
                                        map.selected_wall_assembly,
                                        map.selected_wall_spans.first().copied(),
                                        map.selected_wall_opening,
                                    ) {
                                        let previous = map.clone();
                                        let changed = map
                                            .wall_assembly_mut(assembly_id)
                                            .and_then(|assembly| {
                                                assembly.opening_mut(span_id, opening_id)
                                            })
                                            .is_some_and(|opening| {
                                                if opening.shape == shape {
                                                    false
                                                } else {
                                                    opening.shape = shape;
                                                    true
                                                }
                                            });
                                        if changed {
                                            map.rebuild_wall_geometry();
                                            let mut rusterix = RUSTERIX.write().unwrap();
                                            rusterix.set_dirty();
                                            rusterix.set_overlay_dirty();
                                            ctx.ui.redraw_all = true;
                                            return Some(ProjectUndoAtom::MapEdit(
                                                server_ctx.pc,
                                                Box::new(previous),
                                                Box::new(map.clone()),
                                            ));
                                        }
                                        return None;
                                    }
                                    self.opening_shape = shape;
                                    let mut rebuild_preview = false;
                                    if let Some(preview) = map.wall_opening_preview.as_mut() {
                                        preview.shape = shape;
                                        rebuild_preview = self.opening_anchor.is_some();
                                    }
                                    if rebuild_preview {
                                        map.rebuild_wall_geometry_with_opening_preview();
                                        let mut rusterix = RUSTERIX.write().unwrap();
                                        rusterix.set_dirty();
                                        rusterix.set_overlay_dirty();
                                    }
                                    ctx.ui.redraw_all = true;
                                    return None;
                                }
                            }
                            for (index, surround) in [
                                WallOpeningSurround::None,
                                WallOpeningSurround::Trim,
                                WallOpeningSurround::Blocks,
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                if Self::panel_surround_rect(index as i32).contains(coord) {
                                    if let (Some(assembly_id), Some(span_id), Some(opening_id)) = (
                                        map.selected_wall_assembly,
                                        map.selected_wall_spans.first().copied(),
                                        map.selected_wall_opening,
                                    ) {
                                        let previous = map.clone();
                                        let changed = map
                                            .wall_assembly_mut(assembly_id)
                                            .and_then(|assembly| {
                                                assembly.opening_mut(span_id, opening_id)
                                            })
                                            .is_some_and(|opening| {
                                                if opening.frame.surround == surround {
                                                    false
                                                } else {
                                                    opening.frame.surround = surround;
                                                    true
                                                }
                                            });
                                        if changed {
                                            map.rebuild_wall_geometry();
                                            let mut rusterix = RUSTERIX.write().unwrap();
                                            rusterix.set_dirty();
                                            rusterix.set_overlay_dirty();
                                            ctx.ui.redraw_all = true;
                                            return Some(ProjectUndoAtom::MapEdit(
                                                server_ctx.pc,
                                                Box::new(previous),
                                                Box::new(map.clone()),
                                            ));
                                        }
                                        return None;
                                    }
                                    self.opening_surround = surround;
                                    let mut rebuild_preview = false;
                                    if let Some(preview) = map.wall_opening_preview.as_mut() {
                                        preview.surround = surround;
                                        rebuild_preview = self.opening_anchor.is_some();
                                    }
                                    if rebuild_preview {
                                        map.rebuild_wall_geometry_with_opening_preview();
                                        let mut rusterix = RUSTERIX.write().unwrap();
                                        rusterix.set_dirty();
                                        rusterix.set_overlay_dirty();
                                    }
                                    ctx.ui.redraw_all = true;
                                    return None;
                                }
                            }
                        }
                    }
                    return None;
                }
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    return None;
                }
                if self.interaction_mode == WallInteractionMode::Surface {
                    if let Some((assembly_id, surface_id)) = Self::surface_hit(map, server_ctx)
                        && let Some(surface) = map
                            .wall_assembly(assembly_id)
                            .and_then(|assembly| assembly.area_surface(surface_id))
                            .cloned()
                    {
                        self.cancel_surface_preview(map);
                        if Self::select_area_surface(map, assembly_id, surface_id) {
                            self.surface_elevation = surface.elevation;
                            self.surface_thickness = surface.thickness;
                            self.surface_clearance = surface.clearance;
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Area surface selected. Adjust its fit or apply a Surface material."
                                    .to_string(),
                            ));
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Map Selection Changed"),
                                TheValue::Empty,
                            ));
                            RUSTERIX.write().unwrap().set_overlay_dirty();
                            ctx.ui.redraw_all = true;
                        }
                        return None;
                    }

                    let preview = if let Some(preview) = map.wall_surface_preview.take() {
                        Some(preview)
                    } else {
                        let point = self.pointer_position(ui, map, coord, server_ctx)?;
                        let mut resolved_map = map.clone();
                        resolved_map.wall_surface_preview = None;
                        let contacts = resolved_map.resolve_wall_endpoint_contacts(0.05);
                        resolved_map
                            .wall_surface_region_at(point)
                            .map(|(assembly_id, boundary)| {
                                let mut surface = WallAreaSurface::new(boundary);
                                surface.elevation = self.surface_elevation;
                                surface.thickness = self.surface_thickness;
                                surface.clearance = self.surface_clearance;
                                surface.source = resolved_map
                                    .wall_assembly(assembly_id)
                                    .map(WallAssembly::floor_pixel_source);
                                WallAreaSurfacePreview {
                                    assembly_id,
                                    surface,
                                    wall_assemblies: (contacts > 0)
                                        .then(|| resolved_map.wall_assemblies.clone()),
                                    block_prop_instances: (contacts > 0)
                                        .then(|| resolved_map.block_prop_instances.clone()),
                                }
                            })
                    };
                    let Some(preview) = preview else {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "No bounded wall area here. The surrounding wall nodes must form a closed region."
                                .to_string(),
                        ));
                        return None;
                    };
                    let mut previous = map.clone();
                    previous.wall_surface_preview = None;
                    previous.rebuild_wall_geometry();
                    let assembly_id = preview.assembly_id;
                    let surface_id = preview.surface.id;
                    if let Some(wall_assemblies) = preview.wall_assemblies {
                        map.wall_assemblies = wall_assemblies;
                    }
                    if let Some(block_prop_instances) = preview.block_prop_instances {
                        map.block_prop_instances = block_prop_instances;
                    }
                    let Some(assembly) = map.wall_assembly_mut(assembly_id) else {
                        return None;
                    };
                    if let Some(existing) = assembly
                        .area_surfaces
                        .iter()
                        .find(|surface| surface.boundary == preview.surface.boundary)
                        .map(|surface| surface.id)
                    {
                        map.rebuild_wall_geometry();
                        Self::select_area_surface(map, assembly_id, existing);
                        return None;
                    }
                    assembly.area_surfaces.push(preview.surface);
                    map.rebuild_wall_geometry();
                    Self::select_area_surface(map, assembly_id, surface_id);
                    let mut rusterix = RUSTERIX.write().unwrap();
                    rusterix.set_dirty();
                    rusterix.set_overlay_dirty();
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Fitted area surface created. Use the Surface HUD slot for its coal tile or color."
                            .to_string(),
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                    ctx.ui.redraw_all = true;
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(previous),
                        Box::new(map.clone()),
                    ));
                }
                if self.interaction_mode == WallInteractionMode::Brick {
                    let Some((assembly_id, span_id, key, _)) = Self::brick_pointer(map, server_ctx)
                    else {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Hover a brick on the selected wall before clicking.".to_string(),
                        ));
                        return None;
                    };
                    let mut previous = map.clone();
                    previous.wall_brick_preview = None;
                    previous.rebuild_wall_geometry();
                    let is_removed = map
                        .wall_assembly(assembly_id)
                        .and_then(|assembly| assembly.span(span_id))
                        .is_some_and(|span| span.removed_bricks.contains(&key));
                    map.wall_brick_preview = None;
                    if let Some(assembly) = map.wall_assembly_mut(assembly_id) {
                        let _ = assembly.set_brick_removed(span_id, key, !is_removed);
                    }
                    map.rebuild_wall_geometry();
                    let mut rusterix = RUSTERIX.write().unwrap();
                    rusterix.set_dirty();
                    rusterix.set_overlay_dirty();
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        if is_removed {
                            "Brick restored."
                        } else {
                            "Brick removed."
                        }
                        .to_string(),
                    ));
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(previous),
                        Box::new(map.clone()),
                    ));
                }
                if self.interaction_mode == WallInteractionMode::Opening && !self.opening_armed {
                    let Some((assembly_id, span_id, coordinates)) =
                        self.opening_pointer_coordinates(map, server_ctx)
                    else {
                        return None;
                    };
                    let handle = Self::selected_opening_handle_at(map, coordinates);
                    let opening_id = handle.and(map.selected_wall_opening).or_else(|| {
                        map.wall_assembly(assembly_id)?
                            .opening_at(span_id, coordinates)
                    });
                    if let Some(opening_id) = opening_id {
                        let Some(original) = map
                            .wall_assembly(assembly_id)
                            .and_then(|assembly| assembly.opening(span_id, opening_id))
                            .cloned()
                        else {
                            return None;
                        };
                        map.selected_wall_opening = Some(opening_id);
                        self.opening_shape = original.shape;
                        self.opening_surround = original.frame.surround;
                        self.opening_drag = Some(WallOpeningDrag {
                            assembly_id,
                            span_id,
                            opening_id,
                            handle: handle.unwrap_or(WallOpeningHandle::Move),
                            start_coordinates: coordinates,
                            original,
                            previous: map.clone(),
                            changed: false,
                        });
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Opening selected. Drag its body to move it or a handle to resize it."
                                .to_string(),
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                        RUSTERIX.write().unwrap().set_overlay_dirty();
                        ctx.ui.redraw_all = true;
                        return None;
                    }
                    map.selected_wall_opening = None;
                    self.opening_armed = true;
                    self.opening_anchor = Some((assembly_id, span_id, coordinates));
                    map.wall_opening_preview = Some(WallOpeningPreview {
                        assembly_id,
                        span_id,
                        start: coordinates,
                        end: coordinates,
                        shape: self.opening_shape,
                        surround: self.opening_surround,
                    });
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Opening corner placed. Click the opposite corner on the wall.".to_string(),
                    ));
                    ctx.ui.redraw_all = true;
                    return None;
                }
                if self.opening_armed {
                    let Some((assembly_id, span_id, coordinates)) =
                        self.opening_pointer_coordinates(map, server_ctx)
                    else {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Click directly on the selected wall to place the opening.".to_string(),
                        ));
                        return None;
                    };
                    if self.opening_anchor.is_none() {
                        self.opening_anchor = Some((assembly_id, span_id, coordinates));
                        map.wall_opening_preview = Some(WallOpeningPreview {
                            assembly_id,
                            span_id,
                            start: coordinates,
                            end: coordinates,
                            shape: self.opening_shape,
                            surround: self.opening_surround,
                        });
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Opening corner placed. Click the opposite corner on the wall."
                                .to_string(),
                        ));
                        return None;
                    }
                    let (_, _, first) = self.opening_anchor.unwrap();
                    let mut previous = map.clone();
                    previous.wall_opening_preview = None;
                    previous.rebuild_wall_geometry();
                    let result = map
                        .wall_assembly_mut(assembly_id)
                        .ok_or_else(|| "The selected wall assembly no longer exists.".to_string())
                        .and_then(|assembly| {
                            assembly.add_opening(span_id, first, coordinates, self.opening_shape)
                        });
                    match result {
                        Ok(opening_id) => {
                            if let Some(assembly) = map.wall_assembly_mut(assembly_id)
                                && let Some(opening) = assembly.opening_mut(span_id, opening_id)
                            {
                                opening.frame.surround = self.opening_surround;
                            }
                            map.selected_wall_opening = Some(opening_id);
                            self.cancel_opening(map);
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Wall opening created in the live wall mesh.".to_string(),
                            ));
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Geometry Overlay 3D"),
                                TheValue::Empty,
                            ));
                            return Some(ProjectUndoAtom::MapEdit(
                                server_ctx.pc,
                                Box::new(previous),
                                Box::new(map.clone()),
                            ));
                        }
                        Err(message) => {
                            ctx.ui
                                .send(TheEvent::SetStatusText(TheId::empty(), message));
                            return None;
                        }
                    }
                }
                if self.interaction_mode == WallInteractionMode::Build
                    && self.build_mode == WallBuildMode::Ring
                {
                    let center = self.raw_pointer_position(ui, map, coord, server_ctx, None)?;
                    let previous = map.clone();
                    map.clear_selection();
                    self.anchor = Some(center);
                    self.hover = Some(center);
                    map.curr_grid_pos_3d = Some(center);
                    self.ring_drag = Some(WallRingDrag {
                        center,
                        assembly_id: None,
                        previous,
                    });
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Drag outward to set the wall ring radius.".to_string(),
                    ));
                    ctx.ui.redraw_all = true;
                    return None;
                }
                let point = self.pointer_position(ui, map, coord, server_ctx)?;
                if self.interaction_mode == WallInteractionMode::Build
                    && let Some((assembly_id, node_id, preferred_span)) =
                        Self::wall_node_at_pointer(map, server_ctx, point)
                    && let Some(position) =
                        Self::select_node(map, assembly_id, node_id, preferred_span)
                {
                    let connect_from = self.anchor.filter(|anchor| {
                        Vec3::new(anchor.x - position.x, 0.0, anchor.z - position.z)
                            .magnitude_squared()
                            > Self::snap_distance(map).powi(2)
                    });
                    self.anchor = Some(position);
                    self.hover = Some(position);
                    map.curr_grid_pos_3d = Some(position);
                    self.node_drag = Some(WallNodeDrag {
                        assembly_id,
                        node_id,
                        pressed_at: coord,
                        start_position: position,
                        connect_from,
                        previous: map.clone(),
                        changed: false,
                    });
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        if connect_from.is_some() {
                            "Release to connect the active wall to this node, or drag to move the node instead."
                                .to_string()
                        } else {
                            "Wall node selected. Drag to reshape every connected wall, or click another point to continue building."
                                .to_string()
                        },
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                    RUSTERIX.write().unwrap().set_overlay_dirty();
                    ctx.ui.redraw_all = true;
                    return None;
                }
                if self.interaction_mode == WallInteractionMode::Select {
                    if let Some((assembly_id, span_id)) =
                        Self::wall_span_at_pointer(map, server_ctx, point)
                    {
                        Self::select_span(map, assembly_id, span_id, ui.shift);
                        let count = map.selected_wall_spans.len();
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            if count == 0 {
                                "Wall selection cleared.".to_string()
                            } else if count > 1 {
                                format!(
                                    "{count} wall spans selected. Settings and HUD materials apply to the whole selection."
                                )
                            } else {
                                "Wall span selected. Shift-click adds spans; Wall settings and HUD materials apply to the selection."
                                    .to_string()
                            },
                        ));
                    } else {
                        map.clear_selection();
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "No wall span at that position. Click directly on the visible wall."
                                .to_string(),
                        ));
                    }
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                    RUSTERIX.write().unwrap().set_overlay_dirty();
                    ctx.ui.redraw_all = true;
                    return None;
                }
                if self.anchor.is_none()
                    && let Some((assembly_id, span_id)) =
                        Self::wall_span_at_pointer(map, server_ctx, point)
                {
                    Self::select_span(map, assembly_id, span_id, false);
                    if let Some(endpoint) =
                        Self::closest_span_endpoint(map, assembly_id, span_id, point)
                    {
                        self.anchor = Some(endpoint);
                        self.hover = Some(endpoint);
                        map.curr_grid_pos_3d = Some(endpoint);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Build mode: continuing from the nearest wall endpoint. Click to add a span."
                                .to_string(),
                        ));
                    }
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                    RUSTERIX.write().unwrap().set_overlay_dirty();
                    ctx.ui.redraw_all = true;
                    return None;
                }
                let Some(start) = self.anchor else {
                    map.clear_selection();
                    self.anchor = Some(point);
                    self.hover = Some(point);
                    map.curr_grid_pos_3d = Some(point);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Wall start placed. Click to add connected spans; Escape finishes."
                            .to_string(),
                    ));
                    ctx.ui.redraw_all = true;
                    return None;
                };

                self.place_span(map, start, point, ctx, server_ctx)
            }
            MapDragged(coord) => {
                if let Some(center) = self.ring_drag.as_ref().map(|drag| drag.center) {
                    let Some(point) =
                        self.raw_pointer_position(ui, map, coord, server_ctx, Some(center.y))
                    else {
                        return None;
                    };
                    let delta = Vec3::new(point.x - center.x, 0.0, point.z - center.z);
                    let step = ServerContext::edit_grid_step(map.subdivisions).max(0.05);
                    let radius = ((delta.magnitude() / step).round() * step).max(step);
                    let style = self.build_style.clone();
                    let auto_floor = self.build_auto_floor;
                    let changed = self.ring_drag.as_mut().is_some_and(|drag| {
                        Self::update_ring_preview(map, drag, &style, auto_floor, radius)
                    });
                    if changed {
                        self.hover = Some(point);
                        let mut rusterix = RUSTERIX.write().unwrap();
                        rusterix.set_dirty();
                        rusterix.set_overlay_dirty();
                        ctx.ui.redraw_all = true;
                    }
                    return None;
                }
                if let Some(drag) = self.opening_drag.as_ref() {
                    let Some((assembly_id, span_id, coordinates)) =
                        Self::selected_wall_plane_coordinates(map, server_ctx)
                    else {
                        return None;
                    };
                    if assembly_id == drag.assembly_id
                        && span_id == drag.span_id
                        && Self::update_opening_drag(map, drag, coordinates)
                    {
                        if let Some(drag) = self.opening_drag.as_mut() {
                            drag.changed = true;
                        }
                        map.rebuild_wall_geometry();
                        let mut rusterix = RUSTERIX.write().unwrap();
                        rusterix.set_dirty();
                        rusterix.set_overlay_dirty();
                        ctx.ui.redraw_all = true;
                    }
                    return None;
                }
                if let Some(drag) = self.node_drag.as_ref() {
                    let drag_delta = drag.pressed_at - coord;
                    if drag_delta.x * drag_delta.x + drag_delta.y * drag_delta.y < 9 {
                        return None;
                    }
                    let assembly_id = drag.assembly_id;
                    let node_id = drag.node_id;
                    let start_y = drag.start_position.y;
                    let Some(position) =
                        self.raw_pointer_position(ui, map, coord, server_ctx, Some(start_y))
                    else {
                        return None;
                    };
                    let moved = map
                        .wall_assembly(assembly_id)
                        .and_then(|assembly| assembly.node(node_id))
                        .is_some_and(|node| (node.position - position).magnitude_squared() > 1e-8);
                    if moved
                        && map.wall_assembly_mut(assembly_id).is_some_and(|assembly| {
                            assembly.set_node_position(node_id, position).is_ok()
                        })
                    {
                        if let Some(drag) = self.node_drag.as_mut() {
                            drag.changed = true;
                        }
                        self.anchor = Some(position);
                        self.hover = Some(position);
                        map.curr_grid_pos_3d = Some(position);
                        map.rebuild_wall_geometry();
                        let mut rusterix = RUSTERIX.write().unwrap();
                        rusterix.set_dirty();
                        rusterix.set_overlay_dirty();
                        ctx.ui.redraw_all = true;
                    }
                    return None;
                }
                let _ = self.repeat_held_adjustment(map, ctx, server_ctx);
                self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx);
                None
            }
            MapUp(_) => {
                if let Some(drag) = self.ring_drag.take() {
                    self.build_mode = WallBuildMode::Line;
                    self.anchor = None;
                    self.hover = None;
                    map.curr_grid_pos_3d = None;
                    if drag.assembly_id.is_some() {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Wall ring created; Build returned to Line mode.".to_string(),
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                        return Some(ProjectUndoAtom::MapEdit(
                            server_ctx.pc,
                            Box::new(drag.previous),
                            Box::new(map.clone()),
                        ));
                    }
                    return None;
                }
                if let Some(drag) = self.opening_drag.take() {
                    if drag.changed {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                        return Some(ProjectUndoAtom::MapEdit(
                            server_ctx.pc,
                            Box::new(drag.previous),
                            Box::new(map.clone()),
                        ));
                    }
                    return None;
                }
                if let Some(drag) = self.node_drag.take() {
                    if drag.changed {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                        return Some(ProjectUndoAtom::MapEdit(
                            server_ctx.pc,
                            Box::new(drag.previous),
                            Box::new(map.clone()),
                        ));
                    }
                    if let Some(start) = drag.connect_from {
                        return self.place_span(map, start, drag.start_position, ctx, server_ctx);
                    }
                    return None;
                }
                let held = self.held_adjustment.take()?;
                held.previous.map(|previous| {
                    ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(previous),
                        Box::new(map.clone()),
                    )
                })
            }
            MapEscape => {
                if self.ring_drag.is_some() {
                    self.cancel_ring_drag(map);
                    self.build_mode = WallBuildMode::Line;
                    self.finish_run(map);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Wall ring cancelled; Build returned to Line mode.".to_string(),
                    ));
                    ctx.ui.redraw_all = true;
                    return None;
                }
                if self.opening_armed {
                    self.cancel_opening(map);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Opening creation cancelled; wall selection preserved.".to_string(),
                    ));
                    ctx.ui.redraw_all = true;
                    return None;
                }
                if self.interaction_mode == WallInteractionMode::Opening {
                    map.selected_wall_opening = None;
                    self.opening_drag = None;
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Opening selection cleared. Click an opening or empty wall.".to_string(),
                    ));
                    RUSTERIX.write().unwrap().set_overlay_dirty();
                    ctx.ui.redraw_all = true;
                    return None;
                }
                if self.interaction_mode == WallInteractionMode::Brick {
                    self.cancel_brick_preview(map);
                    self.interaction_mode = WallInteractionMode::Select;
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Brick editing finished; wall selection preserved.".to_string(),
                    ));
                    ctx.ui.redraw_all = true;
                    return None;
                }
                if self.interaction_mode == WallInteractionMode::Surface {
                    self.cancel_surface_preview(map);
                    map.selected_wall_surface = None;
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Surface selection cleared. Click another bounded wall area.".to_string(),
                    ));
                    ctx.ui.redraw_all = true;
                    return None;
                }
                self.finish_run(map);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Wall run finished. Click to start another wall.".to_string(),
                ));
                ctx.ui.redraw_all = true;
                None
            }
            MapDelete
                if self.interaction_mode == WallInteractionMode::Opening
                    && map.selected_wall_opening.is_some() =>
            {
                let assembly_id = map.selected_wall_assembly?;
                let span_id = *map.selected_wall_spans.first()?;
                let opening_id = map.selected_wall_opening?;
                let previous = map.clone();
                if !map
                    .wall_assembly_mut(assembly_id)?
                    .remove_opening(span_id, opening_id)
                {
                    return None;
                }
                map.selected_wall_opening = None;
                map.rebuild_wall_geometry();
                let mut rusterix = RUSTERIX.write().unwrap();
                rusterix.set_dirty();
                rusterix.set_overlay_dirty();
                ctx.ui.redraw_all = true;
                Some(ProjectUndoAtom::MapEdit(
                    server_ctx.pc,
                    Box::new(previous),
                    Box::new(map.clone()),
                ))
            }
            MapDelete
                if self.interaction_mode == WallInteractionMode::Surface
                    && map.selected_wall_surface.is_some() =>
            {
                let assembly_id = map.selected_wall_assembly?;
                let surface_id = map.selected_wall_surface?;
                let previous = map.clone();
                if !map
                    .wall_assembly_mut(assembly_id)?
                    .remove_area_surface(surface_id)
                {
                    return None;
                }
                map.selected_wall_surface = None;
                map.rebuild_wall_geometry();
                let mut rusterix = RUSTERIX.write().unwrap();
                rusterix.set_dirty();
                rusterix.set_overlay_dirty();
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Area surface removed.".to_string(),
                ));
                ctx.ui.redraw_all = true;
                Some(ProjectUndoAtom::MapEdit(
                    server_ctx.pc,
                    Box::new(previous),
                    Box::new(map.clone()),
                ))
            }
            _ => None,
        }
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        assets: &Assets,
    ) {
        if server_ctx.editor_view_mode == EditorViewMode::D2
            && self.interaction_mode == WallInteractionMode::Build
        {
            for assembly in &map.wall_assemblies {
                for node in &assembly.nodes {
                    let center = Self::map_to_screen(map, *buffer.dim(), node.position);
                    let selected = map.selected_wall_assembly == Some(assembly.id)
                        && map.selected_wall_nodes.contains(&node.id);
                    let dim = TheDim::rect(center.x - 5, center.y - 5, 10, 10);
                    buffer.draw_disc(
                        &dim,
                        if selected {
                            &[255, 204, 92, 255]
                        } else {
                            &[88, 196, 221, 255]
                        },
                        1.0,
                        &[20, 24, 29, 255],
                    );
                }
            }
        }
        if server_ctx.editor_view_mode == EditorViewMode::D2
            && let (Some(start), Some(end)) = (self.anchor, self.hover)
        {
            let a = Self::map_to_screen(map, *buffer.dim(), start);
            let b = Self::map_to_screen(map, *buffer.dim(), end);
            buffer.draw_line(a.x, a.y, b.x, b.y, [88, 196, 221, 255]);
            buffer.draw_line(a.x + 1, a.y, b.x + 1, b.y, [14, 73, 92, 220]);
        }
        if server_ctx.editor_view_mode == EditorViewMode::D2
            && let Some(assembly_id) = map.selected_wall_assembly
            && let Some(assembly) = map.wall_assembly(assembly_id)
        {
            for span_id in &map.selected_wall_spans {
                let Some(span) = assembly.span(*span_id) else {
                    continue;
                };
                let Some(length) = assembly.span_length(*span_id) else {
                    continue;
                };
                let segments = if span.curve_offset.abs() <= 1e-5 {
                    1
                } else {
                    span.curve_segments.clamp(2, 64) as usize
                };
                let points = (0..=segments)
                    .filter_map(|index| {
                        assembly.span_point(
                            *span_id,
                            Vec2::new(length * index as f32 / segments as f32, 0.0),
                        )
                    })
                    .map(|point| Self::map_to_screen(map, *buffer.dim(), point))
                    .collect::<Vec<_>>();
                for pair in points.windows(2) {
                    buffer.draw_line(
                        pair[0].x,
                        pair[0].y,
                        pair[1].x,
                        pair[1].y,
                        [255, 204, 92, 255],
                    );
                    buffer.draw_line(
                        pair[0].x + 1,
                        pair[0].y,
                        pair[1].x + 1,
                        pair[1].y,
                        [255, 228, 145, 255],
                    );
                }
            }
        }
        self.hud.draw(buffer, map, ctx, server_ctx, None, assets);
        self.draw_wall_panel(buffer, map, ctx, server_ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_floor_is_not_a_wall_editing_hit_target() {
        let mut map = Map::default();
        let mut assembly = WallAssembly::new("Room".to_string());
        assembly.auto_floor = true;
        let start = assembly.add_node(Vec3::new(0.0, 0.0, 0.0));
        let end = assembly.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span_id = assembly.add_span(start, end).unwrap();
        let assembly_id = assembly.id;
        map.wall_assemblies.push(assembly);
        map.rebuild_wall_geometry();

        let floor_id = map
            .geometry_objects
            .iter()
            .find(|object| object.properties.get_bool_default("wall_auto_floor", false))
            .map(|object| object.id)
            .expect("automatic floor geometry");
        let span_object_id = map
            .geometry_objects
            .iter()
            .find(|object| {
                object.properties.get_id("wall_span_id") == Some(span_id)
                    && !object.properties.get_bool_default("wall_auto_floor", false)
            })
            .map(|object| object.id)
            .expect("wall span geometry");

        assert_eq!(
            WallTool::editable_wall_source_for_geometry_object(&map, floor_id),
            None
        );
        assert_eq!(
            WallTool::editable_wall_source_for_geometry_object(&map, span_object_id),
            Some((assembly_id, span_id))
        );
    }
}
