//! Creator presentation for declarative Entity graphs. The compiler lives in
//! shared code; no configuration operation is dispatched through behavior.
use super::*;

pub(super) fn definitions(project: &Project) -> GraphDefinitions {
    let rules = project
        .rules_source()
        .unwrap_or_default()
        .parse::<shared::entity_graph::RulesTable>()
        .unwrap_or_default();
    let source = shared::entity_graph::builtin_registry().definitions(&rules);
    let mut result = GraphDefinitions::default();
    for definition in source.nodes() {
        let mut d = definition.clone();
        d.title = title(&d.id);
        d.category = if matches!(d.id.as_str(), "entity_input" | "entity_inputs") {
            fl!("entity_group_input")
        } else {
            fl!("entity_group_configuration")
        };
        d.color = if matches!(d.id.as_str(), "entity_input" | "entity_inputs") {
            [35, 87, 134, 255]
        } else {
            [91, 86, 151, 255]
        };
        for param in &mut d.parameters {
            param.label = label(&param.id);
            if let GraphControlValue::List { columns, .. } = &mut param.default {
                for column in columns {
                    column.label = label(&column.id);
                }
            }
            if param.id == "type" {
                if let GraphControlValue::Choice { options, .. } = &mut param.default {
                    *options = vec![
                        fl!("entity_type_text"),
                        fl!("entity_type_integer"),
                        fl!("entity_type_number"),
                        fl!("entity_type_boolean"),
                        fl!("entity_type_list"),
                    ];
                }
            }
        }
        result.register_node(d).unwrap();
    }
    result
}
pub(super) fn title(id: &str) -> String {
    match id {
        "entity" => fl!("entity_root"),
        "entity_identity" => fl!("entity_identity"),
        "entity_appearance" => fl!("entity_appearance"),
        "entity_set_tile" => fl!("entity_set_tile"),
        "entity_body" => fl!("entity_body"),
        "entity_inventory" => fl!("entity_inventory"),
        "entity_equipment" => fl!("entity_equipment"),
        "entity_item_state" => fl!("entity_item_state"),
        "entity_player" => fl!("entity_player"),
        "entity_attribute" => fl!("entity_attribute"),
        "entity_input" => fl!("entity_input"),
        "entity_inputs" => fl!("entity_inputs"),
        "entity_ruleset_item" => fl!("entity_ruleset_item"),
        "entity_light" => fl!("entity_light"),
        _ => id.into(),
    }
}
pub(super) fn label(key: &str) -> String {
    match key {
        "race" => fl!("entity_race"),
        "class" => fl!("entity_class"),
        "level" => fl!("entity_level"),
        "flicker" => fl!("entity_flicker"),
        "avatar" => fl!("entity_avatar"),
        "tile_id" => fl!("entity_tile"),
        "size_2d" => fl!("entity_size"),
        "visible" => fl!("entity_visible"),
        "radius" => fl!("entity_radius"),
        "blocking" => fl!("entity_blocking"),
        "active" => fl!("entity_item_active"),
        "openable" => fl!("entity_item_openable"),
        "locked" => fl!("entity_item_locked"),
        "required_key" => fl!("entity_item_required_key"),
        "inventory_slots" => fl!("entity_slots"),
        "start_equipped_items" => fl!("entity_equipment_items"),
        "item" => fl!("entity_equipment_item"),
        "wealth" => fl!("entity_wealth"),
        "player" => fl!("entity_player_eligible"),
        "attribute" => fl!("entity_attribute_name"),
        "value" => fl!("entity_value"),
        "key" => fl!("entity_key"),
        "command" => fl!("entity_command"),
        "custom_command" => fl!("entity_custom_command"),
        "type" => fl!("entity_type"),
        "color" => fl!("entity_light_color"),
        "strength" => fl!("entity_light_strength"),
        "range" => fl!("entity_light_range"),
        "lift" => fl!("entity_light_lift"),
        "ruleset_path" => fl!("entity_ruleset_path"),
        "bindings" => fl!("entity_bindings"),
        _ => key.into(),
    }
}
pub(super) fn is_configuration(pc: ProjectContext) -> bool {
    matches!(
        pc,
        ProjectContext::CharacterData(_) | ProjectContext::ItemData(_)
    )
}
pub(super) fn help(id: &str) -> String {
    match id {
        "entity" => fl!("entity_root_help"),
        "entity_identity" => fl!("entity_identity_help"),
        "entity_set_tile" => fl!("entity_set_tile_help"),
        "entity_input" => fl!("entity_input_help"),
        "entity_attribute" => fl!("entity_attribute_help"),
        "entity_equipment" => fl!("entity_equipment_help"),
        "entity_item_state" => fl!("entity_item_state_help"),
        _ => fl!("entity_configuration_help"),
    }
}
impl NodesDock {
    pub(super) fn is_configuration(&self) -> bool {
        self.owner
            .as_ref()
            .is_some_and(|key| key.starts_with(shared::entity_graph::PREFIX))
    }
    /// Drops replace the same declared slot, while gameplay policy checks other slots.
    pub(super) fn drop_equipment_item(
        &mut self,
        project: &mut Project,
        drop: &TheDrop,
        point: Vec2<i32>,
    ) -> Result<bool, String> {
        if !self
            .owner
            .as_ref()
            .is_some_and(|o| o.starts_with("entity/character/"))
            || !project.items.contains_key(&drop.id.references)
        {
            return Ok(false);
        }
        let p = self
            .editor
            .viewport
            .to_graph([point.x as f32, point.y as f32]);
        let Some(target) = self
            .doc
            .nodes
            .iter()
            .rev()
            .find(|n| self.editor.node_visible(n.id) && n.rect(&self.doc.metrics()).contains(p))
            .filter(|n| n.definition.as_deref() == Some("entity_equipment"))
            .map(|n| n.id)
        else {
            return Ok(false);
        };
        self.finish(project);
        let rules = project
            .rules_source()?
            .parse::<shared::entity_graph::RulesTable>()
            .map_err(|e| e.to_string())?;
        let owner = self.owner.as_deref().unwrap();
        let data = shared::entity_graph::project_data(
            &self.doc,
            shared::entity_graph::owner_data(project, owner).unwrap_or(""),
            &rules,
        )?;
        let catalog = shared::equipment::EquipmentCatalog::new(project)?;
        let Some(row) = self
            .doc
            .nodes
            .iter_mut()
            .find(|n| n.id == target)
            .and_then(|n| {
                n.rows
                    .iter_mut()
                    .find(|r| r.key.as_deref() == Some("start_equipped_items"))
            })
        else {
            return Ok(false);
        };
        let GraphControlValue::List { rows, row_ids, .. } = &mut row.value else {
            return Ok(false);
        };
        let existing = rows
            .iter()
            .map(|r| {
                r.first()
                    .map(|v| match v {
                        GraphControlValue::Text(s) => s.clone(),
                        _ => String::new(),
                    })
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        let next = catalog.replace_for_drop(&data, &existing, drop.id.references)?;
        if next != existing {
            *rows = next
                .into_iter()
                .map(|s| vec![GraphControlValue::Text(s)])
                .collect();
            row_ids.truncate(rows.len());
            while row_ids.len() < rows.len() {
                row_ids.push(Uuid::new_v4());
            }
        }
        self.configuration_error = None;
        self.editor.selected = Some(target);
        self.editor.selected_connection = None;
        Ok(true)
    }
    /// Friendly labels are presentation only; text editing and saved data retain stable IDs.
    pub(super) fn equipment_display_document(&self) -> GraphDocument {
        let mut doc = self.doc.clone();
        for node in &mut doc.nodes {
            if node.definition.as_deref() != Some("entity_equipment") {
                continue;
            }
            for row in &mut node.rows {
                if row.key.as_deref() != Some("start_equipped_items") {
                    continue;
                }
                if let GraphControlValue::List { rows, columns, .. } = &mut row.value {
                    for column in columns {
                        column.label = fl!("entity_equipment_item");
                    }
                    for (i, cells) in rows.iter_mut().enumerate() {
                        if self.editor.text_focus().is_some_and(|f| {
                            f.node == node.id && f.row == row.id && f.cell == Some((i, 0))
                        }) {
                            continue;
                        }
                        if let Some(GraphControlValue::Text(reference)) = cells.first_mut() {
                            if let Some(name) = self.tiles.equipment_names.get(reference) {
                                *reference = name.clone();
                            }
                        }
                    }
                }
            }
        }
        doc
    }
    /// Show validation feedback without adding a separate values panel.
    pub(super) fn refresh_configuration(&mut self, project: &Project) {
        if !self.is_configuration() {
            return;
        }
        let Some(owner) = &self.owner else {
            return;
        };
        sync_attribute_types(&mut self.doc);
        sync_input_choices(&mut self.doc, &self.definitions);
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
            shared::entity_graph::ensure_character_fields(&mut self.doc, &rules);
        }
        let result = shared::entity_graph::project_data(
            &self.doc,
            shared::entity_graph::owner_data(project, owner).unwrap_or(""),
            &rules,
        );
        self.tiles.refresh_equipment_names(project);
        let equipment = shared::equipment::EquipmentCatalog::new(project);
        if let Ok(catalog) = &equipment {
            for node in &self.doc.nodes {
                if node.definition.as_deref() != Some("entity_equipment") {
                    continue;
                }
                for row in &node.rows {
                    if let GraphControlValue::List { rows, .. } = &row.value {
                        for cells in rows {
                            if let Some(GraphControlValue::Text(reference)) = cells.first() {
                                if let Ok(item) = catalog.find(reference) {
                                    self.tiles
                                        .equipment_names
                                        .insert(reference.clone(), item.name.clone());
                                }
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
                .any(|id| owner == &shared::entity_graph::character_key(*id))
            {
                equipment
                    .as_ref()
                    .map_err(Clone::clone)?
                    .validate_explicit(&data)?;
            }
            Ok(data)
        });
        let error = result.err();
        // Rejected-drop feedback stays visible while panning or hovering.
        if error.is_some() || self.doc != self.committed {
            self.configuration_error = error.clone();
        }
        let Some(root) = self
            .doc
            .nodes
            .iter_mut()
            .find(|n| matches!(n.definition.as_deref(), Some("entity" | "entity_inputs")))
        else {
            return;
        };
        let mut fields = vec![(
            "summary".to_string(),
            fl!("entity_configuration_status"),
            error.unwrap_or_else(|| fl!("entity_configuration_ready")),
        )];
        // Keep IDs stable across edits for hit testing and history.
        for (key, label, text) in fields.drain(..) {
            if let Some(row) = root
                .rows
                .iter_mut()
                .find(|r| r.key.as_deref() == Some(&key))
            {
                row.label = label;
                row.value = GraphControlValue::Label(text);
            } else {
                let mut row = GraphRow::new(&label, GraphControlValue::Label(text));
                row.key = Some(key);
                root.rows.push(row);
            }
        }
        root.rows.retain(|r| r.key.as_deref() != Some("effective"));
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

impl NodesDock {
    pub(super) fn open_configuration_choice(&mut self, point: [f32; 2], ui: &mut TheUI) -> bool {
        let Some(target) = self.editor.choice_at(&self.doc, point) else {
            return false;
        };
        let Some(view) = ui.get_render_view(VIEW) else {
            return false;
        };
        let dim = *view.dim();
        let rect = GraphRect {
            origin: self.editor.viewport.to_screen(target.rect.origin),
            size: [
                target.rect.size[0] * self.editor.viewport.zoom(),
                target.rect.size[1] * self.editor.viewport.zoom(),
            ],
        };
        let items = target
            .options
            .iter()
            .enumerate()
            .map(|(index, label)| GraphPickerItem {
                id: index.to_string(),
                label: if label.is_empty() {
                    fl!("entity_inherit")
                } else {
                    label.clone()
                },
            })
            .collect();
        let mut picker = GraphPicker::compact(rect, [dim.width as f32, dim.height as f32], items);
        picker.search_label = fl!("node_search");
        picker.empty_label = fl!("node_no_results");
        self.choice_target = Some(TextTarget {
            node: target.node,
            row: target.row,
            cell: target.cell,
        });
        self.popup = Some((picker, None));
        true
    }
}
