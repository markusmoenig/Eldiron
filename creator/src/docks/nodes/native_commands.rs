//! Source-addressed domain commands, independent of widget mounting/rendering.
use super::*;
impl NativeNodesWorkspace {
    pub fn command_error(&mut self, key: &NodeSourceKey, error: Option<String>) {
        if let Some(view) = self.views.get_mut(key) {
            view.command_error = error;
        }
        self.refresh_errors();
    }
    /// None means no target; Some reports an accepted drop and whether source changed.
    pub fn drop_equipment_item(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        item: Uuid,
        point: Point,
        fonts: &mut TextSystem,
    ) -> Result<Option<bool>, String> {
        let view = self
            .views
            .get(key)
            .ok_or_else(|| NodeSourceError::MissingBranch.to_string())?;
        if !matches!(view.context, ProjectContext::CharacterData(_))
            || !project.items.contains_key(&item)
        {
            return Ok(None);
        }
        let model = view.model.lock().unwrap();
        let target = model
            .document()
            .nodes
            .iter()
            .rev()
            .find(|node| {
                model.editor().node_visible(node.id)
                    && node.rect(&model.document().metrics()).contains(point)
            })
            .filter(|node| node.definition.as_deref() == Some("entity_equipment"))
            .map(|node| node.id);
        drop(model);
        let Some(target) = target else {
            return Ok(None);
        };
        self.prepare_source_command(project, key, fonts)?;
        let mut document = self.cpu.source(key).unwrap().document().clone();
        if !entity::apply_equipment_item(&mut document, project, &key.owner, target, item)? {
            return Ok(None);
        }
        let changed = self.edit_source(project, key, fonts, |doc, _| {
            *doc = document;
            Ok(key.clone())
        })?;
        self.command_error(key, None);
        self.views[key].model.lock().unwrap().edit(|_, editor| {
            editor.selected = Some(target);
            editor.selected_connection = None;
        });
        Ok(Some(changed))
    }
    fn insert_branch(
        &mut self,
        project: &mut Project,
        from: &NodeSourceKey,
        document: GraphDocument,
        root: GraphId,
        fonts: &mut TextSystem,
    ) -> Result<NodeSourceKey, String> {
        self.prepare_source_command(project, from, fonts)?;
        let key = NodeSourceKey {
            owner: from.owner.clone(),
            branch: Some(root),
        };
        let context = self.views[from].context;
        let viewport = self.views[from].model.lock().unwrap().editor().viewport;
        let definitions = source_definitions(project, &key);
        self.cpu
            .create_branch(project, key.clone(), &definitions)
            .map_err(|error| error.to_string())?;
        let result = self.cpu.transition_document(
            project,
            &key,
            &key,
            document,
            &definitions,
            |key, doc, base, project| project_source_graph(doc, base, project, key, &definitions),
        );
        let (transaction, outcome) = match result {
            Ok(result) => result,
            Err(error) => {
                self.cpu.remove(&key);
                return Err(error.to_string());
            }
        };
        let source = self.cpu.source(&key).unwrap();
        let revision = source.revision();
        let document = source.document().clone();
        self.install_view(&key, document, &definitions, context, revision)?;
        let view = self.views.get_mut(&key).unwrap();
        view.history.record(transaction);
        view.model.lock().unwrap().edit(|_, editor| {
            editor.viewport = viewport;
            editor.selected = Some(root);
        });
        self.changes.extend(outcome.changes);
        if self.active.as_ref() == Some(from) {
            self.active = Some(key.clone());
            self.selected.insert(key.owner.clone(), key.branch);
        }
        self.refresh_errors();
        Ok(key)
    }
    pub fn add_catalog_node(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        definition: &str,
        position: Point,
        fonts: &mut TextSystem,
    ) -> Result<bool, String> {
        let context = self
            .views
            .get(key)
            .ok_or_else(|| NodeSourceError::MissingBranch.to_string())?
            .context;
        if !node_available(context, definition) {
            return Ok(false);
        }
        self.prepare_source_command(project, key, fonts)?;
        let definitions = source_definitions(project, key);
        let mut node = definitions.instantiate(definition, position)?;
        if definition == "event"
            && !matches!(
                context,
                ProjectContext::Character(_)
                    | ProjectContext::CharacterCode(_)
                    | ProjectContext::RegionCharacterInstance(_, _)
            )
        {
            if let Some(row) = node
                .rows
                .iter_mut()
                .find(|row| row.key.as_deref() == Some("event"))
            {
                row.value = GraphControlValue::Custom {
                    kind: "event".into(),
                    data: "startup".into(),
                };
            }
        }
        let root = is_branch_trigger(&node, &definitions);
        let id = node.id;
        let existing_root = self
            .cpu
            .source(key)
            .unwrap()
            .document()
            .nodes
            .iter()
            .any(|node| is_branch_trigger(node, &definitions));
        let target = if key.kind() == NodeGraphKind::Configuration {
            self.views[key].shown
        } else {
            key.branch
        };
        node.branch = if root { Some(id) } else { target };
        let to = if root && key.kind() != NodeGraphKind::Configuration {
            if existing_root {
                self.insert_branch(
                    project,
                    key,
                    GraphDocument {
                        nodes: vec![node],
                        ..Default::default()
                    },
                    id,
                    fonts,
                )?
            } else {
                let to = NodeSourceKey {
                    owner: key.owner.clone(),
                    branch: Some(id),
                };
                self.edit_source(project, key, fonts, |doc, _| {
                    doc.nodes.push(node);
                    Ok(to.clone())
                })?;
                to
            }
        } else {
            self.edit_source(project, key, fonts, |doc, _| {
                doc.nodes.push(node);
                Ok(key.clone())
            })?;
            if root {
                self.show_configuration_branch(key, id, &definitions, fonts)?;
            }
            key.clone()
        };
        self.views[&to].model.lock().unwrap().edit(|_, editor| {
            editor.selected = Some(id);
            editor.selected_connection = None;
        });
        Ok(true)
    }
    pub fn branch_document(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        fonts: &mut TextSystem,
    ) -> Result<GraphDocument, String> {
        self.prepare_source_command(project, key, fonts)?;
        let mut document = self.cpu.source(key).unwrap().document().clone();
        if key.kind() == NodeGraphKind::Configuration
            && let Some(root) = self.views[key].shown
        {
            let definitions = source_definitions(project, key);
            let branches = graph_branches(&document, &definitions);
            let nodes = branches
                .branch(root)
                .ok_or_else(|| NodeSourceError::MissingBranch.to_string())?
                .nodes
                .clone();
            document.nodes.retain(|node| nodes.contains(&node.id));
            let ports: std::collections::HashSet<_> = document
                .nodes
                .iter()
                .flat_map(|node| node.ports.iter().map(|port| port.id))
                .collect();
            document
                .connections
                .retain(|wire| ports.contains(&wire.from) && ports.contains(&wire.to));
        }
        Ok(document)
    }
    pub fn paste_branch_document(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        clipboard: &GraphDocument,
        fonts: &mut TextSystem,
    ) -> Result<bool, String> {
        let configuration = key.kind() == NodeGraphKind::Configuration;
        // Availability controls creation from the catalogue. Existing legacy
        // branches retain the reference adapter's domain compatibility policy.
        if clipboard.nodes.iter().any(|node| {
            node.definition.as_deref().is_some_and(|id| {
                id.starts_with("rules_")
                    || id.starts_with("particle_")
                    || id.starts_with("entity") != configuration
            })
        }) {
            return Ok(false);
        }
        self.prepare_source_command(project, key, fonts)?;
        let definitions = source_definitions(project, key);
        let mut copy = GraphDocument::default();
        append_graph_copy(&mut copy, clipboard, [PASTE_OFFSET; 2]);
        let root = copy
            .nodes
            .iter()
            .find(|node| is_branch_trigger(node, &definitions))
            .map(|node| node.id);
        let existing_root = self
            .cpu
            .source(key)
            .unwrap()
            .document()
            .nodes
            .iter()
            .any(|node| is_branch_trigger(node, &definitions));
        if key.kind() != NodeGraphKind::Configuration
            && let Some(root) = root
            && existing_root
        {
            self.insert_branch(project, key, copy, root, fonts)?;
            return Ok(true);
        }
        let to = if key.kind() == NodeGraphKind::Configuration {
            key.clone()
        } else {
            NodeSourceKey {
                owner: key.owner.clone(),
                branch: root.or(key.branch),
            }
        };
        let changed = self.edit_source(project, key, fonts, |doc, _| {
            doc.nodes.extend(copy.nodes);
            doc.connections.extend(copy.connections);
            Ok(to.clone())
        })?;
        if key.kind() == NodeGraphKind::Configuration
            && let Some(root) = root
        {
            self.show_configuration_branch(&to, root, &definitions, fonts)?;
        }
        Ok(changed)
    }
}
