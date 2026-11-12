//! Independent authoring copies, shared by clipboard hosts and legacy imports.
use super::*;
use std::collections::HashMap;

pub fn append_graph_copy(
    document: &mut GraphDocument,
    clipboard: &GraphDocument,
    offset: Point,
) -> HashMap<GraphId, GraphId> {
    let mut moved: HashMap<GraphId, GraphId> = HashMap::new();
    for source in &clipboard.nodes {
        let mut node = source.clone();
        let id = GraphId::new_v4();
        moved.insert(node.id, id);
        node.id = id;
        for row in &mut node.rows {
            let row_id = GraphId::new_v4();
            moved.insert(row.id, row_id);
            row.id = row_id;
            if let GraphControlValue::List { row_ids, .. } = &mut row.value {
                for item in row_ids {
                    let fresh = GraphId::new_v4();
                    moved.insert(*item, fresh);
                    *item = fresh;
                }
            }
        }
        for port in &mut node.ports {
            let port_id = GraphId::new_v4();
            moved.insert(port.id, port_id);
            port.id = port_id;
            port.row = port.row.and_then(|id| moved.get(&id).copied());
            port.list_item = port.list_item.and_then(|id| moved.get(&id).copied());
            if let Some(item) = port.list_item {
                port.key = Some(format!("field:{item}"));
            }
        }
        node.position[0] += offset[0];
        node.position[1] += offset[1];
        document.nodes.push(node);
    }
    for node in document
        .nodes
        .iter_mut()
        .filter(|n| moved.values().any(|id| *id == n.id))
    {
        node.branch = node.branch.and_then(|owner| moved.get(&owner).copied());
        for row in &mut node.rows {
            if let Some(binding) = &mut row.binding {
                binding.node = moved.get(&binding.node).copied().unwrap_or(binding.node);
            }
        }
    }
    for source in &clipboard.connections {
        document.connections.push(GraphConnection {
            id: GraphId::new_v4(),
            from: moved.get(&source.from).copied().unwrap_or(source.from),
            to: moved.get(&source.to).copied().unwrap_or(source.to),
        });
    }
    moved
}
