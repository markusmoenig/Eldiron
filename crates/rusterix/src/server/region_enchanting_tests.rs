use super::*;
use crate::{effective_values, learning};

#[test]
fn journal_owned_target_preview_matches_enchanting_and_repair_without_leaking_attachment_facts() {
    let (mut ctx, actor, item, _tool, _messages) = fixture();
    let before = snapshot(&ctx);
    let high = ctx.world.high_water();
    let mut inspector = preview::RecipeInspector::new(ctx.rules.clone()).unwrap();
    inspector.bind(&ctx.map.entities[0], &ctx.map.items, false);
    assert!(!inspector.inspect("inscribe_bear_cuirass", None).ready);
    let result = inspector.inspect("inscribe_bear_cuirass", Some(item));
    assert!(result.ready, "{:?}", result.reason);
    assert!(
        result
            .results
            .iter()
            .any(|line| line.contains("Bone Cuirass of the Bear"))
    );
    assert!(result.results.iter().any(|line| line.contains("STR: +2")));
    assert_eq!(snapshot(&ctx), before);
    assert_eq!(ctx.world.high_water(), high);
    assert!(
        effective_values::enchantments(target(&ctx, item))
            .unwrap()
            .is_empty()
    );
    assert!(perform(&mut ctx, actor, item, actor));
    inspector.bind(&ctx.map.entities[0], &ctx.map.items, false);
    assert!(!inspector.inspect("inscribe_bear_cuirass", Some(item)).ready);
    add(&mut ctx, "bone_plate", 1);
    add(&mut ctx, "leather_strap", 1);
    inspector.bind(&ctx.map.entities[0], &ctx.map.items, false);
    let result = inspector.inspect("repair_bone_armor", Some(item));
    assert!(result.ready);
    assert!(result.results.iter().any(|line| line.contains("63 → 83")));
}

fn snapshot(ctx: &RegionCtx) -> serde_json::Value {
    serde_json::to_value(&ctx.map.entities[0]).unwrap()
}
fn add(ctx: &mut RegionCtx, id: &str, quantity: usize) -> u32 {
    let item = ruleset_item_from_table(&ctx.rules, id, quantity, &ctx.world).unwrap();
    let id = item.id;
    ctx.map.entities[0].add_item(item).unwrap();
    id
}
fn fixture() -> (RegionCtx, u32, u32, u32, Receiver<RegionMessage>) {
    let mut ctx = RegionCtx::default();
    let mut root = eldiron_ruleset::document::RulesState::default()
        .document()
        .unwrap()
        .table()
        .clone();
    // Exercise ordinary tuition/resource-cost staging without requiring mana in
    // the supplied world-only inscription chain.
    root["actions"]["inscribe_bear_cuirass"]
        .as_table_mut()
        .unwrap()
        .insert("cost".into(), toml::Value::Table("MP=2".parse().unwrap()));
    ctx.set_rules(root).unwrap();
    let (sender, receiver) = unbounded();
    ctx.from_sender.set(sender).unwrap();
    let mut actor = Entity::new();
    actor.id = ctx.world.next_id();
    let id = actor.id;
    actor.inventory = vec![None; 10];
    for (key, value) in [
        ("HP", 100),
        ("MP", 5),
        ("STR", 12),
        ("skill_boneworking", 10),
    ] {
        actor.set_attribute(key, Value::Int(value));
    }
    let rules = ctx.with_value_rules(Clone::clone).unwrap();
    learning::acquire(
        &mut actor,
        &rules,
        &["bear_inscription".into()],
        learning::Origin::Character,
    )
    .unwrap();
    ctx.map.entities.push(actor);
    add(&mut ctx, "bear_inlay", 2);
    add(&mut ctx, "tree_resin", 2);
    let vessel = add(&mut ctx, "wooden_bowl", 1);
    let bowl = ctx.map.entities[0].get_item_mut(vessel).unwrap();
    bowl.set_attribute("liquid_item", Value::Str("fresh_water".into()));
    bowl.set_attribute("liquid_quantity", Value::Int(2));
    let tool = add(&mut ctx, "stone_knife", 1);
    let target = add(&mut ctx, "bone_cuirass", 1);
    let item = ctx.map.entities[0].get_item_mut(target).unwrap();
    item.set_attribute("condition", Value::Int(63));
    item.set_attribute("quality", Value::Int(73));
    item.set_attribute("future_property", Value::Str("keep me".into()));
    let mut station = ruleset_item_from_table(&ctx.rules, "bone_workbench", 1, &ctx.world).unwrap();
    station.position = Vec3::zero();
    ctx.map.items.push(station);
    (ctx, id, target, tool, receiver)
}
fn perform(ctx: &mut RegionCtx, actor: u32, target: u32, owner: u32) -> bool {
    execute_ruleset_action_with_target(
        ctx,
        actor,
        "inscribe_bear_cuirass",
        Some(RulesetActionTarget::Item {
            item_id: target,
            owner_entity_id: Some(owner),
        }),
    )
}
fn target(ctx: &RegionCtx, id: u32) -> &Item {
    owned(&ctx.map.entities[0]).find(|i| i.id == id).unwrap()
}

#[test]
fn enchanting_commits_one_attachment_and_costs_on_the_exact_owned_piece_and_preserves_repair() {
    for equipped in [false, true] {
        let (mut ctx, actor, item, tool, _messages) = fixture();
        let other = add(&mut ctx, "bone_cuirass", 1);
        let other_before = serde_json::to_value(target(&ctx, other)).unwrap();
        let before = target(&ctx, item).clone();
        if equipped {
            ctx.map.entities[0]
                .equip_item_unchecked(item, "torso")
                .unwrap();
        }
        assert!(perform(&mut ctx, actor, item, actor));
        let after = target(&ctx, item);
        assert_eq!((after.id, after.creator_id), (before.id, before.creator_id));
        assert_eq!(after.attributes.get_str("ruleset_id"), Some("bone_cuirass"));
        assert_eq!(
            after.attributes.get_str("name"),
            Some("Bone Cuirass of the Bear")
        );
        assert_eq!(item_number(after, "quality", 0), 73);
        assert_eq!(item_number(after, "condition", 0), 63);
        assert_eq!(after.attributes.get_str("future_property"), Some("keep me"));
        let attachments = effective_values::enchantments(after).unwrap();
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].definition, "strength");
        assert!(Uuid::parse_str(&attachments[0].id).is_ok());
        assert_eq!(
            serde_json::to_value(target(&ctx, other)).unwrap(),
            other_before
        );
        assert_eq!(ctx.map.entities[0].attributes.get_int("MP"), Some(3));
        assert_eq!(ctx.map.entities[0].attributes.get_int("STR"), Some(12));
        assert_eq!(item_number(target(&ctx, tool), "condition", 0), 98);
        assert_eq!(
            entity_inventory_item_count(&ctx.map.entities[0], "bear_inlay"),
            1
        );
        assert_eq!(
            ctx.with_value_rules(|r| r.explain(&ctx.map.entities[0], "STR", 0.).unwrap().value)
                .unwrap(),
            if equipped { 14. } else { 12. }
        );
        let committed = snapshot(&ctx);
        assert!(!perform(&mut ctx, actor, item, actor));
        assert_eq!(
            snapshot(&ctx),
            committed,
            "repeat inscription cannot charge or stack twice"
        );
        if !equipped {
            ctx.map.entities[0]
                .equip_item_unchecked(item, "torso")
                .unwrap();
        }
        ctx.map.entities[0]
            .get_equipped_item_mut("torso")
            .unwrap()
            .set_attribute("condition", Value::Int(0));
        assert_eq!(
            effective_entity_attribute(&ctx, &ctx.map.entities[0], "STR", 0.),
            12.
        );
        add(&mut ctx, "bone_plate", 1);
        add(&mut ctx, "leather_strap", 1);
        execute_targeted(&mut ctx, actor, "repair_bone_armor", item).unwrap();
        assert_eq!(
            effective_values::enchantments(target(&ctx, item)).unwrap(),
            attachments
        );
        assert_eq!(item_number(target(&ctx, item), "condition", 0), 20);
        assert_eq!(
            effective_entity_attribute(&ctx, &ctx.map.entities[0], "STR", 0.),
            14.
        );
    }
}

#[test]
fn enchanting_rejections_leave_inputs_target_wear_resources_and_training_unchanged() {
    for failure in [
        "knowledge",
        "skill",
        "materials",
        "water",
        "tool",
        "station",
        "foreign",
        "wrong_item",
        "broken",
        "malformed",
        "overflow",
        "paused",
        "cost",
        "target_cost",
        "no_target",
    ] {
        let (mut ctx, actor, mut item, tool, _messages) = fixture();
        match failure {
            "knowledge" => {
                ctx.map.entities[0].attributes.remove(learning::JOURNAL);
            }
            "skill" => ctx.map.entities[0].set_attribute("skill_boneworking", Value::Int(0)),
            "materials" => {
                let slot = ctx.map.entities[0].get_item_slot(tool).unwrap();
                ctx.map.entities[0].remove_item_from_slot(0);
                assert!(slot > 0);
            }
            "water" => {
                let bowl = owned(&ctx.map.entities[0])
                    .find(|i| i.attributes.get_str("ruleset_id") == Some("wooden_bowl"))
                    .unwrap()
                    .id;
                ctx.map.entities[0]
                    .get_item_mut(bowl)
                    .unwrap()
                    .set_attribute("liquid_quantity", Value::Int(0));
            }
            "tool" => ctx.map.entities[0]
                .get_item_mut(tool)
                .unwrap()
                .set_attribute("condition", Value::Int(1)),
            "station" => ctx.map.items[0].position.x = 10.,
            "wrong_item" => item = add(&mut ctx, "raw_bone", 1),
            "broken" => ctx.map.entities[0]
                .get_item_mut(item)
                .unwrap()
                .set_attribute("condition", Value::Int(0)),
            "malformed" => ctx.map.entities[0]
                .get_item_mut(item)
                .unwrap()
                .set_attribute("item_enchantment_instances", Value::Str("invalid".into())),
            "overflow" => {
                let mut rules = ctx.rules.clone();
                rules["enchantments"]["strength"]["modifiers"][0]["add"] =
                    toml::Value::Float(f32::MAX as f64);
                ctx.set_rules(rules).unwrap();
                ctx.map.entities[0].set_attribute("STR", Value::Float(f32::MAX));
                ctx.map.entities[0]
                    .equip_item_unchecked(item, "torso")
                    .unwrap();
            }
            "paused" => ctx.paused = true,
            "cost" => ctx.map.entities[0].set_attribute("MP", Value::Int(1)),
            "target_cost" => {
                let mut rules = ctx.rules.clone();
                rules["actions"]["inscribe_bear_cuirass"]
                    .as_table_mut()
                    .unwrap()
                    .insert(
                        "consumes".into(),
                        toml::Value::Array(vec![toml::Value::Table(
                            "item='bone_cuirass'\nquantity=1".parse().unwrap(),
                        )]),
                    );
                ctx.set_rules(rules).unwrap();
            }
            _ => {}
        }
        let before = snapshot(&ctx);
        let result = if failure == "no_target" {
            execute(&mut ctx, actor, "inscribe_bear_cuirass", &[]).is_ok()
        } else {
            perform(
                &mut ctx,
                actor,
                item,
                if failure == "foreign" {
                    actor + 1000
                } else {
                    actor
                },
            )
        };
        assert!(!result, "{failure}");
        assert_eq!(snapshot(&ctx), before, "{failure}");
    }
}
