use crate::actions::edit_vertex::EDIT_VERTEX_ACTION_ID;
use crate::editor::RUSTERIX;
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::Assets;
use rusterix::Surface;
use rusterix::prelude::*;
use scenevm::GeoId;
use std::str::FromStr;

pub struct VertexTool {
    id: TheId,
    click_pos: Vec2<f32>,
    click_pos_3d: Vec3<f32>,
    /// The initial ray intersection point on the drag plane at click time
    click_ray_intersection_3d: Option<Vec3<f32>>,
    click_selected: bool,
    drag_changed: bool,
    rectangle_undo_map: Map,
    was_clicked: bool,

    hud: Hud,
}

impl Tool for VertexTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Vertex Tool"),
            click_pos: Vec2::zero(),
            click_pos_3d: Vec3::zero(),
            click_ray_intersection_3d: None,
            click_selected: false,
            drag_changed: false,
            rectangle_undo_map: Map::default(),
            was_clicked: false,

            hud: Hud::new(HudMode::Vertex),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        fl!("tool_vertex")
    }
    fn icon_name(&self) -> String {
        str!("dot-outline")
    }
    fn accel(&self) -> Option<char> {
        Some('V')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/vertex".to_string())
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
                server_ctx.curr_map_tool_type = MapToolType::Vertex;

                if let Some(map) = project.get_map_mut(server_ctx) {
                    map.selected_linedefs.clear();
                    map.selected_sectors.clear();
                }

                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));

                return true;
            }
            DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                return true;
            }
            _ => {}
        };
        false
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut undo_atom: Option<ProjectUndoAtom> = None;
        let detail_mode_3d = server_ctx.editor_view_mode != EditorViewMode::D2
            && server_ctx.geometry_edit_mode == GeometryEditMode::Detail;

        fn detail_surface_at_point(map: &Map, point: Vec3<f32>) -> Option<Surface> {
            let mut best_surface: Option<(Surface, f32)> = None;
            for surface in map.surfaces.values() {
                let loop_uv = match surface.sector_loop_uv(map) {
                    Some(loop_uv) if !loop_uv.is_empty() => loop_uv,
                    _ => continue,
                };
                let uv = surface.world_to_uv(point);
                let mut min = loop_uv[0];
                let mut max = loop_uv[0];
                for p in loop_uv.iter().skip(1) {
                    min.x = min.x.min(p.x);
                    min.y = min.y.min(p.y);
                    max.x = max.x.max(p.x);
                    max.y = max.y.max(p.y);
                }
                let eps = 0.01;
                if uv.x < min.x - eps
                    || uv.x > max.x + eps
                    || uv.y < min.y - eps
                    || uv.y > max.y + eps
                {
                    continue;
                }
                let n = surface.plane.normal;
                let n_len = n.magnitude();
                if n_len <= 1e-6 {
                    continue;
                }
                let dist = ((point - surface.plane.origin).dot(n / n_len)).abs();
                if best_surface
                    .as_ref()
                    .map(|(_, best_dist)| dist < *best_dist)
                    .unwrap_or(true)
                {
                    best_surface = Some((surface.clone(), dist));
                }
            }
            best_surface.map(|(surface, _)| surface)
        }

        match map_event {
            MapKey(c) => {
                match c {
                    '1'..='9' => map.subdivisions = (c as u8 - b'0') as f32,
                    '0' => map.subdivisions = 10.0,
                    _ => {}
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            MapClicked(coord) => {
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    self.was_clicked = false;
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }
                self.was_clicked = true;

                if detail_mode_3d
                    && let Some(surface) = detail_surface_at_point(
                        map,
                        server_ctx.hover_cursor_3d.unwrap_or(server_ctx.geo_hit_pos),
                    )
                    && server_ctx.active_detail_surface.as_ref().map(|s| s.id) != Some(surface.id)
                {
                    if let Some(old_profile_id) = server_ctx
                        .active_detail_surface
                        .as_ref()
                        .and_then(|active| active.profile)
                        && let Some(old_profile_map) = map.profiles.get_mut(&old_profile_id)
                    {
                        old_profile_map.selected_vertices.clear();
                    }
                    server_ctx.active_detail_surface = Some(surface);
                    map.curr_grid_pos = None;
                    map.curr_grid_pos_3d = None;
                    map.clear_temp();
                }

                if detail_mode_3d {
                    server_ctx.hover.0 = match server_ctx.geo_hit {
                        Some(GeoId::Vertex(id)) => Some(id),
                        _ => None,
                    };
                }

                self.click_selected = false;
                if server_ctx.hover.0.is_some() {
                    let mut changed = false;

                    map.selected_entity_item = None;

                    if detail_mode_3d
                        && let Some(surface) = server_ctx.active_detail_surface.as_ref()
                        && let Some(profile_id) = surface.profile
                        && let Some(profile_map) = map.profiles.get_mut(&profile_id)
                    {
                        if ui.shift {
                            if let Some(v) = server_ctx.hover.0 {
                                if !profile_map.selected_vertices.contains(&v) {
                                    profile_map.selected_vertices.push(v);
                                    changed = true;
                                }
                            }
                            self.click_selected = true;
                        } else if ui.alt {
                            if let Some(v) = server_ctx.hover.0 {
                                profile_map
                                    .selected_vertices
                                    .retain(|&selected| selected != v);
                                changed = true;
                            }
                        } else {
                            if let Some(v) = server_ctx.hover.0 {
                                profile_map.selected_vertices = vec![v];
                                changed = true;
                            } else {
                                profile_map.selected_vertices.clear();
                                changed = true;
                            }
                            self.click_selected = true;
                        }
                    } else {
                        if ui.shift {
                            // Add
                            if let Some(v) = server_ctx.hover.0 {
                                if !map.selected_vertices.contains(&v) {
                                    map.selected_vertices.push(v);
                                    changed = true;
                                }
                            }
                            self.click_selected = true;
                        } else if ui.alt {
                            // Subtract
                            if let Some(v) = server_ctx.hover.0 {
                                map.selected_vertices.retain(|&selected| selected != v);
                                changed = true;
                            }
                        } else {
                            // Replace
                            if let Some(v) = server_ctx.hover.0 {
                                map.selected_vertices = vec![v];
                                changed = true;
                            } else {
                                map.selected_vertices.clear();
                                changed = true;
                            }
                            self.click_selected = true;
                        }
                    }

                    if changed {
                        server_ctx.curr_action_id =
                            Some(Uuid::from_str(EDIT_VERTEX_ACTION_ID).unwrap());

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                    }
                } else {
                    if ui.shift {
                        // Add a new vertex
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            if server_ctx.editor_view_mode == EditorViewMode::D2 {
                                let prev = map.clone();
                                let dim = *render_view.dim();
                                let grid_pos = server_ctx.local_to_map_grid(
                                    Vec2::new(dim.width as f32, dim.height as f32),
                                    Vec2::new(coord.x as f32, coord.y as f32),
                                    map,
                                    map.subdivisions,
                                );

                                let id = map.add_vertex_at(grid_pos.x, grid_pos.y);
                                map.selected_vertices = vec![id];

                                server_ctx.curr_action_id =
                                    Some(Uuid::from_str(EDIT_VERTEX_ACTION_ID).unwrap());

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Map Selection Changed"),
                                    TheValue::Empty,
                                ));

                                undo_atom = Some(ProjectUndoAtom::MapEdit(
                                    server_ctx.pc,
                                    Box::new(prev),
                                    Box::new(map.clone()),
                                ));
                            } else if let Some(pt) = server_ctx.hover_cursor_3d {
                                let prev = map.clone();
                                let snapped = server_ctx.snap_world_point_for_edit(map, pt);
                                if detail_mode_3d
                                    && let Some(surface) = server_ctx.active_detail_surface.clone()
                                {
                                    let mut profile_to_add = None;
                                    let profile_id = if let Some(surface_mut) =
                                        map.surfaces.get_mut(&surface.id)
                                    {
                                        if surface_mut.profile.is_none() {
                                            let profile = Map::default();
                                            surface_mut.profile = Some(profile.id);
                                            profile_to_add = Some(profile);
                                        }
                                        surface_mut.profile
                                    } else {
                                        None
                                    };

                                    if let Some(profile) = profile_to_add {
                                        map.profiles.insert(profile.id, profile.clone());
                                        if let Some(active) =
                                            server_ctx.active_detail_surface.as_mut()
                                            && active.id == surface.id
                                        {
                                            active.profile = Some(profile.id);
                                        }
                                    }

                                    if let Some(profile_id) = profile_id
                                        && let Some(profile_map) = map.profiles.get_mut(&profile_id)
                                    {
                                        let uv = surface.world_to_uv(snapped);
                                        let id = profile_map.add_vertex_at(uv.x, -uv.y);
                                        profile_map.selected_vertices = vec![id];
                                    }
                                } else {
                                    let id = map
                                        .add_vertex_at_3d(snapped.x, snapped.z, snapped.y, false);
                                    map.selected_vertices = vec![id];
                                }

                                server_ctx.curr_action_id =
                                    Some(Uuid::from_str(EDIT_VERTEX_ACTION_ID).unwrap());

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Map Selection Changed"),
                                    TheValue::Empty,
                                ));

                                undo_atom = Some(ProjectUndoAtom::MapEdit(
                                    server_ctx.pc,
                                    Box::new(prev),
                                    Box::new(map.clone()),
                                ));
                            }
                        }
                    }
                }

                self.click_pos = Vec2::new(coord.x as f32, coord.y as f32);
                self.click_ray_intersection_3d = None;

                // For 3D dragging, use the actual vertex position if one is selected
                if detail_mode_3d
                    && self.click_selected
                    && let Some(surface) = server_ctx.active_detail_surface.as_ref()
                    && let Some(profile_id) = surface.profile
                    && let Some(profile_map) = map.profiles.get(&profile_id)
                    && !profile_map.selected_vertices.is_empty()
                {
                    let vertex_id = profile_map.selected_vertices[0];
                    if let Some(vertex) = profile_map.find_vertex(vertex_id) {
                        self.click_pos_3d = surface.uv_to_world(Vec2::new(vertex.x, -vertex.y));
                    } else {
                        self.click_pos_3d = server_ctx.geo_hit_pos;
                    }

                    if let Some(render_view) = ui.get_render_view("PolyView") {
                        let dim = *render_view.dim();
                        let screen_uv = [
                            coord.x as f32 / dim.width as f32,
                            coord.y as f32 / dim.height as f32,
                        ];

                        let rusterix = RUSTERIX.read().unwrap();
                        let ray = rusterix.client.camera_d3.create_ray(
                            Vec2::new(screen_uv[0], 1.0 - screen_uv[1]),
                            Vec2::new(dim.width as f32, dim.height as f32),
                            Vec2::zero(),
                        );
                        drop(rusterix);

                        let plane_normal = surface.frame.normal;
                        let denom: f32 = plane_normal.dot(ray.dir);
                        if denom.abs() > 0.0001 {
                            let t = (self.click_pos_3d - ray.origin).dot(plane_normal) / denom;
                            if t >= 0.0 {
                                self.click_ray_intersection_3d = Some(ray.origin + ray.dir * t);
                            }
                        }
                    }
                } else if self.click_selected && !map.selected_vertices.is_empty() {
                    if let Some(vertex) = map.find_vertex(map.selected_vertices[0]) {
                        // Convert vertex storage to world 3D coords
                        self.click_pos_3d = vertex.as_vec3_world();
                    } else {
                        self.click_pos_3d = server_ctx.geo_hit_pos;
                    }

                    // Compute initial ray intersection on the drag plane at click time
                    if server_ctx.editor_view_mode != EditorViewMode::D2 {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            let screen_uv = [
                                coord.x as f32 / dim.width as f32,
                                coord.y as f32 / dim.height as f32,
                            ];

                            let rusterix = RUSTERIX.read().unwrap();
                            let ray = rusterix.client.camera_d3.create_ray(
                                Vec2::new(screen_uv[0], 1.0 - screen_uv[1]),
                                Vec2::new(dim.width as f32, dim.height as f32),
                                Vec2::zero(),
                            );
                            drop(rusterix);

                            let plane = server_ctx.gizmo_mode;
                            let plane_normal = match plane {
                                GizmoMode::XZ => Vec3::new(0.0, 1.0, 0.0),
                                GizmoMode::XY => Vec3::new(0.0, 0.0, 1.0),
                                GizmoMode::YZ => Vec3::new(1.0, 0.0, 0.0),
                            };

                            let denom: f32 = plane_normal.dot(ray.dir);
                            if denom.abs() > 0.0001 {
                                let t = (self.click_pos_3d - ray.origin).dot(plane_normal) / denom;
                                if t >= 0.0 {
                                    self.click_ray_intersection_3d = Some(ray.origin + ray.dir * t);
                                }
                            }
                        }
                    }
                } else {
                    self.click_pos_3d = server_ctx.geo_hit_pos;
                }

                if detail_mode_3d
                    && let Some(surface) = server_ctx.active_detail_surface.as_ref()
                    && let Some(profile_id) = surface.profile
                    && let Some(profile_map) = map.profiles.get(&profile_id)
                {
                    self.rectangle_undo_map = profile_map.clone();
                } else {
                    self.rectangle_undo_map = map.clone();
                }
            }
            MapDragged(coord) => {
                if self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if self.click_selected {
                    // If we selected a vertex, drag means we move all selected vertices
                    if let Some(render_view) = ui.get_render_view("PolyView") {
                        let dim = *render_view.dim();

                        if server_ctx.editor_view_mode == EditorViewMode::D2 {
                            let click_pos = server_ctx.local_to_map_grid(
                                Vec2::new(dim.width as f32, dim.height as f32),
                                self.click_pos,
                                map,
                                map.subdivisions,
                            );
                            let drag_pos = server_ctx.local_to_map_grid(
                                Vec2::new(dim.width as f32, dim.height as f32),
                                Vec2::new(coord.x as f32, coord.y as f32),
                                map,
                                map.subdivisions,
                            );

                            let drag_delta = click_pos - drag_pos;
                            for vertex_id in &map.selected_vertices.clone() {
                                if let Some(original_vertex) =
                                    self.rectangle_undo_map.find_vertex_mut(*vertex_id)
                                {
                                    let new_pos = Vec2::new(
                                        original_vertex.x - drag_delta.x,
                                        original_vertex.y - drag_delta.y,
                                    );
                                    let grid_step = 1.0 / map.subdivisions.max(1.0);
                                    let snapped_pos = Vec2::new(
                                        (new_pos.x / grid_step).round() * grid_step,
                                        (new_pos.y / grid_step).round() * grid_step,
                                    );
                                    map.update_vertex(*vertex_id, snapped_pos);
                                }
                            }
                            server_ctx.hover_cursor = Some(drag_pos);

                            if drag_delta.x != 0.0 || drag_delta.y != 0.0 {
                                self.drag_changed = true;
                            }
                        } else if detail_mode_3d
                            && let Some(surface) = server_ctx.active_detail_surface.as_ref()
                            && let Some(profile_id) = surface.profile
                        {
                            let drag_distance = self
                                .click_pos
                                .distance(Vec2::new(coord.x as f32, coord.y as f32));
                            if drag_distance < 5.0 {
                                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                return None;
                            }

                            let click_intersection = match self.click_ray_intersection_3d {
                                Some(pos) => pos,
                                None => {
                                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                    return None;
                                }
                            };

                            if let Some(render_view) = ui.get_render_view("PolyView") {
                                let dim = *render_view.dim();
                                let screen_uv = [
                                    coord.x as f32 / dim.width as f32,
                                    coord.y as f32 / dim.height as f32,
                                ];
                                let rusterix = RUSTERIX.read().unwrap();
                                let ray = rusterix.client.camera_d3.create_ray(
                                    Vec2::new(screen_uv[0], 1.0 - screen_uv[1]),
                                    Vec2::new(dim.width as f32, dim.height as f32),
                                    Vec2::zero(),
                                );
                                drop(rusterix);

                                let plane_normal = surface.frame.normal;
                                let denom: f32 = plane_normal.dot(ray.dir);
                                if denom.abs() > 0.0001
                                    && let Some(profile_map) = map.profiles.get_mut(&profile_id)
                                {
                                    let t =
                                        (self.click_pos_3d - ray.origin).dot(plane_normal) / denom;
                                    if t >= 0.0 {
                                        let current_pos = ray.origin + ray.dir * t;
                                        let start_uv = surface.world_to_uv(click_intersection);
                                        let current_uv = surface.world_to_uv(current_pos);
                                        let drag_delta_uv = current_uv - start_uv;
                                        let step = 1.0 / map.subdivisions.max(1.0);

                                        for vertex_id in
                                            &self.rectangle_undo_map.selected_vertices.clone()
                                        {
                                            if let Some(original_vertex) =
                                                self.rectangle_undo_map.find_vertex(*vertex_id)
                                                && let Some(vertex) =
                                                    profile_map.find_vertex_mut(*vertex_id)
                                            {
                                                vertex.x = ((original_vertex.x + drag_delta_uv.x)
                                                    / step)
                                                    .round()
                                                    * step;
                                                vertex.y = ((original_vertex.y - drag_delta_uv.y)
                                                    / step)
                                                    .round()
                                                    * step;
                                            }
                                        }

                                        if drag_delta_uv.x != 0.0 || drag_delta_uv.y != 0.0 {
                                            self.drag_changed = true;
                                        }
                                    }
                                }
                            }
                        } else {
                            // 3D Drag
                            // Only start dragging after a minimum distance threshold
                            let drag_distance = self
                                .click_pos
                                .distance(Vec2::new(coord.x as f32, coord.y as f32));
                            if drag_distance < 5.0 {
                                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                return None;
                            }

                            // Use the initial ray intersection as reference (not vertex position)
                            // This prevents the "jump" when starting to drag
                            let click_intersection = match self.click_ray_intersection_3d {
                                Some(pos) => pos,
                                None => {
                                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                    return None;
                                }
                            };

                            let start_pos = self.click_pos_3d;
                            let plane = server_ctx.gizmo_mode;

                            // Get current mouse ray and intersect with drag plane
                            if let Some(render_view) = ui.get_render_view("PolyView") {
                                let dim = *render_view.dim();
                                let screen_uv = [
                                    coord.x as f32 / dim.width as f32,
                                    coord.y as f32 / dim.height as f32,
                                ];

                                // Get the camera ray for current mouse position
                                let rusterix = RUSTERIX.read().unwrap();
                                let ray = rusterix.client.camera_d3.create_ray(
                                    Vec2::new(screen_uv[0], 1.0 - screen_uv[1]),
                                    Vec2::new(dim.width as f32, dim.height as f32),
                                    Vec2::zero(),
                                );
                                drop(rusterix);

                                // Define plane normal based on gizmo mode
                                let plane_normal = match plane {
                                    GizmoMode::XZ => Vec3::new(0.0, 1.0, 0.0), // Horizontal plane
                                    GizmoMode::XY => Vec3::new(0.0, 0.0, 1.0), // Front plane
                                    GizmoMode::YZ => Vec3::new(1.0, 0.0, 0.0), // Side plane
                                };

                                // Ray-plane intersection
                                let denom: f32 = plane_normal.dot(ray.dir);

                                if denom.abs() > 0.0001 {
                                    let t = (start_pos - ray.origin).dot(plane_normal) / denom;
                                    if t >= 0.0 {
                                        let current_pos = ray.origin + ray.dir * t;

                                        // Calculate drag delta relative to initial click intersection
                                        // (not the vertex position)
                                        let drag_delta = match plane {
                                            GizmoMode::XZ => {
                                                // XZ plane: allow movement in X and Z, lock Y
                                                Vec3::new(
                                                    current_pos.x - click_intersection.x,
                                                    0.0,
                                                    current_pos.z - click_intersection.z,
                                                )
                                            }
                                            GizmoMode::XY => {
                                                // XY plane: allow movement in X and Y, lock Z
                                                Vec3::new(
                                                    current_pos.x - click_intersection.x,
                                                    current_pos.y - click_intersection.y,
                                                    0.0,
                                                )
                                            }
                                            GizmoMode::YZ => {
                                                // YZ plane: allow movement in Y and Z, lock X
                                                Vec3::new(
                                                    0.0,
                                                    current_pos.y - click_intersection.y,
                                                    current_pos.z - click_intersection.z,
                                                )
                                            }
                                        };

                                        // Apply drag delta to all selected vertices
                                        for vertex_id in &map.selected_vertices.clone() {
                                            if let Some(original_vertex) =
                                                self.rectangle_undo_map.find_vertex(*vertex_id)
                                            {
                                                // Coordinate mapping:
                                                // vertex.x = world X, vertex.y = world Z, vertex.z = world Y
                                                // drag_delta is in world coords (x, y, z) where y is up
                                                // update_vertex_3d takes (vertex_id, new_x, new_z, new_y)
                                                let new_x = original_vertex.x + drag_delta.x;
                                                let new_y = original_vertex.y + drag_delta.z;
                                                let new_z = original_vertex.z + drag_delta.y;

                                                // Snap to grid
                                                let subdivisions = 1.0 / map.subdivisions;
                                                let snapped_x =
                                                    (new_x / subdivisions).round() * subdivisions;
                                                let snapped_y =
                                                    (new_y / subdivisions).round() * subdivisions;
                                                let snapped_z =
                                                    (new_z / subdivisions).round() * subdivisions;

                                                if let Some(vertex) =
                                                    map.find_vertex_mut(*vertex_id)
                                                {
                                                    vertex.x = snapped_x;
                                                    vertex.y = snapped_y;
                                                    vertex.z = snapped_z;
                                                }
                                            }
                                        }
                                        if drag_delta.x != 0.0
                                            || drag_delta.y != 0.0
                                            || drag_delta.z != 0.0
                                        {
                                            self.drag_changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(render_view) = ui.get_render_view("PolyView") {
                    if detail_mode_3d {
                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                        return None;
                    }

                    if !self.was_clicked {
                        return None;
                    }

                    // Otherwise we treat it as rectangle selection
                    let dim = *render_view.dim();

                    let click_pos = server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        self.click_pos,
                        map,
                        map.subdivisions,
                    );
                    let drag_pos = server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    );

                    let selection = if server_ctx.editor_view_mode == EditorViewMode::D2 {
                        let top_left =
                            Vec2::new(click_pos.x.min(drag_pos.x), click_pos.y.min(drag_pos.y));
                        let bottom_right =
                            Vec2::new(click_pos.x.max(drag_pos.x), click_pos.y.max(drag_pos.y));

                        let mut selection =
                            server_ctx.geometry_in_rectangle(top_left, bottom_right, map);
                        selection.1 = vec![];
                        selection.2 = vec![];
                        selection
                    } else {
                        let mut selection = (vec![], vec![], vec![]);

                        let click_pos = self.click_pos;
                        let drag_pos = Vec2::new(coord.x as f32, coord.y as f32);

                        let top_left =
                            Vec2::new(click_pos.x.min(drag_pos.x), click_pos.y.min(drag_pos.y));
                        let bottom_right =
                            Vec2::new(click_pos.x.max(drag_pos.x), click_pos.y.max(drag_pos.y));

                        let mut rusterix = RUSTERIX.write().unwrap();
                        rusterix.scene_handler.vm.set_active_vm(2);
                        let vertices = rusterix.scene_handler.vm.active_vm().pick_geo_ids_in_rect(
                            dim.width as u32,
                            dim.height as u32,
                            top_left,
                            bottom_right,
                            GeoId::Vertex(0),
                            false,
                            false,
                        );
                        for v in vertices {
                            if let GeoId::Vertex(v) = v {
                                selection.0.push(v);
                            }
                        }
                        rusterix.scene_handler.vm.set_active_vm(0);
                        selection
                    };

                    *map = self.rectangle_undo_map.clone();
                    map.curr_rectangle = Some((click_pos, drag_pos));

                    if ui.shift {
                        // Add
                        map.add_to_selection(selection.0, selection.1, selection.2);
                    } else if ui.alt {
                        // Remove
                        map.remove_from_selection(selection.0, selection.1, selection.2);
                    } else {
                        // Replace
                        map.selected_vertices = selection.0;
                    }
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            MapUp(_) => {
                if self.click_selected {
                    if self.drag_changed {
                        undo_atom = Some(ProjectUndoAtom::MapEdit(
                            server_ctx.pc,
                            Box::new(self.rectangle_undo_map.clone()),
                            Box::new(map.clone()),
                        ));

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                    }
                } else if map.curr_rectangle.is_some() {
                    map.curr_rectangle = None;

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
                self.drag_changed = false;
                self.click_selected = false;
                self.was_clicked = false;
            }
            MapHover(coord) => {
                if self.hud.hovered(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    if let Some(render_view) = ui.get_render_view("PolyView") {
                        let dim = *render_view.dim();
                        let h = server_ctx.geometry_at(
                            Vec2::new(dim.width as f32, dim.height as f32),
                            Vec2::new(coord.x as f32, coord.y as f32),
                            map,
                        );
                        server_ctx.hover.0 = h.0;

                        let cp = server_ctx.local_to_map_grid(
                            Vec2::new(dim.width as f32, dim.height as f32),
                            Vec2::new(coord.x as f32, coord.y as f32),
                            map,
                            map.subdivisions,
                        );

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Cursor Pos Changed"),
                            TheValue::Float2(cp),
                        ));

                        server_ctx.hover_cursor = Some(cp);
                    }
                } else {
                    if let Some(geo_id) = server_ctx.geo_hit {
                        match geo_id {
                            GeoId::Vertex(id) => {
                                server_ctx.hover = (Some(id), None, None);
                            }
                            _ => {
                                server_ctx.hover = (None, None, None);
                            }
                        }
                    } else {
                        server_ctx.hover = (None, None, None);
                    }

                    if let Some(cp) = server_ctx.hover_cursor {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Cursor Pos Changed"),
                            TheValue::Float2(cp),
                        ));
                    }
                }
                if let Some(v) = server_ctx.hover.0 {
                    if let Some(vertex) = map.find_vertex(v) {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            format!(
                                "Vertex {}: (X: {:.2}, Y: {:.2}, Z: {:.2})",
                                v, vertex.x, vertex.z, vertex.y
                            ),
                        ));
                    }
                } else {
                    ctx.ui
                        .send(TheEvent::SetStatusText(TheId::empty(), "".into()));
                }
            }
            MapDelete => {
                let prev = map.clone();
                if detail_mode_3d
                    && let Some(surface) = server_ctx.active_detail_surface.as_ref()
                    && let Some(profile_id) = surface.profile
                    && let Some(profile_map) = map.profiles.get_mut(&profile_id)
                    && !profile_map.selected_vertices.is_empty()
                {
                    let vertices = profile_map.selected_vertices.clone();
                    profile_map.delete_elements(&vertices, &[], &[]);
                    profile_map.selected_vertices.clear();

                    undo_atom = Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(prev),
                        Box::new(map.clone()),
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                } else if !map.selected_vertices.is_empty() {
                    let prev = map.clone();
                    let vertices = map.selected_vertices.clone();

                    #[allow(clippy::useless_vec)]
                    map.delete_elements(&vertices, &vec![], &vec![]);
                    map.selected_vertices.clear();

                    undo_atom = Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(prev),
                        Box::new(map.clone()),
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
            }
            MapEscape => {
                if detail_mode_3d
                    && let Some(surface) = server_ctx.active_detail_surface.as_ref()
                    && let Some(profile_id) = surface.profile
                    && let Some(profile_map) = map.profiles.get_mut(&profile_id)
                    && !profile_map.selected_vertices.is_empty()
                {
                    profile_map.selected_vertices.clear();

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                } else if !map.selected_vertices.is_empty() {
                    map.selected_vertices.clear();

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
                self.was_clicked = false;
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
        };
        undo_atom
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        assets: &Assets,
    ) {
        let id = if server_ctx.editor_view_mode != EditorViewMode::D2
            && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
            && let Some(surface) = server_ctx.active_detail_surface.as_ref()
            && let Some(profile_id) = surface.profile
            && let Some(profile_map) = map.profiles.get(&profile_id)
            && !profile_map.selected_vertices.is_empty()
        {
            Some(profile_map.selected_vertices[0])
        } else if !map.selected_vertices.is_empty() {
            Some(map.selected_vertices[0])
        } else {
            None
        };
        self.hud.draw(buffer, map, ctx, server_ctx, id, assets);
    }
}
