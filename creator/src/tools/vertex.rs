use crate::prelude::*;
use shared::server::prelude::MapToolType;
use ToolEvent::*;

use crate::editor::UNDOMANAGER;

pub struct VertexTool {
    id: TheId,
    click_pos: Vec2f,
    rectangle_undo_map: Map,
}

impl Tool for VertexTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Vertex Tool"),
            click_pos: Vec2f::zero(),
            rectangle_undo_map: Map::default(),
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
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let _coord = match tool_event {
            TileDown(_, c) => c,
            TileDrag(_, c) => c,
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
                server_ctx.curr_map_tool_type = MapToolType::Vertex;

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.selected_linedefs.clear();
                    region.map.selected_sectors.clear();
                    server.update_region(region);
                }

                return true;
            }
            DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                return true;
            }
            _ => {
                return false;
            }
        };
        false
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        match event {
            TheEvent::RenderViewClicked(id, coord) => {
                if id.name == "PolyView" {
                    if server_ctx.hover.0.is_some() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let prev = region.map.clone();
                            let mut changed = false;

                            if ui.shift {
                                // Add
                                if let Some(v) = server_ctx.hover.0 {
                                    if !region.map.selected_vertices.contains(&v) {
                                        region.map.selected_vertices.push(v);
                                        changed = true;
                                    }
                                }
                            } else if ui.alt {
                                // Subtract
                                if let Some(v) = server_ctx.hover.0 {
                                    region
                                        .map
                                        .selected_vertices
                                        .retain(|&selected| selected != v);
                                    changed = true;
                                }
                            } else {
                                // Replace
                                if let Some(v) = server_ctx.hover.0 {
                                    region.map.selected_vertices = vec![v];
                                    changed = true;
                                } else {
                                    region.map.selected_vertices.clear();
                                    changed = true;
                                }
                            }

                            if changed {
                                let undo = RegionUndoAtom::MapEdit(
                                    Box::new(prev),
                                    Box::new(region.map.clone()),
                                );

                                UNDOMANAGER
                                    .lock()
                                    .unwrap()
                                    .add_region_undo(&region.id, undo, ctx);

                                server.update_region(region);
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Map Selection Changed"),
                                    TheValue::Empty,
                                ));
                            }
                        }
                    }

                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        self.click_pos = vec2f(coord.x as f32, coord.y as f32);
                        self.rectangle_undo_map = region.map.clone();
                    }
                    redraw = true;
                }
            }
            TheEvent::RenderViewHoverChanged(id, coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            let h = server_ctx.geometry_at(
                                vec2f(dim.width as f32, dim.height as f32),
                                vec2f(coord.x as f32, coord.y as f32),
                                &region.map,
                            );
                            server_ctx.hover.0 = h.0;

                            let cp = server_ctx.local_to_map_grid(
                                vec2f(dim.width as f32, dim.height as f32),
                                vec2f(coord.x as f32, coord.y as f32),
                                &region.map,
                                region.map.subdivisions,
                            );

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Cursor Pos Changed"),
                                TheValue::Float2(cp),
                            ));
                            server_ctx.hover_cursor = Some(cp);
                        }
                    }
                }
                redraw = true;
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            let click_pos = server_ctx.local_to_map_grid(
                                vec2f(dim.width as f32, dim.height as f32),
                                self.click_pos,
                                &region.map,
                                region.map.subdivisions,
                            );
                            let drag_pos = server_ctx.local_to_map_grid(
                                vec2f(dim.width as f32, dim.height as f32),
                                vec2f(coord.x as f32, coord.y as f32),
                                &region.map,
                                region.map.subdivisions,
                            );

                            let top_left = Vec2f::new(
                                click_pos.x.min(drag_pos.x),
                                click_pos.y.min(drag_pos.y),
                            );
                            let bottom_right = Vec2f::new(
                                click_pos.x.max(drag_pos.x),
                                click_pos.y.max(drag_pos.y),
                            );

                            let mut selection = server_ctx.geometry_in_rectangle(
                                top_left,
                                bottom_right,
                                &region.map,
                            );

                            selection.1 = vec![];
                            selection.2 = vec![];

                            region.map = self.rectangle_undo_map.clone();
                            region.map.curr_rectangle =
                                Some((self.click_pos, vec2f(coord.x as f32, coord.y as f32)));

                            if ui.shift {
                                // Add
                                region
                                    .map
                                    .add_to_selection(selection.0, selection.1, selection.2);
                            } else if ui.alt {
                                // Remove
                                region.map.remove_from_selection(
                                    selection.0,
                                    selection.1,
                                    selection.2,
                                );
                            } else {
                                // Replace
                                region.map.selected_vertices = selection.0;
                            }

                            server.update_region(region);
                        }
                    }
                }
                redraw = true;
            }
            TheEvent::RenderViewUp(id, _coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if region.map.curr_rectangle.is_some() {
                            region.map.curr_rectangle = None;
                            server.update_region(region);

                            let undo = RegionUndoAtom::MapEdit(
                                Box::new(self.rectangle_undo_map.clone()),
                                Box::new(region.map.clone()),
                            );

                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);

                            server.update_region(region);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Map Selection Changed"),
                                TheValue::Empty,
                            ));
                        }
                    }
                }
                redraw = true;
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                #[allow(clippy::collapsible_if)]
                if *code == TheKeyCode::Escape {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        // Hover is empty, check if we need to clear selection
                        if !region.map.selected_vertices.is_empty() {
                            let prev = region.map.clone();

                            region.map.selected_vertices.clear();

                            let undo = RegionUndoAtom::MapEdit(
                                Box::new(prev),
                                Box::new(region.map.clone()),
                            );

                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);

                            server.update_region(region);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Map Selection Changed"),
                                TheValue::Empty,
                            ));
                        }
                    }
                }
                if *code == TheKeyCode::Delete {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if !region.map.selected_vertices.is_empty() {
                            let prev = region.map.clone();
                            let vertices = region.map.selected_vertices.clone();

                            #[allow(clippy::useless_vec)]
                            region.map.delete_elements(&vertices, &vec![], &vec![]);
                            region.map.selected_vertices.clear();

                            let undo = RegionUndoAtom::MapEdit(
                                Box::new(prev),
                                Box::new(region.map.clone()),
                            );

                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);

                            server.update_region(region);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Map Selection Changed"),
                                TheValue::Empty,
                            ));
                        }
                    }
                }
                redraw = true;
            }
            _ => {}
        }
        redraw
    }
}
