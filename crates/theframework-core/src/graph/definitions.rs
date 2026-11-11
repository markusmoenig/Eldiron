//! Host-supplied definitions and typed event bindings; no gameplay execution.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GraphValueType {
    Text,
    Number,
    Boolean,
    Entity,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GraphDatum {
    Text(String),
    Number(f64),
    Boolean(bool),
    Entity(u64),
}
impl GraphDatum {
    pub fn value_type(&self) -> GraphValueType {
        match self {
            Self::Text(_) => GraphValueType::Text,
            Self::Number(_) => GraphValueType::Number,
            Self::Boolean(_) => GraphValueType::Boolean,
            Self::Entity(_) => GraphValueType::Entity,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphEventBinding {
    pub node: GraphId,
    pub event: String,
    pub field: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEventField {
    pub id: String,
    pub label: String,
    pub value_type: GraphValueType,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEventDefinition {
    pub id: String,
    pub label: String,
    pub fields: Vec<GraphEventField>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphParameterDefinition {
    pub id: String,
    pub label: String,
    pub default: GraphControlValue,
    pub value_type: Option<GraphValueType>,
    pub height: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphPortDefinition {
    pub id: String,
    pub label: String,
    pub side: PortSide,
    pub direction: PortDirection,
    pub position: f32,
    pub row: Option<String>,
    pub kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNodeDefinition {
    pub id: String,
    pub title: String,
    pub category: String,
    pub color: GraphColor,
    pub width: f32,
    pub parameters: Vec<GraphParameterDefinition>,
    pub ports: Vec<GraphPortDefinition>,
    /// Whether this node starts a branch when it has no flow inputs.
    pub starts_branch: bool,
    /// Parameter ID storing the selected event's stable ID, when this is an event entry.
    pub event_parameter: Option<String>,
}
impl GraphNodeDefinition {
    /// Convert an authored visual template into a reusable definition. Row IDs become
    /// definition-local keys; callers can assign semantic keys before registration.
    pub fn from_template(id: &str, category: &str, node: &GraphNode) -> Self {
        Self {
            id: id.into(),
            title: node.title.clone(),
            category: category.into(),
            color: node.color,
            width: node.width,
            parameters: node
                .rows
                .iter()
                .enumerate()
                .map(|(i, r)| GraphParameterDefinition {
                    id: r.key.clone().unwrap_or_else(|| format!("parameter_{i}")),
                    label: r.label.clone(),
                    default: r.value.clone(),
                    value_type: r.value_type,
                    height: r.height,
                })
                .collect(),
            ports: node
                .ports
                .iter()
                .enumerate()
                .map(|(i, p)| GraphPortDefinition {
                    id: p.key.clone().unwrap_or_else(|| format!("port_{i}")),
                    label: p.label.clone(),
                    side: p.side,
                    direction: p.direction,
                    position: p.position,
                    kind: p.kind.clone(),
                    row: p
                        .row
                        .and_then(|id| node.rows.iter().position(|r| r.id == id))
                        .map(|i| {
                            node.rows[i]
                                .key
                                .clone()
                                .unwrap_or_else(|| format!("parameter_{i}"))
                        }),
                })
                .collect(),
            starts_branch: !node
                .ports
                .iter()
                .any(|port| port.direction == PortDirection::Input),
            event_parameter: None,
        }
    }
    pub fn instantiate(&self, position: Point) -> GraphNode {
        let mut node = GraphNode::new(&self.title, position, self.color);
        node.definition = Some(self.id.clone());
        node.width = self.width;
        node.rows = self
            .parameters
            .iter()
            .map(|p| {
                let mut row = GraphRow::new(&p.label, p.default.clone());
                row.key = Some(p.id.clone());
                row.value_type = p.value_type;
                row.height = p.height;
                row
            })
            .collect();
        node.ports = self
            .ports
            .iter()
            .map(|p| {
                let mut port = GraphPort::new(&p.label, p.direction, p.side, p.position);
                port.key = Some(p.id.clone());
                port.kind = p.kind.clone();
                port.row = p
                    .row
                    .as_ref()
                    .and_then(|key| node.rows.iter().find(|r| r.key.as_ref() == Some(key)))
                    .map(|r| r.id);
                port
            })
            .collect();
        node
    }
}
#[derive(Default)]
pub struct GraphDefinitions {
    nodes: BTreeMap<String, GraphNodeDefinition>,
    events: BTreeMap<String, GraphEventDefinition>,
}
impl GraphDefinitions {
    pub fn register_node(&mut self, definition: GraphNodeDefinition) -> Result<(), String> {
        if self.nodes.contains_key(&definition.id) {
            return Err("Duplicate node definition ID".into());
        }
        let mut ids = HashSet::new();
        for p in &definition.parameters {
            if !ids.insert(&p.id) {
                return Err("Duplicate parameter ID".into());
            }
        }
        if definition
            .event_parameter
            .as_ref()
            .is_some_and(|p| !ids.contains(p))
        {
            return Err("Missing event parameter".into());
        }
        let mut ports = HashSet::new();
        for p in &definition.ports {
            if !ports.insert(&p.id) || p.row.as_ref().is_some_and(|r| !ids.contains(r)) {
                return Err("Invalid port definition".into());
            }
        }
        self.nodes.insert(definition.id.clone(), definition);
        Ok(())
    }
    pub fn register_event(&mut self, definition: GraphEventDefinition) -> Result<(), String> {
        if self.events.contains_key(&definition.id) {
            return Err("Duplicate event ID".into());
        }
        let mut fields = HashSet::new();
        if definition.fields.iter().any(|f| !fields.insert(&f.id)) {
            return Err("Duplicate event field ID".into());
        }
        self.events.insert(definition.id.clone(), definition);
        Ok(())
    }
    pub fn nodes(&self) -> impl Iterator<Item = &GraphNodeDefinition> {
        self.nodes.values()
    }
    pub fn events(&self) -> impl Iterator<Item = &GraphEventDefinition> {
        self.events.values()
    }
    pub fn node(&self, id: &str) -> Option<&GraphNodeDefinition> {
        self.nodes.get(id)
    }
    pub fn event(&self, id: &str) -> Option<&GraphEventDefinition> {
        self.events.get(id)
    }
    pub fn instantiate(&self, id: &str, position: Point) -> Result<GraphNode, String> {
        Ok(self
            .node(id)
            .ok_or("Unknown node definition")?
            .instantiate(position))
    }
    pub fn selected_event<'a>(&self, node: &'a GraphNode) -> Option<&'a str> {
        let key = self
            .node(node.definition.as_deref()?)?
            .event_parameter
            .as_ref()?;
        match &node
            .rows
            .iter()
            .find(|r| r.key.as_ref() == Some(key))?
            .value
        {
            GraphControlValue::Custom { kind, data } if kind == "event" => data.as_str(),
            _ => None,
        }
    }
    pub fn parameter_type(&self, node: &GraphNode, row: &GraphRow) -> Option<GraphValueType> {
        if let (Some(id), Some(key)) = (&node.definition, &row.key) {
            return self
                .node(id)?
                .parameters
                .iter()
                .find(|p| &p.id == key)?
                .value_type;
        }
        row.value_type
    }
    pub fn binding_label(&self, b: &GraphEventBinding) -> String {
        let event = self.event(&b.event);
        let field = event.and_then(|e| e.fields.iter().find(|f| f.id == b.field));
        format!(
            "{} > {}",
            event.map(|e| e.label.as_str()).unwrap_or(&b.event),
            field.map(|f| f.label.as_str()).unwrap_or(&b.field)
        )
    }
    pub fn reachable(&self, doc: &GraphDocument, from: GraphId, to: GraphId) -> bool {
        let mut pending = vec![from];
        let mut seen = HashSet::new();
        while let Some(id) = pending.pop() {
            if id == to {
                return true;
            }
            if !seen.insert(id) {
                continue;
            }
            for c in &doc.connections {
                if let (Some((a, _)), Some((b, _))) = (doc.port(c.from), doc.port(c.to)) {
                    if a.id == id && self.selected_event(b).is_none() {
                        pending.push(b.id);
                    }
                }
            }
        }
        false
    }
    pub fn validate_binding(
        &self,
        doc: &GraphDocument,
        node: &GraphNode,
        row: &GraphRow,
    ) -> Result<(), String> {
        let Some(b) = &row.binding else {
            return Ok(());
        };
        let source = doc
            .nodes
            .iter()
            .find(|n| n.id == b.node)
            .ok_or("Missing event node")?;
        if self.selected_event(source) != Some(b.event.as_str()) {
            return Err("Event changed; rebind value".into());
        }
        let field = self
            .event(&b.event)
            .and_then(|e| e.fields.iter().find(|f| f.id == b.field))
            .ok_or("Missing event field")?;
        if self.parameter_type(node, row) != Some(field.value_type) {
            return Err("Incompatible field type".into());
        }
        if !self.reachable(doc, source.id, node.id) {
            return Err("Event is not connected to this node".into());
        }
        Ok(())
    }
    pub fn binding_options(
        &self,
        doc: &GraphDocument,
        node: &GraphNode,
        row: &GraphRow,
    ) -> Vec<(String, GraphEventBinding)> {
        let mut result = vec![];
        for source in &doc.nodes {
            if !self.reachable(doc, source.id, node.id) {
                continue;
            }
            if let Some(event) = self.selected_event(source).and_then(|e| self.event(e)) {
                for field in &event.fields {
                    if self.parameter_type(node, row) == Some(field.value_type) {
                        let binding = GraphEventBinding {
                            node: source.id,
                            event: event.id.clone(),
                            field: field.id.clone(),
                        };
                        result.push((
                            format!(
                                "{} [{}]",
                                self.binding_label(&binding),
                                &source.id.to_string()[..6]
                            ),
                            binding,
                        ));
                    }
                }
            }
        }
        result
    }
    /// Explicit sample/live invocation data is keyed to its source node and schema.
    pub fn resolve(
        &self,
        doc: &GraphDocument,
        node: &GraphNode,
        row: &GraphRow,
        samples: &BTreeMap<Uuid, GraphEventSample>,
    ) -> Result<Option<GraphDatum>, String> {
        self.validate_binding(doc, node, row)?;
        if let Some(binding) = &row.binding {
            let Some(sample) = samples.get(&binding.node) else {
                return Ok(None);
            };
            if sample.event != binding.event {
                return Ok(None);
            }
            let value = sample.fields.get(&binding.field).cloned();
            if value
                .as_ref()
                .is_some_and(|v| Some(v.value_type()) != self.parameter_type(node, row))
            {
                return Err("Payload type does not match schema".into());
            }
            return Ok(value);
        }
        let value = match &row.value {
            GraphControlValue::Text(t) => Some(GraphDatum::Text(t.clone())),
            GraphControlValue::Number { value, .. } => Some(GraphDatum::Number(*value as f64)),
            GraphControlValue::Toggle(v) => Some(GraphDatum::Boolean(*v)),
            GraphControlValue::Choice { options, selected } => {
                options.get(*selected).cloned().map(GraphDatum::Text)
            }
            _ => None,
        };
        if let (Some(expected), Some(value)) = (self.parameter_type(node, row), &value) {
            if expected != value.value_type() {
                return Err("Literal type does not match parameter".into());
            }
        }
        Ok(value)
    }
}
#[derive(Clone, Debug)]
pub struct GraphEventSample {
    pub event: String,
    pub fields: BTreeMap<String, GraphDatum>,
}
