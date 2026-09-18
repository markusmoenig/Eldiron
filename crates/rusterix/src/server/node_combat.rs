//! Native behavior policies use the same target attributes and action executor as Eldrin.
use super::*;

pub(crate) fn node_lookout(
    ctx: &mut RegionCtx,
    actor_id: u32,
    profile: &str,
) -> Result<bool, String> {
    let policy = eldiron_ruleset::behavior::lookout(&ctx.rules, profile)?;
    let actor = ctx
        .map
        .entities
        .iter()
        .find(|e| e.id == actor_id)
        .ok_or("Character no longer exists")?;
    if matches!(actor.attributes.get("__node_lookout_retry"), Some(Value::Int64(tick)) if *tick > ctx.ticks)
    {
        return Ok(false);
    }
    if actor.attributes.get_bool_default("__node_engage", false) {
        return Ok(false);
    }
    let position = actor.get_pos_xz();
    let target = ctx
        .map
        .entities
        .iter()
        .filter(|target| {
            target.id != actor_id
                && target.get_mode() != "dead"
                && target.attributes.get_bool_default("visible", true)
                && position.distance(target.get_pos_xz()) <= policy.radius
                && entity_disposition(ctx, actor, target) == policy.disposition
        })
        .min_by(|a, b| {
            position
                .distance_squared(a.get_pos_xz())
                .total_cmp(&position.distance_squared(b.get_pos_xz()))
                .then(a.id.cmp(&b.id))
        })
        .map(|e| e.id);
    if let Some(target) = target {
        ctx.set_entity_target(actor_id, Some(target));
        let delay = RegionInstance::realtime_seconds_to_ticks(ctx, policy.retry_seconds);
        if let Some(actor) = ctx.map.entities.iter_mut().find(|e| e.id == actor_id) {
            actor
                .attributes
                .set("__node_lookout_delay", Value::Int64(delay));
        }
    }
    Ok(target.is_some())
}

pub(crate) fn node_engage_start(
    ctx: &mut RegionCtx,
    actor_id: u32,
    profile: &str,
) -> Result<(), String> {
    eldiron_ruleset::behavior::engage(&ctx.rules, profile)?;
    let target = ctx.entity_target(actor_id).ok_or("No current target")?;
    let identity = ctx
        .map
        .entities
        .iter()
        .find(|e| e.id == target)
        .ok_or("Target no longer exists")?
        .creator_id;
    ctx.set_entity_target(actor_id, Some(target));
    let actor = ctx
        .map
        .entities
        .iter_mut()
        .find(|e| e.id == actor_id)
        .unwrap();
    let position = actor.get_pos_xz();
    actor.attributes.set("__node_engage", Value::Bool(true));
    actor
        .attributes
        .set("__node_engage_target", Value::Str(identity.to_string()));
    for (key, value) in [
        ("__node_engage_origin_x", position.x),
        ("__node_engage_origin_z", position.y),
        ("__node_engage_last_x", position.x),
        ("__node_engage_last_z", position.y),
    ] {
        actor.attributes.set(key, Value::Float(value));
    }
    actor
        .attributes
        .set("__node_engage_progress", Value::Int64(ctx.ticks));
    actor.set_attribute("__node_behavior_status", Value::Str("closing_in".into()));
    actor.action = EntityAction::Off;
    Ok(())
}
fn finish(
    ctx: &mut RegionCtx,
    actor_id: u32,
    output: &'static str,
) -> Result<Option<&'static str>, String> {
    ctx.set_entity_target(actor_id, None);
    if let Some(actor) = ctx.map.entities.iter_mut().find(|e| e.id == actor_id) {
        let delay = match actor.attributes.get("__node_lookout_delay") {
            Some(Value::Int64(delay)) => *delay,
            _ => 0,
        };
        actor
            .attributes
            .set("__node_lookout_retry", Value::Int64(ctx.ticks + delay));
        actor.attributes.remove("__node_engage");
        actor.attributes.remove("__node_engage_target");
        actor.set_attribute("__node_behavior_status", Value::Str(output.into()));
        if matches!(actor.action, EntityAction::CloseIn(..)) {
            actor.action = EntityAction::Off;
        }
    }
    Ok(Some(output))
}

pub(crate) fn node_engage_tick(
    ctx: &mut RegionCtx,
    actor_id: u32,
    profile: &str,
) -> Result<Option<&'static str>, String> {
    let policy = eldiron_ruleset::behavior::engage(&ctx.rules, profile)?;
    let actor = ctx
        .map
        .entities
        .iter()
        .find(|e| e.id == actor_id)
        .cloned()
        .ok_or("Character no longer exists")?;
    if !actor.attributes.get_bool_default("__node_engage", false) {
        return finish(ctx, actor_id, "cannot_engage");
    }
    let Some(target_id) = ctx.entity_target(actor_id) else {
        return finish(ctx, actor_id, "lost");
    };
    let Some(target) = ctx.map.entities.iter().find(|e| e.id == target_id).cloned() else {
        return finish(ctx, actor_id, "lost");
    };
    if actor.attributes.get_str("__node_engage_target")
        != Some(target.creator_id.to_string().as_str())
    {
        return finish(ctx, actor_id, "lost");
    }
    if target.get_mode() == "dead" {
        return finish(ctx, actor_id, "defeated");
    }
    let position = actor.get_pos_xz();
    let origin = Vec2::new(
        actor
            .attributes
            .get_float_default("__node_engage_origin_x", position.x),
        actor
            .attributes
            .get_float_default("__node_engage_origin_z", position.y),
    );
    if !target.attributes.get_bool_default("visible", true)
        || origin.distance(target.get_pos_xz()) > policy.pursuit_distance
    {
        return finish(ctx, actor_id, "lost");
    }
    if !matches!(actor.action, EntityAction::Off | EntityAction::CloseIn(..)) {
        return finish(ctx, actor_id, "cannot_engage");
    }
    let mut selected = None;
    let mut cooling = false;
    for id in &policy.actions {
        let action = resolved_ruleset_action(ctx, id)?
            .ok_or_else(|| format!("Unknown engagement action '{id}'"))?;
        // Only native attack actions are eligible; script callbacks never bypass the rules.
        if action.kind != ResolvedActionKind::Attack
            || action.damage_source().is_none()
            || !resolved_action_target_allowed(ctx, &action.target, actor_id, target_id)
            || !entity_meets_action_attribute_requirements(ctx, &actor, &action)
            || !entity_meets_action_target_attribute_requirements(ctx, &target, &action)
            || action
                .required_ability()
                .is_some_and(|id| !entity_knows_ruleset_ability(ctx, &actor, id))
            || action
                .skill_requirement()
                .is_some_and(|(id, amount)| entity_skill_points(&actor, id) < amount)
            || prepare_action_resource_costs(&actor, &action).is_none()
        {
            continue;
        }
        let items = action
            .item_costs
            .iter()
            .map(|cost| (cost.item.clone(), cost.quantity))
            .collect::<Vec<_>>();
        if entity_has_action_consumes(&actor, &items).is_some() {
            continue;
        }
        if is_action_on_cooldown(ctx, actor_id, id) {
            cooling = true;
            continue;
        }
        selected = Some(action);
        break;
    }
    let Some(action) = selected else {
        if cooling {
            if let Some(actor) = ctx.map.entities.iter_mut().find(|e| e.id == actor_id) {
                actor.action = EntityAction::Off;
                actor
                    .attributes
                    .set("__node_engage_progress", Value::Int64(ctx.ticks));
                actor.set_attribute("__node_behavior_status", Value::Str("cooldown".into()));
            }
            return Ok(None);
        }
        return finish(ctx, actor_id, "cannot_engage");
    };
    let range = resolved_action_range_limit(ctx, &action.range, &actor, 1.5);
    if range <= 0.0 || position.distance(target.get_pos_xz()) <= range {
        if let Some(actor) = ctx.map.entities.iter_mut().find(|e| e.id == actor_id) {
            actor.action = EntityAction::Off;
            actor.set_attribute("__node_behavior_status", Value::Str("attacking".into()));
            actor
                .attributes
                .set("__node_engage_progress", Value::Int64(ctx.ticks));
        }
        if !execute_ruleset_action_with_target(
            ctx,
            actor_id,
            &action.id,
            Some(RulesetActionTarget::Entity(target_id)),
        ) {
            return finish(ctx, actor_id, "cannot_engage");
        }
        return Ok(None);
    }
    let last = Vec2::new(
        actor
            .attributes
            .get_float_default("__node_engage_last_x", position.x),
        actor
            .attributes
            .get_float_default("__node_engage_last_z", position.y),
    );
    let progressed = position.distance(last) > 0.02;
    let since = match actor.attributes.get("__node_engage_progress") {
        Some(Value::Int64(value)) => *value,
        _ => ctx.ticks,
    };
    if !progressed
        && ctx.ticks - since
            > RegionInstance::realtime_seconds_to_ticks(ctx, policy.blocked_seconds)
    {
        return finish(ctx, actor_id, "cannot_engage");
    }
    if let Some(actor) = ctx.map.entities.iter_mut().find(|e| e.id == actor_id) {
        if progressed {
            actor
                .attributes
                .set("__node_engage_progress", Value::Int64(ctx.ticks));
            actor
                .attributes
                .set("__node_engage_last_x", Value::Float(position.x));
            actor
                .attributes
                .set("__node_engage_last_z", Value::Float(position.y));
        }
        actor.set_attribute("__node_behavior_status", Value::Str("closing_in".into()));
        actor.action = EntityAction::CloseIn(target_id, range * 0.9, policy.speed);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn arena() -> RegionCtx {
        let mut ctx = RegionCtx::default();
        ctx.set_rules(eldiron_ruleset::latest_official_ruleset().parse().unwrap())
            .unwrap();
        let mut orc = Entity::new();
        orc.id = 1;
        orc.set_attribute("race", Value::Str("Orc".into()));
        orc.set_attribute("abilities", Value::StrArray(vec!["basic_attack".into()]));
        orc.set_pos_xz(Vec2::new(0.0, 0.0));
        let mut human = Entity::new();
        human.id = 2;
        human.set_attribute("race", Value::Str("Human".into()));
        human.set_pos_xz(Vec2::new(4.0, 0.0));
        ctx.map.entities.extend([orc, human]);
        ctx
    }
    #[test]
    fn lookout_uses_relationships_and_shared_target_attributes() {
        let mut ctx = arena();
        assert!(!node_lookout(&mut ctx, 1, "friendly").unwrap());
        assert!(node_lookout(&mut ctx, 1, "hostile").unwrap());
        assert_eq!(ctx.entity_target(1), Some(2));
        assert_eq!(
            ctx.map.entities[0].attributes.get_uint("attack_target"),
            Some(2)
        );
        ctx.map.entities[1].set_attribute("visible", Value::Bool(false));
        assert!(!node_lookout(&mut ctx, 1, "hostile").unwrap());
        assert!(node_lookout(&mut ctx, 1, "unknown").is_err());
    }
    #[test]
    fn engage_closes_in_and_returns_defeated_without_changing_visibility() {
        let mut ctx = arena();
        node_lookout(&mut ctx, 1, "hostile").unwrap();
        node_engage_start(&mut ctx, 1, "default").unwrap();
        assert_eq!(node_engage_tick(&mut ctx, 1, "default").unwrap(), None);
        assert!(matches!(
            ctx.map.entities[0].action,
            EntityAction::CloseIn(2, _, _)
        ));
        ctx.map.entities[1].set_attribute("mode", Value::Str("dead".into()));
        assert_eq!(
            node_engage_tick(&mut ctx, 1, "default").unwrap(),
            Some("defeated")
        );
        assert_eq!(ctx.entity_target(1), None);
        assert!(matches!(ctx.map.entities[0].action, EntityAction::Off));
        assert!(
            ctx.map.entities[1]
                .attributes
                .get_bool_default("visible", true)
        );
    }
    #[test]
    fn engage_rejects_reused_target_ids_and_enforces_pursuit_limit() {
        let mut ctx = arena();
        node_lookout(&mut ctx, 1, "hostile").unwrap();
        node_engage_start(&mut ctx, 1, "default").unwrap();
        ctx.map.entities[1].creator_id = Uuid::new_v4();
        assert_eq!(
            node_engage_tick(&mut ctx, 1, "default").unwrap(),
            Some("lost")
        );
        ctx.set_entity_target(1, Some(2));
        node_engage_start(&mut ctx, 1, "default").unwrap();
        ctx.map.entities[1].set_pos_xz(Vec2::new(100.0, 0.0));
        assert_eq!(
            node_engage_tick(&mut ctx, 1, "default").unwrap(),
            Some("lost")
        );
    }
    #[test]
    fn engagement_uses_shared_attack_costs_and_cooldowns() {
        let mut ctx = arena();
        let mut rules = ctx.rules.clone();
        rules
            .get_mut("actions")
            .unwrap()
            .get_mut("basic_attack")
            .unwrap()
            .as_table_mut()
            .unwrap()
            .insert(
                "cost".into(),
                toml::Value::Table(toml::from_str("MP = 3").unwrap()),
            );
        ctx.set_rules(rules).unwrap();
        ctx.map.entities[0].set_attribute("MP", Value::Int(5));
        ctx.map.entities[1].set_pos_xz(Vec2::new(0.5, 0.0));
        node_lookout(&mut ctx, 1, "hostile").unwrap();
        node_engage_start(&mut ctx, 1, "default").unwrap();
        assert_eq!(node_engage_tick(&mut ctx, 1, "default").unwrap(), None);
        assert_eq!(ctx.map.entities[0].attributes.get_int_default("MP", 0), 2);
        assert!(is_action_on_cooldown(&ctx, 1, "basic_attack"));
        // Insufficient resources end engagement rather than bypassing its costs.
        assert_eq!(
            node_engage_tick(&mut ctx, 1, "default").unwrap(),
            Some("cannot_engage")
        );
        assert_eq!(ctx.map.entities[0].attributes.get_int_default("MP", 0), 2);
    }
}
