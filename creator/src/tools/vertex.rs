use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;

pub struct VertexTool {
    id: TheId,
    click_pos: Vec2<f32>,
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
        str!("Vertex Tool (P).")
    }
    fn icon_name(&self) -> String {
        str!("dot-outline")
    }
    fn accel(&self) -> Option<char> {
        Some('v')
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

                    server_ctx.curr_map_tool_type = MapToolType::Vertex;
                }

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.selected_linedefs.clear();
                    region.map.selected_sectors.clear();
                }

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
    ) -> Option<RegionUndoAtom> {
        let mut undo_atom: Option<RegionUndoAtom> = None;

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
                    let prev = map.clone();
                    let mut changed = false;

                    map.selected_entity_item = None;
                    map.selected_light = None;

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
                if self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if self.click_selected {
                    // If we selected a vertex, drag means we move all selected vertices
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

                        let drag_delta = click_pos - drag_pos;
                        for vertex_id in &map.selected_vertices.clone() {
                            if let Some(original_vertex) =
                                self.rectangle_undo_map.find_vertex_mut(*vertex_id)
                            {
                                // if let Some(vertex) = map.find_vertex_mut(*vertex_id) {
                                //     vertex.x = original_vertex.x - drag_delta.x;
                                //     vertex.y = original_vertex.y - drag_delta.y;
                                // }

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

                    let top_left =
                        Vec2::new(click_pos.x.min(drag_pos.x), click_pos.y.min(drag_pos.y));
                    let bottom_right =
                        Vec2::new(click_pos.x.max(drag_pos.x), click_pos.y.max(drag_pos.y));

                    let mut selection =
                        server_ctx.geometry_in_rectangle(top_left, bottom_right, map);

                    selection.1 = vec![];
                    selection.2 = vec![];

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
                self.drag_changed = false;
                self.click_selected = false;
            }
            MapHover(coord) => {
                if self.hud.hovered(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

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
            }
            MapDelete => {
                if !map.selected_vertices.is_empty() {
                    let prev = map.clone();
                    let vertices = map.selected_vertices.clone();

                    #[allow(clippy::useless_vec)]
                    map.delete_elements(&vertices, &vec![], &vec![]);
                    map.selected_vertices.clear();

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
                if !map.selected_vertices.is_empty() {
                    let prev = map.clone();

                    map.selected_vertices.clear();

                    undo_atom = Some(RegionUndoAtom::MapEdit(
                        Box::new(prev),
                        Box::new(map.clone()),
                    ));

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
        palette: &ThePalette,
    ) {
        let id = if !map.selected_vertices.is_empty() {
            Some(map.selected_vertices[0])
        } else {
            None
        };
        self.hud.draw(buffer, map, ctx, server_ctx, id, palette);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        #[allow(clippy::single_match)]
        match event {
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
                    }
                    redraw = true;
                }
            }
            _ => {}
        }
        redraw
    }
}
