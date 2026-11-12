//! Command selection, activation and action-bar assignment have balanced owners.
use super::*;
use model::HudAction;
use rusterix::{EntityAction, client::command::parse_client_command};
use theframework::platform::{InputEvent, PhysicalKey, PointerButton};
#[derive(Clone)]
pub(super) struct CommandReceipt {
    pub control: HudControlId,
    pub(super) bounds: Rect,
    point: [f32; 2],
    binding: world_pointer::Binding,
    pub(super) action: HudAction,
    available: bool,
    immediate: bool,
}
impl CommandReceipt {
    fn command(&self) -> &str {
        match &self.action {
            HudAction::CatalogCommand { command, .. } => command,
            _ => unreachable!("catalogue receipt action"),
        }
    }
    fn assignment(&self) -> bool {
        matches!(
            self.action,
            HudAction::CatalogCommand {
                assignment: true,
                ..
            }
        )
    }
}
pub(super) struct CommandCapture {
    pub receipt: CommandReceipt,
    pub cancelled: bool,
    pub dragging: bool,
    key: Option<PhysicalKey>,
    assigning: bool,
    target: Option<world_pointer::CommandTarget>,
}
impl GameScreenController {
    fn command_receipt_valid(&self, world: &RuntimeView<'_>, receipt: &CommandReceipt) -> bool {
        self.hud_capture_valid(world, receipt.binding)
            && !self.pointer_modal()
            && self.hud_action_valid(
                world,
                receipt.control,
                receipt.bounds,
                Some(&receipt.action),
            )
            && self.hud_control_visible(world, receipt.control, receipt.point)
    }
    pub(in crate::game_screen) fn hud_command_input(
        &mut self,
        world: &RuntimeView<'_>,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> Result<Option<ScreenInputResponse>, String> {
        self.hud.bind(world);
        if matches!(event, InputEvent::Focus(false)) {
            self.hud.command_capture = None;
            self.hud.pending_command = None;
            return Ok(None);
        }
        if let Some(receipt) = &self.hud.pending_command {
            if !self.command_receipt_valid(world, receipt) {
                self.hud.pending_command = None;
            }
        }
        if matches!(event, InputEvent::PointerLeft) {
            self.hud.pending_command = None;
            if let Some(capture) = &mut self.hud.command_capture {
                capture.cancelled = true;
                return Ok(Some(ScreenInputResponse::handled(true)));
            }
            return Ok(None);
        }
        if let Some(mut capture) = self.hud.command_capture.take() {
            capture.cancelled |= !self.command_receipt_valid(world, &capture.receipt);
            let mut released = false;
            let mut point = capture.receipt.point;
            match event {
                InputEvent::PointerMoved { position } if capture.key.is_none() => {
                    self.cursor = *position;
                    capture.dragging |= distance_exceeded(capture.receipt.point, *position);
                }
                InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed: false,
                    position,
                } if capture.key.is_none() => {
                    self.cursor = *position;
                    point = *position;
                    released = true;
                    capture.dragging |= distance_exceeded(capture.receipt.point, *position);
                }
                InputEvent::Key {
                    physical,
                    pressed: false,
                    ..
                } if capture.key == Some(*physical) => released = true,
                InputEvent::Key {
                    physical: PhysicalKey::Escape,
                    pressed: true,
                    ..
                } => {
                    capture.cancelled = true;
                    self.hud.pending_command = None;
                }
                _ => {}
            }
            if !released {
                self.hud.command_capture = Some(capture);
                return Ok(Some(ScreenInputResponse::handled(true)));
            }
            let mut effects = Vec::new();
            if !capture.cancelled {
                let receipt = &capture.receipt;
                if capture.assigning {
                    if let Some(target) = capture
                        .target
                        .filter(|target| self.command_target_valid(world, target, point))
                    {
                        effects.push(ScreenEffect::Action(EntityAction::SetCommandSlot {
                            slot: target.slot,
                            command: Some(receipt.command().into()),
                        }));
                        self.hud.pending_command = None;
                        self.hud.state.finish_assignment(receipt.control.panel);
                    }
                } else if capture.dragging && capture.key.is_none() {
                    if let Some(target) = self.command_target_at(world, point) {
                        effects.push(ScreenEffect::Action(EntityAction::SetCommandSlot {
                            slot: target.slot,
                            command: Some(receipt.command().into()),
                        }));
                    }
                } else if rect(receipt.bounds).contains(point)
                    && self.hud_control_visible(world, receipt.control, point)
                {
                    if receipt.assignment() {
                        self.hud.pending_command = Some(receipt.clone());
                    } else if receipt.available
                        && receipt.immediate == self.immediate_intents(world)?
                    {
                        if let Some(binding) = parse_client_command(receipt.command()) {
                            effects = self.activate_binding(world, binding)?.effects;
                        }
                    }
                }
            }
            return Ok(Some(ScreenInputResponse {
                effects,
                ..ScreenInputResponse::handled(true)
            }));
        }
        if self.hud.capture.is_some()
            || self.author_modal()
            || self.pointer_modal()
            || self.pointer_capture.is_some()
            || self.keyboard_capture.is_some()
        {
            return Ok(None);
        }
        if matches!(
            event,
            InputEvent::Key {
                physical: PhysicalKey::Escape,
                pressed: true,
                ..
            }
        ) && self.hud.pending_command.take().is_some()
        {
            return Ok(Some(ScreenInputResponse::handled(true)));
        }
        let pointer = match event {
            InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position,
            } => Some(*position),
            _ => None,
        };
        let key = match event {
            InputEvent::Key {
                physical: PhysicalKey::Enter | PhysicalKey::Space,
                pressed: true,
                repeat: false,
                ..
            } => {
                if self.input_modifiers.control
                    || self.input_modifiers.alt
                    || self.input_modifiers.super_key
                {
                    return Ok(None);
                }
                match event {
                    InputEvent::Key { physical, .. } => Some(*physical),
                    _ => None,
                }
            }
            _ => None,
        };
        if pointer.is_none() && key.is_none() {
            return Ok(None);
        }
        let frames = self.hud.frames(world, &self.elements);
        let card = frames.iter().rev().find_map(|frame| {
            frame.cards.iter().find_map(|card| {
                let control = frame.control(HudPart::CatalogEntry(card.index));
                let point = pointer.unwrap_or([
                    card.hit.x + card.hit.width / 2.,
                    card.hit.y + card.hit.height / 2.,
                ]);
                ((pointer.is_some() && rect(card.hit).contains(point))
                    || (key.is_some() && self.hud.focused == Some(control)))
                .then(|| (control, card, point))
            })
        });
        if let Some((control, card, point)) = card {
            let receipt = self
                .hud_binding(world)
                .zip(self.hud_displayed_immediate(world))
                .map(|(binding, immediate)| CommandReceipt {
                    control,
                    bounds: card.hit,
                    point,
                    binding,
                    action: card.action.clone(),
                    available: card.state.enabled,
                    immediate,
                });
            if let Some(receipt) = receipt {
                let cancelled = !self.command_receipt_valid(world, &receipt);
                if !cancelled {
                    self.hud.state.select_catalog(control.panel, &card.command);
                }
                self.focus_control(fonts, None);
                self.hud.focused = Some(control);
                self.cursor = point;
                self.hud.command_capture = Some(CommandCapture {
                    receipt,
                    cancelled,
                    dragging: false,
                    key,
                    assigning: false,
                    target: None,
                });
            } else {
                // A new/unpresented card still owns and consumes this gesture.
                self.hud.focused = Some(control);
                self.pointer_capture = Some(router::PointerCapture::Cancelled);
            }
            return Ok(Some(ScreenInputResponse {
                cancel_controls: true,
                ..ScreenInputResponse::handled(true)
            }));
        }
        if let Some(point) = pointer {
            if let Some(receipt) = self.hud.pending_command.clone() {
                let target = self.command_target_at(world, point);
                self.hud.command_capture = Some(CommandCapture {
                    receipt,
                    cancelled: target.is_none(),
                    dragging: false,
                    key: None,
                    assigning: true,
                    target,
                });
                self.cursor = point;
                return Ok(Some(ScreenInputResponse {
                    cancel_controls: true,
                    ..ScreenInputResponse::handled(true)
                }));
            }
        }
        Ok(None)
    }
}
fn distance_exceeded(start: [f32; 2], point: [f32; 2]) -> bool {
    point.into_iter().all(f32::is_finite)
        && (point[0] - start[0]).powi(2) + (point[1] - start[1]).powi(2) >= 36.
}
