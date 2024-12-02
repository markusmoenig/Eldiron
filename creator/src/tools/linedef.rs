use crate::prelude::*;
use shared::server::prelude::MapToolType;
use ToolEvent::*;

use crate::editor::UNDOMANAGER;

pub struct LinedefTool {
    id: TheId,
    click_pos: Vec2f,
    rectangle_undo_map: Map,
}

impl Tool for LinedefTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Linedef Tool"),
            click_pos: Vec2f::zero(),
            rectangle_undo_map: Map::default(),
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
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.clear_temp();
                    server.update_region(region);
                }
                return true;
            }
            _ => {
                return false;
            }
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
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                if *code == TheKeyCode::Escape {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.map.clear_temp();
                        server.update_region(region);
                    }
                    redraw = true;
                } else {
                    redraw = false;
                }
            }
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

            _ => {}
        }
        redraw
    }
}
