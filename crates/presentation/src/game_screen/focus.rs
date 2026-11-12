//! One focus owner for every authored control, separate from pointer capture.
//! These are CPU presentation identities; no window or server clock is owned.
use super::router::PointerCapture;
use super::*;
use theframework::platform::{PhysicalKey, TextInputTarget};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenFocus {
    Text(ScreenTextTarget),
    Button(u32),
    Choice(u32),
}
impl ScreenFocus {
    fn id(self) -> u32 {
        match self {
            Self::Text(ScreenTextTarget::Input(id) | ScreenTextTarget::Command(id))
            | Self::Button(id)
            | Self::Choice(id) => id,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenCapture {
    World(Uuid),
    Hud(super::hud::HudControlId),
    Text(TextInputTarget),
    Button(u32),
    Choice(u32),
    Message(u32),
    KeyboardButton(u32),
    Cancelled,
}
#[derive(Clone, Debug)]
pub(super) struct KeyboardCapture {
    pub button: u32,
    pub key: PhysicalKey,
    pub held: bool,
    pub cancelled: bool,
    pub item: Option<Arc<world_pointer::ItemReceipt>>,
}
impl GameScreenController {
    pub fn focused_control(&self) -> Option<ScreenFocus> {
        self.focus
    }
    fn focus_visible(&self, focus: ScreenFocus) -> bool {
        match focus {
            ScreenFocus::Text(target) => self.text_targets().contains(&target),
            ScreenFocus::Button(id) => self.elements.button_widgets.get(&id).is_some_and(|w| {
                visible_control_rect(&self.elements, w.rect)
                    && !screen_model::widget_name_is_hidden(&w.name, &self.elements.widgets_to_hide)
            }),
            ScreenFocus::Choice(id) => self.elements.choice_widgets.get(&id).is_some_and(|w| {
                visible_control_rect(&self.elements, w.rect)
                    && !w.options.is_empty()
                    && !screen_model::widget_name_is_hidden(&w.name, &self.elements.widgets_to_hide)
            }),
        }
    }
    pub fn focus_control(&mut self, fonts: &mut GameFonts, focus: Option<ScreenFocus>) {
        self.hud.clear_focus();
        let focus = focus.filter(|target| self.focus_visible(*target));
        if self.focus != focus {
            self.cancel_item_pointer();
            if let Some(ScreenFocus::Choice(id)) = self.focus {
                if self.choice_controller.open_id() == Some(id) {
                    self.choice_controller
                        .cancel(&mut self.elements.choice_widgets);
                }
            }
            if self.pointer_capture.is_some() {
                self.pointer_capture = Some(PointerCapture::Cancelled);
                self.choice_controller.pointer_up();
            } else if matches!(self.focus, Some(ScreenFocus::Choice(_)))
                && !matches!(focus, Some(ScreenFocus::Choice(_)))
            {
                self.choice_controller.pointer_up();
            }
            self.cancel_keyboard_capture();
            self.pressed_button = None;
            if self.elements.activated_widgets != self.elements.permanently_activated_widgets {
                self.elements.release_button_state();
                self.geometry = None;
            }
            self.focus = focus;
        }
        self.sync_text_focus(fonts);
    }
    pub(super) fn prepare_focus(&mut self, fonts: &mut GameFonts) {
        if self.focus.is_some_and(|target| !self.focus_visible(target))
            || (self.message_blocks_input()
                && self.focus.is_some()
                && self.focused_text().is_none())
        {
            self.focus_control(fonts, None);
        }
    }
    pub fn focus_targets(&self, world: &RuntimeView<'_>) -> Vec<ScreenFocus> {
        let mut targets = self
            .text_targets()
            .into_iter()
            .map(ScreenFocus::Text)
            .collect::<Vec<_>>();
        let map = world.map.unwrap_or(&self.empty_map);
        for (id, widget) in &self.elements.button_widgets {
            let focus = ScreenFocus::Button(*id);
            if !self.focus_visible(focus) {
                continue;
            }
            let command = screen_model::resolved_widget_command(
                widget,
                world.assets,
                screen_model::resolve_party_entity(map, widget.party.as_deref()),
                &self.elements.ui_state,
            );
            if command.as_deref().is_some_and(|command| {
                !rusterix::client::rules_ui::command_state(
                    world.assets,
                    screen_model::resolve_party_entity(map, widget.party.as_deref()),
                    command,
                )
                .enabled
            }) {
                continue;
            }
            targets.push(focus);
        }
        targets.extend(
            self.elements
                .choice_widgets
                .keys()
                .map(|id| ScreenFocus::Choice(*id))
                .filter(|target| self.focus_visible(*target)),
        );
        targets.sort_by_key(|focus| {
            (
                focus.id(),
                match focus {
                    ScreenFocus::Text(ScreenTextTarget::Input(_)) => 0,
                    ScreenFocus::Text(ScreenTextTarget::Command(_)) => 1,
                    ScreenFocus::Button(_) => 2,
                    ScreenFocus::Choice(_) => 3,
                },
            )
        });
        targets
    }
    pub(super) fn focus_next(
        &mut self,
        world: &RuntimeView<'_>,
        fonts: &mut GameFonts,
        backwards: bool,
    ) {
        let targets = self.focus_targets(world);
        let next = if let Some(current) = self.focus {
            if let Some(index) = targets.iter().position(|target| *target == current) {
                let next = if backwards {
                    (index + targets.len() - 1) % targets.len()
                } else {
                    (index + 1) % targets.len()
                };
                Some(targets[next])
            } else if backwards {
                targets
                    .iter()
                    .rev()
                    .find(|target| target.id() < current.id())
                    .or_else(|| targets.last())
                    .copied()
            } else {
                targets
                    .iter()
                    .find(|target| target.id() > current.id())
                    .or_else(|| targets.first())
                    .copied()
            }
        } else if backwards {
            targets.last().copied()
        } else {
            targets.first().copied()
        };
        self.focus_control(fonts, next);
    }
    /// A stable owner stamp lets an application stop held input on a programmatic
    /// focus/capture change without resending Off on ordinary frame/layout work.
    pub fn control_capture(&self) -> Option<ScreenCapture> {
        if matches!(self.pointer_capture, Some(PointerCapture::Item(_))) {
            return self.world_capture_owner();
        }
        if let Some(owner) = self.hud.owner() {
            return Some(owner);
        }
        if let Some(capture) = self.keyboard_capture.as_ref() {
            return Some(if capture.cancelled {
                ScreenCapture::Cancelled
            } else {
                ScreenCapture::KeyboardButton(capture.button)
            });
        }
        match self.pointer_capture.as_ref() {
            Some(PointerCapture::Button(id, _) | PointerCapture::HeldButton(id)) => {
                return Some(ScreenCapture::Button(*id));
            }
            Some(PointerCapture::Item(control)) => return Some(control.capture()),
            Some(PointerCapture::Choice(id)) => return Some(ScreenCapture::Choice(*id)),
            Some(PointerCapture::Message(id)) => return Some(ScreenCapture::Message(*id)),
            Some(PointerCapture::Cancelled) => return Some(ScreenCapture::Cancelled),
            _ => {}
        }
        if let Some(target) = self.focused_text() {
            if let Some(view) = self.inputs.get(&target) {
                return Some(ScreenCapture::Text(view.editor.input_target()));
            }
        }
        if let Some(id) = self.choice_controller.open_id() {
            return Some(ScreenCapture::Choice(id));
        }
        self.elements
            .messages_widgets
            .iter()
            .find(|w| {
                w.blocks_input()
                    && !screen_model::widget_name_is_hidden(&w.name, &self.elements.widgets_to_hide)
            })
            .map(|w| ScreenCapture::Message(w.id))
            .or_else(|| self.world_capture_owner())
    }
}
