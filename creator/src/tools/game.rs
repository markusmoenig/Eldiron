use crate::prelude::*;
//use ToolEvent::*;

pub struct GameTool {
    id: TheId,
}

impl Tool for GameTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Game Tool"),
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
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            ToolEvent::Activate => {
                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_mode(TheSharedVLayoutMode::Top);
                }
                true
            }
            ToolEvent::DeActivate => {
                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_mode(TheSharedVLayoutMode::Shared);
                }
                true
            }
            _ => false,
        }
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
