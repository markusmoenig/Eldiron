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
    action: Option<model::HudAction>,
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
        let current = self.hud.frames(world, &self.elements);
        if let Some(mut capture) = self.hud.capture.take() {
            let valid = capture
                .binding
                .is_some_and(|binding| self.hud_capture_valid(world, binding))
                && current
                    .iter()
                    .any(|frame| frame.id == capture.control.panel);
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
                        capture.cancelled |=
                            capture.control.part == HudPart::Close || capture.action.is_some();
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
                if !capture.cancelled
                    && (capture.control.part == HudPart::Title
                        || self.hud_control_visible(world, capture.control, capture.point))
                {
                    if let Some(item) = &capture.item {
                        effects = self.hud_item_activate(world, item);
                    } else if capture.control.part == HudPart::Close
                        && self.hud_frame_valid(world, capture.control, capture.bounds)
                    {
                        self.hud.state.close_panel(capture.control.panel);
                        self.hud.focused = None;
                    } else if let Some(action) = &capture.action {
                        if self.hud_action_valid(
                            world,
                            capture.control,
                            capture.bounds,
                            Some(action),
                        ) {
                            self.hud.state.activate(action, &mut self.elements);
                            if matches!(
                                action,
                                model::HudAction::CatalogAssign
                                    | model::HudAction::CatalogPage(_)
                                    | model::HudAction::CatalogTab(_)
                            ) {
                                self.hud.pending_command = None;
                            }
                            if matches!(
                                action,
                                model::HudAction::Category(_) | model::HudAction::Sort(_)
                            ) {
                                self.geometry = None;
                            }
                            if matches!(
                                action,
                                model::HudAction::ToggleSort | model::HudAction::PreferencePopup(_)
                            ) && self.hud.state.popup_open()
                            {
                                self.hud.focused = self
                                    .hud
                                    .frames(world, &self.elements)
                                    .iter()
                                    .find(|panel| panel.id == capture.control.panel)
                                    .and_then(|panel| {
                                        panel
                                            .buttons
                                            .iter()
                                            .find(|button| button.popup)
                                            .map(|button| panel.control(button.part))
                                    });
                            } else if let Some(part) = match action {
                                model::HudAction::Sort(_) => Some(HudPart::Sort),
                                model::HudAction::Tooltips(_) => Some(HudPart::Preference(
                                    screen_model::PreferenceSetting::Tooltips,
                                )),
                                model::HudAction::TooltipDelay(_) => Some(HudPart::Preference(
                                    screen_model::PreferenceSetting::TooltipDelay,
                                )),
                                _ => None,
                            } {
                                self.hud.focused = Some(HudControlId {
                                    panel: capture.control.panel,
                                    part,
                                });
                            }
                        }
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
        if self.author_modal() || self.pointer_capture.is_some() || self.keyboard_capture.is_some()
        {
            return None;
        }
        if self
            .hud
            .focused
            .is_some_and(|id| !current.iter().any(|frame| frame.id == id.panel))
        {
            self.hud.focused = None;
        }
        let frame = current
            .iter()
            .rev()
            .find(|frame| frame.popup.is_some())
            .or_else(|| match event {
                InputEvent::PointerButton { position, .. } => current
                    .iter()
                    .rev()
                    .find(|frame| rect(frame.rect).contains(*position))
                    .or_else(|| current.last()),
                InputEvent::Scroll { .. } => current
                    .iter()
                    .rev()
                    .find(|frame| rect(frame.rect).contains(self.cursor)),
                _ => self
                    .hud
                    .focused
                    .and_then(|id| current.iter().find(|frame| frame.id == id.panel))
                    .or_else(|| current.last()),
            })?;
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
                if let Some(popup) = frame.popup {
                    if !rect(popup).contains(*position)
                        && !frame.buttons.iter().any(|button| {
                            Some(button.part) == frame.popup_trigger()
                                && rect(button.rect).contains(*position)
                        })
                    {
                        self.hud.state.dismiss_popup();
                        self.hud.capture = Some(HudCapture {
                            control: frame.control(HudPart::Popup),
                            cancelled: true,
                            binding: self.hud_binding(world),
                            bounds: popup,
                            point: *position,
                            position: frame.position,
                            key: None,
                            item: None,
                            action: None,
                        });
                        return Some(ScreenInputResponse::handled(true));
                    }
                }
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
                let action = frame.action(control.part);
                let cancelled = !self.hud_action_valid(world, control, bounds, action.as_ref())
                    || !self.hud_control_visible(world, control, *position);
                self.hud.capture = Some(HudCapture {
                    control,
                    cancelled,
                    binding,
                    bounds,
                    point: *position,
                    position: frame.position,
                    key: None,
                    item: None,
                    action,
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
                    if frame
                        .detail
                        .as_ref()
                        .is_some_and(|detail| rect(detail.rect).contains(self.cursor))
                    {
                        self.hud.scroll_detail(
                            frame.id,
                            delta[1]
                                * if *precise {
                                    1.
                                } else {
                                    frame.style.font_size * 3.
                                },
                        );
                    } else {
                        self.hud.state.scroll(frame, *delta, *precise);
                    }
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
                        if self.hud.state.popup_open() {
                            self.hud.state.dismiss_popup();
                            self.hud.focused =
                                frame.popup_trigger().map(|part| frame.control(part));
                        } else {
                            self.hud.state.close_panel(frame.id);
                            self.hud.focused = None;
                        }
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
                            .filter(|(id, _)| {
                                if frame.popup.is_some() {
                                    frame
                                        .buttons
                                        .iter()
                                        .any(|button| button.popup && button.part == id.part)
                                } else {
                                    matches!(
                                        id.part,
                                        HudPart::Close
                                            | HudPart::Slot(_)
                                            | HudPart::Category(_)
                                            | HudPart::Sort
                                            | HudPart::PreviousPage
                                            | HudPart::NextPage
                                            | HudPart::CatalogEntry(_)
                                            | HudPart::CatalogTab(_)
                                            | HudPart::Assign
                                            | HudPart::Detail
                                            | HudPart::Preference(_)
                                            | HudPart::ResetPositions
                                    )
                                }
                            })
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
                        if frame.popup.is_some() {
                            let options: Vec<_> =
                                frame.buttons.iter().filter(|button| button.popup).collect();
                            if !options.is_empty() {
                                let old = self
                                    .hud
                                    .focused
                                    .and_then(|id| {
                                        options.iter().position(|button| button.part == id.part)
                                    })
                                    .unwrap_or(0);
                                let next = match physical {
                                    PhysicalKey::Up | PhysicalKey::Left => {
                                        (old + options.len() - 1) % options.len()
                                    }
                                    PhysicalKey::Home | PhysicalKey::PageUp => 0,
                                    PhysicalKey::End | PhysicalKey::PageDown => options.len() - 1,
                                    _ => (old + 1) % options.len(),
                                };
                                self.hud.focused = Some(frame.control(options[next].part));
                            }
                            return Some(ScreenInputResponse::handled(true));
                        }
                        let step = frame.style.scroll_step;
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
                        if self
                            .hud
                            .focused
                            .is_some_and(|id| id.part == HudPart::Detail)
                        {
                            let detail = self
                                .hud
                                .prepared
                                .iter()
                                .find(|panel| panel.frame.id == frame.id)
                                .and_then(|panel| panel.detail.as_ref());
                            let amount = match physical {
                                PhysicalKey::Home => detail.map_or(0., |detail| detail.scroll),
                                PhysicalKey::End => -detail.map_or(0., |detail| detail.max_scroll),
                                _ => delta[1],
                            };
                            self.hud.scroll_detail(frame.id, amount);
                        } else {
                            self.hud.state.scroll(frame, delta, true);
                        }
                        if !self
                            .hud
                            .focused
                            .is_some_and(|id| id.part == HudPart::Detail)
                        {
                            self.hud.focused = Some(frame.control(HudPart::Body));
                        }
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
                    let action = frame.action(control.part);
                    let cancelled = !self.hud_action_valid(world, control, bounds, action.as_ref())
                        || receipt.is_err();
                    self.hud.capture = Some(HudCapture {
                        control,
                        cancelled,
                        binding: self.hud_binding(world),
                        bounds,
                        point: [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.],
                        position: frame.position,
                        key: Some(*physical),
                        item: receipt.ok().flatten(),
                        action,
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
