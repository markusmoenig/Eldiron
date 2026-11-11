//! Detached multiline drafts reuse the common shaped editor, stable graph
//! addresses and ordered transactions. Panel chrome/input scopes belong to hosts.
use super::*;
use crate::{text::TextSystem, text_edit::EditorView};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphDraftError {
    WrongBinding,
    MissingTarget,
    SourceChanged,
    EditorBusy,
}
impl std::fmt::Display for GraphDraftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::WrongBinding => "The graph draft belongs to another document",
            Self::MissingTarget => "The graph draft's field no longer exists",
            Self::SourceChanged => "The graph field changed outside this draft",
            Self::EditorBusy => "Another graph edit is still active",
        })
    }
}
impl std::error::Error for GraphDraftError {}

/// CPU state, independent of widgets, windows, renderers and application project
/// data. One owner routes the public EditorView's native scope/IME/clipboard ports.
/// Dropping/cancelling an uncheckpointed draft never modifies graph source.
pub struct GraphTextDraft {
    pub view: EditorView,
    binding: u64,
    target: GraphTextFocus,
    baseline: String,
}
impl GraphTextDraft {
    pub fn new(
        binding: u64,
        target: GraphTextFocus,
        document: &GraphDocument,
        fonts: &mut TextSystem,
    ) -> Result<Self, GraphDraftError> {
        let baseline = target
            .value(document)
            .ok_or(GraphDraftError::MissingTarget)?
            .to_owned();
        let mut view = EditorView::new(true);
        view.editor.set_text(fonts, &baseline);
        view.set_selection_drag_move(true);
        Ok(Self {
            view,
            binding,
            target,
            baseline,
        })
    }
    pub fn binding(&self) -> u64 {
        self.binding
    }
    pub fn target(&self) -> &GraphTextFocus {
        &self.target
    }
    /// Committed draft content; the common controller excludes transient preedit.
    pub fn text(&self) -> &str {
        self.view.editor.text()
    }
    pub fn has_changes(&self) -> bool {
        self.text() != self.baseline
    }
    pub fn validate(&self, binding: u64, document: &GraphDocument) -> Result<(), GraphDraftError> {
        if binding != self.binding {
            return Err(GraphDraftError::WrongBinding);
        }
        let source = self
            .target
            .value(document)
            .ok_or(GraphDraftError::MissingTarget)?;
        if source != self.baseline {
            return Err(GraphDraftError::SourceChanged);
        }
        Ok(())
    }
    /// Apply committed text as one reversible row transaction. Keep composition,
    /// callback identity, selection and local history alive. A caller storing raw
    /// GraphDocuments must retain the returned edit; GraphViewState supplies the
    /// corresponding retained queue/acknowledgement adapter.
    pub fn checkpoint(
        &mut self,
        binding: u64,
        document: &mut GraphDocument,
    ) -> Result<Option<GraphEdit>, GraphDraftError> {
        self.validate(binding, document)?;
        if !self.has_changes() {
            return Ok(None);
        }
        let text = self.text().to_owned();
        let cell = self.target.resolved_cell(document);
        let row = document
            .nodes
            .iter()
            .find(|node| node.id == self.target.node)
            .and_then(|node| node.rows.iter().find(|row| row.id == self.target.row))
            .ok_or(GraphDraftError::MissingTarget)?;
        let before = row.value.clone();
        let mut after = before.clone();
        match (&mut after, cell) {
            (GraphControlValue::Text(value), None) if self.target.cell.is_none() => {
                *value = text.clone()
            }
            (GraphControlValue::List { rows, .. }, Some((r, c))) => {
                let Some(GraphControlValue::Text(value)) =
                    rows.get_mut(r).and_then(|row| row.get_mut(c))
                else {
                    return Err(GraphDraftError::MissingTarget);
                };
                *value = text.clone();
            }
            _ => return Err(GraphDraftError::MissingTarget),
        }
        let edit = GraphEdit::SetValue {
            node: self.target.node,
            row: self.target.row,
            before,
            after,
        };
        edit.apply(document, true);
        self.baseline = text;
        Ok(Some(edit))
    }
}
