//! Permanent acquisitions are saved facts; equipment grants are read-only views.
use crate::{
    Entity, Value,
    effective_values::{ValueRules, enchantments},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use theframework_core::prelude::Uuid;

pub const JOURNAL: &str = "knowledge_journal";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Origin {
    Class {
        id: String,
    },
    Character,
    Teacher {
        lesson: String,
        entity: u32,
        creator: Uuid,
    },
    Book {
        lesson: String,
        item: u32,
        creator: Uuid,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acquisition {
    pub id: Uuid,
    pub knowledge: String,
    pub origin: Origin,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub version: u32,
    pub acquisitions: Vec<Acquisition>,
}
impl Default for Journal {
    fn default() -> Self {
        Self {
            version: 1,
            acquisitions: Vec::new(),
        }
    }
}
pub fn journal(entity: &Entity) -> Result<Journal, String> {
    let value = match entity.attributes.get(JOURNAL) {
        None => return Ok(Journal::default()),
        Some(Value::Str(value)) if value.len() <= 1_048_576 => value,
        _ => return Err("Invalid knowledge journal".into()),
    };
    let journal: Journal =
        serde_json::from_str(value).map_err(|e| format!("Invalid knowledge journal: {e}"))?;
    let mut definitions = BTreeSet::new();
    let mut instances = BTreeSet::new();
    if journal.version != 1
        || journal.acquisitions.len() > 4096
        || journal.acquisitions.iter().any(|entry| {
            entry.id.is_nil()
                || entry.knowledge.is_empty()
                || entry.knowledge.len() > 128
                || !definitions.insert(&entry.knowledge)
                || !instances.insert(entry.id)
        })
    {
        return Err("Unsupported or invalid knowledge journal".into());
    }
    Ok(journal)
}

/// Adds permanent records once. The caller owns costs, authorization and commit.
pub fn acquire(
    entity: &mut Entity,
    rules: &ValueRules,
    knowledge: &[String],
    origin: Origin,
) -> Result<usize, String> {
    if knowledge
        .iter()
        .any(|id| !rules.learning.knowledge.contains_key(id))
    {
        return Err("Unknown knowledge definition".into());
    }
    acquire_validated(entity, knowledge, origin)
}

fn acquire_validated(
    entity: &mut Entity,
    knowledge: &[String],
    origin: Origin,
) -> Result<usize, String> {
    let mut journal = journal(entity)?;
    let mut added = 0;
    for id in knowledge {
        if journal
            .acquisitions
            .iter()
            .any(|entry| entry.knowledge == *id)
        {
            continue;
        }
        if journal.acquisitions.len() >= 4096 {
            return Err("Knowledge journal is full".into());
        }
        journal.acquisitions.push(Acquisition {
            id: Uuid::new_v4(),
            knowledge: id.clone(),
            origin: origin.clone(),
        });
        added += 1;
    }
    if added > 0 {
        journal
            .acquisitions
            .sort_by(|a, b| a.knowledge.cmp(&b.knowledge));
        let serialized = serde_json::to_string(&journal).map_err(|e| e.to_string())?;
        if serialized.len() > 1_048_576 {
            return Err("Knowledge journal exceeds bounds".into());
        }
        entity.set_attribute(JOURNAL, Value::Str(serialized));
    }
    Ok(added)
}

pub fn initialize_starting(root: &toml::Table, entity: &mut Entity) -> Result<(), String> {
    if entity.attributes.get(JOURNAL).is_some() {
        journal(entity)?;
        return Ok(());
    }
    let Some(definitions) = root.get("knowledge").and_then(toml::Value::as_table) else {
        return Ok(());
    };
    if definitions.is_empty() {
        return Ok(());
    }
    let (knowledge, origin) = if let Some(value) = entity.attributes.get("starting_knowledge") {
        let Value::StrArray(values) = value else {
            return Err("starting_knowledge must be an array of IDs".into());
        };
        (values.clone(), Origin::Character)
    } else if let Some(class) = entity.attributes.get_str("class") {
        let values = root
            .get("classes")
            .and_then(|v| v.get(class))
            .and_then(|v| v.get("starting_knowledge"))
            .map(eldiron_ruleset::knowledge::ids)
            .transpose()?
            .unwrap_or_default();
        (values, Origin::Class { id: class.into() })
    } else {
        (Vec::new(), Origin::Character)
    };
    if knowledge
        .iter()
        .any(|id| !definitions.get(id).is_some_and(toml::Value::is_table))
    {
        return Err("Unknown starting knowledge definition".into());
    }
    let mut staged = entity.clone();
    acquire_validated(&mut staged, &knowledge, origin)?;
    if staged.attributes.get(JOURNAL).is_none() {
        staged.set_attribute(
            JOURNAL,
            Value::Str(serde_json::to_string(&Journal::default()).map_err(|e| e.to_string())?),
        );
    }
    *entity = staged;
    Ok(())
}

pub fn has_permanent(entity: &Entity, knowledge: &str) -> Result<bool, String> {
    Ok(journal(entity)?
        .acquisitions
        .iter()
        .any(|entry| entry.knowledge == knowledge))
}

pub fn has_knowledge(rules: &ValueRules, entity: &Entity, knowledge: &str) -> Result<bool, String> {
    if !rules.learning.knowledge.contains_key(knowledge) {
        return Err("Unknown knowledge definition".into());
    }
    if has_permanent(entity, knowledge)? {
        return Ok(true);
    }
    for item in entity.equipped.values() {
        for instance in enchantments(item)? {
            let definition = rules
                .enchantments
                .get(&instance.definition)
                .ok_or("Unknown enchantment")?;
            if definition.knowledge_grants.contains(knowledge)
                && (!definition.requires_positive_condition
                    || crate::equipment_lifecycle::condition(item).is_some_and(|value| value > 0.))
            {
                rules.validate_item(item)?;
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub fn recipe_access(rules: &ValueRules, entity: &Entity, recipe: &str) -> Result<(), String> {
    let Some(grants) = rules.learning.recipe_grants.get(recipe) else {
        return Ok(());
    };
    for knowledge in grants {
        if has_knowledge(rules, entity, knowledge)? {
            return Ok(());
        }
    }
    Err(format!(
        "Learn {}",
        grants
            .iter()
            .map(|id| rules.learning.knowledge[id].name.as_str())
            .collect::<Vec<_>>()
            .join(" or ")
    ))
}

pub fn summary(rules: &ValueRules, entity: &Entity) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut permanent = BTreeSet::new();
    for acquisition in journal(entity)?.acquisitions {
        let name = rules
            .learning
            .knowledge
            .get(&acquisition.knowledge)
            .map(|v| v.name.as_str())
            .unwrap_or(&acquisition.knowledge);
        let origin = match acquisition.origin {
            Origin::Character => "starting choice".into(),
            Origin::Class { id } => format!("starting class {id}"),
            Origin::Teacher { entity, .. } => format!("teacher #{entity}"),
            Origin::Book { item, .. } => format!("book #{item}"),
        };
        result.push(format!("{name} (permanent; {origin})"));
        permanent.insert(acquisition.knowledge);
    }
    for (id, definition) in &rules.learning.knowledge {
        if !permanent.contains(id) && has_knowledge(rules, entity, id)? {
            result.push(format!("{} (temporary equipment grant)", definition.name));
        }
    }
    Ok(result)
}

pub fn recipe_skill_access(entity: &Entity, recipe: &toml::Table) -> Result<(), String> {
    let required = recipe
        .get("required_skill")
        .and_then(|value| {
            value
                .as_integer()
                .map(|v| v as f64)
                .or_else(|| value.as_float())
        })
        .unwrap_or(0.)
        .round()
        .max(0.) as i32;
    if required <= 0 {
        return Ok(());
    }
    let skill = recipe
        .get("skill")
        .and_then(toml::Value::as_str)
        .ok_or("Recipe has no required skill")?;
    if skill_points(entity, skill) >= required {
        Ok(())
    } else {
        Err(format!("Need {} {required}", skill))
    }
}

pub fn skill_points(entity: &Entity, skill_id: &str) -> i32 {
    let normalized = skill_id.trim();
    if normalized.is_empty() {
        return 0;
    }
    let lower = normalized.to_ascii_lowercase();
    let upper = normalized.to_ascii_uppercase();
    let skill_key = format!("skill_{}", lower);
    let skill_upper_key = format!("SKILL_{}", upper);
    entity
        .attributes
        .get_int(&skill_key)
        .or_else(|| entity.attributes.get_int(&skill_upper_key))
        .or_else(|| entity.attributes.get_int(&upper))
        .or_else(|| entity.attributes.get_int(normalized))
        .unwrap_or(0)
        .max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> toml::Table {
        eldiron_ruleset::document::RulesState::default()
            .document()
            .unwrap()
            .table()
            .clone()
    }
    #[test]
    fn learning_journal_starting_grants_are_once_only_and_character_choices_override_class() {
        let root = root();
        let rules = ValueRules::resolve(&root).unwrap();
        let mut ranger = Entity::new();
        ranger.set_attribute("class", Value::Str("Ranger".into()));
        crate::server::region::apply_ruleset_character_defaults(&root, &mut ranger);
        let first = journal(&ranger).unwrap();
        assert_eq!(first.acquisitions.len(), 1);
        assert_eq!(
            first.acquisitions[0].origin,
            Origin::Class {
                id: "Ranger".into()
            }
        );
        assert_eq!(skill_points(&ranger, "boneworking"), 10);
        crate::server::region::apply_ruleset_character_defaults(&root, &mut ranger);
        assert_eq!(journal(&ranger).unwrap(), first);
        ranger.set_attribute("class", Value::Str("Warrior".into()));
        initialize_starting(&root, &mut ranger).unwrap();
        assert!(has_knowledge(&rules, &ranger, "boneworking_patterns").unwrap());
        assert_eq!(journal(&ranger).unwrap(), first);
        let mut explicit = Entity::new();
        explicit.set_attribute("class", Value::Str("Ranger".into()));
        explicit.set_attribute("starting_knowledge", Value::StrArray(Vec::new()));
        initialize_starting(&root, &mut explicit).unwrap();
        assert!(journal(&explicit).unwrap().acquisitions.is_empty());
        explicit.attributes.remove("starting_knowledge");
        initialize_starting(&root, &mut explicit).unwrap();
        assert!(
            journal(&explicit).unwrap().acquisitions.is_empty(),
            "a reload or class change cannot re-seed starting grants"
        );
    }
    #[test]
    fn learning_journal_permanent_acquisition_outlives_temporary_equipment_without_read_side_effects()
     {
        let rules = ValueRules::resolve(&root()).unwrap();
        let mut actor = Entity::new();
        let mut helm = crate::Item::new();
        helm.id = 41;
        helm.set_attribute("category", Value::Str("bone_armor".into()));
        crate::effective_values::attach(&mut helm, &rules, "boneworking_guidance").unwrap();
        actor.equipped.insert("head".into(), helm);
        let before = serde_json::to_value(&actor).unwrap();
        assert!(recipe_access(&rules, &actor, "shape_bone_batch").is_ok());
        assert!(journal(&actor).unwrap().acquisitions.is_empty());
        assert_eq!(serde_json::to_value(&actor).unwrap(), before);
        actor
            .equipped
            .get_mut("head")
            .unwrap()
            .set_attribute("condition", Value::Int(0));
        assert!(recipe_access(&rules, &actor, "shape_bone_batch").is_err());
        actor
            .equipped
            .get_mut("head")
            .unwrap()
            .set_attribute("condition", Value::Int(100));
        acquire(
            &mut actor,
            &rules,
            &["boneworking_patterns".into()],
            Origin::Character,
        )
        .unwrap();
        let permanent = journal(&actor).unwrap();
        actor.equipped.clear();
        assert!(recipe_access(&rules, &actor, "shape_bone_batch").is_ok());
        assert_eq!(
            acquire(
                &mut actor,
                &rules,
                &["boneworking_patterns".into()],
                Origin::Class {
                    id: "Ranger".into()
                }
            )
            .unwrap(),
            0
        );
        assert_eq!(journal(&actor).unwrap(), permanent);
        let restored: Entity =
            serde_json::from_value(serde_json::to_value(&actor).unwrap()).unwrap();
        assert_eq!(journal(&restored).unwrap(), permanent);
        assert!(recipe_access(&rules, &Entity::new(), "shape_bone").is_ok());
    }
    #[test]
    fn learning_journal_invalid_records_are_preserved_and_rejected_atomically() {
        let rules = ValueRules::resolve(&root()).unwrap();
        let mut actor = Entity::new();
        actor.set_attribute(
            JOURNAL,
            Value::Str("{\"version\":99,\"acquisitions\":[]}".into()),
        );
        let before = serde_json::to_value(&actor).unwrap();
        assert!(
            acquire(
                &mut actor,
                &rules,
                &["boneworking_patterns".into()],
                Origin::Character
            )
            .is_err()
        );
        assert_eq!(serde_json::to_value(&actor).unwrap(), before);
        assert!(has_knowledge(&rules, &actor, "boneworking_patterns").is_err());
    }
}
