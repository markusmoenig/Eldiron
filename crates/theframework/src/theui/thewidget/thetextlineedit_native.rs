//! Native single-line editor. The common controller owns editing state; the
//! legacy text value remains a compatibility projection for existing APIs.
use super::*;
use crate::{
    platform::{InputEvent, LogicalKey, Modifiers, PhysicalKey, PointerButton},
    text_edit::{EditorAction, EditorCommand, EditorResponse},
};
impl TheTextLineEdit {
    pub(super) fn native_focused(&self, ctx: &TheContext) -> bool {
        self.native_active
            && !self.is_disabled
            && if self.embedded {
                self.has_parent_focus(ctx)
            } else {
                ctx.ui.has_focus(&self.id)
            }
    }
    pub(super) fn cancel_native_gesture(&mut self, ctx: &mut TheContext) {
        let was_dragging = std::mem::take(&mut self.native_dragging);
        // A queued blur must never project a cancelled gesture over a newer
        // externally supplied document/value. prepare will install that value.
        if self.native_document_changed {
            return;
        }
        if was_dragging {
            self.modified_since_last_return = self.native_modified_before_drag;
        }
        if self.native_editor.editor.cancel_edit_group(&mut ctx.text) {
            self.state
                .set_text(self.native_editor.editor.text().to_owned());
            self.is_dirty = true;
            if self.continuous {
                self.notify_native_value(ctx);
            }
        }
    }
    pub(super) fn finish_native_gesture(&mut self, ctx: &mut TheContext) {
        let was_dragging = std::mem::take(&mut self.native_dragging);
        if was_dragging && self.is_range() {
            self.native_editor.editor.commit_edit_group();
            if !self.continuous && self.state.to_text() != self.original {
                self.notify_native_value(ctx);
            }
            self.original = self.state.to_text();
            self.modified_since_last_return = false;
        }
    }
    pub(super) fn prepare_native_editor(&mut self, ctx: &mut TheContext) {
        if self.native_document_changed {
            self.native_dragging = false;
        } else if self.native_dragging && !self.native_focused(ctx) {
            self.cancel_native_gesture(ctx);
        }
        let text = self.state.to_text();
        if self.native_document_changed || self.native_editor.editor.text() != text {
            self.native_editor.editor.set_text(&mut ctx.text, &text);
            if self.native_editor.editor.text() != text {
                self.state
                    .set_text(self.native_editor.editor.text().to_owned());
            }
            self.native_document_changed = false;
        }
        let focused = self.native_focused(ctx);
        self.native_editor.set_focused(&mut ctx.text, focused);
        self.native_editor.input(
            &mut ctx.text,
            &InputEvent::Modifiers(ctx.ui.input_modifiers),
        );
        let border = if self.embedded || self.frameless {
            0.
        } else {
            2.
        };
        let left = border + self.renderer.padding.0 as f32;
        let right = border + self.renderer.padding.2 as f32;
        self.native_editor.prepare(
            &mut ctx.text,
            crate::text::TextOptions {
                family: "Roboto",
                size: self.renderer.font_size,
                width: None,
                line_height: None,
            },
            crate::paint::Rect::new(
                self.dim.x as f32 + left,
                self.dim.y as f32 + border,
                (self.dim.width as f32 - left - right).max(0.),
                (self.dim.height as f32 - border * 2.).max(0.),
            ),
            true,
        );
    }
    fn notify_native_value(&mut self, ctx: &mut TheContext) {
        if let Some(layout) = &self.layout_id {
            ctx.ui.send(TheEvent::RedirectWidgetValueToLayout(
                layout.clone(),
                self.id.clone(),
                self.value(),
            ));
        } else {
            ctx.ui.send_widget_value_changed(&self.id, self.value());
        }
    }
    pub(super) fn apply_native_response(
        &mut self,
        response: &EditorResponse,
        ctx: &mut TheContext,
    ) {
        if response.changed {
            self.state
                .set_text(self.native_editor.editor.text().to_owned());
            self.modified_since_last_tick = true;
            self.modified_since_last_return = true;
            if self.continuous {
                self.notify_native_value(ctx);
            }
        }
        if let Some(request) = &response.clipboard {
            ctx.ui.editor_clipboard.push_back(request.clone());
        }
        if response.action == Some(EditorAction::Submit) && self.modified_since_last_return {
            self.notify_native_value(ctx);
            self.modified_since_last_return = false;
            if self.is_range() {
                self.original = self.state.to_text();
            }
            if self.id.name != "Console Input" && !self.embedded {
                ctx.ui.clear_focus();
            }
        } else if response.action == Some(EditorAction::Cancel) && !self.embedded {
            ctx.ui.clear_focus();
        }
        self.is_dirty |= response.input.redraw;
    }
    pub(super) fn native_legacy_event(
        &mut self,
        event: &TheEvent,
        ctx: &mut TheContext,
    ) -> Option<bool> {
        if self.is_disabled && !matches!(event, TheEvent::LostFocus(_)) {
            return Some(false);
        }
        let input = match event {
            TheEvent::GainedFocus(_) => {
                self.prepare_native_editor(ctx);
                return Some(true);
            }
            TheEvent::LostFocus(_) => {
                self.cancel_native_gesture(ctx);
                self.native_editor.set_focused(&mut ctx.text, false);
                self.modifier_shift = false;
                self.modifier_ctrl = false;
                self.modifier_alt = false;
                self.modifier_logo = false;
                self.is_dirty = true;
                return Some(true);
            }
            TheEvent::ModifierChanged(shift, ctrl, alt, logo) => {
                self.modifier_shift = *shift;
                self.modifier_ctrl = *ctrl;
                self.modifier_alt = *alt;
                self.modifier_logo = *logo;
                InputEvent::Modifiers(Modifiers {
                    shift: *shift,
                    control: *ctrl,
                    alt: *alt,
                    super_key: *logo,
                })
            }
            TheEvent::TextInput(text) => InputEvent::Text(text.clone()),
            TheEvent::TextComposition(text, cursor) => InputEvent::Composition {
                text: text.clone(),
                cursor: *cursor,
            },
            TheEvent::KeyDown(value) => {
                let character = value.to_char()?;
                if (self.modifier_ctrl && !self.modifier_alt) || self.modifier_logo {
                    InputEvent::Key {
                        physical: PhysicalKey::Letter(character.to_ascii_lowercase() as u8),
                        logical: LogicalKey::Character(character.to_string()),
                        pressed: true,
                        repeat: false,
                    }
                } else {
                    InputEvent::Text(character.to_string())
                }
            }
            TheEvent::KeyCodeDown(value) => {
                let physical = match value.to_key_code()? {
                    TheKeyCode::Escape => PhysicalKey::Escape,
                    TheKeyCode::Return => PhysicalKey::Enter,
                    TheKeyCode::Delete => PhysicalKey::Backspace,
                    TheKeyCode::Home => PhysicalKey::Home,
                    TheKeyCode::End => PhysicalKey::End,
                    TheKeyCode::Up => PhysicalKey::Up,
                    TheKeyCode::Right => PhysicalKey::Right,
                    TheKeyCode::Down => PhysicalKey::Down,
                    TheKeyCode::Left => PhysicalKey::Left,
                    TheKeyCode::Space => PhysicalKey::Space,
                    TheKeyCode::Tab => PhysicalKey::Tab,
                };
                InputEvent::Key {
                    physical,
                    logical: LogicalKey::Other,
                    pressed: true,
                    repeat: false,
                }
            }
            TheEvent::MouseDown(coord) => {
                if !self.embedded {
                    ctx.ui.set_focus(&self.id);
                }
                self.prepare_native_editor(ctx);
                let bounds = self.native_editor.bounds();
                let mut position = [
                    self.dim.x as f32 + coord.x as f32,
                    self.dim.y as f32 + coord.y as f32,
                ];
                if bounds.width <= 0. || bounds.height <= 0. {
                    return Some(true);
                }
                position[0] =
                    position[0].clamp(bounds.x, bounds.x + bounds.width - bounds.width.min(0.001));
                position[1] = position[1].clamp(
                    bounds.y,
                    bounds.y + bounds.height - bounds.height.min(0.001),
                );
                self.native_clicks = if self.last_mouse_down_time.elapsed().as_millis() < 500
                    && (*coord - self.last_mouse_down_coord).magnitude_squared() <= 16
                {
                    (self.native_clicks % 3) + 1
                } else {
                    1
                };
                self.native_editor
                    .pointer_down(&mut ctx.text, position, self.native_clicks);
                self.native_dragging = true;
                self.last_mouse_down_coord = *coord;
                self.last_mouse_down_time = Instant::now();
                if self.is_range() {
                    self.original = self.state.to_text();
                    self.native_modified_before_drag = self.modified_since_last_return;
                    self.native_editor
                        .editor
                        .execute(&mut ctx.text, EditorCommand::SelectAll);
                    self.native_editor.editor.begin_edit_group(&mut ctx.text);
                }
                self.is_dirty = true;
                return Some(true);
            }
            TheEvent::MouseDragged(coord) if self.native_dragging && self.is_range() => {
                let fraction = (coord.x as f64 / self.dim.width.max(1) as f64).clamp(0., 1.);
                let text = self.range.as_ref().and_then(|range| {
                    if let Some(r) = range.to_range_f32() {
                        Some(format!(
                            "{:.3}",
                            *r.start() as f64 + (*r.end() as f64 - *r.start() as f64) * fraction
                        ))
                    } else {
                        range.to_range_i32().map(|r| {
                            ((*r.start() as f64 + (*r.end() as f64 - *r.start() as f64) * fraction)
                                .trunc() as i32)
                                .to_string()
                        })
                    }
                })?;
                let changed = self.native_editor.editor.replace_text(&mut ctx.text, &text);
                let response = EditorResponse {
                    changed,
                    input: crate::platform::InputResponse {
                        consumed: true,
                        redraw: changed,
                    },
                    ..Default::default()
                };
                self.apply_native_response(&response, ctx);
                return Some(response.input.redraw);
            }
            TheEvent::MouseDragged(coord) => InputEvent::PointerMoved {
                position: [
                    self.dim.x as f32 + coord.x as f32,
                    self.dim.y as f32 + coord.y as f32,
                ],
            },
            TheEvent::MouseUp(coord) => {
                self.finish_native_gesture(ctx);
                InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed: false,
                    position: [
                        self.dim.x as f32 + coord.x as f32,
                        self.dim.y as f32 + coord.y as f32,
                    ],
                }
            }
            TheEvent::MouseWheel(delta) => InputEvent::Scroll {
                delta: [delta.x as f32 / 4., delta.y as f32 / 4.],
                precise: true,
            },
            TheEvent::Undo | TheEvent::Redo | TheEvent::Copy | TheEvent::Cut => {
                let command = match event {
                    TheEvent::Undo => EditorCommand::Undo,
                    TheEvent::Redo => EditorCommand::Redo,
                    TheEvent::Copy => EditorCommand::Copy,
                    _ => EditorCommand::Cut,
                };
                return self.editor_command(command, ctx).map(|r| r.input.redraw);
            }
            TheEvent::Paste(value, _) => InputEvent::Text(value.to_string()?),
            _ => return None,
        };
        self.editor_input(&input, ctx)
            .map(|response| response.input.redraw)
    }
}
