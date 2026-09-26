//! Declarative entity configuration. The graph is authored; TOML is a generated
//! compatibility projection for existing rendering, spawning and client readers.
//! This compiler has no event scheduler or world mutation dependencies.
use crate::project::Project;
use std::collections::{BTreeMap, HashSet};
use theframework::prelude::Uuid;
use theframework::thegraph::*;
use toml::{Table, Value};

pub const PREFIX: &str = "entity/";
/// Public boundary type keeps hosts on the compiler's TOML version.
pub type RulesTable = Table;
fn level_attribute(rules: &Table) -> Option<String> {
    eldiron_ruleset::resolve_attribute_roles(rules)
        .ok()?
        .get("level")
        .map(str::to_string)
}
fn command_valid(command: &str) -> bool {
    rusterix::input_binding::parse_input_command(command).is_some()
}
pub fn character_key(id: Uuid) -> String {
    format!("entity/character/{id}")
}
pub fn item_key(id: Uuid) -> String {
    format!("entity/item/{id}")
}
pub fn instance_key(region: Uuid, kind: &str, id: Uuid) -> String {
    {
        let _ = region;
        format!("entity/{kind}/{id}")
    }
}

/// New configuration functionality registers a compiler and its visual schema.
/// Both authoring and headless compilation use the same registry.
pub trait ConfigurationNode: Send + Sync {
    fn definition(&self, rules: &Table) -> GraphNodeDefinition;
    fn contribute(&self, node: &GraphNode, result: &mut Table, rules: &Table)
    -> Result<(), String>;
}
pub struct ConfigurationRegistry {
    nodes: BTreeMap<String, Box<dyn ConfigurationNode>>,
}
impl ConfigurationRegistry {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }
    pub fn register(&mut self, id: &str, module: impl ConfigurationNode + 'static) {
        self.nodes.insert(id.into(), Box::new(module));
    }
    pub fn definitions(&self, rules: &Table) -> GraphDefinitions {
        let mut defs = GraphDefinitions::default();
        for module in self.nodes.values() {
            defs.register_node(module.definition(rules)).unwrap();
        }
        defs
    }
    pub fn compile(&self, doc: &GraphDocument, rules: &Table) -> Result<Table, String> {
        if doc.version != 1 {
            return Err("Unsupported Entity graph version".into());
        }
        let roots: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| matches!(n.definition.as_deref(), Some("entity" | "entity_inputs")))
            .collect();
        if roots
            .iter()
            .filter(|n| n.definition.as_deref() == Some("entity"))
            .count()
            > 1
        {
            return Err("An Entity graph can have only one Entity node".into());
        }
        for connection in &doc.connections {
            let (_, output) = doc
                .port(connection.from)
                .ok_or("Missing configuration output")?;
            let (_, input) = doc
                .port(connection.to)
                .ok_or("Missing configuration input")?;
            if input.direction != PortDirection::Input
                || output.direction != PortDirection::Output
                || input.kind != "entity-config"
                || output.kind != "entity-config"
            {
                return Err("Invalid configuration connection".into());
            }
        }
        let mut result = Table::new();
        let mut connected = HashSet::new();
        let mut pending: Vec<_> = roots.iter().rev().map(|n| n.id).collect();
        while let Some(id) = pending.pop() {
            if !connected.insert(id) {
                return Err("Configuration chains must not loop or merge".into());
            }
            let node = doc
                .nodes
                .iter()
                .find(|n| n.id == id)
                .ok_or("Missing configuration node")?;
            let kind = node
                .definition
                .as_deref()
                .ok_or("Configuration node has no definition")?;
            self.nodes
                .get(kind)
                .ok_or_else(|| format!("Unknown configuration node: {kind}"))?
                .contribute(node, &mut result, rules)?;
            for c in doc.connections.iter().rev() {
                if doc.port(c.from).is_some_and(|(n, _)| n.id == id) {
                    pending.push(doc.port(c.to).ok_or("Missing configuration node")?.0.id);
                }
            }
        }
        Ok(result)
    }
}
impl Default for ConfigurationRegistry {
    fn default() -> Self {
        builtin_registry()
    }
}

struct Fields {
    section: &'static str,
    id: &'static str,
    title: &'static str,
    keys: &'static [&'static str],
}
fn row(node: &mut GraphNode, key: &str, value: GraphControlValue) {
    let mut r = GraphRow::new(key, value);
    r.key = Some(key.into());
    node.rows.push(r);
}
fn config_port(node: &mut GraphNode, direction: PortDirection) {
    let side = if direction == PortDirection::Input {
        PortSide::Left
    } else {
        PortSide::Right
    };
    let mut p = GraphPort::new("", direction, side, 0.5);
    p.key = Some(
        if direction == PortDirection::Input {
            "configuration_in"
        } else {
            "configuration_out"
        }
        .into(),
    );
    p.kind = "entity-config".into();
    node.ports.push(p);
}
fn number(value: f32, min: f32, max: f32, step: f32) -> GraphControlValue {
    GraphControlValue::Number {
        value,
        min,
        max,
        step,
    }
}
fn prototype(id: &str, title: &str) -> GraphNode {
    let mut n = GraphNode::new(title, [0., 0.], [91, 86, 151, 255]);
    n.definition = Some(id.into());
    n.width = 250.;
    if !matches!(id, "entity" | "entity_inputs") {
        config_port(&mut n, PortDirection::Input);
    }
    config_port(&mut n, PortDirection::Output);
    n
}
fn default_control(key: &str, rules: &Table) -> GraphControlValue {
    match key {
        "race" | "class" => {
            let section = if key == "race" { "races" } else { "classes" };
            let mut options = vec![String::new()];
            if let Some(values) = rules.get(section).and_then(Value::as_table) {
                options.extend(values.keys().cloned());
            }
            GraphControlValue::Choice {
                options,
                selected: 0,
            }
        }
        "level" => number(0., 0., 100., 1.),
        "radius" => number(0.5, 0., 10., 0.1),
        "size_2d" => number(1., 0.1, 10., 0.1),
        "inventory_slots" => number(8., 0., 100., 1.),
        "wealth" => number(0., 0., 100000., 1.),
        "strength" => number(5., 0., 20., 0.1),
        "range" => number(10., 0., 100., 0.1),
        "lift" => number(0., -10., 10., 0.1),
        "color" => GraphControlValue::Text("#ffffff".into()),
        "visible" => GraphControlValue::Toggle(true),
        "player" | "blocking" => GraphControlValue::Toggle(false),
        _ => GraphControlValue::Text(String::new()),
    }
}
fn control_value(control: &GraphControlValue) -> Result<Option<Value>, String> {
    Ok(match control {
        GraphControlValue::Text(s) => (!s.is_empty()).then(|| Value::String(s.clone())),
        GraphControlValue::Toggle(v) => Some(Value::Boolean(*v)),
        GraphControlValue::Number { value, step, .. } => {
            if !value.is_finite() {
                return Err("Configuration number must be finite".into());
            }
            Some(if *step >= 1. {
                Value::Integer(value.round() as i64)
            } else {
                Value::Float(
                    value
                        .to_string()
                        .parse::<f64>()
                        .map_err(|e| e.to_string())?,
                )
            })
        }
        GraphControlValue::Choice { options, selected } => {
            let s = options
                .get(*selected)
                .ok_or("Invalid configuration choice")?;
            (!s.is_empty()).then(|| Value::String(s.clone()))
        }
        GraphControlValue::List { rows, .. } => Some(Value::Array(
            rows.iter()
                .map(|r| {
                    control_value(r.first().ok_or("Missing list value")?)
                        .map(|v| v.unwrap_or_else(|| Value::String(String::new())))
                })
                .collect::<Result<Vec<_>, String>>()?,
        )),
        GraphControlValue::Custom { kind, data } if kind == "entity-value" => {
            Some(serde_json::from_value(data.clone()).map_err(|e| e.to_string())?)
        }
        _ => return Err("Unsupported configuration control".into()),
    })
}
fn put(result: &mut Table, table: &str, key: &str, value: Value) -> Result<(), String> {
    let target = result
        .entry(table.to_string())
        .or_insert_with(|| Value::Table(Table::new()))
        .as_table_mut()
        .ok_or("Configuration section must be a table")?;
    if target.contains_key(key) {
        return Err(format!("Duplicate configuration value: {table}.{key}"));
    }
    target.insert(key.into(), value);
    Ok(())
}
impl ConfigurationNode for Fields {
    fn definition(&self, rules: &Table) -> GraphNodeDefinition {
        let mut node = prototype(self.id, self.title);
        for key in self.keys {
            if *key == "level" && level_attribute(rules).is_none() {
                continue;
            }
            row(&mut node, key, default_control(key, rules));
        }
        let mut d = GraphNodeDefinition::from_template(self.id, "Entity Configuration", &node);
        d.starts_branch = matches!(self.id, "entity" | "entity_inputs");
        d
    }
    fn contribute(
        &self,
        node: &GraphNode,
        result: &mut Table,
        rules: &Table,
    ) -> Result<(), String> {
        for r in &node.rows {
            let Some(key) = r.key.as_deref() else {
                continue;
            };
            if !self.keys.contains(&key) {
                continue;
            }
            let value = if let GraphControlValue::Text(text) = &r.value {
                if let GraphControlValue::Number { step, .. } = default_control(key, rules) {
                    if text.trim().is_empty() {
                        None
                    } else if step >= 1. {
                        Some(Value::Integer(
                            text.trim().parse::<i64>().map_err(|e| e.to_string())?,
                        ))
                    } else {
                        Some(Value::Float(
                            text.trim().parse::<f64>().map_err(|e| e.to_string())?,
                        ))
                    }
                } else {
                    control_value(&r.value)?
                }
            } else {
                control_value(&r.value)?
            };
            if let Some(value) = value {
                // Level 0 means inherit; the output key is hydrated from the active ruleset's role.
                if key == "level" && value.as_integer() == Some(0) {
                    continue;
                }
                let mapped;
                let key = if key == "level" {
                    mapped = level_attribute(rules).ok_or("Ruleset has no level attribute role")?;
                    mapped.as_str()
                } else {
                    key
                };
                put(result, self.section, key, value)?;
            }
        }
        Ok(())
    }
}
struct Attribute;
impl ConfigurationNode for Attribute {
    fn definition(&self, _: &Table) -> GraphNodeDefinition {
        let mut n = prototype("entity_attribute", "Attribute Override");
        row(&mut n, "attribute", GraphControlValue::Text(String::new()));
        row(
            &mut n,
            "type",
            GraphControlValue::Choice {
                options: vec![
                    "Text".into(),
                    "Integer".into(),
                    "Number".into(),
                    "Boolean".into(),
                    "List".into(),
                ],
                selected: 0,
            },
        );
        row(&mut n, "value", GraphControlValue::Text(String::new()));
        let mut d =
            GraphNodeDefinition::from_template("entity_attribute", "Entity Configuration", &n);
        d.starts_branch = false;
        d
    }
    fn contribute(&self, n: &GraphNode, result: &mut Table, _: &Table) -> Result<(), String> {
        let key = n
            .rows
            .iter()
            .find(|r| r.key.as_deref() == Some("attribute"))
            .and_then(|r| match &r.value {
                GraphControlValue::Text(s) => Some(s.trim()),
                _ => None,
            })
            .filter(|s| !s.is_empty())
            .ok_or("Attribute Override needs an attribute name")?;
        let r = n
            .rows
            .iter()
            .find(|r| r.key.as_deref() == Some("value"))
            .ok_or("Missing attribute value")?;
        let kind = n
            .rows
            .iter()
            .find(|r| r.key.as_deref() == Some("type"))
            .and_then(|r| {
                if let GraphControlValue::Choice { selected, .. } = &r.value {
                    Some(*selected)
                } else {
                    None
                }
            });
        let value = match (&r.value, kind) {
            (GraphControlValue::Text(s), Some(1)) => {
                Value::Integer(s.trim().parse::<i64>().map_err(|e| e.to_string())?)
            }
            (GraphControlValue::Text(s), Some(2)) => {
                Value::Float(s.trim().parse::<f64>().map_err(|e| e.to_string())?)
            }
            _ => control_value(&r.value)?.unwrap_or_else(|| Value::String(String::new())),
        };
        put(result, "attributes", key, value)
    }
}
struct Input;
impl ConfigurationNode for Input {
    fn definition(&self, rules: &Table) -> GraphNodeDefinition {
        let mut n = prototype("entity_input", "Input Binding");
        row(&mut n, "key", GraphControlValue::Text(String::new()));
        row(&mut n, "command", input_commands(rules));
        row(
            &mut n,
            "custom_command",
            GraphControlValue::Text(String::new()),
        );
        let mut d = GraphNodeDefinition::from_template("entity_input", "Entity Input", &n);
        d.starts_branch = false;
        d
    }
    fn contribute(&self, n: &GraphNode, result: &mut Table, _: &Table) -> Result<(), String> {
        let bindings = vec![
            vec!["key", "command", "custom_command"]
                .into_iter()
                .map(|key| {
                    n.rows
                        .iter()
                        .find(|r| r.key.as_deref() == Some(key))
                        .map(|r| r.value.clone())
                        .unwrap_or(GraphControlValue::Text(String::new()))
                })
                .collect::<Vec<_>>(),
        ];
        let rows = &bindings;
        let text = |value: Option<&GraphControlValue>| match value {
            Some(GraphControlValue::Text(s)) => s.trim().to_string(),
            Some(GraphControlValue::Choice { options, selected }) => {
                options.get(*selected).cloned().unwrap_or_default()
            }
            _ => String::new(),
        };
        for binding in rows {
            let key = text(binding.first());
            let custom = text(binding.get(2));
            let command = if custom.is_empty() {
                text(binding.get(1))
            } else {
                custom
            };
            if key.is_empty() && command.is_empty() {
                continue;
            }
            if key.is_empty() {
                return Err("Input Binding needs a key".into());
            }
            if !command_valid(&command) {
                return Err(format!("Unknown input command: {command}"));
            }
            put(
                result,
                "input",
                &key.to_ascii_lowercase(),
                Value::String(command),
            )?;
        }
        Ok(())
    }
}
pub fn builtin_registry() -> ConfigurationRegistry {
    let mut r = ConfigurationRegistry::new();
    for fields in [
        Fields {
            section: "attributes",
            id: "entity",
            title: "Entity",
            keys: &["race", "class", "level"],
        },
        Fields {
            section: "attributes",
            id: "entity_identity",
            title: "Identity",
            keys: &["race", "class", "level"],
        },
        Fields {
            section: "attributes",
            id: "entity_appearance",
            title: "Appearance",
            keys: &["avatar", "tile_id", "size_2d", "visible"],
        },
        Fields {
            section: "attributes",
            id: "entity_body",
            title: "Collision",
            keys: &["radius", "blocking"],
        },
        Fields {
            section: "attributes",
            id: "entity_inventory",
            title: "Inventory",
            keys: &["inventory_slots", "wealth"],
        },
        Fields {
            section: "attributes",
            id: "entity_player",
            title: "Player",
            keys: &["player"],
        },
        Fields {
            section: "attributes",
            id: "entity_ruleset_item",
            title: "Ruleset Item",
            keys: &["ruleset_path"],
        },
        Fields {
            section: "light",
            id: "entity_light",
            title: "Light",
            keys: &["color", "strength", "range", "lift"],
        },
    ] {
        r.register(fields.id, fields);
    }
    r.register("entity_attribute", Attribute);
    r.register(
        "entity_inputs",
        Fields {
            section: "input",
            id: "entity_inputs",
            title: "Input Mapping",
            keys: &[],
        },
    );
    r.register("entity_input", Input);
    r
}

fn toml_control(value: &Value) -> GraphControlValue {
    match value {
        Value::String(s) => GraphControlValue::Text(s.clone()),
        Value::Boolean(v) => GraphControlValue::Toggle(*v),
        Value::Integer(v) if (*v as f32) as i64 != *v => GraphControlValue::Text(v.to_string()),
        Value::Integer(v) => number(*v as f32, i32::MIN as f32, i32::MAX as f32, 1.),
        Value::Float(v) => number(*v as f32, -100000., 100000., 0.01),
        Value::Array(values) => GraphControlValue::List {
            columns: vec![GraphListColumn {
                id: "value".into(),
                label: "Value".into(),
                control: GraphControlValue::Text(String::new()),
            }],
            rows: values.iter().map(|v| vec![toml_control(v)]).collect(),
        },
        _ => GraphControlValue::Custom {
            kind: "entity-value".into(),
            data: serde_json::to_value(value).unwrap(),
        },
    }
}
/// Import existing explicit values without changing their meaning. Unknown keys
/// are retained as typed Attribute nodes; metadata tables remain in the projection.
pub fn import(data: &str, rules: &Table) -> Result<GraphDocument, String> {
    let table: Table = if data.trim().is_empty() {
        Table::new()
    } else {
        data.parse::<Table>().map_err(|e| e.to_string())?
    };
    let mut doc = GraphDocument::default();
    let mut root = prototype("entity", "Entity");
    root.position = [440., 40.];
    root.ports.clear();
    config_port(&mut root, PortDirection::Input);
    let mut groups: BTreeMap<&str, GraphNode> = BTreeMap::new();
    let item_path = table
        .get("attributes")
        .and_then(Value::as_table)
        .and_then(|a| a.get("ruleset_path"))
        .and_then(Value::as_str);
    let item_default = item_path
        .and_then(|path| {
            crate::rulesets::ruleset_item_templates_from_source(&toml::to_string(rules).ok()?)
                .ok()?
                .into_iter()
                .find(|t| t.ruleset_path == path)
        })
        .and_then(|t| t.data.parse::<Table>().ok());
    let level_key = level_attribute(rules);
    if let Some(attrs) = table.get("attributes").and_then(Value::as_table) {
        for (key, value) in attrs {
            if key != "ruleset_path"
                && item_default
                    .as_ref()
                    .and_then(|t| t.get("attributes"))
                    .and_then(Value::as_table)
                    .and_then(|a| a.get(key))
                    == Some(value)
            {
                continue;
            }
            let (id, title) = match key.as_str() {
                "race" | "class" => ("entity_identity", "Identity"),
                "ruleset_path" => ("entity_ruleset_item", "Ruleset Item"),
                key if Some(key) == level_key.as_deref() => ("entity_identity", "Identity"),
                "avatar" | "tile_id" | "size_2d" | "visible" => ("entity_appearance", "Appearance"),
                "radius" | "blocking" => ("entity_body", "Collision"),
                "inventory_slots" | "wealth" => ("entity_inventory", "Inventory"),
                "player" => ("entity_player", "Player"),
                _ => ("entity_attribute", "Attribute Override"),
            };
            if id == "entity_attribute" {
                let mut node = prototype(id, title);
                row(&mut node, "attribute", GraphControlValue::Text(key.clone()));
                row(
                    &mut node,
                    "type",
                    GraphControlValue::Choice {
                        options: vec![
                            "Text".into(),
                            "Integer".into(),
                            "Number".into(),
                            "Boolean".into(),
                            "List".into(),
                        ],
                        selected: match value {
                            Value::Integer(_) => 1,
                            Value::Float(_) => 2,
                            Value::Boolean(_) => 3,
                            Value::Array(_) => 4,
                            _ => 0,
                        },
                    },
                );
                row(&mut node, "value", toml_control(value));
                connect_imported(&mut doc, &root, node);
            } else {
                let node = groups.entry(id).or_insert_with(|| prototype(id, title));
                let row_key = if Some(key.as_str()) == level_key.as_deref() {
                    "level"
                } else {
                    key
                };
                let mut control = toml_control(value);
                if key == "race" || key == "class" {
                    control = default_control(key, rules);
                    if let GraphControlValue::Choice { options, selected } = &mut control {
                        let v = value.as_str().unwrap_or_default();
                        *selected = options.iter().position(|o| o == v).unwrap_or_else(|| {
                            options.push(v.into());
                            options.len() - 1
                        });
                    }
                }
                row(node, row_key, control);
            }
        }
    }
    for node in groups.into_values() {
        connect_imported(&mut doc, &root, node);
    }
    if let Some(light) = table.get("light").and_then(Value::as_table) {
        if item_default.as_ref().and_then(|t| t.get("light")) != table.get("light") {
            let mut node = prototype("entity_light", "Light");
            for (key, value) in light {
                row(&mut node, key, toml_control(value));
            }
            connect_imported(&mut doc, &root, node);
        }
    }
    if let Some(inputs) = table.get("input").and_then(Value::as_table) {
        let definition = Input.definition(rules);
        let mut node = definition.instantiate([0., 0.]);
        node.rows.clear();
        row(
            &mut node,
            "bindings",
            GraphControlValue::List {
                columns: vec![],
                rows: vec![],
            },
        );
        if let GraphControlValue::List { rows, .. } = &mut node.rows[0].value {
            for (key, value) in inputs {
                let mut command = input_commands(rules);
                if let GraphControlValue::Choice { options, selected } = &mut command {
                    let value = normalize_command(value.as_str().unwrap_or_default());
                    *selected = options.iter().position(|s| s == &value).unwrap_or_else(|| {
                        options.push(value);
                        options.len() - 1
                    });
                }
                rows.push(vec![
                    GraphControlValue::Text(key.clone()),
                    command,
                    GraphControlValue::Text(String::new()),
                ]);
            }
        }
        connect_imported(&mut doc, &root, node);
    }
    doc.nodes.push(root);
    normalize(&mut doc);
    Ok(doc)
}
fn connect_imported(doc: &mut GraphDocument, root: &GraphNode, mut node: GraphNode) {
    node.ports.clear();
    config_port(&mut node, PortDirection::Output);
    node.position = [
        40.,
        40. + doc
            .nodes
            .iter()
            .map(|n| n.height(&doc.metrics()) + 36.)
            .sum::<f32>(),
    ];
    doc.connections.push(GraphConnection {
        id: Uuid::new_v4(),
        from: node.ports[0].id,
        to: root.ports[0].id,
    });
    doc.nodes.push(node);
}
/// Convert the initial fan-in prototype to two left-to-right configuration strips.
pub fn normalize(doc: &mut GraphDocument) {
    for node in &mut doc.nodes {
        for port in &mut node.ports {
            if port.kind == "entity-config" {
                port.key = Some(
                    if port.direction == PortDirection::Input {
                        "configuration_in"
                    } else {
                        "configuration_out"
                    }
                    .into(),
                );
            }
        }
    }
    let legacy = doc.nodes.iter().any(|n| {
        n.definition.as_deref() == Some("entity")
            && n.ports.iter().any(|p| p.direction == PortDirection::Input)
    });
    if !legacy {
        return;
    }
    let connected: HashSet<_> = doc
        .connections
        .iter()
        .filter_map(|c| doc.port(c.from).map(|(n, _)| n.id))
        .collect();
    let Some(mut root) = doc
        .nodes
        .iter()
        .find(|n| n.definition.as_deref() == Some("entity"))
        .cloned()
    else {
        return;
    };
    root.rows.retain(|r| r.key.as_deref() != Some("summary"));
    let mut nodes = vec![];
    let mut inputs = vec![];
    let mut detached = vec![];
    for n in &doc.nodes {
        match n.definition.as_deref() {
            Some("entity") => {}
            Some("entity_identity") if connected.contains(&n.id) => {
                root.rows.extend(n.rows.clone())
            }
            Some("entity_input") if connected.contains(&n.id) => {
                if let Some(GraphControlValue::List { rows, .. }) = n
                    .rows
                    .iter()
                    .find(|r| r.key.as_deref() == Some("bindings"))
                    .map(|r| &r.value)
                {
                    for binding in rows {
                        let mut node = prototype("entity_input", "Input Binding");
                        for (key, value) in ["key", "command", "custom_command"]
                            .into_iter()
                            .zip(binding)
                        {
                            row(&mut node, key, value.clone());
                        }
                        inputs.push(node);
                    }
                } else {
                    inputs.push(n.clone());
                }
            }
            _ if connected.contains(&n.id) => nodes.push(n.clone()),
            _ => detached.push(n.clone()),
        }
    }
    nodes.insert(0, root);
    if !inputs.is_empty() {
        inputs.insert(0, prototype("entity_inputs", "Input Mapping"));
    }
    doc.nodes.clear();
    doc.connections.clear();
    for (branch, chain) in [nodes, inputs].into_iter().enumerate() {
        let mut previous = None;
        for (i, mut node) in chain.into_iter().enumerate() {
            node.ports.clear();
            if i > 0 {
                config_port(&mut node, PortDirection::Input);
            }
            config_port(&mut node, PortDirection::Output);
            node.position = [40. + i as f32 * 310., 40. + branch as f32 * 360.];
            if let Some(from) = previous {
                doc.connections.push(GraphConnection {
                    id: Uuid::new_v4(),
                    from,
                    to: node.ports[0].id,
                });
            }
            previous = node
                .ports
                .iter()
                .find(|p| p.direction == PortDirection::Output)
                .map(|p| p.id);
            doc.nodes.push(node);
        }
    }
    for mut node in detached {
        node.ports.clear();
        config_port(&mut node, PortDirection::Input);
        config_port(&mut node, PortDirection::Output);
        doc.nodes.push(node);
    }
}
/// Compile overrides and retain non-attribute metadata. No event is run here.
pub fn project_data(doc: &GraphDocument, previous: &str, rules: &Table) -> Result<String, String> {
    let mut projection = builtin_registry().compile(doc, rules)?;
    if let Some(path) = projection
        .get("attributes")
        .and_then(Value::as_table)
        .and_then(|a| a.get("ruleset_path"))
        .and_then(Value::as_str)
        .map(str::to_string)
    {
        let template = crate::rulesets::ruleset_item_templates_from_source(
            &toml::to_string(rules).map_err(|e| e.to_string())?,
        )?
        .into_iter()
        .find(|t| t.ruleset_path == path)
        .ok_or_else(|| format!("Unknown ruleset item: {path}"))?;
        let mut defaults = template.data.parse::<Table>().map_err(|e| e.to_string())?;
        if let Some(Value::Table(overrides)) = projection.remove("attributes") {
            let attrs = defaults
                .entry("attributes".to_string())
                .or_insert_with(|| Value::Table(Table::new()))
                .as_table_mut()
                .ok_or("Invalid item attributes")?;
            attrs.extend(overrides);
        }
        defaults.extend(projection);
        projection = defaults;
    }
    if let Some(attrs) = projection
        .get_mut("attributes")
        .and_then(Value::as_table_mut)
    {
        for (key, section) in [("race", "races"), ("class", "classes")] {
            if let (Some(id), Some(choices)) = (
                attrs.get(key).and_then(Value::as_str),
                rules.get(section).and_then(Value::as_table),
            ) {
                if !choices.contains_key(id) {
                    return Err(format!("Unknown ruleset {key}: {id}"));
                }
            }
        }
    }
    let mut data = if previous.trim().is_empty() {
        Table::new()
    } else {
        previous.parse::<Table>().map_err(|e| e.to_string())?
    };
    data.remove("attributes");
    data.remove("input");
    data.remove("light");
    data.extend(projection);
    toml::to_string_pretty(&data).map_err(|e| e.to_string())
}

/// Upgrade old projects and refresh generated compatibility data. Call before
/// runtime asset creation so character selection sees Player and Input nodes.
pub fn synchronize(project: &mut Project) -> Result<(), String> {
    let source = crate::rulesets::resolve_project_rules(&project.config, &project.rules)?;
    let rules = source.parse::<Table>().map_err(|e| e.to_string())?;
    fn sync(
        graphs: &mut indexmap::IndexMap<String, serde_json::Value>,
        key: String,
        data: &mut String,
        rules: &Table,
    ) -> Result<(), String> {
        if !graphs.contains_key(&key) {
            graphs.insert(
                key.clone(),
                serde_json::to_value(import(data, rules)?).map_err(|e| e.to_string())?,
            );
        }
        let mut doc = serde_json::from_value::<GraphDocument>(graphs[&key].clone())
            .map_err(|e| e.to_string())?;
        normalize(&mut doc);
        graphs.insert(
            key.clone(),
            serde_json::to_value(&doc).map_err(|e| e.to_string())?,
        );
        *data = project_data(&doc, data, rules).map_err(|e| format!("{key}: {e}"))?;
        Ok(())
    }
    let mut errors = vec![];
    for c in project.characters.values_mut() {
        if let Err(e) = sync(
            &mut project.node_graphs,
            character_key(c.id),
            &mut c.data,
            &rules,
        ) {
            errors.push(e);
        }
    }
    for i in project.items.values_mut() {
        if let Err(e) = sync(
            &mut project.node_graphs,
            item_key(i.id),
            &mut i.data,
            &rules,
        ) {
            errors.push(e);
        }
    }
    for region in &mut project.regions {
        for c in region.characters.values_mut() {
            let key = instance_key(region.id, "character", c.id);
            if project.node_graphs.contains_key(&key)
                || c.data.contains("[attributes]")
                || c.data.contains("[input]")
                || c.data.contains("[light]")
            {
                if let Err(e) = sync(&mut project.node_graphs, key, &mut c.data, &rules) {
                    errors.push(e);
                }
            }
        }
        for i in region.items.values_mut() {
            let key = instance_key(region.id, "item", i.id);
            if project.node_graphs.contains_key(&key)
                || i.data.contains("[attributes]")
                || i.data.contains("[light]")
            {
                if let Err(e) = sync(&mut project.node_graphs, key, &mut i.data, &rules) {
                    errors.push(e);
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Commit one graph's generated data only after compilation succeeds.
pub fn update_owner(project: &mut Project, owner: &str, doc: &GraphDocument) -> Result<(), String> {
    let rules = crate::rulesets::resolve_project_rules(&project.config, &project.rules)?
        .parse::<Table>()
        .map_err(|e| e.to_string())?;
    let id = owner
        .rsplit('/')
        .next()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or("Invalid Entity owner")?;
    let is_character = owner.starts_with("entity/character/");
    let data = if is_character {
        if let Some(c) = project.characters.get_mut(&id) {
            Some(&mut c.data)
        } else {
            project
                .regions
                .iter_mut()
                .find_map(|r| r.characters.get_mut(&id).map(|c| &mut c.data))
        }
    } else {
        if let Some(i) = project.items.get_mut(&id) {
            Some(&mut i.data)
        } else {
            project
                .regions
                .iter_mut()
                .find_map(|r| r.items.get_mut(&id).map(|i| &mut i.data))
        }
    }
    .ok_or("Entity owner no longer exists")?;
    let generated = project_data(doc, data, &rules)?;
    *data = generated;
    Ok(())
}
/// Read-only resolved character values. Provenance distinguishes explicit and
/// inherited configuration; runtime health and other transient state are separate.
pub fn effective_character(
    project: &Project,
    id: Uuid,
    doc: &GraphDocument,
) -> Result<Vec<(String, String, &'static str)>, String> {
    let rules = crate::rulesets::resolve_project_rules(&project.config, &project.rules)?
        .parse::<Table>()
        .map_err(|e| e.to_string())?;
    let mut entity = rusterix::Entity::default();
    let template = project.characters.get(&id);
    let instance = project.regions.iter().find_map(|r| r.characters.get(&id));
    if let Some(instance) = instance {
        if let Some(template) = project.characters.get(&instance.character_id) {
            rusterix::server::data::apply_entity_data(&mut entity, &template.data);
        }
    }
    let previous = template
        .map(|c| c.data.as_str())
        .or_else(|| instance.map(|c| c.data.as_str()))
        .unwrap_or("");
    let generated = project_data(doc, previous, &rules)?;
    let explicit = generated.parse::<Table>().map_err(|e| e.to_string())?;
    rusterix::server::data::apply_entity_data(&mut entity, &generated);
    let authored = entity.attributes.keys().cloned().collect::<HashSet<_>>();
    rusterix::server::region::apply_ruleset_character_defaults(&rules, &mut entity);
    let own = explicit.get("attributes").and_then(Value::as_table);
    let mut values = entity
        .attributes
        .keys()
        .filter_map(|key| {
            let value = entity.attributes.get(key)?;
            if key.starts_with('_') || key == "source" {
                return None;
            }
            let origin = if own.is_some_and(|t| t.contains_key(key)) {
                if instance.is_some() {
                    "instance"
                } else {
                    "template"
                }
            } else if authored.contains(key) {
                "template"
            } else {
                "ruleset"
            };
            Some((key.clone(), format!("{value}"), origin))
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(values)
}

pub fn owner_data<'a>(project: &'a Project, owner: &str) -> Option<&'a str> {
    let id = Uuid::parse_str(owner.rsplit('/').next()?).ok()?;
    if owner.starts_with("entity/character/") {
        project
            .characters
            .get(&id)
            .map(|c| c.data.as_str())
            .or_else(|| {
                project
                    .regions
                    .iter()
                    .find_map(|r| r.characters.get(&id).map(|c| c.data.as_str()))
            })
    } else {
        project.items.get(&id).map(|i| i.data.as_str()).or_else(|| {
            project
                .regions
                .iter()
                .find_map(|r| r.items.get(&id).map(|i| i.data.as_str()))
        })
    }
}

/// The selected player's instance can override individual template keys.
pub fn merged_input(template: &str, instance: &str) -> Result<String, String> {
    let mut inputs = Table::new();
    for source in [template, instance] {
        if source.trim().is_empty() {
            continue;
        }
        let table = source.parse::<Table>().map_err(|e| e.to_string())?;
        if let Some(values) = table.get("input").and_then(Value::as_table) {
            inputs.extend(values.clone());
        }
    }
    let mut table = Table::new();
    table.insert("input".into(), Value::Table(inputs));
    toml::to_string(&table).map_err(|e| e.to_string())
}

fn normalize_command(command: &str) -> String {
    rusterix::input_binding::parse_input_command(command)
        .map(|binding| binding.command_string())
        .unwrap_or_else(|| command.to_string())
}
fn input_commands(rules: &Table) -> GraphControlValue {
    let mut options = vec![String::new()];
    options.extend(
        [
            "control.forward",
            "control.backward",
            "control.left",
            "control.right",
            "control.strafe_left",
            "control.strafe_right",
            "intent.use",
            "intent.look",
            "intent.talk",
            "intent.open",
            "intent.close",
            "intent.drop",
            "ui.actions",
            "ui.spellbook",
            "ui.inventory",
        ]
        .into_iter()
        .map(str::to_string),
    );
    if let Some(actions) = rules.get("actions").and_then(Value::as_table) {
        options.extend(actions.keys().map(|id| format!("rules.{id}")));
    }
    GraphControlValue::Choice {
        options,
        selected: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rules() -> Table {
        crate::rulesets::latest_official_ruleset().parse().unwrap()
    }
    fn data(doc: &GraphDocument, previous: &str) -> Table {
        project_data(doc, previous, &rules())
            .unwrap()
            .parse()
            .unwrap()
    }
    #[test]
    fn configuration_definitions_have_unique_port_keys() {
        let definitions = builtin_registry().definitions(&rules());
        for definition in definitions.nodes() {
            let keys: HashSet<_> = definition.ports.iter().map(|p| &p.id).collect();
            assert_eq!(keys.len(), definition.ports.len(), "{}", definition.id);
        }
    }
    #[test]
    fn import_preserves_typed_values_metadata_and_player_bindings() {
        let source = r##"[attributes]
race = "Orc"
class = "Warrior"
LEVEL = 3
player = true
visible = false
radius = 0.5
custom_flag = true
custom_count = 7
languages = ["common", "orcish"]
[input]
w = "action(forward)"
t = "rules.basic_attack"
[content]
role = "guard"
[light]
color = "#ffffff"
strength = 5.0
"##;
        let doc = import(source, &rules()).unwrap();
        let result = data(&doc, source);
        let previous: Table = source.parse().unwrap();
        assert_eq!(result["attributes"], previous["attributes"]);
        assert_eq!(result["content"], previous["content"]);
        assert_eq!(result["light"], previous["light"]);
        assert_eq!(result["input"]["w"].as_str(), Some("control.forward"));
        assert_eq!(result["input"]["t"].as_str(), Some("rules.basic_attack"));
        let roots: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| matches!(n.definition.as_deref(), Some("entity" | "entity_inputs")))
            .collect();
        assert_eq!(roots.len(), 2);
        let entity = roots
            .iter()
            .find(|n| n.definition.as_deref() == Some("entity"))
            .unwrap();
        assert!(entity.rows.iter().any(|r| r.key.as_deref() == Some("race")));
        assert!(
            doc.nodes
                .iter()
                .filter(|n| n.definition.as_deref() == Some("entity_input"))
                .all(|n| n.rows.iter().all(|r| r.key.as_deref() != Some("bindings")))
        );
        for c in &doc.connections {
            assert!(
                doc.port(c.from).unwrap().0.position[0] < doc.port(c.to).unwrap().0.position[0]
            );
        }
        let mut again = doc.clone();
        normalize(&mut again);
        assert_eq!(
            serde_json::to_value(&doc).unwrap(),
            serde_json::to_value(&again).unwrap()
        );
    }
    #[test]
    fn disconnected_nodes_do_not_apply_and_duplicate_writes_fail() {
        let mut doc = import("[attributes]\nvisible = false", &rules()).unwrap();
        doc.connections.clear();
        assert!(!data(&doc, "[attributes]\nvisible = false").contains_key("attributes"));
        let root = doc
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("entity"))
            .unwrap()
            .clone();
        let mut duplicate = doc
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("entity_appearance"))
            .unwrap()
            .clone();
        duplicate.id = Uuid::new_v4();
        for p in &mut duplicate.ports {
            p.id = Uuid::new_v4();
        }
        let original = doc
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("entity_appearance"))
            .unwrap();
        let original_input = original
            .ports
            .iter()
            .find(|p| p.direction == PortDirection::Input)
            .unwrap()
            .id;
        let original_output = original
            .ports
            .iter()
            .find(|p| p.direction == PortDirection::Output)
            .unwrap()
            .id;
        doc.connections.push(GraphConnection {
            id: Uuid::new_v4(),
            from: root.ports[0].id,
            to: original_input,
        });
        doc.connections.push(GraphConnection {
            id: Uuid::new_v4(),
            from: original_output,
            to: duplicate
                .ports
                .iter()
                .find(|p| p.direction == PortDirection::Input)
                .unwrap()
                .id,
        });
        doc.nodes.push(duplicate);
        assert!(
            project_data(&doc, "", &rules())
                .unwrap_err()
                .contains("Duplicate configuration")
        );
    }
    #[test]
    fn invalid_edit_keeps_projection_and_a_corrected_edit_recovers() {
        let mut project = Project::default();
        let mut character = crate::character::Character::new();
        character.data = "[attributes]\nrace = \"Human\"\nclass = \"Warrior\"".into();
        let id = character.id;
        project.add_character(character);
        synchronize(&mut project).unwrap();
        let key = character_key(id);
        let mut doc: GraphDocument =
            serde_json::from_value(project.node_graphs[&key].clone()).unwrap();
        let before = project.characters[&id].data.clone();
        let row = doc
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("entity"))
            .unwrap()
            .rows
            .iter_mut()
            .find(|r| r.key.as_deref() == Some("race"))
            .unwrap();
        row.value = GraphControlValue::Text("missing race".into());
        assert!(update_owner(&mut project, &key, &doc).is_err());
        assert_eq!(project.characters[&id].data, before);
        doc.nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("entity"))
            .unwrap()
            .rows
            .iter_mut()
            .find(|r| r.key.as_deref() == Some("race"))
            .unwrap()
            .value = GraphControlValue::Text("Orc".into());
        update_owner(&mut project, &key, &doc).unwrap();
        assert!(project.characters[&id].data.contains("Orc"));
    }
    #[test]
    fn instance_identity_is_resolved_before_rule_defaults() {
        let mut project = Project::default();
        let mut template = crate::character::Character::new();
        template.data = "[attributes]\nrace = \"Human\"\nclass = \"Warrior\"".into();
        let mut instance = crate::character::Character::new();
        instance.character_id = template.id;
        instance.data = "[attributes]\nrace = \"Orc\"".into();
        let id = instance.id;
        project.add_character(template);
        project.regions[0].characters.insert(id, instance);
        synchronize(&mut project).unwrap();
        let doc: GraphDocument =
            serde_json::from_value(project.node_graphs[&character_key(id)].clone()).unwrap();
        let values = effective_character(&project, id, &doc).unwrap();
        assert!(
            values
                .iter()
                .any(|(k, v, s)| k == "race" && v.contains("Orc") && *s == "instance")
        );
        assert!(
            values
                .iter()
                .any(|(k, v, s)| k == "class" && v.contains("Warrior") && *s == "template")
        );
    }
    #[test]
    fn bindings_merge_per_key_with_instance_precedence() {
        let merged = merged_input(
            "[input]\nw = \"control.forward\"\nt = \"rules.basic_attack\"",
            "[input]\nt = \"intent.talk\"",
        )
        .unwrap();
        let values: Table = merged.parse().unwrap();
        assert_eq!(values["input"]["w"].as_str(), Some("control.forward"));
        assert_eq!(values["input"]["t"].as_str(), Some("intent.talk"));
    }
}

pub fn effective_item(
    project: &Project,
    id: Uuid,
    doc: &GraphDocument,
) -> Result<Vec<(String, String, &'static str)>, String> {
    let rules = crate::rulesets::resolve_project_rules(&project.config, &project.rules)?
        .parse::<Table>()
        .map_err(|e| e.to_string())?;
    let template = project.items.get(&id);
    let instance = project.regions.iter().find_map(|r| r.items.get(&id));
    let mut item = rusterix::Item::default();
    if let Some(instance) = instance {
        if let Some(template) = project.items.get(&instance.item_id) {
            rusterix::server::data::apply_item_data(&mut item, &template.data);
        }
    }
    let previous = template
        .map(|i| i.data.as_str())
        .or_else(|| instance.map(|i| i.data.as_str()))
        .unwrap_or("");
    let generated = project_data(doc, previous, &rules)?;
    let inherited = item.attributes.keys().cloned().collect::<HashSet<_>>();
    rusterix::server::data::apply_item_data(&mut item, &generated);
    let overrides = builtin_registry().compile(doc, &rules)?;
    let own = overrides.get("attributes").and_then(Value::as_table);
    let mut values = item
        .attributes
        .keys()
        .filter_map(|key| {
            if key.starts_with('_') || key == "source" {
                return None;
            }
            let value = item.attributes.get(key)?;
            let source = if own.is_some_and(|a| a.contains_key(key)) {
                if instance.is_some() {
                    "instance"
                } else {
                    "template"
                }
            } else if inherited.contains(key) {
                "template"
            } else {
                "ruleset"
            };
            Some((key.clone(), value.to_string(), source))
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(values)
}
