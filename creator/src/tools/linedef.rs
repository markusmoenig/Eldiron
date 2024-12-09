use crate::prelude::*;
use shared::server::prelude::MapToolType;
use MapEvent::*;
use ToolEvent::*;

pub struct LinedefTool {
    id: TheId,
    click_pos: Vec2f,
    click_selected: bool,
    drag_changed: bool,
    rectangle_undo_map: Map,
    rectangle_mode: bool,
}

impl Tool for LinedefTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Linedef Tool"),
            click_pos: Vec2f::zero(),
            click_selected: false,
            drag_changed: false,
            rectangle_undo_map: Map::default(),
            rectangle_mode: false,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Linedef Tool (L). Create line definitions and sectors.")
    }
    fn icon_name(&self) -> String {
        str!("line-segment")
    }
    fn accel(&self) -> Option<char> {
        Some('l')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::TilePicker as usize,
                ));

                if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                    layout.set_mode(TheSharedHLayoutMode::Right);
                    ctx.ui.relayout = true;
                }

                server_ctx.curr_character_instance = None;
                server_ctx.curr_item_instance = None;
                server_ctx.curr_area = None;
                server_ctx.curr_map_tool_type = MapToolType::Linedef;

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.selected_vertices.clear();
                    region.map.selected_sectors.clear();
                    server.update_region(region);
                }

                return true;
            }
            DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.clear_temp();
                    server.update_region(region);
                }
                return true;
            }
            _ => {}
        };

        /*
        // When we draw in 2D, move the 3D view to the pen position
        if tool_context == ToolContext::TwoD && server_ctx.curr_character_instance.is_none() {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                region.editing_position_3d = Vec3f::new(coord.x as f32, 0.0, coord.y as f32);
                server.set_editing_position_3d(region.editing_position_3d);
            }
        }

        if let Some(curr_tile_id) = server_ctx.curr_tile_id {
            if let Some(rgb_tile) = TILEDRAWER.lock().unwrap().tiles.get(&curr_tile_id) {
                let is_billboard = rgb_tile.billboard;
                if server_ctx.curr_layer_role == Layer2DRole::FX {
                    // Set the tile preview.
                    if let Some(widget) = ui.get_widget("TileFX RGBA") {
                        if let Some(tile_rgba) = widget.as_rgba_view() {
                            if let Some(tile) = project
                                .extract_region_tile(server_ctx.curr_region, (coord.x, coord.y))
                            {
                                let preview_size = TILEFXEDITOR.lock().unwrap().preview_size;
                                tile_rgba.set_grid(Some(preview_size / tile.buffer[0].dim().width));
                                tile_rgba
                                    .set_buffer(tile.buffer[0].scaled(preview_size, preview_size));
                            }
                        }
                    }
                }

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    let mut tiles_to_render: Vec<Vec2i> = vec![];

                    let mut prev = None;
                    if let Some(tile) = region.tiles.get(&(coord.x, coord.y)) {
                        prev = Some(tile.clone());
                    }

                    region.set_tile(
                        (coord.x, coord.y),
                        server_ctx.curr_layer_role,
                        server_ctx.curr_tile_id,
                    );

                    tiles_to_render.push(coord);
                    let region_to_render = Some(region.clone());

                    if let Some(tile) = region.tiles.get(&(coord.x, coord.y)) {
                        if prev != Some(tile.clone()) {
                            let undo = RegionUndoAtom::RegionTileEdit(
                                vec2i(coord.x, coord.y),
                                prev,
                                Some(tile.clone()),
                            );

                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);
                        }
                    }
                    //self.set_icon_previews(region, &palette, coord, ui);

                    server.update_region(region);

                    if !is_billboard {
                        RENDERER.lock().unwrap().set_region(region);

                        if let Some(region) = region_to_render {
                            PRERENDERTHREAD
                                .lock()
                                .unwrap()
                                .render_region(region, Some(tiles_to_render));
                        }
                    }
                }
            }
            //self.redraw_region(ui, server, ctx, server_ctx);
        }
        */
        false
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut undo_atom: Option<RegionUndoAtom> = None;

        match map_event {
            MapClicked(coord) => {
                self.click_selected = false;
                if map.curr_grid_pos.is_none() && server_ctx.hover.1.is_some() {
                    // Selected hovered line
                    let prev = map.clone();
                    let mut changed = false;

                    if ui.shift {
                        // Add
                        if let Some(l) = server_ctx.hover.1 {
                            if !map.selected_linedefs.contains(&l) {
                                map.selected_linedefs.push(l);
                                changed = true;
                                self.click_selected = true;
                            }
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
                        undo_atom = Some(RegionUndoAtom::MapEdit(
                            Box::new(prev),
                            Box::new(map.clone()),
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                    }
                } else {
                    // Line mode
                    let mut set_current_gid_pos = true;
                    if let Some(render_view) = ui.get_render_view("PolyView") {
                        let dim = *render_view.dim();
                        let grid_pos = server_ctx.local_to_map_grid(
                            vec2f(dim.width as f32, dim.height as f32),
                            vec2f(coord.x as f32, coord.y as f32),
                            map,
                            map.subdivisions,
                        );

                        if let Some(curr_grid_pos) = map.curr_grid_pos {
                            if curr_grid_pos.x != grid_pos.x || curr_grid_pos.y != grid_pos.y {
                                let prev = map.clone();

                                let start_vertex =
                                    map.add_vertex_at(curr_grid_pos.x, curr_grid_pos.y);
                                let end_vertex = map.add_vertex_at(grid_pos.x, grid_pos.y);

                                // Returns id of linedef and optional id of new sector if polygon closes
                                let ids = map.create_linedef(start_vertex, end_vertex);

                                if ids.1.is_some() {
                                    // When we close a polygon delete the temporary data
                                    map.clear_temp();
                                    set_current_gid_pos = false;
                                }

                                undo_atom = Some(RegionUndoAtom::MapEdit(
                                    Box::new(prev),
                                    Box::new(map.clone()),
                                ));

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Minimap"),
                                    TheValue::Empty,
                                ));
                            }
                        }

                        if set_current_gid_pos {
                            map.curr_grid_pos = Some(grid_pos);
                        }
                    }
                }

                self.click_pos = vec2f(coord.x as f32, coord.y as f32);
                self.rectangle_undo_map = map.clone();
                self.rectangle_mode = false;
            }
            MapDragged(coord) => {
                if self.click_selected {
                    // Dragging selected lines
                    if let Some(render_view) = ui.get_render_view("PolyView") {
                        let dim = *render_view.dim();
                        let click_pos = server_ctx.local_to_map_grid(
                            vec2f(dim.width as f32, dim.height as f32),
                            self.click_pos,
                            map,
                            map.subdivisions,
                        );
                        let drag_pos = server_ctx.local_to_map_grid(
                            vec2f(dim.width as f32, dim.height as f32),
                            vec2f(coord.x as f32, coord.y as f32),
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
                                if let Some(vertex) = map.find_vertex_mut(*vertex_id) {
                                    vertex.x = original_vertex.x - drag_delta.x;
                                    vertex.y = original_vertex.y - drag_delta.y;
                                }
                            }
                        }
                        server_ctx.hover_cursor = Some(drag_pos);
                        if drag_delta.x != 0.0 || drag_delta.y != 0.0 {
                            self.drag_changed = true;
                        }
                    }
                } else {
                    if !self.rectangle_mode {
                        let dist = distance(self.click_pos, vec2f(coord.x as f32, coord.y as f32));
                        if dist > 10.0 {
                            self.rectangle_mode = true;
                            map.clear_temp();
                        }
                    }

                    if self.rectangle_mode {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            let click_pos = server_ctx.local_to_map_grid(
                                vec2f(dim.width as f32, dim.height as f32),
                                self.click_pos,
                                map,
                                map.subdivisions,
                            );
                            let drag_pos = server_ctx.local_to_map_grid(
                                vec2f(dim.width as f32, dim.height as f32),
                                vec2f(coord.x as f32, coord.y as f32),
                                map,
                                map.subdivisions,
                            );

                            let top_left = Vec2f::new(
                                click_pos.x.min(drag_pos.x),
                                click_pos.y.min(drag_pos.y),
                            );
                            let bottom_right = Vec2f::new(
                                click_pos.x.max(drag_pos.x),
                                click_pos.y.max(drag_pos.y),
                            );

                            let mut selection =
                                server_ctx.geometry_in_rectangle(top_left, bottom_right, map);

                            selection.0 = vec![];
                            selection.2 = vec![];

                            *map = self.rectangle_undo_map.clone();
                            map.curr_grid_pos = None;
                            map.curr_rectangle =
                                Some((self.click_pos, vec2f(coord.x as f32, coord.y as f32)));

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
            }
            MapUp(_) => {
                if self.click_selected {
                    if self.drag_changed {
                        undo_atom = Some(RegionUndoAtom::MapEdit(
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

                    undo_atom = Some(RegionUndoAtom::MapEdit(
                        Box::new(self.rectangle_undo_map.clone()),
                        Box::new(map.clone()),
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
            }
            MapHover(coord) => {
                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();
                    if !self.rectangle_mode {
                        map.curr_mouse_pos = Some(vec2f(coord.x as f32, coord.y as f32));
                    }
                    let mut hover = server_ctx.geometry_at(
                        vec2f(dim.width as f32, dim.height as f32),
                        vec2f(coord.x as f32, coord.y as f32),
                        map,
                    );
                    hover.0 = None;
                    hover.2 = None;

                    server_ctx.hover = hover;
                    let cp = server_ctx.local_to_map_grid(
                        vec2f(dim.width as f32, dim.height as f32),
                        vec2f(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    );
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Cursor Pos Changed"),
                        TheValue::Float2(cp),
                    ));
                    server_ctx.hover_cursor = Some(cp);
                }
            }
            MapDelete => {
                if !map.selected_linedefs.is_empty() {
                    let prev = map.clone();
                    let lines = map.selected_linedefs.clone();

                    #[allow(clippy::useless_vec)]
                    map.delete_elements(&vec![], &lines, &vec![]);
                    map.selected_linedefs.clear();

                    undo_atom = Some(RegionUndoAtom::MapEdit(
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
                // Hover is empty, check if we need to clear selection
                if !map.selected_linedefs.is_empty() {
                    let prev = map.clone();

                    map.selected_linedefs.clear();

                    undo_atom = Some(RegionUndoAtom::MapEdit(
                        Box::new(prev),
                        Box::new(map.clone()),
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
            }
        }
        undo_atom
    }
}
