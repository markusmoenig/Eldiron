//! Keyboard focus uses semantic control operations, never synthetic pointer hits.
use super::focus::KeyboardCapture;
use super::*;
use theframework::platform::{InputEvent, PhysicalKey};
impl GameScreenController {
    pub(super) fn cancel_keyboard_capture(&mut self) {
        if let Some(capture) = &mut self.keyboard_capture {
            capture.cancelled = true;
        }
    }
    pub(super) fn keyboard_input(
        &mut self,
        world: &RuntimeView<'_>,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> Result<Option<ScreenInputResponse>, String> {
        let InputEvent::Key {
            physical,
            pressed,
            repeat,
            ..
        } = event
        else {
            return Ok(None);
        };
        if let Some(capture) = self.keyboard_capture.clone() {
            if *physical == capture.key {
                if *pressed {
                    return Ok(Some(ScreenInputResponse::handled(false)));
                }
                self.keyboard_capture = None;
                if capture.cancelled {
                    return Ok(Some(ScreenInputResponse::handled(false)));
                }
                self.pressed_button = None;
                let effects = if capture.held {
                    vec![ScreenEffect::Action(rusterix::EntityAction::Off)]
                } else if self.focus == Some(ScreenFocus::Button(capture.button))
                    && capture
                        .item
                        .as_ref()
                        .is_none_or(|item| self.item_receipt_valid(world, item))
                {
                    self.activate_button(world, capture.button)?.effects
                } else {
                    Vec::new()
                };
                self.elements.release_button_state();
                self.geometry = None;
                return Ok(Some(ScreenInputResponse {
                    effects,
                    ..ScreenInputResponse::handled(true)
                }));
            }
        }
        if self.keyboard_capture.is_some() && *pressed && *physical == PhysicalKey::Escape {
            self.focus_control(fonts, None);
            return Ok(Some(ScreenInputResponse::handled(true)));
        }
        if self.message_blocks_input() {
            return Ok(None);
        }
        if *physical == PhysicalKey::Tab
            && self.focused_text().is_none()
            && (self.focus.is_some() || self.elements.game_widgets.is_empty())
        {
            if *pressed && !repeat {
                self.focus_next(world, fonts, self.input_modifiers.shift);
            }
            return Ok(Some(ScreenInputResponse::handled(*pressed && !repeat)));
        }
        // A pointer drag and a keyboard activation never own the same control.
        // A text drag still permits editing and composition in its own editor.
        if self.keyboard_capture.is_some()
            || (self.pointer_capture.is_some() && self.focused_text().is_none())
        {
            return Ok(Some(ScreenInputResponse::handled(false)));
        }
        if self.input_modifiers.control
            || self.input_modifiers.super_key
            || self.input_modifiers.alt
        {
            return Ok(None);
        }
        match self.focus {
            Some(ScreenFocus::Button(id))
                if matches!(physical, PhysicalKey::Enter | PhysicalKey::Space) =>
            {
                if !pressed || *repeat {
                    return Ok(Some(ScreenInputResponse::handled(false)));
                }
                if !self.focus_targets(world).contains(&ScreenFocus::Button(id)) {
                    return Ok(Some(ScreenInputResponse::handled(false)));
                }
                let widget = &self.elements.button_widgets[&id];
                let map = world.map.unwrap_or(&self.empty_map);
                let receipt = self.item_receipt(world, id, None);
                let rejected = receipt.is_err();
                let item = receipt.ok().flatten();
                let held = !rejected
                    && item.is_none()
                    && matches!(
                        screen_model::resolved_widget_command(
                            widget,
                            world.assets,
                            world.party_entity(map, widget.party.as_deref()),
                            &self.elements.ui_state,
                        )
                        .as_deref()
                        .and_then(rusterix::client::command::parse_client_command),
                        Some(rusterix::client::command::ClientCommandBinding::Control(_))
                    );
                let effects = if held {
                    self.activate_button(world, id)?.effects
                } else {
                    Vec::new()
                };
                self.pressed_button = (!rejected).then_some(id);
                self.keyboard_capture = Some(KeyboardCapture {
                    button: id,
                    key: *physical,
                    held,
                    cancelled: rejected,
                    item,
                });
                Ok(Some(ScreenInputResponse {
                    effects,
                    cancel_controls: true,
                    ..ScreenInputResponse::handled(true)
                }))
            }
            Some(ScreenFocus::Choice(id)) => {
                use screen_model::ChoiceNavigation as N;
                let navigation = match physical {
                    PhysicalKey::Left | PhysicalKey::Up => Some(N::Previous),
                    PhysicalKey::Right | PhysicalKey::Down => Some(N::Next),
                    PhysicalKey::Home => Some(N::First),
                    PhysicalKey::End => Some(N::Last),
                    PhysicalKey::Escape => Some(N::Cancel),
                    _ => None,
                };
                let activate = matches!(physical, PhysicalKey::Enter | PhysicalKey::Space);
                if navigation.is_none() && !activate {
                    return Ok(None);
                }
                if !pressed || (*repeat && activate) {
                    return Ok(Some(ScreenInputResponse::handled(false)));
                }
                let widgets = &mut self.elements.choice_widgets;
                let outcome = if activate && self.choice_controller.open_id() != Some(id) {
                    self.choice_controller.activate(
                        id,
                        widgets,
                        &self.elements.widgets_to_hide,
                        &self.elements.ui_state,
                    )
                } else {
                    self.choice_controller.navigate_at(
                        id,
                        widgets,
                        &self.elements.widgets_to_hide,
                        &self.elements.ui_state,
                        navigation.unwrap_or(N::Accept),
                    )
                };
                if let Some((binding, value)) = outcome.selection {
                    self.elements.set_binding(&binding, &value);
                    self.geometry = None;
                }
                Ok(Some(ScreenInputResponse::handled(outcome.consumed)))
            }
            Some(ScreenFocus::Button(_)) if *physical == PhysicalKey::Escape => {
                if *pressed {
                    self.focus_control(fonts, None);
                }
                Ok(Some(ScreenInputResponse::handled(*pressed)))
            }
            _ => Ok(None),
        }
    }
}
