//! CPU branch envelope codec. UI selection/cameras, authoring history and project
//! persistence belong to callers; parsing never mutates the supplied source.
use std::collections::{HashMap, HashSet};
use theframework::thegraph::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeGraphKind {
    Behavior,
    Configuration,
    Rules,
}
#[derive(Clone, Debug)]
pub struct NodeGraphDocuments {
    pub kind: NodeGraphKind,
    pub order: Vec<GraphId>,
    pub branches: HashMap<GraphId, GraphDocument>,
    pub draft: GraphDocument,
}
impl NodeGraphDocuments {
    pub fn empty(kind: NodeGraphKind) -> Self {
        Self {
            kind,
            order: vec![],
            branches: HashMap::new(),
            draft: GraphDocument::default(),
        }
    }
    pub fn from_value(
        value: &serde_json::Value,
        kind: NodeGraphKind,
        definitions: &GraphDefinitions,
    ) -> Result<Self, String> {
        if value.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
            return Err("Unsupported node graph envelope version".into());
        }
        let mut result = Self::empty(kind);
        if kind == NodeGraphKind::Configuration {
            result.draft = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            return Ok(result);
        }
        let documents: Vec<GraphDocument> = if let Some(branches) = value.get("branches") {
            serde_json::from_value(branches.clone()).map_err(|e| e.to_string())?
        } else {
            vec![serde_json::from_value(value.clone()).map_err(|e| e.to_string())?]
        };
        let mut used = HashSet::new();
        for document in documents {
            if document.version != 1 {
                return Err("Unsupported node graph document version".into());
            }
            if kind == NodeGraphKind::Rules {
                let root = document
                    .nodes
                    .iter()
                    .find(|node| {
                        matches!(
                            node.definition.as_deref(),
                            Some("rules_definition" | "rules_fx")
                        )
                    })
                    .or_else(|| document.nodes.first())
                    .map(|node| node.id);
                if let Some(root) = root {
                    if result.branches.insert(root, document).is_some() {
                        return Err("Duplicate rules branch identity".into());
                    }
                    result.order.push(root);
                }
                continue;
            }
            let found = graph_branches(&document, definitions);
            if found.branches.is_empty() {
                if !result.draft.nodes.is_empty() && !document.nodes.is_empty() {
                    return Err("Multiple unrooted node graph documents".into());
                }
                if !document.nodes.is_empty() {
                    result.draft = document;
                }
                continue;
            }
            let first = found.branches.first().map(|branch| branch.root);
            let single = found.branches.len() == 1;
            for branch in found.branches {
                let mut graph = document.clone();
                graph.nodes.retain(|node| {
                    single
                        || branch.nodes.contains(&node.id)
                        || (Some(branch.root) == first && found.detached.contains(&node.id))
                });
                let ports: HashSet<_> = graph
                    .nodes
                    .iter()
                    .flat_map(|node| node.ports.iter().map(|port| port.id))
                    .collect();
                graph.connections.retain(|connection| {
                    ports.contains(&connection.from) && ports.contains(&connection.to)
                });
                let root = if graph.nodes.iter().any(|node| used.contains(&node.id)) {
                    let mut independent = GraphDocument::default();
                    let identities = append_graph_copy(&mut independent, &graph, [0.; 2]);
                    graph = independent;
                    identities[&branch.root]
                } else {
                    branch.root
                };
                for node in &mut graph.nodes {
                    node.branch = Some(root);
                    used.insert(node.id);
                }
                result.order.push(root);
                result.branches.insert(root, graph);
            }
        }
        Ok(result)
    }
    pub fn document(&self, branch: Option<GraphId>) -> Option<&GraphDocument> {
        branch
            .map(|root| self.branches.get(&root))
            .unwrap_or(Some(&self.draft))
    }
    pub fn replace(&mut self, branch: Option<GraphId>, document: GraphDocument) {
        if let Some(root) = branch {
            if !self.order.contains(&root) {
                self.order.push(root);
            }
            self.branches.insert(root, document);
        } else {
            self.draft = document;
        }
    }
    pub fn documents(&self) -> Vec<GraphDocument> {
        let mut documents: Vec<_> = self
            .order
            .iter()
            .filter_map(|id| self.branches.get(id))
            .filter(|doc| !doc.nodes.is_empty())
            .cloned()
            .collect();
        if !self.draft.nodes.is_empty() {
            documents.push(self.draft.clone());
        }
        documents
    }
    pub fn to_value(&self) -> serde_json::Value {
        match self.kind {
            NodeGraphKind::Configuration => serde_json::to_value(&self.draft).unwrap(),
            NodeGraphKind::Behavior => {
                serde_json::json!({"version":1,"nodes":[],"connections":[],"branches":self.documents()})
            }
            NodeGraphKind::Rules => serde_json::json!({"version":1,"branches":self.documents()}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definitions() -> GraphDefinitions {
        let mut definitions = GraphDefinitions::default();
        let mut trigger = GraphNode::new("Trigger", [0.; 2], [50, 80, 110, 255]);
        trigger.ports.push(GraphPort::new(
            "Out",
            PortDirection::Output,
            PortSide::Right,
            0.5,
        ));
        definitions
            .register_node(GraphNodeDefinition::from_template(
                "event", "Events", &trigger,
            ))
            .unwrap();
        let mut body = GraphNode::new("Body", [100., 100.], [60, 90, 120, 255]);
        body.ports.push(GraphPort::new(
            "In",
            PortDirection::Input,
            PortSide::Left,
            0.5,
        ));
        body.rows.push(GraphRow::new(
            "Text",
            GraphControlValue::Text("東京\r\nend".into()),
        ));
        definitions
            .register_node(GraphNodeDefinition::from_template("body", "Actions", &body))
            .unwrap();
        definitions
    }
    #[test]
    fn branch_codec_splits_legacy_shared_nodes_into_independent_zero_offset_documents() {
        let definitions = definitions();
        let a = definitions.instantiate("event", [10., 20.]).unwrap();
        let b = definitions.instantiate("event", [30., 40.]).unwrap();
        let body = definitions.instantiate("body", [100., 100.]).unwrap();
        let connections = [&a, &b]
            .into_iter()
            .map(|root| GraphConnection {
                id: GraphId::new_v4(),
                from: root.ports[0].id,
                to: body.ports[0].id,
            })
            .collect();
        let input = serde_json::to_value(GraphDocument {
            nodes: vec![a, b, body],
            connections,
            ..Default::default()
        })
        .unwrap();
        let before = input.clone();
        let codec =
            NodeGraphDocuments::from_value(&input, NodeGraphKind::Behavior, &definitions).unwrap();
        assert_eq!(codec.order.len(), 2);
        assert_eq!(input, before);
        let left = &codec.branches[&codec.order[0]];
        let right = &codec.branches[&codec.order[1]];
        assert!(
            left.nodes
                .iter()
                .all(|node| right.nodes.iter().all(|other| other.id != node.id))
        );
        assert_eq!(left.connections.len(), 1);
        assert_eq!(right.connections.len(), 1);
        for doc in [left, right] {
            let body = doc
                .nodes
                .iter()
                .find(|node| node.definition.as_deref() == Some("body"))
                .unwrap();
            assert_eq!(body.position, [100., 100.]);
            assert_eq!(
                body.rows[0].value,
                GraphControlValue::Text("東京\r\nend".into())
            );
        }
        let saved = codec.to_value();
        assert_eq!(
            NodeGraphDocuments::from_value(&saved, NodeGraphKind::Behavior, &definitions)
                .unwrap()
                .to_value(),
            saved
        );
    }
    #[test]
    fn branch_codec_rejects_unsupported_or_ambiguous_sources_without_mutation() {
        let definitions = definitions();
        let doc = GraphDocument {
            nodes: vec![definitions.instantiate("body", [0.; 2]).unwrap()],
            ..Default::default()
        };
        let inputs = [
            serde_json::json!({"version":2,"branches":[]}),
            serde_json::json!({"version":1,"branches":[doc.clone(),doc.clone()]}),
            serde_json::json!({"version":1,"branches":[{"version":2,"nodes":[],"connections":[]}]}),
        ];
        for input in inputs {
            let exact = input.clone();
            assert!(
                NodeGraphDocuments::from_value(&input, NodeGraphKind::Behavior, &definitions)
                    .is_err()
            );
            assert_eq!(input, exact);
        }
        let input = serde_json::to_value(&doc).unwrap();
        assert_eq!(
            NodeGraphDocuments::from_value(&input, NodeGraphKind::Configuration, &definitions)
                .unwrap()
                .to_value(),
            input
        );
    }
}
