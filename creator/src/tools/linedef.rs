use crate::editor::NODEEDITOR;
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::{Surface, prelude::*};
use scenevm::GeoId;
use vek::Vec2;

pub struct LinedefTool {
    id: TheId,
    click_pos: Vec2<f32>,
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
        str!("Linedef Tool (L). Create line definitions and sectors.")
    }
    fn icon_name(&self) -> String {
        str!("line-segment")
    }
    fn accel(&self) -> Option<char> {
        Some('L')
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
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            MapClicked(coord) => {
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    self.was_clicked = false;
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                self.was_clicked = true;

                // ---

                self.click_selected = false;
                if map.curr_grid_pos.is_none() && server_ctx.hover.1.is_some() {
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

                                // Returns id of linedef and optional id of new sector if polygon closes
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

                                undo_atom = Some(ProjectUndoAtom::MapEdit(
                                    server_ctx.pc,
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
                            map.curr_grid_pos = Some(vek::Vec2::new(grid_pos.x, grid_pos.y));
                        }
                    }
                }

                self.click_pos = Vec2::new(coord.x as f32, coord.y as f32);
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

                            let top_left =
                                Vec2::new(click_pos.x.min(drag_pos.x), click_pos.y.min(drag_pos.y));
                            let bottom_right =
                                Vec2::new(click_pos.x.max(drag_pos.x), click_pos.y.max(drag_pos.y));

                            let mut selection =
                                server_ctx.geometry_in_rectangle(top_left, bottom_right, map);

                            selection.0 = vec![];
                            selection.2 = vec![];

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
