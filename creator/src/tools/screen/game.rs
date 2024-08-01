use crate::prelude::*;
use ToolEvent::*;

pub struct ScreenGameTool {
    id: TheId,
}

impl Tool for ScreenGameTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Screen Game Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Game Tool (G). If the server is running input events are send to the game.")
    }
    fn icon_name(&self) -> String {
        str!("input")
    }
    fn accel(&self) -> Option<char> {
        Some('g')
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
        match tool_event {
            TileDown(coord, _) => {
                if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                    client.touch_down(
                        &server_ctx.curr_screen,
                        vec2i(coord.x * screen.grid_size, coord.y * screen.grid_size),
                    );
                }
            }
            TileDrag(coord, _) => {
                if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                    client.touch_down(
                        &server_ctx.curr_screen,
                        vec2i(coord.x * screen.grid_size, coord.y * screen.grid_size),
                    );
                }
            }
            TileUp => {
                client.touch_up(&server_ctx.curr_screen);
            }
            Activate => {
                return true;
            }
            _ => {
                return false;
            }
        };

        true
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        server: &mut Server,
        client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        #[allow(clippy::single_match)]
        match event {
            TheEvent::KeyDown(key) => {
                if server.state == ServerState::Running {
                    if let Some(c) = key.to_char() {
                        client.key_down(&server_ctx.curr_screen, c);
                    }
                }
            }
            _ => {}
        }

        false
    }
}
