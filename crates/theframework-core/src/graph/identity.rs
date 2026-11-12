use super::GraphId;

/// Identity for newly derived rows/ports in independently projected copies of
/// one node. Keep existing authored IDs; this is not a renumbering/migration API.
pub fn schema_graph_id(node: GraphId, category: &str, key: &str) -> GraphId {
    let category = GraphId::new_v5(&node, format!("eldiron.schema.v1:{category}").as_bytes());
    GraphId::new_v5(&category, key.as_bytes())
}
