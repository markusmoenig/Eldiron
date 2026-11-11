//! Choice controls own modal input before text fields and world controls.
use super::*;
use theframework::platform::{InputEvent, InputResponse, PhysicalKey, PointerButton};
impl GameScreenController {
    pub fn choice_input_event(
        &mut self,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> InputResponse {
        use screen_model::ChoiceNavigation as N;
        self.choice_controller.prepare(
            &mut self.elements.choice_widgets,
            &self.elements.widgets_to_hide,
            &self.elements.ui_state,
        );
        let mut redraw = false;
        match event {
            InputEvent::Focus(false) => {
                redraw = self
                    .choice_controller
                    .cancel(&mut self.elements.choice_widgets);
                self.choice_controller.pointer_up();
            }
            InputEvent::PointerMoved { position } => {
                let changed = self.cursor != *position;
                self.cursor = *position;
                let captured = self.choice_controller.open_id().is_some()
                    || self.choice_controller.pointer_is_captured();
                return InputResponse {
                    consumed: captured,
                    redraw: captured && changed,
                };
            }
            InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed,
                position,
            } => {
                self.cursor = *position;
                if !pressed {
                    return InputResponse {
                        consumed: self.choice_controller.pointer_up(),
                        redraw: false,
                    };
                }
                let outcome = self.choice_controller.pointer(
                    &mut self.elements.choice_widgets,
                    &self.elements.widgets_to_hide,
                    &self.elements.ui_state,
                    scenevm::prelude::Vec2::new(position[0], position[1]),
                );
                if let Some((binding, value)) = outcome.selection {
                    self.elements.set_binding(&binding, &value);
                }
                if outcome.consumed {
                    self.focus_input(fonts, None);
                }
                return InputResponse {
                    consumed: outcome.consumed,
                    redraw: outcome.consumed,
                };
            }
            InputEvent::Key {
                physical, pressed, ..
            } if self.choice_controller.open_id().is_some() => {
                if !pressed {
                    return InputResponse {
                        consumed: true,
                        redraw: false,
                    };
                }
                let navigation = match physical {
                    PhysicalKey::Up => Some(N::Previous),
                    PhysicalKey::Down => Some(N::Next),
                    PhysicalKey::Home => Some(N::First),
                    PhysicalKey::End => Some(N::Last),
                    PhysicalKey::Enter | PhysicalKey::Space => Some(N::Accept),
                    PhysicalKey::Escape | PhysicalKey::Tab => Some(N::Cancel),
                    _ => None,
                };
                if let Some(navigation) = navigation {
                    let outcome = self.choice_controller.navigate(
                        &mut self.elements.choice_widgets,
                        &self.elements.widgets_to_hide,
                        &self.elements.ui_state,
                        navigation,
                    );
                    if let Some((binding, value)) = outcome.selection {
                        self.elements.set_binding(&binding, &value);
                    }
                }
                return InputResponse {
                    consumed: true,
                    redraw: navigation.is_some(),
                };
            }
            InputEvent::Text(_)
            | InputEvent::Composition { .. }
            | InputEvent::Scroll { .. }
            | InputEvent::RelativeMotion { .. }
            | InputEvent::Pinch(_)
            | InputEvent::PointerButton { .. }
                if self.choice_controller.open_id().is_some() =>
            {
                return InputResponse {
                    consumed: true,
                    redraw: false,
                };
            }
            _ => {}
        }
        InputResponse {
            consumed: false,
            redraw,
        }
    }
}
