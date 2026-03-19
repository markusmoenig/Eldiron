use crate::actions::edit_linedef::EDIT_LINEDEF_ACTION_ID;
use crate::editor::RUSTERIX;
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::{Surface, prelude::*};
use scenevm::GeoId;
use std::str::FromStr;
use vek::Vec2;

pub struct LinedefTool {
    id: TheId,
    click_pos: Vec2<f32>,
    click_pos_3d: Vec3<f32>,
    /// The initial ray intersection point on the drag plane at click time
    click_ray_intersection_3d: Option<Vec3<f32>>,
    click_selected: bool,
    drag_changed: bool,
    rectangle_undo_map: Map,
    rectangle_mode: bool,
    was_clicked: bool,

    hud: Hud,
}

impl Tool for LinedefTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Linedef Tool"),
            click_pos: Vec2::zero(),
            click_pos_3d: Vec3::zero(),
            click_ray_intersection_3d: None,
            click_selected: false,
            drag_changed: false,
            rectangle_undo_map: Map::default(),
            rectangle_mode: false,
            was_clicked: false,

            hud: Hud::new(HudMode::Linedef),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        fl!("tool_linedef")
    }
    fn icon_name(&self) -> String {
        str!("line-segment")
    }
    fn accel(&self) -> Option<char> {
        Some('L')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/linedef".to_string())
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
                if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                    layout.set_mode(TheSharedHLayoutMode::Right);
                    ctx.ui.relayout = true;
                }

                server_ctx.curr_map_tool_type = MapToolType::Linedef;

                if let Some(map) = project.get_map_mut(server_ctx) {
                    map.selected_vertices.clear();
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
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    if let Some(surface) = server_ctx.active_detail_surface.as_ref()
                        && let Some(profile_id) = surface.profile
                        && let Some(profile_map) = region.map.profiles.get_mut(&profile_id)
                    {
                        profile_map.clear_temp();
                    }
                    region.map.clear_temp();
                }
                server_ctx.active_detail_surface = None;
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

        fn vertex_is_in_rect_sector(map: &Map, vertex_id: u32) -> bool {
            for sector in &map.sectors {
                if !sector.properties.get_bool_default("rect", false) {
                    continue;
                }
                for &linedef_id in &sector.linedefs {
                    if let Some(linedef) = map.find_linedef(linedef_id)
                        && (linedef.start_vertex == vertex_id || linedef.end_vertex == vertex_id)
                    {
                        return true;
                    }
                }
            }
            false
        }

        fn validate_detail_profile(host_map: &Map, surface: &Surface, profile_map: &Map) -> bool {
            let Some(loop_uv) = surface.sector_loop_uv(host_map) else {
                return false;
            };
            if loop_uv.is_empty() {
                return false;
            }

            let mut min = loop_uv[0];
            let mut max = loop_uv[0];
            for uv in loop_uv.iter().skip(1) {
                min.x = min.x.min(uv.x);
                min.y = min.y.min(uv.y);
                max.x = max.x.max(uv.x);
                max.y = max.y.max(uv.y);
            }
            let eps = 0.001;

            for sector in &profile_map.sectors {
                if sector.linedefs.len() < 3 {
                    return false;
                }
                if sector.generate_geometry(profile_map).is_none()
                    || sector.area(profile_map) <= 0.0001
                {
                    return false;
                }
                for &linedef_id in &sector.linedefs {
                    let Some(linedef) = profile_map.find_linedef(linedef_id) else {
                        return false;
                    };
                    let Some(vertex) = profile_map.find_vertex(linedef.start_vertex) else {
                        return false;
                    };
                    if !vertex.x.is_finite() || !vertex.y.is_finite() {
                        return false;
                    }
                    let uv = Vec2::new(vertex.x, -vertex.y);
                    if uv.x < min.x - eps
                        || uv.x > max.x + eps
                        || uv.y < min.y - eps
                        || uv.y > max.y + eps
                    {
                        return false;
                    }
                }
            }
            true
        }

        fn validate_detail_profile_in_bounds(
            profile_map: &Map,
            min: Vec2<f32>,
            max: Vec2<f32>,
        ) -> bool {
            let eps = 0.001;
            for sector in &profile_map.sectors {
                if sector.linedefs.len() < 3 {
                    return false;
                }
                if sector.generate_geometry(profile_map).is_none()
                    || sector.area(profile_map) <= 0.0001
                {
                    return false;
                }
                for &linedef_id in &sector.linedefs {
                    let Some(linedef) = profile_map.find_linedef(linedef_id) else {
                        return false;
                    };
                    let Some(vertex) = profile_map.find_vertex(linedef.start_vertex) else {
                        return false;
                    };
                    if !vertex.x.is_finite() || !vertex.y.is_finite() {
                        return false;
                    }
                    let uv = Vec2::new(vertex.x, -vertex.y);
                    if uv.x < min.x - eps
                        || uv.x > max.x + eps
                        || uv.y < min.y - eps
                        || uv.y > max.y + eps
                    {
                        return false;
                    }
                }
            }
            true
        }

        fn point_segment_distance(point: Vec3<f32>, a: Vec3<f32>, b: Vec3<f32>) -> f32 {
            let ab = b - a;
            let ab_len_sq = ab.dot(ab);
            if ab_len_sq <= f32::EPSILON {
                return (point - a).magnitude();
            }
            let t = ((point - a).dot(ab) / ab_len_sq).clamp(0.0, 1.0);
            (point - (a + ab * t)).magnitude()
        }

        fn hovered_detail_surface(server_ctx: &ServerContext) -> Option<Surface> {
            server_ctx
                .hover_surface
                .clone()
                .or(server_ctx.editing_surface.clone())
        }

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

        let detail_mode_3d = server_ctx.editor_view_mode != EditorViewMode::D2
            && server_ctx.geometry_edit_mode == GeometryEditMode::Detail;

        match map_event {
            MapKey(c) => {
                match c {
                    '1'..='9' => map.subdivisions = (c as u8 - b'0') as f32,
                    '0' => map.subdivisions = 10.0,
                    _ => {}
                }
                RUSTERIX.write().unwrap().set_dirty();
            }
            MapClicked(coord) => {
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    self.was_clicked = false;
                    RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                self.was_clicked = true;

                // ---

                // If a vertex is under the cursor in D2, prioritize continuing line creation
                // over linedef selection. This allows extending existing chains by clicking endpoints.
                let mut over_vertex = false;
                if let Some(grid) = &server_ctx.hover_cursor {
                    if let Some(vertex_id) = map.find_vertex_at(grid.x, grid.y)
                        && !vertex_is_in_rect_sector(map, vertex_id)
                    {
                        over_vertex = true;
                    }
                }

                self.click_selected = false;
                let hovering_vertex_in_2d =
                    server_ctx.editor_view_mode == EditorViewMode::D2 && over_vertex;
                let mut detail_surface_retargeted = false;

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
                        old_profile_map.selected_linedefs.clear();
                        old_profile_map.clear_temp();
                    }
                    server_ctx.active_detail_surface = Some(surface);
                    server_ctx.hover.1 = None;
                    map.curr_grid_pos = None;
                    map.curr_grid_pos_3d = None;
                    map.clear_temp();
                    detail_surface_retargeted = true;
                }

                let clicked_detail_linedef = if detail_mode_3d {
                    server_ctx
                        .active_detail_surface
                        .as_ref()
                        .and_then(|surface| {
                            let hit_pos =
                                server_ctx.hover_cursor_3d.unwrap_or(server_ctx.geo_hit_pos);
                            surface.profile.and_then(|profile_id| {
                                map.profiles.get(&profile_id).and_then(|profile_map| {
                                    profile_map
                                        .linedefs
                                        .iter()
                                        .find(|linedef| {
                                            let Some(start_vertex) =
                                                profile_map.find_vertex(linedef.start_vertex)
                                            else {
                                                return false;
                                            };
                                            let Some(end_vertex) =
                                                profile_map.find_vertex(linedef.end_vertex)
                                            else {
                                                return false;
                                            };
                                            let a = surface.uv_to_world(Vec2::new(
                                                start_vertex.x,
                                                -start_vertex.y,
                                            ));
                                            let b = surface.uv_to_world(Vec2::new(
                                                end_vertex.x,
                                                -end_vertex.y,
                                            ));
                                            point_segment_distance(hit_pos, a, b) < 0.05
                                        })
                                        .map(|linedef| linedef.id)
                                })
                            })
                        })
                } else {
                    None
                };
                if detail_mode_3d {
                    server_ctx.hover.1 =
                        clicked_detail_linedef.or_else(|| match server_ctx.geo_hit {
                            Some(GeoId::Linedef(id)) => Some(id),
                            _ => None,
                        });
                }

                if map.curr_grid_pos.is_none()
                    && map.curr_grid_pos_3d.is_none()
                    && server_ctx.hover.1.is_some()
                    && !hovering_vertex_in_2d
                    && !detail_surface_retargeted
                {
                    map.selected_entity_item = None;
                    let mut changed = false;
                    let mut handled_detail_selection = false;

                    if detail_mode_3d {
                        if let Some(surface) = server_ctx.active_detail_surface.as_ref()
                            && let Some(profile_id) = surface.profile
                            && let Some(profile_map) = map.profiles.get_mut(&profile_id)
                        {
                            handled_detail_selection = true;
                            if ui.shift {
                                if let Some(l) = server_ctx.hover.1 {
                                    if !profile_map.selected_linedefs.contains(&l) {
                                        profile_map.selected_linedefs.push(l);
                                        changed = true;
                                    }
                                    self.click_selected = true;
                                }
                            } else if ui.alt {
                                if let Some(l) = server_ctx.hover.1 {
                                    profile_map
                                        .selected_linedefs
                                        .retain(|&selected| selected != l);
                                    changed = true;
                                }
                            } else {
                                if let Some(v) = server_ctx.hover.1 {
                                    profile_map.selected_linedefs = vec![v];
                                    changed = true;
                                } else {
                                    profile_map.selected_linedefs.clear();
                                    changed = true;
                                }
                                self.click_selected = true;
                            }
                        }
                    }

                    if !handled_detail_selection {
                        if ui.shift {
                            // Add
                            if let Some(l) = server_ctx.hover.1 {
                                if !map.selected_linedefs.contains(&l) {
                                    map.selected_linedefs.push(l);
                                    changed = true;
                                }
                                self.click_selected = true;
                            }
                        } else if ui.alt {
                            // Subtract
                            if let Some(l) = server_ctx.hover.1 {
                                map.selected_linedefs.retain(|&selected| selected != l);
                                changed = true;
                            }
                        } else {
                            // Replace
                            if let Some(v) = server_ctx.hover.1 {
                                map.selected_linedefs = vec![v];
                                changed = true;
                            } else {
                                map.selected_linedefs.clear();
                                changed = true;
                            }
                            self.click_selected = true;
                        }
                    }

                    if changed {
                        server_ctx.curr_action_id =
                            Some(Uuid::from_str(EDIT_LINEDEF_ACTION_ID).unwrap());
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                    }
                } else if !detail_surface_retargeted {
                    // Line mode
                    let mut set_current_gid_pos = true;
                    if let Some(render_view) = ui.get_render_view("PolyView") {
                        if server_ctx.editor_view_mode == EditorViewMode::D2 {
                            let dim = *render_view.dim();
                            let grid_pos = server_ctx.local_to_map_grid(
                                Vec2::new(dim.width as f32, dim.height as f32),
                                Vec2::new(coord.x as f32, coord.y as f32),
                                map,
                                map.subdivisions,
                            );

                            if let Some(curr_grid_pos) = map.curr_grid_pos {
                                if curr_grid_pos.x != grid_pos.x || curr_grid_pos.y != grid_pos.y {
                                    let prev = map.clone();

                                    let start_vertex =
                                        map.add_vertex_at(curr_grid_pos.x, curr_grid_pos.y);
                                    let end_vertex = map.add_vertex_at(grid_pos.x, grid_pos.y);

                                    let use_manual_mode = !ui.ctrl;

                                    if use_manual_mode {
                                        let _linedef_id =
                                            map.create_linedef_manual(start_vertex, end_vertex);

                                        if let Some(sector_id) = map.close_polygon_manual() {
                                            let mut surface = Surface::new(sector_id);
                                            surface.calculate_geometry(map);
                                            map.surfaces.insert(surface.id, surface);

                                            map.clear_temp();
                                            set_current_gid_pos = false;
                                        }
                                    } else {
                                        let ids = map.create_linedef(start_vertex, end_vertex);

                                        if let Some(sector_id) = ids.1 {
                                            let mut surface = Surface::new(sector_id);
                                            surface.calculate_geometry(map);
                                            map.surfaces.insert(surface.id, surface);

                                            map.clear_temp();
                                            set_current_gid_pos = false;
                                        }
                                    }

                                    undo_atom = Some(ProjectUndoAtom::MapEdit(
                                        server_ctx.pc,
                                        Box::new(prev),
                                        Box::new(map.clone()),
                                    ));
                                }
                            }

                            if set_current_gid_pos {
                                map.curr_grid_pos = Some(vek::Vec2::new(grid_pos.x, grid_pos.y));
                                map.curr_grid_pos_3d = None;
                            }
                        } else if let Some(hit_pos) = server_ctx.hover_cursor_3d {
                            let detail_mode =
                                server_ctx.geometry_edit_mode == GeometryEditMode::Detail;
                            if detail_mode
                                && server_ctx.active_detail_surface.is_none()
                                && let Some(surface) = hovered_detail_surface(server_ctx)
                            {
                                server_ctx.active_detail_surface = Some(surface);
                            }
                            let snapped_pos = server_ctx.snap_world_point_for_edit(map, hit_pos);

                            if let Some(curr_grid_pos) = map.curr_grid_pos_3d {
                                if curr_grid_pos != snapped_pos {
                                    let prev = map.clone();
                                    let use_manual_mode = !ui.ctrl;
                                    let mut handled_detail = false;

                                    if detail_mode
                                        && let Some(surface) =
                                            server_ctx.active_detail_surface.clone()
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
                                            && let Some(profile_map) =
                                                map.profiles.get_mut(&profile_id)
                                        {
                                            let start_uv_world = surface.world_to_uv(curr_grid_pos);
                                            let end_uv_world = surface.world_to_uv(snapped_pos);
                                            // Profile maps store editor-space Y inverted relative
                                            // to surface UV so the runtime flips it back consistently.
                                            let start_uv =
                                                Vec2::new(start_uv_world.x, -start_uv_world.y);
                                            let end_uv = Vec2::new(end_uv_world.x, -end_uv_world.y);

                                            let start_vertex =
                                                profile_map.add_vertex_at(start_uv.x, start_uv.y);
                                            let end_vertex =
                                                profile_map.add_vertex_at(end_uv.x, end_uv.y);

                                            if use_manual_mode {
                                                let _linedef_id = profile_map
                                                    .create_linedef_manual(
                                                        start_vertex,
                                                        end_vertex,
                                                    );

                                                if profile_map.close_polygon_manual().is_some() {
                                                    profile_map.clear_temp();
                                                    map.clear_temp();
                                                    server_ctx.active_detail_surface = None;
                                                    set_current_gid_pos = false;
                                                }
                                            } else {
                                                let ids = profile_map
                                                    .create_linedef(start_vertex, end_vertex);

                                                if ids.1.is_some() {
                                                    profile_map.clear_temp();
                                                    map.clear_temp();
                                                    server_ctx.active_detail_surface = None;
                                                    set_current_gid_pos = false;
                                                }
                                            }

                                            handled_detail = true;
                                        }
                                    }

                                    if !handled_detail {
                                        let start_vertex = map.add_vertex_at_3d(
                                            curr_grid_pos.x,
                                            curr_grid_pos.z,
                                            curr_grid_pos.y,
                                            true,
                                        );
                                        let end_vertex = map.add_vertex_at_3d(
                                            snapped_pos.x,
                                            snapped_pos.z,
                                            snapped_pos.y,
                                            true,
                                        );

                                        if use_manual_mode {
                                            let _linedef_id =
                                                map.create_linedef_manual(start_vertex, end_vertex);

                                            if let Some(sector_id) = map.close_polygon_manual() {
                                                let mut surface = Surface::new(sector_id);
                                                surface.calculate_geometry(map);
                                                map.surfaces.insert(surface.id, surface);

                                                map.clear_temp();
                                                server_ctx.active_detail_surface = None;
                                                set_current_gid_pos = false;
                                            }
                                        } else {
                                            let ids = map.create_linedef(start_vertex, end_vertex);

                                            if let Some(sector_id) = ids.1 {
                                                let mut surface = Surface::new(sector_id);
                                                surface.calculate_geometry(map);
                                                map.surfaces.insert(surface.id, surface);

                                                map.clear_temp();
                                                server_ctx.active_detail_surface = None;
                                                set_current_gid_pos = false;
                                            }
                                        }
                                    }

                                    undo_atom = Some(ProjectUndoAtom::MapEdit(
                                        server_ctx.pc,
                                        Box::new(prev),
                                        Box::new(map.clone()),
                                    ));
                                }
                            }

                            if set_current_gid_pos {
                                map.curr_grid_pos = None;
                                map.curr_grid_pos_3d = Some(snapped_pos);
                            }
                        }
                    }
                }

                self.click_pos = Vec2::new(coord.x as f32, coord.y as f32);
                self.click_ray_intersection_3d = None;

                // For 3D dragging, use the average position of selected linedef vertices
                let mut detail_selection_prepared = false;
                if detail_mode_3d
                    && self.click_selected
                    && let Some(surface) = server_ctx.active_detail_surface.as_ref()
                    && let Some(profile_id) = surface.profile
                    && let Some(profile_map) = map.profiles.get(&profile_id)
                    && !profile_map.selected_linedefs.is_empty()
                {
                    let mut sum_pos = Vec3::zero();
                    let mut count = 0;
                    for line_id in &profile_map.selected_linedefs {
                        if let Some(line) = profile_map.find_linedef(*line_id) {
                            if let Some(v1) = profile_map.find_vertex(line.start_vertex) {
                                sum_pos += surface.uv_to_world(Vec2::new(v1.x, -v1.y));
                                count += 1;
                            }
                            if let Some(v2) = profile_map.find_vertex(line.end_vertex) {
                                sum_pos += surface.uv_to_world(Vec2::new(v2.x, -v2.y));
                                count += 1;
                            }
                        }
                    }
                    if count > 0 {
                        self.click_pos_3d = sum_pos / count as f32;
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

                    detail_selection_prepared = true;
                }

                if !detail_selection_prepared
                    && self.click_selected
                    && !map.selected_linedefs.is_empty()
                {
                    let mut sum_pos = Vec3::zero();
                    let mut count = 0;
                    for line_id in &map.selected_linedefs {
                        if let Some(line) = map.find_linedef(*line_id) {
                            if let Some(v1) = map.find_vertex(line.start_vertex) {
                                sum_pos += v1.as_vec3_world();
                                count += 1;
                            }
                            if let Some(v2) = map.find_vertex(line.end_vertex) {
                                sum_pos += v2.as_vec3_world();
                                count += 1;
                            }
                        }
                    }
                    if count > 0 {
                        self.click_pos_3d = sum_pos / count as f32;
                    } else {
                        self.click_pos_3d = server_ctx.geo_hit_pos;
                    }

                    // Compute initial ray intersection on the drag plane at click time
                    // This ensures dragging is relative to this point, not the vertex average
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
                self.rectangle_mode = false;
            }
            MapDragged(coord) => {
                if self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if self.click_selected {
                    // Dragging selected lines
                    if let Some(render_view) = ui.get_render_view("PolyView") {
                        let dim = *render_view.dim();

                        if server_ctx.editor_view_mode == EditorViewMode::D2 {
                            // 2D dragging
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

                            let mut selected_vertices = vec![];

                            let drag_delta = click_pos - drag_pos;
                            for line_id in self.rectangle_undo_map.selected_linedefs.iter() {
                                if let Some(line) = self.rectangle_undo_map.find_linedef(*line_id) {
                                    selected_vertices.push(line.start_vertex);
                                    selected_vertices.push(line.end_vertex);
                                }
                            }

                            for vertex_id in selected_vertices.iter() {
                                if let Some(original_vertex) =
                                    self.rectangle_undo_map.find_vertex_mut(*vertex_id)
                                {
                                    let new_pos = Vec2::new(
                                        original_vertex.x - drag_delta.x,
                                        original_vertex.y - drag_delta.y,
                                    );
                                    map.update_vertex(*vertex_id, new_pos);
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
                            let uv_bounds = surface.sector_loop_uv(map).and_then(|loop_uv| {
                                if loop_uv.is_empty() {
                                    None
                                } else {
                                    let mut min = loop_uv[0];
                                    let mut max = loop_uv[0];
                                    for uv in loop_uv.iter().skip(1) {
                                        min.x = min.x.min(uv.x);
                                        min.y = min.y.min(uv.y);
                                        max.x = max.x.max(uv.x);
                                        max.y = max.y.max(uv.y);
                                    }
                                    Some((min, max))
                                }
                            });
                            if denom.abs() > 0.0001 {
                                let t = (self.click_pos_3d - ray.origin).dot(plane_normal) / denom;
                                if t >= 0.0
                                    && let Some(profile_map) = map.profiles.get_mut(&profile_id)
                                {
                                    let current_pos = ray.origin + ray.dir * t;
                                    let start_uv = surface.world_to_uv(click_intersection);
                                    let current_uv = surface.world_to_uv(current_pos);
                                    let drag_delta_uv = current_uv - start_uv;
                                    let step = 1.0 / map.subdivisions.max(1.0);

                                    let mut selected_vertices = vec![];
                                    for line_id in self.rectangle_undo_map.selected_linedefs.iter()
                                    {
                                        if let Some(line) =
                                            self.rectangle_undo_map.find_linedef(*line_id)
                                        {
                                            if !selected_vertices.contains(&line.start_vertex) {
                                                selected_vertices.push(line.start_vertex);
                                            }
                                            if !selected_vertices.contains(&line.end_vertex) {
                                                selected_vertices.push(line.end_vertex);
                                            }
                                        }
                                    }

                                    for vertex_id in selected_vertices.iter() {
                                        if let Some(original_vertex) =
                                            self.rectangle_undo_map.find_vertex(*vertex_id)
                                            && let Some(vertex) =
                                                profile_map.find_vertex_mut(*vertex_id)
                                        {
                                            let mut snapped_x =
                                                ((original_vertex.x + drag_delta_uv.x) / step)
                                                    .round()
                                                    * step;
                                            let mut snapped_y =
                                                ((-original_vertex.y + drag_delta_uv.y) / step)
                                                    .round()
                                                    * step;
                                            if let Some((min, max)) = uv_bounds {
                                                snapped_x = snapped_x.clamp(min.x, max.x);
                                                snapped_y = snapped_y.clamp(min.y, max.y);
                                            }
                                            vertex.x = snapped_x;
                                            vertex.y = -snapped_y;
                                        }
                                    }

                                    if let Some((min, max)) = uv_bounds
                                        && !validate_detail_profile_in_bounds(profile_map, min, max)
                                    {
                                        *profile_map = self.rectangle_undo_map.clone();
                                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                        return None;
                                    }

                                    if drag_delta_uv.x != 0.0 || drag_delta_uv.y != 0.0 {
                                        self.drag_changed = true;
                                    }
                                }
                            }
                        } else {
                            // 3D dragging
                            // Only start dragging after a minimum distance threshold
                            let drag_distance = self
                                .click_pos
                                .distance(Vec2::new(coord.x as f32, coord.y as f32));
                            if drag_distance < 5.0 {
                                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                return None;
                            }

                            // Use the initial ray intersection as reference (not vertex average)
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

                            let plane_normal = match plane {
                                GizmoMode::XZ => Vec3::new(0.0, 1.0, 0.0),
                                GizmoMode::XY => Vec3::new(0.0, 0.0, 1.0),
                                GizmoMode::YZ => Vec3::new(1.0, 0.0, 0.0),
                            };

                            let denom: f32 = plane_normal.dot(ray.dir);

                            if denom.abs() > 0.0001 {
                                let t = (start_pos - ray.origin).dot(plane_normal) / denom;
                                if t >= 0.0 {
                                    let current_pos = ray.origin + ray.dir * t;

                                    // Calculate drag delta relative to initial click intersection
                                    // (not the vertex average position)
                                    let drag_delta = match plane {
                                        GizmoMode::XZ => Vec3::new(
                                            current_pos.x - click_intersection.x,
                                            0.0,
                                            current_pos.z - click_intersection.z,
                                        ),
                                        GizmoMode::XY => Vec3::new(
                                            current_pos.x - click_intersection.x,
                                            current_pos.y - click_intersection.y,
                                            0.0,
                                        ),
                                        GizmoMode::YZ => Vec3::new(
                                            0.0,
                                            current_pos.y - click_intersection.y,
                                            current_pos.z - click_intersection.z,
                                        ),
                                    };

                                    let mut selected_vertices = vec![];
                                    for line_id in self.rectangle_undo_map.selected_linedefs.iter()
                                    {
                                        if let Some(line) =
                                            self.rectangle_undo_map.find_linedef(*line_id)
                                        {
                                            if !selected_vertices.contains(&line.start_vertex) {
                                                selected_vertices.push(line.start_vertex);
                                            }
                                            if !selected_vertices.contains(&line.end_vertex) {
                                                selected_vertices.push(line.end_vertex);
                                            }
                                        }
                                    }

                                    for vertex_id in selected_vertices.iter() {
                                        if let Some(original_vertex) =
                                            self.rectangle_undo_map.find_vertex(*vertex_id)
                                        {
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

                                            if let Some(vertex) = map.find_vertex_mut(*vertex_id) {
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
                } else {
                    if detail_mode_3d {
                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                        return None;
                    }

                    if !self.rectangle_mode && self.was_clicked {
                        let dist = self
                            .click_pos
                            .distance(Vec2::new(coord.x as f32, coord.y as f32));
                        if dist > 10.0 {
                            self.rectangle_mode = true;
                            map.clear_temp();
                        }
                    }

                    if self.rectangle_mode {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
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
                                let top_left = Vec2::new(
                                    click_pos.x.min(drag_pos.x),
                                    click_pos.y.min(drag_pos.y),
                                );
                                let bottom_right = Vec2::new(
                                    click_pos.x.max(drag_pos.x),
                                    click_pos.y.max(drag_pos.y),
                                );

                                let mut selection =
                                    server_ctx.geometry_in_rectangle(top_left, bottom_right, map);

                                selection.0 = vec![];
                                selection.2 = vec![];

                                selection
                            } else {
                                let mut selection = (vec![], vec![], vec![]);

                                let click_pos = self.click_pos;
                                let drag_pos = Vec2::new(coord.x as f32, coord.y as f32);

                                let top_left = Vec2::new(
                                    click_pos.x.min(drag_pos.x),
                                    click_pos.y.min(drag_pos.y),
                                );
                                let bottom_right = Vec2::new(
                                    click_pos.x.max(drag_pos.x),
                                    click_pos.y.max(drag_pos.y),
                                );

                                let mut rusterix = RUSTERIX.write().unwrap();
                                rusterix.scene_handler.vm.set_active_vm(2);
                                let linedefs =
                                    rusterix.scene_handler.vm.active_vm().pick_geo_ids_in_rect(
                                        dim.width as u32,
                                        dim.height as u32,
                                        top_left,
                                        bottom_right,
                                        GeoId::Linedef(0),
                                        false,
                                        false,
                                    );
                                for l in linedefs {
                                    if let GeoId::Linedef(l) = l {
                                        selection.1.push(l);
                                    }
                                }
                                rusterix.scene_handler.vm.set_active_vm(0);
                                selection
                            };

                            *map = self.rectangle_undo_map.clone();
                            map.curr_grid_pos = None;
                            map.curr_rectangle = Some((click_pos, drag_pos));

                            if ui.shift {
                                // Add
                                map.add_to_selection(selection.0, selection.1, selection.2);
                            } else if ui.alt {
                                // Remove
                                map.remove_from_selection(selection.0, selection.1, selection.2);
                            } else {
                                // Replace
                                map.selected_linedefs = selection.1;
                            }
                        }
                    }
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            MapUp(_) => {
                if self.click_selected {
                    if self.drag_changed {
                        if detail_mode_3d
                            && let Some(surface) = server_ctx.active_detail_surface.clone()
                            && let Some(profile_id) = surface.profile
                        {
                            let is_valid = map
                                .profiles
                                .get(&profile_id)
                                .map(|profile_map| {
                                    validate_detail_profile(map, &surface, profile_map)
                                })
                                .unwrap_or(false);
                            if let Some(profile_map) = map.profiles.get_mut(&profile_id) {
                                if is_valid {
                                    profile_map.sanitize();
                                } else {
                                    *profile_map = self.rectangle_undo_map.clone();
                                }
                            }
                        }
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
                } else if self.rectangle_mode && map.curr_rectangle.is_some() {
                    map.clear_temp();
                    self.rectangle_mode = false;
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

                if let Some(render_view) = ui.get_render_view("PolyView") {
                    if server_ctx.editor_view_mode == EditorViewMode::D2 {
                        let dim = *render_view.dim();
                        if !self.rectangle_mode {
                            //map.curr_mouse_pos = Some(Vec2::new(coord.x as f32, coord.y as f32));
                            map.curr_mouse_pos = Some(server_ctx.local_to_map_grid(
                                Vec2::new(dim.width as f32, dim.height as f32),
                                Vec2::new(coord.x as f32, coord.y as f32),
                                map,
                                map.subdivisions,
                            ));
                        }
                        let mut hover = server_ctx.geometry_at(
                            Vec2::new(dim.width as f32, dim.height as f32),
                            Vec2::new(coord.x as f32, coord.y as f32),
                            map,
                        );

                        hover.0 = None;
                        hover.2 = None;

                        server_ctx.hover = hover;
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
                    } else {
                        if detail_mode_3d {
                            let selected_count = server_ctx
                                .active_detail_surface
                                .as_ref()
                                .and_then(|surface| surface.profile)
                                .and_then(|profile_id| map.profiles.get(&profile_id))
                                .map(|profile_map| profile_map.selected_linedefs.len())
                                .unwrap_or(0);
                            if map.curr_grid_pos_3d.is_none()
                                && selected_count == 0
                                && let Some(surface) = hovered_detail_surface(server_ctx)
                            {
                                server_ctx.active_detail_surface = Some(surface);
                            }
                        }

                        if let Some(geo_id) = server_ctx.geo_hit {
                            match geo_id {
                                GeoId::Linedef(id) => {
                                    server_ctx.hover = (None, Some(id), None);
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

                    if let Some(l) = server_ctx.hover.1 {
                        if detail_mode_3d
                            && let Some(surface) = server_ctx.active_detail_surface.as_ref()
                            && let Some(profile_id) = surface.profile
                            && let Some(profile_map) = map.profiles.get(&profile_id)
                            && let Some(linedef) = profile_map.find_linedef(l)
                        {
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                format!(
                                    "Detail Linedef {}: V{} - V{}",
                                    l, linedef.start_vertex, linedef.end_vertex
                                ),
                            ));
                        } else if let Some(linedef) = map.find_linedef(l) {
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                format!(
                                    "Linedef {}: V{} - V{}",
                                    l, linedef.start_vertex, linedef.end_vertex
                                ),
                            ));
                        }
                    } else {
                        ctx.ui
                            .send(TheEvent::SetStatusText(TheId::empty(), "".into()));
                    }
                }
            }
            MapDelete => {
                if detail_mode_3d
                    && let Some(surface) = server_ctx.active_detail_surface.as_ref()
                    && let Some(profile_id) = surface.profile
                {
                    let prev = map.clone();
                    if let Some(profile_map) = map.profiles.get_mut(&profile_id)
                        && !profile_map.selected_linedefs.is_empty()
                    {
                        let lines = profile_map.selected_linedefs.clone();
                        profile_map.delete_elements(&[], &lines, &[]);
                        profile_map.selected_linedefs.clear();

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
                } else if !map.selected_linedefs.is_empty() {
                    let prev = map.clone();
                    let lines = map.selected_linedefs.clone();

                    #[allow(clippy::useless_vec)]
                    map.delete_elements(&vec![], &lines, &vec![]);
                    map.selected_linedefs.clear();

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
                map.clear_temp();
                if let Some(surface) = server_ctx.active_detail_surface.as_ref()
                    && let Some(profile_id) = surface.profile
                    && let Some(profile_map) = map.profiles.get_mut(&profile_id)
                {
                    profile_map.clear_temp();
                    profile_map.selected_linedefs.clear();
                }
                server_ctx.active_detail_surface = None;
                if !map.selected_linedefs.is_empty() {
                    map.selected_linedefs.clear();

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
                self.was_clicked = false;
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
        }
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
        let detail_mode_3d = server_ctx.editor_view_mode != EditorViewMode::D2
            && server_ctx.geometry_edit_mode == GeometryEditMode::Detail;
        let id = if detail_mode_3d
            && let Some(surface) = server_ctx.active_detail_surface.as_ref()
            && let Some(profile_id) = surface.profile
            && let Some(profile_map) = map.profiles.get(&profile_id)
            && !profile_map.selected_linedefs.is_empty()
        {
            Some(profile_map.selected_linedefs[0])
        } else if !map.selected_linedefs.is_empty() {
            Some(map.selected_linedefs[0])
        } else {
            None
        };
        self.hud.draw(buffer, map, ctx, server_ctx, id, assets);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::StateChanged(id, state) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "Apply Map Properties" && *state == TheWidgetState::Clicked {
                    // Apply a source
                    let mut source: Option<Value> = None;
                    if let Some(id) = server_ctx.curr_tile_id {
                        source = Some(Value::Source(PixelSource::TileId(id)));
                    }

                    /*else if server_ctx.curr_map_tool_helper == MapToolHelper::MaterialPicker {
                        if let Some(id) = server_ctx.curr_material_id {
                            source = Some(Value::Source(PixelSource::MaterialId(id)));
                        }
                    } */
                    if let Some(source) = source {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let _prev = map.clone();
                            for linedef_id in map.selected_linedefs.clone() {
                                if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                    if self.hud.selected_icon_index == 0 {
                                        linedef.properties.set("row1_source", source.clone());
                                    } else if self.hud.selected_icon_index == 1 {
                                        linedef.properties.set("row2_source", source.clone());
                                    } else if self.hud.selected_icon_index == 2 {
                                        linedef.properties.set("row3_source", source.clone());
                                    } else if self.hud.selected_icon_index == 3 {
                                        linedef.properties.set("row4_source", source.clone());
                                    }
                                }
                            }

                            if server_ctx.get_map_context() == MapContext::Region {
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Render SceneManager Map"),
                                    TheValue::Empty,
                                ));
                            }

                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                        }
                    }
                } else if id.name == "Remove Map Properties" && *state == TheWidgetState::Clicked {
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        let _prev = map.clone();
                        for linedef_id in map.selected_linedefs.clone() {
                            if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                if self.hud.selected_icon_index == 0 {
                                    if linedef.properties.contains("row1_light") {
                                        linedef.properties.remove("row1_light");
                                    } else {
                                        linedef
                                            .properties
                                            .set("row1_source", Value::Source(PixelSource::Off));
                                    }
                                } else if self.hud.selected_icon_index == 1 {
                                    if linedef.properties.contains("row2_light") {
                                        linedef.properties.remove("row2_light");
                                    } else {
                                        linedef
                                            .properties
                                            .set("row2_source", Value::Source(PixelSource::Off));
                                    }
                                } else if self.hud.selected_icon_index == 2 {
                                    if linedef.properties.contains("row3_light") {
                                        linedef.properties.remove("row3_light");
                                    } else {
                                        linedef
                                            .properties
                                            .set("row3_source", Value::Source(PixelSource::Off));
                                    }
                                } else if self.hud.selected_icon_index == 3 {
                                    if linedef.properties.contains("row4_light") {
                                        linedef.properties.remove("row4_light");
                                    } else {
                                        linedef
                                            .properties
                                            .set("row4_source", Value::Source(PixelSource::Off));
                                    }
                                }
                            }
                        }

                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));

                        if server_ctx.get_map_context() == MapContext::Region {
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Render SceneManager Map"),
                                TheValue::Empty,
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
