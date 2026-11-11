//! CPU authoring-schema projection of a supplied working graph. Project services
//! are read-only; this module never persists a project or accesses renderer globals.
use super::*;

pub(super) struct ConfigurationProjection {
    pub error: Option<String>,
    /// Computed before presentation-only status rows are refreshed, preserving
    /// the owner's rejected-drop feedback semantics.
    pub source_changed: bool,
    pub equipment_names: HashMap<String, String>,
}

pub(super) fn equipment_names(project: &Project) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for item in project.items.values() {
        names.insert(item.id.to_string(), item.name.clone());
        names.insert(item.name.clone(), item.name.clone());
    }
    names
}

/// The same projection can run on authoritative source or a newer native working
/// draft. It does not replace either graph with the other's persisted snapshot.
pub(super) fn project_graph(
    document: &mut GraphDocument,
    committed: &GraphDocument,
    owner: &str,
    definitions: &GraphDefinitions,
    project: &Project,
) -> ConfigurationProjection {
    sync_attribute_types(document);
    sync_input_choices(document, definitions);
    let rules = project
        .rules_source()
        .unwrap_or_default()
        .parse::<shared::entity_graph::RulesTable>()
        .unwrap_or_default();
    if owner
        .strip_prefix("entity/character/")
        .and_then(|id| Uuid::parse_str(id).ok())
        .is_some_and(|id| project.characters.contains_key(&id))
    {
        shared::entity_graph::ensure_character_fields(document, &rules);
    }
    let result = shared::entity_graph::project_data(
        document,
        shared::entity_graph::owner_data(project, owner).unwrap_or(""),
        &rules,
    );
    let mut names = equipment_names(project);
    let equipment = shared::equipment::EquipmentCatalog::new(project);
    if let Ok(catalog) = &equipment {
        for node in &document.nodes {
            if node.definition.as_deref() != Some("entity_equipment") {
                continue;
            }
            for row in &node.rows {
                if let GraphControlValue::List { rows, .. } = &row.value {
                    for cells in rows {
                        if let Some(GraphControlValue::Text(reference)) = cells.first()
                            && let Ok(item) = catalog.find(reference)
                        {
                            names.insert(reference.clone(), item.name.clone());
                        }
                    }
                }
            }
        }
    }
    let result = result.and_then(|data| {
        if project
            .characters
            .keys()
            .any(|id| owner == shared::entity_graph::character_key(*id).as_str())
        {
            equipment
                .as_ref()
                .map_err(Clone::clone)?
                .validate_explicit(&data)?;
        }
        Ok(data)
    });
    let error = result.err();
    let source_changed = *document != *committed;
    if let Some(root) = document
        .nodes
        .iter_mut()
        .find(|node| matches!(node.definition.as_deref(), Some("entity" | "entity_inputs")))
    {
        let label = fl!("entity_configuration_status");
        let text = error
            .clone()
            .unwrap_or_else(|| fl!("entity_configuration_ready"));
        if let Some(row) = root
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("summary"))
        {
            row.label = label;
            row.value = GraphControlValue::Label(text);
        } else {
            let mut row = GraphRow::new(&label, GraphControlValue::Label(text));
            row.key = Some("summary".into());
            root.rows.push(row);
        }
        root.rows
            .retain(|row| row.key.as_deref() != Some("effective"));
    }
    ConfigurationProjection {
        error,
        source_changed,
        equipment_names: names,
    }
}

/// Changing the selected attribute type changes the value control, reusing the
/// same mouse/text controls as behavior nodes. Keep an existing matching type.
fn sync_attribute_types(doc: &mut GraphDocument) {
    for node in &mut doc.nodes {
        if node.definition.as_deref() != Some("entity_attribute") {
            continue;
        }
        let selected = node
            .rows
            .iter()
            .find(|r| r.key.as_deref() == Some("type"))
            .and_then(|r| {
                if let GraphControlValue::Choice { selected, .. } = &r.value {
                    Some(*selected)
                } else {
                    None
                }
            });
        let Some(selected) = selected else {
            continue;
        };
        let Some(row) = node
            .rows
            .iter_mut()
            .find(|r| r.key.as_deref() == Some("value"))
        else {
            continue;
        };
        let matches = match (&row.value, selected) {
            (GraphControlValue::Text(_), 0 | 1 | 2)
            | (GraphControlValue::Toggle(_), 3)
            | (GraphControlValue::List { .. }, 4) => true,
            (GraphControlValue::Number { step, .. }, 1) => *step >= 1.,
            (GraphControlValue::Number { step, .. }, 2) => *step < 1.,
            _ => false,
        };
        if matches {
            continue;
        }
        row.value = match selected {
            1 | 2 => GraphControlValue::Number {
                value: 0.,
                min: -100000.,
                max: 100000.,
                step: if selected == 1 { 1. } else { 0.1 },
            },
            3 => GraphControlValue::Toggle(false),
            4 => GraphControlValue::List {
                row_ids: vec![],
                columns: vec![GraphListColumn {
                    id: "value".into(),
                    label: fl!("entity_value"),
                    control: GraphControlValue::Text(String::new()),
                }],
                rows: vec![],
            },
            _ => GraphControlValue::Text(String::new()),
        };
    }
}

fn sync_input_choices(doc: &mut GraphDocument, defs: &GraphDefinitions) {
    let Some(def) = defs.node("entity_input") else {
        return;
    };
    let Some(GraphControlValue::Choice {
        options: available, ..
    }) = def
        .parameters
        .iter()
        .find(|p| p.id == "command")
        .map(|p| &p.default)
    else {
        return;
    };
    for node in &mut doc.nodes {
        if node.definition.as_deref() != Some("entity_input") {
            continue;
        }
        for row in &mut node.rows {
            if row.key.as_deref() != Some("command") {
                continue;
            }
            if let GraphControlValue::Choice { options, selected } = &mut row.value {
                let current = options.get(*selected).cloned().unwrap_or_default();
                *options = available.clone();
                *selected = options
                    .iter()
                    .position(|s| s == &current)
                    .unwrap_or_else(|| {
                        options.push(current);
                        options.len() - 1
                    });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Project, String, GraphDefinitions, GraphDocument) {
        let mut project = Project::default();
        let mut character = shared::character::Character::new();
        character.data =
            "[attributes]\nrace='Human'\nclass='Warrior'\ncustom_note='Original'\n".into();
        let owner = shared::entity_graph::character_key(character.id);
        let rules = project.rules_source().unwrap().parse().unwrap();
        let mut document = shared::entity_graph::import_character(&character.data, &rules).unwrap();
        project.characters.insert(character.id, character);
        let definitions = entity::definitions(&project);
        let baseline = document.clone();
        let projected = project_graph(&mut document, &baseline, &owner, &definitions, &project);
        assert!(projected.error.is_none(), "{:?}", projected.error);
        (project, owner, definitions, document)
    }
    fn note(document: &mut GraphDocument) -> &mut GraphControlValue {
        &mut document
            .nodes
            .iter_mut()
            .find(|node| {
                node.definition.as_deref() == Some("entity_attribute")
                    && node.rows.iter().any(|row| {
                        row.key.as_deref() == Some("attribute")
                            && row.value == GraphControlValue::Text("custom_note".into())
                    })
            })
            .unwrap()
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("value"))
            .unwrap()
            .value
    }
    #[test]
    fn configuration_projection_keeps_a_newer_working_draft_and_project_source_separate() {
        let isolated = std::env::var_os("ELDIRON_NATIVE_NODES_ISOLATED").is_some();
        if isolated {
            assert!(std::sync::LazyLock::get(&crate::editor::RUSTERIX).is_none());
        }
        let (project, owner, definitions, canonical) = fixture();
        let project_source = serde_json::to_value(&project).unwrap();
        let mut working = canonical.clone();
        let draft = GraphControlValue::Text("Private e\u{301}日本\r\nsecond line".into());
        *note(&mut working) = draft.clone();
        let projected = project_graph(&mut working, &canonical, &owner, &definitions, &project);
        assert!(projected.error.is_none());
        assert!(projected.source_changed);
        assert_eq!(note(&mut working), &draft);
        let settled = working.clone();
        for _ in 0..3 {
            project_graph(&mut working, &canonical, &owner, &definitions, &project);
            assert_eq!(
                working, settled,
                "derived row IDs/source stay stable across paints"
            );
        }
        let mut persisted = canonical.clone();
        let unchanged = project_graph(&mut persisted, &canonical, &owner, &definitions, &project);
        assert!(!unchanged.source_changed);
        assert_eq!(persisted, canonical);
        assert_eq!(serde_json::to_value(&project).unwrap(), project_source);
        if isolated {
            assert!(std::sync::LazyLock::get(&crate::editor::RUSTERIX).is_none());
        }
    }
    #[test]
    fn configuration_projection_preserves_rejected_drop_feedback_until_the_source_changes() {
        let (project, owner, definitions, document) = fixture();
        let mut dock = NodesDock::new();
        dock.owner = Some(owner);
        dock.definitions = definitions;
        dock.doc = document.clone();
        dock.committed = document;
        dock.configuration_error = Some("Rejected drop".into());
        dock.refresh_configuration(&project);
        assert_eq!(dock.configuration_error.as_deref(), Some("Rejected drop"));
        *note(&mut dock.doc) = GraphControlValue::Text("Corrected source".into());
        dock.refresh_configuration(&project);
        assert!(dock.configuration_error.is_none());
        assert!(dock.resources.is_none());
    }
}
