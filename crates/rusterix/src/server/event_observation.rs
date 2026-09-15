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
/// Translate the existing engine event envelope without invoking a script.
pub fn capture(ctx: &RegionCtx, name: &str, value: &VMValue) -> Option<EventObservation> {
    let owner = match ctx.current_script_scope {
        ScriptScope::World => EventOwner::World,
        ScriptScope::Region => EventOwner::Region,
        ScriptScope::Entity => {
            let Some(e) = ctx.map.entities.iter().find(|e| e.id == ctx.curr_entity_id) else {
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
        "entered" => text("sector"),
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
