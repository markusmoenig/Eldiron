//! Presentation-independent rules data. Editors can manipulate this tree
//! without creating graph nodes or losing fields they do not understand.
//!
//! Graph migration is deliberately fallible: an invalid graph draft cannot be
//! converted without discarding information. Callers must retain that draft.
use crate::{RawRuleset, ResolvedRuleset, RulesetSelection, graph::RulesGraph};
use serde::{Deserialize, Serialize};
use toml::{Table, Value};
pub use toml::{Table as RulesTable, Value as RulesValue};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RulesDocument {
    pub version: u32,
    #[serde(with = "table_as_toml")]
    table: Table,
}

// JSON cannot distinguish integral floats from integers when deserializing a
// generic TOML value. Store the value tree as TOML to preserve its types.
mod table_as_toml {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};
    use toml::Table;

    pub fn serialize<S: Serializer>(table: &Table, serializer: S) -> Result<S::Ok, S::Error> {
        let source = toml::to_string(table).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&source)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Table, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}

/// Paths use components rather than dotted strings so identifiers containing
/// dots or slashes remain unambiguous. List indices are explicit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathPart {
    Key(String),
    Index(usize),
}

/// Project-owned authoring data and recovery snapshots. The source is kept as
/// text so that incomplete edits survive saving and can be repaired later.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RulesState {
    pub current: String,
    pub original: String,
    #[serde(default)]
    pub checkpoints: Vec<String>,
}

impl Default for RulesState {
    fn default() -> Self {
        let current = RulesDocument::from_graph(&RulesGraph::official())
            .and_then(|document| document.to_toml())
            .expect("Bundled rules must be a serializable document");
        Self {
            original: current.clone(),
            current,
            checkpoints: Vec::new(),
        }
    }
}

impl RulesState {
    pub fn document(&self) -> Result<RulesDocument, String> {
        RulesDocument::parse(&self.current)
    }

    /// Commit a data editor operation without altering the current draft if it
    /// cannot be represented. Validation is separate: invalid references can be
    /// saved and fixed in a later operation.
    pub fn edit<F>(&mut self, operation: F) -> Result<(), String>
    where
        F: FnOnce(&mut Table) -> Result<(), String>,
    {
        let mut document = self.document()?;
        document.edit(operation)?;
        self.current = document.to_toml()?;
        Ok(())
    }

    pub fn checkpoint(&mut self) {
        if self.checkpoints.last() != Some(&self.current) {
            self.checkpoints.push(self.current.clone());
        }
    }

    pub fn restore_original(&mut self) {
        self.checkpoint();
        self.current = self.original.clone();
    }

    pub fn recover(&mut self) -> bool {
        let Some(mut previous) = self.checkpoints.pop() else {
            return false;
        };
        std::mem::swap(&mut previous, &mut self.current);
        self.checkpoints.push(previous);
        true
    }
}

impl RulesDocument {
    pub fn from_table(table: Table) -> Self {
        Self { version: 1, table }
    }

    pub fn from_graph(graph: &RulesGraph) -> Result<Self, String> {
        Ok(Self::from_table(graph.compile()?))
    }

    pub fn parse(source: &str) -> Result<Self, String> {
        source
            .parse::<Table>()
            .map(Self::from_table)
            .map_err(|e| e.to_string())
    }

    pub fn to_toml(&self) -> Result<String, String> {
        self.check_version()?;
        toml::to_string(&self.table).map_err(|e| e.to_string())
    }

    pub fn table(&self) -> &Table {
        &self.table
    }

    /// Apply an edit to a copy and commit it only when the operation succeeds.
    /// This supports additions, deletions and list reordering without exposing
    /// partially applied edits to another view.
    pub fn edit<F>(&mut self, edit: F) -> Result<(), String>
    where
        F: FnOnce(&mut Table) -> Result<(), String>,
    {
        self.check_version()?;
        let mut candidate = self.table.clone();
        edit(&mut candidate)?;
        self.table = candidate;
        Ok(())
    }

    pub fn get(&self, path: &[PathPart]) -> Option<&Value> {
        let (PathPart::Key(key), rest) = path.split_first()? else {
            return None;
        };
        let mut value = self.table.get(key)?;
        for part in rest {
            value = match part {
                PathPart::Key(key) => value.as_table()?.get(key)?,
                PathPart::Index(index) => value.as_array()?.get(*index)?,
            };
        }
        Some(value)
    }

    /// Replace an existing value; this never silently creates or overwrites a
    /// parent. Insertions and removals can be expressed by replacing a table or
    /// list, allowing an editor to commit a whole operation atomically.
    pub fn replace(&mut self, path: &[PathPart], replacement: Value) -> Result<Value, String> {
        self.check_version()?;
        let (PathPart::Key(key), rest) = path.split_first().ok_or("Empty rules path")? else {
            return Err("A rules path must start with a key".into());
        };
        let mut value = self.table.get_mut(key).ok_or("Rules path does not exist")?;
        for part in rest {
            value = match part {
                PathPart::Key(key) => value.as_table_mut().and_then(|t| t.get_mut(key)),
                PathPart::Index(index) => value.as_array_mut().and_then(|a| a.get_mut(*index)),
            }
            .ok_or("Rules path does not exist")?;
        }
        Ok(std::mem::replace(value, replacement))
    }

    pub fn resolve(&self) -> Result<ResolvedRuleset, String> {
        self.check_version()?;
        let schema = self
            .table
            .get("ruleset")
            .and_then(|v| v.get("schema_version"))
            .and_then(Value::as_str)
            .unwrap_or("1");
        ResolvedRuleset::from_raw(
            RawRuleset::from_table(self.table.clone()),
            RulesetSelection {
                id: "project.rules".into(),
                schema_version: schema.into(),
                source: "project".into(),
                ..Default::default()
            },
        )
    }

    fn check_version(&self) -> Result<(), String> {
        if self.version == 1 {
            Ok(())
        } else {
            Err("Unsupported rules document version".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(s: &str) -> PathPart {
        PathPart::Key(s.into())
    }

    #[test]
    fn official_graph_migrates_without_changing_runtime_values() {
        let graph = RulesGraph::official();
        let doc = RulesDocument::from_graph(&graph).unwrap();
        assert_eq!(doc.table(), &graph.compile().unwrap());
        assert!(doc.resolve().unwrap().validation().is_ok());
        assert_eq!(RulesDocument::parse(&doc.to_toml().unwrap()).unwrap(), doc);
        let json = serde_json::to_string(&doc).unwrap();
        assert_eq!(serde_json::from_str::<RulesDocument>(&json).unwrap(), doc);
    }

    #[test]
    fn edits_preserve_unknown_values_and_failed_paths_do_not_mutate() {
        let mut doc = RulesDocument::parse("[custom]\n\"a.b\" = [1, 2]\nextra = 'kept'\n").unwrap();
        let path = [key("custom"), key("a.b"), PathPart::Index(1)];
        assert_eq!(
            doc.replace(&path, Value::Integer(3)).unwrap(),
            Value::Integer(2)
        );
        assert_eq!(doc.get(&path), Some(&Value::Integer(3)));
        let before = doc.clone();
        assert!(
            doc.replace(&[key("custom"), key("missing")], Value::Integer(0))
                .is_err()
        );
        assert_eq!(doc, before);
        assert_eq!(
            doc.get(&[key("custom"), key("extra")])
                .and_then(Value::as_str),
            Some("kept")
        );
    }

    #[test]
    fn invalid_graph_is_not_silently_migrated() {
        let mut graph = RulesGraph::official();
        graph.branches.push(graph.branches[0].clone());
        assert!(RulesDocument::from_graph(&graph).is_err());
    }

    #[test]
    fn failed_multi_field_edit_is_atomic() {
        let mut doc = RulesDocument::parse("[custom]\nvalue = 1\n").unwrap();
        let before = doc.clone();
        let result = doc.edit(|table| {
            table.clear();
            Err("cancelled".into())
        });
        assert!(result.is_err());
        assert_eq!(doc, before);
    }

    #[test]
    fn state_preserves_invalid_draft_and_recovery_across_serialization() {
        let mut state = RulesState::default();
        let original = state.current.clone();
        state.current = "[unfinished".into();
        state.checkpoint();
        state.current = "[another".into();
        let mut loaded: RulesState =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert!(loaded.document().is_err());
        assert!(loaded.recover());
        assert_eq!(loaded.current, "[unfinished");
        assert!(loaded.recover());
        assert_eq!(loaded.current, "[another");
        loaded.restore_original();
        assert_eq!(loaded.current, original);
        assert!(
            loaded
                .document()
                .unwrap()
                .resolve()
                .unwrap()
                .validation()
                .is_ok()
        );
    }

    #[test]
    fn editing_state_is_transactional() {
        let mut state = RulesState::default();
        let before = state.current.clone();
        assert!(
            state
                .edit(|table| {
                    table.clear();
                    Err("cancelled".into())
                })
                .is_err()
        );
        assert_eq!(state.current, before);
        state.current = "[invalid".into();
        assert!(state.edit(|_| Ok(())).is_err());
        assert_eq!(state.current, "[invalid");
    }
}
