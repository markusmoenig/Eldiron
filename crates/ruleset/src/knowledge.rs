//! Knowledge owns recipe grants; recipes without a grant remain open.
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use toml::{Table, Value};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Knowledge {
    pub name: String,
    pub description: Option<String>,
    pub recipes: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Teaching {
    pub name: String,
    pub description: Option<String>,
    pub knowledge: BTreeSet<String>,
    pub skill: Option<String>,
    #[serde(default)]
    pub required_skill: u32,
}

#[derive(Clone, Debug, Default)]
pub struct LearningRules {
    pub knowledge: BTreeMap<String, Knowledge>,
    pub teaching: BTreeMap<String, Teaching>,
    pub recipe_grants: BTreeMap<String, BTreeSet<String>>,
}

pub fn ids(value: &Value) -> Result<Vec<String>, String> {
    let values = value
        .as_array()
        .ok_or("Expected an array of definition IDs")?;
    let mut seen = BTreeSet::new();
    if values.len() > 4096 {
        return Err("Too many definition IDs".into());
    }
    values
        .iter()
        .map(|value| {
            let id = value.as_str().ok_or("Definition IDs must be text")?;
            if id.is_empty() || id != id.trim() || id.len() > 128 || !seen.insert(id) {
                return Err("Definition IDs must be nonempty, bounded and unique".into());
            }
            Ok(id.into())
        })
        .collect()
}

fn definitions<'a>(root: &'a Table, key: &str) -> Result<Option<&'a Table>, String> {
    root.get(key)
        .map(|v| v.as_table().ok_or_else(|| format!("{key} must be a table")))
        .transpose()
}

impl LearningRules {
    pub fn resolve(root: &Table) -> Result<Self, String> {
        let mut result = Self::default();
        for (id, value) in definitions(root, "knowledge")?.into_iter().flatten() {
            let definition: Knowledge = value
                .clone()
                .try_into()
                .map_err(|e| format!("knowledge.{id}: {e}"))?;
            if id.is_empty()
                || id.len() > 128
                || definition.name.trim().is_empty()
                || definition.recipes.is_empty()
            {
                return Err(format!(
                    "knowledge.{id} needs an identity, name and recipe grants"
                ));
            }
            ids(value
                .get("recipes")
                .ok_or("Knowledge needs recipe grants")?)?;
            for recipe in &definition.recipes {
                if root
                    .get("recipes")
                    .and_then(|v| v.get(recipe))
                    .and_then(Value::as_table)
                    .is_none()
                {
                    return Err(format!(
                        "knowledge.{id} references unknown recipe '{recipe}'"
                    ));
                }
                result
                    .recipe_grants
                    .entry(recipe.clone())
                    .or_default()
                    .insert(id.clone());
            }
            result.knowledge.insert(id.clone(), definition);
        }
        for (id, value) in definitions(root, "teaching")?.into_iter().flatten() {
            let definition: Teaching = value
                .clone()
                .try_into()
                .map_err(|e| format!("teaching.{id}: {e}"))?;
            if id.is_empty()
                || id.len() > 128
                || definition.name.trim().is_empty()
                || definition.knowledge.is_empty()
                || definition.required_skill > 1_000_000
            {
                return Err(format!(
                    "teaching.{id} needs a name, knowledge and bounded skill requirement"
                ));
            }
            result
                .check_knowledge_ids(value.get("knowledge").ok_or("Teaching needs knowledge")?)?;
            if definition.required_skill > 0 && definition.skill.is_none()
                || definition
                    .skill
                    .as_ref()
                    .is_some_and(|id| root.get("skills").and_then(|v| v.get(id)).is_none())
            {
                return Err(format!(
                    "teaching.{id} references an unknown or missing skill"
                ));
            }
            result.teaching.insert(id.clone(), definition);
        }
        for (id, class) in definitions(root, "classes")?.into_iter().flatten() {
            if let Some(value) = class.get("starting_knowledge") {
                result
                    .check_knowledge_ids(value)
                    .map_err(|e| format!("classes.{id}.starting_knowledge: {e}"))?;
            }
        }
        for (id, profession) in definitions(root, "professions")?.into_iter().flatten() {
            if let Some(value) = profession.get("teaching") {
                result
                    .check_teaching_ids(value)
                    .map_err(|e| format!("professions.{id}.teaching: {e}"))?;
            }
        }
        for (group, items) in definitions(root, "items")?.into_iter().flatten() {
            for (id, item) in items.as_table().into_iter().flatten() {
                if let Some(value) = item.get("teaching") {
                    result
                        .check_teaching_ids(value)
                        .map_err(|e| format!("items.{group}.{id}.teaching: {e}"))?;
                }
            }
        }
        for (id, action) in definitions(root, "actions")?.into_iter().flatten() {
            let lesson = action.get("lesson");
            if action.get("kind").and_then(Value::as_str) == Some("learn") || lesson.is_some() {
                let valid = action.get("kind").and_then(Value::as_str) == Some("learn")
                    && lesson
                        .and_then(Value::as_str)
                        .is_some_and(|id| result.teaching.contains_key(id))
                    && matches!(
                        action.get("target").and_then(Value::as_str),
                        Some(
                            "any_entity"
                                | "friendly_entity"
                                | "inventory_item"
                                | "item"
                                | "any_item"
                        )
                    )
                    && !["recipe", "result", "source"]
                        .iter()
                        .any(|key| action.get(key).is_some());
                if !valid {
                    return Err(format!(
                        "actions.{id}: learning needs a registered lesson and entity/owned-item target, without another effect or source"
                    ));
                }
            }
        }
        Ok(result)
    }

    pub fn check_knowledge_ids(&self, value: &Value) -> Result<Vec<String>, String> {
        let values = ids(value)?;
        if values.iter().any(|id| !self.knowledge.contains_key(id)) {
            return Err("Unknown knowledge definition".into());
        }
        Ok(values)
    }
    pub fn check_teaching_ids(&self, value: &Value) -> Result<Vec<String>, String> {
        let values = ids(value)?;
        if values.iter().any(|id| !self.teaching.contains_key(id)) {
            return Err("Unknown teaching definition".into());
        }
        Ok(values)
    }
}

pub(crate) fn validate(root: &Table, report: &mut crate::RulesetValidationReport) {
    if let Err(error) = LearningRules::resolve(root) {
        report.error("knowledge", error);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn learning_definitions_validate_references_and_leave_ungranted_recipes_open() {
        let source = "[recipes.basic]\n[recipes.advanced]\n[knowledge.bones]\nname='Bones'\nrecipes=['advanced']\n[teaching.lesson]\nname='Lesson'\nknowledge=['bones']\n[actions.learn]\nkind='learn'\nlesson='lesson'\ntarget='any_entity'";
        let root: toml::Table = source.parse().unwrap();
        let rules = super::LearningRules::resolve(&root).unwrap();
        assert!(!rules.recipe_grants.contains_key("basic"));
        assert!(rules.recipe_grants["advanced"].contains("bones"));
        for source in [
            source.replace("recipes=['advanced']", "recipes=['missing']"),
            source.replace("knowledge=['bones']", "knowledge=['missing']"),
            source.replace("target='any_entity'", "target='self'"),
            format!("{source}\nresult={{script='bypass'}}"),
        ] {
            assert!(super::LearningRules::resolve(&source.parse().unwrap()).is_err());
        }
    }
}
