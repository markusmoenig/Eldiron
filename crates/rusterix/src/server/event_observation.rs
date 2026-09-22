//! Bounded, read-only event observations for editors, independent of source debugging.
use super::regionctx::{RegionCtx, ScriptScope};
use crate::vm::VMValue;
use std::collections::{BTreeMap, VecDeque};
use theframework::prelude::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub enum EventOwner {
    World,
    Region,
    Entity(Uuid),
    Item(Uuid),
    /// A named place that owns its own behavior: a 2D sector or a named 3D
    /// geometry area. Keyed on the place's stable id, never its name.
    Area(Uuid),
}
#[derive(Clone, Debug, PartialEq)]
pub enum EventField {
    Text(String),
    Number(f64),
    Entity(u32),
}
impl EventField {
    pub fn display(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Number(n) => n.to_string(),
            Self::Entity(id) => id.to_string(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct EventObservation {
    pub map: Uuid,
    pub owner: EventOwner,
    pub name: String,
    pub tick: i64,
    pub fields: BTreeMap<String, EventField>,
}
/// Latest observations are retained, never an unbounded per-tick trace.
pub fn retain_observation(queue: &mut VecDeque<EventObservation>, value: EventObservation) {
    if queue.len() >= 256 {
        queue.pop_front();
    }
    queue.push_back(value);
}
/// Legacy event-dispatch adapter. The observation DTO does not expose VM values.
pub fn observe(ctx: &mut RegionCtx, name: &str, value: &VMValue) {
    if let Some(event) = capture(ctx, name, value) {
        retain_observation(&mut ctx.event_observations, event);
    }
}
/// Area-owned view of an enter or leave event.
///
/// The entering entity owns the action, the named place owns the graph. The
/// engine resolves the place by name at emit time (2D sectors first, then named
/// 3D geometry areas) and hands the graph the same payload plus the entrant.
pub fn area_observations(ctx: &RegionCtx, event: &EventObservation) -> Vec<EventObservation> {
    if !matches!(event.name.as_str(), "entered" | "left") {
        return Vec::new();
    }
    let Some(name) = event.fields.get("area").map(EventField::display) else {
        return Vec::new();
    };
    let Some(owner) = ctx.map.named_area_owner(&name) else {
        return Vec::new();
    };
    let mut area = event.clone();
    area.owner = EventOwner::Area(owner);
    if ctx.curr_entity_id != 0 {
        area.fields
            .insert("entity".into(), EventField::Entity(ctx.curr_entity_id));
    }
    vec![area]
}
/// Translate the existing engine event envelope without invoking a script.
pub fn capture(ctx: &RegionCtx, name: &str, value: &VMValue) -> Option<EventObservation> {
    let owner = match ctx.current_script_scope {
        ScriptScope::World => EventOwner::World,
        ScriptScope::Region => EventOwner::Region,
        ScriptScope::Entity => {
            // A stashed body still owns its events. Death removes the body from
            // the world, and the death event is delivered right afterwards, so
            // resolving only `map.entities` would swallow it.
            let Some(e) = ctx.find_entity(ctx.curr_entity_id) else {
                return None;
            };
            EventOwner::Entity(e.creator_id)
        }
        ScriptScope::Item => {
            let Some(i) = ctx
                .curr_item_id
                .and_then(|id| ctx.map.items.iter().find(|i| i.id == id))
            else {
                return None;
            };
            EventOwner::Item(i.creator_id)
        }
    };
    let mut fields = BTreeMap::new();
    let mut text = |key: &str| {
        if let Some(s) = value.as_string() {
            fields.insert(key.into(), EventField::Text(s.into()));
        }
    };
    match name {
        "arrived" => text("destination"),
        "entered" | "left" => text("area"),
        "damaged" => text("kind"),
        "intent" => text("intent"),
        _ => {}
    }
    match name {
        "damaged" => {
            fields.insert("attacker".into(), EventField::Entity(value.x as u32));
            fields.insert("amount".into(), EventField::Number(value.y as f64));
        }
        "intent" => {
            fields.insert("subject".into(), EventField::Entity(value.x as u32));
            fields.insert("distance".into(), EventField::Number(value.y as f64));
            fields.insert("count".into(), EventField::Number(value.z as f64));
        }
        "time" => {
            fields.insert("hour".into(), EventField::Number(value.x as f64));
        }
        // Collision events carry the other party in `x`. `bumped_by_entity` is the
        // one an item reacts to, which is how doors and gates notice a walker.
        "bumped_by_entity" | "bumped_into_entity" => {
            fields.insert("entity".into(), EventField::Entity(value.x as u32));
        }
        "bumped_into_item" => {
            if let Some(item) = ctx.map.items.iter().find(|item| item.id == value.x as u32) {
                let name = item
                    .attributes
                    .get_str("name")
                    .or_else(|| item.attributes.get_str("class_name"))
                    .unwrap_or_default();
                fields.insert("item".into(), EventField::Text(name.to_string()));
            }
        }
        "kill" => {
            let killed = value.x as u32;
            fields.insert("killed".into(), EventField::Entity(killed));
            // The victim is stashed once it dies, so its name has to be resolved
            // through the stash or `{event.name}` loses the name of the killed.
            if let Some(name) = ctx
                .find_entity(killed)
                .and_then(|entity| entity.get_attr_string("name"))
            {
                fields.insert("name".into(), EventField::Text(name));
            }
        }
        _ => {}
    }
    Some(EventObservation {
        map: ctx.map.id,
        owner,
        name: name.into(),
        tick: ctx.ticks,
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_exposes_who_bumped_into_what() {
        let mut ctx = RegionCtx::default();
        let gate_id = Uuid::new_v4();
        let mut gate = crate::Item::new();
        gate.id = 5;
        gate.creator_id = gate_id;
        gate.set_attribute("name", crate::Value::Str("Gate".into()));
        ctx.map.items.push(gate);
        ctx.current_script_scope = ScriptScope::Item;
        ctx.curr_item_id = Some(5);

        // The item answers with whoever walked into it.
        let event = capture(&ctx, "bumped_by_entity", &VMValue::broadcast(9.0)).expect("bump");
        assert_eq!(event.owner, EventOwner::Item(gate_id));
        assert_eq!(event.fields.get("entity"), Some(&EventField::Entity(9)));

        // The mover answers the mirror event with the item it hit.
        let mut walker = crate::Entity::new();
        walker.id = 7;
        walker.creator_id = Uuid::new_v4();
        ctx.map.entities.push(walker);
        ctx.current_script_scope = ScriptScope::Entity;
        ctx.curr_entity_id = 7;
        let event =
            capture(&ctx, "bumped_into_item", &VMValue::broadcast(5.0)).expect("mirror bump");
        assert_eq!(
            event.fields.get("item"),
            Some(&EventField::Text("Gate".into()))
        );
    }

    #[test]
    fn capture_exposes_the_killed_entity_and_its_name() {
        let mut ctx = RegionCtx::default();
        ctx.current_script_scope = ScriptScope::Entity;
        let mut attacker = crate::Entity::new();
        attacker.id = 1;
        attacker.creator_id = Uuid::new_v4();
        let mut victim = crate::Entity::new();
        victim.id = 2;
        victim.creator_id = Uuid::new_v4();
        victim.set_attribute("name", crate::Value::Str("Bone Warden".into()));
        ctx.map.entities.push(attacker);
        ctx.map.entities.push(victim);
        ctx.curr_entity_id = 1;

        let event = capture(&ctx, "kill", &VMValue::broadcast(2.0)).expect("kill observation");
        assert_eq!(event.fields.get("killed"), Some(&EventField::Entity(2)));
        assert_eq!(
            event.fields.get("name"),
            Some(&EventField::Text("Bone Warden".into()))
        );
    }

    #[test]
    fn capture_resolves_the_dead_after_death_stashed_them() {
        let mut ctx = RegionCtx::default();
        ctx.current_script_scope = ScriptScope::Entity;
        let mut victim = crate::Entity::new();
        victim.id = 2;
        victim.creator_id = Uuid::new_v4();
        victim.set_attribute("name", crate::Value::Str("Bone Warden".into()));
        victim.set_attribute("mode", crate::Value::Str("dead".into()));
        ctx.map.entities.push(victim);
        // Death takes the body out of the world before the death and kill events
        // are delivered, so both have to resolve through the stash.
        assert!(ctx.stash_entity(2));
        ctx.curr_entity_id = 2;

        let death = capture(&ctx, "death", &VMValue::zero()).expect("death observation");
        assert_eq!(
            death.owner,
            EventOwner::Entity(ctx.stashed_entities[&2].creator_id)
        );

        let kill = capture(&ctx, "kill", &VMValue::broadcast(2.0)).expect("kill observation");
        assert_eq!(kill.fields.get("killed"), Some(&EventField::Entity(2)));
        assert_eq!(
            kill.fields.get("name"),
            Some(&EventField::Text("Bone Warden".into())),
            "the killer's graph still learns who it killed"
        );
    }
}
