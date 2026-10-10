//! Small shared building blocks for declarative authoring graphs.
use theframework::thegraph::*;

pub fn number(value: f32, min: f32, max: f32, step: f32) -> GraphControlValue {
    GraphControlValue::Number {
        value,
        min,
        max,
        step,
    }
}
pub fn node(
    key: &str,
    title: &str,
    color: GraphColor,
    rows: Vec<(&str, &str, GraphControlValue)>,
    root: bool,
    kind: &str,
) -> GraphNode {
    let mut node = GraphNode::new(title, [0., 0.], color);
    node.definition = Some(key.into());
    node.width = 210.;
    for (key, label, value) in rows {
        let mut row = GraphRow::new(label, value);
        row.key = Some(key.into());
        node.rows.push(row);
    }
    for (key, direction, side) in [
        ("in", PortDirection::Input, PortSide::Left),
        ("out", PortDirection::Output, PortSide::Right),
    ] {
        if root && direction == PortDirection::Input {
            continue;
        }
        let mut port = GraphPort::new("", direction, side, 0.5);
        port.key = Some(key.into());
        port.kind = kind.into();
        node.ports.push(port);
    }
    node
}
pub fn value<'a>(node: &'a GraphNode, key: &str) -> Result<&'a GraphControlValue, String> {
    node.rows
        .iter()
        .find(|row| row.key.as_deref() == Some(key))
        .map(|row| &row.value)
        .ok_or_else(|| format!("{}: missing {key}", node.title))
}
pub fn read_number(node: &GraphNode, key: &str) -> Result<f32, String> {
    match value(node, key)? {
        GraphControlValue::Number { value, .. } if value.is_finite() => Ok(*value),
        GraphControlValue::Text(value) => value
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("{}: invalid {key}", node.title)),
        _ => Err(format!("{}: invalid {key}", node.title)),
    }
}
pub fn text(node: &GraphNode, key: &str) -> Result<String, String> {
    match value(node, key)? {
        GraphControlValue::Text(value) => Ok(value.clone()),
        _ => Err(format!("{}: invalid {key}", node.title)),
    }
}
pub fn set(node: &mut GraphNode, key: &str, value: GraphControlValue) {
    if let Some(row) = node
        .rows
        .iter_mut()
        .find(|row| row.key.as_deref() == Some(key))
    {
        row.value = value;
    }
}
pub fn connect(doc: &mut GraphDocument, from: &GraphNode, to: &GraphNode) {
    doc.connections.push(GraphConnection {
        id: theframework::prelude::Uuid::new_v4(),
        from: from
            .ports
            .iter()
            .find(|p| p.key.as_deref() == Some("out"))
            .unwrap()
            .id,
        to: to
            .ports
            .iter()
            .find(|p| p.key.as_deref() == Some("in"))
            .unwrap()
            .id,
    });
}
