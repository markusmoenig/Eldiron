use super::*;
use unicode_segmentation::UnicodeSegmentation;

/// Host keyboard events, independent of native widget or window-system key enums.
pub enum GraphTextInput {
    Insert(String),
    Left { extend: bool },
    Right { extend: bool },
    Home { extend: bool },
    End { extend: bool },
    Backspace,
    Delete,
    SelectAll,
    Commit,
    Cancel,
}
/// Stable authoring addresses for identified list cells; legacy tables without
/// row IDs conservatively retain a structural guard until their next load.
#[cfg(feature = "text")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum GraphCellKey {
    Named(GraphId, String),
    Indexed(usize, usize),
}
#[derive(Clone, Debug)]
pub(super) struct LegacyCellGuard {
    row: usize,
    column: usize,
    columns: Vec<String>,
    rows: Vec<Vec<GraphControlValue>>,
    row_ids: Vec<GraphId>,
}
#[derive(Clone, Debug)]
pub(super) enum GraphCellIdentity {
    Named { row: GraphId, column: String },
    Indexed(std::sync::Arc<LegacyCellGuard>),
}
impl GraphCellIdentity {
    pub(super) fn capture(value: &GraphControlValue, row: usize, column: usize) -> Self {
        let GraphControlValue::List {
            columns,
            rows,
            row_ids,
        } = value
        else {
            unreachable!()
        };
        if let Some(id) = row_ids
            .get(row)
            .filter(|id| row_ids.iter().filter(|other| *other == *id).count() == 1)
            && columns
                .iter()
                .filter(|other| other.id == columns[column].id)
                .count()
                == 1
        {
            return Self::Named {
                row: *id,
                column: columns[column].id.clone(),
            };
        }
        Self::Indexed(std::sync::Arc::new(LegacyCellGuard {
            row,
            column,
            columns: columns.iter().map(|column| column.id.clone()).collect(),
            rows: rows.clone(),
            row_ids: row_ids.clone(),
        }))
    }
    #[cfg(feature = "text")]
    fn key(&self) -> GraphCellKey {
        match self {
            Self::Named { row, column } => GraphCellKey::Named(*row, column.clone()),
            Self::Indexed(guard) => GraphCellKey::Indexed(guard.row, guard.column),
        }
    }
    fn resolve(&self, value: &GraphControlValue) -> Option<(usize, usize)> {
        let GraphControlValue::List {
            columns,
            rows,
            row_ids,
        } = value
        else {
            return None;
        };
        match self {
            Self::Named { row, column } => {
                let mut matching_rows = row_ids.iter().enumerate().filter(|(_, id)| *id == row);
                let r = matching_rows.next()?.0;
                if matching_rows.next().is_some() {
                    return None;
                }
                let mut matching_columns =
                    columns.iter().enumerate().filter(|(_, c)| &c.id == column);
                let c = matching_columns.next()?.0;
                if matching_columns.next().is_some() {
                    return None;
                }
                rows.get(r)?.get(c)?;
                Some((r, c))
            }
            Self::Indexed(guard) => {
                if &guard.row_ids != row_ids
                    || guard.rows.len() != rows.len()
                    || !guard
                        .columns
                        .iter()
                        .eq(columns.iter().map(|column| &column.id))
                {
                    return None;
                }
                for (r, (before, after)) in guard.rows.iter().zip(rows).enumerate() {
                    if before.len() != after.len() {
                        return None;
                    }
                    for (c, (before, after)) in before.iter().zip(after).enumerate() {
                        if (r, c) != (guard.row, guard.column) && before != after {
                            return None;
                        }
                    }
                }
                rows.get(guard.row)?.get(guard.column)?;
                Some((guard.row, guard.column))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct GraphTextFocus {
    pub node: GraphId,
    pub row: GraphId,
    /// Last resolved `(row, column)` projection for a list field. Use
    /// `resolved_cell` after external source edits, before the next input/prepare.
    pub cell: Option<(usize, usize)>,
    pub(super) cell_identity: Option<GraphCellIdentity>,
    pub(crate) original: String,
    pub(crate) caret: usize,
    pub(crate) anchor: usize,
}
/// Mutable access to whichever text a focus points at: a plain Text row, or a
/// Text cell inside a List row.
pub(super) fn focused_text<'a>(
    doc: &'a mut GraphDocument,
    focus: &GraphTextFocus,
) -> Option<&'a mut String> {
    let row = doc
        .nodes
        .iter_mut()
        .find(|n| n.id == focus.node)?
        .rows
        .iter_mut()
        .find(|r| r.id == focus.row)?;
    let cell = focus.cell_in(&row.value);
    match (&mut row.value, cell) {
        (GraphControlValue::Text(value), None) if focus.cell.is_none() => Some(value),
        (GraphControlValue::List { rows, .. }, Some((r, c))) => {
            match rows.get_mut(r)?.get_mut(c)? {
                GraphControlValue::Text(value) => Some(value),
                _ => None,
            }
        }
        _ => None,
    }
}
impl GraphTextFocus {
    #[cfg(feature = "text")]
    pub(super) fn cell_key(&self) -> Option<GraphCellKey> {
        self.cell_identity.as_ref().map(GraphCellIdentity::key)
    }
    pub(super) fn cell_in(&self, value: &GraphControlValue) -> Option<(usize, usize)> {
        self.cell_identity
            .as_ref()
            .and_then(|identity| identity.resolve(value))
    }
    /// Resolve a list cell by authored row/column identity in the current source.
    /// None also covers plain text rows and invalidated/removed list cells.
    pub fn resolved_cell(&self, doc: &GraphDocument) -> Option<(usize, usize)> {
        let row = doc
            .nodes
            .iter()
            .find(|node| node.id == self.node)?
            .rows
            .iter()
            .find(|row| row.id == self.row)?;
        self.cell_in(&row.value)
    }
    /// The committed draft addressed by this graph focus, without preedit.
    pub fn value<'a>(&self, doc: &'a GraphDocument) -> Option<&'a str> {
        let row = doc
            .nodes
            .iter()
            .find(|n| n.id == self.node)?
            .rows
            .iter()
            .find(|r| r.id == self.row)?;
        match (&row.value, self.cell_in(&row.value)) {
            (GraphControlValue::Text(text), None) if self.cell.is_none() => Some(text),
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

impl GraphTextFocus {
    pub fn selection(&self) -> std::ops::Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }
    pub fn caret(&self) -> usize {
        self.caret
    }
    pub(crate) fn edit(&mut self, text: &mut String, input: GraphTextInput) {
        let previous = |at: usize| {
            text.grapheme_indices(true)
                .map(|(i, _)| i)
                .take_while(|i| *i < at)
                .last()
                .unwrap_or(0)
        };
        let next = |at: usize| {
            text.grapheme_indices(true)
                .map(|(i, _)| i)
                .find(|i| *i > at)
                .unwrap_or(text.len())
        };
        match input {
            GraphTextInput::SelectAll => {
                self.anchor = 0;
                self.caret = text.len();
            }
            GraphTextInput::Left { extend } => {
                self.caret = if !extend && !self.selection().is_empty() {
                    self.selection().start
                } else {
                    previous(self.caret)
                };
                if !extend {
                    self.anchor = self.caret;
                }
            }
            GraphTextInput::Right { extend } => {
                self.caret = if !extend && !self.selection().is_empty() {
                    self.selection().end
                } else {
                    next(self.caret)
                };
                if !extend {
                    self.anchor = self.caret;
                }
            }
            GraphTextInput::Home { extend } => {
                self.caret = 0;
                if !extend {
                    self.anchor = 0;
                }
            }
            GraphTextInput::End { extend } => {
                self.caret = text.len();
                if !extend {
                    self.anchor = self.caret;
                }
            }
            GraphTextInput::Insert(value) => {
                let value: String = value.chars().filter(|c| !c.is_control()).collect();
                let range = self.selection();
                self.caret = range.start + value.len();
                text.replace_range(range, &value);
                self.anchor = self.caret;
            }
            GraphTextInput::Backspace | GraphTextInput::Delete => {
                let mut range = self.selection();
                if range.is_empty() {
                    if matches!(input, GraphTextInput::Backspace) {
                        range.start = previous(self.caret);
                    } else {
                        range.end = next(self.caret);
                    }
                }
                self.caret = range.start;
                text.replace_range(range, "");
                self.anchor = self.caret;
            }
            _ => {}
        }
    }
}
impl GraphEditor {
    pub(super) fn begin_text_focus(&mut self, focus: GraphTextFocus) {
        #[cfg(feature = "text")]
        {
            self.text_epoch = next_text_epoch();
        }
        self.text_focus = Some(focus);
    }
    pub fn text_focus(&self) -> Option<&GraphTextFocus> {
        self.text_focus.as_ref()
    }
    /// Keep the transient index projection current without changing the draft's
    /// epoch. If its authored cell was removed/ambiguous, revoke the old target.
    pub(super) fn refresh_text_focus(&mut self, doc: &GraphDocument) -> bool {
        let Some(focus) = self.text_focus.as_mut() else {
            return false;
        };
        if focus.value(doc).is_none() {
            self.text_focus = None;
            #[cfg(feature = "text")]
            {
                self.text_epoch = next_text_epoch();
            }
            return false;
        }
        if focus.cell.is_some() {
            focus.cell = focus.resolved_cell(doc);
        }
        true
    }
    /// Commit a document transaction without ending the native editing session.
    /// Save/Play barriers use committed source text, leaving preedit and local
    /// selection/history with the view. The next edit starts at this baseline.
    pub fn checkpoint_text(&mut self, doc: &mut GraphDocument) {
        if !self.refresh_text_focus(doc) {
            return;
        }
        let Some(focus) = self.text_focus.clone() else {
            return;
        };
        let Some((before, after)) = focused_values(doc, &focus) else {
            return;
        };
        if before != after {
            self.record_edit(GraphEdit::SetValue {
                node: focus.node,
                row: focus.row,
                before,
                after,
            });
        }
        if let Some(value) = focus.value(doc) {
            self.text_focus.as_mut().unwrap().original = value.to_owned();
        }
    }
    pub fn finish_text(&mut self, doc: &mut GraphDocument, commit: bool) {
        let Some(focus) = self.text_focus.take() else {
            return;
        };
        #[cfg(feature = "text")]
        {
            self.text_epoch = next_text_epoch();
        }
        let Some((before, after)) = focused_values(doc, &focus) else {
            return;
        };
        if commit {
            if before != after {
                self.record_edit(GraphEdit::SetValue {
                    node: focus.node,
                    row: focus.row,
                    before,
                    after,
                });
            }
        } else if let Some(row) = doc
            .nodes
            .iter_mut()
            .find(|node| node.id == focus.node)
            .and_then(|node| node.rows.iter_mut().find(|row| row.id == focus.row))
        {
            row.value = before;
        }
    }
    /// Returns true when keyboard input was captured, including commit/cancel.
    pub fn text_input(&mut self, doc: &mut GraphDocument, input: GraphTextInput) -> bool {
        if !self.refresh_text_focus(doc) {
            return false;
        }
        match input {
            GraphTextInput::Commit => self.finish_text(doc, true),
            GraphTextInput::Cancel => self.finish_text(doc, false),
            _ => {
                let mut focus = self.text_focus.clone().unwrap();
                match focused_text(doc, &focus) {
                    Some(text) => {
                        focus.edit(text, input);
                        self.text_focus = Some(focus);
                    }
                    None => self.text_focus = None,
                }
            }
        }
        true
    }
}

// Distinct editor/documents may retain the same authored node/row IDs. Their
// native draft epochs must never alias when a host reuses its presentation port.
#[cfg(feature = "text")]
fn next_text_epoch() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn focused_values(
    doc: &GraphDocument,
    focus: &GraphTextFocus,
) -> Option<(GraphControlValue, GraphControlValue)> {
    let row = doc
        .nodes
        .iter()
        .find(|node| node.id == focus.node)?
        .rows
        .iter()
        .find(|row| row.id == focus.row)?;
    let after = row.value.clone();
    let mut before = after.clone();
    let cell = focus.cell_in(&before);
    match (&mut before, cell) {
        (GraphControlValue::Text(value), None) if focus.cell.is_none() => {
            *value = focus.original.clone()
        }
        (GraphControlValue::List { rows, .. }, Some((r, c))) => {
            match rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                Some(GraphControlValue::Text(value)) => *value = focus.original.clone(),
                _ => return None,
            }
        }
        _ => return None,
    }
    Some((before, after))
}
