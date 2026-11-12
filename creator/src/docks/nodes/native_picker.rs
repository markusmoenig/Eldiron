//! Source-addressed picker drafts. Query/preedit are view state; choosing one
//! value produces a completed reversible graph edit without accessing Project.
use super::*;
use std::sync::Arc;
use theframework::{text::TextSystem, text_edit::EditorView};

enum Cell {
    Named(GraphId, String),
    Legacy {
        row: usize,
        column: usize,
        table: GraphControlValue,
    },
}
struct Address {
    binding: u64,
    node: GraphId,
    row: GraphId,
    cell: Option<Cell>,
    expected: GraphControlValue,
}
pub(super) struct NativePicker {
    pub picker: GraphPicker,
    pub search: EditorView,
    address: Address,
}
impl Address {
    fn capture(model: &GraphViewState, target: &TextTarget) -> Result<Self, GraphDraftError> {
        let row = model
            .document()
            .nodes
            .iter()
            .find(|node| node.id == target.node)
            .and_then(|node| node.rows.iter().find(|row| row.id == target.row))
            .ok_or(GraphDraftError::MissingTarget)?;
        let (cell, expected) = if let Some((r, c)) = target.cell {
            let GraphControlValue::List {
                columns,
                rows,
                row_ids,
            } = &row.value
            else {
                return Err(GraphDraftError::MissingTarget);
            };
            let value = rows
                .get(r)
                .and_then(|row| row.get(c))
                .ok_or(GraphDraftError::MissingTarget)?;
            let column = columns.get(c).ok_or(GraphDraftError::MissingTarget)?;
            let cell = if let Some(id) = row_ids
                .get(r)
                .filter(|id| row_ids.iter().filter(|other| *other == *id).count() == 1)
                && columns.iter().filter(|other| other.id == column.id).count() == 1
            {
                Cell::Named(*id, column.id.clone())
            } else {
                Cell::Legacy {
                    row: r,
                    column: c,
                    table: row.value.clone(),
                }
            };
            (Some(cell), value.clone())
        } else {
            (None, row.value.clone())
        };
        Ok(Self {
            binding: model.binding(),
            node: target.node,
            row: target.row,
            cell,
            expected,
        })
    }
    fn cell(&self, value: &GraphControlValue) -> Result<Option<(usize, usize)>, GraphDraftError> {
        let Some(cell) = &self.cell else {
            return Ok(None);
        };
        let GraphControlValue::List {
            columns,
            rows,
            row_ids,
        } = value
        else {
            return Err(GraphDraftError::MissingTarget);
        };
        let (r, c) = match cell {
            Cell::Named(id, column) => {
                if row_ids.iter().filter(|other| *other == id).count() != 1
                    || columns.iter().filter(|other| &other.id == column).count() != 1
                {
                    return Err(GraphDraftError::MissingTarget);
                }
                (
                    row_ids.iter().position(|other| other == id).unwrap(),
                    columns
                        .iter()
                        .position(|other| &other.id == column)
                        .unwrap(),
                )
            }
            Cell::Legacy { row, column, table } => {
                if value != table {
                    return Err(GraphDraftError::SourceChanged);
                }
                (*row, *column)
            }
        };
        rows.get(r)
            .and_then(|row| row.get(c))
            .ok_or(GraphDraftError::MissingTarget)?;
        Ok(Some((r, c)))
    }
    fn resolve(
        &self,
        model: &GraphViewState,
    ) -> Result<(GraphNode, Option<(usize, usize)>), GraphDraftError> {
        if self.binding != model.binding() {
            return Err(GraphDraftError::WrongBinding);
        }
        let node = model
            .document()
            .nodes
            .iter()
            .find(|node| node.id == self.node)
            .ok_or(GraphDraftError::MissingTarget)?;
        let row = node
            .rows
            .iter()
            .find(|row| row.id == self.row)
            .ok_or(GraphDraftError::MissingTarget)?;
        let cell = self.cell(&row.value)?;
        let value = if let Some((r, c)) = cell {
            let GraphControlValue::List { rows, .. } = &row.value else {
                unreachable!()
            };
            &rows[r][c]
        } else {
            &row.value
        };
        if value != &self.expected {
            return Err(GraphDraftError::SourceChanged);
        }
        Ok((node.clone(), cell))
    }
}
impl NativePicker {
    pub(super) fn new(
        model: &mut GraphViewState,
        target: &TextTarget,
        mut picker: GraphPicker,
        fonts: &mut TextSystem,
    ) -> Result<Self, GraphDraftError> {
        let address = Address::capture(model, target)?;
        model.edit(|document, editor| editor.cancel_gesture(document));
        model.finish_text(true);
        model.publish_edits();
        picker.set_always_show_search(true);
        let mut search = EditorView::new(false);
        search.editor.set_text(fonts, &picker.query);
        Ok(Self {
            picker,
            search,
            address,
        })
    }
    pub(super) fn validate(&self, model: &GraphViewState) -> Result<(), GraphDraftError> {
        self.address.resolve(model).map(|_| ())
    }
    /// Query follows committed text only. IME preedit is painted by EditorView,
    /// but does not prematurely filter the result set or change authored source.
    pub(super) fn sync_query(&mut self) {
        self.picker.set_query(self.search.editor.text());
    }
    pub(super) fn select(
        &mut self,
        model: &mut GraphViewState,
        item: &str,
    ) -> Result<Option<Arc<GraphCommit>>, GraphDraftError> {
        self.sync_query();
        if !self
            .picker
            .filtered()
            .iter()
            .any(|candidate| candidate.id == item)
        {
            return Err(GraphDraftError::MissingTarget);
        }
        let (before, cell) = self.address.resolve(model)?;
        if model.editor().is_gesturing() || model.editor().text_focus().is_some() {
            return Err(GraphDraftError::EditorBusy);
        }
        let mut after = before.clone();
        let row = after
            .rows
            .iter_mut()
            .find(|row| row.id == self.address.row)
            .unwrap();
        let value = if let Some((r, c)) = cell {
            let GraphControlValue::List { rows, .. } = &mut row.value else {
                unreachable!()
            };
            &mut rows[r][c]
        } else {
            &mut row.value
        };
        match value {
            GraphControlValue::Choice { options, selected } => {
                let index = item
                    .parse::<usize>()
                    .ok()
                    .filter(|index| *index < options.len())
                    .ok_or(GraphDraftError::MissingTarget)?;
                *selected = index;
            }
            GraphControlValue::Custom { kind, data }
                if matches!(kind.as_str(), "tile" | "rules_icon" | "event") =>
            {
                *data = item.into();
            }
            _ => return Err(GraphDraftError::MissingTarget),
        }
        // The rules icon preview and its authored texture entry are one domain
        // operation/undo, not separate serials. Only this addressed node is copied.
        let target = TextTarget {
            node: self.address.node,
            row: self.address.row,
            cell,
        };
        let mut document = GraphDocument {
            nodes: vec![after],
            ..Default::default()
        };
        rules::apply_icon_selection(&mut document, &target, item);
        let after = document.nodes.remove(0);
        model.commit_node(self.address.binding, &before, after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn choice() -> GraphControlValue {
        GraphControlValue::Choice {
            options: vec!["One".into(), "Two".into()],
            selected: 0,
        }
    }
    fn fixture(named: bool) -> (GraphViewState, TextTarget, GraphPicker, TextSystem) {
        let mut node = GraphNode::new("Table", [20., 20.], [50, 80, 100, 255]);
        let row = GraphRow::new(
            "Controls",
            GraphControlValue::List {
                columns: vec![
                    GraphListColumn {
                        id: "select".into(),
                        label: "Select".into(),
                        control: choice(),
                    },
                    GraphListColumn {
                        id: "name".into(),
                        label: "Name".into(),
                        control: GraphControlValue::Text(String::new()),
                    },
                ],
                row_ids: if named {
                    vec![Uuid::new_v4(), Uuid::new_v4()]
                } else {
                    vec![]
                },
                rows: vec![
                    vec![choice(), GraphControlValue::Text("A".into())],
                    vec![choice(), GraphControlValue::Text("B".into())],
                ],
            },
        );
        let target = TextTarget {
            node: node.id,
            row: row.id,
            cell: Some((0, 0)),
        };
        node.rows.push(row);
        let model = GraphViewState::new(
            "table",
            GraphDocument {
                nodes: vec![node],
                ..Default::default()
            },
        );
        let picker = GraphPicker::new(
            "Select",
            [20., 20.],
            vec![
                GraphPickerItem {
                    id: "0".into(),
                    label: "One".into(),
                },
                GraphPickerItem {
                    id: "1".into(),
                    label: "Two".into(),
                },
            ],
        );
        (model, target, picker, TextSystem::default())
    }
    #[test]
    fn native_node_picker_choice_follows_identified_cells_through_reordering_without_overwriting_other_edits()
     {
        let (mut model, target, picker, mut fonts) = fixture(true);
        let mut picker = NativePicker::new(&mut model, &target, picker, &mut fonts).unwrap();
        model.edit(|doc, _| {
            let GraphControlValue::List {
                rows,
                row_ids,
                columns,
            } = &mut doc.nodes[0].rows[0].value
            else {
                unreachable!()
            };
            rows.swap(0, 1);
            row_ids.swap(0, 1);
            columns.swap(0, 1);
            for row in rows.iter_mut() {
                row.swap(0, 1);
            }
            rows[0][0] = GraphControlValue::Text("Other draft".into());
        });
        let before = model.document().clone();
        let commit = picker.select(&mut model, "1").unwrap().unwrap();
        assert_eq!(commit.edits.len(), 1);
        let GraphControlValue::List { rows, .. } = &model.document().nodes[0].rows[0].value else {
            unreachable!()
        };
        assert_eq!(rows[0][0], GraphControlValue::Text("Other draft".into()));
        assert_eq!(rows[0][1], choice());
        assert!(matches!(
            rows[1][1],
            GraphControlValue::Choice { selected: 1, .. }
        ));
        let mut undone = model.document().clone();
        commit.edits[0].apply(&mut undone, false);
        assert_eq!(undone, before);
        assert_eq!(model.pending_commits().count(), 1);
        assert!(matches!(
            picker.select(&mut model, "0"),
            Err(GraphDraftError::SourceChanged)
        ));
    }
    #[test]
    fn native_node_picker_rejects_changed_legacy_tables_and_replaced_sources_atomically() {
        let (mut model, target, picker, mut fonts) = fixture(false);
        let mut picker = NativePicker::new(&mut model, &target, picker, &mut fonts).unwrap();
        model.edit(|doc, _| {
            let GraphControlValue::List { rows, .. } = &mut doc.nodes[0].rows[0].value else {
                unreachable!()
            };
            rows[1][1] = GraphControlValue::Text("Different".into());
        });
        let changed = model.document().clone();
        assert!(matches!(
            picker.select(&mut model, "1"),
            Err(GraphDraftError::SourceChanged)
        ));
        assert_eq!(model.document(), &changed);
        model.replace_document("other", changed.clone());
        assert!(matches!(
            picker.select(&mut model, "1"),
            Err(GraphDraftError::WrongBinding)
        ));
        assert_eq!(model.document(), &changed);
        assert!(model.pending_commits().next().is_none());
    }
    #[test]
    fn native_node_icon_picker_groups_the_preview_and_authored_texture_entry_in_one_transaction() {
        let mut node = GraphNode::new("Icon", [20., 20.], [50, 80, 100, 255]);
        let mut preview = GraphRow::new(
            "Preview",
            GraphControlValue::Custom {
                kind: "rules_icon".into(),
                data: "old".into(),
            },
        );
        preview.key = Some("rules_icon_preview".into());
        let target = TextTarget {
            node: node.id,
            row: preview.id,
            cell: None,
        };
        node.rows.push(preview);
        let prototype = GraphControlValue::Text(String::new());
        let mut entries = GraphRow::new(
            "Entries",
            GraphControlValue::List {
                columns: (0..3)
                    .map(|i| GraphListColumn {
                        id: i.to_string(),
                        label: i.to_string(),
                        control: prototype.clone(),
                    })
                    .collect(),
                row_ids: vec![Uuid::new_v4()],
                rows: vec![vec![
                    GraphControlValue::Text("texture".into()),
                    prototype,
                    GraphControlValue::Custom {
                        kind: "rules_icon".into(),
                        data: "old".into(),
                    },
                ]],
            },
        );
        entries.key = Some("entries".into());
        node.rows.push(entries);
        let before = GraphDocument {
            nodes: vec![node],
            ..Default::default()
        };
        let mut model = GraphViewState::new("rules", before.clone());
        let mut fonts = TextSystem::default();
        let picker = GraphPicker::new(
            "Icon",
            [20., 20.],
            vec![GraphPickerItem {
                id: "new".into(),
                label: "New icon".into(),
            }],
        );
        let mut picker = NativePicker::new(&mut model, &target, picker, &mut fonts).unwrap();
        let commit = picker.select(&mut model, "new").unwrap().unwrap();
        assert_eq!(commit.edits.len(), 1);
        let node = &model.document().nodes[0];
        let expected = GraphControlValue::Custom {
            kind: "rules_icon".into(),
            data: "new".into(),
        };
        assert_eq!(node.rows[0].value, expected);
        let GraphControlValue::List { rows, .. } = &node.rows[1].value else {
            unreachable!()
        };
        assert_eq!(rows[0][2], expected);
        let mut restored = model.document().clone();
        commit.edits[0].apply(&mut restored, false);
        assert_eq!(restored, before);
    }
}
