use crate::prelude::*;
use rusterix::Value;
//use ToolEvent::*;
use theframework::prelude::*;

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
        str!("joystick")
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
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            ToolEvent::Activate => {
                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_mode(TheSharedVLayoutMode::Top);
                }
                server_ctx.game_mode = true;
                true
            }
            ToolEvent::DeActivate => {
                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_mode(TheSharedVLayoutMode::Shared);
                }
                server_ctx.game_mode = false;
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
        _server_ctx: &mut ServerContext,
    ) -> bool {
        #[allow(clippy::single_match)]
        match event {
            TheEvent::KeyDown(TheValue::Char(char)) => {
                let mut rusterix = crate::editor::RUSTERIX.lock().unwrap();
                if rusterix.server.state == rusterix::ServerState::Running {
                    rusterix
                        .server
                        .local_player_event("key_down".into(), Value::Str(char.to_string()));
                }
            }
            TheEvent::KeyUp(TheValue::Char(char)) => {
                let mut rusterix = crate::editor::RUSTERIX.lock().unwrap();
                if rusterix.server.state == rusterix::ServerState::Running {
                    rusterix
                        .server
                        .local_player_event("key_up".into(), Value::Str(char.to_string()));
                }
            }
            _ => {}
        }

        false
    }
}
