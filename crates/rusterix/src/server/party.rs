//! Party operations shared by behavior nodes and world adapters.
use super::regionctx::RegionCtx;
use crate::{EntityAction, Value};

pub(crate) fn join_entity_party(
    ctx: &mut RegionCtx,
    companion_id: u32,
    leader_id: u32,
) -> Option<i32> {
    if companion_id == leader_id
        || !ctx.map.entities.iter().any(|entity| {
            entity.id == leader_id && entity.is_player() && entity.get_mode() != "dead"
        })
    {
        return None;
    }

    if let Some(existing) = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == companion_id)
        .and_then(|entity| entity.attributes.get_int("party_index"))
        .filter(|index| *index > 0)
    {
        return ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == companion_id)
            .filter(|e| e.attributes.get_uint("party_leader_id") == Some(leader_id))
            .map(|_| existing);
    }

    let occupied = ctx
        .map
        .entities
        .iter()
        .filter(|entity| entity.attributes.get_uint("party_leader_id") == Some(leader_id))
        .filter_map(|entity| entity.attributes.get_int("party_index"))
        .collect::<std::collections::HashSet<_>>();
    let capacity = ctx
        .rules
        .get("party")
        .and_then(|p| p.get("max_companions"))
        .and_then(toml::Value::as_integer)
        .unwrap_or(3)
        .clamp(0, 64) as i32;
    let slot = (1..=capacity).find(|slot| !occupied.contains(slot))?;

    let companion = ctx
        .map
        .entities
        .iter_mut()
        .find(|entity| entity.id == companion_id)?;
    companion.set_attribute("party_member", Value::Bool(true));
    companion.set_attribute("party_index", Value::Int(slot));
    companion.set_attribute("party_role", Value::Str(format!("party.{slot}")));
    companion.set_attribute("party_leader_id", Value::UInt(leader_id));
    companion.set_attribute("target", Value::Str(String::new()));
    companion.set_attribute("attack_target", Value::Str(String::new()));
    if companion
        .attributes
        .get_bool_default("hide_when_joined", false)
    {
        companion.set_attribute("visible", Value::Bool(false));
    }
    companion.action = EntityAction::Off;
    companion.active_sequence = None;
    companion.paused_sequence = None;
    companion.mark_all_dirty();

    ctx.entity_proximity_alerts.remove(&companion_id);
    Some(slot)
}

/// Remove membership without changing life state. Visibility follows the optional recruitment flag.
pub(crate) fn leave_entity_party(ctx: &mut RegionCtx, companion_id: u32) -> bool {
    let Some(companion) = ctx
        .map
        .entities
        .iter_mut()
        .find(|e| e.id == companion_id && e.attributes.get_bool_default("party_member", false))
    else {
        return false;
    };
    companion.set_attribute("party_member", Value::Bool(false));
    companion.set_attribute("party_index", Value::Int(0));
    companion.set_attribute("party_role", Value::Str(String::new()));
    companion.set_attribute("party_leader_id", Value::UInt(0));
    if companion
        .attributes
        .get_bool_default("hide_when_joined", false)
    {
        companion.set_attribute("visible", Value::Bool(true));
    }
    companion.mark_all_dirty();
    true
}
