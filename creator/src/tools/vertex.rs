use crate::actions::edit_vertex::EDIT_VERTEX_ACTION_ID;
use crate::editor::{NODEEDITOR, RUSTERIX};
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::Assets;
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
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                self.activate_map_tool_helper(ui, ctx, project, server_ctx);
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

                self.click_selected = false;
                if server_ctx.hover.0.is_some() {
                    let mut changed = false;

                    map.selected_entity_item = None;

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

                                let id = map.add_vertex_at_3d(pt.x, pt.z, pt.y, false);
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
                            }
                        }
                    }
                }

                self.click_pos = Vec2::new(coord.x as f32, coord.y as f32);
                self.click_ray_intersection_3d = None;

                // For 3D dragging, use the actual vertex position if one is selected
                if self.click_selected && !map.selected_vertices.is_empty() {
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

                self.rectangle_undo_map = map.clone();
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
                                    map.update_vertex(*vertex_id, new_pos);
                                }
                            }
                            server_ctx.hover_cursor = Some(drag_pos);

                            if drag_delta.x != 0.0 || drag_delta.y != 0.0 {
                                self.drag_changed = true;
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
                            true,
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
                if !map.selected_vertices.is_empty() {
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
                if !map.selected_vertices.is_empty() {
                    map.selected_vertices.clear();

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
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
        let id = if !map.selected_vertices.is_empty() {
            Some(map.selected_vertices[0])
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
                    if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor {
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
                            for vertex_id in map.selected_vertices.clone() {
                                if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                                    if context == NodeContext::Region {
                                        vertex.properties.set("region_graph", source.clone());
                                    } else if context == NodeContext::Shape {
                                        vertex.properties.set("shape_graph", source.clone());
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
                        }
                    }
                } else if id.name == "Remove Map Properties" && *state == TheWidgetState::Clicked {
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        let prev = map.clone();
                        let context = NODEEDITOR.read().unwrap().context;
                        for vertex_id in map.selected_vertices.clone() {
                            if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                                if context == NodeContext::Region
                                    && server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor
                                {
                                    vertex.properties.remove("region_graph");
                                } else if context == NodeContext::Shape
                                    && server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor
                                {
                                    vertex.properties.remove("shape_graph");
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
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
