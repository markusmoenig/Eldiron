//! Side-effect-free actor values shared by authority, action UI and text/HUD.
use crate::{Entity, Item, Value};
use eldiron_ruleset::contributions::ResolvedEnchantment;
use eldiron_ruleset::{ResolvedCondition, ResolvedDerivedStat, ResolvedEquipmentPolicy};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use theframework_core::prelude::Uuid;

const ATTACHMENTS: &str = "item_enchantment_instances";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnchantmentInstance {
    pub id: String,
    pub definition: String,
    /// Saved display fragment lets removal restore a name without a rules lookup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epithet: Option<String>,
}

/// Presets have a stable compound instance identity. Explicit attachments are
/// saved in an ordinary item attribute, so existing inventory deltas, transfers
/// and world saves preserve them without a parallel item/network identity.
pub fn enchantments(item: &Item) -> Result<Vec<EnchantmentInstance>, String> {
    let values: Vec<EnchantmentInstance> = match item.attributes.get(ATTACHMENTS) {
        Some(Value::Str(value)) => {
            if value.len() > 65_536 {
                return Err("Item enchantment state exceeds bounds".into());
            }
            serde_json::from_str(value).map_err(|e| format!("Invalid item enchantments: {e}"))?
        }
        Some(_) => return Err("Invalid item enchantment state".into()),
        None => match item.attributes.get("enchantments") {
            Some(Value::StrArray(ids)) => ids
                .iter()
                .enumerate()
                .map(|(index, definition)| EnchantmentInstance {
                    id: format!(
                        "preset/{}/{}/{index}/{definition}",
                        item.creator_id, item.id
                    ),
                    definition: definition.clone(),
                    epithet: None,
                })
                .collect(),
            None => Vec::new(),
            _ => return Err("Invalid item enchantment presets".into()),
        },
    };
    let mut ids = BTreeSet::new();
    let mut definitions = BTreeSet::new();
    if values.len() > 64
        || values.iter().any(|v| {
            v.id.trim().is_empty()
                || v.id.len() > 256
                || v.definition.len() > 128
                || v.definition.trim().is_empty()
                || v.epithet
                    .as_ref()
                    .is_some_and(|s| s.trim().is_empty() || s != s.trim() || s.len() > 128)
                || !ids.insert(&v.id)
                || !definitions.insert(&v.definition)
        })
    {
        return Err("Item enchantment identities must be unique and bounded".into());
    }
    Ok(values)
}

/// Materialize presets only at item construction/data mutation boundaries.
/// Reads never generate IDs. Existing state (including []) takes precedence.
pub(crate) fn initialize_presets(item: &mut Item) -> Result<(), String> {
    if item.attributes.get(ATTACHMENTS).is_some() || item.attributes.get("enchantments").is_none() {
        return Ok(());
    }
    let mut instances = enchantments(item)?;
    if instances.is_empty() {
        return Ok(());
    }
    for instance in &mut instances {
        instance.id = Uuid::new_v4().to_string();
    }
    item.set_attribute(
        ATTACHMENTS,
        Value::Str(serde_json::to_string(&instances).map_err(|e| e.to_string())?),
    );
    Ok(())
}

/// Trusted mutation boundary; callers still own material costs and authority.
/// Inspection never calls this or invents a persisted attachment.
pub fn attach(item: &mut Item, rules: &ValueRules, definition: &str) -> Result<String, String> {
    let enchantment = rules
        .enchantments
        .get(definition)
        .ok_or("Unknown enchantment")?;
    if !compatible(enchantment, item) {
        return Err("Enchantment does not apply to this item".into());
    }
    rules.validate_item(item)?;
    let mut instances = enchantments(item)?;
    if instances.len() >= 64 || instances.iter().any(|e| e.definition == definition) {
        return Err("This enchantment is already attached or the item is full".into());
    }
    hydrate_epithets(&mut instances, rules);
    let base = base_name(item, &instances);
    let id = Uuid::new_v4().to_string();
    instances.push(EnchantmentInstance {
        id: id.clone(),
        definition: definition.into(),
        epithet: enchantment.epithet.clone(),
    });
    item.set_attribute(
        ATTACHMENTS,
        Value::Str(serde_json::to_string(&instances).map_err(|e| e.to_string())?),
    );
    if instances.iter().any(|i| i.epithet.is_some()) {
        item.set_attribute("enchantment_base_name", Value::Str(base.clone()));
        item.set_attribute("name", Value::Str(render_name(&base, &instances)));
    }
    Ok(id)
}

pub fn detach(item: &mut Item, instance: &str) -> Result<String, String> {
    let mut instances = enchantments(item)?;
    let base = base_name(item, &instances);
    let index = instances
        .iter()
        .position(|value| value.id == instance)
        .ok_or("Unknown enchantment instance")?;
    let removed = instances.remove(index);
    item.set_attribute(
        ATTACHMENTS,
        Value::Str(serde_json::to_string(&instances).map_err(|e| e.to_string())?),
    );
    if removed.epithet.is_some() || item.attributes.get_str("enchantment_base_name").is_some() {
        item.set_attribute("enchantment_base_name", Value::Str(base.clone()));
        item.set_attribute("name", Value::Str(render_name(&base, &instances)));
    }
    Ok(removed.definition)
}

fn hydrate_epithets(instances: &mut [EnchantmentInstance], rules: &ValueRules) {
    for instance in instances {
        if instance.epithet.is_none() {
            instance.epithet = rules
                .enchantments
                .get(&instance.definition)
                .and_then(|d| d.epithet.clone());
        }
    }
}
fn suffix(instances: &[EnchantmentInstance]) -> Option<String> {
    let parts = instances
        .iter()
        .filter_map(|i| i.epithet.as_deref())
        .collect::<BTreeSet<_>>();
    (!parts.is_empty()).then(|| {
        format!(
            " of {}",
            parts.into_iter().collect::<Vec<_>>().join(" and ")
        )
    })
}
fn render_name(base: &str, instances: &[EnchantmentInstance]) -> String {
    format!("{base}{}", suffix(instances).unwrap_or_default())
}
fn base_name(item: &Item, instances: &[EnchantmentInstance]) -> String {
    let name = item.attributes.get_str("name").unwrap_or(&item.item_type);
    if let Some(base) = item.attributes.get_str("enchantment_base_name") {
        if name == render_name(base, instances) || name == base {
            return base.into();
        }
    }
    // Old named presets have no base-name attribute. Preserve custom names too.
    suffix(instances)
        .and_then(|s| name.strip_suffix(&s))
        .unwrap_or(name)
        .to_owned()
}
/// Pure preview: no attachment IDs are generated and no item data is changed.
pub fn enchanted_name(
    item: &Item,
    rules: &ValueRules,
    definition: &str,
) -> Result<Option<String>, String> {
    let effect = rules
        .enchantments
        .get(definition)
        .ok_or("Unknown enchantment")?;
    let Some(epithet) = &effect.epithet else {
        return Ok(None);
    };
    let mut instances = enchantments(item)?;
    hydrate_epithets(&mut instances, rules);
    let base = base_name(item, &instances);
    instances.push(EnchantmentInstance {
        id: String::new(),
        definition: definition.into(),
        epithet: Some(epithet.clone()),
    });
    Ok(Some(render_name(&base, &instances)))
}

pub fn compatible(definition: &ResolvedEnchantment, item: &Item) -> bool {
    definition.allowed_categories.is_empty()
        || item
            .attributes
            .get_str("category")
            .is_some_and(|c| definition.allowed_categories.contains(c))
}

#[derive(Clone, Debug, Default)]
pub struct ValueRules {
    pub conditions: BTreeMap<String, ResolvedCondition>,
    pub derived: BTreeMap<String, ResolvedDerivedStat>,
    pub equipment: ResolvedEquipmentPolicy,
    pub enchantments: BTreeMap<String, ResolvedEnchantment>,
    pub learning: eldiron_ruleset::knowledge::LearningRules,
    pub level_attribute: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contribution {
    pub identity: String,
    pub definition: String,
    pub label: String,
    pub item: Option<u32>,
    pub active: bool,
    pub reason: Option<String>,
    pub add: f64,
    pub multiply: f64,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValueExplanation {
    pub attribute: String,
    pub formula: Option<String>,
    pub saved: f32,
    pub baseline: f32,
    pub dependencies: BTreeMap<String, ValueExplanation>,
    pub contributions: Vec<Contribution>,
    pub value: f32,
}

impl ValueExplanation {
    pub fn summary(&self) -> String {
        let mut text = format!("{}: {} (base {}", self.attribute, self.value, self.saved);
        if self.baseline != self.saved {
            text.push_str(&format!(", derived {}", self.baseline));
        }
        for source in &self.contributions {
            if source.active {
                if source.add != 0.0 {
                    text.push_str(&format!(", {} {:+}", source.label, source.add));
                }
                if source.multiply != 1.0 {
                    text.push_str(&format!(", {} ×{}", source.label, source.multiply));
                }
                if let Some(value) = source.minimum {
                    text.push_str(&format!(", {} min {value}", source.label));
                }
                if let Some(value) = source.maximum {
                    text.push_str(&format!(", {} max {value}", source.label));
                }
            } else {
                text.push_str(&format!(
                    ", {} inactive: {}",
                    source.label,
                    source.reason.as_deref().unwrap_or("inactive")
                ));
            }
        }
        text.push(')');
        text
    }
}

impl ValueRules {
    pub fn validate_item(&self, item: &Item) -> Result<(), String> {
        for attachment in enchantments(item)? {
            let definition = self
                .enchantments
                .get(&attachment.definition)
                .ok_or("Unknown saved enchantment")?;
            if !compatible(definition, item) {
                return Err("Incompatible saved enchantment".into());
            }
        }
        if let Some(category) = crate::equipment_lifecycle::category(&self.equipment, item) {
            for projection in &category.contributions {
                if let Some(value) = item.attributes.get(&projection.item_attribute) {
                    if !matches!(
                        value,
                        Value::Int(_) | Value::UInt(_) | Value::Int64(_) | Value::Float(_)
                    ) || !item
                        .attributes
                        .get_float(&projection.item_attribute)
                        .is_some_and(f32::is_finite)
                    {
                        return Err("Equipment rating must be a finite number".into());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn affected_attributes(&self, entity: &Entity) -> Result<BTreeSet<String>, String> {
        let mut result: BTreeSet<_> = self.derived.keys().cloned().collect();
        for item in entity.equipped.values() {
            self.validate_item(item)?;
            if let Some(category) = crate::equipment_lifecycle::category(&self.equipment, item) {
                result.extend(category.contributions.iter().map(|v| v.attribute.clone()));
            }
            for attachment in enchantments(item)? {
                result.extend(
                    self.enchantments[&attachment.definition]
                        .modifiers
                        .iter()
                        .map(|v| v.attribute.clone()),
                );
            }
        }
        Ok(result)
    }

    pub fn resolve(root: &toml::Table) -> Result<Self, String> {
        Ok(Self {
            conditions: eldiron_ruleset::resolve_conditions(root)?,
            derived: eldiron_ruleset::resolve_derived_stats(root)?,
            equipment: eldiron_ruleset::resolve_equipment_policy(root)?,
            enchantments: eldiron_ruleset::contributions::resolve_enchantments(root)?,
            learning: eldiron_ruleset::knowledge::LearningRules::resolve(root)?,
            level_attribute: eldiron_ruleset::resolve_attribute_roles(root)?
                .get("level")
                .map(str::to_string),
        })
    }

    pub fn explain(
        &self,
        entity: &Entity,
        attribute: &str,
        default: f32,
    ) -> Result<ValueExplanation, String> {
        self.explain_with_conditions(entity, attribute, default, &|id| {
            let active = match entity.attributes.get("conditions") {
                Some(Value::StrArray(ids)) => ids.iter().any(|v| v == id),
                Some(Value::Str(ids)) => ids.split(',').any(|v| v.trim() == id),
                _ => false,
            };
            if !active {
                return 0;
            }
            let suffix = condition_attribute_suffix(id);
            entity
                .attributes
                .get_float_default(&format!("{suffix}_stacks"), 1.0)
                .round()
                .max(1.0) as usize
        })
    }

    pub(crate) fn explain_with_conditions(
        &self,
        entity: &Entity,
        attribute: &str,
        default: f32,
        stacks: &impl Fn(&str) -> usize,
    ) -> Result<ValueExplanation, String> {
        self.evaluate(entity, attribute, default, stacks, &mut BTreeSet::new())
    }

    fn evaluate(
        &self,
        entity: &Entity,
        attribute: &str,
        default: f32,
        stacks: &impl Fn(&str) -> usize,
        visiting: &mut BTreeSet<String>,
    ) -> Result<ValueExplanation, String> {
        let key = attribute.to_ascii_lowercase();
        if !visiting.insert(key.clone()) {
            return Err(format!("Cyclic effective attribute '{attribute}'"));
        }
        let saved = entity.attributes.get_float_default(attribute, default);
        if !saved.is_finite() {
            return Err("Non-finite saved attribute".into());
        }
        let mut result = ValueExplanation {
            attribute: attribute.into(),
            formula: None,
            saved,
            baseline: saved,
            dependencies: BTreeMap::new(),
            contributions: Vec::new(),
            value: saved,
        };
        if let Some(derived) = self
            .derived
            .values()
            .find(|v| v.id.eq_ignore_ascii_case(attribute))
        {
            result.formula = Some(derived.formula.clone());
            let mut error = None;
            let value = eldiron_ruleset::evaluate_formula(&derived.formula, |name| {
                if name == "base" {
                    return saved;
                }
                if name == "level" {
                    return self
                        .level_attribute
                        .as_deref()
                        .map(|id| entity.attributes.get_float_default(id, 1.0))
                        .unwrap_or(1.0);
                }
                match self.evaluate(entity, name, 0.0, stacks, visiting) {
                    Ok(dependency) => {
                        let value = dependency.value;
                        result.dependencies.insert(name.into(), dependency);
                        value
                    }
                    Err(message) => {
                        error = Some(message);
                        0.0
                    }
                }
            })
            .ok_or("Invalid derived formula")?;
            if let Some(error) = error {
                return Err(error);
            }
            result.baseline = value;
            if let Some(min) = derived.minimum {
                result.baseline = result.baseline.max(min);
            }
            if let Some(max) = derived.maximum {
                result.baseline = result.baseline.min(max);
            }
        }
        for condition in self.conditions.values() {
            let count = stacks(&condition.id);
            if count == 0 {
                continue;
            }
            for modifier in condition
                .modifiers
                .iter()
                .filter(|m| m.attribute.eq_ignore_ascii_case(attribute))
            {
                result.contributions.push(Contribution {
                    identity: format!("condition/{}/{}", entity.creator_id, condition.id),
                    definition: format!("conditions.{}", condition.id),
                    label: condition.name.clone(),
                    item: None,
                    active: true,
                    reason: None,
                    add: modifier.add as f64 * count as f64,
                    multiply: (modifier.multiply as f64).powi(count.min(i32::MAX as usize) as i32),
                    minimum: modifier.minimum.map(f64::from),
                    maximum: modifier.maximum.map(f64::from),
                });
            }
        }
        let mut seen = BTreeSet::new();
        let equipped = entity.equipped.values().map(|i| (i, true));
        let carried = entity.iter_inventory().map(|(_, i)| (i, false));
        for (item, worn) in equipped.chain(carried) {
            if !seen.insert((item.id, item.creator_id)) {
                continue;
            }
            let label = item.attributes.get_str("name").unwrap_or(&item.item_type);
            let usable = crate::equipment_lifecycle::active(&self.equipment, item);
            let reason = if !worn {
                Some("not equipped".into())
            } else if !usable {
                Some("broken or invalid condition".into())
            } else {
                None
            };
            if let Some(category) = crate::equipment_lifecycle::category(&self.equipment, item) {
                for projection in category
                    .contributions
                    .iter()
                    .filter(|p| p.attribute.eq_ignore_ascii_case(attribute))
                {
                    if item.attributes.get(&projection.item_attribute).is_none() {
                        continue;
                    }
                    let rating = item
                        .attributes
                        .get_float_default(&projection.item_attribute, 0.0);
                    result.contributions.push(Contribution {
                        identity: format!(
                            "equipment/{}/{}/{}",
                            item.creator_id, item.id, projection.attribute
                        ),
                        definition: format!("equipment.{}.{}", category.id, projection.attribute),
                        label: label.into(),
                        item: Some(item.id),
                        active: reason.is_none(),
                        reason: reason.clone(),
                        add: rating as f64,
                        multiply: 1.0,
                        minimum: None,
                        maximum: None,
                    });
                }
            }
            for attachment in enchantments(item)? {
                let definition = self
                    .enchantments
                    .get(&attachment.definition)
                    .ok_or_else(|| format!("Unknown enchantment '{}'", attachment.definition))?;
                if !compatible(definition, item) {
                    return Err("Incompatible saved enchantment".into());
                }
                let positive = crate::equipment_lifecycle::condition(item).is_some_and(|v| v > 0.0);
                let inactive = if !worn {
                    Some("not equipped".into())
                } else if definition.requires_positive_condition && !positive {
                    Some("broken or invalid condition".into())
                } else {
                    None
                };
                for modifier in definition
                    .modifiers
                    .iter()
                    .filter(|m| m.attribute.eq_ignore_ascii_case(attribute))
                {
                    result.contributions.push(Contribution {
                        identity: format!(
                            "enchantment/{}/{}/{}",
                            item.creator_id, item.id, attachment.id
                        ),
                        definition: format!("enchantments.{}", definition.id),
                        label: format!("{label}: {}", definition.name),
                        item: Some(item.id),
                        active: inactive.is_none(),
                        reason: inactive.clone(),
                        add: modifier.add as f64,
                        multiply: modifier.multiply as f64,
                        minimum: modifier.minimum.map(f64::from),
                        maximum: modifier.maximum.map(f64::from),
                    });
                }
            }
        }
        result
            .contributions
            .sort_by(|a, b| a.identity.cmp(&b.identity));
        let mut add = 0.0;
        let mut multiply = 1.0;
        let mut minimum: Option<f64> = None;
        let mut maximum: Option<f64> = None;
        for source in &result.contributions {
            if ![source.add, source.multiply]
                .into_iter()
                .all(f64::is_finite)
            {
                return Err("Non-finite contribution".into());
            }
            if !source.active {
                continue;
            }
            add += source.add;
            multiply *= source.multiply;
            if let Some(min) = source.minimum {
                minimum = Some(minimum.map_or(min, |v| v.max(min)));
            }
            if let Some(max) = source.maximum {
                maximum = Some(maximum.map_or(max, |v| v.min(max)));
            }
        }
        let mut value = (result.baseline as f64 + add) * multiply;
        if let Some(min) = minimum {
            value = value.max(min);
        }
        if let Some(max) = maximum {
            value = value.min(max);
        }
        if !value.is_finite() || value < f32::MIN as f64 || value > f32::MAX as f64 {
            return Err("Non-finite or out-of-range effective value".into());
        }
        result.value = value as f32;
        visiting.remove(&key);
        Ok(result)
    }

    /// Legacy structured combat can request a separate equipment sum. Ratings
    /// already projected into the same actor attribute must not be charged twice.
    pub fn projects(&self, item: &Item, attribute: &str) -> bool {
        crate::equipment_lifecycle::category(&self.equipment, item).is_some_and(|c| {
            c.contributions.iter().any(|p| {
                p.attribute.eq_ignore_ascii_case(attribute)
                    && p.item_attribute.eq_ignore_ascii_case(attribute)
            })
        })
    }
}

fn condition_attribute_suffix(condition_id: &str) -> String {
    let mut suffix = "condition_".to_string();
    for ch in condition_id.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            suffix.push(ch.to_ascii_lowercase());
        } else {
            suffix.push('_');
        }
    }
    while suffix.contains("__") {
        suffix = suffix.replace("__", "_");
    }
    suffix.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_enchantments_compose_preserve_legacy_ids_and_restore_names_on_removal() {
        let rules = rules();
        let mut item = cuirass();
        item.set_attribute("category", Value::Str("leather".into()));
        item.set_attribute("name", Value::Str("Leather Vest".into()));
        let before = serde_json::to_value(&item).unwrap();
        assert_eq!(
            enchanted_name(&item, &rules, "fox").unwrap().as_deref(),
            Some("Leather Vest of the Fox")
        );
        assert_eq!(serde_json::to_value(&item).unwrap(), before);
        let fox = attach(&mut item, &rules, "fox").unwrap();
        let bear = attach(&mut item, &rules, "strength").unwrap();
        assert_eq!(
            item.attributes.get_str("name"),
            Some("Leather Vest of the Bear and the Fox")
        );
        let saved = serde_json::to_value(&item).unwrap();
        assert!(attach(&mut item, &rules, "fox").is_err());
        assert_eq!(serde_json::to_value(&item).unwrap(), saved);
        let mut actor = Entity::new();
        actor.set_attribute("STR", Value::Int(10));
        actor.set_attribute("DEX", Value::Int(10));
        actor.equipped.insert("torso".into(), item.clone());
        assert_eq!(rules.explain(&actor, "STR", 0.).unwrap().value, 12.);
        assert_eq!(rules.explain(&actor, "DEX", 0.).unwrap().value, 12.);
        actor
            .equipped
            .get_mut("torso")
            .unwrap()
            .set_attribute("condition", Value::Int(0));
        assert_eq!(rules.explain(&actor, "DEX", 0.).unwrap().value, 10.);
        detach(&mut item, &fox).unwrap();
        assert_eq!(
            item.attributes.get_str("name"),
            Some("Leather Vest of the Bear")
        );
        assert_eq!(enchantments(&item).unwrap()[0].id, bear);
        detach(&mut item, &bear).unwrap();
        assert_eq!(item.attributes.get_str("name"), Some("Leather Vest"));
        item.set_attribute("name", Value::Str("Wayfarer's Vest of the Bear".into()));
        item.attributes.remove("enchantment_base_name");
        item.set_attribute(
            ATTACHMENTS,
            Value::Str(r#"[{"id":"legacy-bear","definition":"strength"}]"#.into()),
        );
        attach(&mut item, &rules, "fox").unwrap();
        assert_eq!(
            item.attributes.get_str("name"),
            Some("Wayfarer's Vest of the Bear and the Fox")
        );
        assert_eq!(enchantments(&item).unwrap()[0].id, "legacy-bear");
    }
    #[test]
    fn instances_from_one_template_keep_distinct_sources_and_materialized_attachment_ids() {
        let rules = rules();
        let template = Uuid::new_v4();
        let data = "[attributes]\ncategory='bone_armor'\nenchantments=['strength']";
        let mut first = Item::new();
        first.id = 1;
        first.creator_id = template;
        let mut second = Item::new();
        second.id = 2;
        second.creator_id = template;
        crate::server::data::apply_item_data(&mut first, data);
        crate::server::data::apply_item_data(&mut second, data);
        assert_ne!(
            enchantments(&first).unwrap()[0].id,
            enchantments(&second).unwrap()[0].id
        );
        let identities = enchantments(&first).unwrap();
        crate::server::data::apply_item_data(&mut first, data);
        assert_eq!(enchantments(&first).unwrap(), identities);
        let mut actor = Entity::new();
        actor.equipped.insert("first".into(), first.clone());
        actor.equipped.insert("second".into(), second);
        actor.equipped.insert("same-source-alias".into(), first);
        assert_eq!(rules.explain(&actor, "STR", 0.).unwrap().value, 4.);
    }
    fn rules() -> ValueRules {
        ValueRules::resolve(
            &eldiron_ruleset::document::RulesState::default()
                .document()
                .unwrap()
                .table(),
        )
        .unwrap()
    }
    fn cuirass() -> Item {
        let mut item = Item::new();
        item.id = 42;
        item.item_type = "Bone Cuirass".into();
        item.set_attribute("category", Value::Str("bone_armor".into()));
        item.set_attribute("ruleset_kind", Value::Str("armor".into()));
        item.set_attribute("slot", Value::Str("torso".into()));
        item.set_attribute("ARMOR", Value::Int(4));
        item.set_attribute("condition", Value::Int(100));
        item
    }
    #[test]
    fn equipment_enchantments_dependencies_and_conditions_share_one_projection() {
        let rules = rules();
        let mut item = cuirass();
        let attachment = attach(&mut item, &rules, "strength").unwrap();
        let mut actor = Entity::new();
        actor.set_attribute("STR", Value::Int(12));
        actor.set_attribute("ARMOR", Value::Int(5));
        actor.equipped.insert("torso".into(), item);
        let before = serde_json::to_value(&actor).unwrap();
        assert_eq!(rules.explain(&actor, "STR", 0.).unwrap().value, 14.);
        let damage = rules.explain(&actor, "DMG", 0.).unwrap();
        assert_eq!(damage.value, 1.);
        assert_eq!(damage.dependencies["STR"].value, 14.);
        assert_eq!(rules.explain(&actor, "ARMOR", 0.).unwrap().value, 9.);
        assert_eq!(serde_json::to_value(&actor).unwrap(), before);
        actor.set_attribute("conditions", Value::StrArray(vec!["guarded".into()]));
        actor.set_attribute("condition_guarded_stacks", Value::Int(1));
        assert_eq!(rules.explain(&actor, "ARMOR", 0.).unwrap().value, 11.);
        actor
            .equipped
            .get_mut("torso")
            .unwrap()
            .set_attribute("condition", Value::Int(0));
        let broken = rules.explain(&actor, "STR", 0.).unwrap();
        assert_eq!(broken.value, 12.);
        assert!(broken.summary().contains("inactive: broken"));
        assert_eq!(rules.explain(&actor, "ARMOR", 0.).unwrap().value, 7.);
        actor
            .equipped
            .get_mut("torso")
            .unwrap()
            .set_attribute("condition", Value::Int(20));
        assert_eq!(rules.explain(&actor, "STR", 0.).unwrap().value, 14.);
        let item = actor.equipped.get_mut("torso").unwrap();
        assert_eq!(enchantments(item).unwrap()[0].id, attachment);
        detach(item, &attachment).unwrap();
        assert_eq!(rules.explain(&actor, "STR", 0.).unwrap().value, 12.);
        assert_eq!(actor.attributes.get_int("STR"), Some(12));
    }
    #[test]
    fn attachment_transfer_delta_and_repeated_inspection_preserve_exact_identity() {
        let rules = rules();
        let mut item = cuirass();
        let id = attach(&mut item, &rules, "strength").unwrap();
        let saved = serde_json::to_value(&item).unwrap();
        assert!(attach(&mut item, &rules, "strength").is_err());
        assert!(detach(&mut item, "missing").is_err());
        assert_eq!(serde_json::to_value(&item).unwrap(), saved);
        let mut client = Item::new();
        client.id = item.id;
        client.apply_update(crate::server::item::ItemUpdate::unpack(
            &item.get_update().pack(),
        ));
        assert_eq!(enchantments(&client).unwrap()[0].id, id);
        let mut alice = Entity::new();
        alice.set_attribute("STR", Value::Int(10));
        alice.equipped.insert("torso".into(), item);
        for _ in 0..3 {
            assert_eq!(rules.explain(&alice, "STR", 0.).unwrap().value, 12.);
        }
        let item: Item = serde_json::from_value(
            serde_json::to_value(alice.equipped.shift_remove("torso").unwrap()).unwrap(),
        )
        .unwrap();
        let mut bob = Entity::new();
        bob.set_attribute("STR", Value::Int(8));
        bob.inventory = vec![Some(item)];
        assert_eq!(rules.explain(&alice, "STR", 0.).unwrap().value, 10.);
        assert_eq!(rules.explain(&bob, "STR", 0.).unwrap().value, 8.);
        bob.equip_item_unchecked(42, "torso").unwrap();
        assert_eq!(rules.explain(&bob, "STR", 0.).unwrap().value, 10.);
        assert_eq!(
            enchantments(bob.get_equipped_item("torso").unwrap()).unwrap()[0].id,
            id
        );
    }
    #[test]
    fn custom_names_and_modifier_arithmetic_do_not_require_official_attributes() {
        let root = r#"
            [attributes]
            primary=['MIGHT','WARD']
            [equipment]
            armor_slots=['shell']
            [equipment.armor_categories.carapace]
            contributions=[{attribute='WARD',item_attribute='WARD'}]
            [enchantments.might]
            name='Might'
            modifiers=[{attribute='MIGHT',add=2,multiply=0.5}]
            [conditions.weak]
            stacking='stack'
            max_stacks=2
            modifiers=[{attribute='MIGHT',add=-1,multiply=0.9}]
        "#
        .parse()
        .unwrap();
        let rules = ValueRules::resolve(&root).unwrap();
        let mut item = Item::new();
        item.id = 1;
        item.set_attribute("category", Value::Str("carapace".into()));
        item.set_attribute("WARD", Value::Int(3));
        attach(&mut item, &rules, "might").unwrap();
        let mut actor = Entity::new();
        actor.equipped.insert("shell".into(), item);
        actor.set_attribute("MIGHT", Value::Int(10));
        actor.set_attribute("WARD", Value::Int(5));
        actor.set_attribute("conditions", Value::StrArray(vec!["weak".into()]));
        actor.set_attribute("condition_weak_stacks", Value::Int(2));
        assert!((rules.explain(&actor, "MIGHT", 0.).unwrap().value - 4.05).abs() < 0.0001);
        assert_eq!(rules.explain(&actor, "WARD", 0.).unwrap().value, 8.);
    }
    #[test]
    fn malformed_or_repeated_attachment_identities_are_rejected_without_mutation() {
        let rules = rules();
        let mut item = cuirass();
        item.set_attribute(ATTACHMENTS, Value::Str("invalid json".into()));
        let before = serde_json::to_value(&item).unwrap();
        assert!(attach(&mut item, &rules, "strength").is_err());
        assert_eq!(serde_json::to_value(&item).unwrap(), before);
        item.set_attribute(ATTACHMENTS, Value::Str("[]".into()));
        item.set_attribute("condition", Value::Float(f32::NAN));
        attach(&mut item, &rules, "strength").unwrap();
        let mut actor = Entity::new();
        actor.equipped.insert("torso".into(), item);
        assert_eq!(rules.explain(&actor, "STR", 0.).unwrap().value, 0.);
    }
}
