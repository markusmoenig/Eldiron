use crate::actions::edit_sector::EDIT_SECTOR_ACTION_ID;
use crate::editor::{NODEEDITOR, RUSTERIX};
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::{Assets, PixelSource, Value};
use scenevm::GeoId;
use std::str::FromStr;
use vek::Vec2;

pub struct SectorTool {
    id: TheId,
    click_pos: Vec2<f32>,
    click_pos_3d: Vec3<f32>,
    /// The initial ray intersection point on the drag plane at click time
    click_ray_intersection_3d: Option<Vec3<f32>>,
    rectangle_undo_map: Map,
    click_selected: bool,
    drag_changed: bool,
    was_clicked: bool,
    vertices_duplicated: bool,
    cached_sectors_to_move: Vec<u32>,

    hud: Hud,
}

impl Tool for SectorTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Sector Tool"),
            click_pos: Vec2::zero(),
            click_pos_3d: Vec3::zero(),
            click_ray_intersection_3d: None,
            click_selected: false,
            drag_changed: false,
            rectangle_undo_map: Map::default(),
            was_clicked: false,
            vertices_duplicated: false,
            cached_sectors_to_move: vec![],

            hud: Hud::new(HudMode::Sector),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        fl!("tool_sector")
    }
    fn icon_name(&self) -> String {
        str!("polygon")
    }
    fn accel(&self) -> Option<char> {
        Some('E')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/sector".to_string())
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
                // Display the tile edit panel.
                ctx.ui
                    .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));

                if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                    layout.set_mode(TheSharedHLayoutMode::Right);
                    ctx.ui.relayout = true;
                }

                server_ctx.curr_map_tool_type = MapToolType::Sector;

                if let Some(map) = project.get_map_mut(server_ctx) {
                    map.selected_vertices.clear();
                    map.selected_linedefs.clear();
                }

                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));

                self.activate_map_tool_helper(ui, ctx, project, server_ctx);

                return true;
            }
            _ => {
                server_ctx.curr_map_tool_type = MapToolType::General;
            }
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
                let mut changed = false;

                self.click_selected = false;
                if server_ctx.hover.2.is_some() {
                    map.selected_entity_item = None;

                    if ui.shift {
                        // Add
                        if let Some(s) = server_ctx.hover.2 {
                            if !map.selected_sectors.contains(&s) {
                                map.selected_sectors.push(s);
                                changed = true;
                            }
                            self.click_selected = true;
                        }
                    } else if ui.alt {
                        // Subtract
                        if let Some(v) = server_ctx.hover.2 {
                            map.selected_sectors.retain(|&selected| selected != v);
                            changed = true;
                        }
                    } else {
                        // Replace
                        if let Some(v) = server_ctx.hover.2 {
                            map.selected_sectors = vec![v];
                            changed = true;
                        } else {
                            map.selected_sectors.clear();
                            changed = true;
                        }
                        self.click_selected = true;
                    }

                    if changed {
                        server_ctx.curr_action_id =
                            Some(Uuid::from_str(EDIT_SECTOR_ACTION_ID).unwrap());
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                    }
                }

                self.click_pos = Vec2::new(coord.x as f32, coord.y as f32);
                self.click_ray_intersection_3d = None;

                // For 3D dragging, use the average position of selected sector vertices
                if self.click_selected && !map.selected_sectors.is_empty() {
                    let mut sum_pos = Vec3::zero();
                    let mut count = 0;
                    for sector_id in &map.selected_sectors {
                        if let Some(sector) = map.find_sector(*sector_id) {
                            for line_id in &sector.linedefs {
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
                        }
                    }
                    if count > 0 {
                        self.click_pos_3d = sum_pos / count as f32;
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
                self.vertices_duplicated = false;
                self.cached_sectors_to_move.clear();
            }
            MapDragged(coord) => {
                if self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if self.click_selected {
                    // Dragging selected sectors
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

                            // Collect sectors to move (selected + optionally embedded)
                            // Only compute this once and cache it
                            if self.cached_sectors_to_move.is_empty() {
                                self.cached_sectors_to_move =
                                    self.rectangle_undo_map.selected_sectors.clone();

                                // If Ctrl is pressed, include all embedded sectors
                                if ui.ctrl {
                                    let mut embedded_sectors = vec![];
                                    for sector_id in &self.rectangle_undo_map.selected_sectors {
                                        let embedded = map.find_embedded_sectors(*sector_id);
                                        for emb_id in embedded {
                                            if !self.cached_sectors_to_move.contains(&emb_id) {
                                                embedded_sectors.push(emb_id);
                                            }
                                        }
                                    }
                                    self.cached_sectors_to_move.extend(embedded_sectors);
                                }
                            }

                            let sectors_to_move = &self.cached_sectors_to_move;

                            for sector_id in sectors_to_move.iter() {
                                if let Some(sector) =
                                    self.rectangle_undo_map.find_sector(*sector_id)
                                {
                                    for line_id in &sector.linedefs {
                                        if let Some(line) =
                                            self.rectangle_undo_map.find_linedef(*line_id)
                                        {
                                            selected_vertices.push(line.start_vertex);
                                            selected_vertices.push(line.end_vertex);
                                        }
                                    }
                                }
                            }

                            // Duplicate shared vertices only once at the start of dragging
                            if !self.vertices_duplicated {
                                for vertex_id in selected_vertices.iter() {
                                    // Check if this vertex is shared with any unselected rect sector
                                    let is_unselected_rect_vertex =
                                        map.sectors.iter().any(|sector| {
                                            if sector.properties.contains("rect")
                                                && !sectors_to_move.contains(&sector.id)
                                            {
                                                sector.linedefs.iter().any(|&line_id| {
                                                    if let Some(line) = map.find_linedef(line_id) {
                                                        line.start_vertex == *vertex_id
                                                            || line.end_vertex == *vertex_id
                                                    } else {
                                                        false
                                                    }
                                                })
                                            } else {
                                                false
                                            }
                                        });

                                    if is_unselected_rect_vertex {
                                        // Vertex is shared with rect geometry - duplicate it for the sectors being moved
                                        if let Some(new_vertex_id) =
                                            map.duplicate_vertex(*vertex_id)
                                        {
                                            // Replace old vertex with new vertex in all sectors being moved
                                            for sector_id in sectors_to_move.iter() {
                                                map.replace_vertex_in_sector(
                                                    *sector_id,
                                                    *vertex_id,
                                                    new_vertex_id,
                                                );
                                            }
                                        }
                                    }
                                }
                                self.vertices_duplicated = true;
                                // Update rectangle_undo_map after duplication so future drags use correct vertex IDs
                                self.rectangle_undo_map = map.clone();
                            }

                            // Re-collect vertices from sectors (they may have new IDs after duplication)
                            let mut current_vertices = vec![];
                            for sector_id in sectors_to_move.iter() {
                                if let Some(sector) = map.find_sector(*sector_id) {
                                    for line_id in &sector.linedefs {
                                        if let Some(line) = map.find_linedef(*line_id) {
                                            if !current_vertices.contains(&line.start_vertex) {
                                                current_vertices.push(line.start_vertex);
                                            }
                                            if !current_vertices.contains(&line.end_vertex) {
                                                current_vertices.push(line.end_vertex);
                                            }
                                        }
                                    }
                                }
                            }

                            for vertex_id in current_vertices.iter() {
                                if let Some(original_vertex) =
                                    self.rectangle_undo_map.find_vertex(*vertex_id)
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

                                    // Only compute sectors to move once and cache it
                                    if self.cached_sectors_to_move.is_empty() {
                                        self.cached_sectors_to_move =
                                            self.rectangle_undo_map.selected_sectors.clone();

                                        // If Ctrl is pressed, include all embedded sectors
                                        if ui.ctrl {
                                            let mut embedded_sectors = vec![];
                                            for sector_id in
                                                &self.rectangle_undo_map.selected_sectors
                                            {
                                                let embedded =
                                                    map.find_embedded_sectors(*sector_id);
                                                for emb_id in embedded {
                                                    if !self
                                                        .cached_sectors_to_move
                                                        .contains(&emb_id)
                                                    {
                                                        embedded_sectors.push(emb_id);
                                                    }
                                                }
                                            }
                                            self.cached_sectors_to_move.extend(embedded_sectors);
                                        }
                                    }

                                    let sectors_to_move = &self.cached_sectors_to_move;

                                    let mut selected_vertices = vec![];
                                    for sector_id in sectors_to_move.iter() {
                                        if let Some(sector) =
                                            self.rectangle_undo_map.find_sector(*sector_id)
                                        {
                                            for line_id in &sector.linedefs {
                                                if let Some(line) =
                                                    self.rectangle_undo_map.find_linedef(*line_id)
                                                {
                                                    if !selected_vertices
                                                        .contains(&line.start_vertex)
                                                    {
                                                        selected_vertices.push(line.start_vertex);
                                                    }
                                                    if !selected_vertices.contains(&line.end_vertex)
                                                    {
                                                        selected_vertices.push(line.end_vertex);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Duplicate shared vertices only once at the start of dragging
                                    if !self.vertices_duplicated {
                                        for vertex_id in selected_vertices.iter() {
                                            // Check if this vertex is shared with any unselected rect sector
                                            let is_unselected_rect_vertex =
                                                map.sectors.iter().any(|sector| {
                                                    if sector.properties.contains("rect")
                                                        && !sectors_to_move.contains(&sector.id)
                                                    {
                                                        sector.linedefs.iter().any(|&line_id| {
                                                            if let Some(line) =
                                                                map.find_linedef(line_id)
                                                            {
                                                                line.start_vertex == *vertex_id
                                                                    || line.end_vertex == *vertex_id
                                                            } else {
                                                                false
                                                            }
                                                        })
                                                    } else {
                                                        false
                                                    }
                                                });

                                            if is_unselected_rect_vertex {
                                                // Vertex is shared with rect geometry - duplicate it for the sectors being moved
                                                if let Some(new_vertex_id) =
                                                    map.duplicate_vertex(*vertex_id)
                                                {
                                                    // Replace old vertex with new vertex in all sectors being moved
                                                    for sector_id in sectors_to_move.iter() {
                                                        map.replace_vertex_in_sector(
                                                            *sector_id,
                                                            *vertex_id,
                                                            new_vertex_id,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        self.vertices_duplicated = true;
                                        // Update rectangle_undo_map after duplication so future drags use correct vertex IDs
                                        self.rectangle_undo_map = map.clone();
                                    }

                                    // Re-collect vertices from sectors (they may have new IDs after duplication)
                                    let mut current_vertices = vec![];
                                    for sector_id in sectors_to_move.iter() {
                                        if let Some(sector) = map.find_sector(*sector_id) {
                                            for line_id in &sector.linedefs {
                                                if let Some(line) = map.find_linedef(*line_id) {
                                                    if !current_vertices
                                                        .contains(&line.start_vertex)
                                                    {
                                                        current_vertices.push(line.start_vertex);
                                                    }
                                                    if !current_vertices.contains(&line.end_vertex)
                                                    {
                                                        current_vertices.push(line.end_vertex);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    for vertex_id in current_vertices.iter() {
                                        if let Some(original_vertex) =
                                            self.rectangle_undo_map.find_vertex(*vertex_id)
                                        {
                                            let new_x = original_vertex.x + drag_delta.x;
                                            let new_y = original_vertex.y + drag_delta.z;
                                            let new_z = original_vertex.z + drag_delta.y;

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
                } else if let Some(render_view) = ui.get_render_view("PolyView") {
                    if !self.was_clicked {
                        return None;
                    }

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

                        selection.0 = vec![];
                        selection.1 = vec![];
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
                        let linedefs = rusterix.scene_handler.vm.active_vm().pick_geo_ids_in_rect(
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
                                if let Some(linedef) = map.find_linedef(l) {
                                    for s in &linedef.sector_ids {
                                        if !selection.2.contains(s) {
                                            selection.2.push(*s);
                                        }
                                    }
                                }
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
                        map.selected_sectors = selection.2;
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
                        let h = server_ctx.geometry_at(
                            Vec2::new(dim.width as f32, dim.height as f32),
                            Vec2::new(coord.x as f32, coord.y as f32),
                            map,
                        );
                        server_ctx.hover.2 = h.2;

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
                                GeoId::Sector(id) => {
                                    server_ctx.hover = (None, None, Some(id));
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

                    if let Some(s) = server_ctx.hover.2 {
                        if let Some(sector) = map.find_sector(s) {
                            let lines = sector
                                .linedefs
                                .iter()
                                .map(|id| id.to_string())
                                .collect::<Vec<_>>()
                                .join(", ");
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                format!("Sector {}: Linedefs ({})", s, lines),
                            ));
                        }
                    } else {
                        ctx.ui
                            .send(TheEvent::SetStatusText(TheId::empty(), "".into()));
                    }
                }
            }
            MapDelete => {
                if !map.selected_sectors.is_empty() {
                    let prev = map.clone();
                    let sectors = map.selected_sectors.clone();

                    #[allow(clippy::useless_vec)]
                    map.delete_elements(&vec![], &vec![], &sectors);
                    map.selected_sectors.clear();

                    undo_atom = Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(prev),
                        Box::new(map.clone()),
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                }
            }
            MapEscape => {
                if !map.selected_sectors.is_empty() {
                    map.selected_sectors.clear();
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                }
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
        let id = if !map.selected_sectors.is_empty() {
            Some(map.selected_sectors[0])
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
                    }*/
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
                            for sector_id in &map.selected_sectors.clone() {
                                if let Some(sector) = map.find_sector_mut(*sector_id) {
                                    if context == NodeContext::Screen
                                        && server_ctx.curr_map_tool_helper
                                            == MapToolHelper::NodeEditor
                                    {
                                        sector.properties.set("screen_graph", source.clone());
                                    } else if context == NodeContext::Region
                                        && server_ctx.curr_map_tool_helper
                                            == MapToolHelper::NodeEditor
                                    {
                                        sector.properties.set("region_graph", source.clone());
                                    } else if context == NodeContext::Shape {
                                        sector.properties.set("shape_graph", source.clone());
                                    } else if self.hud.selected_icon_index == 0
                                        || context == NodeContext::Material
                                    {
                                        sector.properties.set("source", source.clone());
                                    } else if self.hud.selected_icon_index == 1 {
                                        sector.properties.set("ceiling_source", source.clone());
                                    }
                                }
                            }

                            // Force update
                            if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor {
                                NODEEDITOR.read().unwrap().force_update(ctx, map);
                            }

                            let undo_atom =
                                RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));

                            if server_ctx.get_map_context() == MapContext::Region {
                                crate::editor::UNDOMANAGER.write().unwrap().add_region_undo(
                                    &server_ctx.curr_region,
                                    undo_atom,
                                    ctx,
                                );
                            } else if server_ctx.get_map_context() == MapContext::Model {
                                /*
                                if let Some(material_undo_atom) = undo_atom.to_material_atom() {
                                    crate::editor::UNDOMANAGER
                                        .write()
                                        .unwrap()
                                        .add_material_undo(material_undo_atom, ctx);
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Update Materialpicker"),
                                        TheValue::Empty,
                                    ));
                                }*/
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
                        let prev = map.clone();
                        let context = NODEEDITOR.read().unwrap().context;
                        for sector_id in map.selected_sectors.clone() {
                            if let Some(sector) = map.find_sector_mut(sector_id) {
                                if context == NodeContext::Region
                                    && server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor
                                {
                                    sector.properties.remove("region_graph");
                                } else if context == NodeContext::Screen
                                    && server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor
                                {
                                    sector.properties.remove("screen_graph");
                                } else if self.hud.selected_icon_index == 0 {
                                    if sector.properties.contains("floor_light") {
                                        sector.properties.remove("floor_light");
                                    } else {
                                        sector
                                            .properties
                                            .set("source", Value::Source(PixelSource::Off));
                                    }
                                } else if self.hud.selected_icon_index == 1 {
                                    if sector.properties.contains("ceiling_light") {
                                        sector.properties.remove("ceiling_light");
                                    } else {
                                        sector
                                            .properties
                                            .set("ceiling_source", Value::Source(PixelSource::Off));
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

                        if server_ctx.get_map_context() == MapContext::Region {
                            crate::editor::UNDOMANAGER.write().unwrap().add_region_undo(
                                &server_ctx.curr_region,
                                undo_atom,
                                ctx,
                            );
                        } else if server_ctx.get_map_context() == MapContext::Model {
                            /*
                            if let Some(material_undo_atom) = undo_atom.to_material_atom() {
                                crate::editor::UNDOMANAGER
                                    .write()
                                    .unwrap()
                                    .add_material_undo(material_undo_atom, ctx);
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Materialpicker"),
                                    TheValue::Empty,
                                ));
                            }*/
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
            }
            _ => {}
        }
        redraw
    }
}
