//! Opt-in equipment lifecycle policies. Existing categories keep their behavior.
use serde::Deserialize;
use toml::Table;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wear {
    pub damage_kind: String,
    pub slot: String,
    pub condition_cost: u32,
}

pub fn wear(root: &Table, category: &Table) -> Result<Option<Wear>, String> {
    let result: Option<Wear> = category
        .get("wear")
        .map(|v| {
            v.clone()
                .try_into()
                .map_err(|e| format!("Invalid equipment wear policy: {e}"))
        })
        .transpose()?;
    if let Some(wear) = &result {
        if wear.condition_cost == 0 || wear.condition_cost > 100 {
            return Err("Equipment wear cost must be from 1 to 100".into());
        }
        let equipment = root.get("equipment");
        if !["weapon_slots", "armor_slots"].iter().any(|key| {
            equipment
                .and_then(|v| v.get(key))
                .and_then(toml::Value::as_array)
                .is_some_and(|slots| {
                    slots.iter().any(|v| {
                        v.as_str()
                            .is_some_and(|slot| slot.eq_ignore_ascii_case(&wear.slot))
                    })
                })
        }) {
            return Err("Wear must select a declared equipment slot".into());
        }
        if root
            .get("combat")
            .and_then(|v| v.get("kinds"))
            .and_then(|v| v.get(&wear.damage_kind))
            .is_none()
        {
            return Err("Wear must name a declared damage kind".into());
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn lifecycle_policies_are_opt_in_and_validate_kind_slot_cost_and_types() {
        let base = crate::document::RulesState::default()
            .document()
            .unwrap()
            .table()
            .clone();
        assert!(
            !crate::resolve_equipment_policy(&base)
                .unwrap()
                .armor_categories["leather"]
                .requires_positive_condition
        );
        for (key, value) in [
            ("slot", toml::Value::String("unknown".into())),
            ("damage_kind", toml::Value::String("unknown".into())),
            ("condition_cost", toml::Value::Integer(0)),
            ("condition_cost", toml::Value::Integer(101)),
        ] {
            let mut root = base.clone();
            root["equipment"]["armor_categories"]["bone_armor"]["wear"][key] = value;
            assert!(crate::resolve_equipment_policy(&root).is_err());
        }
        let mut root = base;
        root["equipment"]["armor_categories"]["bone_armor"]["requires_positive_condition"] =
            toml::Value::String("true".into());
        assert!(crate::resolve_equipment_policy(&root).is_err());
    }
}
