//! Atomic source-slot transitions for domain commands and source-local history.
//! Unlike view bindings, a transaction can address both an unrooted draft and
//! its new branch. It never restores an owner/Project snapshot over siblings.
use super::*;

/// Retain relative neighbors rather than an owner-wide branch-order snapshot.
/// Surviving anchors place a restored branch around newer unrelated siblings.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeBranchPosition {
    pub previous: Option<GraphId>,
    pub next: Option<GraphId>,
    pub index: usize,
}
impl NodeBranchPosition {
    pub(super) fn capture(order: &[GraphId], root: Option<GraphId>) -> Option<Self> {
        let root = root?;
        let index = order.iter().position(|id| *id == root)?;
        Some(Self {
            previous: index.checked_sub(1).map(|index| order[index]),
            next: order.get(index + 1).copied(),
            index,
        })
    }
    fn restore(&self, order: &mut Vec<GraphId>, root: GraphId) {
        order.retain(|id| *id != root);
        let index = self
            .next
            .and_then(|next| order.iter().position(|id| *id == next))
            .or_else(|| {
                self.previous.and_then(|previous| {
                    order
                        .iter()
                        .position(|id| *id == previous)
                        .map(|index| index + 1)
                })
            })
            .unwrap_or(self.index.min(order.len()));
        order.insert(index, root);
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSourcePatch {
    pub key: NodeSourceKey,
    pub before: Option<GraphDocument>,
    pub after: Option<GraphDocument>,
    pub before_position: Option<NodeBranchPosition>,
    pub after_position: Option<NodeBranchPosition>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSourceTransaction {
    pub focus_before: NodeSourceKey,
    pub focus_after: NodeSourceKey,
    pub patches: Vec<NodeSourcePatch>,
}
impl NodeSourceTransaction {
    pub fn from_change(change: &NodeSourceChange) -> Self {
        let slot = |document: &GraphDocument| {
            if change.key.branch.is_some() && document.nodes.is_empty() {
                None
            } else {
                Some(document.clone())
            }
        };
        Self {
            focus_before: change.key.clone(),
            focus_after: change.key.clone(),
            patches: vec![NodeSourcePatch {
                key: change.key.clone(),
                before: slot(&change.before),
                after: slot(&change.after),
                before_position: change.before_position.clone(),
                after_position: change.after_position.clone(),
            }],
        }
    }
    pub fn is_noop(&self) -> bool {
        self.focus_before == self.focus_after
            && self.patches.iter().all(|patch| patch.before == patch.after)
    }
}
#[derive(Default, Debug)]
pub struct NodeTransactionApply {
    pub changes: Vec<NodeSourceChange>,
    pub validation: Vec<(NodeSourceKey, String)>,
}

impl NodeGraphWorkspace {
    /// A completed replacement or source-address transition. The caller drains
    /// completed UI packets before invoking this operation. On failure, Project
    /// and private source documents/bindings remain unchanged.
    pub fn transition_document<F>(
        &mut self,
        project: &mut Project,
        from: &NodeSourceKey,
        to: &NodeSourceKey,
        document: GraphDocument,
        definitions: &GraphDefinitions,
        projection: F,
    ) -> Result<(NodeSourceTransaction, NodeTransactionApply), NodeSourceError>
    where
        F: FnMut(&NodeSourceKey, &mut GraphDocument, &GraphDocument, &Project) -> Option<String>,
    {
        if from.owner != to.owner || from.kind() == NodeGraphKind::Rules {
            return Err(NodeSourceError::WrongBinding);
        }
        let graphs = self.graphs(project, &from.owner, definitions)?;
        let source = self
            .sources
            .get(from)
            .ok_or(NodeSourceError::MissingBranch)?;
        if source.error.is_some() {
            return Err(NodeSourceError::SourceChanged);
        }
        let private_before = source.document.clone();
        let target = if to.branch.is_some() && document.nodes.is_empty() {
            None
        } else {
            Some(document)
        };
        let mut transaction = NodeSourceTransaction {
            focus_before: from.clone(),
            focus_after: to.clone(),
            patches: vec![],
        };
        if from == to {
            transaction.patches.push(NodeSourcePatch {
                key: from.clone(),
                before: source.stored.clone(),
                after: target,
                before_position: NodeBranchPosition::capture(&graphs.order, from.branch),
                after_position: NodeBranchPosition::capture(&graphs.order, to.branch),
            });
        } else {
            if from.kind() != NodeGraphKind::Behavior {
                return Err(NodeSourceError::WrongBinding);
            }
            // A move may only claim a vacant destination. In particular, undo
            // must not overwrite another editing session's independent draft.

            let vacant = if to.branch.is_none() {
                Some(GraphDocument::default())
            } else {
                None
            };
            if graphs.document(to.branch) != vacant.as_ref()
                || self.sources.get(to).is_some_and(|source| {
                    source.error.is_some()
                        || source.document != GraphDocument::default()
                        || source.stored != vacant
                })
            {
                return Err(NodeSourceError::SourceChanged);
            }
            transaction.patches.extend([
                NodeSourcePatch {
                    key: from.clone(),
                    before: self.sources[from].stored.clone(),
                    before_position: NodeBranchPosition::capture(&graphs.order, from.branch),
                    after_position: None,
                    after: if from.branch.is_none() {
                        Some(GraphDocument::default())
                    } else {
                        None
                    },
                },
                NodeSourcePatch {
                    key: to.clone(),
                    before: vacant,
                    after: target,
                    before_position: None,
                    after_position: None,
                },
            ]);
        }
        let applied =
            self.replay_transaction(project, &mut transaction, true, definitions, projection)?;
        // The initial layout/schema projection is private view preparation,
        // not an authored operation to undo along with the user's command.
        if !applied.changes.is_empty() {
            transaction
                .patches
                .iter_mut()
                .find(|patch| &patch.key == from)
                .unwrap()
                .before = if from.branch.is_some() && private_before.nodes.is_empty() {
                None
            } else {
                Some(private_before)
            };
        }
        Ok((transaction, applied))
    }

    /// Replay one domain/history operation atomically against current source.
    /// The written side is refreshed with the actual read-only projection so
    /// subsequent redo compares the exact canonical data that undo published.
    pub fn replay_transaction<F>(
        &mut self,
        project: &mut Project,
        transaction: &mut NodeSourceTransaction,
        forward: bool,
        definitions: &GraphDefinitions,
        mut projection: F,
    ) -> Result<NodeTransactionApply, NodeSourceError>
    where
        F: FnMut(&NodeSourceKey, &mut GraphDocument, &GraphDocument, &Project) -> Option<String>,
    {
        let owner = &transaction.focus_before.owner;
        if transaction.focus_after.owner != *owner || transaction.patches.is_empty() {
            return Err(NodeSourceError::WrongBinding);
        }
        let kind = transaction.focus_before.kind();
        let mut graphs = self.graphs(project, owner, definitions)?;
        let mut seen = std::collections::HashSet::new();
        for patch in &transaction.patches {
            if patch.key.owner != *owner
                || !seen.insert(patch.key.clone())
                || (kind == NodeGraphKind::Configuration && patch.key.branch.is_some())
            {
                return Err(NodeSourceError::WrongBinding);
            }
            let expected = if forward { &patch.before } else { &patch.after };
            if graphs.document(patch.key.branch) != expected.as_ref()
                || self
                    .sources
                    .get(&patch.key)
                    .is_some_and(|source| source.error.is_some() || &source.stored != expected)
            {
                return Err(NodeSourceError::SourceChanged);
            }
        }
        if !seen.contains(&transaction.focus_before) || !seen.contains(&transaction.focus_after) {
            return Err(NodeSourceError::WrongBinding);
        }
        // Prepare all candidates before changing either source slot or Project.
        let mut candidates = Vec::with_capacity(transaction.patches.len());
        for patch in &transaction.patches {
            let expected = if forward { &patch.before } else { &patch.after };
            let mut target = if forward {
                patch.after.clone()
            } else {
                patch.before.clone()
            };
            let baseline = expected.clone().unwrap_or_default();
            let diagnostic = if let Some(document) = &mut target {
                if document.version != 1 {
                    return Err(NodeSourceError::InvalidSource(
                        "Unsupported node graph document version".into(),
                    ));
                }
                let diagnostic = projection(&patch.key, document, &baseline, project);
                if document.version != 1
                    || patch.key.branch.is_some_and(|root| {
                        let roots = graph_branches(document, definitions);
                        roots.branches.len() != 1 || roots.branches[0].root != root
                    })
                    || (kind == NodeGraphKind::Behavior
                        && patch.key.branch.is_none()
                        && document
                            .nodes
                            .iter()
                            .any(|node| is_branch_trigger(node, definitions)))
                {
                    return Err(NodeSourceError::InvalidSource(
                        "Node graph source address does not match its branch root".into(),
                    ));
                }
                diagnostic
            } else {
                if patch.key.branch.is_none() {
                    return Err(NodeSourceError::InvalidSource(
                        "An owner draft slot cannot be removed".into(),
                    ));
                }
                None
            };
            candidates.push((patch.key.clone(), baseline, target, diagnostic));
        }
        let previous_positions: HashMap<_, _> = transaction
            .patches
            .iter()
            .map(|patch| {
                (
                    patch.key.clone(),
                    NodeBranchPosition::capture(&graphs.order, patch.key.branch),
                )
            })
            .collect();
        for (key, _, target, _) in &candidates {
            if let Some(document) = target {
                graphs.replace(key.branch, document.clone());
            } else if let Some(root) = key.branch {
                graphs.branches.remove(&root);
                graphs.order.retain(|id| *id != root);
            }
        }
        for patch in &transaction.patches {
            let (expected, target, position) = if forward {
                (&patch.before, &patch.after, &patch.after_position)
            } else {
                (&patch.after, &patch.before, &patch.before_position)
            };
            if expected.is_none()
                && target.is_some()
                && let (Some(root), Some(position)) = (patch.key.branch, position)
            {
                position.restore(&mut graphs.order, root);
            }
        }
        let value = graphs.to_value();
        let changed = candidates.iter().any(|(key, _, target, _)| {
            let patch = transaction
                .patches
                .iter()
                .find(|patch| &patch.key == key)
                .unwrap();
            let expected = if forward { &patch.before } else { &patch.after };
            expected != target
        });
        let rebind = changed || transaction.focus_before != transaction.focus_after;
        let mut outcome = NodeTransactionApply::default();
        if changed {
            project.node_graphs.insert(owner.clone(), value);
        }
        for (key, before, target, mut diagnostic) in candidates {
            let after = target.clone().unwrap_or_default();
            if kind == NodeGraphKind::Configuration {
                diagnostic = if changed {
                    entity_graph::update_owner(project, &key.owner, &after).err()
                } else {
                    entity_graph::project_owner_data(project, &key.owner, &after).err()
                }
                .or(diagnostic);
            }
            if let Some(message) = &diagnostic {
                outcome.validation.push((key.clone(), message.clone()));
            }
            let revision = if let Some(source) = self.sources.get(&key) {
                source.revision
            } else {
                self.revision = self
                    .revision
                    .checked_add(1)
                    .expect("node source revision exhausted");
                self.revision
            };
            let working = if rebind {
                after.clone()
            } else {
                self.sources
                    .get(&key)
                    .map(|source| source.document.clone())
                    .unwrap_or_else(|| after.clone())
            };
            self.sources.insert(
                key.clone(),
                NodeAuthoringSource {
                    document: working,
                    stored: target.clone(),
                    revision,
                    error: None,
                    validation: diagnostic,
                },
            );
            if rebind {
                self.bindings.retain(|_, binding| binding.key != key);
            }
            let patch = transaction
                .patches
                .iter_mut()
                .find(|patch| patch.key == key)
                .unwrap();
            let expected = if forward { &patch.before } else { &patch.after };
            if expected != &target {
                outcome.changes.push(NodeSourceChange {
                    key: key.clone(),
                    before,
                    after,
                    before_position: previous_positions[&key].clone(),
                    after_position: NodeBranchPosition::capture(&graphs.order, key.branch),
                });
            }
            if forward {
                patch.after = target;
                patch.before_position = previous_positions[&key].clone();
                patch.after_position = NodeBranchPosition::capture(&graphs.order, key.branch);
            } else {
                patch.before = target;
                patch.after_position = previous_positions[&key].clone();
                patch.before_position = NodeBranchPosition::capture(&graphs.order, key.branch);
            }
        }
        self.owners.insert(
            owner.clone(),
            OwnerSource {
                fingerprint: Self::fingerprint(project, owner, kind),
                graphs,
            },
        );
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (
        Project,
        GraphDefinitions,
        NodeGraphWorkspace,
        NodeSourceKey,
        GraphDocument,
        GraphId,
    ) {
        let mut definitions = GraphDefinitions::default();
        let mut root = GraphNode::new("Event", [10., 20.], [30, 60, 90, 255]);
        root.rows.push(GraphRow::new(
            "Text",
            GraphControlValue::Text("Before".into()),
        ));
        definitions
            .register_node(GraphNodeDefinition::from_template("event", "Event", &root))
            .unwrap();
        let mut body = GraphNodeDefinition::from_template("body", "Body", &root);
        body.starts_branch = false;
        definitions.register_node(body).unwrap();
        let mut body = definitions.instantiate("body", [20., 30.]).unwrap();
        body.rows[0].value = GraphControlValue::Text("Exact 東京\r\nمرحبا e\u{301}".into());
        let draft = GraphDocument {
            nodes: vec![body],
            ..Default::default()
        };
        let sibling = definitions.instantiate("event", [400., 30.]).unwrap();
        let sibling_id = sibling.id;
        let mut project = Project::default();
        project.node_graphs.insert("behavior/world".into(),serde_json::json!({"version":1,"branches":[GraphDocument {nodes:vec![sibling],..Default::default()},draft]}));
        let key = NodeSourceKey {
            owner: "behavior/world".into(),
            branch: None,
        };
        let mut workspace = NodeGraphWorkspace::default();
        workspace
            .load(&project, key.clone(), &definitions, false)
            .unwrap();
        workspace.bind(&key, 10).unwrap();
        (project, definitions, workspace, key, draft, sibling_id)
    }
    fn promoted(
        defs: &GraphDefinitions,
        key: &NodeSourceKey,
        draft: &GraphDocument,
    ) -> (NodeSourceKey, GraphDocument) {
        let root = defs.instantiate("event", [10., 15.]).unwrap();
        let key = NodeSourceKey {
            owner: key.owner.clone(),
            branch: Some(root.id),
        };
        let mut doc = draft.clone();
        doc.nodes.insert(0, root);
        for node in &mut doc.nodes {
            node.branch = key.branch;
        }
        (key, doc)
    }
    #[test]
    fn node_transaction_promotes_private_draft_and_replays_exact_ids_without_restoring_siblings() {
        let (mut project, defs, mut workspace, draft_key, mut draft, sibling) = fixture();
        draft.nodes[0].position = [123.5, 456.25];
        workspace
            .set_working_document(&draft_key, draft.clone())
            .unwrap();
        let (branch, document) = promoted(&defs, &draft_key, &draft);
        let (mut transaction, applied) = workspace
            .transition_document(
                &mut project,
                &draft_key,
                &branch,
                document.clone(),
                &defs,
                |_, _, _, _| None,
            )
            .unwrap();
        assert_eq!(transaction.patches.len(), 2);
        assert_eq!(applied.changes.len(), 2);
        assert!(workspace.binding_source(10).is_none());
        workspace.bind(&branch, 20).unwrap();
        let graphs = workspace.graphs(&project, &branch.owner, &defs).unwrap();
        assert!(graphs.draft.nodes.is_empty());
        assert_eq!(graphs.document(branch.branch), Some(&document));
        let loaded: Project =
            serde_json::from_value(serde_json::to_value(&project).unwrap()).unwrap();
        let mut fresh = NodeGraphWorkspace::default();
        assert_eq!(
            fresh
                .load(&loaded, branch.clone(), &defs, false)
                .unwrap()
                .document(),
            &document
        );
        project.name = "Unrelated owner change".into();
        for graph in project.node_graphs[&branch.owner]["branches"]
            .as_array_mut()
            .unwrap()
        {
            if graph["nodes"][0]["id"] == serde_json::json!(sibling) {
                graph["nodes"][0]["title"] = "New sibling".into();
            }
        }
        workspace
            .replay_transaction(
                &mut project,
                &mut transaction,
                false,
                &defs,
                |_, _, _, _| None,
            )
            .unwrap();
        assert!(workspace.binding_source(20).is_none());
        let graphs = workspace.graphs(&project, &branch.owner, &defs).unwrap();
        assert_eq!(graphs.draft, draft);
        assert!(!graphs.branches.contains_key(&branch.branch.unwrap()));
        assert_eq!(graphs.branches[&sibling].nodes[0].title, "New sibling");
        assert_eq!(project.name, "Unrelated owner change");
        workspace
            .replay_transaction(&mut project, &mut transaction, true, &defs, |_, _, _, _| {
                None
            })
            .unwrap();
        let graphs = workspace.graphs(&project, &branch.owner, &defs).unwrap();
        assert!(graphs.draft.nodes.is_empty());
        assert_eq!(graphs.document(branch.branch), Some(&document));
        assert_eq!(graphs.branches[&sibling].nodes[0].title, "New sibling");
        assert_eq!(
            workspace
                .apply_commit(
                    &mut project,
                    &GraphCommit::new(10, 1, vec![]),
                    &defs,
                    |_, _, _| None
                )
                .unwrap_err(),
            NodeSourceError::UnknownBinding
        );
    }
    #[test]
    fn node_transaction_undo_rejects_an_independent_draft_without_partial_source_or_project_writes()
    {
        let (mut project, defs, mut workspace, key, draft, _) = fixture();
        let (branch, doc) = promoted(&defs, &key, &draft);
        let (mut transaction, _) = workspace
            .transition_document(&mut project, &key, &branch, doc, &defs, |_, _, _, _| None)
            .unwrap();
        let mut independent = draft.clone();
        independent.nodes[0].id = GraphId::new_v4();
        project.node_graphs[&key.owner]["branches"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::to_value(independent).unwrap());
        workspace.bind(&branch, 30).unwrap();
        let before = serde_json::to_value(&project).unwrap();
        let source = workspace.source(&branch).unwrap().document().clone();
        let old = transaction.clone();
        assert_eq!(
            workspace
                .replay_transaction(
                    &mut project,
                    &mut transaction,
                    false,
                    &defs,
                    |_, _, _, _| None
                )
                .unwrap_err(),
            NodeSourceError::SourceChanged
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
        assert_eq!(workspace.source(&branch).unwrap().document(), &source);
        assert_eq!(workspace.binding_source(30), Some(&branch));
        assert_eq!(transaction, old);
    }
    #[test]
    fn node_transaction_rejects_bad_projection_multiple_roots_and_occupied_destinations_atomically()
    {
        for mode in 0..3 {
            let (mut project, defs, mut workspace, key, draft, sibling) = fixture();
            let (mut branch, mut doc) = promoted(&defs, &key, &draft);
            if mode == 1 {
                doc.nodes
                    .push(defs.instantiate("event", [500., 60.]).unwrap());
            }
            if mode == 2 {
                branch.branch = Some(sibling);
            }
            let before = serde_json::to_value(&project).unwrap();
            let source = workspace.source(&key).unwrap().document().clone();
            assert!(
                workspace
                    .transition_document(&mut project, &key, &branch, doc, &defs, |_, doc, _, _| {
                        if mode == 0 {
                            doc.version = 99;
                        }
                        None
                    })
                    .is_err()
            );
            assert_eq!(serde_json::to_value(&project).unwrap(), before);
            assert_eq!(workspace.source(&key).unwrap().document(), &source);
            assert_eq!(workspace.binding_source(10), Some(&key));
        }
    }
    #[test]
    fn node_transaction_noop_does_not_materialize_absent_source_or_generated_entity_data() {
        let mut project = Project::default();
        let key = NodeSourceKey {
            owner: "behavior/world".into(),
            branch: None,
        };
        let defs = GraphDefinitions::default();
        let mut workspace = NodeGraphWorkspace::default();
        workspace.load(&project, key.clone(), &defs, false).unwrap();
        workspace.bind(&key, 55).unwrap();
        let before = serde_json::to_value(&project).unwrap();
        let (transaction, outcome) = workspace
            .transition_document(
                &mut project,
                &key,
                &key,
                GraphDocument::default(),
                &defs,
                |_, _, _, _| None,
            )
            .unwrap();
        assert!(transaction.is_noop());
        assert_eq!(workspace.binding_source(55), Some(&key));
        assert!(outcome.changes.is_empty());
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
        let mut character = crate::character::Character::new();
        character.data = "[attributes]\nrace='Human'\nclass='Warrior'".into();
        let id = character.id;
        project.characters.insert(id, character);
        let key = NodeSourceKey {
            owner: entity_graph::character_key(id),
            branch: None,
        };
        let doc = workspace
            .load(&project, key.clone(), &defs, false)
            .unwrap()
            .document()
            .clone();
        let before = serde_json::to_value(&project).unwrap();
        let (transaction, outcome) = workspace
            .transition_document(&mut project, &key, &key, doc, &defs, |_, _, _, _| None)
            .unwrap();
        assert!(transaction.is_noop());
        assert!(outcome.changes.is_empty());
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }
    #[test]
    fn node_transaction_history_refreshes_projected_configuration_and_preserves_invalid_authoring_source()
     {
        let mut project = Project::default();
        let mut character = crate::character::Character::new();
        character.data = "[attributes]\nrace='Human'\nclass='Warrior'".into();
        let id = character.id;
        project.characters.insert(id, character);
        let key = NodeSourceKey {
            owner: entity_graph::character_key(id),
            branch: None,
        };
        let defs = GraphDefinitions::default();
        let mut workspace = NodeGraphWorkspace::default();
        let original = workspace
            .load(&project, key.clone(), &defs, false)
            .unwrap()
            .document()
            .clone();
        let generated = project.characters[&id].data.clone();
        let mut invalid = original.clone();
        invalid
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("entity"))
            .unwrap()
            .rows
            .iter_mut()
            .find(|r| r.key.as_deref() == Some("race"))
            .unwrap()
            .value = GraphControlValue::Text("missing race".into());
        let projection =
            |_: &NodeSourceKey, doc: &mut GraphDocument, _: &GraphDocument, project: &Project| {
                let node = &mut doc.nodes[0];
                let value = GraphControlValue::Label(project.name.clone());
                if let Some(row) = node
                    .rows
                    .iter_mut()
                    .find(|row| row.key.as_deref() == Some("projection"))
                {
                    row.value = value;
                } else {
                    let mut row = GraphRow::new("Projection", value);
                    row.id = schema_graph_id(node.id, "row", "projection");
                    row.key = Some("projection".into());
                    node.rows.push(row);
                }
                None
            };
        let (mut transaction, outcome) = workspace
            .transition_document(&mut project, &key, &key, invalid, &defs, projection)
            .unwrap();
        assert_eq!(outcome.validation.len(), 1);
        assert_eq!(project.characters[&id].data, generated);
        project.name = "Current metadata".into();
        workspace
            .replay_transaction(&mut project, &mut transaction, false, &defs, projection)
            .unwrap();
        assert_eq!(workspace.source(&key).unwrap().validation(), None);
        workspace
            .replay_transaction(&mut project, &mut transaction, true, &defs, projection)
            .unwrap();
        assert!(workspace.source(&key).unwrap().validation().is_some());
        assert_eq!(project.name, "Current metadata");
        let doc: GraphDocument =
            serde_json::from_value(project.node_graphs[&key.owner].clone()).unwrap();
        assert!(
            doc.nodes[0]
                .rows
                .iter()
                .any(|row| row.key.as_deref() == Some("projection")
                    && row.value == GraphControlValue::Label("Current metadata".into()))
        );
    }
    #[test]
    fn node_transaction_restores_relative_branch_position_without_reordering_new_siblings() {
        let (mut project, defs, mut workspace, _, _, sibling) = fixture();
        let mut graph = NodeGraphDocuments::from_value(
            &project.node_graphs["behavior/world"],
            NodeGraphKind::Behavior,
            &defs,
        )
        .unwrap();
        let branch = defs.instantiate("event", [40., 50.]).unwrap();
        let id = branch.id;
        let last = defs.instantiate("event", [80., 50.]).unwrap();
        let last_id = last.id;
        graph.replace(
            Some(id),
            GraphDocument {
                nodes: vec![branch],
                ..Default::default()
            },
        );
        graph.replace(
            Some(last_id),
            GraphDocument {
                nodes: vec![last],
                ..Default::default()
            },
        );
        project
            .node_graphs
            .insert("behavior/world".into(), graph.to_value());
        let key = NodeSourceKey {
            owner: "behavior/world".into(),
            branch: Some(id),
        };
        workspace.load(&project, key.clone(), &defs, false).unwrap();
        let (mut transaction, _) = workspace
            .transition_document(
                &mut project,
                &key,
                &key,
                GraphDocument::default(),
                &defs,
                |_, _, _, _| None,
            )
            .unwrap();
        let mut graph = workspace.graphs(&project, &key.owner, &defs).unwrap();
        let new = defs.instantiate("event", [120., 50.]).unwrap();
        let new_id = new.id;
        graph.replace(
            Some(new_id),
            GraphDocument {
                nodes: vec![new],
                ..Default::default()
            },
        );
        graph.order = vec![new_id, sibling, last_id];
        project
            .node_graphs
            .insert(key.owner.clone(), graph.to_value());
        workspace
            .replay_transaction(
                &mut project,
                &mut transaction,
                false,
                &defs,
                |_, _, _, _| None,
            )
            .unwrap();
        assert_eq!(
            workspace.graphs(&project, &key.owner, &defs).unwrap().order,
            vec![new_id, sibling, id, last_id]
        );
        workspace
            .replay_transaction(&mut project, &mut transaction, true, &defs, |_, _, _, _| {
                None
            })
            .unwrap();
        assert_eq!(
            workspace.graphs(&project, &key.owner, &defs).unwrap().order,
            vec![new_id, sibling, last_id]
        );
    }
}
