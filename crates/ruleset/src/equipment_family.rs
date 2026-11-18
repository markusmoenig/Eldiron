//! Guided authoring compiles into the ordinary grouped item/recipe/action
//! schema. It does not introduce a second equipment or crafting executor.
use crate::{catalogue::Catalogue, document::RulesDocument};
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoneArmorFamily {
    /// Stable ID prefix. Changing display_name never changes member identities.
    pub id: String,
    pub display_name: String,
    pub plate: String,
    pub strap: String,
    pub skill: String,
    /// Only these classes receive the new category. An unrestricted class
    /// (no allowed_armor field) stays unrestricted.
    pub classes: Vec<String>,
    pub create_missing_materials: bool,
}

impl Default for BoneArmorFamily {
    fn default() -> Self {
        Self {
            id: "bone".into(),
            display_name: "Bone".into(),
            plate: "bone_plate".into(),
            strap: "leather_strap".into(),
            skill: "armorsmithing".into(),
            classes: Vec::new(),
            create_missing_materials: true,
        }
    }
}

pub const BONE_PIECES: [(&str, &str, &str, i64, i64, i64); 5] = [
    ("helm", "Helm", "head", 1, 3, 2),
    ("cuirass", "Cuirass", "torso", 4, 8, 4),
    ("greaves", "Greaves", "legs", 3, 6, 3),
    ("gauntlets", "Gauntlets", "hands", 1, 2, 2),
    ("boots", "Boots", "feet", 1, 3, 2),
];

#[derive(Clone, Debug)]
pub struct FamilyPlan {
    pub item_ids: Vec<String>,
    pub added_materials: Vec<String>,
    pub summary: Vec<String>,
    // Compare before applying: a preview must not overwrite intervening edits.
    before: RulesDocument,
    after: RulesDocument,
}

fn identifier(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        && id.as_bytes()[0].is_ascii_lowercase()
}
fn string(value: impl Into<String>) -> Value {
    Value::String(value.into())
}
fn table(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Table(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn branch<'a>(root: &'a mut Table, path: &[&str]) -> Result<&'a mut Table, String> {
    let mut current = root;
    for key in path {
        current = current
            .entry(key.to_string())
            .or_insert_with(|| Value::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| format!("{} must be a table", path.join(".")))?;
    }
    Ok(current)
}
fn insert(root: &mut Table, path: &[&str], id: &str, value: Value) -> Result<(), String> {
    let parent = branch(root, path)?;
    if parent.contains_key(id) {
        return Err(format!(
            "{}.{} already exists; nothing was changed",
            path.join("."),
            id
        ));
    }
    parent.insert(id.into(), value);
    Ok(())
}
fn quantity(item: &str, amount: i64) -> Value {
    table([("item", string(item)), ("quantity", Value::Integer(amount))])
}

impl BoneArmorFamily {
    pub fn preview(&self, document: &RulesDocument) -> Result<FamilyPlan, String> {
        for id in [&self.id] {
            if !identifier(id) {
                return Err(format!(
                    "Invalid stable ID '{id}'; use lowercase letters, digits and underscores, starting with a letter"
                ));
            }
        }
        if self.display_name.trim().is_empty() {
            return Err("A family display name is required".into());
        }
        if self.plate.trim().is_empty() || self.strap.trim().is_empty() {
            return Err("Both ingredient IDs are required".into());
        }
        if self.plate == self.strap {
            return Err("Plate and binding ingredients must be distinct".into());
        }
        let catalogue = Catalogue::from_table(document.table());
        if !catalogue.conflicts().is_empty() {
            return Err("Resolve duplicate definition IDs before creating a family".into());
        }
        if !catalogue.ids("skills").contains(&self.skill) {
            return Err(format!("Unknown crafting skill '{}'", self.skill));
        }
        let items = catalogue.ids("items");
        let mut after = document.clone();
        let mut added_materials = Vec::new();
        let mut item_ids = Vec::new();
        let mut summary = Vec::new();
        after.edit(|root| {
            let armor_attribute = root
                .get("attributes")
                .and_then(|v| v.get("roles"))
                .and_then(|v| v.get("armor"))
                .and_then(Value::as_str)
                .ok_or("The ruleset must declare attributes.roles.armor")?
                .to_string();
            let slots = root
                .get("equipment")
                .and_then(|v| v.get("armor_slots"))
                .and_then(Value::as_array)
                .ok_or("The ruleset must declare equipment.armor_slots")?;
            for (_, _, slot, _, _, _) in BONE_PIECES {
                if !slots.iter().any(|v| v.as_str() == Some(slot)) {
                    return Err(format!("Required armor slot '{slot}' is missing"));
                }
            }
            for (id, name) in [(&self.plate, "Bone Plate"), (&self.strap, "Leather Strap")] {
                if !items.contains(id) {
                    if !identifier(id) { return Err(format!("Define ingredient '{id}' in the catalogue before using it, or choose a lowercase ID for a new ingredient")); }
                    if !self.create_missing_materials {
                        return Err(format!("Ingredient '{id}' is missing"));
                    }
                    insert(
                        root,
                        &["items", "materials"],
                        id,
                        table([
                            ("name", string(name)),
                            (
                                "description",
                                string("Requires an authored world source or production chain."),
                            ),
                            ("category", string("crafting_material")),
                            ("slot", string("material")),
                            ("max_stack", Value::Integer(100)),
                            (
                                "attributes",
                                table([
                                    ("stackable", Value::Boolean(true)),
                                    ("quantity", Value::Integer(1)),
                                ]),
                            ),
                        ]),
                    )?;
                    added_materials.push(id.clone());
                }
            }
            let category = format!("{}_armor", self.id);
            insert(
                root,
                &["equipment", "armor_categories"],
                &category,
                table([
                    ("crafting_family", string(&self.skill)),
                    ("armor_bonus", Value::Integer(0)),
                    ("dex_penalty", Value::Integer(0)),
                    ("requires_positive_condition", Value::Boolean(true)),
                    ("contributions", Value::Array(vec![table([
                        ("attribute", string(&armor_attribute)),
                        ("item_attribute", string(&armor_attribute)),
                    ])])),
                ]),
            )?;
            for (suffix, title, slot, protection, plates, straps) in BONE_PIECES {
                let id = format!("{}_{suffix}", self.id);
                if items.contains(&id) || id == self.plate || id == self.strap {
                    return Err(format!(
                        "Item '{id}' already exists or overlaps an ingredient"
                    ));
                }
                let recipe = format!("make_{id}");
                insert(
                    root,
                    &["items", "armor"],
                    &id,
                    table([
                        (
                            "name",
                            string(format!("{} {title}", self.display_name.trim())),
                        ),
                        (
                            "description",
                            string(format!(
                                "{} armor, assembled from plates and bindings.",
                                self.display_name.trim()
                            )),
                        ),
                        ("category", string(&category)),
                        ("slot", string(slot)),
                        ("rarity", string("common")),
                        (
                            "attributes",
                            Value::Table(Table::from_iter([(
                                armor_attribute.clone(),
                                Value::Integer(protection),
                            )])),
                        ),
                    ]),
                )?;
                insert(
                    root,
                    &["recipes"],
                    &recipe,
                    table([
                        (
                            "name",
                            string(format!("Make {} {title}", self.display_name.trim())),
                        ),
                        ("skill", string(&self.skill)),
                        ("required_skill", Value::Integer(0)),
                        ("recommended_skill", Value::Integer(10)),
                        ("difficulty", Value::Integer(10)),
                        (
                            "consumes",
                            Value::Array(vec![
                                quantity(&self.plate, plates),
                                quantity(&self.strap, straps),
                            ]),
                        ),
                        ("produces", Value::Array(vec![quantity(&id, 1)])),
                    ]),
                )?;
                insert(
                    root,
                    &["actions"],
                    &format!("craft_{id}"),
                    table([
                        (
                            "name",
                            string(format!("Craft {} {title}", self.display_name.trim())),
                        ),
                        ("kind", string("craft")),
                        ("recipe", string(&recipe)),
                        ("target", string("self")),
                        ("cooldown", Value::Float(0.0)),
                    ]),
                )?;
                summary.push(format!(
                    "{id}: {slot}, protection {protection}, {plates} {} + {straps} {}",
                    self.plate, self.strap
                ));
                item_ids.push(id);
            }
            for class in &self.classes {
                let class_rules = root
                    .get_mut("classes")
                    .and_then(Value::as_table_mut)
                    .and_then(|t| t.get_mut(class))
                    .and_then(Value::as_table_mut)
                    .ok_or_else(|| format!("Unknown class '{class}'"))?;
                if let Some(allowed) = class_rules.get_mut("allowed_armor") {
                    let allowed = allowed
                        .as_array_mut()
                        .ok_or("allowed_armor must be an array")?;
                    if !allowed.iter().any(|v| v.as_str() == Some(&category)) {
                        allowed.push(string(&category));
                    }
                }
                let commands = branch(class_rules, &["action_bar"])?
                    .entry("main".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()))
                    .as_array_mut()
                    .ok_or("Class action_bar.main must be an array")?;
                for id in &item_ids {
                    let command = string(format!("rules.craft_{id}"));
                    if !commands.contains(&command) {
                        commands.push(command);
                    }
                }
            }
            insert(
                root,
                &["equipment_families"],
                &category,
                table([
                    (
                        "name",
                        string(format!("{} Armor", self.display_name.trim())),
                    ),
                    ("category", string(&category)),
                    (
                        "required_members",
                        Value::Array(item_ids.iter().map(string).collect()),
                    ),
                    (
                        "required_slots",
                        Value::Array(BONE_PIECES.iter().map(|p| string(p.2)).collect()),
                    ),
                ]),
            )?;
            let report = crate::validate_ruleset(root);
            if !report.is_ok() {
                return Err(report
                    .issues
                    .iter()
                    .filter(|i| i.severity == crate::RulesetValidationSeverity::Error)
                    .map(|i| format!("{}: {}", i.path, i.message))
                    .collect::<Vec<_>>()
                    .join("\n"));
            }
            Ok(())
        })?;
        summary.push("Full set: 22 plates, 13 straps, protection 10. Adds five recipes and five craft actions.".into());
        summary.push(format!(
            "Armor and crafting command grants: {}",
            if self.classes.is_empty() {
                "none (unrestricted classes remain unrestricted)".into()
            } else {
                self.classes.join(", ")
            }
        ));
        summary.push("World sources are required for both ingredients, including their complete production chains. This guide grants no materials, vendor stock, knowledge, tools, stations or map sources.".into());
        summary.push("Projects protection through shared effective values. Repair, wear, liquids and enchantments need separate content declarations.".into());
        Ok(FamilyPlan {
            item_ids,
            added_materials,
            summary,
            before: document.clone(),
            after,
        })
    }
}

impl FamilyPlan {
    pub fn apply(&self, document: &mut RulesDocument) -> Result<(), String> {
        if document != &self.before {
            return Err("Rules changed since the preview; preview again".into());
        }
        *document = self.after.clone();
        Ok(())
    }
    pub fn document(&self) -> &RulesDocument {
        &self.after
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::RulesState;
    fn test_family() -> BoneArmorFamily {
        BoneArmorFamily {
            id: "test_bone".into(),
            plate: "test_bone_plate".into(),
            strap: "test_leather_strap".into(),
            ..BoneArmorFamily::default()
        }
    }
    #[test]
    fn complete_family_is_valid_atomic_and_survives_reload() {
        let mut document = RulesState::default().document().unwrap();
        let mut spec = test_family();
        spec.classes.push("Warrior".into());
        let plan = spec.preview(&document).unwrap();
        assert_eq!(plan.item_ids.len(), 5);
        assert_eq!(plan.added_materials.len(), 2);
        plan.apply(&mut document).unwrap();
        assert!(crate::validate_ruleset(document.table()).is_ok());
        let reloaded = RulesDocument::parse(&document.to_toml().unwrap()).unwrap();
        assert_eq!(document, reloaded);
        assert_eq!(
            reloaded.table()["items"]["armor"]["test_bone_cuirass"]["attributes"]["ARMOR"]
                .as_integer(),
            Some(4)
        );
        assert!(
            reloaded.table()["classes"]["Warrior"]["allowed_armor"]
                .as_array()
                .unwrap()
                .contains(&string("test_bone_armor"))
        );
        let before = document.clone();
        assert!(spec.preview(&document).is_err());
        assert_eq!(document, before);
        assert!(plan.apply(&mut document).is_err());
    }
    #[test]
    fn collisions_missing_dependencies_and_stale_previews_never_partially_apply() {
        let document = RulesState::default().document().unwrap();
        let mut spec = test_family();
        spec.create_missing_materials = false;
        assert!(spec.preview(&document).is_err());
        spec.create_missing_materials = true;
        spec.skill = "unknown".into();
        assert!(spec.preview(&document).is_err());
        spec.skill = "armorsmithing".into();
        let plan = spec.preview(&document).unwrap();
        let mut changed = document.clone();
        changed
            .edit(|t| {
                t.insert("custom".into(), string("preserved"));
                Ok(())
            })
            .unwrap();
        let before = changed.clone();
        assert!(plan.apply(&mut changed).is_err());
        assert_eq!(changed, before);
        let mut collision = document.clone();
        collision
            .edit(|t| insert(t, &["actions"], "craft_test_bone_boots", table([])))
            .unwrap();
        assert!(spec.preview(&collision).is_err());
        assert!(
            !collision.table()["items"]["armor"]
                .as_table()
                .unwrap()
                .contains_key("test_bone_helm")
        );
    }
    #[test]
    fn custom_armor_role_unrestricted_classes_and_unknown_fields_are_preserved() {
        let mut document = RulesState::default().document().unwrap();
        document
            .edit(|root| {
                root["attributes"]["roles"]["armor"] = string("protection");
                root["attributes"]["defaults"]
                    .as_table_mut()
                    .unwrap()
                    .insert("protection".into(), Value::Integer(0));
                let mut human = root["classes"]["Citizen"].as_table().unwrap().clone();
                human.remove("allowed_armor");
                root["classes"]
                    .as_table_mut()
                    .unwrap()
                    .insert("Human".into(), Value::Table(human));
                root.insert(
                    "future_extension".into(),
                    table([("value", Value::Float(1.0))]),
                );
                Ok(())
            })
            .unwrap();
        let classes = document.table()["classes"].clone();
        let mut spec = test_family();
        spec.classes.push("Human".into());
        spec.preview(&document)
            .unwrap()
            .apply(&mut document)
            .unwrap();
        assert!(
            document.table()["classes"]["Human"]
                .get("allowed_armor")
                .is_none()
        );
        assert_eq!(document.table()["classes"]["Warrior"], classes["Warrior"]);
        assert_eq!(
            document.table()["items"]["armor"]["test_bone_cuirass"]["attributes"]["protection"]
                .as_integer(),
            Some(4)
        );
        assert_eq!(
            document.table()["future_extension"]["value"].as_float(),
            Some(1.0)
        );
        let mut broken = document.clone();
        broken
            .edit(|root| {
                root["items"]["armor"]["test_bone_helm"]["slot"] = string("feet");
                Ok(())
            })
            .unwrap();
        assert!(
            crate::validate_ruleset(broken.table())
                .issues
                .iter()
                .any(|i| i.path == "equipment_families.test_bone_armor")
        );
    }
}

/// Family metadata describes the already materialized definitions; validating
/// it prevents a later inspector edit from silently leaving an incomplete set.
pub(crate) fn validate_families(root: &Table, report: &mut crate::RulesetValidationReport) {
    let Some(value) = root.get("equipment_families") else {
        return;
    };
    let Some(families) = value.as_table() else {
        report.error("equipment_families", "Equipment families must be a table");
        return;
    };
    let catalogue = Catalogue::from_table(root);
    for (id, value) in families {
        let path = format!("equipment_families.{id}");
        let Some(family) = value.as_table() else {
            report.error(&path, "Family must be a table");
            continue;
        };
        let category = family.get("category").and_then(Value::as_str);
        if category.is_none_or(|category| {
            root.get("equipment")
                .and_then(|v| v.get("armor_categories"))
                .and_then(|v| v.get(category))
                .is_none()
        }) {
            report.error(&path, "Family must reference an existing armor category");
        }
        let members = family.get("required_members").and_then(Value::as_array);
        let slots = family.get("required_slots").and_then(Value::as_array);
        let (Some(members), Some(slots)) = (members, slots) else {
            report.error(
                &path,
                "Family needs required_members and required_slots lists",
            );
            continue;
        };
        if members.is_empty() || members.len() != slots.len() {
            report.error(
                &path,
                "Family members and slots must have matching nonzero counts",
            );
        }
        let mut seen_members = std::collections::BTreeSet::new();
        let mut seen_slots = std::collections::BTreeSet::new();
        for (member, slot) in members.iter().zip(slots) {
            let (Some(member), Some(slot)) = (member.as_str(), slot.as_str()) else {
                report.error(&path, "Family members and slots must be IDs");
                continue;
            };
            if !seen_members.insert(member) || !seen_slots.insert(slot) {
                report.error(&path, "Family members and slots must be unique");
            }
            let key = crate::catalogue::DefinitionKey {
                domain: "items".into(),
                id: member.into(),
            };
            let entry = catalogue.resolve(&key).ok().flatten();
            let item = entry.and_then(|e| {
                let mut value = root.get(&e.path[0])?;
                for key in &e.path[1..] {
                    value = value.get(key)?;
                }
                Some(value)
            });
            if entry.is_none_or(|e| e.path.get(1).map(String::as_str) != Some("armor"))
                || item.is_none_or(|i| {
                    i.get("category").and_then(Value::as_str) != category
                        || i.get("slot").and_then(Value::as_str) != Some(slot)
                })
            {
                report.error(
                    &path,
                    format!(
                        "Member '{member}' must be armor of the family category in slot '{slot}'"
                    ),
                );
            }
        }
        if let Some(variants) = family.get("variants") {
            let Some(variants) = variants.as_array() else {
                report.error(&path, "Family variants must be an array of item IDs");
                continue;
            };
            for variant in variants {
                let id = variant.as_str().unwrap_or("");
                let key = crate::catalogue::DefinitionKey {
                    domain: "items".into(),
                    id: id.into(),
                };
                let entry = catalogue.resolve(&key).ok().flatten();
                let item = entry.and_then(|entry| {
                    let mut value = root.get(&entry.path[0])?;
                    for key in &entry.path[1..] {
                        value = value.get(key)?;
                    }
                    Some(value)
                });
                if id.is_empty()
                    || !seen_members.insert(id)
                    || entry
                        .is_none_or(|entry| entry.path.get(1).map(String::as_str) != Some("armor"))
                    || item.is_none_or(|item| {
                        item.get("category").and_then(Value::as_str) != category
                            || item
                                .get("slot")
                                .and_then(Value::as_str)
                                .is_none_or(|slot| !seen_slots.contains(slot))
                    })
                {
                    report.error(&path, format!("Invalid armor family variant '{id}'"));
                }
            }
        }
    }
}
