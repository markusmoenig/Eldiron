use crate::value::{Value, ValueContainer};
use rustc_hash::FxHashMap;
use std::{fs, path::Path};
use toml::Value as TomlValue;

/// Convenience alias for grouped value containers loaded from TOML.
pub type ValueGroups = FxHashMap<String, ValueContainer>;

/// Loads simple key/value groups from a TOML document into ValueContainers.
/// - Top-level tables become entries in the map keyed by their table name.
/// - Integers and floats are both stored as `Value::Float` for convenience.
pub struct ValueTomlLoader;

impl ValueTomlLoader {
    /// Parse from a file on disk.
    pub fn from_file(path: &Path) -> Result<ValueGroups, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::from_str(&content)
    }

    /// Parse from a TOML string.
    pub fn from_str(src: &str) -> Result<ValueGroups, String> {
        let doc: TomlValue = toml::from_str(src).map_err(|e: toml::de::Error| e.to_string())?;
        let root = doc
            .as_table()
            .ok_or_else(|| "TOML root must be a table".to_string())?;

        let mut groups: ValueGroups = FxHashMap::default();

        for (group_name, group_val) in root {
            let Some(table) = group_val.as_table() else {
                continue; // skip non-table entries at root level
            };

            let mut container = ValueContainer::default();
            for (key, val) in table {
                if let Some(v) = toml_value_to_value(val) {
                    container.set(key, v);
                }
            }

            groups.insert(group_name.clone(), container);
        }

        Ok(groups)
    }
}

fn toml_value_to_value(val: &TomlValue) -> Option<Value> {
    match val {
        TomlValue::Integer(i) => Some(Value::Float(*i as f32)),
        TomlValue::Float(f) => Some(Value::Float(*f as f32)),
        TomlValue::Boolean(b) => Some(Value::Bool(*b)),
        TomlValue::String(s) => Some(Value::Str(s.clone())),
        TomlValue::Array(arr) => {
            // Try arrays of numbers -> Vec3, arrays of strings -> StrArray
            if arr.len() == 3
                && arr
                    .iter()
                    .all(|v| matches!(v, TomlValue::Integer(_) | TomlValue::Float(_)))
            {
                let mut out = [0.0f32; 3];
                for (i, v) in arr.iter().take(3).enumerate() {
                    out[i] = number_to_f32(v);
                }
                Some(Value::Vec3(out))
            } else {
                let strings: Option<Vec<String>> = arr
                    .iter()
                    .map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                strings.map(Value::StrArray)
            }
        }
        _ => None,
    }
}

fn number_to_f32(v: &TomlValue) -> f32 {
    match v {
        TomlValue::Integer(i) => *i as f32,
        TomlValue::Float(f) => *f as f32,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_groups() {
        let src = r#"
[ui]
role = "game"

[camera]
type = "firstp"
"#;

        let groups = ValueTomlLoader::from_str(src).expect("parse toml");
        assert!(groups.get("ui").is_some());
        assert!(groups.get("camera").is_some());
        assert_eq!(
            groups
                .get("camera")
                .unwrap()
                .get_str_default("type".into(), "".into()),
            "firstp"
        );
    }
}
