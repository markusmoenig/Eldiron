use crate::prelude::*;
use ToolEvent::*;

pub struct ScreenEraserTool {
    id: TheId,
}

impl Tool for ScreenEraserTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Eraser Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Eraser Tool. Erase tiles drawn on widgets.")
    }
    fn icon_name(&self) -> String {
        str!("eraser")
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c, _) => c,
            TileDrag(c, _) => c,
            Activate => {
                return true;
            }
            _ => {
                return false;
            }
        };

        if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
            if !server_ctx.screen_editor_mode_foreground {
                screen.erase_background_tile((coord.x, coord.y));
            } else {
                screen.erase_foreground_tile((coord.x, coord.y));
            }
            client.update_screen(screen);
        }

        true
    }
}
