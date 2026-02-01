use crate::{editor::RUSTERIX, prelude::*};
use MapEvent::*;
use rusterix::{EntityAction, Value};
use std::sync::Mutex;
use theframework::prelude::*;

pub struct GameTool {
    id: TheId,

    right: Option<Mutex<Box<TheCanvas>>>,
    toolbar: Option<Mutex<Box<TheCanvas>>>,
}

impl Tool for GameTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Game Tool"),

            right: None,
            toolbar: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        fl!("tool_game")
    }
    fn icon_name(&self) -> String {
        str!("joystick")
    }
    fn accel(&self) -> Option<char> {
        Some('G')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/game".to_string())
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            ToolEvent::Activate => {
                self.toolbar = None;
                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_mode(TheSharedVLayoutMode::Top);
                    if let Some(canvas) = layout.get_canvas_mut(0) {
                        if let Some(tool) = canvas.bottom.take() {
                            self.toolbar = Some(Mutex::new(tool));
                        }
                    }
                }
                server_ctx.curr_map_tool_type = MapToolType::Game;
                server_ctx.game_mode = true;

                if let Some(right) = ui.canvas.right.take() {
                    self.right = Some(Mutex::new(right));
                }
                ctx.ui.redraw_all = true;
                ctx.ui.relayout = true;

                true
            }
            ToolEvent::DeActivate => {
                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_mode(TheSharedVLayoutMode::Shared);
                    if let Some(canvas) = layout.get_canvas_mut(0) {
                        if let Some(tool) = &mut self.toolbar {
                            let lock = tool.get_mut().unwrap();
                            let boxed_canvas: Box<TheCanvas> =
                                std::mem::replace(&mut *lock, Box::new(TheCanvas::default()));
                            canvas.bottom = Some(boxed_canvas);
                        }
                    }
                }

                if let Some(right) = &mut self.right {
                    let lock = right.get_mut().unwrap();
                    let boxed_canvas: Box<TheCanvas> =
                        std::mem::replace(&mut *lock, Box::new(TheCanvas::default()));
                    ui.canvas.right = Some(boxed_canvas);
                    ctx.ui.redraw_all = true;
                    ctx.ui.relayout = true;
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
        ctx: &mut TheContext,
        map: &mut Map,
        _server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match map_event {
            MapClicked(coord) => {
                let mut rusterix = RUSTERIX.write().unwrap();
                let is_running = rusterix.server.state == rusterix::ServerState::Running;

                if is_running {
                    if let Some(action) = rusterix.client.touch_down(coord, map) {
                        rusterix.server.local_player_action(action);
                    }
                }
            }
            MapDragged(coord) => {
                let mut rusterix = RUSTERIX.write().unwrap();
                let is_running = rusterix.server.state == rusterix::ServerState::Running;

                let is_inside = rusterix.client.is_inside_game(coord);
                if is_running && is_inside {
                    ctx.set_cursor_visible(false);
                    rusterix.client_touch_dragged(coord, map);
                } else {
                    ctx.set_cursor_visible(true);
                }
            }
            MapUp(coord) => {
                let mut rusterix = RUSTERIX.write().unwrap();
                let is_running = rusterix.server.state == rusterix::ServerState::Running;

                if is_running {
                    rusterix.client.touch_up(coord, map);
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
        ctx: &mut TheContext,
        project: &mut Project,
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
            TheEvent::RenderViewHoverChanged(id, coord) => {
                if id.name == "PolyView" {
                    let mut rusterix = RUSTERIX.write().unwrap();
                    let is_running = rusterix.server.state == rusterix::ServerState::Running;

                    let is_inside = rusterix.client.is_inside_game(*coord);
                    if is_running && is_inside {
                        ctx.set_cursor_visible(false);

                        for region in &project.regions {
                            if region.map.name == rusterix.client.current_map {
                                rusterix.client_touch_hover(*coord, &region.map);
                                break;
                            }
                        }
                    } else {
                        ctx.set_cursor_visible(true);
                    }
                }
            }
            _ => {}
        }

        false
    }
}
