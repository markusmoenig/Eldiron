//! Keyboard-only gameplay in the native authoring viewport. Authoring pointer
//! gestures stay with their existing tool; accepted controls belong to Play.
use super::world::GameInputFrame;
use super::*;
use rusterix::input_binding::ClientCommandBinding;
use theframework::platform::*;

#[derive(Default)]
pub(super) struct NativeGameInput {
    frame: Option<GameInputFrame>,
}
impl Editor {
    pub(crate) fn native_game_input_owns_viewport(&self) -> bool {
        self.native_viewport.enabled
            && self.server_ctx.game_input_mode
            && !self.server_ctx.game_mode
            && !self.server_ctx.text_game_mode
            && self.server_ctx.pc.is_region()
            && self.play_session.as_ref().is_some_and(|play| {
                self.documents.active_id() == Some(play.document)
            })
            && RUSTERIX.read().unwrap().server.state != rusterix::ServerState::Off
            && self.native_world_owns_viewport()
    }
    pub(crate) fn sync_native_game_input(&mut self, ui: &mut TheUI, ctx: &TheContext) {
        let enabled = self.native_game_input_owns_viewport();
        // This is only an ephemeral compatibility routing claim, not document
        // data. Unsupported contexts keep their explicitly unmigrated adapter.
        self.server_ctx.native_game_input = enabled;
        let frame = if enabled
            && self.native_viewport.window_focused != Some(false)
            && ui.dialog.is_none()
            && ui.popover.is_none()
            && ui.context_menu.is_none()
        {
            self.native_game_input_frame(ui, ctx).filter(|frame| {
                ctx.ui
                    .focus
                    .as_ref()
                    .is_some_and(|id| id.uuid == frame.view().uuid)
                    && !text_focused(ui, ctx)
            })
        } else {
            None
        };
        let input = &mut self.native_viewport.game_input;
        if input.frame != frame {
            if let Some(previous) = &input.frame
                && let Some(play) = &mut self.play_session
                && previous.owns_runtime(play.pump.session_id())
            {
                play.cancel_input(&mut RUSTERIX.write().unwrap().server);
            }
            input.frame = frame;
        }
    }
    pub(crate) fn native_game_input_raw(
        &mut self,
        event: &InputEvent,
        session: &mut UiSession,
    ) -> Option<InputResponse> {
        if let InputEvent::Focus(focused) = event {
            self.native_viewport.window_focused = Some(*focused);
        }
        self.sync_native_game_input(&mut session.ui, &session.context);
        if !self.server_ctx.native_game_input {
            return None;
        }
        let view = session.ui.get_render_view("PolyView")?.id().clone();
        if session
            .context
            .ui
            .focus
            .as_ref()
            .is_none_or(|id| id.uuid != view.uuid)
            || session.ui.dialog.is_some()
            || session.ui.popover.is_some()
            || session.ui.context_menu.is_some()
            || text_focused(&mut session.ui, &session.context)
        {
            return None;
        }
        if !matches!(
            event,
            InputEvent::Key { .. }
                | InputEvent::Text(_)
                | InputEvent::Composition { .. }
                | InputEvent::TextFor { .. }
                | InputEvent::CompositionFor { .. }
        ) {
            return None;
        }
        let declined = || InputResponse {
            consumed: true,
            redraw: true,
        };
        if self.native_viewport.game_input.frame.is_none() {
            return Some(declined());
        }
        let InputEvent::Key {
            physical,
            logical,
            pressed,
            repeat,
        } = event
        else {
            // Physical controls never reinterpret composed authoring text as
            // shortcuts or send the key a second time through TheUI.
            return Some(InputResponse {
                consumed: true,
                redraw: false,
            });
        };
        if *physical == PhysicalKey::Escape && *pressed && self.native_world_gesture_active() {
            self.cancel_native_world_gesture(&mut session.ui, &mut session.context);
            return Some(declined());
        }
        let Some(key) = eldiron_presentation::player_binding_key(*physical, logical) else {
            return Some(declined());
        };
        let mut bridge = RUSTERIX.write().unwrap();
        let play = self.play_session.as_mut()?;
        // Releases remain authoritative even after a modifier was pressed.
        if !pressed && !repeat {
            if let Some(action) = play.input.controls.control_key(&key, false) {
                let result = play.submit_action(&mut bridge.server, action);
                if let Err(error) = result {
                    session
                        .context
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), error));
                }
            }
            return Some(declined());
        }
        if session.ui.ctrl || session.ui.logo || session.ui.alt {
            return None;
        }
        if bridge.server.state != rusterix::ServerState::Running || *repeat || !pressed {
            return Some(declined());
        }
        let binding = play.input.controls.binding_for_key(&key);
        if let Some(ClientCommandBinding::Control(_)) = binding {
            if let Some(action) = play.input.controls.control_key(&key, true) {
                if let Err(error) = play.submit_action(&mut bridge.server, action) {
                    session
                        .context
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), error));
                }
            }
            return Some(declined());
        }
        drop(bridge);
        if let Some(binding) = binding {
            if let Err(error) = self
                .sync_native_play(
                    &mut session.ui,
                    &mut session.context,
                    std::time::Duration::ZERO,
                )
                .and_then(|_| self.activate_native_play_binding(binding))
            {
                session
                    .context
                    .ui
                    .send(TheEvent::SetStatusText(TheId::empty(), error));
            }
        }
        Some(declined())
    }
}

fn text_focused(ui: &mut TheUI, ctx: &TheContext) -> bool {
    let Some(id) = ctx.ui.focus.as_ref() else {
        return false;
    };
    ui.get_widget_id(id.uuid)
        .is_some_and(|widget| widget.supports_text_input())
}

#[cfg(test)]
impl Editor {
    pub(crate) fn native_runtime_input_for_test(
        &self,
    ) -> Option<(Uuid, eldiron_runtime::PlayerInputStats)> {
        let play = self.play_session.as_ref()?;
        Some((play.pump.session_id(), play.input.input_stats()))
    }
    pub(crate) fn native_game_input_claim_for_test(&self) -> bool {
        self.server_ctx.native_game_input
    }
}
