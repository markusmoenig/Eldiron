//! Immediate learning stages costs and acquisition on one learner copy.
use super::*;
use crate::learning::{self, Origin};
use std::collections::BTreeMap;

/// A single-lesson owned manual can be studied through ordinary Use. Authored
/// item behavior and explicit on_use handlers keep precedence.
pub(super) fn use_book(ctx: &mut RegionCtx, actor: u32, item: u32, owner: Option<u32>) -> bool {
    if owner.is_some_and(|owner| owner != actor) {
        return false;
    }
    let Some(book) = ctx
        .map
        .entities
        .iter()
        .find(|e| e.id == actor)
        .and_then(|e| {
            e.iter_inventory()
                .map(|(_, i)| i)
                .chain(e.equipped.values())
                .find(|i| i.id == item)
        })
    else {
        return false;
    };
    if book
        .attributes
        .get_str("on_use")
        .is_some_and(|s| !s.trim().is_empty())
        || ctx
            .assets
            .node_behaviors
            .as_ref()
            .is_some_and(|assets| assets.owns_item_intent(ctx, book))
    {
        return false;
    }
    let Some(Value::StrArray(lessons)) = book.attributes.get("teaching") else {
        return false;
    };
    let [lesson] = lessons.as_slice() else {
        return false;
    };
    let Some(action) = ctx
        .rules
        .get("actions")
        .and_then(toml::Value::as_table)
        .and_then(|actions| {
            actions.iter().find_map(|(id, action)| {
                (action.get("kind").and_then(toml::Value::as_str) == Some("learn")
                    && action.get("lesson").and_then(toml::Value::as_str) == Some(lesson)
                    && action.get("target").and_then(toml::Value::as_str) == Some("inventory_item"))
                .then(|| id.clone())
            })
        })
    else {
        return false;
    };
    execute_ruleset_action_with_target(
        ctx,
        actor,
        &action,
        Some(RulesetActionTarget::Item {
            item_id: item,
            owner_entity_id: Some(actor),
        }),
    );
    true
}

pub(super) fn execute(
    ctx: &mut RegionCtx,
    actor_id: u32,
    action: &ResolvedAction,
    target: Option<&RulesetActionTarget>,
) -> Result<bool, String> {
    let lesson_id = action.lesson.as_deref().ok_or("Missing lesson")?;
    let rules = ctx.with_value_rules(Clone::clone)?;
    let lesson = rules
        .learning
        .teaching
        .get(lesson_id)
        .ok_or("Unknown lesson")?;
    let index = ctx
        .map
        .entities
        .iter()
        .position(|e| e.id == actor_id)
        .ok_or("Learner is no longer present")?;
    let mut actor = ctx.map.entities[index].clone();
    if ctx.paused || actor.get_mode() == "dead" {
        return Err("Learner cannot study now".into());
    }
    if !resolved_runtime_action_target_allowed(ctx, &action.target, actor_id, target)
        || !resolved_action_target_is_in_range(ctx, &actor, action, target)
    {
        return Err("Choose an accessible teacher or an owned book nearby".into());
    }
    let origin = match target {
        Some(RulesetActionTarget::Entity(id)) if *id != actor_id => {
            let teacher = ctx
                .map
                .entities
                .iter()
                .find(|e| e.id == *id)
                .ok_or("Teacher is no longer present")?;
            if teacher.get_mode() == "dead"
                || !teacher.attributes.get_bool_default("visible", true)
                || entity_is_hostile_by_id(ctx, actor_id, *id)
            {
                return Err("Teacher is unavailable".into());
            }
            let explicit = matches!(teacher.attributes.get("teaching"),Some(Value::StrArray(ids)) if ids.iter().any(|id|id==lesson_id));
            let profession = teacher
                .attributes
                .get_str("profession")
                .and_then(|id| ctx.rules.get("professions").and_then(|v| v.get(id)))
                .and_then(|v| v.get("teaching"))
                .and_then(toml::Value::as_array)
                .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(lesson_id)));
            if !explicit && !profession {
                return Err("This teacher does not offer that lesson".into());
            }
            Origin::Teacher {
                lesson: lesson_id.into(),
                entity: *id,
                creator: teacher.creator_id,
            }
        }
        Some(RulesetActionTarget::Item {
            item_id,
            owner_entity_id: Some(owner),
        }) if *owner == actor_id => {
            let book = actor
                .iter_inventory()
                .map(|(_, i)| i)
                .chain(actor.equipped.values())
                .find(|i| i.id == *item_id)
                .ok_or("Study the exact book you own")?;
            if !matches!(book.attributes.get("teaching"),Some(Value::StrArray(ids)) if ids.iter().any(|id|id==lesson_id))
                || crafting::item_number(book, "condition", 100) <= 0
            {
                return Err("This book cannot teach that lesson".into());
            }
            Origin::Book {
                lesson: lesson_id.into(),
                item: *item_id,
                creator: book.creator_id,
            }
        }
        _ => return Err("Choose a teacher or the exact book you own".into()),
    };
    let knowledge = lesson.knowledge.iter().cloned().collect::<Vec<_>>();
    // Temporary access does not count as permanent acquisition.
    if knowledge
        .iter()
        .all(|id| learning::has_permanent(&actor, id).unwrap_or(false))
    {
        return Ok(false);
    }
    if let Some(skill) = &lesson.skill {
        if entity_skill_points(&actor, skill) < lesson.required_skill as i32 {
            return Err(format!("Need {} {}", skill, lesson.required_skill));
        }
    }
    let debits =
        prepare_action_resource_costs(&actor, action).ok_or("Insufficient lesson resources")?;
    let mut costs = BTreeMap::<String, usize>::new();
    for cost in &action.item_costs {
        let count = costs.entry(cost.item.clone()).or_default();
        *count = count
            .checked_add(cost.quantity)
            .filter(|count| *count <= 1_000_000)
            .ok_or("Lesson costs exceed bounds")?;
    }
    let costs = costs.into_iter().collect::<Vec<_>>();
    if entity_has_action_consumes(&actor, &costs).is_some() {
        return Err("Lesson materials are missing".into());
    }
    consume_entity_items(&mut actor, &costs);
    if let Origin::Book { item, .. } = &origin {
        if !actor
            .iter_inventory()
            .map(|(_, i)| i)
            .chain(actor.equipped.values())
            .any(|i| i.id == *item)
        {
            return Err("The lesson cannot consume its source book".into());
        }
    }
    learning::acquire(&mut actor, &rules, &knowledge, origin)?;
    for (key, value) in debits {
        actor.set_attribute(&key, value);
    }
    ctx.map.entities[index] = actor;
    set_action_cooldown(ctx, actor_id, &action.id, action.cooldown_seconds);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> (RegionCtx, u32, u32, Receiver<RegionMessage>) {
        let mut ctx = RegionCtx::default();
        let mut root = eldiron_ruleset::document::RulesState::default()
            .document()
            .unwrap()
            .table()
            .clone();
        let action = root["actions"]["learn_boneworking"].as_table_mut().unwrap();
        action.insert("cost".into(), toml::Value::Table("MP=2".parse().unwrap()));
        action.insert(
            "consumes".into(),
            toml::Value::Array(vec![toml::Value::Table(
                "item='bone_plate'\nquantity=1".parse().unwrap(),
            )]),
        );
        ctx.set_rules(root).unwrap();
        let (sender, receiver) = unbounded();
        ctx.from_sender.set(sender).unwrap();
        let mut student = Entity::new();
        student.id = ctx.world.next_id();
        let id = student.id;
        student.inventory = vec![None; 10];
        student.position = Vec3::zero();
        for (key, value) in [
            ("name", Value::Str("Student".into())),
            ("HP", Value::Int(100)),
            ("MP", Value::Int(10)),
            ("skill_boneworking", Value::Int(10)),
        ] {
            student.set_attribute(key, value);
        }
        let mut teacher = Entity::new();
        teacher.id = ctx.world.next_id();
        let npc = teacher.id;
        teacher.position = Vec3::zero();
        teacher.set_attribute("name", Value::Str("Tutor".into()));
        teacher.set_attribute("profession", Value::Str("Boneworker".into()));
        ctx.map.entities.extend([student, teacher]);
        for (item, quantity) in [
            ("bone_plate", 1),
            ("raw_bone", 3),
            ("stone_knife", 1),
            ("wooden_bowl", 1),
        ] {
            let mut item = ruleset_item_from_table(&ctx.rules, item, quantity, &ctx.world).unwrap();
            if item.attributes.get_str("ruleset_id") == Some("wooden_bowl") {
                item.set_attribute("liquid_item", Value::Str("fresh_water".into()));
                item.set_attribute("liquid_quantity", Value::Int(4));
            }
            ctx.map.entities[0].add_item(item).unwrap();
        }
        let mut bench =
            ruleset_item_from_table(&ctx.rules, "bone_workbench", 1, &ctx.world).unwrap();
        bench.position = Vec3::zero();
        ctx.map.items.push(bench);
        (ctx, id, npc, receiver)
    }
    fn snapshot(ctx: &RegionCtx) -> serde_json::Value {
        serde_json::to_value(&ctx.map.entities[0]).unwrap()
    }
    #[test]
    fn learning_actions_teacher_and_node_pay_once_then_unlock_real_batch_crafting() {
        let (mut ctx, student, teacher, _messages) = context();
        let before = snapshot(&ctx);
        assert!(crafting::execute(&mut ctx, student, "shape_bone_batch", &[]).is_err());
        assert_eq!(snapshot(&ctx), before);
        // A real NPC graph invokes the same action with event.subject as learner.
        let template = Uuid::new_v4();
        let output = Uuid::new_v4();
        let input = Uuid::new_v4();
        let graph = serde_json::json!({"version":1,"nodes":[
            {"id":Uuid::new_v4(),"definition":"event","rows":[{"key":"event","value":{"Custom":{"kind":"event","data":"intent"}}}],"ports":[{"id":output,"key":"out","direction":"Output","kind":"flow"}]},
            {"id":Uuid::new_v4(),"definition":"teach","rows":[{"key":"action","value":{"Choice":{"options":["learn_boneworking"],"selected":0}}}],"ports":[{"id":input,"key":"in","direction":"Input","kind":"flow"}]}
        ],"connections":[{"id":Uuid::new_v4(),"from":output,"to":input}]});
        ctx.map.entities[1].set_attribute("class_name", Value::Str("Tutor".into()));
        ctx.assets.node_behaviors = Some(crate::server::nodes::region::BehaviorAssets {
            graphs: std::collections::HashMap::from([(
                format!("behavior/character/{template}"),
                graph,
            )]),
            characters: std::collections::HashMap::from([("Tutor".into(), template)]),
            ..Default::default()
        });
        ctx.curr_entity_id = teacher;
        ctx.current_script_scope = crate::server::regionctx::ScriptScope::Entity;
        let program = crate::vm::VM::default()
            .prepare_str("fn event(event, value) { }")
            .unwrap();
        crate::server::region_host::run_server_named_fn(
            &mut crate::vm::Execution::default(),
            "event",
            &[
                VMValue::from_string("intent"),
                VMValue::new_with_string(student as f32, 1., 0., "use"),
            ],
            &program,
            &mut ctx,
        );
        let acquired = learning::journal(&ctx.map.entities[0]).unwrap();
        assert_eq!(acquired.acquisitions.len(), 1);
        assert!(
            matches!(acquired.acquisitions[0].origin,Origin::Teacher{entity,..} if entity==teacher)
        );
        assert_eq!(ctx.map.entities[0].attributes.get_int("MP"), Some(8));
        assert_eq!(
            entity_inventory_item_count(&ctx.map.entities[0], "bone_plate"),
            0
        );
        let before = snapshot(&ctx);
        assert!(execute_ruleset_action_with_target(
            &mut ctx,
            student,
            "learn_boneworking",
            Some(RulesetActionTarget::Entity(teacher))
        ));
        assert_eq!(snapshot(&ctx), before);
        crafting::execute(&mut ctx, student, "shape_bone_batch", &[]).unwrap();
        assert_eq!(
            entity_inventory_item_count(&ctx.map.entities[0], "bone_plate"),
            3
        );
        assert_eq!(
            entity_inventory_item_count(&ctx.map.entities[0], "raw_bone"),
            0
        );
        assert!(entity_skill_points(&ctx.map.entities[0], "boneworking") > 10);
        let knife = ctx.map.entities[0]
            .iter_inventory()
            .map(|(_, i)| i)
            .find(|i| i.attributes.get_str("ruleset_id") == Some("stone_knife"))
            .unwrap();
        assert_eq!(crafting::item_number(knife, "condition", 0), 99);
        assert_eq!(learning::journal(&ctx.map.entities[0]).unwrap(), acquired);
        assert!(ctx.map.entities[0].attributes.get("class").is_none());
    }
    #[test]
    fn learning_actions_invalid_teacher_skill_costs_and_foreign_books_leave_the_learner_unchanged()
    {
        for failure in ["range", "teacher", "skill", "cost", "materials", "paused"] {
            let (mut ctx, student, teacher, _messages) = context();
            match failure {
                "range" => ctx.map.entities[1].position.x = 10.,
                "teacher" => {
                    ctx.map.entities[1].attributes.remove("profession");
                }
                "skill" => ctx.map.entities[0].set_attribute("skill_boneworking", Value::Int(0)),
                "cost" => ctx.map.entities[0].set_attribute("MP", Value::Int(1)),
                "materials" => {
                    ctx.map.entities[0].remove_item_from_slot(0);
                }
                _ => ctx.paused = true,
            }
            let before = snapshot(&ctx);
            assert!(
                !execute_ruleset_action_with_target(
                    &mut ctx,
                    student,
                    "learn_boneworking",
                    Some(RulesetActionTarget::Entity(teacher))
                ),
                "{failure}"
            );
            assert_eq!(snapshot(&ctx), before, "{failure}");
        }
        let (mut ctx, student, teacher, _messages) = context();
        let book =
            ruleset_item_from_table(&ctx.rules, "boneworking_manual", 1, &ctx.world).unwrap();
        let id = book.id;
        ctx.map.entities[0].add_item(book).unwrap();
        let before = snapshot(&ctx);
        assert!(!execute_ruleset_action_with_target(
            &mut ctx,
            student,
            "study_boneworking",
            Some(RulesetActionTarget::Item {
                item_id: id,
                owner_entity_id: Some(teacher)
            })
        ));
        assert_eq!(snapshot(&ctx), before);
        let values = ctx.with_value_rules(Clone::clone).unwrap();
        let mut guide = ruleset_item_from_table(&ctx.rules, "bone_helm", 1, &ctx.world).unwrap();
        crate::effective_values::attach(&mut guide, &values, "boneworking_guidance").unwrap();
        ctx.map.entities[0].equipped.insert("head".into(), guide);
        assert!(
            learning::has_knowledge(&values, &ctx.map.entities[0], "boneworking_patterns").unwrap()
        );
        let guide = ctx.map.entities[0].equipped.shift_remove("head").unwrap();
        let before = snapshot(&ctx);
        assert!(crafting::execute(&mut ctx, student, "shape_bone_batch", &[]).is_err());
        assert_eq!(
            snapshot(&ctx),
            before,
            "losing a temporary grant before commit must spend no inputs"
        );
        ctx.map.entities[0].equipped.insert("head".into(), guide);
        assert!(execute_ruleset_action_with_target(
            &mut ctx,
            student,
            "study_boneworking",
            Some(RulesetActionTarget::Item {
                item_id: id,
                owner_entity_id: Some(student)
            })
        ));
        assert!(
            matches!(learning::journal(&ctx.map.entities[0]).unwrap().acquisitions[0].origin,Origin::Book{item,..} if item==id)
        );
        assert!(ctx.map.entities[0].get_item(id).is_some());
        ctx.map.entities[0].equipped.clear();
        assert!(learning::has_permanent(&ctx.map.entities[0], "boneworking_patterns").unwrap());
    }
}
