use crate::prelude::*;
use shared::server::prelude::MapToolType;
use MapEvent::*;
use ToolEvent::*;

pub struct SectorTool {
    id: TheId,
    click_pos: Vec2f,
    rectangle_undo_map: Map,
    click_selected: bool,
    drag_changed: bool,
    wall_height: f32,
}

impl Tool for SectorTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Sector Tool"),
            click_pos: Vec2f::zero(),
            click_selected: false,
            drag_changed: false,
            rectangle_undo_map: Map::default(),
            wall_height: 0.0,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Sector Tool (S).")
    }
    fn icon_name(&self) -> String {
        str!("polygon")
    }
    fn accel(&self) -> Option<char> {
        Some('e')
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
                // Display the tile edit panel.
                ctx.ui
                    .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));

                if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                    layout.set_mode(TheSharedHLayoutMode::Right);
                    ctx.ui.relayout = true;
                }

                server_ctx.curr_character_instance = None;
                server_ctx.curr_item_instance = None;
                server_ctx.curr_area = None;
                server_ctx.curr_map_tool_type = MapToolType::Sector;

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.selected_vertices.clear();
                    region.map.selected_linedefs.clear();
                    server.update_region(region);
                }

                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    let mut wall_height = TheTextLineEdit::new(TheId::named("Wall Height"));
                    wall_height.set_value(TheValue::Float(self.wall_height));
                    // opacity.set_default_value(TheValue::Float(1.0));
                    wall_height.set_info_text(Some("Wall Height".to_string()));
                    wall_height.set_range(TheValue::RangeF32(0.0..=4.0));
                    wall_height.set_continuous(true);
                    wall_height.limiter_mut().set_max_width(150);
                    wall_height.set_status_text("The wall height of the enclosing linedefs.");
                    layout.add_widget(Box::new(wall_height));
                }

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
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut undo_atom: Option<RegionUndoAtom> = None;

        match map_event {
            MapClicked(coord) => {
                self.click_selected = false;
                if server_ctx.hover.2.is_some() {
                    let prev = map.clone();
                    let mut changed = false;

                    if ui.shift {
                        // Add
                        if let Some(s) = server_ctx.hover.2 {
                            if !map.selected_sectors.contains(&s) {
                                map.selected_sectors.push(s);
                                changed = true;
                                self.click_selected = true;
                            }
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

                self.click_pos = vec2f(coord.x as f32, coord.y as f32);
                self.rectangle_undo_map = map.clone();
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

                        for sector_id in self.rectangle_undo_map.selected_sectors.iter() {
                            if let Some(sector) = self.rectangle_undo_map.find_sector(*sector_id) {
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
                } else if let Some(render_view) = ui.get_render_view("PolyView") {
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

                    let top_left =
                        Vec2f::new(click_pos.x.min(drag_pos.x), click_pos.y.min(drag_pos.y));
                    let bottom_right =
                        Vec2f::new(click_pos.x.max(drag_pos.x), click_pos.y.max(drag_pos.y));

                    let mut selection =
                        server_ctx.geometry_in_rectangle(top_left, bottom_right, map);

                    selection.0 = vec![];
                    selection.1 = vec![];

                    *map = self.rectangle_undo_map.clone();
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
                        map.selected_sectors = selection.2;
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
                } else if map.curr_rectangle.is_some() {
                    map.curr_rectangle = None;

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
                    let h = server_ctx.geometry_at(
                        vec2f(dim.width as f32, dim.height as f32),
                        vec2f(coord.x as f32, coord.y as f32),
                        map,
                    );
                    server_ctx.hover.2 = h.2;

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
                if !map.selected_sectors.is_empty() {
                    let prev = map.clone();
                    let sectors = map.selected_sectors.clone();

                    #[allow(clippy::useless_vec)]
                    map.delete_elements(&vec![], &vec![], &sectors);
                    map.selected_sectors.clear();

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
                // Hover is empty, check if we need to clear selection
                if !map.selected_sectors.is_empty() {
                    let prev = map.clone();
                    map.selected_sectors.clear();
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

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Wall Height" {
                    if let Some(value) = value.to_f32() {
                        self.wall_height = value;

                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let mut linedef_ids = Vec::new();
                            for sector_id in &region.map.selected_sectors {
                                if let Some(sector) = region.map.find_sector(*sector_id) {
                                    linedef_ids.extend(&sector.linedefs);
                                }
                            }

                            for linedef_id in linedef_ids {
                                if let Some(linedef) = region.map.find_linedef_mut(linedef_id) {
                                    linedef.wall_height = value;
                                }
                            }
                        }
                    }
                    redraw = true;
                }
            }
            _ => {}
        }
        redraw
    }
}
