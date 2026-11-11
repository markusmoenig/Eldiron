//! Logical viewport and paint adapter for the neutral editor. The containing
//! widget supplies chrome, focus order, clipboard services and blink scheduling.
use super::{EditorController, EditorResponse};
use crate::{
    paint::{Color, PaintList, PaintOp, Rect, Transform},
    platform::{InputEvent, Modifiers, PhysicalKey, PointerButton},
    text::{TextLine, TextOptions, TextStyleSpan, TextSystem},
};

use std::sync::Arc;
/// Immutable paint-only document annotations. Byte ranges address the displayed
/// text snapshot; they do not replace shaping, editing selection or IME state.
#[derive(Clone, Default)]
pub struct EditorDecorations {
    pub foreground: Vec<TextStyleSpan>,
    pub backgrounds: Vec<TextStyleSpan>,
    pub underlines: Vec<TextStyleSpan>,
    pub line_backgrounds: Vec<TextStyleSpan>,
    pub whitespace: Option<Color>,
}
struct DecorationCache {
    snapshot: u64,
    source: Arc<EditorDecorations>,
    runs: Vec<(Arc<crate::paint::GlyphRun>, Option<Color>)>,
    whitespace: Vec<Rect>,
}
#[derive(Clone, Copy)]
pub struct EditorColors {
    pub text: Color,
    pub selection: Color,
    pub caret: Color,
    pub composition: Color,
}
impl Default for EditorColors {
    fn default() -> Self {
        Self {
            text: [1.; 4],
            selection: [0.2, 0.4, 0.8, 0.6],
            caret: [1.; 4],
            composition: [1.; 4],
        }
    }
}
/// Bounds and all public positions are logical host coordinates. A host applies
/// density exactly once when presenting and locating its platform IME candidate.
pub struct EditorView {
    pub editor: EditorController,
    bounds: Rect,
    scroll: [f32; 2],
    pointer: Option<[f32; 2]>,
    dragging: bool,
    modifiers: Modifiers,
    center_y: bool,
    auto_reveal: bool,
    decorations: Option<DecorationCache>,
}
impl EditorView {
    pub fn new(multiline: bool) -> Self {
        Self {
            editor: EditorController::new(multiline),
            bounds: Rect::default(),
            scroll: [0.; 2],
            pointer: None,
            dragging: false,
            modifiers: Modifiers::default(),
            center_y: false,
            auto_reveal: true,
            decorations: None,
        }
    }
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn scroll(&self) -> [f32; 2] {
        self.scroll
    }
    pub fn prepare(
        &mut self,
        fonts: &mut TextSystem,
        options: TextOptions<'_>,
        bounds: Rect,
        center_y: bool,
    ) {
        let old_bounds = self.bounds;
        let old_center = self.center_y;
        let builds = self.editor.snapshots_built();
        self.bounds = Rect::new(
            if bounds.x.is_finite() { bounds.x } else { 0. },
            if bounds.y.is_finite() { bounds.y } else { 0. },
            if bounds.width.is_finite() {
                bounds.width.max(0.)
            } else {
                0.
            },
            if bounds.height.is_finite() {
                bounds.height.max(0.)
            } else {
                0.
            },
        );
        self.center_y = center_y;
        self.editor.prepare(fonts, options);
        self.clamp_scroll(fonts);
        if self.auto_reveal
            && self.editor.is_focused()
            && (old_bounds != self.bounds
                || old_center != self.center_y
                || builds != self.editor.snapshots_built())
        {
            self.reveal_caret(fonts);
        }
    }
    /// Disable automatic following for logs/readonly views. Explicit caret
    /// reveal, pointer drags and Page navigation remain available to the owner.
    pub fn set_auto_reveal(&mut self, enabled: bool) {
        self.auto_reveal = enabled;
    }
    pub fn scroll_by(
        &mut self,
        fonts: &mut TextSystem,
        delta: [f32; 2],
        precise: bool,
    ) -> crate::platform::InputResponse {
        let old = self.scroll;
        let unit = if precise { 1. } else { 24. };
        for (scroll, delta) in self.scroll.iter_mut().zip(delta) {
            if delta.is_finite() {
                *scroll -= delta * unit;
            }
        }
        self.clamp_scroll(fonts);
        crate::platform::InputResponse {
            consumed: true,
            redraw: old != self.scroll,
        }
    }
    pub fn set_scroll(&mut self, fonts: &mut TextSystem, position: [f32; 2]) {
        self.scroll = position.map(|v| if v.is_finite() { v.max(0.) } else { 0. });
        self.clamp_scroll(fonts);
    }
    pub fn visible_lines(&mut self, fonts: &mut TextSystem) -> Vec<(usize, TextLine)> {
        let origin = self.origin(fonts);
        let top = self.bounds.y - origin[1];
        self.editor
            .layout(fonts)
            .visible_lines(top, top + self.bounds.height)
    }
    fn origin(&mut self, fonts: &mut TextSystem) -> [f32; 2] {
        let height = self.editor.layout(fonts).height();
        let center = if self.center_y {
            ((self.bounds.height - height) / 2.).max(0.)
        } else {
            0.
        };
        [
            self.bounds.x - self.scroll[0],
            self.bounds.y + center - self.scroll[1],
        ]
    }
    fn local_point(&mut self, fonts: &mut TextSystem, point: [f32; 2]) -> [f32; 2] {
        let origin = self.origin(fonts);
        [point[0] - origin[0], point[1] - origin[1]]
    }
    fn clamp_scroll(&mut self, fonts: &mut TextSystem) {
        let layout = self.editor.layout(fonts);
        self.scroll[0] =
            self.scroll[0].clamp(0., (layout.full_width() + 1. - self.bounds.width).max(0.));
        self.scroll[1] = self.scroll[1].clamp(0., (layout.height() - self.bounds.height).max(0.));
    }
    pub fn reveal_caret(&mut self, fonts: &mut TextSystem) {
        // During an IME-selected preedit range Parley can hide the caret; its
        // candidate geometry remains available for scrolling and native hosting.
        let caret = self
            .editor
            .caret(fonts)
            .unwrap_or_else(|| self.editor.ime_area(fonts));
        if caret.x < self.scroll[0] {
            self.scroll[0] = caret.x;
        }
        if caret.x + caret.width > self.scroll[0] + self.bounds.width {
            self.scroll[0] = caret.x + caret.width - self.bounds.width;
        }
        if caret.y < self.scroll[1] {
            self.scroll[1] = caret.y;
        }
        if caret.y + caret.height > self.scroll[1] + self.bounds.height {
            self.scroll[1] = caret.y + caret.height - self.bounds.height;
        }
        self.clamp_scroll(fonts);
    }
    pub fn set_focused(&mut self, fonts: &mut TextSystem, focused: bool) {
        if self.editor.is_focused() == focused {
            return;
        }
        self.editor.set_focused(fonts, focused);
        if focused && self.auto_reveal {
            self.reveal_caret(fonts);
        } else {
            self.dragging = false;
            self.modifiers = Modifiers::default();
        }
    }
    /// The containing host may supply its native multiple-click count.
    pub fn pointer_down(&mut self, fonts: &mut TextSystem, position: [f32; 2], clicks: u8) -> bool {
        if !self.bounds.contains(position) {
            return false;
        }
        self.pointer = Some(position);
        self.editor.set_focused(fonts, true);
        let local = self.local_point(fonts, position);
        if self.modifiers.shift && clicks <= 1 {
            self.editor.point(fonts, local, true);
        } else {
            self.editor.point_selection(fonts, local, clicks);
        }
        self.dragging = true;
        self.reveal_caret(fonts);
        true
    }
    pub fn input(&mut self, fonts: &mut TextSystem, event: &InputEvent) -> EditorResponse {
        match event {
            InputEvent::Modifiers(modifiers) => {
                self.modifiers = *modifiers;
            }
            InputEvent::Focus(false) => {
                self.dragging = false;
                self.modifiers = Modifiers::default();
            }
            InputEvent::PointerMoved { position } => {
                self.pointer = Some(*position);
                if self.dragging {
                    let local = self.local_point(fonts, *position);
                    self.editor.point(fonts, local, true);
                    self.reveal_caret(fonts);
                    return EditorResponse::handled(false);
                }
                return EditorResponse::default();
            }
            InputEvent::PointerLeft => {
                self.pointer = None;
                return EditorResponse::default();
            }
            InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed,
                position,
            } => {
                if *pressed {
                    if self.pointer_down(fonts, *position, 1) {
                        return EditorResponse::handled(false);
                    }
                } else if self.dragging {
                    self.dragging = false;
                    return EditorResponse::handled(false);
                }
                return EditorResponse::default();
            }
            InputEvent::Scroll { delta, precise } => {
                if self.pointer.is_some_and(|p| self.bounds.contains(p)) {
                    return EditorResponse {
                        input: self.scroll_by(fonts, *delta, *precise),
                        ..Default::default()
                    };
                }
                return EditorResponse::default();
            }
            InputEvent::Key {
                physical: PhysicalKey::PageUp | PhysicalKey::PageDown,
                pressed: true,
                ..
            } if self.editor.is_focused() => {
                if !self.editor.is_composing() {
                    self.editor.move_page(
                        fonts,
                        self.bounds.height,
                        matches!(
                            event,
                            InputEvent::Key {
                                physical: PhysicalKey::PageDown,
                                ..
                            }
                        ),
                        self.modifiers.shift,
                    );
                    self.reveal_caret(fonts);
                }
                return EditorResponse::handled(false);
            }
            _ => {}
        }
        let response = self.editor.input(fonts, event);
        if response.input.redraw && self.auto_reveal {
            self.reveal_caret(fonts);
        }
        response
    }
    pub fn text_input_state(
        &mut self,
        fonts: &mut TextSystem,
    ) -> Option<crate::platform::TextInputState> {
        self.ime_area(fonts)
            .map(|area| crate::platform::TextInputState {
                target: self.editor.input_target(),
                area,
            })
    }
    pub fn ime_area(&mut self, fonts: &mut TextSystem) -> Option<Rect> {
        if !self.editor.accepts_text_input() || self.bounds.width <= 0. || self.bounds.height <= 0.
        {
            return None;
        }
        let local = self.editor.ime_area(fonts);
        let origin = self.origin(fonts);
        let raw = Rect::new(
            local.x + origin[0],
            local.y + origin[1],
            local.width,
            local.height,
        );
        if ![raw.x, raw.y, raw.width, raw.height]
            .iter()
            .all(|v| v.is_finite())
        {
            return None;
        }
        // Font fallback/preedit bounds can exceed the line box, and a user may
        // scroll the caret out of view. Anchor native candidates to the visible
        // editor viewport rather than a clipped or off-screen ink rectangle.
        let right = self.bounds.x + self.bounds.width;
        let bottom = self.bounds.y + self.bounds.height;
        let min_width = self.bounds.width.min(1.);
        let min_height = self.bounds.height.min(1.);
        let x = raw.x.clamp(self.bounds.x, right - min_width);
        let y = raw.y.clamp(self.bounds.y, bottom - min_height);
        Some(Rect::new(
            x,
            y,
            (raw.x + raw.width.max(min_width)).clamp(x + min_width, right) - x,
            (raw.y + raw.height.max(min_height)).clamp(y + min_height, bottom) - y,
        ))
    }
    pub fn paint(
        &mut self,
        fonts: &mut TextSystem,
        list: &mut PaintList,
        colors: EditorColors,
        caret_visible: bool,
    ) {
        self.paint_inner(fonts, list, colors, caret_visible, None);
    }
    pub fn paint_decorated(
        &mut self,
        fonts: &mut TextSystem,
        list: &mut PaintList,
        colors: EditorColors,
        caret_visible: bool,
        decorations: &Arc<EditorDecorations>,
    ) {
        self.paint_inner(fonts, list, colors, caret_visible, Some(decorations));
    }
    fn paint_inner(
        &mut self,
        fonts: &mut TextSystem,
        list: &mut PaintList,
        colors: EditorColors,
        caret_visible: bool,
        decorations: Option<&Arc<EditorDecorations>>,
    ) {
        if self.bounds.width <= 0. || self.bounds.height <= 0. {
            return;
        }
        let origin = self.origin(fonts);
        let top = self.bounds.y - origin[1];
        let bottom = top + self.bounds.height;
        if let Some(source) = decorations {
            let snapshot = self.editor.snapshots_built();
            if self
                .decorations
                .as_ref()
                .is_none_or(|old| old.snapshot != snapshot || !Arc::ptr_eq(&old.source, source))
            {
                let layout = self.editor.layout(fonts);
                let runs = layout.colorized_runs(&source.foreground);
                let whitespace = if source.whitespace.is_some() {
                    layout
                        .source
                        .char_indices()
                        .filter(|(_, c)| *c == ' ')
                        .map(|(index, _)| {
                            let caret = layout.caret(index);
                            Rect::new(caret.x + 2., caret.y + caret.height * 0.6, 1., 1.)
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                self.decorations = Some(DecorationCache {
                    snapshot,
                    source: source.clone(),
                    runs,
                    whitespace,
                });
            }
        } else {
            self.decorations = None;
        }
        list.ops.push(PaintOp::PushClip(self.bounds));
        list.ops.push(PaintOp::PushTransform(Transform::translate(
            origin[0], origin[1],
        )));
        if let Some(source) = decorations {
            let layout = self.editor.layout(fonts);
            for (_, line) in layout.visible_lines(top, bottom) {
                for span in &source.line_backgrounds {
                    if span.range.start == line.range.start
                        || (span.range.start < line.range.end && span.range.end > line.range.start)
                    {
                        list.rect(
                            Rect::new(
                                0.,
                                line.top,
                                self.bounds.width.max(layout.full_width()),
                                line.bottom - line.top,
                            ),
                            0.,
                            span.color,
                        );
                    }
                }
            }
            for span in &source.backgrounds {
                for rect in layout.selection(span.range.start, span.range.end) {
                    list.rect(rect, 0., span.color);
                }
            }
        }
        if self.editor.is_focused() {
            for rect in self.editor.selection(fonts) {
                list.rect(rect, 0., colors.selection);
            }
        }
        if let Some(cache) = &self.decorations {
            for (run, color) in &cache.runs {
                list.ops.push(PaintOp::GlyphRun {
                    run: run.clone(),
                    origin: [0.; 2],
                    color: color.unwrap_or(colors.text),
                });
            }
            if let Some(color) = cache.source.whitespace {
                for rect in &cache.whitespace {
                    if rect.y < bottom && rect.y + rect.height > top {
                        list.rect(*rect, 0., color);
                    }
                }
            }
        } else {
            for run in &self.editor.layout(fonts).runs {
                list.ops.push(PaintOp::GlyphRun {
                    run: run.clone(),
                    origin: [0.; 2],
                    color: colors.text,
                });
            }
        }
        if let Some(source) = decorations {
            let layout = self.editor.layout(fonts);
            for span in &source.underlines {
                for rect in layout.selection(span.range.start, span.range.end) {
                    list.rect(
                        Rect::new(rect.x, rect.y + rect.height - 1., rect.width, 1.),
                        0.,
                        span.color,
                    );
                }
            }
        }
        let underline = self.bounds.height.min(1.);
        for rect in self.editor.composition_geometry(fonts) {
            if rect.y >= bottom || rect.y + rect.height <= top {
                continue;
            }
            // Keep the composition marker visible when fallback-font ink is
            // taller than an authored line box. Off-screen rows stay clipped.
            let y = (rect.y + rect.height - underline).clamp(top, bottom - underline);
            list.rect(
                Rect::new(rect.x, y, rect.width, underline),
                0.,
                colors.composition,
            );
        }
        if self.editor.accepts_text_input() && caret_visible {
            if let Some(caret) = self.editor.caret(fonts) {
                list.rect(caret, 0., colors.caret);
            }
        }
        list.ops.push(PaintOp::PopTransform);
        list.ops.push(PaintOp::PopClip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::LogicalKey;
    fn fixture(multiline: bool, text: &str, bounds: Rect) -> (TextSystem, EditorView) {
        let mut fonts = TextSystem::default();
        fonts.register_font(include_bytes!("../../embedded/fonts/Roboto-Bold.ttf").to_vec());
        let mut view = EditorView::new(multiline);
        view.editor.set_text(&mut fonts, text);
        view.prepare(
            &mut fonts,
            TextOptions {
                family: "Roboto",
                size: 18.,
                width: Some(bounds.width),
                line_height: Some(24.),
            },
            bounds,
            !multiline,
        );
        (fonts, view)
    }
    #[test]
    fn translated_scrolled_editor_keeps_caret_candidate_and_hits_consistent() {
        let bounds = Rect::new(37.5, 29.25, 80., 36.);
        let (mut fonts, mut view) = fixture(false, "Long filename 東京             ", bounds);
        view.set_focused(&mut fonts, true);
        assert!(view.scroll()[0] > 0.);
        let area = view.ime_area(&mut fonts).unwrap();
        assert!(
            area.x >= bounds.x && area.x <= bounds.x + bounds.width + 0.01,
            "{area:?}"
        );
        let origin = view.origin(&mut fonts);
        let caret = view.editor.caret(&mut fonts).unwrap();
        let local_area = view.editor.ime_area(&mut fonts);
        assert!((area.x - local_area.x - origin[0]).abs() < 0.01);
        view.pointer_down(
            &mut fonts,
            [caret.x + origin[0], caret.y + origin[1] + caret.height / 2.],
            1,
        );
        assert_eq!(view.editor.selection_bytes().1, view.editor.text().len());
        let builds = view.editor.snapshots_built();
        let mut paint = PaintList::default();
        view.paint(&mut fonts, &mut paint, EditorColors::default(), true);
        paint.validate().unwrap();
        assert!(matches!(paint.ops.first(), Some(PaintOp::PushClip(r)) if *r == bounds));
        view.paint(
            &mut fonts,
            &mut PaintList::default(),
            EditorColors::default(),
            false,
        );
        assert_eq!(view.editor.snapshots_built(), builds);
    }
    #[test]
    fn ime_candidate_stays_visible_for_tiny_and_zero_sized_viewports() {
        let bounds = Rect::new(30., 20., 32., 8.);
        let (mut fonts, mut view) = fixture(false, "Long command", bounds);
        view.set_focused(&mut fonts, true);
        view.input(
            &mut fonts,
            &InputEvent::Composition {
                text: "東京東京東京".into(),
                cursor: Some((0, 18)),
            },
        );
        let area = view.ime_area(&mut fonts).unwrap();
        assert!(
            area.x >= bounds.x
                && area.y >= bounds.y
                && area.x + area.width <= bounds.x + bounds.width
                && area.y + area.height <= bounds.y + bounds.height,
            "{area:?}"
        );
        let mut list = PaintList::default();
        let color = [1., 0., 1., 1.];
        let origin = view.origin(&mut fonts);
        view.paint(
            &mut fonts,
            &mut list,
            EditorColors {
                composition: color,
                ..Default::default()
            },
            true,
        );
        let markers = list
            .ops
            .iter()
            .filter_map(|op| match op {
                PaintOp::Rect {
                    rect,
                    brush: crate::paint::Brush::Solid(paint_color),
                    ..
                } if *paint_color == color => Some(rect),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!markers.is_empty());
        assert!(markers.iter().all(|r| r.y + origin[1] >= bounds.y
            && r.y + r.height + origin[1] <= bounds.y + bounds.height));
        view.prepare(
            &mut fonts,
            TextOptions {
                family: "Roboto",
                size: 20.,
                width: None,
                line_height: None,
            },
            Rect::new(30., 20., 0., 8.),
            false,
        );
        assert!(view.ime_area(&mut fonts).is_none());
    }
    #[test]
    fn page_selection_drag_scroll_and_focus_are_viewport_operations() {
        let bounds = Rect::new(40., 30., 160., 48.);
        let (mut fonts, mut view) = fixture(
            true,
            "First line\nSecond line\nThird line\nFourth line\nFifth",
            bounds,
        );
        view.set_focused(&mut fonts, true);
        view.editor.select_bytes(&mut fonts, 0, 0);
        view.reveal_caret(&mut fonts);
        assert_eq!(view.scroll()[1], 0.);
        view.input(
            &mut fonts,
            &InputEvent::Modifiers(Modifiers {
                shift: true,
                ..Modifiers::default()
            }),
        );
        let key = InputEvent::Key {
            physical: PhysicalKey::PageDown,
            logical: LogicalKey::Other,
            pressed: true,
            repeat: false,
        };
        assert!(view.input(&mut fonts, &key).input.consumed);
        assert!(view.editor.selection_bytes().1 > "First line\n".len());
        assert!(!view.editor.selection(&mut fonts).is_empty());
        assert!(view.scroll()[1] > 0.);
        view.input(&mut fonts, &InputEvent::Modifiers(Modifiers::default()));
        view.pointer_down(&mut fonts, [45., 35.], 1);
        assert!(
            view.input(
                &mut fonts,
                &InputEvent::PointerMoved {
                    position: [190., 150.]
                }
            )
            .input
            .consumed
        );
        assert!(view.editor.selection_bytes().0 != view.editor.selection_bytes().1);
        assert!(
            view.input(
                &mut fonts,
                &InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed: false,
                    position: [190., 150.]
                }
            )
            .input
            .consumed
        );
        view.input(&mut fonts, &InputEvent::Focus(false));
        assert!(view.ime_area(&mut fonts).is_none());
        assert!(!view.input(&mut fonts, &key).input.consumed);
        assert!(!view.dragging);
    }
    #[test]
    fn double_and_triple_click_share_word_and_hard_line_geometry() {
        let (mut fonts, mut view) = fixture(
            true,
            "Hello world\nNext line",
            Rect::new(20., 10., 200., 80.),
        );
        view.editor.select_bytes(&mut fonts, 0, 0);
        let caret = view.editor.caret(&mut fonts).unwrap();
        let point = [20. + caret.x + 2., 10. + caret.y + caret.height / 2.];
        view.pointer_down(&mut fonts, point, 2);
        assert_eq!(view.editor.selection_bytes(), (0, 5));
        view.pointer_down(&mut fonts, point, 3);
        let (a, b) = view.editor.selection_bytes();
        assert_eq!(&view.editor.text()[a..b], "Hello world");
    }
}

#[cfg(test)]
#[test]
fn repeated_prepare_keeps_user_scrollback_until_caret_moves() {
    let mut fonts = TextSystem::default();
    fonts.register_font(include_bytes!("../../embedded/fonts/Roboto-Bold.ttf").to_vec());
    let mut view = EditorView::new(true);
    view.editor
        .set_text(&mut fonts, "One\nTwo\nThree\nFour\nFive\nSix\nSeven");
    let bounds = Rect::new(0., 0., 160., 48.);
    let options = || TextOptions {
        family: "Roboto",
        size: 18.,
        width: Some(160.),
        line_height: Some(24.),
    };
    view.prepare(&mut fonts, options(), bounds, false);
    view.set_focused(&mut fonts, true);
    assert!(view.scroll()[1] > 0.);
    view.input(
        &mut fonts,
        &InputEvent::PointerMoved {
            position: [10., 10.],
        },
    );
    view.input(
        &mut fonts,
        &InputEvent::Scroll {
            delta: [0., 1000.],
            precise: true,
        },
    );
    assert_eq!(view.scroll()[1], 0.);
    for _ in 0..3 {
        view.set_focused(&mut fonts, true);
        view.prepare(&mut fonts, options(), bounds, false);
        assert_eq!(view.scroll()[1], 0.);
    }
    view.input(&mut fonts, &InputEvent::Text("!".into()));
    assert!(view.scroll()[1] > 0.);
}

#[cfg(test)]
mod decoration_tests {
    use super::*;
    fn fixture() -> (TextSystem, EditorView) {
        let mut fonts = TextSystem::default();
        fonts.register_font(include_bytes!("../../embedded/fonts/Roboto-Bold.ttf").to_vec());
        let mut view = EditorView::new(true);
        view.editor.set_text(
            &mut fonts,
            "let title = \"Eldiron\";\n// العربية e\u{301}\n",
        );
        view.prepare(
            &mut fonts,
            TextOptions {
                family: "Roboto",
                size: 18.,
                width: None,
                line_height: Some(24.),
            },
            Rect::new(16., 16., 360., 96.),
            false,
        );
        (fonts, view)
    }
    #[test]
    fn decorations_reuse_shaped_runs_and_do_not_move_selection_or_rebuild_layout() {
        let (mut fonts, mut view) = fixture();
        view.set_focused(&mut fonts, true);
        view.editor.select_bytes(&mut fonts, 4, 9);
        let selection = view.editor.selection_bytes();
        let caret = view.editor.caret(&mut fonts);
        let builds = view.editor.snapshots_built();
        let mut decorations = Arc::new(EditorDecorations {
            foreground: vec![TextStyleSpan {
                range: 0..3,
                color: [0., 0.5, 1., 1.],
            }],
            backgrounds: vec![TextStyleSpan {
                range: 4..9,
                color: [0.2, 0.4, 0.8, 0.5],
            }],
            underlines: vec![TextStyleSpan {
                range: 12..21,
                color: [1., 0., 0., 1.],
            }],
            line_backgrounds: vec![TextStyleSpan {
                range: 22..40,
                color: [0.2, 0., 0., 0.5],
            }],
            whitespace: Some([0.5; 4]),
        });
        let paint =
            |view: &mut EditorView, fonts: &mut TextSystem, styles: &Arc<EditorDecorations>| {
                let mut list = PaintList::default();
                view.paint_decorated(fonts, &mut list, EditorColors::default(), true, styles);
                list.validate().unwrap();
                list
            };
        let first = paint(&mut view, &mut fonts, &decorations);
        let second = paint(&mut view, &mut fonts, &decorations);
        let glyphs = |list: &PaintList| {
            list.ops
                .iter()
                .filter_map(|op| {
                    if let PaintOp::GlyphRun { run, .. } = op {
                        Some(run.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        let before = glyphs(&first);
        let after = glyphs(&second);
        assert_eq!(before.len(), after.len());
        assert!(
            before.iter().zip(&after).all(|(a, b)| Arc::ptr_eq(a, b)),
            "unchanged decorations keep their immutable glyph runs"
        );
        Arc::make_mut(&mut decorations).foreground[0].color = [0., 1., 0., 1.];
        let third = paint(&mut view, &mut fonts, &decorations);
        assert!(
            third
                .ops
                .iter()
                .any(|op| matches!(op,PaintOp::GlyphRun { color,.. } if *color == [0.,1.,0.,1.]))
        );
        assert_eq!(view.editor.snapshots_built(), builds);
        assert_eq!(view.editor.selection_bytes(), selection);
        assert_eq!(view.editor.caret(&mut fonts), caret);
    }
    #[test]
    fn indexed_visible_lines_skip_a_large_document_prefix_and_follow_explicit_scroll() {
        let (mut fonts, mut view) = fixture();
        let source = (0..2000)
            .map(|index| format!("line {index}\n"))
            .collect::<String>();
        view.editor.set_text(&mut fonts, &source);
        view.prepare(
            &mut fonts,
            TextOptions {
                family: "Roboto",
                size: 18.,
                width: None,
                line_height: Some(24.),
            },
            Rect::new(16., 16., 360., 48.),
            false,
        );
        let builds = view.editor.snapshots_built();
        view.set_scroll(&mut fonts, [0., 24_000.]);
        let visible = view.visible_lines(&mut fonts);
        assert!(!visible.is_empty() && visible.len() <= 3);
        assert!(visible[0].0 >= 999);
        for (index, line) in visible {
            assert!(source[line.range.clone()].starts_with(&format!("line {index}")));
            assert!(line.top < 24_048. && line.bottom > 24_000.);
        }
        assert_eq!(
            view.editor.snapshots_built(),
            builds,
            "scrolling does not reshape the document"
        );
    }
}

#[cfg(test)]
#[test]
fn readonly_streaming_view_keeps_explicit_scrollback_and_exposes_no_ime_candidate() {
    let mut fonts = TextSystem::default();
    fonts.register_font(include_bytes!("../../embedded/fonts/Roboto-Bold.ttf").to_vec());
    let mut view = EditorView::new(true);
    let source = "One\nTwo\nThree\nFour\nFive\nSix\nSeven";
    view.editor.set_text(&mut fonts, source);
    view.editor.set_policy(
        &mut fonts,
        super::EditorPolicy {
            read_only: true,
            ..Default::default()
        },
    );
    view.set_auto_reveal(false);
    let options = || TextOptions {
        family: "Roboto",
        size: 18.,
        width: Some(160.),
        line_height: Some(24.),
    };
    let bounds = Rect::new(16., 16., 160., 48.);
    view.prepare(&mut fonts, options(), bounds, false);
    view.set_focused(&mut fonts, true);
    assert_eq!(view.scroll(), [0.; 2]);
    assert!(view.text_input_state(&mut fonts).is_none());
    view.editor
        .set_text(&mut fonts, &format!("{source}\nEight"));
    view.prepare(&mut fonts, options(), bounds, false);
    assert_eq!(view.scroll(), [0.; 2]);
    view.reveal_caret(&mut fonts);
    assert!(view.scroll()[1] > 0.);
    view.set_scroll(&mut fonts, [0.; 2]);
    view.set_focused(&mut fonts, false);
    view.set_focused(&mut fonts, true);
    assert_eq!(view.scroll(), [0.; 2]);
}
