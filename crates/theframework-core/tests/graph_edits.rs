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
