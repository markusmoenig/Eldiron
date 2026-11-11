//! Neutral editing controller built on Parley's cursor, bidi, selection and IME
//! engine. It emits clipboard requests; no window, widgets, GPU or system access.
use super::{
    EditorPolicy, EditorTab, EditorUndo,
    policy::{self, EditPlan},
};
use crate::{
    paint::Rect,
    platform::{InputEvent, InputResponse, Modifiers, PhysicalKey},
    text::{TextLayout, TextOptions, TextSystem},
};
use parley::{PlainEditor, PlainEditorDriver, StyleProperty};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_CLIPBOARD_REQUEST: AtomicU64 = AtomicU64::new(1);
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, PartialEq, Eq)]
struct Snapshot {
    text: String,
    selection: parley::Selection,
}
struct Change {
    before: Snapshot,
    after: Snapshot,
    bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorClipboard {
    Write { text: String, cut: Option<u64> },
    Read { request: u64 },
}
pub use crate::platform::ClipboardResult;
struct PendingClipboard {
    request: u64,
    cut: bool,
    state: Snapshot,
}
/// Semantic operations shared by physical shortcuts and application menus.
/// Clipboard commands emit requests; they never access a platform clipboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorCommand {
    Undo,
    Redo,
    SelectAll,
    Copy,
    Cut,
    Paste,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorAction {
    Submit,
    Cancel,
    FocusNext { backwards: bool },
}
/// A shaped, extended-grapheme-safe drop position addressed to its editor edit
/// epoch. Geometry may refresh for fonts/layout; document/focus edits invalidate it.
#[derive(Clone, Copy, Debug)]
pub struct EditorHit {
    cursor: parley::Cursor,
    target: crate::platform::TextInputTarget,
    revision: u64,
}
impl EditorHit {
    pub fn index(self) -> usize {
        self.cursor.index()
    }
}
#[derive(Default)]
pub struct EditorResponse {
    pub input: InputResponse,
    pub changed: bool,
    pub clipboard: Option<EditorClipboard>,
    pub action: Option<EditorAction>,
}
impl EditorResponse {
    pub(super) fn handled(changed: bool) -> Self {
        Self {
            input: InputResponse {
                consumed: true,
                redraw: true,
            },
            changed,
            ..Self::default()
        }
    }
}
#[derive(PartialEq)]
struct LayoutKey {
    family: String,
    size: u32,
    width: Option<u32>,
    line_height: Option<u32>,
}
/// One editor owns committed content, transient composition, selection and a
/// bounded undo history. Layout and all hit/caret/IME geometry share Parley.
pub struct EditorController {
    input_target: crate::platform::TextInputTarget,
    edit_revision: u64,
    editor: PlainEditor<usize>,
    multiline: bool,
    policy: EditorPolicy,
    focused: bool,
    modifiers: Modifiers,
    composition: Option<Snapshot>,
    edit_group: Option<Snapshot>,
    undo: VecDeque<Change>,
    redo: Vec<Change>,
    history_bytes: usize,
    history_budget: usize,
    clipboard: Option<PendingClipboard>,
    style: Option<LayoutKey>,
    fonts: Option<(u64, u64)>,
    snapshot: Option<TextLayout>,
    snapshots_built: u64,
    boundaries: Vec<usize>,
    boundaries_dirty: bool,
}
impl EditorController {
    pub fn new(multiline: bool) -> Self {
        Self {
            input_target: crate::platform::TextInputTarget::new(),
            edit_revision: 0,
            editor: PlainEditor::new(14.),
            multiline,
            policy: EditorPolicy::default(),
            focused: false,
            modifiers: Modifiers::default(),
            composition: None,
            edit_group: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            history_bytes: 0,
            history_budget: 8 * 1024 * 1024,
            clipboard: None,
            style: None,
            fonts: None,
            snapshot: None,
            snapshots_built: 0,
            boundaries: vec![0],
            boundaries_dirty: false,
        }
    }
    /// Update the existing saved-state schema as a compatibility projection.
    /// Native history/bidi affinity stay in this controller; options on `state`
    /// remain owned by its containing document/widget.
    pub fn export_state(&self, state: &mut super::TextEditState) {
        state.set_text(self.text().to_owned());
        self.export_selection(state);
    }
    /// Update position in an already synchronized projection without copying
    /// the document text again for every caret/selection operation.
    pub fn export_selection(&self, state: &mut super::TextEditState) {
        let source = self.text();
        let selection = self
            .composition
            .as_ref()
            .map_or(self.editor.raw_selection(), |before| &before.selection);
        let legacy = |index: usize| {
            let index = char_boundary(source, index);
            let prefix = &source[..index];
            let row = prefix.bytes().filter(|byte| *byte == b'\n').count();
            let start = prefix.rfind('\n').map_or(0, |position| position + 1);
            let column = prefix[start..].graphemes(true).count();
            let global = state.rows[..row]
                .iter()
                .map(|line| line.graphemes(true).count() + 1)
                .sum::<usize>()
                + column;
            (super::Cursor::new(row, column), global)
        };
        let (_, anchor) = legacy(selection.anchor().index());
        let (cursor, focus) = legacy(selection.focus().index());
        state.cursor = cursor;
        state.selection = super::Selection {
            start: anchor.min(focus),
            end: anchor.max(focus),
        };
    }
    /// Install an externally supplied document/state without changing its JSON
    /// shape. This starts a new native callback epoch and undo history.
    pub fn import_state(&mut self, fonts: &mut TextSystem, state: &super::TextEditState) {
        let (source, anchor, focus) = self.legacy_position(state);
        self.set_text(fonts, &source);
        self.select_bytes(fonts, anchor, focus);
    }
    pub fn restore_selection_state(
        &mut self,
        fonts: &mut TextSystem,
        state: &super::TextEditState,
    ) -> bool {
        let (source, anchor, focus) = self.legacy_position(state);
        if if self.multiline {
            source != self.text()
        } else {
            self.clean(&source) != self.text()
        } {
            return false;
        }
        self.select_bytes(fonts, anchor, focus);
        true
    }
    fn legacy_position(&self, state: &super::TextEditState) -> (String, usize, usize) {
        let source = state.to_text();
        let byte_for_index = |mut index: usize| {
            let mut byte = 0;
            for (row, line) in state.rows.iter().enumerate() {
                let count = line.graphemes(true).count();
                if index <= count {
                    return byte
                        + line
                            .grapheme_indices(true)
                            .nth(index)
                            .map_or(line.len(), |(offset, _)| offset);
                }
                index = index.saturating_sub(count + 1);
                byte += line.len() + usize::from(row + 1 < state.rows.len());
            }
            source.len()
        };
        let cursor_row = state.cursor.row.min(state.rows.len().saturating_sub(1));
        let cursor_index = state.rows[..cursor_row]
            .iter()
            .map(|line| line.graphemes(true).count() + 1)
            .sum::<usize>()
            + state.rows.get(cursor_row).map_or(0, |line| {
                state.cursor.column.min(line.graphemes(true).count())
            });
        let (anchor, focus) = if state.selection.is_none() {
            let cursor = byte_for_index(cursor_index);
            (cursor, cursor)
        } else {
            let start = state.selection.start.min(state.selection.end);
            let end = state.selection.start.max(state.selection.end);
            if cursor_index == start {
                (byte_for_index(end), byte_for_index(start))
            } else {
                (byte_for_index(start), byte_for_index(end))
            }
        };
        let anchor = if self.multiline {
            anchor
        } else {
            self.clean(&source[..anchor]).len()
        };
        let focus = if self.multiline {
            focus
        } else {
            self.clean(&source[..focus]).len()
        };
        (source, anchor, focus)
    }
    /// Translate stable document annotations into transient displayed text.
    /// Marks intersecting an IME replacement are withheld until it commits.
    pub fn displayed_range(&self, range: std::ops::Range<usize>) -> Option<std::ops::Range<usize>> {
        let source = self.text();
        if range.start > range.end
            || range.end > source.len()
            || !source.is_char_boundary(range.start)
            || !source.is_char_boundary(range.end)
        {
            return None;
        }
        let Some(before) = &self.composition else {
            return Some(range);
        };
        let selection = before.selection;
        let start = selection.anchor().index().min(selection.focus().index());
        let end = selection.anchor().index().max(selection.focus().index());
        if range.end <= start {
            return Some(range);
        }
        let inserted = self
            .editor
            .raw_text()
            .len()
            .checked_sub(before.text.len() - (end - start))?;
        if range.start >= end {
            return Some(
                (start + inserted + range.start - end)..(start + inserted + range.end - end),
            );
        }
        None
    }
    pub fn policy(&self) -> EditorPolicy {
        self.policy
    }
    pub fn accepts_text_input(&self) -> bool {
        self.focused && !self.policy.read_only
    }
    pub fn set_policy(&mut self, fonts: &mut TextSystem, policy: EditorPolicy) {
        if self.policy.read_only != policy.read_only {
            self.cancel_edit_group(fonts);
            self.cancel_composition(fonts);
            self.clipboard = None;
            self.input_target = crate::platform::TextInputTarget::new();
        }
        if self.policy.undo != policy.undo {
            self.undo.clear();
            self.redo.clear();
            self.history_bytes = 0;
        }
        self.policy = policy;
    }
    pub fn input_target(&self) -> crate::platform::TextInputTarget {
        self.input_target
    }
    pub fn text(&self) -> &str {
        self.composition
            .as_ref()
            .map_or(self.editor.raw_text(), |s| &s.text)
    }
    pub fn is_composing(&self) -> bool {
        self.composition.is_some()
    }
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    pub fn snapshots_built(&self) -> u64 {
        self.snapshots_built
    }
    pub fn history_bytes(&self) -> usize {
        self.history_bytes
    }
    fn state(&self) -> Snapshot {
        let selection = self.editor.raw_selection();
        Snapshot {
            text: self.editor.raw_text().into(),
            selection: *selection,
        }
    }
    fn restore(&mut self, fonts: &mut TextSystem, state: &Snapshot) {
        self.edit_revision = self.edit_revision.wrapping_add(1);

        fonts.editor_driver(&mut self.editor).clear_compose();
        self.editor.set_text(&state.text);
        fonts
            .editor_driver(&mut self.editor)
            .set_selection(state.selection);
        self.snapshot = None;
        self.boundaries_dirty = true;
    }
    fn cancel_composition(&mut self, fonts: &mut TextSystem) -> bool {
        if let Some(before) = self.composition.take() {
            self.restore(fonts, &before);
            true
        } else {
            false
        }
    }
    pub fn set_focused(&mut self, fonts: &mut TextSystem, focused: bool) {
        if self.focused && !focused {
            self.input_target = crate::platform::TextInputTarget::new();
        }
        self.focused = focused;
        if !focused {
            self.cancel_edit_group(fonts);
            self.cancel_composition(fonts);
            self.clipboard = None;
            self.modifiers = Modifiers::default();
        }
    }
    fn clean(&self, value: &str) -> String {
        value
            .chars()
            .filter(|c| {
                !c.is_control()
                    || self.multiline
                        && !self.policy.single_line_input
                        && matches!(c, '\n' | '\r' | '\t')
            })
            .collect()
    }
    pub fn set_text(&mut self, fonts: &mut TextSystem, text: &str) {
        self.edit_revision = self.edit_revision.wrapping_add(1);

        self.input_target = crate::platform::TextInputTarget::new();
        self.edit_group = None;
        self.cancel_composition(fonts);
        // Loaded multiline documents retain their exact UTF-8 contents; input
        // filtering belongs to typed/pasted text, not a model/file replacement.
        if self.multiline {
            self.editor.set_text(text);
        } else {
            self.editor.set_text(&self.clean(text));
        }
        fonts.editor_driver(&mut self.editor).move_to_text_end();
        self.undo = VecDeque::new();
        self.redo = Vec::new();
        self.history_bytes = 0;
        self.clipboard = None;
        self.snapshot = None;
        self.boundaries_dirty = true;
    }
    fn ensure_fonts(&mut self, fonts: &mut TextSystem) {
        let stamp = fonts.font_stamp();
        if self.fonts != Some(stamp) {
            self.fonts = Some(stamp);
            let _ = self.editor.edit_styles();
            self.snapshot = None;
            self.boundaries_dirty = true;
        }
    }
    pub fn prepare(&mut self, fonts: &mut TextSystem, options: TextOptions<'_>) {
        self.ensure_fonts(fonts);
        let size = if options.size.is_finite() && options.size > 0. {
            options.size
        } else {
            14.
        };
        let width = self
            .multiline
            .then_some(options.width)
            .flatten()
            .filter(|w| w.is_finite())
            .map(|w| w.max(0.));
        let key = LayoutKey {
            family: options.family.into(),
            size: size.to_bits(),
            width: width.map(f32::to_bits),
            line_height: options
                .line_height
                .filter(|h| h.is_finite() && *h > 0.)
                .map(f32::to_bits),
        };
        if self.style.as_ref() != Some(&key) {
            let styles = self.editor.edit_styles();
            styles.insert(StyleProperty::FontFamily(
                parley::FontFamily::from(options.family).into_owned(),
            ));
            styles.insert(StyleProperty::FontSize(size));
            styles.insert(StyleProperty::LineHeight(
                options
                    .line_height
                    .filter(|h| h.is_finite() && *h > 0.)
                    .map_or(
                        parley::LineHeight::FontSizeRelative(1.2),
                        parley::LineHeight::Absolute,
                    ),
            ));
            self.editor.set_width(width);
            self.style = Some(key);
            self.snapshot = None;
            self.boundaries_dirty = true;
        }
        let _ = self.layout(fonts);
    }
    pub fn layout(&mut self, fonts: &mut TextSystem) -> &TextLayout {
        self.ensure_fonts(fonts);
        fonts.editor_driver(&mut self.editor).layout();
        if !self.is_composing() {
            self.snap_selection(fonts, false);
        }
        if self
            .snapshot
            .as_ref()
            .is_none_or(|layout| layout.source.as_ref() != self.editor.raw_text())
        {
            self.snapshot = Some(fonts.editor_snapshot(&mut self.editor));
            self.snapshots_built += 1;
        }
        self.snapshot.as_ref().unwrap()
    }
    fn update_boundaries(&mut self) {
        if self.boundaries_dirty {
            self.boundaries.clear();
            self.boundaries.extend(
                self.editor
                    .raw_text()
                    .grapheme_indices(true)
                    .map(|(i, _)| i),
            );
            self.boundaries.push(self.editor.raw_text().len());
            self.boundaries_dirty = false;
        }
    }
    fn snap_selection(&mut self, fonts: &mut TextSystem, forward: bool) {
        self.update_boundaries();
        normalize_selection(
            &mut fonts.editor_driver(&mut self.editor),
            &self.boundaries,
            forward,
        );
    }
    pub fn selection_bytes(&self) -> (usize, usize) {
        let selection = self.editor.raw_selection();
        (selection.anchor().index(), selection.focus().index())
    }
    /// Selection addressing committed source bytes, excluding transient preedit.
    pub fn committed_selection_bytes(&self) -> (usize, usize) {
        let selection = self
            .composition
            .as_ref()
            .map_or(self.editor.raw_selection(), |before| &before.selection);
        (selection.anchor().index(), selection.focus().index())
    }
    pub fn select_bytes(&mut self, fonts: &mut TextSystem, anchor: usize, focus: usize) {
        self.cancel_composition(fonts);
        self.ensure_fonts(fonts);
        fonts
            .editor_driver(&mut self.editor)
            .select_byte_range(anchor, focus);
        self.snap_selection(fonts, false);
    }
    pub fn hit_is_current(&self, hit: EditorHit) -> bool {
        hit.target == self.input_target
            && hit.revision == self.edit_revision
            && !self.is_composing()
    }
    pub fn hit_test(&mut self, fonts: &mut TextSystem, point: [f32; 2]) -> Option<EditorHit> {
        if self.is_composing() || !point.iter().all(|v| v.is_finite()) {
            return None;
        }
        self.ensure_fonts(fonts);
        self.update_boundaries();
        let mut driver = fonts.editor_driver(&mut self.editor);
        let layout = driver.layout();
        let cursor = grapheme_cursor(
            parley::Cursor::from_point(layout, point[0], point[1]),
            layout,
            &self.boundaries,
            false,
        );
        Some(EditorHit {
            cursor,
            target: self.input_target,
            revision: self.edit_revision,
        })
    }
    pub fn hit_geometry(&mut self, fonts: &mut TextSystem, hit: EditorHit) -> Option<Rect> {
        if !self.hit_is_current(hit) {
            return None;
        }
        self.ensure_fonts(fonts);
        let mut driver = fonts.editor_driver(&mut self.editor);
        Some(box_rect(hit.cursor.geometry(driver.layout(), 1.)))
    }
    /// Move the current selection directly in the document as one transaction;
    /// no system clipboard or synthetic cut/paste operations participate.
    pub fn move_selection_to(&mut self, fonts: &mut TextSystem, hit: EditorHit) -> bool {
        if self.policy.read_only || !self.hit_is_current(hit) {
            return false;
        }
        let (anchor, focus) = self.selection_bytes();
        let start = anchor.min(focus);
        let end = anchor.max(focus);
        let target = hit.index();
        if start == end || (start..=end).contains(&target) {
            return false;
        }
        let text = self.editor.raw_text()[start..end].to_owned();
        let (edits, position) = if target < start {
            (
                vec![
                    policy::Edit {
                        range: target..target,
                        text,
                    },
                    policy::Edit {
                        range: start..end,
                        text: String::new(),
                    },
                ],
                target,
            )
        } else {
            (
                vec![
                    policy::Edit {
                        range: start..end,
                        text: String::new(),
                    },
                    policy::Edit {
                        range: target..target,
                        text,
                    },
                ],
                target - (end - start),
            )
        };
        let selection = if anchor > focus {
            (position + end - start, position)
        } else {
            (position, position + end - start)
        };
        self.apply_edit_plan(fonts, EditPlan { edits, selection })
    }
    pub fn point(&mut self, fonts: &mut TextSystem, point: [f32; 2], extend: bool) {
        self.cancel_composition(fonts);
        self.ensure_fonts(fonts);
        let mut driver = fonts.editor_driver(&mut self.editor);
        if extend {
            driver.extend_selection_to_point(point[0], point[1]);
        } else {
            driver.move_to_point(point[0], point[1]);
        }
        self.snap_selection(fonts, false);
    }
    /// Multiple-click selection uses the same shaped hit geometry as painting.
    pub fn point_selection(&mut self, fonts: &mut TextSystem, point: [f32; 2], clicks: u8) {
        self.cancel_composition(fonts);
        self.ensure_fonts(fonts);
        let mut driver = fonts.editor_driver(&mut self.editor);
        match clicks {
            0 | 1 => driver.move_to_point(point[0], point[1]),
            2 => driver.select_word_at_point(point[0], point[1]),
            _ => driver.select_hard_line_at_point(point[0], point[1]),
        }
        self.snap_selection(fonts, false);
    }
    pub fn move_page(&mut self, fonts: &mut TextSystem, height: f32, down: bool, extend: bool) {
        self.cancel_composition(fonts);
        let line = self.caret(fonts).map_or(14., |r| r.height).max(1.);
        let count = (height / line).floor().clamp(1., 10_000.) as usize;
        let mut driver = fonts.editor_driver(&mut self.editor);
        for _ in 0..count {
            match (down, extend) {
                (true, false) => driver.move_down(),
                (false, false) => driver.move_up(),
                (true, true) => driver.select_down(),
                (false, true) => driver.select_up(),
            }
        }
        self.snap_selection(fonts, false);
    }
    pub fn caret(&mut self, fonts: &mut TextSystem) -> Option<Rect> {
        self.layout(fonts);
        self.editor.cursor_geometry(1.).map(box_rect)
    }
    pub fn selection(&mut self, fonts: &mut TextSystem) -> Vec<Rect> {
        self.layout(fonts);
        self.editor
            .selection_geometry()
            .into_iter()
            .map(|(bounds, _)| box_rect(bounds))
            .collect()
    }
    pub fn composition_geometry(&mut self, fonts: &mut TextSystem) -> Vec<Rect> {
        self.layout(fonts);
        let Some(range) = self.editor.raw_compose().clone() else {
            return Vec::new();
        };
        self.snapshot
            .as_ref()
            .unwrap()
            .selection(range.start, range.end)
    }
    pub fn ime_area(&mut self, fonts: &mut TextSystem) -> Rect {
        self.layout(fonts);
        box_rect(self.editor.ime_cursor_area())
    }
    fn transaction(
        &mut self,
        fonts: &mut TextSystem,
        edit: impl FnOnce(&mut PlainEditorDriver<'_, usize>),
    ) -> bool {
        if self.policy.read_only {
            return false;
        }
        self.cancel_composition(fonts);
        self.ensure_fonts(fonts);
        let before = self.state();
        let mut driver = fonts.editor_driver(&mut self.editor);
        driver.refresh_layout();
        edit(&mut driver);
        if before.text == self.editor.raw_text() {
            return false;
        }
        self.edit_revision = self.edit_revision.wrapping_add(1);
        self.snapshot = None;
        self.boundaries_dirty = true;
        self.snap_selection(fonts, true);
        if self.edit_group.is_none() {
            self.record_change(before, self.state());
        }
        self.clipboard = None;
        true
    }
    fn record_change(&mut self, before: Snapshot, after: Snapshot) {
        if self.policy.undo == EditorUndo::External {
            self.clipboard = None;
            return;
        }
        self.history_bytes -= self.redo.iter().map(|c| c.bytes).sum::<usize>();
        self.redo.clear();
        let bytes = before.text.capacity() + after.text.capacity() + std::mem::size_of::<Change>();
        self.undo.push_back(Change {
            before,
            after,
            bytes,
        });
        self.history_bytes += bytes;
        while self.history_bytes > self.history_budget {
            if let Some(change) = self.undo.pop_front() {
                self.history_bytes -= change.bytes;
            } else {
                break;
            }
        }
        self.clipboard = None;
    }
    /// Group a continuous user gesture into one bounded undo record. A blur
    /// cancels an unfinished gesture; external document replacement discards it.
    pub fn begin_edit_group(&mut self, fonts: &mut TextSystem) -> bool {
        if self.policy.read_only || self.edit_group.is_some() {
            return false;
        }
        self.cancel_composition(fonts);
        self.ensure_fonts(fonts);
        self.edit_group = Some(self.state());
        self.clipboard = None;
        true
    }
    pub fn commit_edit_group(&mut self) -> bool {
        let Some(before) = self.edit_group.take() else {
            return false;
        };
        let after = self.state();
        if before.text == after.text {
            return false;
        }
        self.record_change(before, after);
        true
    }
    pub fn cancel_edit_group(&mut self, fonts: &mut TextSystem) -> bool {
        let Some(before) = self.edit_group.take() else {
            return false;
        };
        let changed = before.text != self.text();
        self.restore(fonts, &before);
        self.clipboard = None;
        changed
    }
    pub fn undo(&mut self, fonts: &mut TextSystem) -> bool {
        if self.policy.read_only || self.policy.undo == EditorUndo::External {
            return false;
        }
        self.commit_edit_group();
        self.cancel_composition(fonts);
        let Some(change) = self.undo.pop_back() else {
            return false;
        };
        self.restore(fonts, &change.before);
        self.redo.push(change);
        self.clipboard = None;
        true
    }
    pub fn redo(&mut self, fonts: &mut TextSystem) -> bool {
        if self.policy.read_only || self.policy.undo == EditorUndo::External {
            return false;
        }
        self.commit_edit_group();
        self.cancel_composition(fonts);
        let Some(change) = self.redo.pop() else {
            return false;
        };
        self.restore(fonts, &change.after);
        self.undo.push_back(change);
        self.clipboard = None;
        true
    }
    pub fn complete_clipboard(
        &mut self,
        fonts: &mut TextSystem,
        request: u64,
        result: ClipboardResult,
    ) -> EditorResponse {
        if self.policy.read_only {
            self.clipboard = None;
            return EditorResponse::default();
        }
        let Some(pending) = self.clipboard.take() else {
            return EditorResponse::default();
        };
        if pending.request != request {
            self.clipboard = Some(pending);
            return EditorResponse::default();
        }
        if !self.focused || self.composition.is_some() || pending.state != self.state() {
            return EditorResponse::default();
        }
        let changed = match result {
            ClipboardResult::Written(true) if pending.cut => {
                self.transaction(fonts, |driver| driver.delete_selection())
            }
            ClipboardResult::Text(Some(text)) if !pending.cut => {
                let text = self.clean(&text);
                !text.is_empty()
                    && self.transaction(fonts, |driver| driver.insert_or_replace_selection(&text))
            }
            _ => false,
        };
        EditorResponse::handled(changed)
    }
    pub fn input(&mut self, fonts: &mut TextSystem, event: &InputEvent) -> EditorResponse {
        match event {
            InputEvent::Modifiers(modifiers) => {
                self.modifiers = *modifiers;
                return EditorResponse::default();
            }
            InputEvent::Focus(false) => {
                let was = self.focused;
                self.set_focused(fonts, false);
                return if was {
                    EditorResponse::handled(false)
                } else {
                    EditorResponse::default()
                };
            }
            _ => {}
        }
        if !self.focused
            || event
                .text_target()
                .is_some_and(|target| target != self.input_target)
        {
            return EditorResponse::default();
        }
        self.ensure_fonts(fonts);
        if self.policy.read_only
            && matches!(
                event,
                InputEvent::Text(_)
                    | InputEvent::TextFor { .. }
                    | InputEvent::Composition { .. }
                    | InputEvent::CompositionFor { .. }
            )
        {
            return EditorResponse::handled(false);
        }
        match event {
            InputEvent::Clipboard { request, result } => {
                self.complete_clipboard(fonts, *request, result.clone())
            }
            InputEvent::Text(text) | InputEvent::TextFor { text, .. } => {
                let text = self.clean(text);
                let composing = self.is_composing();
                let cancelled = self.cancel_composition(fonts);
                if text.is_empty() {
                    return if cancelled {
                        EditorResponse::handled(false)
                    } else {
                        EditorResponse::default()
                    };
                }
                if self.policy.auto_brackets && !composing && text.chars().count() == 1 {
                    if let Some(plan) =
                        policy::brackets(self.selection_bytes(), text.chars().next().unwrap())
                    {
                        return EditorResponse::handled(self.apply_edit_plan(fonts, plan));
                    }
                }
                EditorResponse::handled(self.replace_selection(fonts, &text))
            }
            InputEvent::Composition { text, cursor }
            | InputEvent::CompositionFor { text, cursor, .. } => {
                if text.is_empty() {
                    self.cancel_composition(fonts);
                    return EditorResponse::handled(false);
                }
                if self.composition.is_none() {
                    self.commit_edit_group();
                    self.composition = Some(self.state());
                }
                // Platform cursor indices address the original preedit bytes.
                // Filtering controls in a single-line field also remaps them.
                let cursor = cursor.map(|(a, b)| {
                    (
                        self.clean(&text[..char_boundary(text, a)]).len(),
                        self.clean(&text[..char_boundary(text, b)]).len(),
                    )
                });
                let text = self.clean(text);
                if text.is_empty() {
                    self.cancel_composition(fonts);
                    return EditorResponse::handled(false);
                }
                fonts
                    .editor_driver(&mut self.editor)
                    .set_compose(&text, cursor);
                self.snapshot = None;
                self.boundaries_dirty = true;
                EditorResponse::handled(false)
            }
            InputEvent::Key {
                physical, pressed, ..
            } => {
                if !pressed {
                    return EditorResponse {
                        input: InputResponse {
                            consumed: true,
                            redraw: false,
                        },
                        ..EditorResponse::default()
                    };
                }
                self.key(fonts, *physical)
            }
            _ => EditorResponse::default(),
        }
    }
    fn apply_edit_plan(&mut self, fonts: &mut TextSystem, plan: EditPlan) -> bool {
        self.transaction(fonts, |driver| {
            for edit in plan.edits.iter().rev() {
                driver.select_byte_range(edit.range.start, edit.range.end);
                driver.insert_or_replace_selection(&edit.text);
            }
            driver.select_byte_range(plan.selection.0, plan.selection.1);
        })
    }
    pub fn toggle_line_comment(&mut self, fonts: &mut TextSystem, prefix: &str) -> bool {
        if self.policy.read_only {
            return false;
        }
        self.cancel_composition(fonts);
        policy::comment(self.editor.raw_text(), self.selection_bytes(), prefix)
            .is_some_and(|plan| self.apply_edit_plan(fonts, plan))
    }
    /// Explicit user insertion such as paste: preserve undo and skip automatic
    /// typing-only bracket completion. External set_text remains a new document.
    pub fn replace_selection(&mut self, fonts: &mut TextSystem, text: &str) -> bool {
        let text = self.clean(text);
        self.transaction(fonts, |driver| driver.insert_or_replace_selection(&text))
    }
    /// An in-document user replacement (for example a numeric drag) participates
    /// in the same undo history without replacing the document/input identity.
    pub fn replace_text(&mut self, fonts: &mut TextSystem, text: &str) -> bool {
        let text = self.clean(text);
        self.transaction(fonts, |driver| {
            driver.select_all();
            driver.insert_or_replace_selection(&text);
        })
    }
    pub fn execute(&mut self, fonts: &mut TextSystem, command: EditorCommand) -> EditorResponse {
        if !self.focused {
            return EditorResponse::default();
        }
        if self.policy.undo == EditorUndo::External
            && matches!(command, EditorCommand::Undo | EditorCommand::Redo)
        {
            return EditorResponse::default();
        }
        if self.policy.read_only
            && !matches!(command, EditorCommand::Copy | EditorCommand::SelectAll)
        {
            return EditorResponse::handled(false);
        }
        self.cancel_composition(fonts);
        match command {
            EditorCommand::Undo => EditorResponse::handled(self.undo(fonts)),
            EditorCommand::Redo => EditorResponse::handled(self.redo(fonts)),
            EditorCommand::SelectAll => {
                fonts.editor_driver(&mut self.editor).select_all();
                EditorResponse::handled(false)
            }
            EditorCommand::Copy | EditorCommand::Cut => {
                // A new copy supersedes older cuts/pastes, including when the
                // current selection is empty. Their late completions cannot edit.
                self.clipboard = None;
                let Some(text) = self.editor.selected_text().map(str::to_owned) else {
                    return EditorResponse::handled(false);
                };
                let cut = if command == EditorCommand::Cut {
                    let request = NEXT_CLIPBOARD_REQUEST.fetch_add(1, Ordering::Relaxed);
                    self.clipboard = Some(PendingClipboard {
                        request,
                        cut: true,
                        state: self.state(),
                    });
                    Some(request)
                } else {
                    None
                };
                EditorResponse {
                    clipboard: Some(EditorClipboard::Write { text, cut }),
                    ..EditorResponse::handled(false)
                }
            }
            EditorCommand::Paste => {
                let request = NEXT_CLIPBOARD_REQUEST.fetch_add(1, Ordering::Relaxed);
                self.clipboard = Some(PendingClipboard {
                    request,
                    cut: false,
                    state: self.state(),
                });
                EditorResponse {
                    clipboard: Some(EditorClipboard::Read { request }),
                    ..EditorResponse::handled(false)
                }
            }
        }
    }
    fn key(&mut self, fonts: &mut TextSystem, key: PhysicalKey) -> EditorResponse {
        let mods = self.modifiers;
        let shortcut = (mods.control && !mods.alt) || mods.super_key;
        if let PhysicalKey::Letter(letter) = key {
            if shortcut {
                let command = match letter.to_ascii_lowercase() {
                    b'z' => Some(if mods.shift {
                        EditorCommand::Redo
                    } else {
                        EditorCommand::Undo
                    }),
                    b'y' => Some(EditorCommand::Redo),
                    b'a' => Some(EditorCommand::SelectAll),
                    b'c' => Some(EditorCommand::Copy),
                    b'x' => Some(EditorCommand::Cut),
                    b'v' => Some(EditorCommand::Paste),
                    _ => None,
                };
                if let Some(command) = command {
                    return self.execute(fonts, command);
                }
                // Application accelerators such as Save keep their own owner.
                // The following committed text event remains an independent edit.
                return EditorResponse::default();
            }
        }
        if key == PhysicalKey::Escape {
            if self.cancel_composition(fonts) {
                return EditorResponse::handled(false);
            }
            return EditorResponse {
                action: Some(EditorAction::Cancel),
                ..EditorResponse::handled(false)
            };
        }
        if self.is_composing() {
            return EditorResponse::handled(false);
        }
        if self.policy.read_only
            && matches!(
                key,
                PhysicalKey::Enter | PhysicalKey::Backspace | PhysicalKey::Delete
            )
        {
            return EditorResponse::handled(false);
        }
        if key == PhysicalKey::Enter {
            if self.multiline && !self.policy.single_line_input {
                let plan = policy::newline(
                    self.editor.raw_text(),
                    self.selection_bytes(),
                    self.policy.auto_indent,
                );
                return EditorResponse::handled(self.apply_edit_plan(fonts, plan));
            }
            return EditorResponse {
                action: Some(EditorAction::Submit),
                ..EditorResponse::handled(false)
            };
        }
        if key == PhysicalKey::Tab {
            if self.multiline && !self.policy.read_only {
                if let EditorTab::Spaces(width) = self.policy.tab {
                    let plan = policy::indent(
                        self.editor.raw_text(),
                        self.selection_bytes(),
                        width,
                        mods.shift,
                    );
                    return EditorResponse::handled(
                        plan.is_some_and(|plan| self.apply_edit_plan(fonts, plan)),
                    );
                }
            }
            return EditorResponse {
                action: Some(EditorAction::FocusNext {
                    backwards: mods.shift,
                }),
                ..EditorResponse::handled(false)
            };
        }
        match key {
            PhysicalKey::Backspace => EditorResponse::handled(self.transaction(fonts, |driver| {
                if mods.control || mods.alt {
                    driver.backdelete_word();
                } else if driver.editor.raw_selection().is_collapsed() {
                    let focus = driver.editor.raw_selection().focus().index();
                    if let Some((start, _)) = driver.editor.raw_text()[..focus]
                        .grapheme_indices(true)
                        .next_back()
                    {
                        if let Some(len) = std::num::NonZeroUsize::new(focus - start) {
                            driver.delete_bytes_before_selection(len);
                        }
                    }
                } else {
                    driver.delete_selection();
                }
            })),
            PhysicalKey::Delete => EditorResponse::handled(self.transaction(fonts, |driver| {
                if mods.control || mods.alt {
                    driver.delete_word();
                } else if driver.editor.raw_selection().is_collapsed() {
                    let focus = driver.editor.raw_selection().focus().index();
                    if let Some(cluster) = driver.editor.raw_text()[focus..].graphemes(true).next()
                    {
                        let end = focus + cluster.len();
                        driver.select_byte_range(focus, end);
                        driver.delete_selection();
                    }
                } else {
                    driver.delete_selection();
                }
            })),
            PhysicalKey::Left
            | PhysicalKey::Right
            | PhysicalKey::Up
            | PhysicalKey::Down
            | PhysicalKey::Home
            | PhysicalKey::End => {
                self.update_boundaries();
                let mut driver = fonts.editor_driver(&mut self.editor);
                match (key, mods.shift) {
                    (PhysicalKey::Left, false) if mods.super_key => driver.move_to_line_start(),
                    (PhysicalKey::Right, false) if mods.super_key => driver.move_to_line_end(),
                    (PhysicalKey::Left, true) if mods.super_key => driver.select_to_line_start(),
                    (PhysicalKey::Right, true) if mods.super_key => driver.select_to_line_end(),
                    (PhysicalKey::Left, false) if mods.control || mods.alt => {
                        driver.move_word_left()
                    }
                    (PhysicalKey::Right, false) if mods.control || mods.alt => {
                        driver.move_word_right()
                    }
                    (PhysicalKey::Left, true) if mods.control || mods.alt => {
                        driver.select_word_left()
                    }
                    (PhysicalKey::Right, true) if mods.control || mods.alt => {
                        driver.select_word_right()
                    }
                    (PhysicalKey::Left, false) => {
                        visual_grapheme(&mut driver, &self.boundaries, false, false)
                    }
                    (PhysicalKey::Right, false) => {
                        visual_grapheme(&mut driver, &self.boundaries, true, false)
                    }
                    (PhysicalKey::Left, true) => {
                        visual_grapheme(&mut driver, &self.boundaries, false, true)
                    }
                    (PhysicalKey::Right, true) => {
                        visual_grapheme(&mut driver, &self.boundaries, true, true)
                    }
                    (PhysicalKey::Up, false) => driver.move_up(),
                    (PhysicalKey::Down, false) => driver.move_down(),
                    (PhysicalKey::Up, true) => driver.select_up(),
                    (PhysicalKey::Down, true) => driver.select_down(),
                    (PhysicalKey::Home, false) if shortcut => driver.move_to_text_start(),
                    (PhysicalKey::End, false) if shortcut => driver.move_to_text_end(),
                    (PhysicalKey::Home, true) if shortcut => driver.select_to_text_start(),
                    (PhysicalKey::End, true) if shortcut => driver.select_to_text_end(),
                    (PhysicalKey::Home, false) => driver.move_to_line_start(),
                    (PhysicalKey::End, false) => driver.move_to_line_end(),
                    (PhysicalKey::Home, true) => driver.select_to_line_start(),
                    (PhysicalKey::End, true) => driver.select_to_line_end(),
                    _ => {}
                }
                normalize_selection(&mut driver, &self.boundaries, false);
                EditorResponse::handled(false)
            }
            _ => EditorResponse {
                input: InputResponse {
                    consumed: true,
                    redraw: false,
                },
                ..EditorResponse::default()
            },
        }
    }
}
/// Parley visual clusters can subdivide one extended grapheme. Keep its visual
/// bidi ordering and affinity while traversing those internal shaping stops.
fn visual_grapheme(
    driver: &mut PlainEditorDriver<'_, usize>,
    boundaries: &[usize],
    right: bool,
    extend: bool,
) {
    let mut selection = *driver.editor.raw_selection();
    let limit = driver
        .editor
        .raw_text()
        .len()
        .saturating_mul(2)
        .saturating_add(1);
    let layout = driver.layout();
    for _ in 0..limit {
        let next = if right {
            selection.next_visual(layout, extend)
        } else {
            selection.previous_visual(layout, extend)
        };
        if next.focus() == selection.focus() {
            break;
        }
        selection = next;
        if boundaries.binary_search(&selection.focus().index()).is_ok() {
            break;
        }
    }
    driver.set_selection(selection);
}
fn grapheme_cursor(
    cursor: parley::Cursor,
    layout: &parley::Layout<usize>,
    boundaries: &[usize],
    forward: bool,
) -> parley::Cursor {
    let Err(index) = boundaries.binary_search(&cursor.index()) else {
        return cursor;
    };
    let before = boundaries[index.saturating_sub(1)];
    let after = boundaries[index.min(boundaries.len() - 1)];
    let left = parley::Cursor::from_byte_index(layout, before, parley::Affinity::Downstream);
    let right = parley::Cursor::from_byte_index(layout, after, parley::Affinity::Upstream);
    if forward {
        return right;
    }
    let origin = cursor.geometry(layout, 0.);
    let distance = |candidate: parley::Cursor| {
        let rect = candidate.geometry(layout, 0.);
        (rect.x0 - origin.x0).powi(2) + (rect.y0 - origin.y0).powi(2)
    };
    if distance(left) <= distance(right) {
        left
    } else {
        right
    }
}
fn normalize_selection(
    driver: &mut PlainEditorDriver<'_, usize>,
    boundaries: &[usize],
    forward: bool,
) {
    let selection = *driver.editor.raw_selection();
    let layout = driver.layout();
    let mut changed = false;
    let corrected = selection.map_cursors(|cursor| {
        let snapped = grapheme_cursor(cursor, layout, boundaries, forward);
        changed |= snapped != cursor;
        snapped
    });
    if changed {
        driver.set_selection(corrected);
    }
}
fn char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}
fn box_rect(bounds: parley::BoundingBox) -> Rect {
    Rect::new(
        bounds.x0 as f32,
        bounds.y0 as f32,
        (bounds.x1 - bounds.x0) as f32,
        (bounds.y1 - bounds.y0) as f32,
    )
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
