use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{PRERENDERTHREAD, RENDERER, TILEDRAWER, TILEFXEDITOR, UNDOMANAGER};

pub struct TileDrawerTool {
    id: TheId,
}

impl Tool for TileDrawerTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Tile Drawer Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Pen Tool (P). Draw tiles.")
    }
    fn icon_name(&self) -> String {
        str!("pen")
    }
    fn accel(&self) -> Option<char> {
        Some('p')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c, _) => c,
            TileDrag(c, _) => c,
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

                return true;
            }
            _ => {
                return false;
            }
        };

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
        false
    }
}
