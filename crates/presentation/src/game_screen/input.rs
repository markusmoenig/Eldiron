//! One text-focus owner for fields and transcript commands. Clipboard work is
//! addressed to its original editor; runtime changes are emitted as effects.
use super::*;
use theframework::{
    platform::{InputEvent, InputResponse, PointerButton},
    text::TextOptions,
    text_edit::{ClipboardResult, EditorAction, EditorResponse, EditorView},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenTextTarget {
    Input(u32),
    Command(u32),
}
impl ScreenTextTarget {
    fn id(self) -> u32 {
        match self {
            Self::Input(id) | Self::Command(id) => id,
        }
    }
}
#[derive(Default)]
pub struct ScreenTextResponse {
    pub input: InputResponse,
    pub target: Option<ScreenTextTarget>,
    pub editor: EditorResponse,
    pub effects: Vec<ScreenEffect>,
}
struct TextSpec<'a> {
    target: ScreenTextTarget,
    hit: Uv,
    bounds: Uv,
    text: &'a str,
    family: &'a str,
    size: f32,
    line_height: Option<f32>,
    center: bool,
    hit_order: (u8, usize),
}
fn text_specs<'a>(
    elements: &'a ScreenElements,
    messages: &'a [MessageLayouts],
) -> Vec<TextSpec<'a>> {
    let visible =
        |name: &str| !screen_model::widget_name_is_hidden(name, &elements.widgets_to_hide);
    let mut specs = Vec::new();
    for (id, widget) in &elements.text_input_widgets {
        if visible(&widget.name) && visible_control_rect(elements, widget.rect) {
            specs.push(TextSpec {
                target: ScreenTextTarget::Input(*id),
                hit: rect(widget.rect),
                bounds: Uv::new(
                    widget.rect.x + 8.,
                    widget.rect.y,
                    (widget.rect.width - 16.).max(0.),
                    widget.rect.height,
                ),
                text: &widget.text,
                family: &widget.font,
                size: widget.font_size,
                line_height: None,
                center: true,
                hit_order: (1, *id as usize),
            });
        }
    }
    for (index, (widget, layout)) in elements.messages_widgets.iter().zip(messages).enumerate() {
        if !visible(&widget.name) {
            continue;
        }
        if let Some(command) = layout
            .command()
            .filter(|command| visible_control_rect(elements, command.hit))
        {
            specs.push(TextSpec {
                target: ScreenTextTarget::Command(widget.id),
                hit: rect(command.hit),
                bounds: rect(command.bounds),
                text: &widget.command_text,
                family: &command.family,
                size: command.size,
                line_height: Some(command.bounds.height),
                center: false,
                hit_order: (0, index),
            });
        }
    }
    specs.sort_by_key(|s| s.target.id());
    specs
}
impl GameScreenController {
    pub fn focused_text(&self) -> Option<ScreenTextTarget> {
        if let Some(ScreenFocus::Text(target)) = self.focus {
            Some(target)
        } else {
            None
        }
    }
    pub fn focused_input(&self) -> Option<u32> {
        if let Some(ScreenTextTarget::Input(id)) = self.focused_text() {
            Some(id)
        } else {
            None
        }
    }
    pub(super) fn text_targets(&self) -> Vec<ScreenTextTarget> {
        text_specs(&self.elements, &self.messages)
            .iter()
            .map(|s| s.target)
            .collect()
    }
    pub(super) fn prepare_inputs(&mut self, fonts: &mut GameFonts) {
        self.inputs.retain(|target, _| match target {
            ScreenTextTarget::Input(id) => self.elements.text_input_widgets.contains_key(id),
            ScreenTextTarget::Command(id) => {
                self.elements.messages_widgets.iter().any(|w| w.id == *id)
            }
        });
        if self
            .focused_text()
            .is_some_and(|target| !self.text_targets().contains(&target))
        {
            self.focus_control(fonts, None);
        }
        let focused = self.focused_text();
        let specs = text_specs(&self.elements, &self.messages);
        for (target, view) in &mut self.inputs {
            view.set_focused(&mut fonts.text, focused == Some(*target));
        }
        for spec in specs {
            let view = self
                .inputs
                .entry(spec.target)
                .or_insert_with(|| EditorView::new(false));
            if view.editor.text() != spec.text {
                view.editor.set_text(&mut fonts.text, spec.text);
            }
            view.set_focused(&mut fonts.text, focused == Some(spec.target));
            if focused == Some(spec.target) {
                view.input(
                    &mut fonts.text,
                    &InputEvent::Modifiers(self.input_modifiers),
                );
            }
            view.prepare(
                &mut fonts.text,
                TextOptions {
                    family: spec.family,
                    size: spec.size,
                    width: None,
                    line_height: spec.line_height,
                },
                spec.bounds,
                spec.center,
            );
        }
        for widget in &mut self.elements.messages_widgets {
            widget.command_active = focused == Some(ScreenTextTarget::Command(widget.id));
        }
    }
    pub fn focus_text(&mut self, fonts: &mut GameFonts, target: Option<ScreenTextTarget>) {
        self.focus_control(fonts, target.map(ScreenFocus::Text));
    }
    pub(super) fn sync_text_focus(&mut self, fonts: &mut GameFonts) {
        let focused = self.focused_text();
        for (target, view) in &mut self.inputs {
            view.set_focused(&mut fonts.text, focused == Some(*target));
            if focused == Some(*target) {
                view.input(
                    &mut fonts.text,
                    &InputEvent::Modifiers(self.input_modifiers),
                );
            }
        }
        for widget in &mut self.elements.messages_widgets {
            widget.command_active = focused == Some(ScreenTextTarget::Command(widget.id));
        }
    }
    pub fn focus_input(&mut self, fonts: &mut GameFonts, id: Option<u32>) {
        self.focus_text(fonts, id.map(ScreenTextTarget::Input));
    }
    fn publish_input(&mut self, target: ScreenTextTarget) {
        let Some(view) = self.inputs.get(&target) else {
            return;
        };
        match target {
            ScreenTextTarget::Input(id) => {
                let Some(widget) = self.elements.text_input_widgets.get_mut(&id) else {
                    return;
                };
                widget.text = view.editor.text().to_owned();
                let (binding, text) = (widget.binding.clone(), widget.text.clone());
                self.elements.set_binding(&binding, &text);
                self.geometry = None;
            }
            ScreenTextTarget::Command(id) => {
                if let Some(widget) = self
                    .elements
                    .messages_widgets
                    .iter_mut()
                    .find(|w| w.id == id)
                {
                    widget.command_text = view.editor.text().to_owned();
                }
            }
        }
    }
    fn clear_command(&mut self, fonts: &mut GameFonts, target: ScreenTextTarget) {
        if let Some(view) = self.inputs.get_mut(&target) {
            view.editor.set_text(&mut fonts.text, "");
        }
        self.publish_input(target);
        self.focus_text(fonts, None);
    }
    /// Prepared hit, selection, paint and IME geometry all use screen logical units.
    pub fn text_input_event(
        &mut self,
        fonts: &mut GameFonts,
        event: &InputEvent,
    ) -> ScreenTextResponse {
        let visible = self.text_targets();
        let old_focus = self.focused_text();
        if let InputEvent::Modifiers(modifiers) = event {
            self.input_modifiers = *modifiers;
        }
        if self
            .focused_text()
            .is_some_and(|target| !visible.contains(&target))
        {
            self.focus_text(fonts, None);
        }
        let mut focus_changed = old_focus != self.focused_text();
        if let InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position,
        } = event
        {
            // Fields paint over messages, regardless of their authored IDs.
            let target = text_specs(&self.elements, &self.messages)
                .into_iter()
                .filter(|s| {
                    s.hit.contains(*position)
                        && Uv::new(
                            0.,
                            0.,
                            self.elements.layout.viewport.x as f32,
                            self.elements.layout.viewport.y as f32,
                        )
                        .contains(*position)
                })
                .max_by_key(|s| s.hit_order)
                .map(|s| s.target);
            focus_changed |= target != self.focused_text();
            self.focus_text(fonts, target);
            if let Some(target) = target {
                let mut editor = self
                    .inputs
                    .get_mut(&target)
                    .map(|view| {
                        let bounds = view.bounds();
                        if bounds.width <= 0. || bounds.height <= 0. {
                            return EditorResponse::default();
                        }
                        view.input(
                            &mut fonts.text,
                            &InputEvent::PointerButton {
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
                            },
                        )
                    })
                    .unwrap_or_default();
                editor.input = InputResponse {
                    consumed: true,
                    redraw: true,
                };
                return ScreenTextResponse {
                    input: editor.input,
                    target: Some(target),
                    editor,
                    ..Default::default()
                };
            }
        }
        if matches!(event, InputEvent::Focus(false)) {
            focus_changed |= self.focused_text().is_some();
            self.input_modifiers = Default::default();
            self.focus_text(fonts, None);
        }
        let Some(target) = self.focused_text() else {
            return ScreenTextResponse {
                input: InputResponse {
                    consumed: false,
                    redraw: focus_changed,
                },
                ..Default::default()
            };
        };
        let Some(view) = self.inputs.get_mut(&target) else {
            return ScreenTextResponse::default();
        };
        let editor = view.input(&mut fonts.text, event);
        if editor.changed {
            self.publish_input(target);
        }
        let mut effects = Vec::new();
        match editor.action {
            Some(EditorAction::Submit) => {
                if matches!(target, ScreenTextTarget::Command(_)) {
                    let command = self.inputs[&target].editor.text().trim().to_owned();
                    if !command.is_empty() {
                        effects.push(ScreenEffect::Action(rusterix::EntityAction::TextCommand(
                            command,
                        )));
                    }
                    self.clear_command(fonts, target);
                } else {
                    self.focus_text(fonts, None);
                }
            }
            Some(EditorAction::Cancel) => {
                if matches!(target, ScreenTextTarget::Command(_)) {
                    self.clear_command(fonts, target);
                } else {
                    self.focus_text(fonts, None);
                }
            }
            _ => {}
        }
        ScreenTextResponse {
            input: editor.input,
            target: Some(target),
            editor,
            effects,
        }
    }
    pub fn complete_editor_clipboard(
        &mut self,
        fonts: &mut GameFonts,
        target: ScreenTextTarget,
        request: u64,
        result: ClipboardResult,
    ) -> ScreenTextResponse {
        if self.focused_text() != Some(target) || !self.text_targets().contains(&target) {
            return ScreenTextResponse::default();
        }
        let Some(view) = self.inputs.get_mut(&target) else {
            return ScreenTextResponse::default();
        };
        let editor = view
            .editor
            .complete_clipboard(&mut fonts.text, request, result);
        if editor.input.redraw {
            view.reveal_caret(&mut fonts.text);
        }
        if editor.changed {
            self.publish_input(target);
        }
        ScreenTextResponse {
            input: editor.input,
            target: Some(target),
            editor,
            ..Default::default()
        }
    }
    /// Compatibility convenience for a named authored field.
    pub fn complete_text_clipboard(
        &mut self,
        fonts: &mut GameFonts,
        widget: u32,
        request: u64,
        result: ClipboardResult,
    ) -> ScreenTextResponse {
        self.complete_editor_clipboard(fonts, ScreenTextTarget::Input(widget), request, result)
    }
    pub fn text_input_state(
        &mut self,
        fonts: &mut GameFonts,
    ) -> Option<theframework::platform::TextInputState> {
        let target = self.focused_text()?;
        if !self.text_targets().contains(&target) {
            return None;
        }
        self.inputs
            .get_mut(&target)?
            .text_input_state(&mut fonts.text)
    }
    pub fn text_ime_area(&mut self, fonts: &mut GameFonts) -> Option<Uv> {
        let target = self.focused_text()?;
        if !self.text_targets().contains(&target) {
            return None;
        }
        self.inputs.get_mut(&target)?.ime_area(&mut fonts.text)
    }
}
