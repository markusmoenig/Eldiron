use crate::{editor::RUSTERIX, prelude::*};
use MapEvent::*;
use rusterix::{EntityAction, PlayerCamera, Value};
use std::sync::Mutex;
use theframework::prelude::*;

pub struct GameTool {
    id: TheId,

    right: Option<Mutex<Box<TheCanvas>>>,
    toolbar: Option<Mutex<Box<TheCanvas>>>,
    editor_routed_prev_camera: Option<PlayerCamera>,
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
            editor_routed_prev_camera: None,
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
        project: &mut Project,
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

                // If editor-routed input temporarily forced a different camera mapping
                // (e.g. Iso), restore the previous game camera mapping when entering
                // actual Game Tool mode.
                if RUSTERIX.read().unwrap().server.state == rusterix::ServerState::Running {
                    let map_camera_to_player_camera = |mode: MapCamera| -> PlayerCamera {
                        match mode {
                            MapCamera::TwoD => PlayerCamera::D2,
                            MapCamera::ThreeDIso => PlayerCamera::D3Iso,
                            MapCamera::ThreeDFirstPerson => PlayerCamera::D3FirstP,
                        }
                    };
                    let restore_camera = if let Some(prev) = self.editor_routed_prev_camera.take() {
                        prev
                    } else {
                        let rusterix = RUSTERIX.read().unwrap();
                        let by_active_game_widget =
                            rusterix.client.active_game_widget_camera_mode();
                        let by_running_map = project
                            .regions
                            .iter()
                            .find(|r| r.map.name == rusterix.client.current_map)
                            .map(|r| map_camera_to_player_camera(r.map.camera));
                        let by_editor_region = project
                            .get_region(&server_ctx.curr_region)
                            .map(|r| map_camera_to_player_camera(r.map.camera));
                        by_active_game_widget
                            .or(by_running_map)
                            .or(by_editor_region)
                            .unwrap_or_else(|| rusterix.player_camera.clone())
                    };

                    RUSTERIX
                        .write()
                        .unwrap()
                        .server
                        .local_player_action(EntityAction::SetPlayerCamera(restore_camera));
                } else {
                    self.editor_routed_prev_camera = None;
                }

                if let Some(right) = ui.canvas.right.take() {
                    self.right = Some(Mutex::new(right));
                }
                ctx.ui.redraw_all = true;
                ctx.ui.relayout = true;

                true
            }
            ToolEvent::DeActivate => {
                ctx.set_cursor_visible(true);

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
        server_ctx: &mut ServerContext,
    ) -> bool {
        let editor_view_to_player_camera = |mode: EditorViewMode| match mode {
            EditorViewMode::D2 => PlayerCamera::D2,
            EditorViewMode::FirstP => PlayerCamera::D3FirstP,
            EditorViewMode::Iso | EditorViewMode::Orbit => PlayerCamera::D3Iso,
        };
        let key_to_action = |c: char| -> Option<EntityAction> {
            match c.to_ascii_lowercase() {
                'w' => Some(EntityAction::Forward),
                'a' => Some(EntityAction::Left),
                's' => Some(EntityAction::Backward),
                'd' => Some(EntityAction::Right),
                _ => None,
            }
        };

        #[allow(clippy::single_match)]
        match event {
            TheEvent::KeyDown(TheValue::Char(char)) => {
                let mut rusterix = crate::editor::RUSTERIX.write().unwrap();
                if rusterix.server.state == rusterix::ServerState::Running {
                    if server_ctx.game_input_mode && !server_ctx.game_mode {
                        // While routing editor input, temporarily enforce movement mapping from
                        // the current editor camera mode (D2 / Iso / FirstP).
                        if self.editor_routed_prev_camera.is_none() {
                            self.editor_routed_prev_camera = Some(rusterix.player_camera.clone());
                        }
                        let camera = editor_view_to_player_camera(server_ctx.editor_view_mode);
                        rusterix
                            .server
                            .local_player_action(EntityAction::SetPlayerCamera(camera));

                        // Use action mapping so editor routed input behaves like in-game controls.
                        let mut action = rusterix
                            .client
                            .user_event("key_down".into(), Value::Str(char.to_string()));
                        if matches!(action, EntityAction::Off)
                            && let Some(fallback) = key_to_action(*char)
                        {
                            action = fallback;
                        }
                        rusterix.server.local_player_action(action);
                    } else {
                        let action = rusterix
                            .client
                            .user_event("key_down".into(), Value::Str(char.to_string()));

                        rusterix.server.local_player_action(action);
                    }
                }
            }
            TheEvent::KeyUp(TheValue::Char(char)) => {
                let mut rusterix = crate::editor::RUSTERIX.write().unwrap();
                if rusterix.server.state == rusterix::ServerState::Running {
                    if server_ctx.game_input_mode && !server_ctx.game_mode {
                        let mut action = rusterix
                            .client
                            .user_event("key_up".into(), Value::Str(char.to_string()));
                        if matches!(action, EntityAction::Off) && key_to_action(*char).is_some() {
                            action = EntityAction::Off;
                        }
                        rusterix.server.local_player_action(action);
                        if let Some(prev) = self.editor_routed_prev_camera.take() {
                            rusterix
                                .server
                                .local_player_action(EntityAction::SetPlayerCamera(prev));
                        }
                    } else {
                        let action = rusterix
                            .client
                            .user_event("key_up".into(), Value::Str(char.to_string()));
                        rusterix.server.local_player_action(action);
                    }
                }
            }
            TheEvent::RenderViewHoverChanged(id, coord) => {
                // Do not run "real game play" hover/cursor logic when only routing input
                // from the editor view into the running server.
                if server_ctx.game_input_mode && !server_ctx.game_mode {
                    return false;
                }
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
