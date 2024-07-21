use crate::prelude::*;
use ToolEvent::*;

pub struct ScreenTileDrawerTool {
    id: TheId,
}

impl Tool for ScreenTileDrawerTool {
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
        str!("Tile Drawer Tool. Draw tiles on the current widget.")
    }
    fn icon_name(&self) -> String {
        str!("pen")
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c) => c,
            TileDrag(c) => c,
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

        if let Some(curr_tile_id) = server_ctx.curr_tile_id {
            if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                if !server_ctx.screen_editor_mode_foreground {
                    screen.add_background_tile((coord.x, coord.y), curr_tile_id);
                } else {
                    screen.add_foreground_tile((coord.x, coord.y), curr_tile_id);
                }
                client.update_screen(screen);
            }
        }

        true
    }
}
