//! One pointer owner across authored controls. Application/world input only
//! receives events that the screen has declined; text never becomes a shortcut.
use super::*;
use theframework::platform::{InputEvent, InputResponse, PointerButton};

#[derive(Clone, Copy)]
pub(super) enum PointerCapture {
    Button(u32),
    HeldButton,
    Text,
    Choice,
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
    fn handled(redraw: bool) -> Self {
        Self {
            input: InputResponse {
                consumed: true,
                redraw,
            },
            ..Default::default()
        }
    }
    fn text(text: ScreenTextResponse) -> Self {
        Self {
            input: text.input,
            text: Some(text),
            ..Default::default()
        }
    }
}
impl GameScreenController {
    /// Coordinates are in the same logical screen space used by `paint`.
    /// Modal choices precede buttons, fields and world input, matching paint order.
    /// A captured drag/release keeps its original owner even outside its bounds.
    pub fn input(
        &mut self,
        world: &RuntimeView<'_>,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> Result<ScreenInputResponse, String> {
        if matches!(event, InputEvent::Focus(false)) {
            self.pointer_capture = None;
            self.pressed_button = None;
            self.elements.release_button_state();
            self.geometry = None;
            self.choice_input_event(fonts, event);
            let mut response = ScreenInputResponse::text(self.text_input_event(fonts, event));
            response.input.redraw = true;
            response.cancel_controls = true;
            return Ok(response);
        }
        if let InputEvent::PointerMoved { position } = event {
            let moved = self.cursor != *position;
            self.cursor = *position;
            match self.pointer_capture {
                Some(PointerCapture::Button(_) | PointerCapture::HeldButton) => {
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
                    let activation = if top == Some(id) {
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
                Some(PointerCapture::HeldButton) => {
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
                Some(PointerCapture::Choice) => {
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
                self.pointer_capture = Some(PointerCapture::Choice);
            }
            return Ok(ScreenInputResponse {
                input: choice,
                cancel_controls: press,
                ..Default::default()
            });
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
                self.focus_input(fonts, None);
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
                    self.pointer_capture = Some(PointerCapture::HeldButton);
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
        if matches!(
            event,
            InputEvent::PointerMoved { .. } | InputEvent::PointerLeft
        ) {
            response.input.redraw = true;
        }
        Ok(response)
    }
}
