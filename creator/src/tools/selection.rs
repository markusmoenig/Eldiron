use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::Assets;

pub struct SelectionTool {
    id: TheId,
    click_pos: Vec2<f32>,
    rectangle_undo_map: Map,
    hud: Hud,
}

impl Tool for SelectionTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Select Tool"),
            click_pos: Vec2::zero(),
            rectangle_undo_map: Map::default(),
            hud: Hud::new(HudMode::Selection),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        if cfg!(target_os = "macos") {
            fl!("tool_selection_mac")
        } else {
            fl!("tool_selection")
        }
    }
    fn icon_name(&self) -> String {
        str!("cursor")
    }
    fn accel(&self) -> Option<char> {
        Some('S')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/selection".to_string())
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
                server_ctx.curr_map_tool_type = MapToolType::Selection;

                return true;
            }
            DeActivate => {
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                server_ctx.curr_map_tool_type = MapToolType::General;
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
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if !server_ctx.hover_is_empty() {
                    let prev = map.clone();
                    let arrays = server_ctx.hover_to_arrays();
                    if ui.shift {
                        // Add
                        map.add_to_selection(arrays.0, arrays.1, arrays.2);
                    } else if ui.alt {
                        // Remove
                        map.remove_from_selection(arrays.0, arrays.1, arrays.2);
                    } else {
                        // Replace
                        map.selected_vertices = arrays.0;
                        map.selected_linedefs = arrays.1;
                        map.selected_sectors = arrays.2;
                    }

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

                self.click_pos = Vec2::new(coord.x as f32, coord.y as f32);
                self.rectangle_undo_map = map.clone();
            }
            MapDragged(coord) => {
                if server_ctx.editor_view_mode != EditorViewMode::D2 {
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

                        let selection =
                            server_ctx.geometry_in_rectangle(top_left, bottom_right, map);

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
                            map.selected_linedefs = selection.1;
                            map.selected_sectors = selection.2;
                        }
                    }
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                }
            }
            MapUp(_) => {
                if server_ctx.editor_view_mode != EditorViewMode::D2 {
                    if map.curr_rectangle.is_some() {
                        map.curr_rectangle = None;

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
                }
            }
            MapHover(coord) => {
                if self.hud.hovered(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();
                    server_ctx.hover = server_ctx.geometry_at(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                    );

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
                if !map.selected_vertices.is_empty()
                    || !map.selected_linedefs.is_empty()
                    || !map.selected_sectors.is_empty()
                {
                    let prev = map.clone();

                    let vertices = map.selected_vertices.clone();
                    let linedefs = map.selected_linedefs.clone();
                    let sectors = map.selected_sectors.clone();

                    map.delete_elements(&vertices, &linedefs, &sectors);
                    map.selected_vertices.clear();
                    map.selected_linedefs.clear();
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
                }
            }
            MapEscape => {
                // Hover is empty, check if we need to clear selection
                if !map.selected_vertices.is_empty()
                    || !map.selected_linedefs.is_empty()
                    || !map.selected_sectors.is_empty()
                {
                    let prev = map.clone();

                    map.selected_vertices.clear();
                    map.selected_linedefs.clear();
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
                }
                server_ctx.profile_view = None;
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
        self.hud.draw(buffer, map, ctx, server_ctx, None, assets);
    }
    /*
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
            _ => {}
        }
        redraw
    }*/
}
