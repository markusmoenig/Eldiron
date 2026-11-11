//! Transcript flow stays in MessagesWidget. This adapter routes native logical
//! input, shared Parley command editing and exactly one choice action.
use super::router::{PointerCapture, ScreenInputResponse};
use super::*;
use rusterix::{EntityAction, Value};
use theframework::platform::{InputEvent, LogicalKey, PhysicalKey, PointerButton};
impl GameScreenController {
    fn message_action(&mut self, action: EntityAction) -> ScreenInputResponse {
        // The same dialog can be displayed in several authored transcripts.
        // Acknowledge it in every matching view, submit one runtime action.
        if let EntityAction::Choice(choice) = &action {
            for widget in &mut self.elements.messages_widgets {
                let key = widget
                    .active_choices
                    .iter()
                    .find(|line| line.choice.as_ref() == Some(choice))
                    .and_then(|line| line.choice_key);
                if let Some(key) = key {
                    widget.select_active_choice_if_matches(key, choice);
                }
            }
        }
        let effects = if action == EntityAction::Off {
            Vec::new()
        } else {
            vec![ScreenEffect::Action(action)]
        };
        ScreenInputResponse {
            effects,
            cancel_controls: true,
            ..ScreenInputResponse::handled(true)
        }
    }
    pub(super) fn message_modal_event(
        &mut self,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> Option<ScreenInputResponse> {
        let index = self.elements.messages_widgets.iter().rposition(|w| {
            w.blocks_input()
                && !screen_model::widget_name_is_hidden(&w.name, &self.elements.widgets_to_hide)
        })?;
        match event {
            InputEvent::Key {
                physical, pressed, ..
            } if self.focused_text.is_none() => {
                if *pressed {
                    let key = match physical {
                        PhysicalKey::Enter => "enter",
                        PhysicalKey::Space => "space",
                        _ => "",
                    };
                    self.elements.messages_widgets[index]
                        .user_event("key_down", &Value::Str(key.into()));
                }
            }
            InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position,
            } => {
                self.focus_text(fonts, None);
                let action = self.elements.messages_widgets[index]
                    .pointer_down(scenevm::prelude::Vec2::new(position[0], position[1]));
                self.pointer_capture = Some(PointerCapture::Message);
                return Some(self.message_action(action.unwrap_or(EntityAction::Off)));
            }
            InputEvent::Scroll { .. }
            | InputEvent::Pinch(_)
            | InputEvent::RelativeMotion { .. }
            | InputEvent::PointerButton { .. } => {}
            InputEvent::Text(_)
            | InputEvent::TextFor { .. }
            | InputEvent::CompositionFor { .. }
            | InputEvent::Composition { .. }
                if self.focused_text.is_none() => {}
            _ => return None,
        }
        Some(ScreenInputResponse {
            cancel_controls: true,
            ..ScreenInputResponse::handled(true)
        })
    }
    pub(super) fn message_input_event(
        &mut self,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> Option<ScreenInputResponse> {
        let mut visible = self
            .elements
            .messages_widgets
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                !screen_model::widget_name_is_hidden(&w.name, &self.elements.widgets_to_hide)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        visible.reverse();
        match event {
            InputEvent::Key {
                physical,
                logical,
                pressed,
                repeat,
            } if self.focused_text.is_none() => {
                if !pressed
                    || *repeat
                    || self.input_modifiers.control
                    || self.input_modifiers.super_key
                    || self.input_modifiers.alt
                {
                    return None;
                }
                let digit = match physical {
                    PhysicalKey::Digit(digit) => char::from_digit(*digit as u32, 10),
                    _ => {
                        if let LogicalKey::Character(text) = logical {
                            let mut chars = text.chars();
                            chars
                                .next()
                                .filter(|c| c.is_ascii_digit() && chars.next().is_none())
                        } else {
                            None
                        }
                    }
                };
                if let Some(digit) = digit {
                    for index in &visible {
                        if let Some(choice) =
                            self.elements.messages_widgets[*index].select_active_choice(digit)
                        {
                            return Some(self.message_action(EntityAction::Choice(choice)));
                        }
                    }
                }
                let slash = matches!(logical,LogicalKey::Character(text) if text=="/");
                if *physical == PhysicalKey::Enter || slash {
                    let target = visible.iter().find_map(|index| {
                        self.messages
                            .get(*index)
                            .and_then(MessageLayouts::command)
                            .map(|_| {
                                ScreenTextTarget::Command(self.elements.messages_widgets[*index].id)
                            })
                    });
                    if let Some(target) = target {
                        self.focus_text(fonts, Some(target));
                        self.suppress_activation_slash = slash;
                        return Some(ScreenInputResponse {
                            cancel_controls: true,
                            ..ScreenInputResponse::handled(true)
                        });
                    }
                }
            }
            InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position,
            } => {
                for index in visible {
                    let widget = &mut self.elements.messages_widgets[index];
                    if widget
                        .rect
                        .contains(scenevm::prelude::Vec2::new(position[0], position[1]))
                    {
                        if let Some(action) = widget
                            .pointer_down(scenevm::prelude::Vec2::new(position[0], position[1]))
                        {
                            self.pointer_capture = Some(PointerCapture::Message);
                            return Some(self.message_action(action));
                        }
                    }
                }
            }
            InputEvent::Scroll { delta, precise } if delta[1].is_finite() && delta[1] != 0. => {
                for index in visible {
                    let widget = &mut self.elements.messages_widgets[index];
                    if !widget
                        .rect
                        .contains(scenevm::prelude::Vec2::new(self.cursor[0], self.cursor[1]))
                        || !widget.scrollback
                        || widget.messages.is_empty()
                    {
                        continue;
                    }
                    let unit = if *precise {
                        widget.view_state().line_height.max(1.)
                    } else {
                        1.
                    };
                    let residual = self.message_scroll.entry(widget.id).or_default();
                    *residual = (*residual + delta[1] / unit).clamp(-1000., 1000.);
                    let steps = residual.trunc().clamp(-1000., 1000.) as isize;
                    *residual -= steps as f32;
                    let redraw = steps != 0 && widget.scroll_at(steps * 120, None);
                    return Some(ScreenInputResponse {
                        input: theframework::platform::InputResponse {
                            consumed: true,
                            redraw,
                        },
                        ..Default::default()
                    });
                }
            }
            _ => {}
        }
        None
    }
}
