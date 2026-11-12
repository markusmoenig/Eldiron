use std::sync::Arc;
use theframework_core::graph::*;

#[test]
fn cpu_graph_packets_roundtrip_reversible_edits_without_widget_or_renderer_types() {
    fn assert_shared<T: Send + Sync>() {}
    assert_shared::<GraphCommit>();
    assert_shared::<GraphEdit>();
    let mut first = GraphNode::new("First", [10., 20.], [20, 30, 40, 255]);
    first.rows.push(GraphRow::new(
        "Name",
        GraphControlValue::Text("Before".into()),
    ));
    first.ports.push(GraphPort::new(
        "Out",
        PortDirection::Output,
        PortSide::Right,
        0.5,
    ));
    let mut second = GraphNode::new("Second", [200., 20.], [50, 60, 70, 255]);
    second.ports.push(GraphPort::new(
        "In",
        PortDirection::Input,
        PortSide::Left,
        0.5,
    ));
    let third = GraphNode::new("Added", [400., 20.], [80, 90, 100, 255]);
    let mut changed = second.clone();
    changed.title = "Updated second".into();
    changed.folded = true;
    let source = GraphDocument {
        nodes: vec![first.clone(), second.clone()],
        ..Default::default()
    };
    let packet = Arc::new(GraphCommit::new(
        17,
        1,
        vec![
            GraphEdit::Move {
                node: first.id,
                before: first.position,
                after: [50., 60.],
            },
            GraphEdit::SetValue {
                node: first.id,
                row: first.rows[0].id,
                before: first.rows[0].value.clone(),
                after: GraphControlValue::Text("e\u{301}日本\r\nline".into()),
            },
            GraphEdit::ReplaceNode {
                before: second.clone(),
                after: changed,
            },
            GraphEdit::Connection {
                connection: GraphConnection {
                    id: uuid::Uuid::new_v4(),
                    from: first.ports[0].id,
                    to: second.ports[0].id,
                },
                added: true,
            },
            GraphEdit::AddNode { node: third },
        ],
    ));
    let notification = packet.clone();
    assert!(Arc::ptr_eq(&packet, &notification));
    let mut document = source.clone();
    for edit in &packet.edits {
        edit.apply(&mut document, true);
    }
    assert_eq!(document.nodes.len(), 3);
    assert_eq!(
        document.nodes[0].rows[0].value,
        GraphControlValue::Text("e\u{301}日本\r\nline".into())
    );
    let after = document.clone();
    let serialized = serde_json::to_vec(&document).unwrap();
    let restored: GraphDocument = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(restored, after);
    for edit in packet.edits.iter().rev() {
        edit.apply(&mut document, false);
    }
    assert_eq!(document, source);
    for edit in &notification.edits {
        edit.apply(&mut document, true);
    }
    assert_eq!(document, after);
}

#[test]
fn cpu_graph_copy_remaps_branch_list_row_and_port_references_without_changing_source() {
    let mut root = GraphNode::new("Source", [10., 20.], [30, 60, 90, 255]);
    let item = uuid::Uuid::new_v4();
    let row = GraphRow::new(
        "List",
        GraphControlValue::List {
            columns: vec![GraphListColumn {
                id: "text".into(),
                label: "Text".into(),
                control: GraphControlValue::Text(String::new()),
            }],
            row_ids: vec![item],
            rows: vec![vec![GraphControlValue::Text("東京\r\n".into())]],
        },
    );
    let mut port = GraphPort::new("Out", PortDirection::Output, PortSide::Right, 0.5);
    port.row = Some(row.id);
    port.list_item = Some(item);
    port.key = Some(format!("field:{item}"));
    let mut row = row;
    row.binding = Some(GraphEventBinding {
        node: root.id,
        event: "event".into(),
        field: "message".into(),
    });
    root.rows.push(row);
    root.ports.push(port);
    root.branch = Some(root.id);
    let mut next = GraphNode::new("Next", [100., 20.], [50, 80, 110, 255]);
    next.branch = Some(root.id);
    next.ports.push(GraphPort::new(
        "In",
        PortDirection::Input,
        PortSide::Left,
        0.5,
    ));
    let connection = GraphConnection {
        id: uuid::Uuid::new_v4(),
        from: root.ports[0].id,
        to: next.ports[0].id,
    };
    let source = GraphDocument {
        nodes: vec![root, next],
        connections: vec![connection],
        ..Default::default()
    };
    let exact = serde_json::to_value(&source).unwrap();
    let mut target = GraphDocument::default();
    let ids = append_graph_copy(&mut target, &source, [40., 50.]);
    assert_eq!(serde_json::to_value(&source).unwrap(), exact);
    assert_eq!(target.nodes[0].position, [50., 70.]);
    assert_eq!(target.nodes[1].branch, Some(target.nodes[0].id));
    let port = &target.nodes[0].ports[0];
    assert_eq!(
        target.nodes[0].rows[0].binding.as_ref().unwrap().node,
        target.nodes[0].id
    );
    assert_eq!(port.row, Some(target.nodes[0].rows[0].id));
    assert_eq!(port.list_item, Some(ids[&item]));
    assert_eq!(port.key, Some(format!("field:{}", ids[&item])));
    assert_eq!(target.connections[0].from, port.id);
    assert_eq!(target.connections[0].to, target.nodes[1].ports[0].id);
    assert_ne!(target.connections[0].id, source.connections[0].id);
    let GraphControlValue::List { row_ids, rows, .. } = &target.nodes[0].rows[0].value else {
        unreachable!()
    };
    assert_eq!(row_ids[0], ids[&item]);
    assert_eq!(rows[0][0], GraphControlValue::Text("東京\r\n".into()));
}

#[test]
fn schema_identities_are_node_scoped_and_separate_rows_from_ports() {
    let node = uuid::Uuid::new_v4();
    let row = schema_graph_id(node, "row", "field");
    assert_eq!(row, schema_graph_id(node, "row", "field"));
    assert_ne!(row, schema_graph_id(node, "port", "field"));
    assert_ne!(row, schema_graph_id(uuid::Uuid::new_v4(), "row", "field"));
}

#[test]
fn schema_identity_categories_cannot_alias_keys_containing_separators() {
    let node = uuid::Uuid::new_v4();
    assert_ne!(
        schema_graph_id(node, r"row\0a", "b"),
        schema_graph_id(node, "row", r"a\0b")
    );
    assert_ne!(
        schema_graph_id(node, "row\0a", "b"),
        schema_graph_id(node, "row", "a\0b")
    );
}
