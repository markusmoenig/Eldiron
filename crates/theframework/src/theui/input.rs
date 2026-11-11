use super::*;
use crate::platform::{InputEvent, InputResponse, LogicalKey, PointerButton};
impl TheUI {
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
            | InputEvent::FileDropped(_) => {}
        }
        result
    }
}
