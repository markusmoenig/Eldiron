//! Shared view of grouped rules data. A definition key is independent of its
//! storage group and of project-template UUIDs or live item instance IDs.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use toml::{Table, Value};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DefinitionKey {
    pub domain: String,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub key: Option<DefinitionKey>,
    pub path: Vec<String>,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct Catalogue {
    pub entries: Vec<Entry>,
    index: BTreeMap<DefinitionKey, Vec<usize>>,
}

impl Catalogue {
    pub fn from_table(root: &Table) -> Self {
        let mut catalogue = Self::default();
        for (domain, value) in root {
            let Some(table) = value.as_table() else {
                continue;
            };
            for (id, value) in table {
                if domain == "items" || (domain == "fx" && id == "presets") {
                    if let Some(group) = value.as_table() {
                        for (child, value) in group {
                            catalogue.add(
                                domain,
                                child,
                                vec![domain.clone(), id.clone(), child.clone()],
                                value,
                                value.is_table(),
                            );
                        }
                    }
                } else {
                    let definition = matches!(
                        domain.as_str(),
                        "actions"
                            | "animals"
                            | "recipes"
                            | "classes"
                            | "races"
                            | "professions"
                            | "skills"
                            | "spells"
                            | "abilities"
                            | "conditions"
                            | "enchantments"
                            | "knowledge"
                            | "teaching"
                            | "traits"
                            | "resources"
                            | "visual_templates"
                            | "invocation_schemes"
                            | "equipment_families"
                            | "resource_profiles"
                            | "resource_pools"
                    );
                    catalogue.add(
                        domain,
                        id,
                        vec![domain.clone(), id.clone()],
                        value,
                        definition && value.is_table(),
                    );
                }
            }
        }
        catalogue
    }

    fn add(&mut self, domain: &str, id: &str, path: Vec<String>, value: &Value, definition: bool) {
        let key = definition.then(|| DefinitionKey {
            domain: domain.into(),
            id: id.into(),
        });
        if let Some(key) = &key {
            self.index
                .entry(key.clone())
                .or_default()
                .push(self.entries.len());
        }
        self.entries.push(Entry {
            key,
            path,
            name: value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(id)
                .into(),
        });
    }

    pub fn ids(&self, domain: &str) -> Vec<String> {
        self.index
            .keys()
            .filter(|key| key.domain == domain)
            .map(|key| key.id.clone())
            .collect()
    }

    /// Ambiguous IDs must never silently resolve to whichever group came first.
    pub fn resolve(&self, key: &DefinitionKey) -> Result<Option<&Entry>, String> {
        match self.index.get(key).map(Vec::as_slice) {
            None => Ok(None),
            Some([index]) => Ok(Some(&self.entries[*index])),
            _ => Err(format!("Ambiguous {} definition '{}'", key.domain, key.id)),
        }
    }

    pub fn conflicts(&self) -> Vec<DefinitionKey> {
        self.index
            .iter()
            .filter(|(_, indices)| indices.len() > 1)
            .map(|(key, _)| key.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_identity_survives_group_changes_and_detects_collisions() {
        let mut root: Table = "[items.armor.bone_helm]\nname='Bone Helm'\n"
            .parse()
            .unwrap();
        let key = DefinitionKey {
            domain: "items".into(),
            id: "bone_helm".into(),
        };
        let first = Catalogue::from_table(&root);
        let item = root["items"]["armor"]
            .as_table_mut()
            .unwrap()
            .remove("bone_helm")
            .unwrap();
        root["items"].as_table_mut().unwrap().insert(
            "custom".into(),
            Value::Table(Table::from_iter([("bone_helm".into(), item.clone())])),
        );
        let moved = Catalogue::from_table(&root);
        assert_eq!(
            first.resolve(&key).unwrap().unwrap().key,
            moved.resolve(&key).unwrap().unwrap().key
        );
        root["items"]["armor"]
            .as_table_mut()
            .unwrap()
            .insert("bone_helm".into(), item);
        let duplicate = Catalogue::from_table(&root);
        assert_eq!(duplicate.conflicts(), vec![key.clone()]);
        assert!(duplicate.resolve(&key).is_err());
    }
    #[test]
    fn large_catalogue_keeps_unknown_fields_and_resolves_stable_keys() {
        let mut group = Table::new();
        for index in 0..2000 {
            group.insert(
                format!("item_{index:04}"),
                Value::Table(Table::from_iter([(
                    "name".into(),
                    Value::String(format!("Item {index}")),
                )])),
            );
        }
        let root = Table::from_iter([
            (
                "items".into(),
                Value::Table(Table::from_iter([("custom".into(), Value::Table(group))])),
            ),
            (
                "future".into(),
                Value::Table(Table::from_iter([("data".into(), Value::Integer(7))])),
            ),
        ]);
        let catalogue = Catalogue::from_table(&root);
        assert_eq!(catalogue.ids("items").len(), 2000);
        let key = DefinitionKey {
            domain: "items".into(),
            id: "item_1999".into(),
        };
        assert_eq!(
            catalogue.resolve(&key).unwrap().unwrap().path,
            vec!["items", "custom", "item_1999"]
        );
        assert!(
            catalogue
                .entries
                .iter()
                .any(|e| e.key.is_none() && e.path == vec!["future", "data"])
        );
        assert!(catalogue.conflicts().is_empty());
    }
}
