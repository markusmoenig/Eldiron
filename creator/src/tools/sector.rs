use crate::prelude::*;
use shared::server::prelude::MapToolType;
use ToolEvent::*;

use crate::editor::UNDOMANAGER;

pub struct SectorTool {
    id: TheId,
}

impl Tool for SectorTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Sector Tool"),
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
                server_ctx.curr_map_tool_type = MapToolType::Sector;

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.selected_vertices.clear();
                    region.map.selected_linedefs.clear();
                    server.update_region(region);
                }

                return true;
            }
            _ => {
                server_ctx.curr_map_tool_type = MapToolType::General;

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
                    let mut set_current_gid_pos = true;

                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            let grid_pos = server_ctx.local_to_map_grid(
                                vec2f(dim.width as f32, dim.height as f32),
                                vec2f(coord.x as f32, coord.y as f32),
                                &region.map,
                                region.map.subdivisions,
                            );

                            if let Some(curr_grid_pos) = region.map.curr_grid_pos {
                                if curr_grid_pos.x != grid_pos.x || curr_grid_pos.y != grid_pos.y {
                                    let prev = region.map.clone();

                                    let start_vertex =
                                        region.map.add_vertex_at(curr_grid_pos.x, curr_grid_pos.y);
                                    let end_vertex =
                                        region.map.add_vertex_at(grid_pos.x, grid_pos.y);

                                    // Returns id of linedef and optional id of new sector if polygon closes
                                    let ids = region.map.create_linedef(start_vertex, end_vertex);

                                    if ids.1.is_some() {
                                        // When we close a polygon delete the temporary data
                                        region.map.clear_temp();
                                        set_current_gid_pos = false;
                                    }

                                    server.update_region(region);

                                    let undo = RegionUndoAtom::MapEdit(
                                        Box::new(prev),
                                        Box::new(region.map.clone()),
                                    );

                                    UNDOMANAGER
                                        .lock()
                                        .unwrap()
                                        .add_region_undo(&region.id, undo, ctx);
                                }
                            }

                            if set_current_gid_pos {
                                region.map.curr_grid_pos = Some(grid_pos);
                            }
                            redraw = true;
                        }
                    }
                }
            }
            TheEvent::RenderViewDragged(_id, _coord) => {}

            TheEvent::RenderViewHoverChanged(id, coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.map.curr_mouse_pos = Some(vec2f(coord.x as f32, coord.y as f32));
                        server.update_region(region);
                        redraw = true;
                    }
                }
            }
            TheEvent::RenderViewScrollBy(id, coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if ui.ctrl || ui.logo {
                            region.map.grid_size += coord.y as f32;
                            region.map.grid_size = clamp(region.map.grid_size, 5.0, 100.0);
                        } else {
                            region.map.offset += Vec2f::new(-coord.x as f32, coord.y as f32);
                        }
                        server.update_region(region);
                        redraw = true;
                    }
                }
            }

            _ => {}
        }
        redraw
    }
}
