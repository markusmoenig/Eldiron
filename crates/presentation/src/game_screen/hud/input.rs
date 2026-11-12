//! Floating controls use their own identities and balanced press/release owners.
use super::*;
use scenevm::prelude::Vec2;
use theframework::platform::{InputEvent, PhysicalKey, PointerButton};

pub(super) struct HudCapture {
    pub control: HudControlId,
    pub cancelled: bool,
    binding: Option<world_pointer::Binding>,
    bounds: Rect,
    point: [f32; 2],
    position: Vec2<i32>,
    key: Option<PhysicalKey>,
    item: Option<Arc<world_pointer::ItemReceipt>>,
}
impl GameScreenController {
    pub(in crate::game_screen) fn hud_input(
        &mut self,
        world: &RuntimeView<'_>,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> Option<ScreenInputResponse> {
        let size = self.elements.layout.viewport;
        self.hud.bind(world);
        if matches!(event, InputEvent::Focus(false)) {
            self.hud.capture = None;
            self.hud.focused = None;
            self.hud.pointer_present = false;
            return None;
        }
        if matches!(event, InputEvent::PointerLeft) {
            self.hud.pointer_present = false;
            if let Some(capture) = &mut self.hud.capture {
                capture.cancelled = true;
                return Some(ScreenInputResponse::handled(true));
            }
            return None;
        }
        let current = self.hud.current(world, size);
        if let Some(mut capture) = self.hud.capture.take() {
            let valid = capture
                .binding
                .is_some_and(|binding| self.hud_capture_valid(world, binding))
                && current
                    .as_ref()
                    .is_some_and(|frame| frame.id == capture.control.panel);
            capture.cancelled |= !valid;
            let mut released = false;
            match event {
                InputEvent::PointerMoved { position } if capture.key.is_none() => {
                    self.cursor = *position;
                    if capture.control.part == HudPart::Title
                        && !capture.cancelled
                        && position.into_iter().all(|v| v.is_finite())
                    {
                        self.hud.state.move_to(capture.control.panel, Vec2::new(
                            capture.position.x.saturating_add((position[0] - capture.point[0]).round() as i32),
                            capture.position.y.saturating_add((position[1] - capture.point[1]).round() as i32)), size);
                    }
                }
                InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed: false,
                    position,
                } if capture.key.is_none() => {
                    self.cursor = *position;
                    released = true;
                    if !rect(capture.bounds).contains(*position) {
                        capture.cancelled |= capture.control.part == HudPart::Close;
                    }
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
                } => capture.cancelled = true,
                _ => {}
            }
            if released {
                let mut effects = Vec::new();
                if !capture.cancelled {
                    if let Some(item) = &capture.item {
                        effects = self.hud_item_activate(world, item);
                    } else if capture.control.part == HudPart::Close
                        && self.hud_frame_valid(world, capture.control, capture.bounds)
                    {
                        self.hud.state.close();
                        self.hud.focused = None;
                    }
                }
                return Some(ScreenInputResponse {
                    effects,
                    ..ScreenInputResponse::handled(true)
                });
            }
            self.hud.capture = Some(capture);
            return Some(ScreenInputResponse::handled(true));
        }
        if self.pointer_modal() || self.pointer_capture.is_some() || self.keyboard_capture.is_some()
        {
            return None;
        }
        let frame = current?;
        if self.hud.focused.is_some_and(|id| id.panel != frame.id) {
            self.hud.focused = None;
        }
        match event {
            InputEvent::PointerMoved { position } => {
                self.hud.pointer_present = true;
                self.cursor = *position;
                None
            }
            InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position,
            } => {
                self.hud.pointer_present = true;
                let Some((control, bounds)) = frame
                    .controls()
                    .into_iter()
                    .rev()
                    .find(|(_, bounds)| rect(*bounds).contains(*position))
                else {
                    self.hud.focused = None;
                    return None;
                };
                self.focus_control(fonts, None);
                self.hud.focused = Some(control);
                self.cursor = *position;
                if matches!(control.part, HudPart::Slot(_)) {
                    if let Some(response) = self.hud_item_begin(world, control, *position) {
                        return Some(response);
                    }
                }
                let binding = self.hud_binding(world);
                let cancelled = !self.hud_frame_valid(world, control, bounds);
                self.hud.capture = Some(HudCapture {
                    control,
                    cancelled,
                    binding,
                    bounds,
                    point: *position,
                    position: frame.position,
                    key: None,
                    item: None,
                });
                Some(ScreenInputResponse {
                    cancel_controls: true,
                    ..ScreenInputResponse::handled(true)
                })
            }
            InputEvent::Scroll { delta, precise }
                if self.hud.pointer_present
                    && self
                        .pointer_visual_position()
                        .is_some_and(|point| rect(frame.rect).contains(point)) =>
            {
                if self.hud_frame_valid(world, frame.control(HudPart::Body), frame.rect) {
                    self.hud.state.scroll(&frame, *delta, *precise);
                }
                Some(ScreenInputResponse::handled(true))
            }
            InputEvent::Key {
                physical,
                pressed,
                repeat,
                ..
            } if self.hud.focused.is_some() => {
                if *physical == PhysicalKey::Escape {
                    if *pressed && !repeat {
                        self.hud.state.close();
                        self.hud.focused = None;
                    }
                    return Some(ScreenInputResponse::handled(true));
                }
                if self.input_modifiers.control
                    || self.input_modifiers.super_key
                    || self.input_modifiers.alt
                {
                    return None;
                }
                if *physical == PhysicalKey::Tab {
                    if *pressed && !repeat {
                        let controls: Vec<_> = frame
                            .controls()
                            .into_iter()
                            .filter(|(id, _)| matches!(id.part, HudPart::Close | HudPart::Slot(_)))
                            .collect();
                        if !controls.is_empty() {
                            let index = controls
                                .iter()
                                .position(|(id, _)| Some(*id) == self.hud.focused);
                            let next = if self.input_modifiers.shift {
                                index.map_or(controls.len() - 1, |i| {
                                    (i + controls.len() - 1) % controls.len()
                                })
                            } else {
                                index.map_or(0, |i| (i + 1) % controls.len())
                            };
                            self.hud.focused = Some(controls[next].0);
                        }
                    }
                    return Some(ScreenInputResponse::handled(true));
                }
                if matches!(
                    physical,
                    PhysicalKey::Up
                        | PhysicalKey::Down
                        | PhysicalKey::Left
                        | PhysicalKey::Right
                        | PhysicalKey::PageUp
                        | PhysicalKey::PageDown
                        | PhysicalKey::Home
                        | PhysicalKey::End
                ) {
                    if *pressed {
                        let step = frame.template.slot_size.max(8) as f32
                            + frame.template.gap.max(0) as f32;
                        let delta = match physical {
                            PhysicalKey::Up => [0., step],
                            PhysicalKey::Down => [0., -step],
                            PhysicalKey::Left => [step, 0.],
                            PhysicalKey::Right => [-step, 0.],
                            PhysicalKey::PageUp => [0., frame.clip.height],
                            PhysicalKey::PageDown => [0., -frame.clip.height],
                            PhysicalKey::Home => [frame.scroll[0] as f32, frame.scroll[1] as f32],
                            _ => [-frame.max_scroll[0] as f32, -frame.max_scroll[1] as f32],
                        };
                        self.hud.state.scroll(&frame, delta, true);
                        self.hud.focused = Some(frame.control(HudPart::Body));
                    }
                    return Some(ScreenInputResponse::handled(true));
                }
                if matches!(physical, PhysicalKey::Enter | PhysicalKey::Space) {
                    if !pressed || *repeat {
                        return Some(ScreenInputResponse::handled(false));
                    }
                    let control = self.hud.focused?;
                    let (_, bounds) = frame
                        .controls()
                        .into_iter()
                        .find(|(id, _)| *id == control)?;
                    let receipt = if matches!(control.part, HudPart::Slot(_)) {
                        self.hud_item_receipt(world, control)
                    } else {
                        Ok(None)
                    };
                    let cancelled =
                        !self.hud_frame_valid(world, control, bounds) || receipt.is_err();
                    self.hud.capture = Some(HudCapture {
                        control,
                        cancelled,
                        binding: self.hud_binding(world),
                        bounds,
                        point: [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.],
                        position: frame.position,
                        key: Some(*physical),
                        item: receipt.ok().flatten(),
                    });
                    return Some(ScreenInputResponse {
                        cancel_controls: true,
                        ..ScreenInputResponse::handled(true)
                    });
                }
                None
            }
            _ => None,
        }
    }
}
