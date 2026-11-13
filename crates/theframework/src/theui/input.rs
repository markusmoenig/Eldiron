use super::*;
use crate::platform::{InputEvent, InputResponse, LogicalKey, PointerButton};
impl TheUI {
    /// Only the focused editor in the foremost painted input scope may accept
    /// native text. Suspending a scope cancels preedit/clipboard without losing
    /// committed content or undo history, and changes the native callback token.
    pub(crate) fn synchronize_editor_scope(&mut self, ctx: &mut TheContext) -> bool {
        let Some(id) = ctx.ui.focus.clone() else {
            return false;
        };
        let mut visible = false;
        if self.context_menu.is_none() && ctx.ui.overlay.is_none() {
            let mut find = |widget: &mut dyn TheWidget| {
                if widget.id().uuid == id.uuid
                    && widget.supports_editor_focus()
                    && !widget.disabled()
                    && !ctx.ui.is_disabled(&id.name)
                {
                    visible = true;
                }
            };
            if let Some(popover) = &mut self.popover {
                popover.canvas.visit_visible_widgets(&mut find);
            } else if let Some(dialog) = &mut self.dialog {
                dialog.visit_visible_widgets(&mut find);
            } else {
                self.canvas.visit_visible_widgets(&mut find);
            }
        }
        if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
            widget.editor_set_active(visible, ctx);
        }
        visible
    }
    fn advance_editor_focus(&mut self, ctx: &mut TheContext, backwards: bool) {
        let mut targets = Vec::new();
        let screen = crate::paint::Rect::new(0., 0., ctx.width as f32, ctx.height as f32);
        let mut collect = |widget: &mut dyn TheWidget| {
            if !widget.supports_editor_focus()
                || widget.disabled()
                || ctx.ui.is_disabled(&widget.id().name)
            {
                return;
            }
            let dim = widget.dim();
            let transform = ctx
                .ui
                .input_transforms
                .get(&widget.id().uuid)
                .copied()
                .unwrap_or_default();
            let p = transform.point([dim.x as f32, dim.y as f32]);
            let r = crate::paint::Rect::new(p[0], p[1], dim.width as f32, dim.height as f32)
                .intersection(screen);
            if r.width > 0. && r.height > 0. {
                targets.push(widget.id().clone());
            }
        };
        if let Some(popover) = &mut self.popover {
            popover.canvas.visit_visible_widgets(&mut collect);
        } else if let Some(dialog) = &mut self.dialog {
            dialog.visit_visible_widgets(&mut collect);
        } else {
            self.canvas.visit_visible_widgets(&mut collect);
        }
        let current = targets
            .iter()
            .position(|id| Some(id) == ctx.ui.focus.as_ref());
        let next = if targets.is_empty() {
            None
        } else if let Some(index) = current {
            Some(
                targets[if backwards {
                    (index + targets.len() - 1) % targets.len()
                } else {
                    (index + 1) % targets.len()
                }]
                .clone(),
            )
        } else if backwards {
            targets.last().cloned()
        } else {
            targets.first().cloned()
        };
        if let Some(next) = next {
            ctx.ui.set_focus(&next);
        } else {
            ctx.ui.clear_focus();
        }
    }
    pub(super) fn focused_editor_command(
        &mut self,
        command: crate::text_edit::EditorCommand,
        ctx: &mut TheContext,
    ) -> bool {
        if !ctx.ui.command_paint {
            return false;
        }
        let visible = self.synchronize_editor_scope(ctx);
        let Some(id) = ctx.ui.focus.clone() else {
            return false;
        };
        if !visible {
            // A suspended native owner must not fall through to the legacy
            // event/OS clipboard path behind a menu or another input scope.
            return self
                .get_widget_abs(None, Some(&id.uuid))
                .is_some_and(|widget| widget.uses_native_editor());
        }
        let Some(response) = self
            .get_widget_abs(None, Some(&id.uuid))
            .and_then(|widget| widget.editor_command(command, ctx))
        else {
            return false;
        };
        self.is_dirty |= response.input.redraw;
        self.process_events(ctx);
        true
    }
    /// Consumption is based on routing/capture, never inferred from a redraw flag.
    /// Exactly one widget path receives each input operation.
    pub fn dispatch_input(&mut self, event: &InputEvent, ctx: &mut TheContext) -> InputResponse {
        let modal = self.dialog.is_some()
            || self.popover.is_some()
            || self.context_menu.is_some()
            || ctx.ui.overlay.is_some();
        let mut result = InputResponse {
            consumed: modal,
            redraw: false,
        };
        let editor_scope = if ctx.ui.command_paint {
            self.synchronize_editor_scope(ctx)
        } else {
            true
        };
        if !editor_scope
            && modal
            && matches!(
                event,
                InputEvent::Text(_)
                    | InputEvent::TextFor { .. }
                    | InputEvent::Composition { .. }
                    | InputEvent::CompositionFor { .. }
                    | InputEvent::NativeText { .. }
                    | InputEvent::Clipboard { .. }
            )
        {
            return result;
        }
        if editor_scope
            && ctx.ui.command_paint
            && matches!(
                event,
                InputEvent::Key { .. }
                    | InputEvent::Text(_)
                    | InputEvent::TextFor { .. }
                    | InputEvent::Composition { .. }
                    | InputEvent::CompositionFor { .. }
                    | InputEvent::NativeText { .. }
                    | InputEvent::Clipboard { .. }
            )
        {
            if let Some(id) = ctx.ui.focus.clone() {
                let response = self
                    .get_widget_abs(None, Some(&id.uuid))
                    .and_then(|widget| widget.editor_input(event, ctx));
                if let Some(response) = response {
                    if let Some(crate::text_edit::EditorAction::FocusNext { backwards }) =
                        response.action
                    {
                        self.advance_editor_focus(ctx, backwards);
                    }
                    if response.action == Some(crate::text_edit::EditorAction::Cancel) {
                        if self.popover.is_some() {
                            self.clear_popover(ctx);
                        } else if self.dialog.is_some() {
                            self.clear_dialog();
                            ctx.ui.clear_focus();
                            ctx.ui.clear_hover();
                            ctx.ui.clear_overlay();
                            ctx.ui.redraw_all = true;
                        }
                    }
                    self.is_dirty |= response.input.redraw;
                    self.process_events(ctx);
                    if !response.input.consumed {
                        if let InputEvent::Key {
                            physical: crate::platform::PhysicalKey::Letter(letter),
                            pressed: true,
                            ..
                        } = event
                        {
                            let character = letter.to_ascii_lowercase() as char;
                            if ctx.ui.accelerators.values().any(|accel| {
                                accel.matches(self.shift, self.ctrl, self.alt, self.logo, character)
                            }) {
                                let redraw = self.key_down(Some(character), None, ctx);
                                self.process_events(ctx);
                                return InputResponse {
                                    consumed: true,
                                    redraw,
                                };
                            }
                        }
                    }
                    return InputResponse {
                        consumed: result.consumed || response.input.consumed,
                        redraw: response.input.redraw,
                    };
                }
            }
        }
        match event {
            InputEvent::PointerMoved { position } => {
                self.mouse_coord = Vec2::new(position[0] as i32, position[1] as i32);
                let captured = self.mouse_capture_id.is_some();
                result.consumed |= captured || self.get_widget_at_coord(self.mouse_coord).is_some();
                result.redraw = if captured {
                    self.touch_dragged(position[0], position[1], ctx)
                } else {
                    self.hover(position[0], position[1], ctx)
                };
            }
            InputEvent::PointerButton {
                button,
                pressed,
                position,
            } => {
                self.mouse_coord = Vec2::new(position[0] as i32, position[1] as i32);
                result.consumed |= self.mouse_capture_id.is_some()
                    || self.get_widget_at_coord(self.mouse_coord).is_some();
                match button {
                    PointerButton::Primary => {
                        result.redraw = if *pressed {
                            self.touch_down(position[0], position[1], ctx)
                        } else {
                            self.touch_up(position[0], position[1], ctx)
                        }
                    }
                    PointerButton::Secondary => {
                        self.right_mouse_down = *pressed;
                        if *pressed {
                            result.redraw = self.context(position[0], position[1], ctx);
                        }
                    }
                    _ => {}
                }
            }
            InputEvent::Scroll { delta, precise } => {
                result.consumed |= self.get_widget_at_coord(self.mouse_coord).is_some();
                if let Some(id) = self.get_layout_at_coord(self.mouse_coord) {
                    if let Some(layout) = self.get_layout(&id.name) {
                        result.consumed |= layout.supports_mouse_wheel();
                    }
                }
                result.redraw = if *precise {
                    self.precise_scroll((delta[0], delta[1]), ctx)
                } else {
                    self.mouse_wheel((delta[0] as i32, delta[1] as i32), ctx)
                };
            }
            InputEvent::Pinch(delta) => {
                result.consumed |= self.get_widget_at_coord(self.mouse_coord).is_some();
                result.redraw = self.pinch(*delta, ctx);
            }
            InputEvent::Modifiers(m) => {
                result.redraw = self.modifier_changed(m.shift, m.control, m.alt, m.super_key, ctx);
            }
            InputEvent::Key {
                logical, pressed, ..
            } => {
                result.consumed |= self.focus_widget_supports_text_input(ctx);
                match logical {
                    LogicalKey::Named(key) => {
                        result.redraw = if *pressed {
                            self.key_down(None, Some(key.clone()), ctx)
                        } else {
                            self.key_up(None, Some(key.clone()), ctx)
                        };
                    }
                    LogicalKey::Character(text) if (self.ctrl && !self.alt) || self.logo => {
                        if let Some(c) = text.chars().next() {
                            if *pressed {
                                let clipboard = self.focus_widget_supports_clipboard(ctx);
                                let undo = self.focus_widget_supports_undo_redo(ctx);
                                let handled = match c.to_ascii_lowercase() {
                                    'z' if undo => {
                                        if self.shift {
                                            self.redo(ctx);
                                        } else {
                                            self.undo(ctx);
                                        }
                                        true
                                    }
                                    'y' if undo => {
                                        self.redo(ctx);
                                        true
                                    }
                                    'c' if clipboard => {
                                        self.copy(ctx);
                                        true
                                    }
                                    'x' if clipboard => {
                                        self.cut(ctx);
                                        true
                                    }
                                    'v' if clipboard => {
                                        self.paste(ctx);
                                        true
                                    }
                                    _ => false,
                                };
                                if handled {
                                    result.consumed = true;
                                    result.redraw = true;
                                    return result;
                                }
                            }
                            let accelerator =
                                ctx.ui.accelerators.values().any(|a| {
                                    a.matches(self.shift, self.ctrl, self.alt, self.logo, c)
                                });
                            result.consumed |= accelerator;
                            result.redraw = if *pressed {
                                self.key_down(Some(c), None, ctx)
                            } else {
                                self.key_up(Some(c), None, ctx)
                            };
                        }
                    }
                    _ => {}
                }
            }
            InputEvent::Text(text) => {
                if let Some(id) = ctx.ui.focus.clone() {
                    if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                        if widget.supports_text_input() {
                            result.consumed = true;
                            result.redraw =
                                widget.on_event(&TheEvent::TextInput(text.clone()), ctx);
                        }
                    }
                }
                self.process_events(ctx);
            }
            InputEvent::Composition { text, cursor } => {
                if let Some(id) = ctx.ui.focus.clone() {
                    if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                        if widget.supports_text_input() {
                            result.consumed = true;
                            result.redraw = widget
                                .on_event(&TheEvent::TextComposition(text.clone(), *cursor), ctx);
                        }
                    }
                }
                self.process_events(ctx);
            }
            InputEvent::Focus(false) => {
                self.mouse_capture_id = None;
                self.right_mouse_down = false;
                ctx.ui.clear_focus();
                ctx.ui.clear_hover();
                ctx.ui.clear_overlay();
                self.modifier_changed(false, false, false, false, ctx);
                self.process_events(ctx);
                result.redraw = true;
            }
            InputEvent::PointerLeft => {
                if self.mouse_capture_id.is_none() {
                    ctx.ui.clear_hover();
                    self.process_events(ctx);
                    result.redraw = true;
                }
            }
            InputEvent::RelativeMotion { .. }
            | InputEvent::Focus(true)
            | InputEvent::FileDropped(_)
            | InputEvent::Clipboard { .. }
            | InputEvent::TextFor { .. }
            | InputEvent::ClipboardImage { .. }
            | InputEvent::CompositionFor { .. }
            | InputEvent::NativeText { .. } => {}
        }
        result
    }
}

#[cfg(test)]
mod native_owner_tests {
    use super::*;
    #[test]
    fn nonselectable_native_display_still_owns_commands_and_prevents_legacy_clipboard_access() {
        let mut ctx = TheContext::new(300, 100, 1.);
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        let id = TheId::named("Nonselectable help");
        let mut help = TheTextView::new(id.clone());
        help.set_text("Visible hint".into());
        help.set_selectable(false);
        ui.canvas.set_widget(help);
        ui.prepare_paint_layout(&mut ctx);
        ctx.ui.set_focus(&id);
        ui.process_events(&mut ctx);
        let widget = ui.get_widget_id(id.uuid).unwrap();
        assert!(widget.uses_native_editor());
        assert!(!widget.supports_editor_focus());
        for command in [
            crate::text_edit::EditorCommand::Copy,
            crate::text_edit::EditorCommand::Paste,
            crate::text_edit::EditorCommand::Cut,
        ] {
            assert!(
                ui.focused_editor_command(command, &mut ctx),
                "must stop before the legacy event or OS clipboard path"
            );
        }
        assert!(ctx.ui.editor_clipboard.is_empty());
    }
}
