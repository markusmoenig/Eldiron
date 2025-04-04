use crate::editor::RUSTERIX;
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use rusterix::prelude::*;
use vek::Vec2;
use MapEvent::*;
use ToolEvent::*;

pub struct RectTool {
    id: TheId,

    hovered_vertices: Option<[Vec2<f32>; 4]>,
    mode: i32,
    hud: Hud,
}

impl Tool for RectTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Rect Tool"),

            hovered_vertices: None,
            mode: 0,
            hud: Hud::new(HudMode::Rect),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Rect Tool (R). Click to draw current source in the grid. Shift-click to delete.")
    }
    fn icon_name(&self) -> String {
        str!("square")
    }
    fn accel(&self) -> Option<char> {
        Some('r')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
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

                server_ctx.curr_map_tool_type = MapToolType::Rect;

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.selected_vertices.clear();
                    region.map.selected_sectors.clear();
                }

                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    let mut source_switch = TheGroupButton::new(TheId::named("Map Helper Switch"));
                    source_switch.add_text_status(
                        "Tile Picker".to_string(),
                        "Show tile picker.".to_string(),
                    );
                    source_switch.add_text_status(
                        "Materials".to_string(),
                        "Apply procedural materials.".to_string(),
                    );
                    source_switch
                        .add_text_status("Colors".to_string(), "Apply a color.".to_string());
                    source_switch
                        .add_text_status("Effects".to_string(), "Apply an effect.".to_string());
                    source_switch
                        .add_text_status("Preview".to_string(), "Preview the map.".to_string());
                    source_switch.set_item_width(80);
                    source_switch.set_index(server_ctx.curr_map_tool_helper as i32);
                    layout.add_widget(Box::new(source_switch));

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
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::EffectsPicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::EffectPicker as usize,
                        ));
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::Preview {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::PreviewView as usize,
                        ));
                    }

                    let mut mode_switch = TheGroupButton::new(TheId::named("Rect Mode Switch"));
                    mode_switch.add_text_status(
                        "Overwrite".to_string(),
                        "Overwrite existing rects.".to_string(),
                    );
                    mode_switch.add_text_status(
                        "Layer".to_string(),
                        "Layer rects on top of each other.".to_string(),
                    );

                    mode_switch.set_item_width(100);
                    mode_switch.set_index(self.mode);
                    layout.add_widget(Box::new(mode_switch));

                    layout.set_reverse_index(Some(1));
                }

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
    ) -> Option<RegionUndoAtom> {
        let mut undo_atom: Option<RegionUndoAtom> = None;

        /// Add a tile at the current hover position
        fn add_tile(
            ui: &mut TheUI,
            _ctx: &mut TheContext,
            map: &mut Map,
            server_ctx: &mut ServerContext,
            hovered_vertices: Option<[Vec2<f32>; 4]>,
            mode: i32,
        ) -> Option<RegionUndoAtom> {
            let mut undo_atom: Option<RegionUndoAtom> = None;
            let size = 1.0 / map.subdivisions;

            let prev = map.clone();
            if let Some(vertices) = hovered_vertices {
                let mut add_it = true;
                let mut layer: u8 = 0;

                if ui.shift {
                    // Delete the top tile at the given position if shift is pressed
                    if let Some(ev0) = map.find_vertex_at(vertices[0].x, vertices[0].y) {
                        if let Some(ev1) = map.find_vertex_at(vertices[1].x, vertices[1].y) {
                            if let Some(ev2) = map.find_vertex_at(vertices[2].x, vertices[2].y) {
                                if let Some(ev3) = map.find_vertex_at(vertices[3].x, vertices[3].y)
                                {
                                    let sectors =
                                        map.find_sectors_with_vertex_indices(&[ev0, ev1, ev2, ev3]);

                                    if let Some(sector_id) = sectors.last() {
                                        let mut lines = vec![];
                                        if let Some(s) = map.find_sector(*sector_id) {
                                            lines = s.linedefs.clone();
                                        }
                                        map.delete_elements(&[], &lines, &[*sector_id]);
                                        undo_atom = Some(RegionUndoAtom::MapEdit(
                                            Box::new(prev),
                                            Box::new(map.clone()),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(source) = get_source(ui, server_ctx) {
                    // Add mode
                    // Check if tile already exists with same source
                    if let Some(ev0) = map.find_vertex_at(vertices[0].x, vertices[0].y) {
                        if let Some(ev1) = map.find_vertex_at(vertices[1].x, vertices[1].y) {
                            if let Some(ev2) = map.find_vertex_at(vertices[2].x, vertices[2].y) {
                                if let Some(ev3) = map.find_vertex_at(vertices[3].x, vertices[3].y)
                                {
                                    let sectors =
                                        map.find_sectors_with_vertex_indices(&[ev0, ev1, ev2, ev3]);

                                    for sector_id in sectors {
                                        if let Some(sector) = map.find_sector_mut(sector_id) {
                                            if let Some(sector_floor_source) =
                                                sector.properties.get("floor_source")
                                            {
                                                // Assign id to the higher current layer id (+1).
                                                if let Some(l) = &sector.layer {
                                                    if *l > layer {
                                                        layer = *l;
                                                    }
                                                }

                                                if source == *sector_floor_source {
                                                    // A tile with the same floor_source exists, do not add.
                                                    add_it = false;
                                                } else if mode == 0 {
                                                    // In overlay mode we just overwrite the source
                                                    sector
                                                        .properties
                                                        .set("floor_source", source.clone());
                                                    add_it = false;
                                                }
                                            }
                                        }
                                    }

                                    if !add_it {
                                        undo_atom = Some(RegionUndoAtom::MapEdit(
                                            Box::new(prev.clone()),
                                            Box::new(map.clone()),
                                        ));
                                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                    }
                                }
                            }
                        }
                    }

                    if add_it {
                        let v0 = map.add_vertex_at(vertices[0].x, vertices[0].y);
                        let v1 = map.add_vertex_at(vertices[1].x, vertices[1].y);
                        let v2 = map.add_vertex_at(vertices[2].x, vertices[2].y);
                        let v3 = map.add_vertex_at(vertices[3].x, vertices[3].y);

                        map.possible_polygon = vec![];
                        let l0 = map.create_linedef(v0, v1);
                        let l1 = map.create_linedef(v1, v2);
                        let l2 = map.create_linedef(v2, v3);
                        let id = map.create_linedef(v3, v0);

                        if let Some(sector_id) = id.1 {
                            // Add the info for correct box rendering
                            if let Some(l) = map.find_linedef_mut(l0.0) {
                                l.properties.set("row1_source", source.clone());
                                l.properties.set("wall_height", Value::Float(size));
                            }
                            if let Some(l) = map.find_linedef_mut(l1.0) {
                                l.properties.set("row1_source", source.clone());
                                l.properties.set("wall_height", Value::Float(size));
                            }
                            if let Some(l) = map.find_linedef_mut(l2.0) {
                                l.properties.set("row1_source", source.clone());
                                l.properties.set("wall_height", Value::Float(size));
                            }
                            if let Some(l) = map.find_linedef_mut(id.0) {
                                l.properties.set("row1_source", source.clone());
                                l.properties.set("wall_height", Value::Float(size));
                            }

                            if let Some(sector) = map.find_sector_mut(sector_id) {
                                if let Value::Source(PixelSource::TileId(id)) = source {
                                    if let Some(tile) =
                                        RUSTERIX.read().unwrap().assets.tiles.get(&id)
                                    {
                                        sector.properties.set(
                                            "rect_rendering",
                                            Value::Int(tile.render_mode as i32),
                                        );
                                    }
                                }

                                sector.properties.set("floor_source", source);
                                sector.properties.set("ceiling_height", Value::Float(size));
                                sector.layer = Some(layer + 1);

                                undo_atom = Some(RegionUndoAtom::MapEdit(
                                    Box::new(prev),
                                    Box::new(map.clone()),
                                ));

                                map.selected_vertices.clear();
                                map.selected_linedefs.clear();
                                map.selected_sectors = vec![sector_id];
                                // ctx.ui.send(TheEvent::Custom(
                                //     TheId::named("Map Selection Changed"),
                                //     TheValue::Empty,
                                // ));
                                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            }
                        } else {
                            println!("rect polygon not created");
                        }
                    }
                }
            }
            undo_atom
        }

        fn apply_hover(
            coord: Vec2<i32>,
            ui: &mut TheUI,
            _ctx: &mut TheContext,
            map: &mut Map,
            server_ctx: &mut ServerContext,
        ) -> Option<[Vec2<f32>; 4]> {
            let mut hovered_vertices: Option<[Vec2<f32>; 4]> = None;

            if let Some(render_view) = ui.get_render_view("PolyView") {
                let dim = *render_view.dim();
                server_ctx.hover = (None, None, None);
                let cp = server_ctx.local_to_map_cell(
                    Vec2::new(dim.width as f32, dim.height as f32),
                    Vec2::new(coord.x as f32, coord.y as f32),
                    map,
                    map.subdivisions,
                );
                // The size of the rect is always 1
                let step = 1.0; // / map.subdivisions;
                map.curr_rectangle = Some((cp, cp + step));
                hovered_vertices = Some([
                    cp,
                    cp + Vec2::new(0.0, step),
                    cp + Vec2::new(step, step),
                    cp + Vec2::new(step, 0.0),
                ]);
                server_ctx.hover_cursor = Some(cp);
            }

            hovered_vertices
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
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }
                undo_atom = add_tile(ui, ctx, map, server_ctx, self.hovered_vertices, self.mode);
                if undo_atom.is_some() {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Minimap"),
                        TheValue::Empty,
                    ));
                }
            }
            MapDragged(coord) => {
                if self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }
                self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
                undo_atom = add_tile(ui, ctx, map, server_ctx, self.hovered_vertices, self.mode);
                if undo_atom.is_some() {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Minimap"),
                        TheValue::Empty,
                    ));
                }
            }
            MapUp(_) => {}
            MapHover(coord) => {
                self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
            }
            MapDelete => {}
            MapEscape => {
                map.clear_temp();
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
    ) {
        let id = if !map.selected_linedefs.is_empty() {
            Some(map.selected_linedefs[0])
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
                                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                }
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

                        for linedef_id in map.selected_linedefs.clone() {
                            if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                if self.hud.selected_icon_index == 0 {
                                    linedef
                                        .properties
                                        .set("row1_source", Value::Source(PixelSource::Off));
                                } else if self.hud.selected_icon_index == 1 {
                                    linedef
                                        .properties
                                        .set("row2_source", Value::Source(PixelSource::Off));
                                } else if self.hud.selected_icon_index == 2 {
                                    linedef
                                        .properties
                                        .set("row3_source", Value::Source(PixelSource::Off));
                                } else if self.hud.selected_icon_index == 3 {
                                    linedef
                                        .properties
                                        .set("row4_source", Value::Source(PixelSource::Off));
                                }
                                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            }
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
            }
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
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::EffectsPicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::EffectPicker as usize,
                        ));
                    } else if server_ctx.curr_map_tool_helper == MapToolHelper::Preview {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::PreviewView as usize,
                        ));
                    };
                    redraw = true;
                } else if id.name == "Rect Mode Switch" {
                    self.mode = *index as i32;
                }
            }
            _ => {}
        }
        redraw
    }
}
