use crate::prelude::*;
use crate::{
    editor::{RUSTERIX, UNDOMANAGER},
    hud::{Hud, HudMode},
};
use MapEvent::*;
use ToolEvent::*;
use rusterix::prelude::*;
use scenevm::GeoId;
use shared::construction_graph::{
    self, ConstructionPatternAsset, ConstructionPatternKind, SurfacePatternProjection,
};

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

struct SurfaceRectDrag {
    start: Vec3<f32>,
    assembly_id: Uuid,
    surface_id: Uuid,
    previous: Map,
    changed: bool,
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
    node_drag: Option<WallNodeDrag>,
    ring_drag: Option<WallRingDrag>,
    opening_drag: Option<WallOpeningDrag>,
    surface_rect_drag: Option<SurfaceRectDrag>,
    build_pattern_id: Option<Uuid>,
    surface_pattern_id: Option<Uuid>,
    surface_projection: Option<SurfacePatternProjection>,
    ceiling_pattern_id: Option<Uuid>,
    ceiling_projection: Option<SurfacePatternProjection>,
    surface_elevation: f32,
    surface_thickness: f32,
    surface_clearance: f32,
    surface_kind: WallAreaSurfaceKind,
    surface_fill_preview: Option<Vec<Vec<Vec3<f32>>>>,
    previous_dock: Option<String>,
}

impl WallTool {
    const PANEL_X: i32 = 12;
    const PANEL_Y: i32 = 30;
    const PANEL_WIDTH: i32 = 276;
    const PANEL_HEIGHT: i32 = 190;
    const PANEL_ROW_Y: i32 = 112;
    const PANEL_ROW_SPACING: i32 = 24;
    const SURFACE_GAP_LIMIT: f32 = 4.0;

    fn configure_new_surface(&self, surface: &mut WallAreaSurface) {
        surface.kind = self.surface_kind;
        surface.elevation = self.surface_elevation;
        surface.thickness = self.surface_thickness;
        surface.clearance = self.surface_clearance;
        let (pattern_id, projection) = if self.surface_kind == WallAreaSurfaceKind::Ceiling {
            (self.ceiling_pattern_id, self.ceiling_projection.as_ref())
        } else {
            (self.surface_pattern_id, self.surface_projection.as_ref())
        };
        surface.pattern_id = pattern_id;
        if let Some(projection) = projection {
            construction_graph::apply_to_surface(surface, projection);
        }
    }

    fn cancel_surface_rect_drag(&mut self, map: &mut Map) {
        if let Some(drag) = self.surface_rect_drag.take() {
            *map = drag.previous;
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
        }
    }

    fn rect_outline(start: Vec3<f32>, end: Vec3<f32>) -> Vec<Vec3<f32>> {
        let (x0, x1) = (start.x.min(end.x), start.x.max(end.x));
        let (z0, z1) = (start.z.min(end.z), start.z.max(end.z));
        vec![
            Vec3::new(x0, 0.0, z0),
            Vec3::new(x1, 0.0, z0),
            Vec3::new(x1, 0.0, z1),
            Vec3::new(x0, 0.0, z1),
        ]
    }

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

    fn panel_surface_action_rect(index: i32) -> TheDim {
        TheDim::rect(
            Self::PANEL_X + 14 + index * 125,
            Self::PANEL_Y + 112,
            121,
            28,
        )
    }

    fn enclosed_surface_preview(&self, map: &Map) -> Vec<Vec<Vec3<f32>>> {
        map.wall_assemblies
            .iter()
            .flat_map(|assembly| {
                assembly
                    .inferred_surface_outlines(Self::SURFACE_GAP_LIMIT)
                    .into_iter()
                    .filter_map(|(outline, _)| {
                        (!assembly.has_area_surface_outline(self.surface_kind, &outline))
                            .then_some(outline)
                    })
            })
            .collect()
    }

    fn fill_enclosed_surfaces(&mut self, map: &mut Map) -> usize {
        // Doorways are temporary graph edges during detection; the authored walls are untouched.
        let regions = map
            .wall_assemblies
            .iter()
            .map(|assembly| {
                (
                    assembly.id,
                    assembly.floor_pixel_source(),
                    assembly.inferred_surface_outlines(Self::SURFACE_GAP_LIMIT),
                )
            })
            .collect::<Vec<_>>();
        let kind = self.surface_kind;
        let previous_elevation = self.surface_elevation;
        let mut created = 0;
        for (assembly_id, source, outlines) in regions {
            for (outline, wall_height) in outlines {
                let already_exists = map
                    .wall_assembly(assembly_id)
                    .is_some_and(|assembly| assembly.has_area_surface_outline(kind, &outline));
                if already_exists {
                    continue;
                }
                let mut surface = WallAreaSurface::new(Vec::new());
                surface.outline = outline.clone();
                surface.source = Some(source.clone());
                self.surface_elevation = if kind == WallAreaSurfaceKind::Floor {
                    0.0
                } else {
                    wall_height
                };
                self.configure_new_surface(&mut surface);
                // Retain the inferred fit until the user deliberately assigns a shared
                // Surface branch, which can then control elevation along with materials.
                surface.pattern_id = None;
                // Adjacent inferred regions meet at virtual doorway edges. An inset
                // would leave unsupported gaps across those passages.
                surface.clearance = 0.0;
                if kind == WallAreaSurfaceKind::Ceiling {
                    surface.elevation = wall_height + surface.thickness;
                } else {
                    surface.elevation = 0.0;
                }
                if let Some(assembly) = map.wall_assembly_mut(assembly_id) {
                    assembly.area_surfaces.push(surface);
                    created += 1;
                }
            }
        }
        self.surface_elevation = previous_elevation;
        if created > 0 {
            map.rebuild_wall_geometry();
        }
        created
    }

    fn update_ring_preview(
        map: &mut Map,
        drag: &mut WallRingDrag,
        style: &WallStyle,
        pattern_id: Option<Uuid>,
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
            assembly.pattern_id = pattern_id;
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
        self.cancel_surface_rect_drag(map);
        self.cancel_ring_drag(map);
        self.surface_fill_preview = None;
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
                fl!("construction_surface_mode_help"),
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
                assembly.area_surfaces.iter().any(|surface| {
                    surface.boundary == boundary && surface.kind == self.surface_kind
                })
            })
        {
            self.cancel_surface_preview(map);
            return;
        }
        let unchanged = map.wall_surface_preview.as_ref().is_some_and(|preview| {
            preview.assembly_id == assembly_id
                && preview.surface.boundary == boundary
                && preview.surface.kind == self.surface_kind
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
        surface.source = source;
        self.configure_new_surface(&mut surface);
        if let Some(preview) = map.wall_surface_preview.as_ref()
            && preview.assembly_id == assembly_id
            && preview.surface.boundary == surface.boundary
        {
            surface.id = preview.surface.id;
        }
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
        let previous = map.clone();
        match map.connect_wall_points(start, end, Self::snap_distance(map)) {
            Ok((assembly_id, span_id, start_node, end_node)) => {
                if let Some(assembly) = map.wall_assembly_mut(assembly_id) {
                    if assembly.spans.len() == 1 {
                        assembly.auto_floor = self.build_auto_floor;
                        assembly.pattern_id = self.build_pattern_id;
                        assembly.style = self.build_style.clone();
                    }
                    // New spans use their assembly's shared construction graph.
                    if let Some(span) = assembly.span_mut(span_id) {
                        span.style_override = None;
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
                match self.surface_kind {
                    WallAreaSurfaceKind::Floor => "CREATE FLOOR",
                    WallAreaSurfaceKind::Ceiling => "CREATE CEILING",
                }
            } else {
                "STYLE IN CONSTRUCTION GRAPH"
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
        if self.interaction_mode == WallInteractionMode::Surface {
            for (line, y) in [
                (fl!("construction_surface_click_loop"), Self::PANEL_Y + 72),
                (fl!("construction_surface_drag_open"), Self::PANEL_Y + 91),
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
                    &line,
                    TheFontSettings {
                        size: 11.0,
                        ..Default::default()
                    },
                    &[216, 219, 224, 255],
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }
        }
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
        } else if self.interaction_mode == WallInteractionMode::Surface {
            for (index, (kind, label)) in [
                (WallAreaSurfaceKind::Floor, "FLOOR"),
                (WallAreaSurfaceKind::Ceiling, "CEILING"),
            ]
            .into_iter()
            .enumerate()
            {
                Self::draw_panel_button(
                    buffer,
                    ctx,
                    Self::panel_build_mode_rect(index as i32),
                    label,
                    self.surface_kind == kind,
                    true,
                );
            }
            let ceilings_hidden = crate::editor::SCENEMANAGER
                .read()
                .unwrap()
                .preview_wall_surfaces_hidden()
                .1;
            Self::draw_panel_button(
                buffer,
                ctx,
                Self::panel_surface_action_rect(0),
                &if self.surface_fill_preview.is_some() {
                    fl!("construction_surface_create_all")
                } else {
                    fl!("construction_surface_fill_all")
                },
                self.surface_fill_preview.is_some(),
                true,
            );
            Self::draw_panel_button(
                buffer,
                ctx,
                Self::panel_surface_action_rect(1),
                &if ceilings_hidden {
                    fl!("construction_surface_show_ceilings")
                } else {
                    fl!("construction_surface_hide_ceilings")
                },
                ceilings_hidden,
                true,
            );
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
        if !matches!(
            self.interaction_mode,
            WallInteractionMode::Build | WallInteractionMode::Surface
        ) {
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
            WallInteractionMode::Surface => "Click loop · Shift-drag area · H hide",
        };
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(
                (Self::PANEL_X + 12) as usize,
                (Self::PANEL_Y + 160) as usize,
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
            node_drag: None,
            ring_drag: None,
            opening_drag: None,
            surface_rect_drag: None,
            build_pattern_id: None,
            surface_pattern_id: None,
            surface_projection: None,
            ceiling_pattern_id: None,
            ceiling_projection: None,
            surface_elevation: 0.25,
            surface_thickness: 0.08,
            surface_clearance: 0.015,
            surface_kind: WallAreaSurfaceKind::Floor,
            surface_fill_preview: None,
            previous_dock: None,
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
                self.node_drag = None;
                self.ring_drag = None;
                self.opening_drag = None;
                self.build_mode = WallBuildMode::Line;
                let needs_wall = !project
                    .construction_patterns
                    .values()
                    .any(|asset| asset.kind == ConstructionPatternKind::Wall);
                let needs_floor = !project.construction_patterns.values().any(|asset| {
                    asset.kind == ConstructionPatternKind::Surface
                        && asset.name != "Default Ceiling"
                });
                let needs_ceiling = !project.construction_patterns.values().any(|asset| {
                    asset.name == "Default Ceiling"
                        && asset.kind == ConstructionPatternKind::Surface
                });
                let before_patterns =
                    (needs_wall || needs_floor || needs_ceiling).then(|| project.clone());
                if needs_wall {
                    let style = WallStyle::default();
                    let pattern = ConstructionPatternAsset::new_pattern("Default Pattern", &style);
                    let pattern_id = pattern.id;
                    project.construction_patterns.insert(pattern_id, pattern);
                    let mut wall = ConstructionPatternAsset::new_wall("Default Wall", &style);
                    wall.graph = construction_graph::wall_reference_graph(&style, pattern_id);
                    construction_graph::set_root_name(&mut wall.graph, &wall.name);
                    project.construction_patterns.insert(wall.id, wall);
                }
                if needs_floor {
                    let asset = ConstructionPatternAsset::new_surface("Default Floor", 0.25, 0.08);
                    project.construction_patterns.insert(asset.id, asset);
                }
                if needs_ceiling {
                    let asset = ConstructionPatternAsset::new_surface("Default Ceiling", 3.0, 0.08);
                    project.construction_patterns.insert(asset.id, asset);
                }
                if let Some(before_patterns) = before_patterns {
                    UNDOMANAGER.write().unwrap().add_undo(
                        ProjectUndoAtom::ProjectEdit(
                            "Create construction patterns".into(),
                            Box::new(before_patterns),
                            Box::new(project.clone()),
                        ),
                        ctx,
                    );
                }
                self.build_pattern_id = project
                    .construction_patterns
                    .iter()
                    .find(|(_, asset)| asset.kind == ConstructionPatternKind::Wall)
                    .map(|(id, _)| *id);
                if let Some(asset) = self
                    .build_pattern_id
                    .and_then(|id| project.construction_patterns.get(&id))
                {
                    self.build_style = construction_graph::compile_wall_in_project(
                        &asset.graph,
                        &WallStyle::default(),
                        project,
                    )
                    .unwrap_or_default();
                }
                self.surface_pattern_id = project
                    .construction_patterns
                    .iter()
                    .find(|(_, asset)| {
                        asset.kind == ConstructionPatternKind::Surface
                            && asset.name != "Default Ceiling"
                    })
                    .map(|(id, _)| *id);
                self.surface_projection = self
                    .surface_pattern_id
                    .and_then(|id| project.construction_patterns.get(&id))
                    .and_then(|asset| {
                        construction_graph::compile_surface_with_patterns(
                            &asset.graph,
                            &project.construction_patterns,
                        )
                        .ok()
                    });
                self.ceiling_pattern_id = project
                    .construction_patterns
                    .iter()
                    .find(|(_, asset)| {
                        asset.kind == ConstructionPatternKind::Surface
                            && asset.name == "Default Ceiling"
                    })
                    .map(|(id, _)| *id);
                self.ceiling_projection = self
                    .ceiling_pattern_id
                    .and_then(|id| project.construction_patterns.get(&id))
                    .and_then(|asset| {
                        construction_graph::compile_surface_with_patterns(
                            &asset.graph,
                            &project.construction_patterns,
                        )
                        .ok()
                    });
                server_ctx.curr_map_tool_type = MapToolType::Wall;
                let current_dock = crate::editor::DOCKMANAGER.read().unwrap().dock.clone();
                if current_dock != "Construction" {
                    self.previous_dock = (!current_dock.is_empty()).then_some(current_dock);
                }
                crate::editor::DOCKMANAGER.write().unwrap().set_dock(
                    "Construction".into(),
                    _ui,
                    ctx,
                    project,
                    server_ctx,
                );
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
                self.node_drag = None;
                self.surface_fill_preview = None;
                self.opening_drag = None;
                self.build_mode = WallBuildMode::Line;
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    self.cancel_surface_rect_drag(map);
                    self.cancel_ring_drag(map);
                    self.finish_run(map);
                    self.cancel_opening(map);
                    self.cancel_brick_preview(map);
                    self.cancel_surface_preview(map);
                    map.selected_wall_opening = None;
                    map.selected_wall_surface = None;
                    map.hovered_wall_span = None;
                }
                if crate::editor::DOCKMANAGER.read().unwrap().dock == "Construction" {
                    let mut manager = crate::editor::DOCKMANAGER.write().unwrap();
                    manager.minimize_for_tool_switch(_ui, ctx, project, server_ctx);
                    if let Some(previous) = self.previous_dock.take() {
                        manager.set_dock(previous, _ui, ctx, project, server_ctx);
                    }
                }
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
            MapKey(key) if matches!(key, 'f' | 'F' | 'c' | 'C') => {
                self.set_interaction_mode(WallInteractionMode::Surface, map, ctx, server_ctx);
                self.cancel_surface_preview(map);
                map.selected_wall_surface = None;
                self.surface_kind = if matches!(key, 'f' | 'F') {
                    WallAreaSurfaceKind::Floor
                } else {
                    WallAreaSurfaceKind::Ceiling
                };
                self.surface_elevation = if self.surface_kind == WallAreaSurfaceKind::Floor {
                    0.25
                } else {
                    3.0
                };
                RUSTERIX.write().unwrap().set_overlay_dirty();
                ctx.ui.redraw_all = true;
                None
            }
            MapKey(key)
                if matches!(key, 'h' | 'H')
                    && self.interaction_mode == WallInteractionMode::Surface =>
            {
                let mut manager = crate::editor::SCENEMANAGER.write().unwrap();
                let (mut floors, mut ceilings) = manager.preview_wall_surfaces_hidden();
                match self.surface_kind {
                    WallAreaSurfaceKind::Floor => floors = !floors,
                    WallAreaSurfaceKind::Ceiling => ceilings = !ceilings,
                }
                manager.set_preview_wall_surfaces_hidden(floors, ceilings);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!(
                        "Editor preview: floors {}, ceilings {}. Game visibility is unchanged.",
                        if floors { "hidden" } else { "shown" },
                        if ceilings { "hidden" } else { "shown" }
                    ),
                ));
                ctx.ui.redraw_all = true;
                None
            }
            MapKey(key)
                if matches!(key, 'v' | 'V')
                    && self.interaction_mode == WallInteractionMode::Surface =>
            {
                crate::editor::SCENEMANAGER
                    .write()
                    .unwrap()
                    .set_preview_wall_surfaces_hidden(false, false);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Editor preview: floors and ceilings shown.".to_string(),
                ));
                ctx.ui.redraw_all = true;
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
                if self.interaction_mode == WallInteractionMode::Surface
                    && self.surface_rect_drag.is_none()
                {
                    let hit_existing = Self::surface_hit(map, server_ctx).is_some_and(
                        |(assembly_id, surface_id)| {
                            map.wall_assembly(assembly_id)
                                .and_then(|assembly| assembly.area_surface(surface_id))
                                .is_some_and(|surface| surface.kind == self.surface_kind)
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
                    if self.interaction_mode == WallInteractionMode::Surface {
                        for (index, kind) in
                            [WallAreaSurfaceKind::Floor, WallAreaSurfaceKind::Ceiling]
                                .into_iter()
                                .enumerate()
                        {
                            if Self::panel_build_mode_rect(index as i32).contains(coord) {
                                self.cancel_surface_preview(map);
                                self.surface_fill_preview = None;
                                map.selected_wall_surface = None;
                                self.surface_kind = kind;
                                self.surface_elevation = if kind == WallAreaSurfaceKind::Floor {
                                    0.25
                                } else {
                                    3.0
                                };
                                ctx.ui.redraw_all = true;
                                return None;
                            }
                        }
                        if Self::panel_surface_action_rect(0).contains(coord) {
                            self.cancel_surface_preview(map);
                            if self.surface_fill_preview.is_none() {
                                let preview = self.enclosed_surface_preview(map);
                                let count = preview.len();
                                self.surface_fill_preview =
                                    (!preview.is_empty()).then_some(preview);
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    if count == 0 {
                                        fl!("construction_surface_fill_none")
                                    } else {
                                        fl!("construction_surface_fill_preview", count = count)
                                    },
                                ));
                                ctx.ui.redraw_all = true;
                                return None;
                            }
                            self.surface_fill_preview = None;
                            let previous = map.clone();
                            let count = self.fill_enclosed_surfaces(map);
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                if count == 0 {
                                    fl!("construction_surface_fill_none")
                                } else {
                                    fl!("construction_surface_fill_created", count = count)
                                },
                            ));
                            if count > 0 {
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
                        if Self::panel_surface_action_rect(1).contains(coord) {
                            let mut manager = crate::editor::SCENEMANAGER.write().unwrap();
                            let (floors_hidden, ceilings_hidden) =
                                manager.preview_wall_surfaces_hidden();
                            manager
                                .set_preview_wall_surfaces_hidden(floors_hidden, !ceilings_hidden);
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                if ceilings_hidden {
                                    fl!("construction_surface_ceilings_shown")
                                } else {
                                    fl!("construction_surface_ceilings_hidden")
                                },
                            ));
                            ctx.ui.redraw_all = true;
                            return None;
                        }
                    }

                    return None;
                }
                if self.surface_fill_preview.take().is_some() {
                    ctx.ui.redraw_all = true;
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
                        && surface.kind == self.surface_kind
                    {
                        self.cancel_surface_preview(map);
                        if Self::select_area_surface(map, assembly_id, surface_id) {
                            self.surface_elevation = surface.elevation;
                            self.surface_thickness = surface.thickness;
                            self.surface_clearance = surface.clearance;
                            self.surface_kind = surface.kind;
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

                    if ui.shift {
                        self.cancel_surface_preview(map);
                        let start =
                            self.raw_pointer_position(ui, map, coord, server_ctx, Some(0.0))?;
                        let previous = map.clone();
                        let assembly_id = map
                            .selected_wall_assembly
                            .or_else(|| map.wall_assemblies.first().map(|assembly| assembly.id))
                            .unwrap_or_else(|| {
                                let assembly = WallAssembly::new("Surfaces");
                                let id = assembly.id;
                                map.wall_assemblies.push(assembly);
                                id
                            });
                        let mut surface = WallAreaSurface::new(Vec::new());
                        surface.source = map
                            .wall_assembly(assembly_id)
                            .map(WallAssembly::floor_pixel_source);
                        self.configure_new_surface(&mut surface);
                        let surface_id = surface.id;
                        map.wall_assembly_mut(assembly_id)?
                            .area_surfaces
                            .push(surface);
                        self.surface_rect_drag = Some(SurfaceRectDrag {
                            start,
                            assembly_id,
                            surface_id,
                            previous,
                            changed: false,
                        });
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Drag to draw a free floor or ceiling rectangle.".to_string(),
                        ));
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
                                surface.source = resolved_map
                                    .wall_assembly(assembly_id)
                                    .map(WallAssembly::floor_pixel_source);
                                self.configure_new_surface(&mut surface);
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
                            fl!("construction_surface_open_area"),
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
                        .find(|surface| {
                            surface.boundary == preview.surface.boundary
                                && surface.kind == preview.surface.kind
                        })
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
                        fl!("construction_surface_created"),
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
                if let Some(drag) = self.surface_rect_drag.as_ref() {
                    let Some(end) =
                        self.raw_pointer_position(ui, map, coord, server_ctx, Some(0.0))
                    else {
                        return None;
                    };
                    let step = ServerContext::edit_grid_step(map.subdivisions).max(0.05);
                    let changed = (end.x - drag.start.x).abs() >= step * 0.5
                        && (end.z - drag.start.z).abs() >= step * 0.5;
                    if changed {
                        let outline = Self::rect_outline(drag.start, end);
                        if let Some(surface) = map
                            .wall_assembly_mut(drag.assembly_id)
                            .and_then(|assembly| assembly.area_surface_mut(drag.surface_id))
                        {
                            surface.outline = outline;
                            map.rebuild_wall_geometry();
                            if let Some(drag) = self.surface_rect_drag.as_mut() {
                                drag.changed = true;
                            }
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.set_dirty();
                            rusterix.set_overlay_dirty();
                            ctx.ui.redraw_all = true;
                        }
                    }
                    return None;
                }
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
                        Self::update_ring_preview(
                            map,
                            drag,
                            &style,
                            self.build_pattern_id,
                            auto_floor,
                            radius,
                        )
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
                self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx);
                None
            }
            MapUp(_) => {
                if let Some(drag) = self.surface_rect_drag.take() {
                    if !drag.changed {
                        *map = drag.previous;
                        return None;
                    }
                    Self::select_area_surface(map, drag.assembly_id, drag.surface_id);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                    ctx.ui.redraw_all = true;
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(drag.previous),
                        Box::new(map.clone()),
                    ));
                }
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
                None
            }
            MapEscape => {
                if self.surface_rect_drag.is_some() {
                    self.cancel_surface_rect_drag(map);
                    return None;
                }
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
                    if self.surface_fill_preview.take().is_some() {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("construction_surface_fill_cancelled"),
                        ));
                        ctx.ui.redraw_all = true;
                        return None;
                    }
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
        if let Some(preview) = &self.surface_fill_preview {
            let dim = *buffer.dim();
            let camera = if server_ctx.editor_view_mode == EditorViewMode::D2 {
                None
            } else {
                RUSTERIX.read().ok().map(|rusterix| {
                    (
                        rusterix.client.camera_d3.view_matrix(),
                        rusterix
                            .client
                            .camera_d3
                            .projection_matrix(dim.width as f32, dim.height as f32),
                    )
                })
            };
            let project = |point: Vec3<f32>| -> Option<Vec2<i32>> {
                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    return Some(Self::map_to_screen(map, dim, point));
                }
                let (view, projection) = camera.as_ref()?;
                let clip = (*projection).clone()
                    * (*view).clone()
                    * Vec4::new(point.x, point.y + 0.5, point.z, 1.0);
                if clip.w <= 0.0 || !clip.w.is_finite() {
                    return None;
                }
                let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
                if !ndc.x.is_finite() || !ndc.y.is_finite() || !(-1.0..=1.0).contains(&ndc.z) {
                    return None;
                }
                Some(Vec2::new(
                    ((ndc.x * 0.5 + 0.5) * dim.width as f32).round() as i32,
                    ((1.0 - (ndc.y * 0.5 + 0.5)) * dim.height as f32).round() as i32,
                ))
            };
            for outline in preview {
                for index in 0..outline.len() {
                    if let (Some(a), Some(b)) = (
                        project(outline[index]),
                        project(outline[(index + 1) % outline.len()]),
                    ) {
                        buffer.draw_line(a.x, a.y, b.x, b.y, [105, 239, 158, 255]);
                        buffer.draw_line(a.x + 1, a.y, b.x + 1, b.y, [23, 103, 64, 255]);
                    }
                }
            }
        }
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
    fn bulk_surfaces_respect_selected_kind_and_do_not_duplicate() {
        let mut map = Map::default();
        let mut assembly = WallAssembly::new("Room".to_string());
        let nodes = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 4.0),
            Vec3::new(0.0, 0.0, 4.0),
        ]
        .map(|position| assembly.add_node(position));
        for i in 0..4 {
            assembly.add_span(nodes[i], nodes[(i + 1) % 4]).unwrap();
        }
        map.wall_assemblies.push(assembly);
        let mut tool = WallTool::new();
        for kind in [WallAreaSurfaceKind::Floor, WallAreaSurfaceKind::Ceiling] {
            tool.surface_kind = kind;
            assert_eq!(tool.enclosed_surface_preview(&map).len(), 1);
            assert_eq!(tool.fill_enclosed_surfaces(&mut map), 1);
            let surfaces = &map.wall_assemblies[0].area_surfaces;
            assert_eq!(
                surfaces
                    .iter()
                    .filter(|surface| surface.kind == kind)
                    .count(),
                1
            );
            assert_eq!(surfaces.last().unwrap().clearance, 0.0);
            assert!(tool.enclosed_surface_preview(&map).is_empty());
            assert_eq!(tool.fill_enclosed_surfaces(&mut map), 0);
        }
        assert_eq!(map.wall_assemblies[0].area_surfaces.len(), 2);
    }

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
