//! Common native read-only text viewport. Document projection and link actions
//! belong to the containing widget; selection, geometry and clipboard use the
//! same controller as editable text, without exposing a platform IME target.
use super::thetextedit::TheTextStyle;
use crate::{
    paint::Rect,
    platform::{InputEvent, PointerButton},
    prelude::*,
    text::{TextOptions, TextStyleSpan},
    text_edit::{
        EditorColors, EditorCommand, EditorDecorations, EditorPolicy, EditorResponse, EditorView,
    },
    theui::paint::{WidgetPainter, rgba},
};
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;
use web_time::Instant;

pub(super) struct DisplayOptions<'a> {
    pub outer: Rect,
    pub padding: (i32, i32, i32, i32),
    pub scrollbar: f32,
    pub wrap: bool,
    pub selectable: bool,
    pub family: &'a str,
    pub size: f32,
}
#[derive(Clone, Copy)]
enum Capture {
    Text,
    Horizontal,
    Vertical,
    Link,
}
pub(super) struct NativeTextDisplay {
    pub view: EditorView,
    pub replaced: bool,
    active: bool,
    selectable: bool,
    horizontal: Option<Rect>,
    vertical: Option<Rect>,
    hscrollbar: Box<dyn TheWidget>,
    vscrollbar: Box<dyn TheWidget>,
    hovered: [bool; 2],
    capture: Option<Capture>,
    click: Option<(Instant, [f32; 2], u8)>,
    styles: Vec<(Range<usize>, TheTextStyle)>,
    decorations: Arc<EditorDecorations>,
    scroll_request: Option<bool>,
}
impl NativeTextDisplay {
    pub fn new(id: &TheId) -> Self {
        let mut horizontal = TheHorizontalScrollbar::new(TheId::named("Text horizontal"));
        horizontal.set_parent_id(id.clone());
        let mut vertical = TheVerticalScrollbar::new(TheId::named("Text vertical"));
        vertical.set_parent_id(id.clone());
        let mut view = EditorView::new(true);
        view.set_auto_reveal(false);
        Self {
            view,
            replaced: true,
            active: true,
            selectable: true,
            horizontal: None,
            vertical: None,
            hscrollbar: Box::new(horizontal),
            vscrollbar: Box::new(vertical),
            hovered: [false; 2],
            capture: None,
            click: None,
            styles: Vec::new(),
            decorations: Arc::new(EditorDecorations::default()),
            scroll_request: None,
        }
    }
    fn synchronize(
        &mut self,
        state: &crate::text_edit::TextEditState,
        ctx: &mut TheContext,
    ) -> bool {
        if !self.replaced {
            return false;
        }
        self.cancel_capture(ctx);
        self.view.editor.import_state(&mut ctx.text, state);
        self.view.editor.set_policy(
            &mut ctx.text,
            EditorPolicy {
                read_only: true,
                ..Default::default()
            },
        );
        self.replaced = false;
        true
    }
    pub fn set_active(&mut self, active: bool, ctx: &mut TheContext) -> bool {
        let was_focused = self.view.editor.is_focused();
        self.active = active;
        if !active {
            self.cancel_capture(ctx);
        }
        self.view
            .set_focused(&mut ctx.text, active && self.selectable);
        was_focused != self.view.editor.is_focused()
    }
    pub fn request_scroll(&mut self, bottom: bool) {
        self.scroll_request = Some(bottom);
    }
    pub fn prepare(
        &mut self,
        id: &TheId,
        state: &crate::text_edit::TextEditState,
        options: DisplayOptions<'_>,
        styles: &[(Range<usize>, TheTextStyle)],
        ctx: &mut TheContext,
    ) {
        let replaced = self.synchronize(state, ctx);
        self.selectable = options.selectable;
        self.view.set_focused(
            &mut ctx.text,
            self.active && self.selectable && ctx.ui.has_focus(id),
        );
        if !self.selectable {
            let end = self.view.editor.selection_bytes().1;
            self.view.editor.select_bytes(&mut ctx.text, end, end);
        }
        let content =
            crate::theui::text_viewport::text_content_rect(options.outer, options.padding);
        let width = options
            .wrap
            .then_some((content.width - options.scrollbar).max(1.));
        let text_options = || TextOptions {
            family: options.family,
            size: options.size,
            width,
            line_height: None,
        };
        self.view.editor.prepare(&mut ctx.text, text_options());
        let layout = self.view.editor.layout(&mut ctx.text);
        let size = [layout.full_width(), layout.height()];
        let plan = crate::theui::text_viewport::plan_text_viewport(
            options.outer,
            options.padding,
            size,
            options.scrollbar,
            options.wrap,
        );
        self.view
            .prepare(&mut ctx.text, text_options(), plan.viewport, false);
        self.horizontal = plan.horizontal;
        self.vertical = plan.vertical;
        if let Some(bottom) = self.scroll_request.take() {
            self.view
                .set_scroll(&mut ctx.text, if bottom { [0., size[1]] } else { [0.; 2] });
        }
        for (widget, rect) in [
            (&mut self.hscrollbar, plan.horizontal),
            (&mut self.vscrollbar, plan.vertical),
        ] {
            if let Some(rect) = rect {
                widget.set_dim(
                    TheDim::new(
                        rect.x as i32,
                        rect.y as i32,
                        rect.width as i32,
                        rect.height as i32,
                    ),
                    ctx,
                );
            }
        }
        self.hscrollbar
            .as_horizontal_scrollbar()
            .unwrap()
            .set_total_width(size[0].ceil() as i32 + options.padding.0 + options.padding.2);
        self.vscrollbar
            .as_vertical_scrollbar()
            .unwrap()
            .set_total_height(size[1].ceil() as i32 + options.padding.1 + options.padding.3);
        self.synchronize_scrollbars();
        if replaced || self.styles != styles {
            self.styles = styles.to_vec();
            let source = self.view.editor.text();
            let mut bytes = source
                .grapheme_indices(true)
                .map(|(byte, _)| byte)
                .collect::<Vec<_>>();
            bytes.push(source.len());
            let mut marks = EditorDecorations::default();
            for (range, style) in styles {
                let range = bytes.get(range.start).copied().unwrap_or(source.len())
                    ..bytes.get(range.end).copied().unwrap_or(source.len());
                if range.is_empty() {
                    continue;
                }
                for (color, output) in [
                    (&style.foreground, &mut marks.foreground),
                    (&style.background, &mut marks.backgrounds),
                    (&style.underline, &mut marks.underlines),
                ] {
                    if let Some(color) = color {
                        output.push(TextStyleSpan {
                            range: range.clone(),
                            color: rgba(color.to_u8_array()),
                        });
                    }
                }
            }
            self.decorations = Arc::new(marks);
        }
    }
    fn synchronize_scrollbars(&mut self) {
        let scroll = self.view.scroll();
        self.hscrollbar
            .as_horizontal_scrollbar()
            .unwrap()
            .set_scroll_offset(scroll[0].round() as i32);
        self.vscrollbar
            .as_vertical_scrollbar()
            .unwrap()
            .set_scroll_offset(scroll[1].round() as i32);
    }
    fn apply(
        &mut self,
        response: &EditorResponse,
        state: &mut crate::text_edit::TextEditState,
        ctx: &mut TheContext,
    ) {
        debug_assert!(!response.changed, "read-only display mutated its document");
        self.view.editor.export_selection(state);
        if let Some(request) = &response.clipboard {
            ctx.ui.editor_clipboard.push_back(request.clone());
        }
    }
    pub fn input(
        &mut self,
        event: &InputEvent,
        state: &mut crate::text_edit::TextEditState,
        ctx: &mut TheContext,
    ) -> EditorResponse {
        self.synchronize(state, ctx);
        self.view.input(
            &mut ctx.text,
            &InputEvent::Modifiers(ctx.ui.input_modifiers),
        );
        let response = self.view.input(&mut ctx.text, event);
        if response.input.consumed
            && matches!(
                event,
                InputEvent::Key {
                    physical: crate::platform::PhysicalKey::Left
                        | crate::platform::PhysicalKey::Right
                        | crate::platform::PhysicalKey::Up
                        | crate::platform::PhysicalKey::Down
                        | crate::platform::PhysicalKey::Home
                        | crate::platform::PhysicalKey::End,
                    pressed: true,
                    ..
                }
            )
        {
            self.view.reveal_caret(&mut ctx.text);
        }
        self.apply(&response, state, ctx);
        response
    }
    pub fn command(
        &mut self,
        command: EditorCommand,
        state: &mut crate::text_edit::TextEditState,
        ctx: &mut TheContext,
    ) -> EditorResponse {
        self.synchronize(state, ctx);
        let response = self.view.editor.execute(&mut ctx.text, command);
        self.apply(&response, state, ctx);
        response
    }
    pub fn grapheme_at(&mut self, point: [f32; 2], ctx: &mut TheContext) -> Option<usize> {
        if !self.view.bounds().contains(point) {
            return None;
        }
        let hit = self.view.hit_test(&mut ctx.text, point)?;
        Some(
            self.view.editor.text()[..hit.index()]
                .graphemes(true)
                .count(),
        )
    }
    pub fn link_pressed(
        &mut self,
        state: &mut crate::text_edit::TextEditState,
        ctx: &mut TheContext,
    ) {
        self.cancel_capture(ctx);
        self.capture = Some(Capture::Link);
        let focus = self.view.editor.selection_bytes().1;
        self.view.editor.select_bytes(&mut ctx.text, focus, focus);
        self.view.editor.export_selection(state);
    }
    fn cancel_capture(&mut self, ctx: &mut TheContext) {
        match self.capture.take() {
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
        self.view.input(
            &mut ctx.text,
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: [0.; 2],
            },
        );
    }
    pub fn event(
        &mut self,
        id: &TheId,
        dim: &TheDim,
        state: &mut crate::text_edit::TextEditState,
        event: &TheEvent,
        ctx: &mut TheContext,
    ) -> Option<bool> {
        self.synchronize(state, ctx);
        let point =
            |coord: &Vec2<i32>| [dim.x as f32 + coord.x as f32, dim.y as f32 + coord.y as f32];
        match event {
            TheEvent::GainedFocus(_) => {
                self.set_active(true, ctx);
                Some(true)
            }
            TheEvent::LostFocus(_) => {
                self.set_active(false, ctx);
                Some(true)
            }
            TheEvent::Copy => Some(self.command(EditorCommand::Copy, state, ctx).input.redraw),
            TheEvent::Cut | TheEvent::Paste(..) | TheEvent::Undo | TheEvent::Redo => Some(false),
            TheEvent::ModifierChanged(shift, control, alt, super_key) => {
                ctx.ui.input_modifiers = crate::platform::Modifiers {
                    shift: *shift,
                    control: *control,
                    alt: *alt,
                    super_key: *super_key,
                };
                self.view.input(
                    &mut ctx.text,
                    &InputEvent::Modifiers(ctx.ui.input_modifiers),
                );
                Some(false)
            }
            TheEvent::MouseDown(coord) => {
                let position = point(coord);
                if self.selectable {
                    ctx.ui.set_focus(id);
                    self.set_active(true, ctx);
                }
                self.capture = if self.horizontal.is_some_and(|r| r.contains(position)) {
                    self.hscrollbar.on_event(
                        &TheEvent::MouseDown(
                            self.hscrollbar
                                .dim()
                                .to_local(Vec2::new(position[0] as i32, position[1] as i32)),
                        ),
                        ctx,
                    );
                    Some(Capture::Horizontal)
                } else if self.vertical.is_some_and(|r| r.contains(position)) {
                    self.vscrollbar.on_event(
                        &TheEvent::MouseDown(
                            self.vscrollbar
                                .dim()
                                .to_local(Vec2::new(position[0] as i32, position[1] as i32)),
                        ),
                        ctx,
                    );
                    Some(Capture::Vertical)
                } else if self.selectable && self.view.bounds().contains(position) {
                    let clicks = self
                        .click
                        .filter(|(time, p, _)| {
                            time.elapsed().as_millis() < 500
                                && (p[0] - position[0]).powi(2) + (p[1] - position[1]).powi(2)
                                    <= 16.
                        })
                        .map_or(1, |(_, _, count)| count % 3 + 1);
                    self.view.pointer_down(&mut ctx.text, position, clicks);
                    self.click = Some((Instant::now(), position, clicks));
                    self.view.editor.export_selection(state);
                    Some(Capture::Text)
                } else {
                    None
                };
                // Track clicks page immediately; they need no drag event.
                match self.capture {
                    Some(Capture::Horizontal) => {
                        let offset = self
                            .hscrollbar
                            .as_horizontal_scrollbar()
                            .unwrap()
                            .scroll_offset();
                        self.view
                            .set_scroll(&mut ctx.text, [offset as f32, self.view.scroll()[1]]);
                    }
                    Some(Capture::Vertical) => {
                        let offset = self
                            .vscrollbar
                            .as_vertical_scrollbar()
                            .unwrap()
                            .scroll_offset();
                        self.view
                            .set_scroll(&mut ctx.text, [self.view.scroll()[0], offset as f32]);
                    }
                    _ => {}
                }
                Some(true)
            }
            TheEvent::MouseDragged(coord) => {
                let position = point(coord);
                match self.capture {
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
                        self.view
                            .set_scroll(&mut ctx.text, [offset as f32, self.view.scroll()[1]]);
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
                        self.view
                            .set_scroll(&mut ctx.text, [self.view.scroll()[0], offset as f32]);
                    }
                    Some(Capture::Text) => {
                        self.input(&InputEvent::PointerMoved { position }, state, ctx);
                    }
                    _ => return Some(false),
                }
                Some(true)
            }
            TheEvent::MouseUp(coord) => {
                let captured = self.capture.is_some();
                let response = self.input(
                    &InputEvent::PointerButton {
                        button: PointerButton::Primary,
                        pressed: false,
                        position: point(coord),
                    },
                    state,
                    ctx,
                );
                self.cancel_capture(ctx);
                Some(captured || response.input.redraw)
            }
            TheEvent::Hover(coord) => {
                let position = point(coord);
                self.hovered = [
                    self.horizontal.is_some_and(|r| r.contains(position)),
                    self.vertical.is_some_and(|r| r.contains(position)),
                ];
                self.view
                    .input(&mut ctx.text, &InputEvent::PointerMoved { position });
                ctx.ui.set_hover(id);
                Some(true)
            }
            TheEvent::LostHover(_) => {
                self.hovered = [false; 2];
                Some(true)
            }
            TheEvent::PreciseScroll(delta) => {
                let delta = if self.hovered[0] {
                    [
                        if delta.x.abs() > delta.y.abs() {
                            delta.x
                        } else {
                            delta.y
                        },
                        0.,
                    ]
                } else {
                    [delta.x, delta.y]
                };
                Some(self.view.scroll_by(&mut ctx.text, delta, true).redraw)
            }
            TheEvent::MouseWheel(delta) => {
                let delta = if self.hovered[0] {
                    [
                        if delta.x.abs() > delta.y.abs() {
                            delta.x as f32
                        } else {
                            delta.y as f32
                        },
                        0.,
                    ]
                } else {
                    [delta.x as f32, delta.y as f32]
                };
                Some(self.view.scroll_by(&mut ctx.text, delta, false).redraw)
            }
            _ => None,
        }
    }
    pub fn paint(&mut self, p: &mut WidgetPainter<'_>) {
        self.view.paint_decorated(
            p.text,
            p.list,
            EditorColors {
                text: rgba(*p.theme.color(TextEditTextColor)),
                selection: rgba(*p.theme.color(DefaultSelection)),
                ..Default::default()
            },
            false,
            &self.decorations,
        );
        self.synchronize_scrollbars();
        if let Some(r) = self.horizontal {
            let scroll = self.hscrollbar.as_horizontal_scrollbar().unwrap();
            let width = scroll.scrollbar_thumb_width().clamp(0, r.width as i32) as f32;
            let offset = scroll
                .scrollbar_position()
                .round()
                .clamp(0., (r.width - width).max(0.));
            p.scrollbar(
                r,
                Rect::new(r.x + offset, r.y + 2., width, (r.height - 4.).max(0.)),
                self.hovered[0],
                matches!(self.capture, Some(Capture::Horizontal)),
            );
        }
        if let Some(r) = self.vertical {
            let scroll = self.vscrollbar.as_vertical_scrollbar().unwrap();
            let height = scroll.scrollbar_thumb_height().clamp(0, r.height as i32) as f32;
            let offset = scroll
                .scrollbar_position()
                .round()
                .clamp(0., (r.height - height).max(0.));
            p.scrollbar(
                r,
                Rect::new(r.x + 2., r.y + offset, (r.width - 4.).max(0.), height),
                self.hovered[1],
                matches!(self.capture, Some(Capture::Vertical)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (
        TheContext,
        TheId,
        crate::text_edit::TextEditState,
        NativeTextDisplay,
    ) {
        let mut ctx = TheContext::new(300, 160, 1.);
        ctx.text
            .register_font(include_bytes!("../../../embedded/fonts/Roboto-Bold.ttf").to_vec());
        let id = TheId::named("Read-only log");
        ctx.ui.set_focus(&id);
        let mut state = crate::text_edit::TextEditState::default();
        state.set_text("e\u{301}東京 log\nSecond\nThird\nFourth\nFifth\nSixth".into());
        let display = NativeTextDisplay::new(&id);
        (ctx, id, state, display)
    }
    fn options() -> DisplayOptions<'static> {
        DisplayOptions {
            outer: Rect::new(10., 20., 200., 50.),
            padding: (4, 3, 4, 3),
            scrollbar: 13.,
            wrap: true,
            selectable: true,
            family: "Roboto",
            size: 18.,
        }
    }
    #[test]
    fn repaint_and_hover_styles_preserve_log_scrollback_and_one_shared_layout() {
        let (mut ctx, id, state, mut display) = fixture();
        display.prepare(&id, &state, options(), &[], &mut ctx);
        display.view.set_scroll(&mut ctx.text, [0., 35.]);
        display.view.editor.select_bytes(&mut ctx.text, 3, 9);
        let builds = display.view.editor.snapshots_built();
        let styles = [(
            1..3,
            TheTextStyle {
                foreground: Some(TheColor::from([20u8, 30, 40, 255])),
                background: Some(TheColor::from([50u8, 60, 70, 255])),
                underline: None,
            },
        )];
        display.prepare(&id, &state, options(), &styles, &mut ctx);
        assert_eq!(display.view.editor.snapshots_built(), builds);
        assert_eq!(display.view.scroll(), [0., 35.]);
        assert_eq!(display.view.editor.selection_bytes(), (3, 9));
        assert_eq!(display.decorations.foreground[0].range, 3..9);
        assert_eq!(display.decorations.backgrounds[0].range, 3..9);
        display.prepare(&id, &state, options(), &styles, &mut ctx);
        assert_eq!(display.view.editor.snapshots_built(), builds);
        assert!(display.view.text_input_state(&mut ctx.text).is_none());
        assert!(
            !display.set_active(true, &mut ctx),
            "stable scope must not create an idle repaint loop"
        );
    }
    #[test]
    fn scrollbar_track_click_pages_without_drag_and_keeps_readonly_focus() {
        let (mut ctx, id, mut state, mut display) = fixture();
        display.prepare(&id, &state, options(), &[], &mut ctx);
        let rect = display.vertical.unwrap();
        let dim = TheDim::new(10, 20, 200, 50);
        let point = Vec2::new(
            (rect.x + 5.) as i32 - dim.x,
            (rect.y + rect.height - 2.) as i32 - dim.y,
        );
        assert_eq!(display.view.scroll(), [0.; 2]);
        assert_eq!(
            display.event(&id, &dim, &mut state, &TheEvent::MouseDown(point), &mut ctx),
            Some(true)
        );
        assert!(display.view.scroll()[1] > 0.);
        let after = display.view.scroll();
        assert_eq!(
            display.event(&id, &dim, &mut state, &TheEvent::MouseUp(point), &mut ctx),
            Some(true)
        );
        assert_eq!(ctx.ui.focus.as_ref(), Some(&id));
        display.prepare(&id, &state, options(), &[], &mut ctx);
        assert_eq!(display.view.scroll(), after);
        assert_eq!(display.view.editor.text(), state.to_text());
    }
    #[test]
    fn replacing_a_display_during_selection_cancels_old_capture_and_copies_the_new_source() {
        let (mut ctx, id, mut state, mut display) = fixture();
        display.prepare(&id, &state, options(), &[], &mut ctx);
        let dim = TheDim::new(10, 20, 200, 50);
        display.event(
            &id,
            &dim,
            &mut state,
            &TheEvent::MouseDown(Vec2::new(5, 12)),
            &mut ctx,
        );
        assert!(matches!(display.capture, Some(Capture::Text)));
        state.set_text("replacement 東京".into());
        display.replaced = true;
        display.prepare(&id, &state, options(), &[], &mut ctx);
        assert!(display.capture.is_none());
        display.event(
            &id,
            &dim,
            &mut state,
            &TheEvent::MouseDragged(Vec2::new(100, 12)),
            &mut ctx,
        );
        display.event(
            &id,
            &dim,
            &mut state,
            &TheEvent::MouseUp(Vec2::new(100, 12)),
            &mut ctx,
        );
        display.command(EditorCommand::SelectAll, &mut state, &mut ctx);
        display.command(EditorCommand::Copy, &mut state, &mut ctx);
        assert!(
            matches!(ctx.ui.editor_clipboard.pop_front(), Some(crate::text_edit::EditorClipboard::Write { text, cut: None }) if text=="replacement 東京")
        );
        assert_eq!(display.view.editor.text(), "replacement 東京");
    }
}
