//! Logical viewport and paint adapter for the neutral editor. The containing
//! widget supplies chrome, focus order, clipboard services and blink scheduling.
use super::{EditorController, EditorResponse};
use crate::{
    paint::{Color, PaintList, PaintOp, Rect, Transform},
    platform::{InputEvent, Modifiers, PhysicalKey, PointerButton},
    text::{TextOptions, TextSystem},
};

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
        if self.editor.is_focused()
            && (old_bounds != self.bounds
                || old_center != self.center_y
                || builds != self.editor.snapshots_built())
        {
            self.reveal_caret(fonts);
        }
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
        if focused {
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
                    let old = self.scroll;
                    let unit = if *precise { 1. } else { 24. };
                    for (scroll, delta) in self.scroll.iter_mut().zip(delta) {
                        if delta.is_finite() {
                            *scroll -= delta * unit;
                        }
                    }
                    self.clamp_scroll(fonts);
                    let mut result = EditorResponse::handled(false);
                    result.input.redraw = old != self.scroll;
                    return result;
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
        if response.input.redraw {
            self.reveal_caret(fonts);
        }
        response
    }
    pub fn ime_area(&mut self, fonts: &mut TextSystem) -> Option<Rect> {
        if !self.editor.is_focused() {
            return None;
        }
        let local = self.editor.ime_area(fonts);
        let origin = self.origin(fonts);
        Some(Rect::new(
            local.x + origin[0],
            local.y + origin[1],
            local.width,
            local.height,
        ))
    }
    pub fn paint(
        &mut self,
        fonts: &mut TextSystem,
        list: &mut PaintList,
        colors: EditorColors,
        caret_visible: bool,
    ) {
        if self.bounds.width <= 0. || self.bounds.height <= 0. {
            return;
        }
        let origin = self.origin(fonts);
        list.ops.push(PaintOp::PushClip(self.bounds));
        list.ops.push(PaintOp::PushTransform(Transform::translate(
            origin[0], origin[1],
        )));
        if self.editor.is_focused() {
            for rect in self.editor.selection(fonts) {
                list.rect(rect, 0., colors.selection);
            }
        }
        for run in &self.editor.layout(fonts).runs {
            list.ops.push(PaintOp::GlyphRun {
                run: run.clone(),
                origin: [0.; 2],
                color: colors.text,
            });
        }
        for rect in self.editor.composition_geometry(fonts) {
            list.rect(
                Rect::new(rect.x, rect.y + rect.height - 1., rect.width, 1.),
                0.,
                colors.composition,
            );
        }
        if self.editor.is_focused() && caret_visible {
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
