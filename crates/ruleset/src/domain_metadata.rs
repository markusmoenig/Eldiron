//! Shared RPG authoring hints. Unknown fields and values remain editable.
use toml::{Table, Value as TomlValue};

pub fn field_order(domain: &str, key: &str) -> usize {
    let leading = match domain {
        "actions" => &[
            "name",
            "description",
            "kind",
            "target",
            "range",
            "cooldown",
            "requires",
            "cost",
            "consumes",
            "result",
            "fx",
            "ui",
        ][..],
        "items" => &[
            "name",
            "description",
            "category",
            "slot",
            "rarity",
            "enchantments",
            "damage",
            "attributes",
            "icon",
            "visual_template",
        ][..],
        "classes" => &[
            "name",
            "description",
            "role",
            "primary_attributes",
            "attributes",
            "progression",
            "unlocks",
            "starting_loadout",
            "allowed_weapons",
            "allowed_armor",
        ][..],
        "races" => &[
            "name",
            "description",
            "default_avatar",
            "traits",
            "languages",
            "attributes",
        ][..],
        "resource_profiles" => &[
            "name",
            "initial_phase",
            "default_interaction",
            "expires_after_ms",
            "spent_when_empty",
            "allow_overrides",
            "supplies",
            "interactions",
            "exploitation",
        ][..],
        "resource_pools" => &["profile", "supplies"][..],
        "recipes" => &[
            "name",
            "description",
            "skill",
            "required_skill",
            "recommended_skill",
            "difficulty",
            "attribute",
            "station",
            "time",
            "requires",
            "consumes",
            "produces",
        ][..],
        "conditions" => &[
            "name",
            "description",
            "duration",
            "stacking",
            "max_stacks",
            "tags",
            "immune_traits",
            "modifiers",
            "periodic",
            "fx",
        ][..],
        "enchantments" => &["name", "requires_positive_condition", "allowed_categories", "modifiers"][..],
        "spells" | "abilities" => {
            &["name", "description", "kind", "school", "damage", "healing"][..]
        }
        _ => &["name", "description", "kind"][..],
    };
    leading
        .iter()
        .position(|field| *field == key)
        .unwrap_or(leading.len())
}

pub fn field_help(domain: &str, key: &str) -> &'static str {
    match (domain, key) {
        ("actions", "cooldown") => "Time in seconds before this action can be used again.",
        ("actions", "target") => "Which target the action can affect.",
        ("recipes", "time") => "Crafting time in seconds.",
        ("recipes", "consumes") => "Ingredients consumed by the recipe.",
        ("recipes", "produces") => "Items created by the recipe.",
        ("recipes", "tool") => "Owned tool capability and condition cost, paid only on successful completion.",
        ("recipes", "station") => "A capability/range record requires a nearby deployed world station. Legacy text values are descriptive hints.",
        ("recipes", "fixed_quality") => "Optional fixed output quality from 1 to 100; otherwise the existing skill calculation applies.",
        ("recipes", "repair") => "Mutates an exact owned family member, restoring condition while preserving its identity and other properties. No replacement output.",
        ("equipment", "requires_positive_condition") => "Opt-in equipment contributions require finite positive condition. Missing condition means full condition for legacy instances.",
        ("equipment", "wear") => "Charges the declared equipped slot after matching damage actually reduces health; no costs on absorbed or rejected damage.",
        ("equipment", "contributions") => "Explicit item-rating projections into actor attributes. Shared values count each equipped source once.",
        ("enchantments", "modifiers") => "Passive wearer additions, multipliers and bounds over declared numeric attributes.",
        ("enchantments", "requires_positive_condition") => "This enchantment is active only while equipped and the item's condition is positive.",
        ("items", "enchantments") => "Initial passive enchantment definition IDs. Attachment identity follows the actual item instance.",
        ("conditions", "duration") => "Duration in seconds.",
        ("conditions", "stacking") => "How repeated applications combine.",
        ("classes", "unlocks") => "Abilities and spells learned at each level.",
        ("classes", "starting_loadout") => "Equipment and inventory provided to new characters.",
        ("races", "default_avatar") => {
            "Avatar used when the character has no explicit visual override."
        }
        ("resource_profiles", "capacity") => {
            "Finite stock capacity. Omit capacity for unlimited supply; initial and recovery must then be absent."
        }
        ("resource_profiles", "initial") | ("resource_pools", "initial") => {
            "Stock for new world instances only. Editing this never refills an existing source."
        }
        ("resource_profiles", "interval_ms") | ("resource_profiles", "expires_after_ms") => {
            "Milliseconds on the host's monotonic simulation clock. Pausing or offline time adds no recovery."
        }
        ("resource_profiles", "outputs") => {
            "Registered item outputs per stock unit. Map bindings cannot replace these outputs."
        }
        ("resource_profiles", "tool_wear") => {
            "Condition charged to the exact selected tool only when the complete operation commits."
        }
        ("resource_pools", "profile") => {
            "One world-wide stock owner shared by access points, including access in different regions."
        }
        ("items", "slot") => "Equipment slot used by this item.",
        _ => "Edit this value and press Return.",
    }
}

/// Choices are derived from the active document, and the current value is
/// always retained so custom rules never disappear from the editor.
pub fn choices(rules: &Table, path: &[String], key: &str, current: &str) -> Vec<String> {
    choices_with_catalogue(
        rules,
        &crate::catalogue::Catalogue::from_table(rules),
        path,
        key,
        current,
    )
}

pub fn choices_with_catalogue(
    rules: &Table,
    catalogue: &crate::catalogue::Catalogue,
    path: &[String],
    key: &str,
    current: &str,
) -> Vec<String> {
    let domain = path.first().map(String::as_str).unwrap_or("");
    let section = reference_domain(key);
    let mut options: Vec<String> = if domain == "resource_profiles" && key == "supply" {
        path.get(1)
            .and_then(|id| rules.get("resource_profiles").and_then(|v| v.get(id)))
            .and_then(|v| v.get("supplies"))
            .and_then(TomlValue::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    } else if domain == "resource_profiles" && key == "unit" {
        vec!["count".into(), "volume".into(), "opportunity".into()]
    } else if domain == "resource_profiles" && key == "policy" {
        vec!["whole".into(), "clamped".into()]
    } else if let Some(section) = section {
        catalogue.ids(section)
    } else if matches!(
        key,
        "attribute" | "bonus_attribute" | "maximum_attribute" | "primary_attributes"
    ) {
        rules
            .get("attributes")
            .and_then(|v| v.get("defaults"))
            .and_then(TomlValue::as_table)
            .map(|t| {
                t.iter()
                    .filter(|(_, v)| v.is_integer() || v.is_float())
                    .map(|(key, _)| key.clone())
                    .collect()
            })
            .unwrap_or_default()
    } else if key == "item"
        || (domain == "classes" && path.iter().any(|part| part == "starting_loadout"))
    {
        catalogue.ids("items")
    } else if key == "damage_kind" {
        rules
            .get("combat")
            .and_then(|v| v.get("kinds"))
            .and_then(TomlValue::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    } else if key == "default_avatar" {
        rules
            .get("assets")
            .and_then(|v| v.get("avatars"))
            .and_then(TomlValue::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    } else if domain == "items"
        && key == "slot"
        && path
            .get(1)
            .is_some_and(|g| matches!(g.as_str(), "weapons" | "armor" | "clothing"))
    {
        rules
            .get("equipment")
            .and_then(TomlValue::as_table)
            .map(|equipment| {
                ["weapon_slots", "armor_slots"]
                    .into_iter()
                    .filter_map(|k| equipment.get(k).and_then(TomlValue::as_array))
                    .flatten()
                    .filter_map(TomlValue::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    } else if (domain == "items"
        && key == "category"
        && path
            .get(1)
            .is_some_and(|g| matches!(g.as_str(), "weapons" | "armor" | "clothing")))
        || (domain == "classes" && matches!(key, "allowed_weapons" | "allowed_armor"))
    {
        let group = if key == "allowed_armor"
            || path
                .get(1)
                .is_some_and(|part| matches!(part.as_str(), "armor" | "clothing"))
        {
            "armor_categories"
        } else {
            "weapon_categories"
        };
        rules
            .get("equipment")
            .and_then(|v| v.get(group))
            .and_then(TomlValue::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    } else if domain == "conditions" && key == "stacking" {
        ["replace", "refresh", "stack", "ignore"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else if domain == "items" && key == "rig_layer" {
        ["front", "back"].iter().map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    };
    options.sort();
    options.dedup();
    if !options.is_empty() && !options.iter().any(|option| option == current) {
        options.push(current.into());
    }
    options
}

/// Reference target is metadata, independent of the editor widget or engine.
pub fn reference_domain(key: &str) -> Option<&'static str> {
    match key {
        "ability" | "abilities" => Some("abilities"),
        "spell" | "spells" => Some("spells"),
        "recipe" | "recipes" => Some("recipes"),
        "apply_condition" | "condition" => Some("conditions"),
        "class_hint" => Some("classes"),
        "profession_hint" => Some("professions"),
        "trait" | "traits" | "immune_traits" => Some("traits"),
        "skill" => Some("skills"),
        "race" => Some("races"),
        "class" => Some("classes"),
        "visual_template" => Some("visual_templates"),
        "item" | "required_members" => Some("items"),
        "profile" | "remains_profile" => Some("resource_profiles"),
        "pool" => Some("resource_pools"),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldMetadata {
    pub order: usize,
    pub help: &'static str,
    pub reference: Option<&'static str>,
    pub unit: Option<&'static str>,
}

pub fn field(domain: &str, key: &str) -> FieldMetadata {
    FieldMetadata {
        order: field_order(domain, key),
        help: field_help(domain, key),
        reference: if key == "item" {
            Some("items")
        } else {
            reference_domain(key)
        },
        unit: match (domain, key) {
            (
                "resource_profiles" | "resource_pools",
                "interval_ms"
                | "expires_after_ms"
                | "window_ms"
                | "suspension_ms"
                | "recovery_interval_ms",
            ) => Some("simulation milliseconds"),
            ("actions", "cooldown") | ("recipes", "time") | ("conditions", "duration") => {
                Some("seconds")
            }
            ("actions", "range") => Some("tiles or weapon"),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_attributes_and_grouped_items_are_references_without_losing_unknown_values() {
        let root: Table =
            "[attributes.defaults]\nstrength=10\n[items.custom.bone_plate]\nname='Plate'\n"
                .parse()
                .unwrap();
        assert_eq!(
            choices(&root, &["recipes".into(), "make".into()], "item", "missing"),
            vec!["bone_plate", "missing"]
        );
        assert_eq!(
            choices(
                &root,
                &["recipes".into(), "make".into()],
                "attribute",
                "strength"
            ),
            vec!["strength"]
        );
        assert_eq!(field("recipes", "skill").reference, Some("skills"));
        assert_eq!(field("actions", "cooldown").unit, Some("seconds"));
        assert!(field("custom", "future").reference.is_none());
    }
}
