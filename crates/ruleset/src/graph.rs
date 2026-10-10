//! Declarative rules authoring. Branches own definitions; connections never execute.
//! TOML values are currently the runtime's value tree, not an authoring source.
use crate::{RawRuleset, ResolvedRuleset, RulesetSelection};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use theframework::prelude::Uuid;
use theframework::thegraph::*;
use toml::{Table, Value};

pub const OWNER: &str = "rules/game";
const TYPES: &[&str] = &[
    "Text", "Integer", "Number", "Boolean", "Table", "List", "DateTime",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RulesGraph {
    pub version: u32,
    pub branches: Vec<GraphDocument>,
}

impl RulesGraph {
    pub fn empty() -> Self {
        Self {
            version: 1,
            branches: Vec::new(),
        }
    }

    pub fn official() -> Self {
        static OFFICIAL: std::sync::LazyLock<RulesGraph> = std::sync::LazyLock::new(|| {
            serde_json::from_str(include_str!("../rulesets/eldiron/v1/rules.graph.json"))
                .expect("Bundled rules graph must be valid")
        });
        OFFICIAL.clone()
    }

    pub fn compile(&self) -> Result<Table, String> {
        if self.version != 1 {
            return Err("Unsupported rules graph version".into());
        }
        let mut result = Table::new();
        let mut paths = Vec::<Vec<String>>::new();
        for branch in &self.branches {
            if branch.version != 1 {
                return Err("Unsupported rules branch version".into());
            }
            if branch.nodes.is_empty() {
                continue;
            }
            let roots: Vec<_> = branch
                .nodes
                .iter()
                .filter(|n| {
                    matches!(
                        n.definition.as_deref(),
                        Some("rules_definition" | "rules_fx")
                    )
                })
                .collect();
            if roots.len() != 1 {
                return Err("Each rules branch needs exactly one Definition node".into());
            }
            let root = roots[0];
            if root.disabled {
                continue;
            }
            let path = pointer(text(root, "path")?)?;
            if path.is_empty() {
                return Err("A definition needs a non-empty path".into());
            }
            if paths
                .iter()
                .any(|p| p.starts_with(&path) || path.starts_with(p))
            {
                return Err(format!(
                    "Overlapping rules definition: {}",
                    text(root, "path")?
                ));
            }
            paths.push(path.clone());
            if root.definition.as_deref() == Some("rules_fx") {
                insert_definition(
                    &mut result,
                    &path,
                    Value::Table(crate::particle_graph::compile_branch(branch)?),
                )?;
                continue;
            }
            let mut reachable = BTreeSet::new();
            let mut pending = vec![root.id];
            for c in &branch.connections {
                validate_connection(branch, c.from, c.to)?;
                let (source, output) = branch.port(c.from).ok_or("Missing rules output")?;
                let (target, input) = branch.port(c.to).ok_or("Missing rules input")?;
                if output.direction != PortDirection::Output
                    || input.direction != PortDirection::Input
                    || output.kind != "rules-data"
                    || input.kind != "rules-data"
                    || source.id == target.id
                {
                    return Err("Invalid rules connection".into());
                }
            }
            while let Some(id) = pending.pop() {
                if !reachable.insert(id) {
                    return Err("Rules connections must not loop or merge".into());
                }
                for c in &branch.connections {
                    if branch.port(c.from).is_some_and(|(n, _)| n.id == id) {
                        pending.push(branch.port(c.to).unwrap().0.id);
                    }
                }
            }
            if branch
                .nodes
                .iter()
                .any(|n| !n.disabled && !reachable.contains(&n.id))
            {
                return Err("Connect every rules node to its Definition".into());
            }
            let mut consumed = BTreeSet::new();
            let value = build_value(root, branch, &mut consumed)?;
            if branch
                .nodes
                .iter()
                .any(|n| !n.disabled && !consumed.contains(&n.id))
            {
                return Err("A rules node is not referenced by a key terminal".into());
            }
            insert_definition(&mut result, &path, value)?;
        }
        Ok(result)
    }

    pub fn resolve(&self) -> Result<ResolvedRuleset, String> {
        let table = self.compile()?;
        let schema = table
            .get("ruleset")
            .and_then(|v| v.get("schema_version"))
            .and_then(Value::as_str)
            .unwrap_or("1");
        ResolvedRuleset::from_raw(
            RawRuleset::from_table(table.clone()),
            RulesetSelection {
                id: "project.rules".into(),
                schema_version: schema.into(),
                source: "project".into(),
                ..Default::default()
            },
        )
    }

    /// Restore a definition, including a deleted branch, using its stable path.
    pub fn restore_definition(&mut self, original: &Self, path: &str) -> Result<(), String> {
        let saved = original
            .branches
            .iter()
            .find(|b| branch_path(b) == Some(path))
            .ok_or_else(|| format!("No original definition at {path}"))?
            .clone();
        self.branches.retain(|b| branch_path(b) != Some(path));
        self.branches.push(saved);
        Ok(())
    }
}

/// A full owned graph is the first authoring mode. Its original travels with the
/// project, so restoring never substitutes a newer bundled ruleset.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RulesetState {
    pub current: RulesGraph,
    pub original: RulesGraph,
    #[serde(default)]
    pub checkpoints: Vec<RulesGraph>,
}
impl Default for RulesetState {
    fn default() -> Self {
        let original = RulesGraph::official();
        Self {
            current: original.clone(),
            original,
            checkpoints: Vec::new(),
        }
    }
}
impl RulesetState {
    pub fn checkpoint(&mut self) {
        if self.checkpoints.last() != Some(&self.current) {
            self.checkpoints.push(self.current.clone());
        }
    }
    pub fn restore_original(&mut self) {
        self.checkpoint();
        self.current = self.original.clone();
    }
    /// Swapping preserves the discarded draft, so recovery can itself be undone.
    pub fn restore_checkpoint(&mut self) -> bool {
        let Some(mut saved) = self.checkpoints.pop() else {
            return false;
        };
        std::mem::swap(&mut saved, &mut self.current);
        self.checkpoints.push(saved);
        true
    }
}

pub fn branch_path(branch: &GraphDocument) -> Option<&str> {
    branch
        .nodes
        .iter()
        .find(|n| {
            matches!(
                n.definition.as_deref(),
                Some("rules_definition" | "rules_fx")
            )
        })
        .and_then(|n| text(n, "path").ok())
}
fn control<'a>(node: &'a GraphNode, key: &str) -> Result<&'a GraphControlValue, String> {
    node.rows
        .iter()
        .find(|r| r.key.as_deref() == Some(key))
        .map(|r| &r.value)
        .ok_or_else(|| format!("{}: missing {key}", node.title))
}
fn text<'a>(node: &'a GraphNode, key: &str) -> Result<&'a str, String> {
    match control(node, key)? {
        GraphControlValue::Text(s) => Ok(s),
        GraphControlValue::Custom { kind, data } if kind == "rules_icon" => {
            data.as_str().ok_or("Icon must be an asset id".into())
        }
        _ => Err(format!("{}: {key} must be text", node.title)),
    }
}
fn choice<'a>(node: &'a GraphNode, key: &str) -> Result<&'a str, String> {
    match control(node, key)? {
        GraphControlValue::Choice { options, selected } => options
            .get(*selected)
            .map(String::as_str)
            .ok_or("Invalid type choice".into()),
        _ => Err("Missing type choice".into()),
    }
}
fn pointer(path: &str) -> Result<Vec<String>, String> {
    if path.is_empty() {
        return Ok(vec![]);
    }
    if !path.starts_with('/') {
        return Err(format!("Path must start with /: {path}"));
    }
    path[1..]
        .split('/')
        .map(|part| {
            if part.is_empty() {
                return Err("Path components cannot be empty".into());
            }
            let mut result = String::new();
            let mut chars = part.chars();
            while let Some(c) = chars.next() {
                if c == '~' {
                    result.push(match chars.next() {
                        Some('0') => '~',
                        Some('1') => '/',
                        _ => return Err("Invalid path escape".into()),
                    });
                } else {
                    result.push(c);
                }
            }
            Ok(result)
        })
        .collect()
}
fn scalar(kind: &str, raw: &str) -> Result<Value, String> {
    let invalid = || format!("Invalid {kind} value: {raw}");
    match kind {
        "Text" => Ok(Value::String(raw.into())),
        "Integer" => raw
            .trim()
            .parse::<i64>()
            .map(Value::Integer)
            .map_err(|_| invalid()),
        "Number" => raw
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .map(Value::Float)
            .ok_or_else(invalid),
        "Boolean" => raw
            .trim()
            .parse::<bool>()
            .map(Value::Boolean)
            .map_err(|_| invalid()),
        "DateTime" => raw
            .parse::<toml::value::Datetime>()
            .map(Value::Datetime)
            .map_err(|_| invalid()),
        _ => Err(format!("Unsupported scalar type: {kind}")),
    }
}
/// Refresh per-key terminals while retaining each item's identity through edits.
pub fn sync_ports(doc: &mut GraphDocument) {
    let mut removed = BTreeSet::new();
    for node in &mut doc.nodes {
        if !node
            .definition
            .as_deref()
            .is_some_and(|id| id.starts_with("rules_") && id != "rules_fx")
        {
            continue;
        }
        let mut fields = Vec::new();
        for row in &mut node.rows {
            if row.key.as_deref() != Some("entries") {
                continue;
            }
            if let GraphControlValue::List { rows, row_ids, .. } = &mut row.value {
                row_ids.truncate(rows.len());
                while row_ids.len() < rows.len() {
                    row_ids.push(Uuid::new_v4());
                }
                for (index, id) in row_ids.iter().enumerate() {
                    fields.push((row.id, *id, index));
                }
            }
        }
        node.ports.retain(|port| {
            let keep = port.direction == PortDirection::Input
                || fields.iter().any(|(_, id, _)| port.list_item == Some(*id));
            if !keep {
                removed.insert(port.id);
            }
            keep
        });
        for (row, item, index) in fields {
            let key = format!("field:{item}");
            let port = if let Some(index) = node
                .ports
                .iter()
                .position(|p| p.key.as_deref() == Some(&key))
            {
                &mut node.ports[index]
            } else {
                node.ports.push(GraphPort::new(
                    "",
                    PortDirection::Output,
                    PortSide::Right,
                    0.5,
                ));
                node.ports.last_mut().unwrap()
            };
            port.key = Some(key);
            port.row = Some(row);
            port.list_item = Some(item);
            port.kind = "rules-data".into();
            port.position = (index as f32 + 1.)
                / (node
                    .rows
                    .iter()
                    .find_map(|r| match &r.value {
                        GraphControlValue::List { rows, .. } => Some(rows.len() as f32 + 1.),
                        _ => None,
                    })
                    .unwrap_or(1.));
        }
    }
    doc.connections
        .retain(|c| !removed.contains(&c.from) && !removed.contains(&c.to));
}

/// Check a candidate key wire before the editor commits it.
pub fn validate_connection(doc: &GraphDocument, from: GraphId, to: GraphId) -> Result<(), String> {
    let (parent, output) = doc.port(from).ok_or("Missing rules key terminal")?;
    let (child, input) = doc.port(to).ok_or("Missing rules input terminal")?;
    if output.direction != PortDirection::Output
        || input.direction != PortDirection::Input
        || input.key.as_deref() != Some("in")
        || output.kind != "rules-data"
        || input.kind != "rules-data"
    {
        return Err("Connect a rule key output to a rules node input".into());
    }
    let item = output
        .list_item
        .ok_or("Connect from a specific key terminal")?;
    let (rows, ids) = match control(parent, "entries")? {
        GraphControlValue::List { rows, row_ids, .. } => (rows, row_ids),
        _ => return Err("Missing rule keys".into()),
    };
    let cells = ids
        .iter()
        .position(|id| *id == item)
        .and_then(|i| rows.get(i))
        .ok_or("Missing rule key")?;
    let Some(GraphControlValue::Choice { options, selected }) = cells.get(1) else {
        return Err("Missing key type".into());
    };
    let kind = options.get(*selected).ok_or("Invalid key type")?;
    let expected = match kind.as_str() {
        "Table" => "rules_table",
        "List" => "rules_list",
        _ => "rules_attribute",
    };
    if child.definition.as_deref() != Some(expected) {
        return Err(format!("This key requires a {kind} node"));
    }
    if expected == "rules_attribute" && choice(child, "type")? != kind {
        return Err("Value type mismatch for key".into());
    }
    if doc.connections.iter().any(|c| c.from == from && c.to != to) {
        return Err("A key can have only one connection".into());
    }
    if doc.connections.iter().any(|c| c.to == to && c.from != from) {
        return Err("A rules node can belong to only one key".into());
    }
    Ok(())
}

fn build_value(
    node: &GraphNode,
    doc: &GraphDocument,
    consumed: &mut BTreeSet<Uuid>,
) -> Result<Value, String> {
    if node.disabled {
        return Err("A connected rules node is disabled".into());
    }
    if !consumed.insert(node.id) {
        return Err("Rules connections must not loop or merge".into());
    }
    if node.definition.as_deref() == Some("rules_attribute") {
        return scalar(choice(node, "type")?, text(node, "value")?);
    }
    if !matches!(
        node.definition.as_deref(),
        Some("rules_definition" | "rules_table" | "rules_list")
    ) {
        return Err("Unknown rules node".into());
    }
    let list = node.definition.as_deref() == Some("rules_list");
    let (rows, ids) = match control(node, "entries")? {
        GraphControlValue::List { rows, row_ids, .. } if rows.len() == row_ids.len() => {
            (rows, row_ids)
        }
        _ => return Err("Missing rule key terminal identities".into()),
    };
    if ids.iter().copied().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err("Duplicate key terminal identity".into());
    }
    let mut table = Table::new();
    let mut array = BTreeMap::new();
    for (cells, id) in rows.iter().zip(ids) {
        let [
            GraphControlValue::Text(key),
            GraphControlValue::Choice { options, selected },
            raw_control,
        ] = cells.as_slice()
        else {
            return Err("Malformed typed rule entry".into());
        };
        let raw = match raw_control {
            GraphControlValue::Text(raw) => raw.as_str(),
            GraphControlValue::Custom { kind, data } if kind == "rules_icon" => {
                data.as_str().ok_or("Icon must be an asset id")?
            }
            _ => return Err("Rule entry value must be text or an icon selector".into()),
        };
        if key.is_empty() {
            return Err("Rule entry needs a key".into());
        }
        let kind = options.get(*selected).ok_or("Invalid rule entry type")?;
        let port = node
            .ports
            .iter()
            .find(|p| p.list_item == Some(*id) && p.direction == PortDirection::Output)
            .ok_or("Missing key terminal")?;
        let links: Vec<_> = doc
            .connections
            .iter()
            .filter(|c| c.from == port.id)
            .collect();
        if links.len() > 1 {
            return Err(format!("Key {key} can have only one connection"));
        }
        let value = if let Some(link) = links.first() {
            let (child, input) = doc.port(link.to).ok_or("Missing key input")?;
            if input.key.as_deref() != Some("in") {
                return Err("Invalid rules input terminal".into());
            }
            let child_kind = child.definition.as_deref();
            let expected = match kind.as_str() {
                "Table" => "rules_table",
                "List" => "rules_list",
                _ => "rules_attribute",
            };
            if child_kind != Some(expected) {
                return Err(format!(
                    "Wrong node type connected to {key}: expected {kind}"
                ));
            }
            if expected == "rules_attribute" && choice(child, "type")? != kind {
                return Err(format!("Value type mismatch for {key}"));
            }
            build_value(child, doc, consumed)?
        } else if matches!(kind.as_str(), "Table" | "List") {
            return Err(format!("Connect a {kind} node to key {key}"));
        } else {
            scalar(kind, raw)?
        };
        if list {
            let index = key
                .parse::<usize>()
                .map_err(|_| "List keys must be numeric indices")?;
            if key != &index.to_string() {
                return Err("List indices must be canonical integers".into());
            }
            if array.insert(index, value).is_some() {
                return Err("Duplicate list index".into());
            }
        } else if table.insert(key.clone(), value).is_some() {
            return Err(format!("Duplicate rule key: {key}"));
        }
    }
    if list {
        if array.keys().copied().ne(0..array.len()) {
            return Err("List indices must be contiguous from 0".into());
        }
        Ok(Value::Array(array.into_values().collect()))
    } else {
        Ok(Value::Table(table))
    }
}
fn insert_definition(table: &mut Table, path: &[String], value: Value) -> Result<(), String> {
    let (key, rest) = path.split_first().ok_or("Empty definition path")?;
    if rest.is_empty() {
        table.insert(key.clone(), value);
        return Ok(());
    }
    let next = table
        .entry(key.clone())
        .or_insert_with(|| Value::Table(Table::new()));
    insert_definition(
        next.as_table_mut()
            .ok_or("Definition parent is not a table")?,
        rest,
        value,
    )
}
pub fn definitions() -> GraphDefinitions {
    let mut result = GraphDefinitions::default();
    for (id, title) in [
        ("rules_definition", "Definition"),
        ("rules_table", "Table"),
        ("rules_list", "List"),
        ("rules_attribute", "Set Attribute"),
    ] {
        let mut n = GraphNode::new(title, [0., 0.], [91, 86, 151, 255]);
        n.definition = Some(id.into());
        n.width = 520.;
        if id == "rules_definition" {
            let mut path = GraphRow::new(
                "Definition path (e.g. /actions/my_action)",
                GraphControlValue::Text(String::new()),
            );
            path.key = Some("path".into());
            n.rows.push(path);
        }
        if id == "rules_attribute" {
            for (key, value) in [
                (
                    "type",
                    GraphControlValue::Choice {
                        options: TYPES
                            .iter()
                            .filter(|s| !matches!(**s, "Table" | "List"))
                            .map(|s| s.to_string())
                            .collect(),
                        selected: 0,
                    },
                ),
                ("value", GraphControlValue::Text(String::new())),
            ] {
                let mut row = GraphRow::new(key, value);
                row.key = Some(key.into());
                n.rows.push(row);
            }
        } else {
            let mut entries = GraphRow::new(
                "Fields",
                GraphControlValue::List {
                    row_ids: vec![],
                    columns: vec![
                        GraphListColumn {
                            id: "key".into(),
                            label: if id == "rules_list" { "Index" } else { "Key" }.into(),
                            control: GraphControlValue::Text(String::new()),
                        },
                        GraphListColumn {
                            id: "type".into(),
                            label: "Type".into(),
                            control: GraphControlValue::Choice {
                                options: TYPES.iter().map(|s| s.to_string()).collect(),
                                selected: 0,
                            },
                        },
                        GraphListColumn {
                            id: "value".into(),
                            label: "Value".into(),
                            control: GraphControlValue::Text(String::new()),
                        },
                    ],
                    rows: vec![],
                },
            );
            entries.key = Some("entries".into());
            n.rows.push(entries);
        }
        for (key, direction, side) in [
            ("in", PortDirection::Input, PortSide::Left),
            ("out", PortDirection::Output, PortSide::Right),
        ] {
            if direction == PortDirection::Output || id == "rules_definition" {
                continue;
            }
            let mut p = GraphPort::new("", direction, side, 0.5);
            p.key = Some(key.into());
            p.kind = "rules-data".into();
            n.ports.push(p);
        }
        result
            .register_node(GraphNodeDefinition::from_template(id, "Rules", &n))
            .unwrap();
    }
    let root = crate::particle_graph::root_definition();
    result
        .register_node(GraphNodeDefinition::from_template(
            "rules_fx",
            "Particles",
            &root,
        ))
        .unwrap();
    for module in crate::particle_graph::modules() {
        let n = module.definition();
        result
            .register_node(GraphNodeDefinition::from_template(
                n.definition.as_deref().unwrap(),
                "Particles",
                &n,
            ))
            .unwrap();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_conversion_is_exact_and_round_trips() {
        let graph = RulesGraph::official();
        let baseline = crate::OFFICIAL_TOML_REFERENCE.parse::<Table>().unwrap();
        let compiled = graph.compile().unwrap();
        // FX recipes now compile to explicit emitter settings; the runtime suite
        // compares all of those settings against the old semantic translator.
        let mut expected = baseline.clone();
        expected
            .get_mut("fx")
            .unwrap()
            .as_table_mut()
            .unwrap()
            .insert("presets".into(), compiled["fx"]["presets"].clone());
        assert_eq!(compiled, expected);
        let saved = serde_json::to_string(&graph).unwrap();
        let loaded: RulesGraph = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded.compile().unwrap(), compiled);
        assert_eq!(
            graph.resolve().unwrap().actions().unwrap(),
            crate::resolve_actions(&baseline).unwrap()
        );
        assert_eq!(
            graph.resolve().unwrap().conditions().unwrap(),
            crate::resolve_conditions(&baseline).unwrap()
        );
        assert!(graph.resolve().unwrap().validation().error_count() == 0);
    }
    #[test]
    fn removal_disable_addition_and_recovery_survive_saving() {
        let mut state = RulesetState::default();
        let baseline = state.current.compile().unwrap();
        state
            .current
            .branches
            .retain(|b| !branch_path(b).unwrap().starts_with("/classes/"));
        assert!(state.current.compile().unwrap().get("classes").is_none());
        state
            .current
            .restore_definition(&state.original, "/classes/Warrior")
            .unwrap();
        assert!(
            state.current.compile().unwrap()["classes"]
                .get("Warrior")
                .is_some()
        );
        let b = state
            .current
            .branches
            .iter_mut()
            .find(|b| branch_path(b) == Some("/actions/basic_attack"))
            .unwrap();
        b.nodes[0].disabled = true;
        assert!(
            state.current.compile().unwrap()["actions"]
                .get("basic_attack")
                .is_none()
        );
        let mut root = definitions()
            .instantiate("rules_definition", [0., 0.])
            .unwrap();
        root.rows[0].value = GraphControlValue::Text("/chassis/scout".into());
        let mut branch = GraphDocument::default();
        branch.nodes.push(root);
        state.current.branches.push(branch);
        assert!(
            state.current.compile().unwrap()["chassis"]
                .get("scout")
                .is_some()
        );
        let mut state: RulesetState =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        state.restore_original();
        assert_eq!(state.current.compile().unwrap(), baseline);
        assert!(state.restore_checkpoint());
        assert!(state.current.compile().unwrap().get("chassis").is_some());
    }
    #[test]
    fn layout_and_invalid_drafts_do_not_change_original() {
        let mut state = RulesetState::default();
        let baseline = state.current.compile().unwrap();
        for b in &mut state.current.branches {
            for n in &mut b.nodes {
                n.position = [123., -456.];
                n.folded = true;
            }
        }
        assert_eq!(state.current.compile().unwrap(), baseline);
        let copy = state.current.branches[0].clone();
        state.current.branches.push(copy);
        assert!(state.current.compile().unwrap_err().contains("Overlapping"));
        state.restore_original();
        assert_eq!(state.current.compile().unwrap(), baseline);
    }
    #[test]
    fn key_connections_survive_rename_reorder_and_roundtrip() {
        let mut graph = RulesGraph::official();
        let branch = graph
            .branches
            .iter_mut()
            .find(|b| branch_path(b) == Some("/actions/basic_attack"))
            .unwrap();
        let root = branch
            .nodes
            .iter_mut()
            .find(|n| {
                matches!(
                    n.definition.as_deref(),
                    Some("rules_definition" | "rules_fx")
                )
            })
            .unwrap();
        let GraphControlValue::List { rows, row_ids, .. } = &mut root.rows[1].value else {
            panic!()
        };
        let index = rows
            .iter()
            .position(|r| r[0] == GraphControlValue::Text("result".into()))
            .unwrap();
        let item = row_ids[index];
        let terminal = root
            .ports
            .iter()
            .find(|p| p.list_item == Some(item))
            .unwrap()
            .id;
        rows[index][0] = GraphControlValue::Text("renamed_result".into());
        rows.swap(0, index);
        row_ids.swap(0, index);
        sync_ports(branch);
        assert!(branch.connections.iter().any(|c| c.from == terminal));
        assert_eq!(
            graph.compile().unwrap()["actions"]["basic_attack"]["renamed_result"]["damage"]
                .as_str(),
            Some("weapon")
        );
        let loaded: RulesGraph =
            serde_json::from_str(&serde_json::to_string(&graph).unwrap()).unwrap();
        assert_eq!(loaded.compile().unwrap(), graph.compile().unwrap());
        let branch = graph
            .branches
            .iter_mut()
            .find(|b| branch_path(b) == Some("/actions/basic_attack"))
            .unwrap();
        let root = branch
            .nodes
            .iter_mut()
            .find(|n| {
                matches!(
                    n.definition.as_deref(),
                    Some("rules_definition" | "rules_fx")
                )
            })
            .unwrap();
        let GraphControlValue::List { rows, row_ids, .. } = &mut root.rows[1].value else {
            panic!()
        };
        rows.remove(0);
        row_ids.remove(0);
        sync_ports(branch);
        assert!(!branch.connections.iter().any(|c| c.from == terminal));
        assert!(
            graph
                .compile()
                .unwrap_err()
                .contains("Connect every rules node")
        );
    }

    #[test]
    fn scalar_key_connections_override_inline_values_and_reject_wrong_types() {
        let mut graph = RulesGraph::official();
        let branch = graph
            .branches
            .iter_mut()
            .find(|b| branch_path(b) == Some("/actions/basic_attack"))
            .unwrap();
        let root = branch
            .nodes
            .iter()
            .find(|n| {
                matches!(
                    n.definition.as_deref(),
                    Some("rules_definition" | "rules_fx")
                )
            })
            .unwrap();
        let GraphControlValue::List { rows, row_ids, .. } = &root.rows[1].value else {
            panic!()
        };
        let index = rows
            .iter()
            .position(|r| r[0] == GraphControlValue::Text("cooldown".into()))
            .unwrap();
        let port = root
            .ports
            .iter()
            .find(|p| p.list_item == Some(row_ids[index]))
            .unwrap()
            .id;
        let mut value = definitions()
            .instantiate("rules_attribute", [0., 0.])
            .unwrap();
        let GraphControlValue::Choice { options, selected } = &mut value.rows[0].value else {
            panic!()
        };
        *selected = options.iter().position(|s| s == "Number").unwrap();
        value.rows[1].value = GraphControlValue::Text("0.125".into());
        branch.connections.push(GraphConnection {
            id: Uuid::new_v4(),
            from: port,
            to: value.ports[0].id,
        });
        branch.nodes.push(value);
        assert_eq!(
            graph.compile().unwrap()["actions"]["basic_attack"]["cooldown"].as_float(),
            Some(0.125)
        );
        let branch = graph
            .branches
            .iter_mut()
            .find(|b| branch_path(b) == Some("/actions/basic_attack"))
            .unwrap();
        let value = branch.nodes.last_mut().unwrap();
        let GraphControlValue::Choice { selected, .. } = &mut value.rows[0].value else {
            panic!()
        };
        *selected = 0;
        assert!(graph.compile().unwrap_err().contains("Value type mismatch"));
        let branch = graph
            .branches
            .iter_mut()
            .find(|b| branch_path(b) == Some("/actions/basic_attack"))
            .unwrap();
        branch.connections.retain(|c| c.from != port);
        branch.nodes.pop();
        assert_eq!(
            graph.compile().unwrap()["actions"]["basic_attack"]["cooldown"].as_float(),
            Some(1.0)
        );
    }
}
