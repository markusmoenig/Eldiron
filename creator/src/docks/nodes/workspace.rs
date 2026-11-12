//! Each behavior branch owns a complete graph document. Only that document is
//! given to the canvas and history; the project envelope groups them by entity.
use super::*;
impl NodesDock {
    pub(super) fn history_key(&self) -> Option<String> {
        self.owner.as_ref().map(|owner| {
            if self.is_configuration() {
                owner.clone()
            } else {
                format!(
                    "{owner}/branch/{}",
                    self.active_branch
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "draft".into())
                )
            }
        })
    }
    fn remember_graph(&mut self) {
        if self.is_configuration() {
            return;
        }
        if let Some(root) = self.active_branch {
            if !self.branch_order.contains(&root) {
                self.branch_order.push(root);
            }
            self.branch_graphs.insert(root, self.doc.clone());
        }
    }
    pub(super) fn begin_branch(&mut self) {
        if !self.is_configuration()
            && self
                .doc
                .nodes
                .iter()
                .any(|n| is_branch_trigger(n, &self.definitions))
        {
            self.remember_graph();
            self.doc = GraphDocument::default();
            self.committed = self.doc.clone();
            self.active_branch = None;
            self.editor.selected = None;
        }
    }
    pub(super) fn open_branch_document(&mut self, root: GraphId) {
        if self.is_configuration() {
            return;
        }
        self.remember_graph();
        if let Some(doc) = self.branch_graphs.get(&root).cloned() {
            self.doc = doc;
            self.committed = self.doc.clone();
            self.active_branch = Some(root);
            self.editor = node_editor();
        }
    }
    pub(super) fn branch_value(&mut self) -> serde_json::Value {
        if self.is_configuration() {
            return serde_json::to_value(&self.doc).unwrap();
        }
        let documents = self.branch_documents();
        serde_json::json!({"version":1,"nodes":[],"connections":[],"branches":documents})
    }
    pub(super) fn branch_documents(&mut self) -> Vec<GraphDocument> {
        self.remember_graph();
        let mut documents: Vec<_> = self
            .branch_order
            .iter()
            .filter_map(|root| self.branch_graphs.get(root))
            .cloned()
            .collect();
        documents.retain(|doc| !doc.nodes.is_empty());
        if self.active_branch.is_none() && !self.doc.nodes.is_empty() {
            documents.push(self.doc.clone());
        }
        documents
    }
    /// Read separate documents, or split an older connection-based graph.
    pub(super) fn load_branch_documents(
        &mut self,
        value: &serde_json::Value,
    ) -> Result<GraphDocument, String> {
        use shared::graph_authoring::{NodeGraphDocuments, NodeGraphKind};
        let kind = if self.is_rules() {
            NodeGraphKind::Rules
        } else {
            NodeGraphKind::Behavior
        };
        let documents = NodeGraphDocuments::from_value(value, kind, &self.definitions)?;
        self.branch_graphs = documents.branches;
        self.branch_order = documents.order;
        self.active_branch = self
            .active_branch
            .filter(|root| self.branch_graphs.contains_key(root))
            .or_else(|| self.branch_order.first().copied());
        Ok(self
            .active_branch
            .and_then(|root| self.branch_graphs.get(&root).cloned())
            .unwrap_or(documents.draft))
    }
}
