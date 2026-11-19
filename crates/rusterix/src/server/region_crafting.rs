//! Immediate recipes stage debits, outputs and owned-target changes on one actor copy.
use super::*;
use std::collections::BTreeMap;

#[path = "region_recipe_preview.rs"]
pub mod preview;

pub(super) fn item_number(item: &Item, key: &str, default: i32) -> i32 {
    match item.attributes.get(key) {
        Some(Value::Int(n)) => *n,
        Some(Value::UInt(n)) => i32::try_from(*n).unwrap_or(-1),
        Some(Value::Int64(n)) => i32::try_from(*n).unwrap_or(-1),
        None => default,
        _ => -1,
    }
}
fn has_capability(item: &Item, key: &str, capability: &str) -> bool {
    matches!(item.attributes.get(key), Some(Value::StrArray(values)) if values.iter().any(|v| v == capability))
}
fn owned(actor: &Entity) -> impl Iterator<Item = &Item> {
    actor
        .iter_inventory()
        .map(|(_, i)| i)
        .chain(actor.equipped.values())
}
fn owned_mut(actor: &mut Entity, id: u32) -> Option<&mut Item> {
    if actor.get_item_slot(id).is_some() {
        actor.get_item_mut(id)
    } else {
        actor.equipped.values_mut().find(|i| i.id == id)
    }
}
fn consume_liquid(actor: &mut Entity, item: &str, mut amount: usize) -> Result<(), String> {
    let mut vessels: Vec<u32> = owned(actor)
        .filter(|v| {
            let capacity = item_number(v, "liquid_capacity", 0);
            let quantity = item_number(v, "liquid_quantity", 0);
            capacity > 0
                && quantity > 0
                && quantity <= capacity
                && v.attributes.get_str("liquid_item") == Some(item)
        })
        .map(|v| v.id)
        .collect();
    vessels.sort_unstable();
    for id in vessels {
        if amount == 0 {
            break;
        }
        let vessel = owned_mut(actor, id).ok_or("Vessel is no longer owned")?;
        let quantity = item_number(vessel, "liquid_quantity", 0) as usize;
        let debit = amount.min(quantity);
        vessel.set_attribute("liquid_quantity", Value::Int((quantity - debit) as i32));
        if quantity == debit {
            // Item deltas merge attributes; explicitly clear the type so an
            // empty vessel does not retain stale liquid presentation remotely.
            vessel.set_attribute("liquid_item", Value::Str(String::new()));
        }
        amount -= debit;
    }
    if amount > 0 {
        return Err(format!("Not enough {item} in owned vessels"));
    }
    Ok(())
}

pub fn execute(
    ctx: &mut RegionCtx,
    actor_id: u32,
    recipe_id: &str,
    action_costs: &[(String, usize)],
) -> Result<(), String> {
    execute_plan(ctx, actor_id, recipe_id, action_costs, None, None)
}

/// Trusted callers must name the exact owned item; selection never substitutes
/// another instance of the same definition.
pub fn execute_targeted(
    ctx: &mut RegionCtx,
    actor_id: u32,
    recipe_id: &str,
    target: u32,
) -> Result<(), String> {
    if !ruleset_recipe_table(ctx, recipe_id)
        .is_some_and(|recipe| recipe.contains_key("repair") || recipe.contains_key("enchant"))
    {
        return Err("This recipe does not mutate an owned target".into());
    }
    execute_plan(ctx, actor_id, recipe_id, &[], None, Some(target))
}

pub(super) fn execute_action(
    ctx: &mut RegionCtx,
    actor_id: u32,
    recipe_id: &str,
    costs: &[(String, usize)],
    action: &ResolvedAction,
    target: Option<&RulesetActionTarget>,
) -> Result<(), String> {
    validate_action(ctx, actor_id, action, target)?;
    let target = match target {
        Some(RulesetActionTarget::Item {
            item_id,
            owner_entity_id: Some(owner),
        }) if *owner == actor_id => Some(*item_id),
        _ => None,
    };
    execute_plan(ctx, actor_id, recipe_id, costs, Some(action), target)
}

fn validate_action(
    ctx: &RegionCtx,
    actor_id: u32,
    action: &ResolvedAction,
    target: Option<&RulesetActionTarget>,
) -> Result<(), String> {
    let actor = ctx
        .map
        .entities
        .iter()
        .find(|actor| actor.id == actor_id)
        .ok_or("Crafter is unavailable")?;
    if !entity_meets_action_attribute_requirements(ctx, actor, action) {
        return Err("Action attribute requirements are not met".into());
    }
    if ruleset_action_target_entity(ctx, actor_id, action, target).is_some_and(|target| {
        !entity_meets_action_target_attribute_requirements(ctx, target, action)
    }) {
        return Err("Target attribute requirements are not met".into());
    }
    if action
        .required_ability()
        .is_some_and(|id| !entity_knows_ruleset_ability(ctx, actor, id))
        || action
            .required_spell()
            .is_some_and(|id| !entity_knows_ruleset_spell(ctx, actor, id))
    {
        return Err("Action has not been learned".into());
    }
    if let Some((skill, required)) = action.skill_requirement() {
        if entity_skill_points(actor, skill) < required {
            return Err(format!("Need {skill} {required}"));
        }
    }
    if is_action_on_cooldown(ctx, actor_id, &action.id) {
        return Err("Action is on cooldown".into());
    }
    if !resolved_runtime_action_target_allowed(ctx, &action.target, actor_id, target)
        || !resolved_action_target_is_in_range(ctx, actor, action, target)
    {
        return Err("Select an accessible target".into());
    }
    Ok(())
}

fn execute_plan(
    ctx: &mut RegionCtx,
    actor_id: u32,
    recipe_id: &str,
    action_costs: &[(String, usize)],
    action: Option<&ResolvedAction>,
    target: Option<u32>,
) -> Result<(), String> {
    let recipe = ruleset_recipe_table(ctx, recipe_id).ok_or("Unknown recipe")?;
    let requirements = eldiron_ruleset::crafting::requirements(recipe)?;
    let index = ctx
        .map
        .entities
        .iter()
        .position(|e| e.id == actor_id)
        .ok_or("Crafter is unavailable")?;
    let mut actor = ctx.map.entities[index].clone();
    let resource_debits = match action {
        Some(action) => {
            prepare_action_resource_costs(&actor, action).ok_or("Insufficient action resources")?
        }
        None => Vec::new(),
    };
    if ctx.paused || actor.get_mode() == "dead" {
        return Err("Crafter cannot work now".into());
    }
    ctx.with_value_rules(|rules| crate::learning::recipe_access(rules, &actor, recipe_id))??;
    check_recipe_requirements(ctx, &actor, recipe)?;
    let repair = if let Some(repair) = &requirements.repair {
        let target_id = target.ok_or("Select the exact owned item to repair")?;
        let item = owned(&actor)
            .find(|i| i.id == target_id)
            .ok_or("Repair target is no longer owned")?;
        ctx.with_value_rules(|rules| rules.validate_item(item))??;
        if !preview::target_matches_recipe(&ctx.rules, recipe, item)? {
            return Err("This repair does not apply to the selected item".into());
        }
        let condition = item_number(item, "condition", 100);
        if condition < 0 || condition >= repair.maximum as i32 {
            return Err("Selected item does not need this repair".into());
        }
        Some((
            target_id,
            (condition + repair.amount as i32).min(repair.maximum as i32),
        ))
    } else {
        None
    };
    let enchanted = if let Some(enchant) = &requirements.enchant {
        let target_id = target.ok_or("Select the exact owned item to enchant")?;
        let mut item = owned(&actor)
            .find(|item| item.id == target_id)
            .ok_or("Enchanting target is no longer owned")?
            .clone();
        let definition = item
            .attributes
            .get_str("ruleset_id")
            .ok_or("Enchanting target has no definition")?;
        if !preview::target_matches_recipe(&ctx.rules, recipe, &item)? || item.stack_quantity() != 1
        {
            return Err("This inscription does not apply to the selected item".into());
        }
        if item_number(&item, "condition", 100) <= 0 {
            return Err("Repair the selected item before enchanting it".into());
        }
        if item_quantity_entries(recipe, "consumes")
            .iter()
            .chain(action_costs.iter())
            .any(|(id, _)| id == definition)
        {
            return Err("An enchanting recipe cannot consume its target".into());
        }
        ctx.with_value_rules(|rules| {
            crate::effective_values::attach(&mut item, rules, &enchant.definition)
        })??;
        if let Some(name) = &enchant.name {
            item.set_attribute("name", Value::Str(name.clone()));
        }
        Some((target_id, item))
    } else {
        None
    };
    let mutation_target = repair
        .map(|(id, _)| id)
        .or_else(|| enchanted.as_ref().map(|(id, _)| *id));
    let tool = if let Some(required) = &requirements.tool {
        Some(
            owned(&actor)
                .filter(|item| {
                    mutation_target != Some(item.id)
                        && has_capability(item, "tool_capabilities", &required.capability)
                        && item_number(item, "condition", 100)
                            >= required.condition_cost.max(1) as i32
                })
                .map(|item| item.id)
                .min()
                .ok_or("A usable owned tool is required")?,
        )
    } else {
        None
    };
    if let Some(required) = &requirements.station {
        if !ctx.map.items.iter().any(|item| {
            let distance = item.position.distance(actor.position);
            item.attributes.get_bool_default("visible", true)
                && item_number(item, "condition", 100) > 0
                && has_capability(item, "station_capabilities", &required.capability)
                && distance.is_finite()
                && distance <= required.range
        }) {
            return Err("A nearby deployed station is required".into());
        }
    }

    // Aggregate duplicate entries and action costs before testing availability.
    let mut costs = BTreeMap::<String, usize>::new();
    for (item, quantity) in item_quantity_entries(recipe, "consumes")
        .into_iter()
        .chain(action_costs.iter().cloned())
    {
        let total = costs.entry(item).or_default();
        *total = total
            .checked_add(quantity)
            .filter(|n| *n <= 1_000_000)
            .ok_or("Recipe input quantity exceeds bounds")?;
    }
    for (id, quantity) in costs {
        let (_, definition) = ruleset_item_table_by_id(&ctx.rules, &id)
            .ok_or("Recipe input definition is missing")?;
        if rule_string(definition, "quantity_unit") == Some("volume") {
            consume_liquid(&mut actor, &id, quantity)?;
        } else {
            let debit = vec![(id, quantity)];
            if entity_has_action_consumes(&actor, &debit).is_some() {
                return Err("Recipe materials are missing".into());
            }
            consume_entity_items(&mut actor, &debit);
        }
    }
    if let Some(id) = tool {
        let tool = owned_mut(&mut actor, id).ok_or("Recipe cannot consume its required tool")?;
        let condition =
            item_number(tool, "condition", 100) - requirements.tool.unwrap().condition_cost as i32;
        tool.set_attribute("condition", Value::Int(condition));
    }
    if let Some((id, condition)) = repair {
        owned_mut(&mut actor, id)
            .ok_or("Recipe cannot consume its repair target")?
            .set_attribute("condition", Value::Int(condition));
    }
    if let Some((id, item)) = enchanted {
        *owned_mut(&mut actor, id).ok_or("Recipe cannot consume its enchanting target")? = item;
    }
    let output_quality = requirements
        .fixed_quality
        .unwrap_or_else(|| recipe_output_quality(&actor, recipe));
    for (id, quantity) in item_quantity_entries(recipe, "produces") {
        if quantity > 1_000_000 {
            return Err("Recipe output quantity exceeds bounds".into());
        }
        let (_, definition) = ruleset_item_table_by_id(&ctx.rules, &id)
            .ok_or("Recipe output definition is missing")?;
        if rule_string(definition, "quantity_unit") == Some("volume") {
            return Err("Liquid recipe outputs require a vessel transfer policy".into());
        }
        let mut item = ruleset_item_from_table(&ctx.rules, &id, 1, &ctx.world)
            .ok_or("Recipe output definition is missing")?;
        item.set_attribute("quality", Value::Int(output_quality));
        item.set_attribute("condition", Value::Int(100));
        if item.is_stackable() {
            item.set_stack_quantity(quantity as i32);
            actor
                .add_item(item)
                .map_err(|_| "The complete output does not fit")?;
        } else {
            actor
                .add_item(item)
                .map_err(|_| "The complete output does not fit")?;
            for _ in 1..quantity {
                let mut item = ruleset_item_from_table(&ctx.rules, &id, 1, &ctx.world)
                    .ok_or("Recipe output definition is missing")?;
                item.set_attribute("quality", Value::Int(output_quality));
                item.set_attribute("condition", Value::Int(100));
                actor
                    .add_item(item)
                    .map_err(|_| "The complete output does not fit")?;
            }
        }
    }
    apply_recipe_skill_gain(&ctx.rules, &mut actor, recipe);
    for (key, value) in resource_debits {
        actor.set_attribute(&key, value);
    }
    if requirements.enchant.is_some() {
        ctx.with_value_rules(|rules| {
            for attribute in rules.affected_attributes(&actor)? {
                rules.explain_with_conditions(&actor, &attribute, 0.0, &|id| {
                    active_condition_stacks(ctx, actor_id, id)
                })?;
            }
            Ok::<_, String>(())
        })??;
    }
    actor.dirty_flags |= 0b11000;
    ctx.map.entities[index] = actor;
    Ok(())
}
#[cfg(test)]
#[path = "region_enchanting_tests.rs"]
mod enchanting_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn context(slots: usize) -> (RegionCtx, u32) {
        let mut ctx = RegionCtx::default();
        let rules = eldiron_ruleset::document::RulesState::default()
            .document()
            .unwrap();
        ctx.set_rules(rules.table().clone()).unwrap();
        let mut actor = Entity::default();
        actor.id = ctx.world.next_id();
        actor.position = Vec3::zero();
        actor.inventory = vec![None; slots];
        let id = actor.id;
        ctx.map.entities.push(actor);
        (ctx, id)
    }
    fn add(ctx: &mut RegionCtx, id: &str, quantity: usize) -> u32 {
        let item = ruleset_item_from_table(&ctx.rules, id, quantity, &ctx.world).unwrap();
        let id = item.id;
        ctx.map.entities[0].add_item(item).unwrap();
        id
    }
    fn snapshot(ctx: &RegionCtx) -> serde_json::Value {
        serde_json::to_value(&ctx.map.entities[0]).unwrap()
    }
    fn repair_fixture(condition: i32) -> (RegionCtx, u32, u32, u32) {
        let (mut ctx, actor) = context(8);
        add(&mut ctx, "bone_plate", 2);
        add(&mut ctx, "leather_strap", 2);
        let knife = add(&mut ctx, "stone_knife", 1);
        let target = add(&mut ctx, "bone_cuirass", 1);
        let mut bench =
            ruleset_item_from_table(&ctx.rules, "bone_workbench", 1, &ctx.world).unwrap();
        bench.position = Vec3::zero();
        ctx.map.items.push(bench);
        let armor = ctx.map.entities[0].get_item_mut(target).unwrap();
        armor.set_attribute("condition", Value::Int(condition));
        armor.set_attribute("quality", Value::Int(73));
        armor.set_attribute(
            "future_attachment",
            Value::Str("preserved opaque metadata".into()),
        );
        armor.set_attribute("charges", Value::Int(3));
        (ctx, actor, target, knife)
    }

    #[test]
    fn repair_preserves_exact_inventory_and_equipped_identity_and_only_changes_condition() {
        for equipped in [false, true] {
            let (mut ctx, actor, target, knife) = repair_fixture(80);
            let other = add(&mut ctx, "bone_cuirass", 1);
            if equipped {
                ctx.map.entities[0]
                    .equip_item_unchecked(target, "torso")
                    .unwrap();
            }
            let before = owned(&ctx.map.entities[0])
                .find(|i| i.id == target)
                .unwrap()
                .clone();
            execute_targeted(&mut ctx, actor, "repair_bone_armor", target).unwrap();
            let after = owned(&ctx.map.entities[0])
                .find(|i| i.id == target)
                .unwrap();
            assert_eq!((after.id, after.creator_id), (before.id, before.creator_id));
            let mut expected = before.attributes.clone();
            expected.set("condition", Value::Int(100));
            assert_eq!(
                serde_json::to_value(&after.attributes).unwrap(),
                serde_json::to_value(&expected).unwrap()
            );
            assert_eq!(
                item_number(ctx.map.entities[0].get_item(other).unwrap(), "condition", 0),
                100
            );
            assert_eq!(
                item_number(ctx.map.entities[0].get_item(knife).unwrap(), "condition", 0),
                99
            );
            assert_eq!(
                entity_inventory_item_count(&ctx.map.entities[0], "bone_plate"),
                1
            );
            assert_eq!(
                entity_inventory_item_count(&ctx.map.entities[0], "leather_strap"),
                1
            );
            let before = snapshot(&ctx);
            assert!(execute_targeted(&mut ctx, actor, "repair_bone_armor", target).is_err());
            assert_eq!(snapshot(&ctx), before);
        }
    }

    #[test]
    fn repair_clamps_at_maximum_and_reactivates_a_broken_piece_without_changing_saved_armor() {
        for (condition, expected) in [(95, 100), (0, 20)] {
            let (mut ctx, actor, target, _) = repair_fixture(condition);
            ctx.map.entities[0].set_attribute("ARMOR", Value::Int(7));
            ctx.map.entities[0]
                .equip_item_unchecked(target, "torso")
                .unwrap();
            assert_eq!(
                armor_equipped_attr(&ctx, &ctx.map.entities[0], "ARMOR"),
                if condition == 0 { 0.0 } else { 4.0 }
            );
            execute_targeted(&mut ctx, actor, "repair_bone_armor", target).unwrap();
            assert_eq!(
                item_number(
                    ctx.map.entities[0].get_equipped_item("torso").unwrap(),
                    "condition",
                    -1
                ),
                expected
            );
            assert_eq!(
                armor_equipped_attr(&ctx, &ctx.map.entities[0], "ARMOR"),
                4.0
            );
            assert_eq!(ctx.map.entities[0].attributes.get_int("ARMOR"), Some(7));
        }
    }
    #[test]
    fn text_repair_resolves_owned_equipment_and_requires_exact_selection_for_duplicates() {
        let (mut ctx, actor, target, _) = repair_fixture(80);
        let (sender, messages) = crossbeam_channel::unbounded();
        ctx.from_sender.set(sender).unwrap();
        ctx.map.entities[0]
            .equip_item_unchecked(target, "torso")
            .unwrap();
        let instance = RegionInstance::new(9918);
        instance.handle_text_command(&mut ctx, actor, "action repair_bone_armor Bone Cuirass");
        assert_eq!(
            item_number(
                ctx.map.entities[0].get_equipped_item("torso").unwrap(),
                "condition",
                -1
            ),
            100,
            "text repair feedback: {:?}",
            messages.try_iter().collect::<Vec<_>>()
        );
        ctx.map.entities[0]
            .equipped
            .get_mut("torso")
            .unwrap()
            .set_attribute("condition", Value::Int(80));
        let other = add(&mut ctx, "bone_cuirass", 1);
        ctx.map.entities[0]
            .get_item_mut(other)
            .unwrap()
            .set_attribute("condition", Value::Int(80));
        let before = snapshot(&ctx);
        instance.handle_text_command(&mut ctx, actor, "action repair_bone_armor Bone Cuirass");
        assert_eq!(snapshot(&ctx), before);
        instance.handle_text_command(
            &mut ctx,
            actor,
            &format!("action repair_bone_armor #{target}"),
        );
        assert_eq!(
            item_number(
                ctx.map.entities[0].get_equipped_item("torso").unwrap(),
                "condition",
                -1
            ),
            100
        );
        assert_eq!(
            item_number(
                ctx.map.entities[0].get_item(other).unwrap(),
                "condition",
                -1
            ),
            80
        );
    }

    #[test]
    fn repair_rejects_foreign_missing_wrong_family_targets_and_failed_requirements_atomically() {
        let (mut ctx, actor, target, knife) = repair_fixture(80);
        let action = eldiron_ruleset::resolve_action(&ctx.rules, "repair_bone_armor")
            .unwrap()
            .unwrap();
        for owner in [None, Some(actor + 1)] {
            let before = snapshot(&ctx);
            assert!(
                execute_action(
                    &mut ctx,
                    actor,
                    "repair_bone_armor",
                    &[],
                    &action,
                    Some(&RulesetActionTarget::Item {
                        item_id: target,
                        owner_entity_id: owner
                    })
                )
                .is_err()
            );
            assert_eq!(snapshot(&ctx), before);
        }
        let before = snapshot(&ctx);
        assert!(execute_targeted(&mut ctx, actor, "repair_bone_armor", knife).is_err());
        assert_eq!(snapshot(&ctx), before);
        ctx.map.items[0].position = Vec3::new(0.0, 3.0, 0.0);
        let before = snapshot(&ctx);
        assert!(execute_targeted(&mut ctx, actor, "repair_bone_armor", target).is_err());
        assert_eq!(snapshot(&ctx), before);
        ctx.map.items[0].position = Vec3::zero();
        ctx.map.entities[0]
            .get_item_mut(knife)
            .unwrap()
            .set_attribute("condition", Value::Int(0));
        let before = snapshot(&ctx);
        assert!(execute_targeted(&mut ctx, actor, "repair_bone_armor", target).is_err());
        assert_eq!(snapshot(&ctx), before);
        ctx.map.entities[0]
            .get_item_mut(knife)
            .unwrap()
            .set_attribute("condition", Value::Int(100));
        consume_entity_items(&mut ctx.map.entities[0], &[("leather_strap".into(), 2)]);
        let before = snapshot(&ctx);
        assert!(execute_targeted(&mut ctx, actor, "repair_bone_armor", target).is_err());
        assert_eq!(snapshot(&ctx), before);
        add(&mut ctx, "leather_strap", 2);
        let slot = ctx.map.entities[0].get_item_slot(target).unwrap();
        ctx.map.entities[0].remove_item_from_slot(slot).unwrap();
        let other = add(&mut ctx, "bone_cuirass", 1);
        ctx.map.entities[0]
            .get_item_mut(other)
            .unwrap()
            .set_attribute("condition", Value::Int(80));
        let before = snapshot(&ctx);
        assert!(execute_targeted(&mut ctx, actor, "repair_bone_armor", target).is_err());
        assert_eq!(snapshot(&ctx), before);
    }

    #[test]
    fn declared_wear_occurs_once_after_positive_health_damage_and_only_on_the_selected_slot() {
        let (mut ctx, actor, target, _) = repair_fixture(1);
        ctx.map.entities[0]
            .equip_item_unchecked(target, "torso")
            .unwrap();
        ctx.map.entities[0].set_attribute("HP", Value::Int(100));
        let head = add(&mut ctx, "bone_helm", 1);
        ctx.map.entities[0]
            .equip_item_unchecked(head, "head")
            .unwrap();
        assert!(!apply_damage_direct(
            &mut ctx, actor, 0, 0, "physical", None
        ));
        assert!(apply_damage_direct(&mut ctx, actor, 0, 1, "fire", None));
        assert_eq!(
            item_number(
                ctx.map.entities[0].get_equipped_item("torso").unwrap(),
                "condition",
                -1
            ),
            1
        );
        assert!(apply_damage_direct(&mut ctx, actor, 0, 1, "physical", None));
        let armor = ctx.map.entities[0].get_equipped_item("torso").unwrap();
        assert_eq!(item_number(armor, "condition", -1), 0);
        assert_eq!(
            armor_equipped_attr(&ctx, &ctx.map.entities[0], "ARMOR"),
            1.0
        );
        assert_eq!(
            item_number(
                ctx.map.entities[0].get_equipped_item("head").unwrap(),
                "condition",
                -1
            ),
            100
        );
        execute_targeted(&mut ctx, actor, "repair_bone_armor", target).unwrap();
        assert_eq!(
            armor_equipped_attr(&ctx, &ctx.map.entities[0], "ARMOR"),
            5.0
        );
    }
    #[test]
    fn liquid_inputs_span_vessels_and_only_deployed_reachable_stations_count() {
        let (mut ctx, actor) = context(8);
        add(&mut ctx, "raw_hide", 1);
        add(&mut ctx, "tree_bark", 2);
        let a = add(&mut ctx, "wooden_bowl", 1);
        let b = add(&mut ctx, "wooden_bowl", 1);
        let frame = add(&mut ctx, "tanning_frame", 1);
        for id in [a, b] {
            let vessel = ctx.map.entities[0].get_item_mut(id).unwrap();
            vessel.set_attribute("liquid_item", Value::Str("fresh_water".into()));
            vessel.set_attribute("liquid_quantity", Value::Int(2));
        }
        let before = snapshot(&ctx);
        assert!(execute(&mut ctx, actor, "tan_hide", &[]).is_err());
        assert_eq!(snapshot(&ctx), before);
        let slot = ctx.map.entities[0].get_item_slot(frame).unwrap();
        let mut frame = ctx.map.entities[0].remove_item_from_slot(slot).unwrap();
        frame.position = Vec3::new(0.0, 4.0, 0.0);
        ctx.map.items.push(frame);
        let before = snapshot(&ctx);
        assert!(execute(&mut ctx, actor, "tan_hide", &[]).is_err());
        assert_eq!(snapshot(&ctx), before);
        ctx.map.items[0].position = Vec3::zero();
        let creators = [a, b].map(|id| ctx.map.entities[0].get_item(id).unwrap().creator_id);
        execute(&mut ctx, actor, "tan_hide", &[]).unwrap();
        for (id, creator) in [a, b].into_iter().zip(creators) {
            let vessel = ctx.map.entities[0].get_item(id).unwrap();
            assert_eq!(vessel.creator_id, creator);
            assert_eq!(item_number(vessel, "liquid_quantity", -1), 0);
            assert_eq!(vessel.attributes.get_str("liquid_item"), Some(""));
        }
        assert_eq!(
            entity_inventory_item_count(&ctx.map.entities[0], "leather"),
            2
        );
    }
    #[test]
    fn shortages_capacity_and_unusable_tools_never_charge_partial_inputs_or_practice() {
        let (mut ctx, actor) = context(2);
        add(&mut ctx, "wood_branch", 3);
        let knife = add(&mut ctx, "stone_knife", 1);
        let before = snapshot(&ctx);
        // Consuming two branches leaves their stack occupying the output slot.
        assert!(execute(&mut ctx, actor, "carve_bowl", &[]).is_err());
        assert_eq!(snapshot(&ctx), before);
        ctx.map.entities[0].inventory.push(None);
        let before = snapshot(&ctx);
        // Action costs and recipe costs cannot each spend the same branches.
        assert!(execute(&mut ctx, actor, "carve_bowl", &[("wood_branch".into(), 2)]).is_err());
        assert_eq!(snapshot(&ctx), before);
        ctx.map.entities[0]
            .get_item_mut(knife)
            .unwrap()
            .set_attribute("condition", Value::UInt(0));
        let before = snapshot(&ctx);
        assert!(execute(&mut ctx, actor, "carve_bowl", &[]).is_err());
        assert_eq!(snapshot(&ctx), before);
        ctx.map.entities[0]
            .get_item_mut(knife)
            .unwrap()
            .set_attribute("condition", Value::UInt(100));
        execute(&mut ctx, actor, "carve_bowl", &[]).unwrap();
        assert_eq!(
            item_number(ctx.map.entities[0].get_item(knife).unwrap(), "condition", 0),
            99
        );
        assert_eq!(
            entity_inventory_item_count(&ctx.map.entities[0], "wooden_bowl"),
            1
        );
    }
    #[test]
    fn liquid_shortage_preserves_every_vessel_and_material_and_nonstack_outputs_are_distinct() {
        let (mut ctx, actor) = context(8);
        add(&mut ctx, "raw_bone", 1);
        let vessel = add(&mut ctx, "wooden_bowl", 1);
        add(&mut ctx, "stone_knife", 1);
        let before = snapshot(&ctx);
        assert!(execute(&mut ctx, actor, "shape_bone", &[]).is_err());
        assert_eq!(snapshot(&ctx), before);
        let bowl = ctx.map.entities[0].get_item_mut(vessel).unwrap();
        bowl.set_attribute("liquid_item", Value::Str("fresh_water".into()));
        bowl.set_attribute("liquid_quantity", Value::Int(1));
        execute(&mut ctx, actor, "shape_bone", &[]).unwrap();
        let mut recipe = ruleset_recipe_table(&ctx, "shape_bone").unwrap().clone();
        recipe.insert("consumes".into(), toml::Value::Array(vec![]));
        recipe.insert(
            "produces".into(),
            "values=[{item='stone_knife',quantity=2}]"
                .parse::<toml::Table>()
                .unwrap()
                .remove("values")
                .unwrap(),
        );
        ctx.rules["recipes"]
            .as_table_mut()
            .unwrap()
            .insert("test_multiple_tools".into(), toml::Value::Table(recipe));
        execute(&mut ctx, actor, "test_multiple_tools", &[]).unwrap();
        let tools: Vec<_> = ctx.map.entities[0]
            .iter_inventory()
            .map(|(_, i)| i)
            .filter(|i| ruleset_item_matches_id(i, "stone_knife"))
            .collect();
        assert_eq!(tools.len(), 3);
        assert!(
            tools
                .iter()
                .all(|item| item.attributes.get_str("appearance_icon")
                    == Some("icon/stone_knife/on"))
        );
        assert_eq!(tools.iter().map(|i| i.id).collect::<BTreeSet<_>>().len(), 3);
    }
    #[test]
    fn crafting_action_resource_costs_are_staged_with_recipe_inputs() {
        let (mut ctx, actor) = context(2);
        add(&mut ctx, "wood_branch", 3);
        let knife = add(&mut ctx, "stone_knife", 1);
        ctx.map.entities[0].set_attribute("MP", Value::Int(10));
        ctx.rules["actions"]["craft_carve_bowl"]
            .as_table_mut()
            .unwrap()
            .insert("cost".into(), toml::Value::Table("MP=2".parse().unwrap()));
        let action = eldiron_ruleset::resolve_action(&ctx.rules, "craft_carve_bowl")
            .unwrap()
            .unwrap();
        let before = snapshot(&ctx);
        assert!(execute_action(&mut ctx, actor, "carve_bowl", &[], &action, None).is_err());
        assert_eq!(snapshot(&ctx), before);
        ctx.map.entities[0].inventory.push(None);
        execute_action(&mut ctx, actor, "carve_bowl", &[], &action, None).unwrap();
        assert_eq!(ctx.map.entities[0].attributes.get_int("MP"), Some(8));
        assert_eq!(
            item_number(ctx.map.entities[0].get_item(knife).unwrap(), "condition", 0),
            99
        );
    }

    #[test]
    fn enchanted_variant_armor_strength_ui_and_combat_agree_through_break_and_repair() {
        let (mut ctx, actor, target, _) = repair_fixture(1);
        let (sender, _receiver) = crossbeam_channel::unbounded();
        ctx.from_sender.set(sender).unwrap();
        let mut variant =
            ruleset_item_from_table(&ctx.rules, "strength_bone_cuirass", 1, &ctx.world).unwrap();
        variant.id = target;
        variant.set_attribute("condition", Value::Int(1));
        variant.set_attribute("quality", Value::Int(73));
        *ctx.map.entities[0].get_item_mut(target).unwrap() = variant;
        ctx.map.entities[0].set_attribute("ARMOR", Value::Int(5));
        ctx.map.entities[0].set_attribute("STR", Value::Int(12));
        ctx.map.entities[0].set_attribute("HP", Value::Int(100));
        ctx.map.entities[0]
            .equip_item_unchecked(target, "torso")
            .unwrap();
        let identities = crate::effective_values::enchantments(
            ctx.map.entities[0].get_equipped_item("torso").unwrap(),
        )
        .unwrap();
        assert_eq!(
            effective_entity_attribute(&ctx, &ctx.map.entities[0], "STR", 0.),
            14.
        );
        assert_eq!(
            effective_entity_attribute(&ctx, &ctx.map.entities[0], "ARMOR", 0.),
            9.
        );
        let guarded = ctx.resolved_condition("guarded").unwrap().unwrap();
        assert!(apply_condition(&mut ctx, actor, actor, &guarded));
        assert_eq!(
            effective_entity_attribute(&ctx, &ctx.map.entities[0], "ARMOR", 0.),
            11.
        );
        assert_eq!(
            apply_damage_rules(&ctx, actor, 0, 20, "physical", 0),
            9,
            "equipment is not subtracted twice"
        );
        let mut assets = Assets::new();
        assets.rules = toml::to_string(&ctx.rules).unwrap();
        let rules = assets.value_rules().unwrap();
        assert_eq!(
            rules
                .explain(&ctx.map.entities[0], "ARMOR", 0.)
                .unwrap()
                .value,
            11.
        );
        let rendered = crate::message_text::MsgResolver::new().resolve(
            crate::MsgParser::new().parse(&format!(
                "{{E:{actor}.STR}} / {{E:{actor}.ARMOR}} / {{E:{actor}.explain.STR}}"
            )),
            &ctx.map,
            &assets,
        );
        assert!(rendered.starts_with("14 / 11 /"), "{rendered}");
        assert!(rendered.contains("Bear's Vigor +2"), "{rendered}");
        #[cfg(feature = "graphics")]
        {
            let description = crate::client::rules_ui::describe_item(
                ctx.map.entities[0].get_equipped_item("torso").unwrap(),
                &assets,
            );
            assert!(
                description
                    .lines
                    .iter()
                    .any(|line| line.contains("STR +2") && line.contains("while equipped"))
            );
        }
        assert!(
            RegionInstance::text_command_ruleset_stats(&ctx, &ctx.map.entities[0])
                .contains("Bear's Vigor +2")
        );
        assert!(apply_damage_direct(&mut ctx, actor, 0, 1, "physical", None));
        assert_eq!(
            effective_entity_attribute(&ctx, &ctx.map.entities[0], "STR", 0.),
            12.
        );
        #[cfg(feature = "graphics")]
        {
            let description = crate::client::rules_ui::describe_item(
                ctx.map.entities[0].get_equipped_item("torso").unwrap(),
                &assets,
            );
            assert!(
                description
                    .lines
                    .iter()
                    .any(|line| line.contains("Armor") && line.contains("inactive: broken"))
            );
            assert!(
                description
                    .lines
                    .iter()
                    .any(|line| line.contains("STR +2") && line.contains("inactive: broken"))
            );
        }
        assert_eq!(
            effective_entity_attribute(&ctx, &ctx.map.entities[0], "ARMOR", 0.),
            7.
        );
        execute_targeted(&mut ctx, actor, "repair_bone_armor", target).unwrap();
        let item = ctx.map.entities[0].get_equipped_item("torso").unwrap();
        assert_eq!(
            crate::effective_values::enchantments(item).unwrap(),
            identities
        );
        assert_eq!(item.attributes.get_int("quality"), Some(73));
        assert_eq!(
            effective_entity_attribute(&ctx, &ctx.map.entities[0], "STR", 0.),
            14.
        );
        assert_eq!(ctx.map.entities[0].attributes.get_int("STR"), Some(12));
        assert_eq!(ctx.map.entities[0].attributes.get_int("ARMOR"), Some(5));
    }

    #[test]
    fn invalid_enchantment_state_and_overflow_reject_equip_without_moving_or_charging_items() {
        let (mut ctx, actor) = context(4);
        let target = add(&mut ctx, "strength_bone_cuirass", 1);
        ctx.map.entities[0]
            .get_item_mut(target)
            .unwrap()
            .set_attribute(
                "item_enchantment_instances",
                Value::Str("invalid json".into()),
            );
        let before = snapshot(&ctx);
        assert!(equip_inventory_item_for_entity(&mut ctx, actor, target, "torso").is_err());
        assert_eq!(snapshot(&ctx), before);
        ctx.map.entities[0]
            .get_item_mut(target)
            .unwrap()
            .attributes
            .remove("item_enchantment_instances");
        ctx.map.entities[0].set_attribute("STR", Value::Int(12));
        let mut rules = ctx.rules.clone();
        rules["enchantments"]["strength"]["modifiers"][0]["add"] =
            toml::Value::Float(f32::MAX as f64);
        rules["enchantments"]["strength"]["modifiers"][0]
            .as_table_mut()
            .unwrap()
            .insert("multiply".into(), toml::Value::Float(f32::MAX as f64));
        ctx.set_rules(rules).unwrap();
        let before = snapshot(&ctx);
        assert!(equip_inventory_item_for_entity(&mut ctx, actor, target, "torso").is_err());
        assert_eq!(snapshot(&ctx), before);
    }

    #[test]
    fn projected_focus_power_is_not_implicitly_added_again_as_a_source_bonus() {
        let (mut ctx, actor) = context(4);
        let focus = add(&mut ctx, "ritual_censer", 1);
        let slot = ctx.map.entities[0]
            .get_item(focus)
            .unwrap()
            .attributes
            .get_str("slot")
            .unwrap()
            .to_string();
        ctx.map.entities[0]
            .equip_item_unchecked(focus, &slot)
            .unwrap();
        let source = ctx.map.entities[0].get_equipped_item(&slot).unwrap();
        let power = source.attributes.get_float_default("POWER", 0.);
        assert!(power > 0.);
        let kind: toml::Table =
            "[damage]\nsource_bonus_attribute='POWER'\nattacker_bonus_attribute='POWER'"
                .parse()
                .unwrap();
        assert_eq!(
            apply_structured_combat_table(
                &ctx,
                10.,
                "outgoing_damage",
                Some(&ctx.map.entities[0]),
                None,
                Some(source),
                &kind
            ),
            Some(10. + power)
        );
        // A carried instance of the same template is a different source. Its
        // explicit action-local bonus must not be mistaken for the worn item.
        let mut carried = source.clone();
        carried.id = ctx.world.next_id();
        ctx.map.entities[0].add_item(carried.clone()).unwrap();
        assert_eq!(
            apply_structured_combat_table(
                &ctx,
                10.,
                "outgoing_damage",
                Some(&ctx.map.entities[0]),
                None,
                Some(&carried),
                &kind
            ),
            Some(10. + power * 2.)
        );
    }
}
