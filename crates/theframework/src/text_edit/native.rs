//! Native IME metadata/range edits use the same Parley document as paint. No
//! native widget, renderer, serialized state or destination pixels live here.
use super::*;
use crate::platform::{NativeTextAction, TextInputTarget};
use std::{
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT_SOURCE: AtomicU64 = AtomicU64::new(1);

/// Immutable displayed source, including transient preedit. Selection ranges
/// and composition address these UTF-8 bytes. Revision changes with source or
/// composition identity, and survives font/layout/DPI and selection-only changes.
#[derive(Clone, Debug)]
pub struct NativeTextSnapshot {
    pub target: TextInputTarget,
    pub revision: u64,
    pub text: Arc<str>,
    pub selection: (usize, usize),
    pub composition: Option<Range<usize>>,
}
/// Queries use the same displayed source/Parley geometry as paint. Hit positions
/// and answers are logical coordinates; hosts apply their enclosing transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeTextDirection {
    Left,
    Right,
    Up,
    Down,
}
#[derive(Clone, Debug, PartialEq)]
pub enum NativeTextQuery {
    Geometry(Range<usize>),
    Hit([f32; 2]),
    Navigate {
        position: usize,
        direction: NativeTextDirection,
        count: usize,
    },
}
#[derive(Clone, Debug)]
pub struct NativeTextRequest {
    pub target: TextInputTarget,
    pub revision: u64,
    pub query: NativeTextQuery,
}
#[derive(Clone, Debug, PartialEq)]
pub enum NativeTextAnswer {
    Geometry {
        caret: Rect,
        selection: Vec<Rect>,
        first_range: Range<usize>,
    },
    Hit(usize),
}
impl NativeTextRequest {
    pub fn map_hit(&self, transform: crate::paint::Transform) -> Self {
        let mut request = self.clone();
        if let NativeTextQuery::Hit(point) = &mut request.query {
            *point = transform.point(*point);
        }
        request
    }
}
impl NativeTextAnswer {
    pub fn transformed(self, transform: crate::paint::Transform) -> Self {
        let rect = |r: Rect| {
            let points = [
                [r.x, r.y],
                [r.x + r.width, r.y],
                [r.x, r.y + r.height],
                [r.x + r.width, r.y + r.height],
            ]
            .map(|p| transform.point(p));
            let min = std::array::from_fn::<_, 2, _>(|i| {
                points.iter().map(|p| p[i]).fold(f32::INFINITY, f32::min)
            });
            let max = std::array::from_fn::<_, 2, _>(|i| {
                points
                    .iter()
                    .map(|p| p[i])
                    .fold(f32::NEG_INFINITY, f32::max)
            });
            Rect::new(min[0], min[1], max[0] - min[0], max[1] - min[1])
        };
        match self {
            Self::Geometry {
                caret,
                selection,
                first_range,
            } => Self::Geometry {
                caret: rect(caret),
                selection: selection.into_iter().map(rect).collect(),
                first_range,
            },
            hit => hit,
        }
    }
}
pub(super) struct NativeTextCache {
    snapshot: NativeTextSnapshot,
    edit_revision: u64,
}
/// Convert Apple UTF-16 positions into source UTF-8 without splitting scalars.
pub fn utf16_byte_offset(text: &str, position: usize) -> Option<usize> {
    let mut units = 0;
    for (byte, c) in text.char_indices() {
        if units == position {
            return Some(byte);
        }
        units += c.len_utf16();
        if units > position {
            return None;
        }
    }
    (units == position).then_some(text.len())
}
impl NativeTextSnapshot {
    /// Apple text positions count UTF-16 code units. Never split a surrogate or
    /// silently turn an invalid/stale range into a different valid edit.
    pub fn byte_from_utf16(&self, position: usize) -> Option<usize> {
        utf16_byte_offset(&self.text, position)
    }
    pub fn utf16_from_byte(&self, position: usize) -> Option<usize> {
        self.text
            .get(..position)
            .map(|prefix| prefix.encode_utf16().count())
    }
    pub fn byte_range_from_utf16(&self, start: usize, end: usize) -> Option<Range<usize>> {
        (start <= end).then_some(())?;
        Some(self.byte_from_utf16(start)?..self.byte_from_utf16(end)?)
    }
}
impl EditorController {
    pub fn native_text_snapshot(&mut self, fonts: &mut TextSystem) -> Option<NativeTextSnapshot> {
        if !self.accepts_text_input() {
            return None;
        }
        let text = self.layout(fonts).source.clone();
        let composition = self.editor.raw_compose().clone();
        let revision = self
            .native_snapshot
            .as_ref()
            .filter(|old| {
                old.edit_revision == self.edit_revision
                    && old.snapshot.target == self.input_target
                    && old.snapshot.text == text
                    && old.snapshot.composition == composition
            })
            .map_or_else(
                || NEXT_SOURCE.fetch_add(1, Ordering::Relaxed),
                |old| old.snapshot.revision,
            );
        let snapshot = NativeTextSnapshot {
            target: self.input_target,
            revision,
            text,
            selection: self.selection_bytes(),
            composition,
        };
        self.native_snapshot = Some(NativeTextCache {
            snapshot: snapshot.clone(),
            edit_revision: self.edit_revision,
        });
        Some(snapshot)
    }
    pub fn native_text_query(
        &mut self,
        fonts: &mut TextSystem,
        request: &NativeTextRequest,
    ) -> Option<NativeTextAnswer> {
        let source = self
            .native_text_snapshot(fonts)
            .filter(|s| s.target == request.target && s.revision == request.revision)?;
        match &request.query {
            NativeTextQuery::Geometry(range) => {
                if range.start > range.end
                    || !source.text.is_char_boundary(range.start)
                    || !source.text.is_char_boundary(range.end)
                {
                    return None;
                }
                let layout = self.layout(fonts);
                let mut caret = layout.caret(range.end);
                let selection = layout.selection(range.start, range.end);
                let first_range = layout
                    .line_range_at(range.start)
                    .map(|line| range.start..range.end.min(line.end))
                    .unwrap_or_else(|| range.clone());
                // Preserve the active caret's bidi affinity, rather than
                // inventing a different side of a bidirectional boundary.
                if range.end == source.selection.1 {
                    let focus = self.editor.raw_selection().focus();
                    caret = box_rect(
                        focus.geometry(fonts.editor_driver(&mut self.editor).layout(), 1.),
                    );
                }
                Some(NativeTextAnswer::Geometry {
                    caret,
                    selection,
                    first_range,
                })
            }
            NativeTextQuery::Navigate {
                position,
                direction,
                count,
            } => {
                if !source.text.is_char_boundary(*position) {
                    return None;
                }
                self.update_boundaries();
                let focus = self.editor.raw_selection().focus();
                let mut driver = fonts.editor_driver(&mut self.editor);
                let layout = driver.layout();
                let mut selection = if *position == source.selection.1 {
                    parley::Selection::new(focus, focus)
                } else {
                    parley::Selection::from_byte_index(
                        layout,
                        *position,
                        parley::Affinity::Downstream,
                    )
                };
                // Pure layout query: never move the document's live selection or
                // undo boundary. Bound large external counts by the document.
                for _ in 0..(*count).min(source.text.len().saturating_mul(2).saturating_add(1)) {
                    let next = match direction {
                        NativeTextDirection::Left | NativeTextDirection::Right => {
                            visual_grapheme_selection(
                                selection,
                                layout,
                                &self.boundaries,
                                *direction == NativeTextDirection::Right,
                                false,
                            )
                        }
                        NativeTextDirection::Up => selection.previous_line(layout, false),
                        NativeTextDirection::Down => selection.next_line(layout, false),
                    };
                    if next.focus() == selection.focus() {
                        break;
                    }
                    selection = next;
                }
                let cursor = grapheme_cursor(selection.focus(), layout, &self.boundaries, false);
                Some(NativeTextAnswer::Hit(cursor.index()))
            }
            NativeTextQuery::Hit(point) => {
                if !point.iter().all(|v| v.is_finite()) {
                    return None;
                }
                self.update_boundaries();
                let mut driver = fonts.editor_driver(&mut self.editor);
                let layout = driver.layout();
                let cursor = grapheme_cursor(
                    parley::Cursor::from_point(layout, point[0], point[1]),
                    layout,
                    &self.boundaries,
                    false,
                );
                Some(NativeTextAnswer::Hit(cursor.index()))
            }
        }
    }
    fn finish_native_composition(&mut self, fonts: &mut TextSystem) -> bool {
        let Some(before) = self.composition.take() else {
            return false;
        };
        fonts.editor_driver(&mut self.editor).finish_compose();
        self.invalidate_source();
        self.snapshot = None;
        self.boundaries_dirty = true;
        self.snap_selection(fonts, true);
        if before.text == self.editor.raw_text() {
            return false;
        }
        self.record_change(before, self.state());
        self.clipboard = None;
        true
    }
    pub(super) fn replace_native_range(
        &mut self,
        fonts: &mut TextSystem,
        range: Range<usize>,
        text: &str,
    ) -> bool {
        self.commit_edit_group();
        let was_composing = self.is_composing();
        let before = self.composition.take().unwrap_or_else(|| self.state());
        let text = self.clean(text);
        let mut driver = fonts.editor_driver(&mut self.editor);
        driver.finish_compose();
        driver.select_byte_range(range.start, range.end);
        driver.insert_or_replace_selection(&text);
        self.snapshot = None;
        self.boundaries_dirty = true;
        self.snap_selection(fonts, true);
        let changed = before.text != self.editor.raw_text();
        if changed || was_composing {
            self.invalidate_source();
        }
        if changed {
            self.record_change(before, self.state());
        }
        self.clipboard = None;
        changed
    }
    pub(super) fn native_input(
        &mut self,
        fonts: &mut TextSystem,
        revision: u64,
        action: &NativeTextAction,
    ) -> EditorResponse {
        let Some(source) = self
            .native_text_snapshot(fonts)
            .filter(|s| s.revision == revision)
        else {
            return EditorResponse::default();
        };
        let boundary = |position| source.text.is_char_boundary(position);
        let valid = |range: &Range<usize>| {
            range.start <= range.end && boundary(range.start) && boundary(range.end)
        };
        match action {
            NativeTextAction::Select { anchor, focus } => {
                if !boundary(*anchor) || !boundary(*focus) {
                    return EditorResponse::default();
                }
                let inside = source
                    .composition
                    .as_ref()
                    .is_some_and(|r| r.contains(anchor) || *anchor == r.end)
                    && source
                        .composition
                        .as_ref()
                        .is_some_and(|r| r.contains(focus) || *focus == r.end);
                let changed = !inside && self.finish_native_composition(fonts);
                fonts
                    .editor_driver(&mut self.editor)
                    .select_byte_range(*anchor, *focus);
                if !self.is_composing() {
                    self.snap_selection(fonts, false);
                }
                self.clipboard = None;
                EditorResponse::handled(changed)
            }
            NativeTextAction::Insert { selection, text } => {
                if *selection != source.selection {
                    return EditorResponse::default();
                }
                if text.is_empty()
                    && let Some(range) = source.composition
                {
                    return EditorResponse::handled(self.replace_native_range(fonts, range, ""));
                }
                self.input(
                    fonts,
                    &InputEvent::TextFor {
                        target: source.target,
                        text: text.clone(),
                    },
                )
            }
            NativeTextAction::DeleteBackward { selection } => {
                if *selection != source.selection {
                    return EditorResponse::default();
                }
                let Some(marked) = source.composition.clone() else {
                    return self.key(fonts, PhysicalKey::Backspace);
                };
                self.update_boundaries();
                let mut start = selection.0.min(selection.1).clamp(marked.start, marked.end);
                let end = selection.0.max(selection.1).clamp(marked.start, marked.end);
                if start == end {
                    start = self
                        .boundaries
                        .iter()
                        .copied()
                        .take_while(|b| *b < start)
                        .last()
                        .unwrap_or(start)
                        .max(marked.start);
                }
                if start == end {
                    return EditorResponse::handled(false);
                }
                let mut text = source.text[marked.clone()].to_owned();
                text.replace_range(start - marked.start..end - marked.start, "");
                self.native_input(
                    fonts,
                    revision,
                    &NativeTextAction::Compose {
                        range: None,
                        text,
                        cursor: Some((start - marked.start, start - marked.start)),
                    },
                )
            }
            NativeTextAction::Replace { range, text } => {
                if !valid(range) {
                    return EditorResponse::default();
                }
                EditorResponse::handled(self.replace_native_range(fonts, range.clone(), text))
            }
            NativeTextAction::Compose {
                range,
                text,
                cursor,
            } => {
                if range.as_ref().is_some_and(|r| !valid(r))
                    || cursor.is_some_and(|(a, b)| {
                        !text.is_char_boundary(a) || !text.is_char_boundary(b)
                    })
                {
                    return EditorResponse::default();
                }
                if text.is_empty() {
                    // Native UITextInput can keep a provisional empty marked
                    // range. Winit's empty-preedit cancellation remains separate.
                    if self.composition.is_none() {
                        self.commit_edit_group();
                        self.composition = Some(self.state());
                    }
                    let range = range.clone().or(source.composition).unwrap_or_else(|| {
                        source.selection.0.min(source.selection.1)
                            ..source.selection.0.max(source.selection.1)
                    });
                    let mut driver = fonts.editor_driver(&mut self.editor);
                    driver.finish_compose();
                    driver.select_byte_range(range.start, range.end);
                    driver.insert_or_replace_selection("");
                    driver.set_compose_byte_range(range.start, range.start);
                    self.snapshot = None;
                    self.boundaries_dirty = true;
                    self.invalidate_source();
                    return EditorResponse::handled(false);
                }
                if !text.is_empty() {
                    if self.composition.is_none() {
                        self.commit_edit_group();
                        self.composition = Some(self.state());
                    }
                    if let Some(range) = range {
                        let mut driver = fonts.editor_driver(&mut self.editor);
                        driver.finish_compose();
                        driver.select_byte_range(range.start, range.end);
                    }
                }
                self.input(
                    fonts,
                    &InputEvent::CompositionFor {
                        target: source.target,
                        text: text.clone(),
                        cursor: *cursor,
                    },
                )
            }
            NativeTextAction::CommitComposition => {
                EditorResponse::handled(self.finish_native_composition(fonts))
            }
            NativeTextAction::CancelComposition => {
                self.cancel_composition(fonts);
                EditorResponse::handled(false)
            }
        }
    }
}
