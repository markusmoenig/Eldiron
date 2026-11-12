//! One pointer owner across authored controls. Application/world input only
//! receives events that the screen has declined; text never becomes a shortcut.
use super::*;
use theframework::platform::{InputEvent, InputResponse, PointerButton};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PointerCapture {
    Button(u32),
    HeldButton(u32),
    Text,
    Choice(u32),
    Message(u32),
    Cancelled,
}
#[derive(Default)]
pub struct ScreenInputResponse {
    pub input: InputResponse,
    pub text: Option<ScreenTextResponse>,
    pub effects: Vec<ScreenEffect>,
    /// Stop held world movement when entering a UI interaction or losing focus.
    pub cancel_controls: bool,
}
impl ScreenInputResponse {
    pub(super) fn handled(redraw: bool) -> Self {
        Self {
            input: InputResponse {
                consumed: true,
                redraw,
            },
            ..Default::default()
        }
    }
    fn text(mut text: ScreenTextResponse) -> Self {
        Self {
            input: text.input,
            effects: std::mem::take(&mut text.effects),
            text: Some(text),
            ..Default::default()
        }
    }
}
impl GameScreenController {
    fn inside_screen(&self, position: [f32; 2]) -> bool {
        position.iter().all(|v| v.is_finite())
            && Uv::new(
                0.,
                0.,
                self.elements.layout.viewport.x as f32,
                self.elements.layout.viewport.y as f32,
            )
            .contains(position)
    }
    pub fn captures_world_input(&self) -> bool {
        self.control_capture()
            .is_some_and(|capture| !matches!(capture, ScreenCapture::World(_)))
    }
    /// Coordinates are in the same logical screen space used by `paint`.
    /// Modal choices precede buttons, fields and world input, matching paint order.
    /// A captured drag/release keeps its original owner even outside its bounds.
    pub fn input(
        &mut self,
        world: &RuntimeView<'_>,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> Result<ScreenInputResponse, String> {
        let old_focus = self.focus;
        let held = self
            .keyboard_capture
            .is_some_and(|c| c.held && !c.cancelled);
        self.prepare_focus(fonts);
        let mut response = self.input_inner(world, fonts, event)?;
        if let Some(theframework::text_edit::EditorAction::FocusNext { backwards }) =
            response.text.as_ref().and_then(|text| text.editor.action)
        {
            self.focus_next(world, fonts, backwards);
        }
        response.cancel_controls |=
            old_focus != self.focus || (held && self.keyboard_capture.is_some_and(|c| c.cancelled));
        Ok(response)
    }
    fn input_inner(
        &mut self,
        world: &RuntimeView<'_>,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> Result<ScreenInputResponse, String> {
        if let Some(target) = event.text_target() {
            if self.text_input_state(fonts).map(|state| state.target) != Some(target) {
                return Ok(ScreenInputResponse::default());
            }
        }
        if matches!(event, InputEvent::Focus(false)) {
            self.world_texts.input(event);
            self.pointer_capture = None;
            self.keyboard_capture = None;
            self.focus_control(fonts, None);
            self.suppress_activation_slash = false;
            self.pressed_button = None;
            self.elements.release_button_state();
            self.geometry = None;
            self.choice_input_event(fonts, event);
            let mut response = ScreenInputResponse::text(self.text_input_event(fonts, event));
            response.input.redraw = true;
            response.cancel_controls = true;
            return Ok(response);
        }
        if let InputEvent::Text(text) | InputEvent::TextFor { text, .. } = event {
            if std::mem::take(&mut self.suppress_activation_slash) && text == "/" {
                return Ok(ScreenInputResponse::handled(false));
            }
        } else if matches!(
            event,
            InputEvent::Key { pressed: true, .. } | InputEvent::PointerButton { pressed: true, .. }
        ) {
            self.suppress_activation_slash = false;
        }
        if let InputEvent::PointerMoved { position } = event {
            let moved = self.cursor != *position;
            self.cursor = *position;
            match self.pointer_capture {
                Some(
                    PointerCapture::Button(_)
                    | PointerCapture::HeldButton(_)
                    | PointerCapture::Message(_)
                    | PointerCapture::Cancelled,
                ) => {
                    return Ok(ScreenInputResponse::handled(moved));
                }
                Some(PointerCapture::Text) => {
                    let mut response =
                        ScreenInputResponse::text(self.text_input_event(fonts, event));
                    response.input.consumed = true;
                    return Ok(response);
                }
                _ => {}
            }
        }
        if let InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            position,
        } = event
        {
            self.cursor = *position;
            match self.pointer_capture.take() {
                Some(PointerCapture::Button(id)) => {
                    self.pressed_button = None;
                    let top = screen_model::topmost_button_at(
                        &self.elements.button_widgets,
                        &self.elements.widgets_to_hide,
                        scenevm::prelude::Vec2::new(position[0], position[1]),
                    );
                    let activation = if top == Some(id) && self.inside_screen(*position) {
                        self.activate_button(world, id)?
                    } else {
                        ScreenActivation::default()
                    };
                    self.elements.release_button_state();
                    self.geometry = None;
                    return Ok(ScreenInputResponse {
                        effects: activation.effects,
                        ..ScreenInputResponse::handled(true)
                    });
                }
                Some(PointerCapture::HeldButton(_)) => {
                    self.pressed_button = None;
                    self.elements.release_button_state();
                    self.geometry = None;
                    return Ok(ScreenInputResponse {
                        effects: vec![ScreenEffect::Action(rusterix::EntityAction::Off)],
                        ..ScreenInputResponse::handled(true)
                    });
                }
                Some(PointerCapture::Text) => {
                    let mut response =
                        ScreenInputResponse::text(self.text_input_event(fonts, event));
                    response.input.consumed = true;
                    return Ok(response);
                }
                Some(PointerCapture::Message(_)) => {
                    for widget in &mut self.elements.messages_widgets {
                        widget.touch_up();
                    }
                    return Ok(ScreenInputResponse::handled(false));
                }
                Some(PointerCapture::Cancelled) => return Ok(ScreenInputResponse::handled(false)),
                Some(PointerCapture::Choice(_)) => {
                    self.choice_input_event(fonts, event);
                    return Ok(ScreenInputResponse::handled(false));
                }
                None => {}
            }
        }
        if self.pointer_capture.is_some()
            && matches!(event, InputEvent::PointerButton { pressed: true, .. })
        {
            return Ok(ScreenInputResponse::handled(false));
        }
        if let InputEvent::Modifiers(modifiers) = event {
            self.input_modifiers = *modifiers;
        }
        if let Some(response) = self.keyboard_input(world, fonts, event)? {
            return Ok(response);
        }
        if matches!(event, InputEvent::PointerButton { pressed: true, .. }) {
            self.cancel_keyboard_capture();
        }
        if let InputEvent::PointerButton {
            position,
            pressed: true,
            ..
        } = event
        {
            if !self.inside_screen(*position)
                && self.choice_controller.open_id().is_none()
                && !self.message_blocks_input()
            {
                self.focus_control(fonts, None);
                return Ok(ScreenInputResponse::default());
            }
        }
        let choice = self.choice_input_event(fonts, event);
        if choice.consumed {
            let press = matches!(
                event,
                InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed: true,
                    ..
                }
            );
            if press {
                if let Some(ScreenFocus::Choice(id)) = self.focus {
                    self.pointer_capture = Some(PointerCapture::Choice(id));
                }
            }
            return Ok(ScreenInputResponse {
                input: choice,
                cancel_controls: press,
                ..Default::default()
            });
        }
        if let Some(response) = self.message_modal_event(fonts, event) {
            return Ok(response);
        }
        if let InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position,
        } = event
        {
            self.cursor = *position;
            // Ignore a second primary press until the original owner releases.
            if self.pointer_capture.is_some() {
                return Ok(ScreenInputResponse::handled(false));
            }
            if let Some(id) = screen_model::topmost_button_at(
                &self.elements.button_widgets,
                &self.elements.widgets_to_hide,
                scenevm::prelude::Vec2::new(position[0], position[1]),
            ) {
                self.focus_control(fonts, Some(ScreenFocus::Button(id)));
                self.pressed_button = Some(id);
                let widget = &self.elements.button_widgets[&id];
                let map = world.map.unwrap_or(&self.empty_map);
                let command = screen_model::resolved_widget_command(
                    widget,
                    world.assets,
                    screen_model::resolve_party_entity(map, widget.party.as_deref()),
                    &self.elements.ui_state,
                );
                let held = matches!(
                    command
                        .as_deref()
                        .and_then(rusterix::client::command::parse_client_command),
                    Some(rusterix::client::command::ClientCommandBinding::Control(_))
                );
                if held {
                    let activation = self.activate_button(world, id)?;
                    self.pointer_capture = Some(PointerCapture::HeldButton(id));
                    return Ok(ScreenInputResponse {
                        effects: activation.effects,
                        cancel_controls: true,
                        ..ScreenInputResponse::handled(true)
                    });
                }
                self.pointer_capture = Some(PointerCapture::Button(id));
                return Ok(ScreenInputResponse {
                    cancel_controls: true,
                    ..ScreenInputResponse::handled(true)
                });
            }
        }
        let mut response = ScreenInputResponse::text(self.text_input_event(fonts, event));
        if response.input.consumed
            && matches!(
                event,
                InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed: true,
                    ..
                }
            )
        {
            self.pointer_capture = Some(PointerCapture::Text);
            response.cancel_controls = true;
        }
        if !response.input.consumed {
            if let Some(message) = self.message_input_event(fonts, event) {
                return Ok(message);
            }
        }
        if !response.input.consumed && self.world_texts.input(event) {
            let press = matches!(event, InputEvent::PointerButton { pressed: true, .. });
            if press {
                self.focus_control(fonts, None);
                self.pointer_capture = Some(PointerCapture::Cancelled);
            }
            return Ok(ScreenInputResponse {
                cancel_controls: press,
                ..ScreenInputResponse::handled(true)
            });
        }
        if matches!(
            event,
            InputEvent::PointerMoved { .. } | InputEvent::PointerLeft
        ) {
            response.input.redraw = true;
        }
        Ok(response)
    }
}
