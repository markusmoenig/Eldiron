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
    pub(crate) original: String,
    pub(crate) caret: usize,
    pub(crate) anchor: usize,
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
        if let Some(row) = doc
            .nodes
            .iter_mut()
            .find(|n| n.id == focus.node)
            .and_then(|n| n.rows.iter_mut().find(|r| r.id == focus.row))
        {
            if let GraphControlValue::Text(value) = &mut row.value {
                if commit && *value != focus.original {
                    self.record_edit(GraphEdit::SetValue {
                        node: focus.node,
                        row: focus.row,
                        before: GraphControlValue::Text(focus.original),
                        after: GraphControlValue::Text(value.clone()),
                    });
                } else if !commit {
                    *value = focus.original;
                }
            }
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
                let focus = self.text_focus.as_mut().unwrap();
                if let Some(GraphRow {
                    value: GraphControlValue::Text(value),
                    ..
                }) = doc
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == focus.node)
                    .and_then(|n| n.rows.iter_mut().find(|r| r.id == focus.row))
                {
                    focus.edit(value, input);
                } else {
                    self.text_focus = None;
                }
            }
        }
        true
    }
}
