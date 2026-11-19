//! Additional recipe requirements shared by validation and authoritative crafting.
use serde::Deserialize;
use toml::Table;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRequirement {
    pub capability: String,
    #[serde(default)]
    pub condition_cost: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationRequirement {
    pub capability: String,
    pub range: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repair {
    pub family: String,
    pub amount: u32,
    pub maximum: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enchant {
    pub definition: String,
    /// An empty allowlist uses the enchantment's category compatibility.
    #[serde(default)]
    pub items: Vec<String>,
    /// Optional authored result name; independent of the numeric effect's ID.
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RecipeRequirements {
    pub tool: Option<ToolRequirement>,
    pub station: Option<StationRequirement>,
    pub fixed_quality: Option<i32>,
    pub repair: Option<Repair>,
    pub enchant: Option<Enchant>,
}

pub fn requirements(recipe: &Table) -> Result<RecipeRequirements, String> {
    let tool: Option<ToolRequirement> = recipe
        .get("tool")
        .map(|value| {
            value
                .clone()
                .try_into()
                .map_err(|e| format!("Invalid recipe tool: {e}"))
        })
        .transpose()?;
    if let Some(tool) = &tool {
        if tool.capability.trim().is_empty() || tool.condition_cost > 1_000_000 {
            return Err("Recipe tool needs a capability and bounded condition cost".into());
        }
    }
    // Legacy station strings remain descriptive hints. Enforced requirements
    // are explicit capability/range records, so old projects retain behavior.
    let station: Option<StationRequirement> = match recipe.get("station") {
        Some(value) if value.is_str() => None,
        Some(value) => Some(
            value
                .clone()
                .try_into()
                .map_err(|e| format!("Invalid recipe station: {e}"))?,
        ),
        None => None,
    };
    if let Some(station) = &station {
        if station.capability.trim().is_empty()
            || !station.range.is_finite()
            || station.range <= 0.0
        {
            return Err("Recipe station needs a capability and positive finite range".into());
        }
    }
    let fixed_quality = recipe
        .get("fixed_quality")
        .map(|value| {
            value
                .as_integer()
                .filter(|n| (1..=100).contains(n))
                .map(|n| n as i32)
                .ok_or("Recipe fixed_quality must be an integer from 1 to 100".to_string())
        })
        .transpose()?;
    let repair: Option<Repair> = recipe
        .get("repair")
        .map(|value| {
            value
                .clone()
                .try_into()
                .map_err(|e| format!("Invalid repair policy: {e}"))
        })
        .transpose()?;
    if let Some(repair) = &repair {
        if repair.family.trim().is_empty()
            || repair.amount == 0
            || repair.amount > 100
            || repair.maximum == 0
            || repair.maximum > 100
        {
            return Err("Repair needs a family and condition amounts from 1 to 100".into());
        }
        if fixed_quality.is_some()
            || recipe
                .get("produces")
                .is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty()))
        {
            return Err("Repair preserves quality and cannot produce replacement items".into());
        }
    }
    let enchant: Option<Enchant> = recipe
        .get("enchant")
        .map(|value| {
            value
                .clone()
                .try_into()
                .map_err(|e| format!("Invalid enchanting policy: {e}"))
        })
        .transpose()?;
    if let Some(enchant) = &enchant {
        let mut seen = std::collections::BTreeSet::new();
        if enchant.definition.trim().is_empty()
            || enchant.definition.len() > 128
            || enchant.items.len() > 4096
            || enchant
                .items
                .iter()
                .any(|id| id.is_empty() || id != id.trim() || id.len() > 128 || !seen.insert(id))
            || enchant.name.as_ref().is_some_and(|name| {
                name.trim().is_empty() || name != name.trim() || name.len() > 256
            })
        {
            return Err("Enchanting needs a definition, unique item IDs and an optional bounded result name".into());
        }
        if repair.is_some()
            || fixed_quality.is_some()
            || recipe
                .get("produces")
                .is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty()))
        {
            return Err("Enchanting preserves the target and cannot repair it, reset quality or produce replacement items".into());
        }
    }
    Ok(RecipeRequirements {
        tool,
        station,
        fixed_quality,
        repair,
        enchant,
    })
}

pub(crate) fn validate(root: &Table, report: &mut crate::RulesetValidationReport) {
    let mut targeted_recipes = std::collections::BTreeSet::new();
    if let Some(recipes) = root.get("recipes").and_then(toml::Value::as_table) {
        for (id, value) in recipes {
            let Some(recipe) = value.as_table() else {
                continue;
            };
            match requirements(recipe) {
                Err(error) => report.error(format!("recipes.{id}"), error),
                Ok(requirements) => {
                    if let Some(repair) = requirements.repair {
                        targeted_recipes.insert(id.as_str());
                        if root
                            .get("equipment_families")
                            .and_then(|v| v.get(&repair.family))
                            .is_none()
                        {
                            report.error(
                                format!("recipes.{id}.repair.family"),
                                "Unknown equipment family",
                            );
                        }
                    }
                    if let Some(enchant) = requirements.enchant {
                        targeted_recipes.insert(id.as_str());
                        let definition = root
                            .get("enchantments")
                            .and_then(|v| v.get(&enchant.definition));
                        if definition.is_none() {
                            report.error(
                                format!("recipes.{id}.enchant.definition"),
                                "Unknown enchantment",
                            );
                        }
                        for item_id in &enchant.items {
                            let item = root
                                .get("items")
                                .and_then(toml::Value::as_table)
                                .into_iter()
                                .flatten()
                                .find_map(|(_, group)| group.get(item_id));
                            let compatible = item.is_some_and(|item| {
                                definition.is_none_or(|definition| {
                                    definition
                                        .get("allowed_categories")
                                        .and_then(toml::Value::as_array)
                                        .is_none_or(|categories| {
                                            categories.is_empty()
                                                || categories.iter().any(|category| {
                                                    category.as_str()
                                                        == item
                                                            .get("category")
                                                            .and_then(toml::Value::as_str)
                                                })
                                        })
                                })
                            });
                            if !compatible {
                                report.error(
                                    format!("recipes.{id}.enchant.items"),
                                    "Unknown or incompatible target item",
                                );
                            }
                            if recipe
                                .get("consumes")
                                .and_then(toml::Value::as_array)
                                .is_some_and(|costs| {
                                    costs.iter().any(|cost| {
                                        cost.get("item").and_then(toml::Value::as_str)
                                            == Some(item_id)
                                    })
                                })
                            {
                                report.error(
                                    format!("recipes.{id}.consumes"),
                                    "An enchanting recipe cannot consume its target definition",
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(actions) = root.get("actions").and_then(toml::Value::as_table) {
        for (id, action) in actions {
            if action
                .get("recipe")
                .and_then(toml::Value::as_str)
                .is_some_and(|id| targeted_recipes.contains(id))
                && action
                    .get("target")
                    .and_then(toml::Value::as_str)
                    .map(crate::ResolvedActionTarget::parse)
                    != Some(crate::ResolvedActionTarget::AnyItem)
            {
                report.error(
                    format!("actions.{id}.target"),
                    "Targeted mutation actions must select an exact owned item with target = 'item'",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enchanting_requirements_validate_references_and_exact_target_mutation() {
        assert!(
            requirements(&"enchant={definition='strength'}".parse().unwrap())
                .unwrap()
                .enchant
                .unwrap()
                .items
                .is_empty()
        );
        let root = crate::document::RulesState::default()
            .document()
            .unwrap()
            .table()
            .clone();
        assert!(crate::validate_ruleset(&root).is_ok());
        for source in [
            "enchant={definition='strength',items=['bone_cuirass','bone_cuirass']}",
            "enchant={definition='strength',items=['bone_cuirass'],name=''}",
            "enchant={definition='strength',items=['bone_cuirass'],unknown=true}",
            "enchant={definition='strength',items=['bone_cuirass']}\nfixed_quality=100",
            "enchant={definition='strength',items=['bone_cuirass']}\nproduces=[{item='bone_cuirass',quantity=1}]",
            "enchant={definition='strength',items=['bone_cuirass']}\nrepair={family='bone_armor',amount=20,maximum=100}",
        ] {
            assert!(requirements(&source.parse().unwrap()).is_err(), "{source}");
        }
        for (field, value) in [
            ("definition", toml::Value::String("missing".into())),
            (
                "items",
                toml::Value::Array(vec![toml::Value::String("stone_knife".into())]),
            ),
        ] {
            let mut invalid = root.clone();
            invalid["recipes"]["inscribe_bear_cuirass"]["enchant"][field] = value;
            assert!(crate::validate_ruleset(&invalid).error_count() > 0);
        }
        let mut invalid = root.clone();
        invalid["actions"]["inscribe_bear_cuirass"]["target"] = toml::Value::String("self".into());
        assert!(
            crate::validate_ruleset(&invalid)
                .issues
                .iter()
                .any(|issue| issue.path == "actions.inscribe_bear_cuirass.target")
        );
        let mut invalid = root.clone();
        invalid["recipes"]["inscribe_bear_cuirass"]["enchant"]["items"] =
            toml::Value::Array(vec![toml::Value::String("bone_cuirass".into())]);
        invalid["recipes"]["inscribe_bear_cuirass"]["consumes"] =
            toml::Value::Array(vec![toml::Value::Table(
                "item='bone_cuirass'\nquantity=1".parse().unwrap(),
            )]);
        assert!(
            crate::validate_ruleset(&invalid)
                .issues
                .iter()
                .any(|issue| issue.path == "recipes.inscribe_bear_cuirass.consumes")
        );
        assert_eq!(
            root["items"]["armor"]["strength_bone_cuirass"]["name"].as_str(),
            Some("Bone Cuirass of the Bear")
        );
        assert_eq!(
            root["enchantments"]["strength"]["name"].as_str(),
            Some("Bear's Vigor")
        );
    }
    #[test]
    fn recipe_requirements_validate_and_legacy_station_hints_remain_compatible() {
        for source in [
            "tool={capability=''}",
            "tool={capability='carve',condition_cost=-1}",
            "station={capability='tan',range=0.0}",
            "station={capability='tan',range=2.0,typo=true}",
            "fixed_quality=101",
            "tool='knife'",
            "repair={family='bone_armor',amount=0,maximum=100}",
            "repair={family='bone_armor',amount=20,maximum=101}",
            "repair={family='bone_armor',amount=20,maximum=100}\nfixed_quality=100",
            "repair={family='bone_armor',amount=20,maximum=100}\nproduces=[{item='bone_cuirass',quantity=1}]",
        ] {
            assert!(requirements(&source.parse().unwrap()).is_err(), "{source}");
        }
        assert!(
            requirements(&"station='workbench'".parse().unwrap())
                .unwrap()
                .station
                .is_none()
        );
        let parsed = requirements(&"tool={capability='carve',condition_cost=1}\nstation={capability='bonework',range=2.0}\nfixed_quality=100".parse().unwrap()).unwrap();
        assert_eq!(parsed.tool.unwrap().condition_cost, 1);
        assert_eq!(parsed.station.unwrap().capability, "bonework");
        assert_eq!(parsed.fixed_quality, Some(100));
    }
    #[test]
    fn repair_references_and_targeting_are_validated() {
        let mut root = crate::document::RulesState::default()
            .document()
            .unwrap()
            .table()
            .clone();
        root["recipes"]["repair_bone_armor"]["repair"]["family"] =
            toml::Value::String("missing".into());
        root["actions"]["repair_bone_armor"]["target"] = toml::Value::String("self".into());
        let report = crate::validate_ruleset(&root);
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.path == "recipes.repair_bone_armor.repair.family")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.path == "actions.repair_bone_armor.target")
        );
    }
}
