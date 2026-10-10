//! Rules mode uses the same branch canvas, but never publishes behavior plans.
use super::*;

pub(super) const BUTTONS: &[&str] = &[
    "Rules Checkpoint",
    "Rules Restore Definition",
    "Rules Restore Original",
    "Rules Recover Checkpoint",
    "Rules Start Empty",
];
pub(super) fn add_toolbar(layout: &mut dyn TheHLayoutTrait) {
    for (id, label, help) in [
        (
            BUTTONS[0],
            "Checkpoint",
            "Save the current rules nodes as a recovery checkpoint",
        ),
        (
            BUTTONS[1],
            "Restore Branch",
            "Restore the selected definition from the project's preserved original",
        ),
        (
            BUTTONS[2],
            "Restore All",
            "Restore all original rules; the discarded draft is saved as a checkpoint",
        ),
        (
            BUTTONS[3],
            "Recover",
            "Swap the current rules with the latest recovery checkpoint",
        ),
        (
            BUTTONS[4],
            "Start Empty",
            "Start a standalone ruleset with no definitions; first save a recovery checkpoint",
        ),
    ] {
        let mut button = TheTraybarButton::new(TheId::named(id));
        button.set_text(label.into());
        button.set_status_text(help);
        button.set_fixed_size(false);
        layout.add_widget(Box::new(button));
    }
}
pub(super) fn help(key: &str) -> &'static str {
    match key {
        "rules_fx" => {
            "A reusable particle preset. Choose a unique /fx/presets/name path, connect particle settings modules, then reference that name from action or condition FX stages. The preview uses the game particle simulator."
        }
        "rules_definition" => {
            "A global definition branch. Use a path such as /actions/repair or /chassis/scout. Disable or remove its branch to omit the definition."
        }
        "rules_table" => {
            "A nested table within this definition. Connect its input to the parent key terminal whose type is Table. The connection defines its location."
        }
        "rules_list" => {
            "An ordered list within this definition. Use consecutive indices starting at 0. Connect its input to a parent key terminal whose type is List."
        }
        "rules_attribute" => {
            "Supply a typed scalar value. Connect its input to a matching key terminal to override that key's inline value."
        }
        _ => "",
    }
}
impl NodesDock {
    pub(super) fn update_particle_preview(&mut self) {
        if !self.is_rules()
            || !self
                .doc
                .nodes
                .iter()
                .any(|n| n.definition.as_deref() == Some("rules_fx"))
        {
            return;
        }
        let result =
            shared::rulesets::particle_graph::compile_branch(&self.doc).and_then(|table| {
                let def: rusterix::ParticleEmitterDef = table
                    .get("emitter")
                    .ok_or("Missing emitter")?
                    .clone()
                    .try_into::<rusterix::ParticleEmitterDef>()
                    .map_err(|e| e.to_string())?;
                let mut emitter = def.instantiate(vek::Vec3::zero(), def.direction);
                let scale = table
                    .get("size_scale")
                    .and_then(|v| v.as_float())
                    .unwrap_or(1.) as f32;
                emitter.radius_range.0 *= scale;
                emitter.radius_range.1 *= scale;
                Ok(emitter)
            });
        match result {
            Ok(emitter) => {
                let time = 0.4 + self.particle_preview_started.elapsed().as_secs_f32() % 2.;
                let image = crate::docks::particle_preview::render_particle_emitter_preview(
                    &emitter, 280, 160, time,
                );
                self.tiles
                    .set_preview("rules_particle_preview", Some(image));
            }
            Err(_) => self.tiles.set_preview("rules_particle_preview", None),
        }
        self.particle_preview_updated = Some(Instant::now());
    }

    pub(super) fn is_rules(&self) -> bool {
        self.owner.as_deref() == Some(shared::rulesets::graph::OWNER)
    }
    pub(super) fn handle_rules_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        if !self.is_rules() {
            return false;
        }
        let TheEvent::StateChanged(id, TheWidgetState::Clicked) = event else {
            return false;
        };
        if !BUTTONS.contains(&id.name.as_str()) {
            return false;
        }
        self.finish(project);
        match id.name.as_str() {
            "Rules Checkpoint" => {
                project.rules.checkpoint();
                self.dirty = true;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Rules checkpoint saved.".into(),
                ));
                return true;
            }
            "Rules Start Empty" => {
                project.rules.checkpoint();
                project.rules.current = shared::rulesets::graph::RulesGraph::empty();
            }
            "Rules Restore Original" => project.rules.restore_original(),
            "Rules Recover Checkpoint" => {
                if !project.rules.restore_checkpoint() {
                    return true;
                }
            }
            "Rules Restore Definition" => {
                if let Some(path) =
                    shared::rulesets::graph::branch_path(&self.doc).map(str::to_string)
                {
                    project.rules.checkpoint();
                    if let Err(error) = project
                        .rules
                        .current
                        .restore_definition(&project.rules.original, &path)
                    {
                        self.rules_error = Some(error);
                        self.render(ui, ctx);
                        return true;
                    }
                }
            }
            _ => {}
        }
        self.rules_error = sync_live_rules(project).err();
        self.histories.clear();
        self.branch_graphs.clear();
        self.branch_order.clear();
        self.active_branch = None;
        self.doc = GraphDocument::default();
        self.committed = self.doc.clone();
        self.dirty = true;
        self.activate(ui, ctx, project, server);
        true
    }
}

const ICON_PREVIEW: &str = "rules_icon_preview";

/// Icon selectors keep the same string value as the runtime rules tree.
pub(super) fn sync_icon_controls(doc: &mut GraphDocument) {
    let Some(path) = shared::rulesets::graph::branch_path(doc).map(str::to_string) else {
        return;
    };
    let fallback = matches!(
        path.as_str(),
        "/ui/action_icon_fallbacks" | "/ui/item_icon_fallbacks"
    );
    for node in &mut doc.nodes {
        if !path.starts_with("/icons/") {
            node.rows.retain(|r| r.key.as_deref() != Some(ICON_PREVIEW));
        }
        for row in &mut node.rows {
            let GraphControlValue::List { rows, .. } = &mut row.value else {
                continue;
            };
            for cells in rows {
                let [
                    GraphControlValue::Text(key),
                    GraphControlValue::Choice { options, selected },
                    value,
                ] = cells.as_mut_slice()
                else {
                    continue;
                };
                let icon = options.get(*selected).is_some_and(|s| s == "Text")
                    && (fallback
                        || key == "icon"
                        || key.ends_with("_icon")
                        || (path.starts_with("/icons/") && key == "texture"));
                if icon {
                    if let GraphControlValue::Text(id) = value {
                        *value = GraphControlValue::Custom {
                            kind: "rules_icon".into(),
                            data: id.clone().into(),
                        };
                    }
                } else if let GraphControlValue::Custom { kind, data } = value {
                    if kind == "rules_icon" {
                        *value = GraphControlValue::Text(data.as_str().unwrap_or_default().into());
                    }
                }
            }
        }
        if node.definition.as_deref() == Some("rules_definition") && path.starts_with("/icons/") {
            let texture = node
                .rows
                .iter()
                .find_map(|r| match &r.value {
                    GraphControlValue::List { rows, .. } => rows.iter().find_map(|cells| {
                        if cells.first() != Some(&GraphControlValue::Text("texture".into())) {
                            return None;
                        }
                        match cells.get(2)? {
                            GraphControlValue::Text(id) => Some(id.clone()),
                            GraphControlValue::Custom { data, .. } => {
                                data.as_str().map(str::to_string)
                            }
                            _ => None,
                        }
                    }),
                    _ => None,
                })
                .filter(|id| !id.trim().is_empty())
                .unwrap_or_else(|| {
                    path.trim_start_matches("/icons/")
                        .replace("~1", "/")
                        .replace("~0", "~")
                });
            let value = GraphControlValue::Custom {
                kind: "rules_icon".into(),
                data: texture.into(),
            };
            if let Some(row) = node
                .rows
                .iter_mut()
                .find(|r| r.key.as_deref() == Some(ICON_PREVIEW))
            {
                row.value = value;
            } else {
                let mut row = GraphRow::new("Icon artwork (click to replace)", value);
                row.key = Some(ICON_PREVIEW.into());
                node.rows.push(row);
            }
        }
    }
    let icon_values: std::collections::HashSet<_> = doc.connections.iter().filter_map(|link| {
        let (source, port) = doc.port(link.from)?;
        let (target, _) = doc.port(link.to)?;
        if target.definition.as_deref() != Some("rules_attribute") { return None; }
        let item = port.list_item?;
        let is_icon = source.rows.iter().any(|row| match &row.value {
            GraphControlValue::List { rows, row_ids, .. } => row_ids.iter().position(|id| *id == item).and_then(|i| rows.get(i)).and_then(|cells| cells.get(2)).is_some_and(|v| matches!(v, GraphControlValue::Custom { kind, .. } if kind == "rules_icon")),
            _ => false,
        });
        is_icon.then_some(target.id)
    }).collect();
    for node in &mut doc.nodes {
        if node.definition.as_deref() != Some("rules_attribute") {
            continue;
        }
        if let Some(row) = node
            .rows
            .iter_mut()
            .find(|r| r.key.as_deref() == Some("value"))
        {
            if icon_values.contains(&node.id) {
                if let GraphControlValue::Text(id) = &row.value {
                    row.value = GraphControlValue::Custom {
                        kind: "rules_icon".into(),
                        data: id.clone().into(),
                    };
                }
            } else if let GraphControlValue::Custom { kind, data } = &row.value {
                if kind == "rules_icon" {
                    row.value = GraphControlValue::Text(data.as_str().unwrap_or_default().into());
                }
            }
        }
    }
}

pub(super) fn apply_icon_selection(doc: &mut GraphDocument, target: &TextTarget, tile: &str) {
    if target.cell.is_some() {
        return;
    }
    let Some(node) = doc.nodes.iter_mut().find(|n| n.id == target.node) else {
        return;
    };
    if !node
        .rows
        .iter()
        .any(|r| r.id == target.row && r.key.as_deref() == Some(ICON_PREVIEW))
    {
        return;
    }
    let Some(entries) = node
        .rows
        .iter_mut()
        .find(|r| r.key.as_deref() == Some("entries"))
    else {
        return;
    };
    let GraphControlValue::List { rows, columns, .. } = &mut entries.value else {
        return;
    };
    let value = GraphControlValue::Custom {
        kind: "rules_icon".into(),
        data: tile.into(),
    };
    if let Some(cells) = rows
        .iter_mut()
        .find(|cells| cells.first() == Some(&GraphControlValue::Text("texture".into())))
    {
        cells[2] = value;
    } else {
        let mut cells: Vec<_> = columns.iter().map(|c| c.control.clone()).collect();
        cells[0] = GraphControlValue::Text("texture".into());
        cells[2] = value;
        rows.push(cells);
    }
}
