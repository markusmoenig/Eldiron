//! Native text-field adapter. Clipboard operations remain addressed to their
//! screen/widget; the application host completes them without advancing runtime.
use super::*;
use theframework::{
    platform::{InputEvent, InputResponse, PointerButton},
    text::TextOptions,
    text_edit::{ClipboardResult, EditorAction, EditorResponse, EditorView},
};
#[derive(Default)]
pub struct ScreenTextResponse {
    pub input: InputResponse,
    pub widget: Option<u32>,
    pub editor: EditorResponse,
}
impl GameScreenController {
    fn visible_inputs(&self) -> Vec<u32> {
        let mut ids: Vec<_> = self
            .elements
            .text_input_widgets
            .iter()
            .filter(|(_, w)| {
                !screen_model::widget_name_is_hidden(&w.name, &self.elements.widgets_to_hide)
                    && w.rect.width > 0.
                    && w.rect.height > 0.
            })
            .map(|(id, _)| *id)
            .collect();
        // Authored sector IDs provide stable traversal and stacking order,
        // independent of hash map allocation or layout changes.
        ids.sort_unstable();
        ids
    }
    pub(super) fn prepare_inputs(&mut self, fonts: &mut GameFonts) {
        self.inputs
            .retain(|id, _| self.elements.text_input_widgets.contains_key(id));
        let visible = self.visible_inputs();
        if self.focused_input.is_some_and(|id| !visible.contains(&id)) {
            self.focused_input = None;
        }
        for (id, widget) in &self.elements.text_input_widgets {
            let view = self
                .inputs
                .entry(*id)
                .or_insert_with(|| EditorView::new(false));
            if view.editor.text() != widget.text {
                view.editor.set_text(&mut fonts.text, &widget.text);
            }
            view.set_focused(&mut fonts.text, self.focused_input == Some(*id));
            if self.focused_input == Some(*id) {
                view.input(
                    &mut fonts.text,
                    &InputEvent::Modifiers(self.input_modifiers),
                );
            }
            if !visible.contains(id) {
                continue;
            }
            let bounds = Uv::new(
                widget.rect.x + 8.,
                widget.rect.y,
                (widget.rect.width - 16.).max(0.),
                widget.rect.height,
            );
            view.prepare(
                &mut fonts.text,
                TextOptions {
                    family: &widget.font,
                    size: widget.font_size,
                    width: None,
                    line_height: None,
                },
                bounds,
                true,
            );
        }
    }
    pub fn focus_input(&mut self, fonts: &mut GameFonts, id: Option<u32>) {
        self.focused_input = id.filter(|id| self.visible_inputs().contains(id));
        for (id, view) in &mut self.inputs {
            view.set_focused(&mut fonts.text, self.focused_input == Some(*id));
            if self.focused_input == Some(*id) {
                view.input(
                    &mut fonts.text,
                    &InputEvent::Modifiers(self.input_modifiers),
                );
            }
        }
    }
    fn publish_input(&mut self, id: u32) {
        let Some(view) = self.inputs.get(&id) else {
            return;
        };
        let Some(widget) = self.elements.text_input_widgets.get_mut(&id) else {
            return;
        };
        widget.text = view.editor.text().to_owned();
        let (binding, text) = (widget.binding.clone(), widget.text.clone());
        self.elements.set_binding(&binding, &text);
        self.geometry = None;
    }
    /// Route text fields before world/game controls. `prepare` supplies current
    /// hit geometry. Pointer positions and returned IME area are logical units.
    pub fn text_input_event(
        &mut self,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> ScreenTextResponse {
        let visible = self.visible_inputs();
        let old_focus = self.focused_input;
        if let InputEvent::Modifiers(modifiers) = event {
            self.input_modifiers = *modifiers;
        }
        if self.focused_input.is_some_and(|id| !visible.contains(&id)) {
            self.focus_input(fonts, None);
        }
        let mut focus_changed = old_focus != self.focused_input;
        if let InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position,
        } = event
        {
            let target = visible
                .iter()
                .rev()
                .find(|id| {
                    self.elements.text_input_widgets[id]
                        .rect
                        .contains(scenevm::prelude::Vec2::new(position[0], position[1]))
                })
                .copied();
            focus_changed |= target != self.focused_input;
            self.focus_input(fonts, target);
            // Chrome padding belongs to the field too. The editor's content
            // hit can be outside; focus still consumes this primary press.
            if let Some(id) = target {
                let mut result = self
                    .inputs
                    .get_mut(&id)
                    .map(|v| {
                        let bounds = v.bounds();
                        if bounds.width <= 0. || bounds.height <= 0. {
                            return EditorResponse::default();
                        }
                        let event = InputEvent::PointerButton {
                            button: PointerButton::Primary,
                            pressed: true,
                            position: [
                                position[0].clamp(
                                    bounds.x,
                                    bounds.x + bounds.width - bounds.width.min(0.001),
                                ),
                                position[1].clamp(
                                    bounds.y,
                                    bounds.y + bounds.height - bounds.height.min(0.001),
                                ),
                            ],
                        };
                        v.input(&mut fonts.text, &event)
                    })
                    .unwrap_or_default();
                result.input = InputResponse {
                    consumed: true,
                    redraw: true,
                };
                return ScreenTextResponse {
                    input: result.input,
                    widget: Some(id),
                    editor: result,
                };
            }
        }
        if matches!(event, InputEvent::Focus(false)) {
            focus_changed |= self.focused_input.is_some();
            self.input_modifiers = Default::default();
            self.focus_input(fonts, None);
        }
        let Some(id) = self.focused_input else {
            return ScreenTextResponse {
                input: InputResponse {
                    consumed: false,
                    redraw: focus_changed,
                },
                ..Default::default()
            };
        };
        let Some(view) = self.inputs.get_mut(&id) else {
            return ScreenTextResponse::default();
        };
        let editor = view.input(&mut fonts.text, event);
        if editor.changed {
            self.publish_input(id);
        }
        match editor.action {
            Some(EditorAction::FocusNext { backwards }) => {
                let index = visible.iter().position(|v| *v == id).unwrap_or(0);
                let next = if backwards {
                    (index + visible.len() - 1) % visible.len()
                } else {
                    (index + 1) % visible.len()
                };
                self.focus_input(fonts, Some(visible[next]));
            }
            Some(EditorAction::Submit | EditorAction::Cancel) => self.focus_input(fonts, None),
            _ => {}
        }
        ScreenTextResponse {
            input: editor.input,
            widget: Some(id),
            editor,
        }
    }
    pub fn complete_text_clipboard(
        &mut self,
        fonts: &mut GameFonts,
        widget: u32,
        request: u64,
        result: ClipboardResult,
    ) -> ScreenTextResponse {
        if self.focused_input != Some(widget) || !self.visible_inputs().contains(&widget) {
            return ScreenTextResponse::default();
        }
        let Some(view) = self.inputs.get_mut(&widget) else {
            return ScreenTextResponse::default();
        };
        let editor = view
            .editor
            .complete_clipboard(&mut fonts.text, request, result);
        if editor.input.redraw {
            view.reveal_caret(&mut fonts.text);
        }
        if editor.changed {
            self.publish_input(widget);
        }
        ScreenTextResponse {
            input: editor.input,
            widget: Some(widget),
            editor,
        }
    }
    pub fn text_ime_area(&mut self, fonts: &mut GameFonts) -> Option<Uv> {
        let id = self.focused_input?;
        if !self.visible_inputs().contains(&id) {
            return None;
        }
        self.inputs.get_mut(&id)?.ime_area(&mut fonts.text)
    }
}
