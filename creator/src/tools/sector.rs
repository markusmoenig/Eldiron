use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use rusterix::{PixelSource, Value};
use vek::Vec2;
use MapEvent::*;
use ToolEvent::*;

pub struct SectorTool {
    id: TheId,
    click_pos: Vec2<f32>,
    rectangle_undo_map: Map,
    click_selected: bool,
    drag_changed: bool,

    properties_code: String,
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
            click_selected: false,
            drag_changed: false,
            rectangle_undo_map: Map::default(),

            properties_code: r#"# Sets the wall height (default is 2.0)
# set("wall_height", 2.0)
"#
            .to_string(),
            hud: Hud::new(HudMode::Sector),
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

                    let mut material_switch =
                        TheGroupButton::new(TheId::named("Map Helper Switch"));
                    material_switch.add_text_status(
                        "Tile Picker".to_string(),
                        "Show tile picker.".to_string(),
                        // "bricks".to_string(),
                    );
                    material_switch.add_text_status(
                        "Materials".to_string(),
                        "Apply materials.".to_string(),
                        // "faders".to_string(),
                    );
                    material_switch.add_text_status(
                        "Colors".to_string(),
                        "Apply a color.".to_string(),
                        // "square".to_string(),
                    );
                    material_switch.add_text_status(
                        "Script".to_string(),
                        "Sector Script.".to_string(),
                        // "code".to_string(),
                    );
                    material_switch.set_item_width(100);
                    material_switch.set_index(server_ctx.curr_map_tool_helper as i32);
                    layout.add_widget(Box::new(material_switch));

                    if server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::TilePicker as usize,
                        ));
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::MaterialPicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::MaterialPicker as usize,
                        ));
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::ColorPicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::ColorPicker as usize,
                        ));
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::Properties {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::TextEditor as usize,
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Material Previews"),
                            TheValue::Empty,
                        ));
                    };

                    let mut run_properties_button =
                        TheTraybarButton::new(TheId::named("Apply Map Properties"));
                    run_properties_button.set_status_text("Apply to the selected sectors.");
                    run_properties_button.set_text("Apply Property".to_string());
                    layout.add_widget(Box::new(run_properties_button));
                    layout.set_reverse_index(Some(1));

                    ui.set_widget_value(
                        "CodeEdit",
                        ctx,
                        TheValue::Text(self.properties_code.clone()),
                    );

                    /*
                    let mut wall_width = TheTextLineEdit::new(TheId::named("Wall Width"));
                    wall_width.set_value(TheValue::Float(self.wall_width));
                    // opacity.set_default_value(TheValue::Float(1.0));
                    wall_width.set_info_text(Some("Wall Width".to_string()));
                    wall_width.set_range(TheValue::RangeF32(0.0..=4.0));
                    wall_width.set_continuous(true);
                    wall_width.limiter_mut().set_max_width(150);
                    wall_width.set_status_text("The wall width of the enclosing linedefs.");
                    layout.add_widget(Box::new(wall_width));

                    let mut wall_height = TheTextLineEdit::new(TheId::named("Wall Height"));
                    wall_height.set_value(TheValue::Float(self.wall_height));
                    // opacity.set_default_value(TheValue::Float(1.0));
                    wall_height.set_info_text(Some("Wall Height".to_string()));
                    wall_height.set_range(TheValue::RangeF32(0.0..=4.0));
                    wall_height.set_continuous(true);
                    wall_height.limiter_mut().set_max_width(150);
                    wall_height.set_status_text("The wall height of the enclosing linedefs.");
                    layout.add_widget(Box::new(wall_height));*/
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
            MapKey(c) => {
                match c {
                    '1'..='9' => map.subdivisions = (c as u8 - b'0') as f32,
                    '0' => map.subdivisions = 10.0,
                    _ => {}
                }
                crate::editor::RUSTERIX.lock().unwrap().set_dirty();
            }
            MapClicked(coord) => {
                if self.hud.clicked(coord.x, coord.y, map, server_ctx) {
                    crate::editor::RUSTERIX.lock().unwrap().set_dirty();
                    return None;
                }
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

                self.click_pos = Vec2::new(coord.x as f32, coord.y as f32);
                self.rectangle_undo_map = map.clone();
            }
            MapDragged(coord) => {
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
                    selection.1 = vec![];

                    *map = self.rectangle_undo_map.clone();
                    map.curr_rectangle = Some((
                        Vec2::new(self.click_pos.x, self.click_pos.y),
                        Vec2::new(coord.x as f32, coord.y as f32),
                    ));
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

                    crate::editor::RUSTERIX.lock().unwrap().set_dirty();
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
                crate::editor::RUSTERIX.lock().unwrap().set_dirty();
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
    ) {
        let id = if map.selected_sectors.len() == 1 {
            Some(map.selected_sectors[0])
        } else {
            None
        };
        self.hud.draw(buffer, map, ctx, server_ctx, id);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
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
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::ColorPicker {
                        if let Some(palette_picker) = ui.get_palette_picker("Panel Palette Picker")
                        {
                            if let Some(color) = &project.palette.colors[palette_picker.index()] {
                                source = Some(Value::Source(PixelSource::Color(color.clone())));
                            }
                        }
                    }

                    if let Some(source) = source {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let prev = map.clone();

                            for sector_id in &map.selected_sectors.clone() {
                                if let Some(sector) = map.find_sector_mut(*sector_id) {
                                    if self.hud.selected_icon_index == 0 {
                                        sector.properties.set("floor_source", source.clone());
                                    } else if self.hud.selected_icon_index == 1 {
                                        sector.properties.set("ceiling_source", source.clone());
                                    } else if self.hud.selected_icon_index == 2 {
                                        sector.properties.set("row1_source", source.clone());
                                    } else if self.hud.selected_icon_index == 3 {
                                        sector.properties.set("row2_source", source.clone());
                                    } else if self.hud.selected_icon_index == 4 {
                                        sector.properties.set("row3_source", source.clone());
                                    }
                                    crate::editor::RUSTERIX.lock().unwrap().set_dirty();
                                }
                            }

                            let undo_atom =
                                RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));

                            crate::editor::UNDOMANAGER.lock().unwrap().add_region_undo(
                                &server_ctx.curr_region,
                                undo_atom,
                                ctx,
                            );
                            crate::editor::RUSTERIX.lock().unwrap().set_dirty();
                        }
                    }
                }
            }
            // TheEvent::ValueChanged(id, value) => {
            //     if id.name == "CodeEdit" {
            //         if let Some(code) = value.to_string() {
            //             self.properties_code = code;
            //         }
            //     }
            // }
            /*
            TheEvent::StateChanged(id, state) => {
                if id.name == "Apply Sector Properties" && *state == TheWidgetState::Clicked {
                    if let Some(value) = ui.get_widget_value("CodeEdit") {
                        if let Some(code) = value.to_string() {
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                for sector_id in &region.map.selected_sectors.clone() {
                                    let mut mapscript = rusterix::MapScript::new();
                                    let result = mapscript.transform(
                                        code.clone(),
                                        Some(region.map.clone()),
                                        None,
                                        Some(*sector_id),
                                    );
                                    match &result {
                                        Ok(meta) => region.map = meta.map.clone(),
                                        Err(err) => {
                                            if let Some(first) = err.first() {
                                                ctx.ui.send(TheEvent::SetStatusText(
                                                    TheId::empty(),
                                                    first.to_string(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }*/
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Map Helper Switch" {
                    server_ctx.curr_map_tool_helper.set_from_index(*index);
                    if server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::TilePicker as usize,
                        ));
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::MaterialPicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::MaterialPicker as usize,
                        ));
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::ColorPicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::ColorPicker as usize,
                        ));
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::Properties {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::TextEditor as usize,
                        ));
                    };
                    redraw = true;
                }
            }
            /*
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Wall Width" {
                    if let Some(value) = value.to_f32() {
                        self.wall_width = value;

                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let mut linedef_ids = Vec::new();
                            for sector_id in &region.map.selected_sectors {
                                if let Some(sector) = region.map.find_sector(*sector_id) {
                                    linedef_ids.extend(&sector.linedefs);
                                }
                            }

                            for linedef_id in linedef_ids {
                                if let Some(linedef) = region.map.find_linedef_mut(linedef_id) {
                                    linedef.wall_width = value;
                                }
                            }

                            server.update_region(region);
                        }
                    }
                    redraw = true;
                }
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
            }*/
            _ => {}
        }
        redraw
    }
}
