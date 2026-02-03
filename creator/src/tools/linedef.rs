use crate::actions::edit_linedef::EDIT_LINEDEF_ACTION_ID;
use crate::editor::{NODEEDITOR, RUSTERIX};
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

                self.activate_map_tool_helper(ui, ctx, project, server_ctx);

                return true;
            }
            DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.clear_temp();
                }
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

                // Audo Mode: Test if a vertex is under the cursor, in that case in D2 we dont select but do add lines
                let mut over_vertex = false;
                if let Some(grid) = &server_ctx.hover_cursor
                    && ui.ctrl
                {
                    if map.find_vertex_at(grid.x, grid.y).is_some() {
                        over_vertex = true;
                    }
                }

                self.click_selected = false;
                let hovering_vertex_in_2d =
                    server_ctx.editor_view_mode == EditorViewMode::D2 && over_vertex;

                if map.curr_grid_pos.is_none()
                    && server_ctx.hover.1.is_some()
                    && !hovering_vertex_in_2d
                {
                    map.selected_entity_item = None;
                    let mut changed = false;

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

                    if changed {
                        server_ctx.curr_action_id =
                            Some(Uuid::from_str(EDIT_LINEDEF_ACTION_ID).unwrap());
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                    }
                } else if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    // Line mode
                    let mut set_current_gid_pos = true;
                    if let Some(render_view) = ui.get_render_view("PolyView") {
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

                                // Choose between manual and auto polygon creation modes
                                // Manual mode (default): Only creates sectors when you close the polygon
                                // Auto mode (Ctrl/Cmd): Automatically detects and creates sectors when a loop is closed
                                let use_manual_mode = !ui.ctrl; // Ctrl/Cmd enables auto-detection

                                if use_manual_mode {
                                    // MANUAL MODE: Only creates sectors when you manually close the polygon
                                    // Good for drawing in existing grids where auto-detection would trigger too early
                                    let _linedef_id =
                                        map.create_linedef_manual(start_vertex, end_vertex);

                                    // Check if the user manually closed the polygon (clicked back to start)
                                    if let Some(sector_id) = map.close_polygon_manual() {
                                        // When we close a polygon add a surface
                                        let mut surface = Surface::new(sector_id);
                                        surface.calculate_geometry(map);
                                        map.surfaces.insert(surface.id, surface);

                                        // and delete the temporary data
                                        map.clear_temp();
                                        set_current_gid_pos = false;
                                    }
                                } else {
                                    // AUTO MODE: Automatically detects and creates sectors when a loop is closed
                                    // Good for quick polygon creation in empty areas
                                    let ids = map.create_linedef(start_vertex, end_vertex);

                                    if let Some(sector_id) = ids.1 {
                                        // When we close a polygon add a surface
                                        let mut surface = Surface::new(sector_id);
                                        surface.calculate_geometry(map);
                                        map.surfaces.insert(surface.id, surface);

                                        // and delete the temporary data
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
                        }
                    }
                }

                self.click_pos = Vec2::new(coord.x as f32, coord.y as f32);
                self.click_ray_intersection_3d = None;

                // For 3D dragging, use the average position of selected linedef vertices
                if self.click_selected && !map.selected_linedefs.is_empty() {
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

                self.rectangle_undo_map = map.clone();
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
                                        true,
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
                        if let Some(linedef) = map.find_linedef(l) {
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
                if !map.selected_linedefs.is_empty() {
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
                if !map.selected_linedefs.is_empty() {
                    map.selected_linedefs.clear();

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
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
        let id = if !map.selected_linedefs.is_empty() {
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
                    if server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker {
                        if let Some(id) = server_ctx.curr_tile_id {
                            source = Some(Value::Source(PixelSource::TileId(id)));
                        }
                    }
                    /*else if server_ctx.curr_map_tool_helper == MapToolHelper::MaterialPicker {
                        if let Some(id) = server_ctx.curr_material_id {
                            source = Some(Value::Source(PixelSource::MaterialId(id)));
                        }
                    } */
                    else if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor {
                        let node_editor = NODEEDITOR.read().unwrap();
                        if !node_editor.graph.nodes.is_empty() {
                            source = Some(Value::Source(PixelSource::ShapeFXGraphId(
                                node_editor.graph.id,
                            )));
                        }
                    }

                    if let Some(source) = source {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let prev = map.clone();
                            let context = NODEEDITOR.read().unwrap().context;
                            for linedef_id in map.selected_linedefs.clone() {
                                if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                    if context == NodeContext::Region
                                        && server_ctx.curr_map_tool_helper
                                            == MapToolHelper::NodeEditor
                                    {
                                        linedef.properties.set("region_graph", source.clone());
                                    } else if context == NodeContext::Shape {
                                        linedef.properties.set("shape_graph", source.clone());
                                    } else if self.hud.selected_icon_index == 0 {
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

                            // Force node update
                            if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor {
                                NODEEDITOR.read().unwrap().force_update(ctx, map);
                            }

                            let undo_atom =
                                RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));

                            crate::editor::UNDOMANAGER.write().unwrap().add_region_undo(
                                &server_ctx.curr_region,
                                undo_atom,
                                ctx,
                            );

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
                        let prev = map.clone();
                        let context = NODEEDITOR.read().unwrap().context;
                        for linedef_id in map.selected_linedefs.clone() {
                            if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                if context == NodeContext::Region
                                    && server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor
                                {
                                    linedef.properties.remove("region_graph");
                                }
                                if context == NodeContext::Shape
                                    && server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor
                                {
                                    linedef.properties.remove("shape_graph");
                                } else if self.hud.selected_icon_index == 0 {
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

                        // Force node update
                        if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor {
                            NODEEDITOR.read().unwrap().force_update(ctx, map);
                        }

                        let undo_atom =
                            RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));

                        crate::editor::UNDOMANAGER.write().unwrap().add_region_undo(
                            &server_ctx.curr_region,
                            undo_atom,
                            ctx,
                        );
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
