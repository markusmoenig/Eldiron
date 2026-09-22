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
#[derive(Clone, Debug)]
pub struct GraphTextFocus {
    pub node: GraphId,
    pub row: GraphId,
    /// For a list row, the `(row, column)` cell being edited.
    pub cell: Option<(usize, usize)>,
    pub(crate) original: String,
    pub(crate) caret: usize,
    pub(crate) anchor: usize,
}
/// Mutable access to whichever text a focus points at: a plain Text row, or a
/// Text cell inside a List row.
fn focused_text<'a>(
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
    match (&mut row.value, focus.cell) {
        (GraphControlValue::Text(value), None) => Some(value),
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
    pub fn text_focus(&self) -> Option<&GraphTextFocus> {
        self.text_focus.as_ref()
    }
    pub fn finish_text(&mut self, doc: &mut GraphDocument, commit: bool) {
        let Some(focus) = self.text_focus.take() else {
            return;
        };
        let Some(row) = doc
            .nodes
            .iter_mut()
            .find(|n| n.id == focus.node)
            .and_then(|n| n.rows.iter_mut().find(|r| r.id == focus.row))
        else {
            return;
        };
        // Editing mutates the live value, so `before` rewinds the edited text to
        // its old content; undo then restores the whole row (list included).
        let edited = row.value.clone();
        let mut before = edited.clone();
        match (&mut before, focus.cell) {
            (GraphControlValue::Text(value), None) => *value = focus.original.clone(),
            (GraphControlValue::List { rows, .. }, Some((r, c))) => {
                match rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                    Some(GraphControlValue::Text(value)) => *value = focus.original.clone(),
                    _ => return,
                }
            }
            _ => return,
        }
        if commit {
            if before != edited {
                self.record_edit(GraphEdit::SetValue {
                    node: focus.node,
                    row: focus.row,
                    before,
                    after: edited,
                });
            }
        } else {
            row.value = before;
        }
    }
    /// Returns true when keyboard input was captured, including commit/cancel.
    pub fn text_input(&mut self, doc: &mut GraphDocument, input: GraphTextInput) -> bool {
        if self.text_focus.is_none() {
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
