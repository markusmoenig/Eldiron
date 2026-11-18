//! Shared equipment activation for combat, stats and presentation.
use crate::{Entity, Item, Value};
use eldiron_ruleset::{ResolvedEquipmentCategory, ResolvedEquipmentPolicy};

pub(crate) fn category<'a>(
    policy: &'a ResolvedEquipmentPolicy,
    item: &Item,
) -> Option<&'a ResolvedEquipmentCategory> {
    let category = item
        .attributes
        .get_str("category")?
        .trim()
        .to_ascii_lowercase();
    let weapon = item.attributes.get_str("ruleset_kind") == Some("weapon")
        || item.attributes.get_str("slot").is_some_and(|slot| {
            policy
                .weapon_slots
                .iter()
                .any(|s| s.eq_ignore_ascii_case(slot))
        });
    if weapon {
        policy.weapon_categories.get(&category)
    } else {
        policy.armor_categories.get(&category)
    }
}

pub(crate) fn condition(item: &Item) -> Option<f32> {
    match item.attributes.get("condition") {
        None => Some(100.0),
        Some(Value::Int(_) | Value::UInt(_) | Value::Int64(_) | Value::Float(_)) => {
            let value = item.attributes.get_float("condition")?;
            value.is_finite().then_some(value)
        }
        _ => None,
    }
}

pub fn active(policy: &ResolvedEquipmentPolicy, item: &Item) -> bool {
    if !category(policy, item).is_some_and(|c| c.requires_positive_condition) {
        return true;
    }
    condition(item).is_some_and(|value| value > 0.0)
}

/// Called only after positive health damage actually commits. No hit attempt,
/// absorbed damage or notification replay can charge wear through this helper.
pub(crate) fn damage_wear(policy: &ResolvedEquipmentPolicy, entity: &mut Entity, kind: &str) {
    let mut changed = false;
    for (slot, item) in &mut entity.equipped {
        let Some(wear) = category(policy, item).and_then(|c| c.wear.as_ref()) else {
            continue;
        };
        if !wear.slot.eq_ignore_ascii_case(slot) || wear.damage_kind != kind {
            continue;
        }
        let Some(condition) = condition(item) else {
            continue;
        };
        if condition <= 0.0 {
            continue;
        }
        let next = (condition - wear.condition_cost as f32).max(0.0);
        let value = match item.attributes.get("condition") {
            Some(Value::Float(_)) => Value::Float(next),
            Some(Value::UInt(_)) => Value::UInt(next as u32),
            Some(Value::Int64(_)) => Value::Int64(next as i64),
            _ => Value::Int(next as i32),
        };
        item.set_attribute("condition", value);
        changed = true;
    }
    if changed {
        entity.dirty_flags |= 0b10000;
    }
}
