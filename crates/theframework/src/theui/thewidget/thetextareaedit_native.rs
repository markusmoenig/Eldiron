//! Native multiline editing and paint. The common editor owns geometry/history;
//! the legacy renderer supplies authored configuration/highlighter/reference APIs.
use super::*;
use crate::{
    paint::{PaintOp, Rect},
    platform::{InputEvent, InputResponse, LogicalKey, PhysicalKey, PointerButton},
    text::TextOptions,
    text_edit::{
        EditorColors, EditorCommand, EditorDecorations, EditorPolicy, EditorResponse, EditorTab,
        EditorUndo, EditorView,
    },
    theui::paint::{UiPaintResult, WidgetPainter, bounds, rgba},
};
use std::sync::Arc;
#[derive(Clone, Copy)]
pub(super) enum Capture {
    Text,
    Horizontal,
    Vertical,
}
pub(super) struct NativeArea {
    pub view: EditorView,
    pub active: bool,
    pub replaced: bool,
    pub selection_changed: bool,
    pub annotations_dirty: bool,
    pub capture: Option<Capture>,
    annotations: Arc<EditorDecorations>,
    snapshot: u64,
    lines: Vec<usize>,
    horizontal: Option<Rect>,
    vertical: Option<Rect>,
    colors: EditorColors,
    clicks: u8,
}
impl NativeArea {
    pub fn new() -> Self {
        Self {
            view: EditorView::new(true),
            active: true,
            replaced: true,
            selection_changed: false,
            annotations_dirty: true,
            capture: None,
            annotations: Arc::new(EditorDecorations::default()),
            snapshot: 0,
            lines: vec![0],
            horizontal: None,
            vertical: None,
            colors: EditorColors::default(),
            clicks: 0,
        }
    }
}
impl TheTextAreaEdit {
    pub(super) fn cancel_native_area_capture(&mut self, ctx: &mut TheContext) {
        match self.native.capture.take() {
            Some(Capture::Horizontal) => {
                self.hscrollbar
                    .on_event(&TheEvent::MouseUp(Vec2::zero()), ctx);
            }
            Some(Capture::Vertical) => {
                self.vscrollbar
                    .on_event(&TheEvent::MouseUp(Vec2::zero()), ctx);
            }
            _ => {}
        }
        self.is_hscrollbar_clicked = false;
        self.is_vscrollbar_clicked = false;
    }
    pub(super) fn native_code_shortcut(
        &mut self,
        event: &InputEvent,
        ctx: &mut TheContext,
    ) -> Option<EditorResponse> {
        let InputEvent::Key {
            logical: LogicalKey::Character(key),
            pressed: true,
            repeat,
            ..
        } = event
        else {
            return None;
        };
        let modifiers = ctx.ui.input_modifiers;
        if !(modifiers.control && !modifiers.alt || modifiers.super_key) {
            return None;
        }
        let changed = match key.as_str() {
            "+" => {
                self.renderer.font_size += 1.;
                false
            }
            "-" => {
                self.renderer.font_size = (self.renderer.font_size - 1.).max(5.);
                false
            }
            "/" => {
                if *repeat {
                    return Some(EditorResponse {
                        input: InputResponse {
                            consumed: true,
                            redraw: false,
                        },
                        ..Default::default()
                    });
                }
                if self
                    .renderer
                    .highlighter
                    .as_ref()
                    .is_some_and(|highlighter| matches!(highlighter.syntax(), "Python" | "TOML"))
                {
                    self.native
                        .view
                        .editor
                        .toggle_line_comment(&mut ctx.text, "# ")
                } else {
                    false
                }
            }
            _ => return None,
        };
        Some(EditorResponse {
            changed,
            input: InputResponse {
                consumed: true,
                redraw: true,
            },
            ..Default::default()
        })
    }
    pub(super) fn ensure_native_editor(&mut self, ctx: &mut TheContext) {
        if self.native.replaced {
            self.cancel_native_area_capture(ctx);
            self.native.view.set_focused(&mut ctx.text, false);
            self.native
                .view
                .editor
                .import_state(&mut ctx.text, &self.state);
            self.native.replaced = false;
            self.native.selection_changed = false;
            self.native.capture = None;
        } else if self.native.selection_changed {
            self.native
                .view
                .editor
                .restore_selection_state(&mut ctx.text, &self.state);
            self.native.selection_changed = false;
        }
        self.native.view.editor.set_policy(
            &mut ctx.text,
            EditorPolicy {
                read_only: self.readonly || self.is_disabled,
                single_line_input: false,
                undo: if self.supports_undo {
                    EditorUndo::Internal
                } else {
                    EditorUndo::External
                },
                tab: EditorTab::Spaces(self.state.tab_spaces),
                auto_indent: self.state.auto_indent,
                auto_brackets: self.state.auto_bracket_completion,
            },
        );
        self.native.view.set_auto_reveal(self.auto_scroll_to_cursor);
        self.native.view.set_selection_drag_move(!self.readonly);

        self.native.view.set_focused(
            &mut ctx.text,
            self.native.active && !self.is_disabled && ctx.ui.has_focus(&self.id),
        );
        self.native.view.input(
            &mut ctx.text,
            &InputEvent::Modifiers(ctx.ui.input_modifiers),
        );
    }
    pub(super) fn prepare_native_area(&mut self, ctx: &mut TheContext, theme: &dyn TheTheme) {
        self.ensure_native_editor(ctx);
        let bg = *theme.color(TextEditBackground);
        let light = bg[..3].iter().map(|v| *v as u16).sum::<u16>() > 480;
        if self.last_light_ui != Some(light) {
            if let Some(highlighter) = self.renderer.highlighter.as_mut() {
                highlighter.set_light_ui(light);
            }
            self.last_light_ui = Some(light);
            self.native.annotations_dirty = true;
        }
        self.native.colors = EditorColors {
            text: rgba(*theme.color(TextEditTextColor)),
            selection: self
                .renderer
                .highlighter
                .as_ref()
                .and_then(|h| h.selection_background())
                .map(|color| rgba(color.to_u8_array()))
                .unwrap_or_else(|| rgba(*theme.color(DefaultSelection))),
            caret: rgba(*theme.color(TextEditCursorColor)),
            composition: rgba(*theme.color(TextEditCursorColor)),
        };
        let mut outer = Rect::new(
            self.dim.x as f32 + 1.,
            self.dim.y as f32 + 1.,
            (self.dim.width as f32 - 2.).max(0.),
            (self.dim.height as f32 - 2.).max(0.),
        );
        if let StatusbarType::Widget(dim) = &mut self.statusbar_type {
            let height = (self.renderer.font_size * 1.5).round().min(outer.height);
            *dim = TheDim::new(
                outer.x as i32,
                (outer.y + outer.height - height) as i32,
                outer.width as i32,
                height as i32,
            );
            outer.height -= height;
        }
        if let Some(dim) = &mut self.ln_area_dim {
            let width = ctx
                .text
                .layout(
                    &"9".repeat(self.state.row_count().to_string().len()),
                    "Source Code Pro",
                    self.renderer.font_size,
                    None,
                )
                .width()
                .ceil()
                + self.renderer.font_size;
            *dim = TheDim::new(
                outer.x as i32,
                outer.y as i32,
                width as i32,
                outer.height as i32,
            );
            outer.x += width;
            outer.width = (outer.width - width).max(0.);
        }
        let content = crate::theui::text_viewport::text_content_rect(outer, self.renderer.padding);
        let wrap = self.readonly && self.word_wrap;
        let width = wrap.then_some((content.width - self.scrollbar_size as f32).max(1.));
        let options = || TextOptions {
            family: "Source Code Pro",
            size: self.renderer.font_size,
            width,
            line_height: None,
        };
        self.native.view.editor.prepare(&mut ctx.text, options());
        let layout = self.native.view.editor.layout(&mut ctx.text);
        let size = [layout.full_width(), layout.height()];
        let plan = crate::theui::text_viewport::plan_text_viewport(
            outer,
            self.renderer.padding,
            size,
            self.scrollbar_size as f32,
            wrap,
        );
        self.native
            .view
            .prepare(&mut ctx.text, options(), plan.viewport, false);
        self.native.horizontal = plan.horizontal;
        self.native.vertical = plan.vertical;
        if self.pending_scroll_row.take().is_some() {
            if self.pending_scroll_centered {
                if let Some(caret) = self.native.view.editor.caret(&mut ctx.text) {
                    self.native.view.set_scroll(
                        &mut ctx.text,
                        [
                            self.native.view.scroll()[0],
                            (caret.y + caret.height * 0.5 - plan.viewport.height * 0.5).max(0.),
                        ],
                    );
                }
            } else {
                self.native.view.reveal_caret(&mut ctx.text);
            }
            self.pending_scroll_centered = false;
        }
        if let Some(rect) = plan.horizontal {
            self.hscrollbar.set_dim(
                TheDim::new(
                    rect.x as i32,
                    rect.y as i32,
                    rect.width as i32,
                    rect.height as i32,
                ),
                ctx,
            );
        }
        if let Some(rect) = plan.vertical {
            self.vscrollbar.set_dim(
                TheDim::new(
                    rect.x as i32,
                    rect.y as i32,
                    rect.width as i32,
                    rect.height as i32,
                ),
                ctx,
            );
        }
        if let Some(scroll) = self.hscrollbar.as_horizontal_scrollbar() {
            scroll.set_total_width(
                size[0].ceil() as i32 + self.renderer.padding.0 + self.renderer.padding.2,
            );
            scroll.set_scroll_offset(self.native.view.scroll()[0].round() as i32);
        }
        if let Some(scroll) = self.vscrollbar.as_vertical_scrollbar() {
            scroll.set_total_height(
                size[1].ceil() as i32 + self.renderer.padding.1 + self.renderer.padding.3,
            );
            scroll.set_scroll_offset(self.native.view.scroll()[1].round() as i32);
        }
        let snapshot = self.native.view.editor.snapshots_built();
        if self.auto_scroll_to_cursor && self.native.snapshot != snapshot {
            self.native.view.reveal_caret(&mut ctx.text);
        }
        if self.native.snapshot != snapshot
            || self.native.annotations_dirty
            || self.modified_since_last_tick
        {
            let display = self.native.view.editor.layout(&mut ctx.text).source.clone();
            self.native.lines = std::iter::once(0)
                .chain(display.match_indices('\n').map(|(index, _)| index + 1))
                .collect();
            self.native.annotations = Arc::new(self.renderer.editor_annotations(
                &display,
                self.highlight_source.as_deref(),
                self.native.view.editor.text(),
                |range| self.native.view.editor.displayed_range(range),
                theme,
            ));
            self.native.snapshot = snapshot;
            self.native.annotations_dirty = false;
        }
    }
    pub(super) fn paint_native_area(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let outer = bounds(&self.dim);
        p.list.ops.push(PaintOp::PushClip(outer));
        let inner = p.text_input(
            outer,
            self.id.equals(&p.ui.focus),
            self.is_disabled,
            self.embedded,
            true,
            true,
        );
        if let Some(color) = self
            .renderer
            .highlighter
            .as_ref()
            .and_then(|h| h.background())
        {
            p.list.rect(inner, 0., rgba(color.to_u8_array()));
        }
        self.native.view.paint_decorated(
            p.text,
            p.list,
            self.native.colors,
            !self.readonly,
            &self.native.annotations,
        );
        if let StatusbarType::Widget(dim) = &self.statusbar_type {
            let rect = bounds(dim);
            p.fill(rect, TextEditBackground);
            p.fill(Rect::new(rect.x, rect.y, rect.width, 1.), TextEditBorder);
            p.label(
                Rect::new(
                    rect.x,
                    rect.y,
                    (rect.width - self.renderer.font_size).max(0.),
                    rect.height,
                ),
                &self.statusbar_text(),
                self.renderer.font_size * 0.8,
                *p.theme.color(TextEditTextColor),
                TheHorizontalAlign::Right,
                TheVerticalAlign::Center,
            );
        }
        if let Some(dim) = &self.ln_area_dim {
            let gutter = bounds(dim);
            p.fill(gutter, TextEditBackground);
            p.fill(
                Rect::new(gutter.x + gutter.width - 1., gutter.y, 1., gutter.height),
                TextEditBorder,
            );
            p.list.ops.push(PaintOp::PushClip(gutter));
            for (_, line) in self.native.view.visible_lines(p.text) {
                let row = self
                    .native
                    .lines
                    .partition_point(|start| *start <= line.range.start)
                    .saturating_sub(1);
                if self.native.lines.get(row) != Some(&line.range.start) {
                    continue;
                }
                let role = if self.renderer.has_debug_line(row) {
                    TextEditLineNumberDebugColor
                } else if self.state.cursor.row == row {
                    TextEditLineNumberHighlightColor
                } else {
                    TextEditLineNumberColor
                };
                let layout = p.text.layout(
                    &(row + 1).to_string(),
                    "Source Code Pro",
                    self.renderer.font_size,
                    None,
                );
                let baseline = layout
                    .lines()
                    .first()
                    .map_or(self.renderer.font_size, |line| line.baseline);
                for run in &layout.runs {
                    p.list.ops.push(PaintOp::GlyphRun {
                        run: run.clone(),
                        origin: [
                            gutter.x + gutter.width
                                - self.renderer.font_size * 0.5
                                - layout.width(),
                            self.native.view.bounds().y + line.baseline
                                - self.native.view.scroll()[1]
                                - baseline,
                        ],
                        color: p.color(role),
                    });
                }
            }
            p.list.ops.push(PaintOp::PopClip);
        }
        if let Some(rect) = self.native.horizontal {
            let scroll = self.hscrollbar.as_horizontal_scrollbar().unwrap();
            let width = scroll.scrollbar_thumb_width().clamp(0, rect.width as i32) as f32;
            let offset = scroll
                .scrollbar_position()
                .round()
                .clamp(0., (rect.width - width).max(0.));
            p.scrollbar(
                rect,
                Rect::new(
                    rect.x + offset,
                    rect.y + 2.,
                    width,
                    (rect.height - 4.).max(0.),
                ),
                self.is_hscrollbar_hovered,
                self.is_hscrollbar_clicked,
            );
        }
        if let Some(rect) = self.native.vertical {
            let scroll = self.vscrollbar.as_vertical_scrollbar().unwrap();
            let height = scroll.scrollbar_thumb_height().clamp(0, rect.height as i32) as f32;
            let offset = scroll
                .scrollbar_position()
                .round()
                .clamp(0., (rect.height - height).max(0.));
            p.scrollbar(
                rect,
                Rect::new(
                    rect.x + 2.,
                    rect.y + offset,
                    (rect.width - 4.).max(0.),
                    height,
                ),
                self.is_vscrollbar_hovered,
                self.is_vscrollbar_clicked,
            );
        }
        p.list.ops.push(PaintOp::PopClip);
        self.modified_since_last_return |= self.modified_since_last_tick;
        self.modified_since_last_tick = false;
        self.is_dirty = false;
        Ok(())
    }
    pub(super) fn apply_native_area_response(
        &mut self,
        response: &EditorResponse,
        ctx: &mut TheContext,
    ) {
        if response.changed {
            self.native.view.editor.export_state(&mut self.state);
            self.modified_since_last_tick = true;
            self.modified_since_last_return = true;
            self.native.annotations_dirty = true;
        } else {
            self.native.view.editor.export_selection(&mut self.state);
        }
        if let Some(request) = &response.clipboard {
            ctx.ui.editor_clipboard.push_back(request.clone());
        }
        if response.changed && self.continuous {
            self.emit_value_changed(ctx);
        }
        if self.statusbar_type == StatusbarType::Global && response.input.redraw {
            ctx.ui.send(TheEvent::SetStatusText(
                self.id.clone(),
                self.statusbar_text(),
            ));
        }
        if response.action == Some(crate::text_edit::EditorAction::Cancel) {
            ctx.ui.clear_focus();
        }
        self.is_dirty |= response.input.redraw;
    }
    fn native_pointer(&self, coord: Vec2<i32>) -> [f32; 2] {
        [
            self.dim.x as f32 + coord.x as f32,
            self.dim.y as f32 + coord.y as f32,
        ]
    }
    pub(super) fn native_area_legacy_event(
        &mut self,
        event: &TheEvent,
        ctx: &mut TheContext,
    ) -> Option<bool> {
        if self.is_disabled && !matches!(event, TheEvent::LostFocus(_)) {
            return Some(false);
        }
        match event {
            TheEvent::GainedFocus(_) => {
                self.ensure_native_editor(ctx);
                self.is_dirty = true;
                return Some(true);
            }
            TheEvent::LostFocus(_) => {
                self.cancel_native_area_capture(ctx);
                self.native.view.set_focused(&mut ctx.text, false);
                self.is_dirty = true;
                return Some(true);
            }
            TheEvent::ModifierChanged(shift, ctrl, alt, logo) => {
                self.modifier_shift = *shift;
                self.modifier_ctrl = *ctrl;
                self.modifier_alt = *alt;
                self.modifier_logo = *logo;
                ctx.ui.input_modifiers = crate::platform::Modifiers {
                    shift: *shift,
                    control: *ctrl,
                    alt: *alt,
                    super_key: *logo,
                };
                self.native.view.input(
                    &mut ctx.text,
                    &InputEvent::Modifiers(ctx.ui.input_modifiers),
                );
                return Some(false);
            }
            TheEvent::MouseDown(coord) => {
                ctx.ui.set_focus(&self.id);
                self.ensure_native_editor(ctx);
                let position = self.native_pointer(*coord);
                if self
                    .native
                    .horizontal
                    .is_some_and(|rect| rect.contains(position))
                {
                    self.native.capture = Some(Capture::Horizontal);
                    self.is_hscrollbar_clicked = true;
                    self.hscrollbar.on_event(
                        &TheEvent::MouseDown(
                            self.hscrollbar
                                .dim()
                                .to_local(Vec2::new(position[0] as i32, position[1] as i32)),
                        ),
                        ctx,
                    );
                } else if self
                    .native
                    .vertical
                    .is_some_and(|rect| rect.contains(position))
                {
                    self.native.capture = Some(Capture::Vertical);
                    self.is_vscrollbar_clicked = true;
                    self.vscrollbar.on_event(
                        &TheEvent::MouseDown(
                            self.vscrollbar
                                .dim()
                                .to_local(Vec2::new(position[0] as i32, position[1] as i32)),
                        ),
                        ctx,
                    );
                } else if self.native.view.bounds().contains(position) {
                    self.native.clicks = if self.last_mouse_down_time.elapsed().as_millis() < 500
                        && (*coord - self.last_mouse_down_coord).magnitude_squared() <= 16
                    {
                        self.native.clicks % 3 + 1
                    } else {
                        1
                    };
                    self.native
                        .view
                        .pointer_down(&mut ctx.text, position, self.native.clicks);
                    self.native.capture = Some(Capture::Text);
                    self.native.view.editor.export_selection(&mut self.state);
                }
                match self.native.capture {
                    Some(Capture::Horizontal) => {
                        let offset = self
                            .hscrollbar
                            .as_horizontal_scrollbar()
                            .unwrap()
                            .scroll_offset();
                        self.native.view.set_scroll(
                            &mut ctx.text,
                            [offset as f32, self.native.view.scroll()[1]],
                        );
                    }
                    Some(Capture::Vertical) => {
                        let offset = self
                            .vscrollbar
                            .as_vertical_scrollbar()
                            .unwrap()
                            .scroll_offset();
                        self.native.view.set_scroll(
                            &mut ctx.text,
                            [self.native.view.scroll()[0], offset as f32],
                        );
                    }
                    _ => {}
                }
                self.last_mouse_down_coord = *coord;
                self.last_mouse_down_time = Instant::now();
                self.is_dirty = true;
                return Some(true);
            }
            TheEvent::MouseDragged(coord) => {
                let position = self.native_pointer(*coord);
                let previous = self.native.view.scroll();
                match self.native.capture {
                    Some(Capture::Horizontal) => {
                        self.hscrollbar.on_event(
                            &TheEvent::MouseDragged(
                                self.hscrollbar
                                    .dim()
                                    .to_local(Vec2::new(position[0] as i32, position[1] as i32)),
                            ),
                            ctx,
                        );
                        let offset = self
                            .hscrollbar
                            .as_horizontal_scrollbar()
                            .unwrap()
                            .scroll_offset();
                        self.native
                            .view
                            .set_scroll(&mut ctx.text, [offset as f32, previous[1]]);
                    }
                    Some(Capture::Vertical) => {
                        self.vscrollbar.on_event(
                            &TheEvent::MouseDragged(
                                self.vscrollbar
                                    .dim()
                                    .to_local(Vec2::new(position[0] as i32, position[1] as i32)),
                            ),
                            ctx,
                        );
                        let offset = self
                            .vscrollbar
                            .as_vertical_scrollbar()
                            .unwrap()
                            .scroll_offset();
                        self.native
                            .view
                            .set_scroll(&mut ctx.text, [previous[0], offset as f32]);
                    }
                    Some(Capture::Text) => {
                        let response = self
                            .native
                            .view
                            .input(&mut ctx.text, &InputEvent::PointerMoved { position });
                        self.apply_native_area_response(&response, ctx);
                    }
                    None => return Some(false),
                }
                self.is_dirty = true;
                return Some(true);
            }
            TheEvent::MouseUp(coord) => {
                let position = self.native_pointer(*coord);
                match self.native.capture.take() {
                    Some(Capture::Horizontal) => {
                        self.hscrollbar
                            .on_event(&TheEvent::MouseUp(Vec2::zero()), ctx);
                    }
                    Some(Capture::Vertical) => {
                        self.vscrollbar
                            .on_event(&TheEvent::MouseUp(Vec2::zero()), ctx);
                    }
                    _ => {
                        let response = self.native.view.input(
                            &mut ctx.text,
                            &InputEvent::PointerButton {
                                button: PointerButton::Primary,
                                pressed: false,
                                position,
                            },
                        );
                        self.apply_native_area_response(&response, ctx);
                    }
                }
                self.is_hscrollbar_clicked = false;
                self.is_vscrollbar_clicked = false;
                self.is_dirty = true;
                return Some(true);
            }
            TheEvent::Hover(coord) => {
                let position = self.native_pointer(*coord);
                self.hover_coord = *coord;
                self.is_hscrollbar_hovered = self
                    .native
                    .horizontal
                    .is_some_and(|rect| rect.contains(position));
                self.is_vscrollbar_hovered = self
                    .native
                    .vertical
                    .is_some_and(|rect| rect.contains(position));
                ctx.ui.set_hover(&self.id);
                self.native
                    .view
                    .input(&mut ctx.text, &InputEvent::PointerMoved { position });
                self.is_dirty = true;
                return Some(true);
            }
            TheEvent::MouseWheel(delta) => {
                let mut delta = [delta.x as f32, delta.y as f32];
                if self.is_hscrollbar_hovered {
                    delta = [
                        if delta[0].abs() > delta[1].abs() {
                            delta[0]
                        } else {
                            delta[1]
                        },
                        0.,
                    ];
                }
                let response = self.native.view.scroll_by(&mut ctx.text, delta, true);
                self.is_dirty |= response.redraw;
                return Some(response.redraw);
            }
            TheEvent::PreciseScroll(delta) => {
                let response = self
                    .native
                    .view
                    .scroll_by(&mut ctx.text, [delta.x, delta.y], true);
                self.is_dirty |= response.redraw;
                return Some(response.redraw);
            }
            TheEvent::Undo | TheEvent::Redo | TheEvent::Copy | TheEvent::Cut => {
                let command = match event {
                    TheEvent::Undo => EditorCommand::Undo,
                    TheEvent::Redo => EditorCommand::Redo,
                    TheEvent::Copy => EditorCommand::Copy,
                    _ => EditorCommand::Cut,
                };
                return self
                    .editor_command(command, ctx)
                    .map(|response| response.input.redraw);
            }
            TheEvent::Paste(value, _) => {
                self.ensure_native_editor(ctx);
                let text = value.to_string()?;
                let changed = self
                    .native
                    .view
                    .editor
                    .replace_selection(&mut ctx.text, &text);
                let response = EditorResponse {
                    changed,
                    input: InputResponse {
                        consumed: true,
                        redraw: changed,
                    },
                    ..Default::default()
                };
                self.apply_native_area_response(&response, ctx);
                return Some(changed);
            }
            _ => {}
        }
        let input = match event {
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
            _ => return None,
        };
        self.editor_input(&input, ctx)
            .map(|response| response.input.redraw)
    }
}
