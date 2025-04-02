use crate::{editor::RUSTERIX, prelude::*};
use rusterix::{EntityAction, Value};
use theframework::prelude::*;
use MapEvent::*;

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
                server_ctx.curr_map_tool_type = MapToolType::Game;
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

    fn map_event(
        &mut self,
        map_event: MapEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _map: &mut Map,
        _server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        match map_event {
            MapClicked(coord) => {
                let mut rusterix = RUSTERIX.write().unwrap();
                let is_running = rusterix.server.state == rusterix::ServerState::Running;

                if is_running {
                    if let Some(action) = rusterix.client.touch_down(coord) {
                        rusterix.server.local_player_action(action);
                    }
                }
            }
            MapUp(coord) => {
                let mut rusterix = RUSTERIX.write().unwrap();
                let is_running = rusterix.server.state == rusterix::ServerState::Running;

                if is_running {
                    rusterix.client.touch_up(coord);
                    rusterix.server.local_player_action(EntityAction::Off);
                }
            }
            _ => {}
        }

        None
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
                let mut rusterix = crate::editor::RUSTERIX.write().unwrap();
                if rusterix.server.state == rusterix::ServerState::Running {
                    let action = rusterix
                        .client
                        .user_event("key_down".into(), Value::Str(char.to_string()));

                    rusterix.server.local_player_action(action);
                }
            }
            TheEvent::KeyUp(TheValue::Char(char)) => {
                let mut rusterix = crate::editor::RUSTERIX.write().unwrap();
                if rusterix.server.state == rusterix::ServerState::Running {
                    let action = rusterix
                        .client
                        .user_event("key_up".into(), Value::Str(char.to_string()));
                    rusterix.server.local_player_action(action);
                }
            }
            _ => {}
        }

        false
    }
}
