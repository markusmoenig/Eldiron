//! Passive wearer contributions. Item ratings and saved actor values stay separate.
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use toml::Table;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub attribute: String,
    pub item_attribute: String,
}

pub fn projections(root: &Table, category: &Table) -> Result<Vec<Projection>, String> {
    let values: Vec<Projection> = category
        .get("contributions")
        .map(|v| v.clone().try_into())
        .transpose()
        .map_err(|e| format!("Invalid equipment contributions: {e}"))?
        .unwrap_or_default();
    let mut seen = BTreeSet::new();
    for value in &values {
        if !declared_attribute(root, &value.attribute)
            || !declared_attribute(root, &value.item_attribute)
            || !seen.insert(value.attribute.clone())
        {
            return Err(
                "Equipment contributions need declared numeric attributes and unique targets"
                    .into(),
            );
        }
    }
    Ok(values)
}

pub fn declared_attribute(root: &Table, id: &str) -> bool {
    root.get("attributes")
        .and_then(|v| v.as_table())
        .is_some_and(|attributes| {
            attributes
                .get("defaults")
                .and_then(|v| v.get(id))
                .is_some_and(|v| v.is_integer() || v.is_float())
                || attributes
                    .iter()
                    .filter_map(|(_, v)| v.as_array())
                    .flatten()
                    .any(|v| v.as_str() == Some(id))
        })
        || root.get("derived_stats").and_then(|v| v.get(id)).is_some()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Modifier {
    attribute: String,
    #[serde(default)]
    add: f32,
    #[serde(default = "one")]
    multiply: f32,
    minimum: Option<f32>,
    maximum: Option<f32>,
}
fn one() -> f32 {
    1.0
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Enchantment {
    name: String,
    epithet: Option<String>,
    #[serde(default = "yes")]
    requires_positive_condition: bool,
    #[serde(default)]
    allowed_categories: Vec<String>,
    modifiers: Vec<Modifier>,
    #[serde(default)]
    knowledge_grants: Vec<String>,
}
fn yes() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedEnchantment {
    pub id: String,
    pub name: String,
    pub epithet: Option<String>,
    pub requires_positive_condition: bool,
    pub allowed_categories: BTreeSet<String>,
    pub modifiers: Vec<crate::ResolvedConditionModifier>,
    pub knowledge_grants: BTreeSet<String>,
}

pub fn resolve_enchantments(root: &Table) -> Result<BTreeMap<String, ResolvedEnchantment>, String> {
    let mut result = BTreeMap::new();
    let Some(definitions) = root.get("enchantments") else {
        return Ok(result);
    };
    for (id, value) in definitions
        .as_table()
        .ok_or("enchantments must be a table")?
    {
        let parsed: Enchantment = value
            .clone()
            .try_into()
            .map_err(|e| format!("enchantments.{id}: {e}"))?;
        if id.trim().is_empty()
            || id.len() > 128
            || parsed.name.trim().is_empty()
            || parsed
                .epithet
                .as_ref()
                .is_some_and(|s| s.trim().is_empty() || s != s.trim() || s.len() > 128)
            || (parsed.modifiers.is_empty() && parsed.knowledge_grants.is_empty())
        {
            return Err(format!(
                "enchantments.{id} needs an identity, name and modifiers or knowledge grants"
            ));
        }
        for category in &parsed.allowed_categories {
            if !["armor_categories", "weapon_categories"]
                .iter()
                .any(|kind| {
                    root.get("equipment")
                        .and_then(|v| v.get(kind))
                        .and_then(|v| v.get(category))
                        .is_some()
                })
            {
                return Err(format!(
                    "enchantments.{id} references unknown category '{category}'"
                ));
            }
        }
        let mut targets = BTreeSet::new();
        let mut grants = BTreeSet::new();
        for knowledge in &parsed.knowledge_grants {
            if !grants.insert(knowledge.clone())
                || root
                    .get("knowledge")
                    .and_then(|v| v.get(knowledge))
                    .is_none()
            {
                return Err(format!(
                    "enchantments.{id} has duplicate or unknown knowledge grants"
                ));
            }
        }
        let mut modifiers = Vec::new();
        for m in parsed.modifiers {
            if !declared_attribute(root, &m.attribute)
                || !targets.insert(m.attribute.clone())
                || ![Some(m.add), Some(m.multiply), m.minimum, m.maximum]
                    .into_iter()
                    .flatten()
                    .all(f32::is_finite)
            {
                return Err(format!(
                    "enchantments.{id} needs unique declared targets and finite modifiers"
                ));
            }
            modifiers.push(crate::ResolvedConditionModifier {
                attribute: m.attribute,
                add: m.add,
                multiply: m.multiply,
                minimum: m.minimum,
                maximum: m.maximum,
            });
        }
        result.insert(
            id.clone(),
            ResolvedEnchantment {
                id: id.clone(),
                name: parsed.name,
                epithet: parsed.epithet,
                requires_positive_condition: parsed.requires_positive_condition,
                allowed_categories: parsed.allowed_categories.into_iter().collect(),
                modifiers,
                knowledge_grants: grants,
            },
        );
    }
    Ok(result)
}

pub(crate) fn validate(root: &Table, report: &mut crate::RulesetValidationReport) {
    let definitions = match resolve_enchantments(root) {
        Ok(value) => value,
        Err(error) => {
            report.error("enchantments", error);
            return;
        }
    };
    if let Some(groups) = root.get("items").and_then(|v| v.as_table()) {
        for (group, items) in groups {
            for (id, item) in items.as_table().into_iter().flatten() {
                let Some(presets) = item.get("enchantments") else {
                    continue;
                };
                let path = format!("items.{group}.{id}.enchantments");
                let Some(presets) = presets.as_array() else {
                    report.error(&path, "Must be an array of enchantment IDs");
                    continue;
                };
                let mut seen = BTreeSet::new();
                for preset in presets {
                    let valid = preset
                        .as_str()
                        .and_then(|id| definitions.get(id))
                        .is_some_and(|definition| {
                            seen.insert(&definition.id)
                                && (definition.allowed_categories.is_empty()
                                    || item
                                        .get("category")
                                        .and_then(|v| v.as_str())
                                        .is_some_and(|c| definition.allowed_categories.contains(c)))
                        });
                    if !valid {
                        report.error(&path, "Unknown, duplicate or incompatible enchantment");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_invalid_modifier_references_types_and_duplicate_targets() {
        for modifier in [
            "{attribute='missing',add=2}",
            "{attribute='STR',add=inf}",
            "{attribute='STR',add=2,unknown=true}",
        ] {
            let root = format!("[attributes]\nprimary=['STR']\n[enchantments.strength]\nname='Strength'\nmodifiers=[{modifier}]").parse().unwrap();
            assert!(super::resolve_enchantments(&root).is_err());
        }
    }
}
