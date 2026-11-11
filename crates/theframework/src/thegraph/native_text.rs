//! App-owned native graph text drafts. The graph owns values and reversible
//! transactions; this session owns shaped editing, IME, clipboard and local undo.
//! It depends on neither a window, a widget tree, nor a GPU implementation.
use super::*;
use crate::{
    paint::Rect,
    platform::{InputEvent, Modifiers, PointerButton, TextInputState},
    text::{TextOptions, TextSystem},
    text_edit::{
        EditorAction, EditorColors, EditorCommand, EditorPolicy, EditorResponse, EditorView,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Target {
    node: GraphId,
    row: GraphId,
    cell: Option<(usize, usize)>,
}
impl From<&GraphTextFocus> for Target {
    fn from(focus: &GraphTextFocus) -> Self {
        Self {
            node: focus.node,
            row: focus.row,
            cell: focus.cell,
        }
    }
}
impl GraphTextFocus {
    /// The committed draft addressed by this graph focus, without preedit.
    pub fn value<'a>(&self, doc: &'a GraphDocument) -> Option<&'a str> {
        let row = doc
            .nodes
            .iter()
            .find(|n| n.id == self.node)?
            .rows
            .iter()
            .find(|r| r.id == self.row)?;
        match (&row.value, self.cell) {
            (GraphControlValue::Text(text), None) => Some(text),
            (GraphControlValue::List { rows, .. }, Some((row, column))) => {
                match rows.get(row)?.get(column)? {
                    GraphControlValue::Text(text) => Some(text),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// A draft session survives graph zoom, layout, theme and renderer replacement.
/// Hosts forward modifier/focus transitions and route editing events only while
/// this graph is the foremost input scope. Clipboard responses stay in
/// `EditorResponse` for the host to service.
pub struct GraphTextSession {
    target: Option<(u64, Target)>,
    view: Option<EditorView>,
    modifiers: Modifiers,
    family: String,
}
impl Default for GraphTextSession {
    fn default() -> Self {
        Self {
            target: None,
            view: None,
            modifiers: Modifiers::default(),
            family: "Roboto".into(),
        }
    }
}
impl GraphTextSession {
    /// Bounds are logical coordinates local to the graph viewport. Hosts use the
    /// same enclosing transform for pointer input, paint and IME candidate area.
    pub fn bounds(&self) -> Option<Rect> {
        self.view.as_ref().map(EditorView::bounds)
    }
    pub fn prepare(
        &mut self,
        graph: &mut GraphEditor,
        doc: &GraphDocument,
        fonts: &mut TextSystem,
        active: bool,
    ) -> bool {
        let Some(focus) = graph.text_focus().cloned() else {
            self.reset();
            return false;
        };
        let Some((rect, size)) = layout(graph, doc, &focus) else {
            self.reset();
            return false;
        };
        let Some(source) = focus.value(doc) else {
            self.reset();
            return false;
        };
        let target = (graph.text_epoch, Target::from(&focus));
        if self.target != Some(target) || self.view.is_none() {
            // Multiline storage preserves exact authored text (including CRLF).
            // Enter submits the inline draft; paste/loaded text is not rewritten.
            let mut view = EditorView::new(true);
            view.editor.set_policy(
                fonts,
                EditorPolicy {
                    submit_on_enter: true,
                    ..Default::default()
                },
            );
            view.editor.set_text(fonts, source);
            view.editor.select_bytes(fonts, focus.anchor, focus.caret);
            view.set_selection_drag_move(true);
            self.view = Some(view);
            self.target = Some(target);
        }
        let view = self.view.as_mut().unwrap();
        if view.editor.text() != source {
            // An external document replacement establishes a new draft
            // baseline. Escape must never restore a value from the old model.
            graph.text_focus.as_mut().unwrap().original = source.to_owned();
            view.editor.set_text(fonts, source);
            view.editor.select_bytes(fonts, focus.anchor, focus.caret);
        } else if view.editor.committed_selection_bytes() != (focus.anchor, focus.caret) {
            view.editor.select_bytes(fonts, focus.anchor, focus.caret);
        }
        view.prepare(
            fonts,
            TextOptions {
                family: &self.family,
                size,
                width: None,
                line_height: None,
            },
            rect,
            false,
        );
        view.set_focused(fonts, active);
        if active {
            view.input(fonts, &InputEvent::Modifiers(self.modifiers));
        }
        let (anchor, caret) = view.editor.committed_selection_bytes();
        let focus = graph.text_focus.as_mut().unwrap();
        focus.anchor = anchor;
        focus.caret = caret;
        true
    }
    pub fn set_font_family(&mut self, family: impl Into<String>) {
        self.family = family.into();
    }
    /// Paint graph geometry and the shaped draft in the row/cell's original
    /// painter order. No CPU destination or raster font metrics are involved.
    pub fn paint(
        &mut self,
        graph: &mut GraphEditor,
        doc: &GraphDocument,
        context: &dyn GraphContext,
        controls: &dyn GraphControls,
        painter: PaintGraphPainter<'_>,
        size: Point,
        theme: &GraphTheme,
        active: bool,
    ) {
        self.set_font_family(painter.family);
        self.prepare(graph, doc, painter.text, active);
        let mut painter = GraphTextPainter {
            painter,
            session: self,
        };
        graph.paint(doc, context, controls, &mut painter, size, theme);
    }
    pub fn reset(&mut self) {
        self.view = None;
        self.target = None;
    }
    pub fn input_state(
        &mut self,
        graph: &mut GraphEditor,
        doc: &GraphDocument,
        fonts: &mut TextSystem,
        active: bool,
    ) -> Option<TextInputState> {
        self.prepare(graph, doc, fonts, active);
        self.view.as_mut()?.text_input_state(fonts)
    }
    /// Pointer/key/IME input for an already focused graph value. Unconsumed input
    /// remains available to graph gestures or application accelerators.
    pub fn input(
        &mut self,
        graph: &mut GraphEditor,
        doc: &mut GraphDocument,
        fonts: &mut TextSystem,
        event: &InputEvent,
        active: bool,
    ) -> EditorResponse {
        if let InputEvent::Modifiers(modifiers) = event {
            self.modifiers = *modifiers;
        }
        if matches!(event, InputEvent::Focus(false)) {
            self.modifiers = Modifiers::default();
        }
        let active = active && !matches!(event, InputEvent::Focus(false));
        let was_active = self
            .view
            .as_ref()
            .is_some_and(|view| view.editor.is_focused());
        if !self.prepare(graph, doc, fonts, active) || !active {
            return EditorResponse {
                input: crate::platform::InputResponse {
                    consumed: false,
                    redraw: was_active,
                },
                ..Default::default()
            };
        }
        let response = self.view.as_mut().unwrap().input(fonts, event);
        self.project(graph, doc, response)
    }
    pub fn command(
        &mut self,
        graph: &mut GraphEditor,
        doc: &mut GraphDocument,
        fonts: &mut TextSystem,
        command: EditorCommand,
        active: bool,
    ) -> EditorResponse {
        if !self.prepare(graph, doc, fonts, active) || !active {
            return EditorResponse::default();
        }
        let view = self.view.as_mut().unwrap();
        let response = view.editor.execute(fonts, command);
        if response.input.redraw {
            view.reveal_caret(fonts);
        }
        self.project(graph, doc, response)
    }
    /// Enter/reposition an inline editor using its shaped geometry. Returns
    /// `None` when the click started an ordinary graph gesture. A click inside
    /// the current editor keeps its local undo/IME epoch instead of recommitting.
    pub fn pointer_down(
        &mut self,
        graph: &mut GraphEditor,
        doc: &mut GraphDocument,
        fonts: &mut TextSystem,
        point: Point,
        controls: &dyn GraphControls,
    ) -> Option<EditorResponse> {
        graph.cursor = point;
        self.prepare(graph, doc, fonts, true);
        let same = self.bounds().is_some_and(|bounds| bounds.contains(point));
        if !same {
            graph.pointer_down(doc, point, controls);
            if !self.prepare(graph, doc, fonts, true) {
                return None;
            }
        }
        let bounds = self.bounds().unwrap();
        let point = [
            point[0].clamp(bounds.x, bounds.x + bounds.width.max(0.1) - 0.1),
            point[1].clamp(bounds.y, bounds.y + bounds.height.max(0.1) - 0.1),
        ];
        Some(self.input(
            graph,
            doc,
            fonts,
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position: point,
            },
            true,
        ))
    }
    fn project(
        &mut self,
        graph: &mut GraphEditor,
        doc: &mut GraphDocument,
        mut response: EditorResponse,
    ) -> EditorResponse {
        let Some(focus) = graph.text_focus.as_mut() else {
            self.reset();
            return response;
        };
        let Some(view) = &self.view else {
            return response;
        };
        let Some(value) = super::text::focused_text(doc, focus) else {
            self.reset();
            return response;
        };
        if response.changed {
            *value = view.editor.text().to_owned();
        }
        let (anchor, caret) = view.editor.committed_selection_bytes();
        focus.anchor = anchor;
        focus.caret = caret;
        if let Some(action) = response.action {
            match action {
                EditorAction::Cancel => {
                    let changed = *value != focus.original;
                    graph.finish_text(doc, false);
                    response.changed |= changed;
                }
                EditorAction::Submit | EditorAction::FocusNext { .. } => {
                    graph.finish_text(doc, true)
                }
            }
            self.reset();
        }
        response
    }
}

fn layout(graph: &GraphEditor, doc: &GraphDocument, focus: &GraphTextFocus) -> Option<(Rect, f32)> {
    let node = doc
        .nodes
        .iter()
        .find(|n| n.id == focus.node && !n.folded && graph.node_visible(n.id))?;
    let index = node
        .rows
        .iter()
        .position(|r| r.id == focus.row && r.binding.is_none())?;
    let metrics = doc.metrics();
    let row = &node.rows[index];
    let rect = node.row_rect(index, &metrics);
    let (rect, size) = match (&row.value, focus.cell) {
        (GraphControlValue::Text(_), None) => (
            GraphRect {
                origin: [rect.origin[0] + 9., rect.origin[1] + 4.],
                size: [rect.size[0] - 18., 22.],
            },
            metrics.text_size,
        ),
        (GraphControlValue::List { columns, rows, .. }, Some((r, c)))
            if rows
                .get(r)
                .and_then(|cells| cells.get(c))
                .is_some_and(|v| matches!(v, GraphControlValue::Text(_))) =>
        {
            let width = rect.size[0] * LIST_DELETE_FRACTION / columns.len().max(1) as f32;
            (
                GraphRect {
                    origin: [
                        rect.origin[0] + width * c as f32 + 6.,
                        rect.origin[1]
                            + metrics.list_header
                            + metrics.list_row * r as f32
                            + ((metrics.list_row - 18.) * 0.5).max(0.),
                    ],
                    size: [width - 10., 18.],
                },
                metrics.cell_size,
            )
        }
        _ => return None,
    };
    let origin = graph.viewport.to_screen(rect.origin);
    let zoom = graph.viewport.zoom();
    Some((
        Rect::new(
            origin[0],
            origin[1],
            rect.size[0] * zoom,
            rect.size[1] * zoom,
        ),
        size * zoom,
    ))
}

/// Graph painter with the shared native editor at the original row/cell paint
/// position. Other node bodies/previews can still cover it in painter order.
struct GraphTextPainter<'s, 'p> {
    pub painter: PaintGraphPainter<'p>,
    pub session: &'s mut GraphTextSession,
}
impl GraphPainter for GraphTextPainter<'_, '_> {
    fn round_rect(&mut self, rect: GraphRect, radius: f32, color: GraphColor) {
        self.painter.round_rect(rect, radius, color);
    }
    fn text_width(&mut self, text: &str, size: f32) -> f32 {
        self.painter.text_width(text, size)
    }
    fn text(&mut self, rect: GraphRect, text: &str, size: f32, color: GraphColor) {
        self.painter.text(rect, text, size, color);
    }
    fn curve(&mut self, points: [Point; 4], width: f32, color: GraphColor) {
        self.painter.curve(points, width, color);
    }
    fn preview(&mut self, rect: GraphRect, asset: &str) {
        self.painter.preview(rect, asset);
    }
    fn focused_text(
        &mut self,
        focus: &GraphTextFocus,
        rect: GraphRect,
        size: f32,
        theme: &GraphTheme,
    ) -> bool {
        if self.session.target.map(|(_, target)| target) != Some(Target::from(focus)) {
            return false;
        }
        let Some(view) = &mut self.session.view else {
            return false;
        };
        view.prepare(
            self.painter.text,
            TextOptions {
                family: self.painter.family,
                size,
                width: None,
                line_height: None,
            },
            Rect::new(rect.origin[0], rect.origin[1], rect.size[0], rect.size[1]),
            false,
        );
        let rgba = |color: GraphColor| color.map(|v| v as f32 / 255.);
        view.paint(
            self.painter.text,
            self.painter.paint,
            EditorColors {
                text: rgba(theme.text),
                selection: [0.16, 0.44, 0.53, 0.8],
                caret: rgba(theme.text),
                composition: rgba(theme.active),
            },
            true,
        );
        true
    }
}
