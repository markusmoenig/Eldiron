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
        serde_json::json!({"version":1,"nodes":[],"connections":[],"branches":documents})
    }
    /// Read separate documents, or split an older connection-based graph.
    pub(super) fn load_branch_documents(
        &mut self,
        value: &serde_json::Value,
    ) -> Result<GraphDocument, String> {
        self.branch_graphs.clear();
        self.branch_order.clear();
        let mut documents: Vec<GraphDocument> = if let Some(branches) = value.get("branches") {
            serde_json::from_value(branches.clone()).map_err(|e| e.to_string())?
        } else {
            vec![serde_json::from_value(value.clone()).map_err(|e| e.to_string())?]
        };
        let mut draft = GraphDocument::default();
        let mut used = std::collections::HashSet::new();
        for doc in documents.drain(..) {
            if doc.version != 1 {
                return Err(fl!("node_unsupported"));
            }
            let branches = graph_branches(&doc, &self.definitions);
            if branches.branches.is_empty() {
                draft = doc;
                continue;
            }
            let first = branches.branches.first().map(|b| b.root);
            let single = branches.branches.len() == 1;
            for branch in branches.branches {
                let mut graph = doc.clone();
                graph.nodes.retain(|n| {
                    single
                        || branch.nodes.contains(&n.id)
                        || (Some(branch.root) == first && branches.detached.contains(&n.id))
                });
                let ports: std::collections::HashSet<_> = graph
                    .nodes
                    .iter()
                    .flat_map(|n| n.ports.iter().map(|p| p.id))
                    .collect();
                graph
                    .connections
                    .retain(|c| ports.contains(&c.from) && ports.contains(&c.to));
                // Legacy shared nodes become independent copies, never shared editing state.
                let root = if graph.nodes.iter().any(|n| used.contains(&n.id)) {
                    let mut copy = GraphDocument::default();
                    let ids = paste_graph(&mut copy, &graph);
                    for n in &mut copy.nodes {
                        n.position[0] -= PASTE_OFFSET;
                        n.position[1] -= PASTE_OFFSET;
                    }
                    graph = copy;
                    ids[&branch.root]
                } else {
                    branch.root
                };
                for node in &mut graph.nodes {
                    node.branch = Some(root);
                    used.insert(node.id);
                }
                self.branch_order.push(root);
                self.branch_graphs.insert(root, graph);
            }
        }
        self.active_branch = self
            .active_branch
            .filter(|root| self.branch_graphs.contains_key(root))
            .or_else(|| self.branch_order.first().copied());
        Ok(self
            .active_branch
            .and_then(|root| self.branch_graphs.get(&root).cloned())
            .unwrap_or(draft))
    }
}
