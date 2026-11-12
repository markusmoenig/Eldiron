//! Retained CPU sources for behavior and entity configuration graphs.
//! Project persistence/validation is independent of view selection and rendering.
use super::*;
use crate::{entity_graph, project::Project, rulesets};
use std::collections::HashMap;
use theframework::thegraph::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeSourceKey {
    pub owner: String,
    pub branch: Option<GraphId>,
}
impl NodeSourceKey {
    pub fn kind(&self) -> NodeGraphKind {
        if self.owner == rulesets::graph::OWNER {
            NodeGraphKind::Rules
        } else if self.owner.starts_with(entity_graph::PREFIX) {
            NodeGraphKind::Configuration
        } else {
            NodeGraphKind::Behavior
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeSourceError {
    MissingOwner,
    MissingBranch,
    InvalidSource(String),
    SourceChanged,
    UnknownBinding,
    WrongBinding,
    InvalidOrder,
}
impl std::fmt::Display for NodeSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MissingOwner => "The node graph owner is no longer available",
            Self::MissingBranch => "The node graph branch is no longer available",
            Self::InvalidSource(message) => message,
            Self::SourceChanged => "The node graph source changed outside this draft",
            Self::UnknownBinding | Self::WrongBinding => "The node edit belongs to another source",
            Self::InvalidOrder => "Node edits arrived out of order",
        })
    }
}
impl std::error::Error for NodeSourceError {}
#[derive(Clone, Debug)]
pub struct NodeAuthoringSource {
    document: GraphDocument,
    stored: Option<GraphDocument>,
    revision: u64,
    error: Option<NodeSourceError>,
    validation: Option<String>,
}
impl NodeAuthoringSource {
    pub fn document(&self) -> &GraphDocument {
        &self.document
    }
    pub fn stored(&self) -> Option<&GraphDocument> {
        self.stored.as_ref()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn error(&self) -> Option<&NodeSourceError> {
        self.error.as_ref()
    }
    pub fn validation(&self) -> Option<&str> {
        self.validation.as_deref()
    }
}
#[derive(Clone, Debug)]
pub struct NodeSourceChange {
    pub key: NodeSourceKey,
    pub before: GraphDocument,
    pub after: GraphDocument,
}
#[derive(Debug)]
pub struct NodeSourceApply {
    pub key: NodeSourceKey,
    pub adopted: bool,
    pub change: Option<NodeSourceChange>,
    pub error: Option<NodeSourceError>,
    pub validation: Option<String>,
}
#[derive(PartialEq)]
struct Fingerprint {
    graph: Option<serde_json::Value>,
    imported_data: Option<String>,
}
struct OwnerSource {
    fingerprint: Fingerprint,
    graphs: NodeGraphDocuments,
}
struct Binding {
    key: NodeSourceKey,
    serial: u64,
}
#[derive(Default)]
pub struct NodeGraphWorkspace {
    owners: HashMap<String, OwnerSource>,
    sources: HashMap<NodeSourceKey, NodeAuthoringSource>,
    bindings: HashMap<u64, Binding>,
    revision: u64,
    sequence: u64,
}
/// Check the actual target rather than allowing a stale edit to recreate an
/// authoring entry after its character/item/region/area was deleted.
pub fn node_owner_exists(project: &Project, owner: &str) -> bool {
    let id = |text: &str| GraphId::parse_str(text).ok();
    match owner.split('/').collect::<Vec<_>>().as_slice() {
        ["rules", "game"] | ["behavior", "world"] => true,
        ["entity", "character", value] | ["entity", "item", value] => {
            id(value).is_some() && entity_graph::owner_data(project, owner).is_some()
        }
        ["behavior", "character", value] => {
            id(value).is_some_and(|id| project.characters.contains_key(&id))
        }
        ["behavior", "item", value] => id(value).is_some_and(|id| project.items.contains_key(&id)),
        ["behavior", "region", region] => {
            id(region).is_some_and(|id| project.get_region(&id).is_some())
        }
        ["behavior", "region", region, kind, value] => id(region)
            .and_then(|id| project.get_region(&id))
            .is_some_and(|region| {
                id(value).is_some_and(|id| match *kind {
                    "character" => region.characters.contains_key(&id),
                    "item" => region.items.contains_key(&id),
                    "area" => {
                        region
                            .map
                            .sectors
                            .iter()
                            .any(|sector| sector.creator_id == id)
                            || region
                                .map
                                .geometry_objects
                                .iter()
                                .any(|object| object.id == id)
                    }
                    _ => false,
                })
            }),
        _ => false,
    }
}
impl NodeGraphWorkspace {
    fn fingerprint(project: &Project, owner: &str, kind: NodeGraphKind) -> Fingerprint {
        let graph = project.node_graphs.get(owner).cloned();
        let imported_data = if graph.is_none() && kind == NodeGraphKind::Configuration {
            entity_graph::owner_data(project, owner).map(str::to_owned)
        } else {
            None
        };
        Fingerprint {
            graph,
            imported_data,
        }
    }
    pub fn graphs(
        &mut self,
        project: &Project,
        owner: &str,
        definitions: &GraphDefinitions,
    ) -> Result<NodeGraphDocuments, NodeSourceError> {
        if !node_owner_exists(project, owner) {
            return Err(NodeSourceError::MissingOwner);
        }
        let key = NodeSourceKey {
            owner: owner.into(),
            branch: None,
        };
        let kind = key.kind();
        // The catalogue's RulesState is the persisted/runtime authority. The
        // old RulesGraph is serde-skipped compatibility state, not a source
        // that this graph workspace may edit or silently convert.
        if kind == NodeGraphKind::Rules {
            return Err(NodeSourceError::InvalidSource(
                "Game rules belong to the rules document catalogue, not the node workspace".into(),
            ));
        }
        let fingerprint = Self::fingerprint(project, owner, kind);
        if let Some(cached) = self
            .owners
            .get(owner)
            .filter(|cached| cached.fingerprint == fingerprint)
        {
            return Ok(cached.graphs.clone());
        }
        let mut graphs = if let Some(value) = &fingerprint.graph {
            NodeGraphDocuments::from_value(value, kind, definitions)
                .map_err(NodeSourceError::InvalidSource)?
        } else {
            NodeGraphDocuments::empty(kind)
        };
        if kind == NodeGraphKind::Configuration && fingerprint.graph.is_none() {
            let rules = project
                .rules_source()
                .unwrap_or_default()
                .parse::<entity_graph::RulesTable>()
                .unwrap_or_default();
            let data = fingerprint.imported_data.as_deref().unwrap_or("");
            let id = GraphId::parse_str(owner.rsplit('/').next().unwrap_or("")).unwrap();
            graphs.draft =
                if owner.starts_with("entity/character/") && project.characters.contains_key(&id) {
                    entity_graph::import_character(data, &rules)
                } else if owner.starts_with("entity/item/") && project.items.contains_key(&id) {
                    entity_graph::import_item(data, &rules)
                } else {
                    entity_graph::import(data, &rules)
                }
                .map_err(NodeSourceError::InvalidSource)?;
            entity_graph::normalize(&mut graphs.draft);
        }
        self.owners.insert(
            owner.into(),
            OwnerSource {
                fingerprint,
                graphs: graphs.clone(),
            },
        );
        Ok(graphs)
    }
    pub fn source(&self, key: &NodeSourceKey) -> Option<&NodeAuthoringSource> {
        self.sources.get(key)
    }
    pub fn diagnostics(&self) -> impl Iterator<Item = (&NodeSourceKey, &NodeSourceError)> {
        self.sources
            .iter()
            .filter_map(|(key, source)| source.error.as_ref().map(|error| (key, error)))
    }
    pub fn load(
        &mut self,
        project: &Project,
        key: NodeSourceKey,
        definitions: &GraphDefinitions,
        preserve: bool,
    ) -> Result<&NodeAuthoringSource, NodeSourceError> {
        let graphs = self.graphs(project, &key.owner, definitions)?;
        let canonical = graphs
            .document(key.branch)
            .cloned()
            .ok_or(NodeSourceError::MissingBranch)?;
        let changed = self
            .sources
            .get(&key)
            .is_none_or(|source| source.stored.as_ref() != Some(&canonical));
        if changed {
            if preserve && self.sources.contains_key(&key) {
                self.sources.get_mut(&key).unwrap().error = Some(NodeSourceError::SourceChanged);
            } else {
                self.revision = self
                    .revision
                    .checked_add(1)
                    .expect("node source revision exhausted");
                self.sources.insert(
                    key.clone(),
                    NodeAuthoringSource {
                        document: canonical.clone(),
                        stored: Some(canonical),
                        revision: self.revision,
                        error: None,
                        validation: None,
                    },
                );
                self.bindings.retain(|_, binding| binding.key != key);
            }
        }
        Ok(&self.sources[&key])
    }
    /// Explicit new-branch operation; arbitrary stale packets cannot create it.
    pub fn create_branch(
        &mut self,
        project: &Project,
        key: NodeSourceKey,
        definitions: &GraphDefinitions,
    ) -> Result<(), NodeSourceError> {
        let graphs = self.graphs(project, &key.owner, definitions)?;
        if key.kind() == NodeGraphKind::Configuration
            || key.branch.is_none()
            || graphs.document(key.branch).is_some()
            || self.sources.contains_key(&key)
        {
            return Err(NodeSourceError::SourceChanged);
        }
        self.revision = self
            .revision
            .checked_add(1)
            .expect("node source revision exhausted");
        self.sources.insert(
            key,
            NodeAuthoringSource {
                document: GraphDocument::default(),
                stored: None,
                revision: self.revision,
                error: None,
                validation: None,
            },
        );
        Ok(())
    }
    pub fn set_working_document(
        &mut self,
        key: &NodeSourceKey,
        document: GraphDocument,
    ) -> Result<(), NodeSourceError> {
        self.sources
            .get_mut(key)
            .ok_or(NodeSourceError::MissingBranch)?
            .document = document;
        Ok(())
    }
    /// Refresh private schema/diagnostics without authoring a Project change.
    /// Views run the same projection on their possibly newer working document.
    pub fn project_source<F>(
        &mut self,
        project: &Project,
        key: &NodeSourceKey,
        projection: F,
    ) -> Result<(), NodeSourceError>
    where
        F: FnOnce(&mut GraphDocument, &GraphDocument, &Project) -> Option<String>,
    {
        let source = self
            .sources
            .get_mut(key)
            .ok_or(NodeSourceError::MissingBranch)?;
        let baseline = source
            .stored
            .clone()
            .unwrap_or_else(|| source.document.clone());
        source.validation = projection(&mut source.document, &baseline, project);
        Ok(())
    }
    pub fn bind(&mut self, key: &NodeSourceKey, binding: u64) -> Result<(), NodeSourceError> {
        if !self.sources.contains_key(key) {
            return Err(NodeSourceError::MissingBranch);
        }
        if let Some(previous) = self.bindings.get(&binding) {
            return if previous.key == *key {
                Ok(())
            } else {
                Err(NodeSourceError::WrongBinding)
            };
        }
        self.bindings.insert(
            binding,
            Binding {
                key: key.clone(),
                serial: 0,
            },
        );
        Ok(())
    }
    pub fn unbind(&mut self, binding: u64) {
        self.bindings.remove(&binding);
    }
    pub fn binding_source(&self, binding: u64) -> Option<&NodeSourceKey> {
        self.bindings.get(&binding).map(|binding| &binding.key)
    }
    pub fn apply_commit<F>(
        &mut self,
        project: &mut Project,
        commit: &GraphCommit,
        definitions: &GraphDefinitions,
        project_graph: F,
    ) -> Result<NodeSourceApply, NodeSourceError>
    where
        F: FnOnce(&mut GraphDocument, &GraphDocument, &Project) -> Option<String>,
    {
        let binding = self
            .bindings
            .get(&commit.binding)
            .ok_or(NodeSourceError::UnknownBinding)?;
        let key = binding.key.clone();
        if commit.serial == 0 {
            return Err(NodeSourceError::InvalidOrder);
        }
        if commit.serial <= binding.serial {
            return Ok(self.outcome(key, false, None));
        }
        if commit.serial != binding.serial + 1 || commit.sequence <= self.sequence {
            return Err(NodeSourceError::InvalidOrder);
        }
        let graphs = self.graphs(project, &key.owner, definitions)?;
        let source = self
            .sources
            .get_mut(&key)
            .ok_or(NodeSourceError::MissingBranch)?;
        let before = source.document.clone();
        for edit in &commit.edits {
            edit.apply(&mut source.document, true);
        }
        self.bindings.get_mut(&commit.binding).unwrap().serial = commit.serial;
        self.sequence = commit.sequence;
        self.persist(project, key, graphs, before, project_graph)
    }
    pub fn commit_document<F>(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        document: GraphDocument,
        definitions: &GraphDefinitions,
        project_graph: F,
    ) -> Result<NodeSourceApply, NodeSourceError>
    where
        F: FnOnce(&mut GraphDocument, &GraphDocument, &Project) -> Option<String>,
    {
        let graphs = self.graphs(project, &key.owner, definitions)?;
        let source = self
            .sources
            .get_mut(key)
            .ok_or(NodeSourceError::MissingBranch)?;
        let before = std::mem::replace(&mut source.document, document);
        let outcome = self.persist(project, key.clone(), graphs, before, project_graph)?;
        // A completed domain replacement starts a fresh view epoch. Callers
        // drain prior packets first, replace their model, then bind the new one.
        self.bindings.retain(|_, binding| &binding.key != key);
        Ok(outcome)
    }
    fn persist<F>(
        &mut self,
        project: &mut Project,
        key: NodeSourceKey,
        mut graphs: NodeGraphDocuments,
        before: GraphDocument,
        project_graph: F,
    ) -> Result<NodeSourceApply, NodeSourceError>
    where
        F: FnOnce(&mut GraphDocument, &GraphDocument, &Project) -> Option<String>,
    {
        let source = self
            .sources
            .get_mut(&key)
            .ok_or(NodeSourceError::MissingBranch)?;
        if graphs.document(key.branch) != source.stored.as_ref() {
            source.error = Some(NodeSourceError::SourceChanged);
            return Ok(self.outcome(key, true, None));
        }
        let previous = source.stored.as_ref().unwrap_or(&before);
        let validation = project_graph(&mut source.document, previous, project);
        if source.stored.as_ref() == Some(&source.document)
            || (source.stored.is_none()
                && source.document.nodes.is_empty()
                && source.document.connections.is_empty())
        {
            source.error = None;
            source.validation = match key.kind() {
                NodeGraphKind::Configuration => {
                    entity_graph::project_owner_data(project, &key.owner, &source.document)
                        .err()
                        .or(validation)
                }
                NodeGraphKind::Rules => unreachable!("rules catalogue has its own source owner"),
                NodeGraphKind::Behavior => validation,
            };
            return Ok(self.outcome(key, true, None));
        }
        if source.document.version != 1 {
            source.error = Some(NodeSourceError::InvalidSource(
                "Unsupported node graph document version".into(),
            ));
            return Ok(self.outcome(key, true, None));
        }
        graphs.replace(key.branch, source.document.clone());
        if key.branch.is_some() && source.document.nodes.is_empty() {
            let root = key.branch.unwrap();
            graphs.branches.remove(&root);
            graphs.order.retain(|id| *id != root);
        }
        let value = graphs.to_value();
        let validation = match key.kind() {
            NodeGraphKind::Rules => unreachable!("rules catalogue has its own source owner"),
            NodeGraphKind::Configuration => {
                project.node_graphs.insert(key.owner.clone(), value);
                entity_graph::update_owner(project, &key.owner, &source.document)
                    .err()
                    .or(validation)
            }
            NodeGraphKind::Behavior => {
                project.node_graphs.insert(key.owner.clone(), value);
                validation
            }
        };
        source.stored = graphs.document(key.branch).cloned();
        source.error = None;
        source.validation = validation;
        let change = NodeSourceChange {
            key: key.clone(),
            before,
            after: source.document.clone(),
        };
        self.owners.insert(
            key.owner.clone(),
            OwnerSource {
                fingerprint: Self::fingerprint(project, &key.owner, key.kind()),
                graphs,
            },
        );
        Ok(self.outcome(key, true, Some(change)))
    }
    fn outcome(
        &self,
        key: NodeSourceKey,
        adopted: bool,
        change: Option<NodeSourceChange>,
    ) -> NodeSourceApply {
        let source = self.sources.get(&key);
        NodeSourceApply {
            adopted,
            change,
            error: source.and_then(|source| source.error.clone()),
            validation: source.and_then(|source| source.validation.clone()),
            key,
        }
    }
    pub fn remove(&mut self, key: &NodeSourceKey) {
        self.sources.remove(key);
        self.bindings.retain(|_, binding| &binding.key != key);
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definitions() -> GraphDefinitions {
        let mut definitions = GraphDefinitions::default();
        let mut node = GraphNode::new("Trigger", [10., 20.], [50, 80, 110, 255]);
        node.rows.push(GraphRow::new(
            "Text",
            GraphControlValue::Text("Before".into()),
        ));
        definitions
            .register_node(GraphNodeDefinition::from_template("event", "Events", &node))
            .unwrap();
        definitions
    }
    fn fixture() -> (
        Project,
        GraphDefinitions,
        NodeGraphWorkspace,
        NodeSourceKey,
        NodeSourceKey,
        GraphId,
    ) {
        let mut project = Project::default();
        let definitions = definitions();
        let character = crate::character::Character::new();
        let owner = format!("behavior/character/{}", character.id);
        project.characters.insert(character.id, character);
        let a = definitions.instantiate("event", [10., 20.]).unwrap();
        let sibling = definitions.instantiate("event", [30., 40.]).unwrap();
        let b = definitions.instantiate("event", [50., 60.]).unwrap();
        let (aid, sid, bid) = (a.id, sibling.id, b.id);
        let doc = |node| GraphDocument {
            nodes: vec![node],
            ..Default::default()
        };
        project.node_graphs.insert(
            "behavior/world".into(),
            serde_json::json!({"version":1,"branches":[doc(a),doc(sibling)]}),
        );
        project.node_graphs.insert(
            owner.clone(),
            serde_json::json!({"version":1,"branches":[doc(b)]}),
        );
        let a = NodeSourceKey {
            owner: "behavior/world".into(),
            branch: Some(aid),
        };
        let b = NodeSourceKey {
            owner,
            branch: Some(bid),
        };
        let mut workspace = NodeGraphWorkspace::default();
        workspace
            .load(&project, a.clone(), &definitions, false)
            .unwrap();
        workspace.bind(&a, 11).unwrap();
        workspace
            .load(&project, b.clone(), &definitions, false)
            .unwrap();
        workspace.bind(&b, 22).unwrap();
        (project, definitions, workspace, a, b, sid)
    }
    fn packet(
        workspace: &NodeGraphWorkspace,
        key: &NodeSourceKey,
        binding: u64,
        serial: u64,
        before: &str,
        after: &str,
    ) -> GraphCommit {
        let node = &workspace.source(key).unwrap().document().nodes[0];
        GraphCommit::new(
            binding,
            serial,
            vec![GraphEdit::SetValue {
                node: node.id,
                row: node.rows[0].id,
                before: GraphControlValue::Text(before.into()),
                after: GraphControlValue::Text(after.into()),
            }],
        )
    }
    fn value(workspace: &NodeGraphWorkspace, key: &NodeSourceKey) -> GraphControlValue {
        workspace.source(key).unwrap().document().nodes[0].rows[0]
            .value
            .clone()
    }
    #[test]
    fn node_source_workspace_orders_owners_and_preserves_current_sibling_branches_through_undo() {
        fn shared<T: Send + Sync>() {}
        shared::<NodeGraphWorkspace>();
        let (mut project, defs, mut workspace, a, b, sibling) = fixture();
        let a0 = workspace.source(&a).unwrap().document().clone();
        let b0 = workspace.source(&b).unwrap().document().clone();
        let a1 = packet(&workspace, &a, 11, 1, "Before", "A1 東京\r\n");
        let b1 = packet(&workspace, &b, 22, 1, "Before", "B1");
        let a2 = packet(&workspace, &a, 11, 2, "A1 東京\r\n", "A2");
        let mut changes = vec![];
        changes.push(
            workspace
                .apply_commit(&mut project, &a1, &defs, |_, _, _| None)
                .unwrap()
                .change
                .unwrap(),
        );
        let mut current = NodeGraphDocuments::from_value(
            &project.node_graphs[&a.owner],
            NodeGraphKind::Behavior,
            &defs,
        )
        .unwrap();
        current.branches.get_mut(&sibling).unwrap().nodes[0].title = "External sibling".into();
        project
            .node_graphs
            .insert(a.owner.clone(), current.to_value());
        project.name = "Independent project change".into();
        changes.push(
            workspace
                .apply_commit(&mut project, &b1, &defs, |_, _, _| None)
                .unwrap()
                .change
                .unwrap(),
        );
        changes.push(
            workspace
                .apply_commit(&mut project, &a2, &defs, |_, _, _| None)
                .unwrap()
                .change
                .unwrap(),
        );
        assert_eq!(value(&workspace, &a), GraphControlValue::Text("A2".into()));
        let exact = serde_json::to_value(&project.node_graphs).unwrap();
        assert!(
            !workspace
                .apply_commit(&mut project, &a1, &defs, |_, _, _| None)
                .unwrap()
                .adopted
        );
        assert_eq!(serde_json::to_value(&project.node_graphs).unwrap(), exact);
        for change in changes.iter().rev() {
            workspace
                .commit_document(
                    &mut project,
                    &change.key,
                    change.before.clone(),
                    &defs,
                    |_, _, _| None,
                )
                .unwrap();
        }
        assert_eq!(workspace.source(&a).unwrap().document(), &a0);
        assert_eq!(workspace.source(&b).unwrap().document(), &b0);
        assert_eq!(
            workspace
                .graphs(&project, &a.owner, &defs)
                .unwrap()
                .branches[&sibling]
                .nodes[0]
                .title,
            "External sibling"
        );
        assert_eq!(project.name, "Independent project change");
        for change in &changes {
            workspace
                .commit_document(
                    &mut project,
                    &change.key,
                    change.after.clone(),
                    &defs,
                    |_, _, _| None,
                )
                .unwrap();
        }
        assert_eq!(serde_json::to_value(&project.node_graphs).unwrap(), exact);
    }
    #[test]
    fn node_source_workspace_preserves_conflicting_private_packets_until_explicit_clean_reload() {
        let (mut project, defs, mut workspace, a, _, _) = fixture();
        let pending = packet(&workspace, &a, 11, 1, "Before", "Private draft");
        let revision = workspace.source(&a).unwrap().revision();
        let mut external = workspace.graphs(&project, &a.owner, &defs).unwrap();
        external.branches.get_mut(&a.branch.unwrap()).unwrap().nodes[0].rows[0].value =
            GraphControlValue::Text("External source".into());
        project
            .node_graphs
            .insert(a.owner.clone(), external.to_value());
        let exact = project.node_graphs.clone();
        let outcome = workspace
            .apply_commit(&mut project, &pending, &defs, |_, _, _| None)
            .unwrap();
        assert!(outcome.adopted && outcome.change.is_none());
        assert_eq!(outcome.error, Some(NodeSourceError::SourceChanged));
        assert_eq!(
            value(&workspace, &a),
            GraphControlValue::Text("Private draft".into())
        );
        assert_eq!(project.node_graphs, exact);
        workspace.load(&project, a.clone(), &defs, true).unwrap();
        assert_eq!(workspace.source(&a).unwrap().revision(), revision);
        assert_eq!(workspace.diagnostics().count(), 1);
        workspace.load(&project, a.clone(), &defs, false).unwrap();
        assert_ne!(workspace.source(&a).unwrap().revision(), revision);
        assert_eq!(
            value(&workspace, &a),
            GraphControlValue::Text("External source".into())
        );
        assert_eq!(
            workspace
                .apply_commit(&mut project, &pending, &defs, |_, _, _| None)
                .unwrap_err(),
            NodeSourceError::UnknownBinding
        );
    }
    #[test]
    fn node_source_workspace_rejects_serial_gaps_deleted_owners_and_obsolete_publication_without_project_mutation()
     {
        let (mut project, defs, mut workspace, a, b, _) = fixture();
        let gap = packet(&workspace, &a, 11, 2, "Before", "Gap");
        let before = project.node_graphs.clone();
        assert_eq!(
            workspace
                .apply_commit(&mut project, &gap, &defs, |_, _, _| None)
                .unwrap_err(),
            NodeSourceError::InvalidOrder
        );
        assert_eq!(project.node_graphs, before);
        let older = packet(&workspace, &a, 11, 1, "Before", "Old");
        let newer = packet(&workspace, &b, 22, 1, "Before", "New");
        workspace
            .apply_commit(&mut project, &newer, &defs, |_, _, _| None)
            .unwrap();
        let before = project.node_graphs.clone();
        assert_eq!(
            workspace
                .apply_commit(&mut project, &older, &defs, |_, _, _| None)
                .unwrap_err(),
            NodeSourceError::InvalidOrder
        );
        assert_eq!(project.node_graphs, before);
        let deleted = packet(&workspace, &b, 22, 2, "New", "Deleted");
        project.characters.clear();
        project.node_graphs.shift_remove(&b.owner);
        assert_eq!(
            workspace
                .apply_commit(&mut project, &deleted, &defs, |_, _, _| None)
                .unwrap_err(),
            NodeSourceError::MissingOwner
        );
        assert!(!project.node_graphs.contains_key(&b.owner));
    }
    #[test]
    fn node_source_workspace_explicit_branch_create_delete_and_restore_revoke_old_bindings() {
        let mut project = Project::default();
        let defs = definitions();
        let mut workspace = NodeGraphWorkspace::default();
        let node = defs.instantiate("event", [20., 30.]).unwrap();
        let key = NodeSourceKey {
            owner: "behavior/world".into(),
            branch: Some(node.id),
        };
        workspace
            .create_branch(&project, key.clone(), &defs)
            .unwrap();
        workspace.bind(&key, 77).unwrap();
        assert!(project.node_graphs.is_empty());
        let add = GraphCommit::new(77, 1, vec![GraphEdit::AddNode { node }]);
        let after = workspace
            .apply_commit(&mut project, &add, &defs, |_, _, _| None)
            .unwrap()
            .change
            .unwrap()
            .after;
        let stale = GraphCommit::new(
            77,
            2,
            vec![GraphEdit::Move {
                node: key.branch.unwrap(),
                before: [20., 30.],
                after: [80., 90.],
            }],
        );
        workspace
            .commit_document(
                &mut project,
                &key,
                GraphDocument::default(),
                &defs,
                |_, _, _| None,
            )
            .unwrap();
        assert!(
            workspace
                .graphs(&project, &key.owner, &defs)
                .unwrap()
                .order
                .is_empty()
        );
        assert_eq!(
            workspace
                .apply_commit(&mut project, &stale, &defs, |_, _, _| None)
                .unwrap_err(),
            NodeSourceError::UnknownBinding
        );
        workspace
            .commit_document(&mut project, &key, after.clone(), &defs, |_, _, _| None)
            .unwrap();
        assert_eq!(
            workspace
                .graphs(&project, &key.owner, &defs)
                .unwrap()
                .document(key.branch),
            Some(&after)
        );
        workspace.bind(&key, 78).unwrap();
        workspace.clear();
        assert!(workspace.binding_source(78).is_none());
        assert_eq!(
            workspace
                .apply_commit(&mut project, &stale, &defs, |_, _, _| None)
                .unwrap_err(),
            NodeSourceError::UnknownBinding
        );
    }
    #[test]
    fn node_source_workspace_imports_configuration_once_and_persists_invalid_source_without_replacing_generated_data()
     {
        let mut project = Project::default();
        let mut character = crate::character::Character::new();
        character.data = "[attributes]\nrace = \"Human\"\nclass = \"Warrior\"".into();
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
        assert_eq!(
            workspace
                .load(&project, key.clone(), &defs, false)
                .unwrap()
                .document(),
            &original
        );
        assert!(!project.node_graphs.contains_key(&key.owner));
        let generated = project.characters[&id].data.clone();
        let mut invalid = original.clone();
        invalid
            .nodes
            .iter_mut()
            .find(|node| node.definition.as_deref() == Some("entity"))
            .unwrap()
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("race"))
            .unwrap()
            .value = GraphControlValue::Text("missing race".into());
        let change = workspace
            .commit_document(&mut project, &key, invalid.clone(), &defs, |_, _, _| None)
            .unwrap();
        assert!(change.change.is_some() && change.validation.is_some() && change.error.is_none());
        assert_eq!(project.characters[&id].data, generated);
        assert_eq!(
            serde_json::from_value::<GraphDocument>(project.node_graphs[&key.owner].clone())
                .unwrap(),
            invalid
        );
        let no_change = workspace
            .commit_document(&mut project, &key, invalid, &defs, |_, _, _| None)
            .unwrap();
        assert!(no_change.change.is_none() && no_change.validation.is_some());
        assert_eq!(project.characters[&id].data, generated);
        let corrected = workspace
            .commit_document(&mut project, &key, original.clone(), &defs, |_, _, _| None)
            .unwrap();
        assert!(corrected.change.is_some());
        assert!(corrected.validation.is_none(), "{:?}", corrected.validation);
        let saved = project.node_graphs.clone();
        assert!(
            workspace
                .commit_document(&mut project, &key, original, &defs, |_, _, _| None)
                .unwrap()
                .change
                .is_none()
        );
        assert_eq!(project.node_graphs, saved);
    }
    #[test]
    fn node_source_workspace_rejects_legacy_rules_without_changing_the_real_authority() {
        let project = Project::default();
        let definitions = rulesets::graph::definitions();
        let key = NodeSourceKey {
            owner: rulesets::graph::OWNER.into(),
            branch: Some(GraphId::new_v4()),
        };
        let serialized = serde_json::to_value(&project).unwrap();
        let runtime = project.rules_source().unwrap();
        let legacy = project.rules.clone();
        let mut workspace = NodeGraphWorkspace::default();
        assert!(matches!(
            workspace.graphs(&project, &key.owner, &definitions),
            Err(NodeSourceError::InvalidSource(_))
        ));
        assert!(matches!(
            workspace.load(&project, key.clone(), &definitions, false),
            Err(NodeSourceError::InvalidSource(_))
        ));
        assert!(matches!(
            workspace.create_branch(&project, key.clone(), &definitions),
            Err(NodeSourceError::InvalidSource(_))
        ));
        assert!(workspace.source(&key).is_none());
        assert_eq!(serde_json::to_value(&project).unwrap(), serialized);
        assert_eq!(project.rules_source().unwrap(), runtime);
        assert_eq!(project.rules, legacy);
        let loaded: Project = serde_json::from_value(serialized).unwrap();
        assert_eq!(loaded.rules_document, project.rules_document);
        assert_eq!(loaded.rules_source().unwrap(), runtime);
    }
}
