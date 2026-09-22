//! Behavior execution adapter for the existing region lifecycle. This module never
//! spawns map entities, resets the client, or changes rendering initialization.
use super::*;
use crate::server::region::drop_items_into_ruleset_loot_container;
use crate::server::region_host::{
    apply_geometry_object_item_attr, opening_geo_for_item, rebuild_runtime_navigation,
};
use crate::server::{message::RegionMessage, regionctx::RegionCtx};
use crate::{PlayerCamera, Value as WorldValue};

#[derive(Clone, Default)]
pub struct BehaviorAssets {
    pub graphs: HashMap<String, Value>,
    pub characters: HashMap<String, Uuid>,
    pub items: HashMap<String, Uuid>,
    pub regions: HashMap<Uuid, Uuid>,
}
#[derive(Default)]
pub struct Behaviors {
    runtime: Runtime,
    owners: HashMap<Uuid, EventOwner>,
    /// Compiled plans shared by every instance and template using one graph key.
    plans: HashMap<String, Arc<Plan>>,
}

impl BehaviorAssets {
    /// Graph keys addressing this owner, most specific first: a region instance
    /// graph, then the character or item template graph.
    fn graph_keys(&self, ctx: &RegionCtx, owner: &EventOwner, class: &str) -> Vec<String> {
        let region = self.regions.get(&ctx.map.id);
        let keys = match owner {
            EventOwner::World => vec!["behavior/world".into()],
            EventOwner::Region => region
                .into_iter()
                .map(|id| format!("behavior/region/{id}"))
                .collect(),
            EventOwner::Entity(id) => {
                let mut keys: Vec<_> = region
                    .into_iter()
                    .map(|r| format!("behavior/region/{r}/character/{id}"))
                    .collect();
                if let Some(template) = self.characters.get(class) {
                    keys.push(format!("behavior/character/{template}"));
                }
                keys
            }
            EventOwner::Item(id) => {
                let mut keys: Vec<_> = region
                    .into_iter()
                    .map(|r| format!("behavior/region/{r}/item/{id}"))
                    .collect();
                if let Some(template) = self.items.get(class) {
                    keys.push(format!("behavior/item/{template}"));
                }
                keys
            }
            EventOwner::Area(id) => region
                .into_iter()
                .map(|r| format!("behavior/region/{r}/area/{id}"))
                .collect(),
        };
        keys
    }

    /// The graph an owner runs, with the cache key describing it. A specific
    /// graph is layered over the general one: it answers the events it defines
    /// itself and the template answers the rest, so an instance graph only has
    /// to describe what it changes instead of copying the whole behavior.
    fn owner_graph(
        &self,
        ctx: &RegionCtx,
        owner: &EventOwner,
        class: &str,
    ) -> Option<(String, Value)> {
        let keys: Vec<String> = self
            .graph_keys(ctx, owner, class)
            .into_iter()
            .filter(|key| self.graphs.contains_key(key))
            .collect();
        let key = keys.first()?.clone();
        let Some(template_key) = keys.get(1).cloned() else {
            let document = self.graphs.get(&key)?.clone();
            return Some((key, document));
        };
        // A specific graph that will not compile must not take the general graph
        // down with it: fall back to the template alone.
        let Some(instance) = Registry::shared().compile(self.graphs.get(&key)?).ok() else {
            let document = self.graphs.get(&template_key)?.clone();
            return Some((template_key, document));
        };
        let template = Registry::shared()
            .compile(self.graphs.get(&template_key)?)
            .ok()?;
        let owned: HashSet<String> = instance.events.keys().cloned().collect();
        let replaced: HashSet<String> = template
            .events
            .iter()
            .filter(|(event, _)| owned.contains(*event))
            .flat_map(|(_, nodes)| nodes.iter().map(|node| node.to_string()))
            .collect();
        let document = compose_graphs(
            self.graphs.get(&template_key)?,
            self.graphs.get(&key)?,
            &replaced,
        );
        Some((format!("{key}+{template_key}"), document))
    }
}

/// Layers a specific graph over its template. The template keeps every chain the
/// specific graph does not answer for itself, so overriding `startup` leaves the
/// routine, engagement and death chains of the template in place.
pub(super) fn compose_graphs(
    template: &Value,
    specific: &Value,
    replaced_nodes: &HashSet<String>,
) -> Value {
    let mut document = template.clone();
    if let Some(nodes) = document.get_mut("nodes").and_then(Value::as_array_mut) {
        nodes.retain(|node| match node.get("id").and_then(Value::as_str) {
            Some(id) => !replaced_nodes.contains(id),
            None => true,
        });
        if let Some(extra) = specific.get("nodes").and_then(Value::as_array) {
            nodes.extend(extra.iter().cloned());
        }
    }
    if let Some(connections) = document
        .get_mut("connections")
        .and_then(Value::as_array_mut)
        && let Some(extra) = specific.get("connections").and_then(Value::as_array)
    {
        connections.extend(extra.iter().cloned());
    }
    document
}
fn log(ctx: &RegionCtx, text: String) {
    if let Some(sender) = ctx.from_sender.get() {
        let _ = sender.send(RegionMessage::LogMessage(text));
    }
}
/// Receives a captured engine event. No legacy program is evaluated as fallback.
pub fn dispatch(ctx: &mut RegionCtx, event: EventObservation) {
    let Some(assets) = ctx.assets.node_behaviors.as_ref() else {
        return;
    };
    let (identity, render_id, class) = match event.owner {
        EventOwner::World => (Uuid::nil(), 0, "World".into()),
        EventOwner::Region => (ctx.map.id, 0, "Region".into()),
        EventOwner::Entity(identity) => {
            // Stashed bodies are still owned by their graph: the death chain is
            // exactly the event that arrives after death takes the body away.
            let Some(entity) = ctx
                .map
                .entities
                .iter()
                .chain(ctx.stashed_entities.values())
                .find(|e| e.creator_id == identity)
            else {
                return;
            };
            (
                identity,
                entity.id,
                entity.get_attr_string("class_name").unwrap_or_default(),
            )
        }
        EventOwner::Item(identity) => {
            let Some(item) = ctx.map.items.iter().find(|i| i.creator_id == identity) else {
                return;
            };
            (
                identity,
                item.id,
                item.get_attr_string("class_name").unwrap_or_default(),
            )
        }
        EventOwner::Area(identity) => {
            // A place has no body of its own: the entrant carries the action.
            let render_id = match event.fields.get("entity") {
                Some(EventField::Entity(id)) => *id,
                _ => 0,
            };
            (identity, render_id, "Area".into())
        }
    };
    let mut behaviors = ctx.node_behaviors.take().unwrap_or_default();
    // Follow authoritative ownership, not just world presence: a stashed body
    // is out of the world but still belongs to its graph, so it is not stale.
    // Visibility is deliberately not consulted.
    let stale: Vec<_> = behaviors
        .runtime
        .actors
        .values()
        .filter(|actor| match behaviors.owners.get(&actor.handle.identity) {
            Some(EventOwner::Entity(id)) => !ctx
                .map
                .entities
                .iter()
                .chain(ctx.stashed_entities.values())
                .any(|e| e.creator_id == *id && e.id == actor.render_id),
            Some(EventOwner::Item(id)) => !ctx
                .map
                .items
                .iter()
                .any(|i| i.creator_id == *id && i.id == actor.render_id),
            _ => false,
        })
        .map(|actor| actor.handle)
        .collect();
    for handle in stale {
        behaviors.runtime.despawn(handle);
        behaviors.owners.remove(&handle.identity);
    }
    if !behaviors.runtime.actors.contains_key(&identity) {
        let plan = match assets.owner_graph(ctx, &event.owner, &class) {
            Some((key, document)) => match behaviors.plans.get(&key).cloned() {
                Some(plan) => Some(plan),
                None => match Registry::shared().compile(&document) {
                    Ok(plan) => {
                        let plan = Arc::new(plan);
                        behaviors.plans.insert(key, plan.clone());
                        Some(plan)
                    }
                    Err(error) => {
                        log(ctx, format!("[error] Node behavior {class}: {error}"));
                        None
                    }
                },
            },
            None => {
                // No graph means no behavior. This stays silent on purpose: a
                // missing graph is for the author to notice and fix.
                None
            }
        };
        // Attaching is not spawning. Startup arrives from the normal engine path.
        if let Err(error) = behaviors
            .runtime
            .attach_shared(identity, ctx.map.id, render_id, plan)
        {
            log(ctx, format!("[error] Node attachment: {error}"));
            ctx.node_behaviors = Some(behaviors);
            return;
        }
        behaviors.owners.insert(identity, event.owner.clone());
    }
    // Area graphs act on whoever triggered them, so the actor follows the entrant.
    if let Some(actor) = behaviors.runtime.actors.get_mut(&identity) {
        actor.render_id = render_id;
    }
    let visible = match event.owner {
        EventOwner::Entity(_) => ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == render_id)
            .map(|entity| entity.attributes.get_bool_default("visible", true))
            .unwrap_or(true),
        EventOwner::Item(_) => ctx
            .map
            .items
            .iter()
            .find(|item| item.id == render_id)
            .map(|item| item.attributes.get_bool_default("visible", true))
            .unwrap_or(true),
        _ => true,
    };
    behaviors.runtime.actors.get_mut(&identity).unwrap().visible = visible;
    let handle = behaviors.runtime.actors[&identity].handle;
    if !behaviors.runtime.send_observation(handle, event.clone()) {
        log(ctx, "[error] Node event queue is full".into());
    }
    // Startup is emitted only by the engine after actual actor creation. The
    // normal routine then runs in the same context, after the startup chain.
    // An entity with its own graph also gets the instance entry, so an instance
    // graph adds to the template startup instead of replacing it.
    if event.name == "startup" && matches!(event.owner, EventOwner::Entity(_) | EventOwner::Item(_))
    {
        let keys = assets.graph_keys(ctx, &event.owner, &class);
        if keys.len() > 1 && assets.graphs.contains_key(&keys[0]) {
            let mut instance = event.clone();
            instance.name = "instance".into();
            instance.fields.clear();
            behaviors.runtime.send_observation(handle, instance);
        }
    }
    if event.name == "startup" && matches!(event.owner, EventOwner::Entity(_)) {
        let mut routine = event.clone();
        routine.name = "routine".into();
        routine.fields.clear();
        behaviors.runtime.send_observation(handle, routine);
    }
    behaviors.runtime.update(&mut RegionServices {
        ctx,
        owner: event.owner,
    });
    for event in behaviors.runtime.observations.drain(..) {
        super::super::event_observation::retain_observation(&mut ctx.event_observations, event);
    }
    let traces: Vec<_> = behaviors.runtime.traces.drain(..).collect();
    for trace in &traces {
        if let Some(error) = &trace.error {
            log(ctx, format!("[error] Node {}: {error}", trace.node));
        }
    }
    if !traces.is_empty() {
        if let Some(sender) = ctx.from_sender.get() {
            let _ = sender.send(RegionMessage::NodeTraces(traces));
        }
    }
    let highlights = behaviors.runtime.take_highlights();
    if !highlights.is_empty() {
        if let Some(sender) = ctx.from_sender.get() {
            let _ = sender.send(RegionMessage::NodeHighlights(highlights));
        }
    }
    ctx.node_behaviors = Some(behaviors);
}
/// Does this entity belong to the graph's actor? Entity and item graphs match
/// the owner identity as well; an area graph acts on its entrant, which is only
/// known by render id.
fn actor_entity_matches(owner: &EventOwner, entity: &crate::Entity, actor: &Actor) -> bool {
    entity.id == actor.render_id
        && (matches!(owner, EventOwner::Area(_)) || entity.creator_id == actor.handle.identity)
}

/// Resolve the body a graph actor acts on.
///
/// A stashed body is out of the world but still owned by its graph, so it stays
/// resolvable here: that is what lets a death chain run on the character it is
/// about to raise. Only `map.entities` is collidable, targetable and rendered,
/// so presence still answers every question the world asks.
fn find_actor_entity<'a>(
    ctx: &'a RegionCtx,
    owner: &EventOwner,
    actor: &Actor,
) -> Option<&'a crate::Entity> {
    ctx.map
        .entities
        .iter()
        .find(|entity| actor_entity_matches(owner, entity, actor))
        .or_else(|| {
            ctx.stashed_entities
                .values()
                .find(|entity| actor_entity_matches(owner, entity, actor))
        })
}

fn find_actor_entity_mut<'a>(
    ctx: &'a mut RegionCtx,
    owner: &EventOwner,
    actor: &Actor,
) -> Option<&'a mut crate::Entity> {
    if ctx
        .map
        .entities
        .iter()
        .any(|entity| actor_entity_matches(owner, entity, actor))
    {
        return ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| actor_entity_matches(owner, entity, actor));
    }
    ctx.stashed_entities
        .values_mut()
        .find(|entity| actor_entity_matches(owner, entity, actor))
}

/// Apply a character state, including the presence change it implies.
///
/// Kept as a thin local alias so the node services read the same way the script
/// host does; `RegionCtx::apply_entity_mode` owns the rule.
fn apply_entity_mode(ctx: &mut RegionCtx, entity_id: u32, mode: &str) -> bool {
    ctx.apply_entity_mode(entity_id, mode)
}

/// Convert a `Set Attribute` value into the type the target attribute expects.
///
/// The script host keeps health integral and honours the attribute's existing
/// value as a type hint. The node has to do the same, or a graph that sets `HP`
/// stores a float that `get_int` then refuses to read, which silently makes the
/// character undamageable.
pub(super) fn typed_attribute_value(
    attribute: &str,
    value: EventAttribute,
    hint: Option<&WorldValue>,
    health_attr: &str,
) -> WorldValue {
    match value {
        EventAttribute::Text(value) => WorldValue::Str(value),
        EventAttribute::Bool(value) => WorldValue::Bool(value),
        EventAttribute::Number(value) => {
            // Health is integer gameplay state, exactly as the script host treats it.
            if attribute == health_attr {
                return WorldValue::Int(value as i32);
            }
            match hint {
                Some(WorldValue::Int(_)) => WorldValue::Int(value as i32),
                Some(WorldValue::UInt(_)) => WorldValue::UInt(value.max(0.0) as u32),
                Some(WorldValue::Int64(_)) => WorldValue::Int64(value as i64),
                Some(WorldValue::Float(_)) => WorldValue::Float(value as f32),
                _ => WorldValue::Float(value as f32),
            }
        }
    }
}

struct RegionServices<'a> {
    ctx: &'a mut RegionCtx,
    owner: EventOwner,
}
impl RegionServices<'_> {
    /// True when the graph owns a body to act on: its own entity, or the entity
    /// that triggered an area graph.
    fn acts_on_entity(&self) -> bool {
        matches!(self.owner, EventOwner::Entity(_) | EventOwner::Area(_))
    }
    /// True when the graph acts on something that carries attributes, which
    /// includes items.
    fn acts_on_owned(&self) -> bool {
        self.acts_on_entity() || matches!(self.owner, EventOwner::Item(_))
    }
    /// Writes an attribute on the item an item graph belongs to.
    fn set_item_attribute(
        &mut self,
        item_id: u32,
        attribute: &str,
        value: WorldValue,
    ) -> Result<(), String> {
        if matches!(attribute, "visible" | "blocking") {
            // A door keeps its state in the linked geometry object and the wall
            // opening, so mirror what the script host did for it.
            let WorldValue::Bool(enabled) = value else {
                return Err(format!("'{attribute}' expects true or false"));
            };
            let item = self
                .ctx
                .map
                .items
                .iter_mut()
                .find(|item| item.id == item_id)
                .ok_or("Item no longer exists")?;
            item.set_attribute(attribute, WorldValue::Bool(enabled));
            let object = item.attributes.get_id("geometry_object_id");
            let blocking = item.attributes.get_bool_default("blocking", false);
            let opening = opening_geo_for_item(item);
            if attribute == "blocking"
                && let Some(geo_id) = opening
            {
                self.ctx
                    .collision_world
                    .set_opening_state(geo_id, !blocking);
            }
            if let Some(object_id) = object {
                apply_geometry_object_item_attr(self.ctx, object_id, attribute, enabled);
                if attribute == "blocking" {
                    rebuild_runtime_navigation(self.ctx);
                }
            }
            return Ok(());
        }
        let item = self
            .ctx
            .map
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
            .ok_or("Item no longer exists")?;
        item.set_attribute(attribute, value);
        let active = item.attributes.get_bool_default("active", false);
        if attribute == "active" {
            // Changing active drives the item's tile and light, so announce it
            // exactly like the script host did.
            self.ctx.to_execute_item.push((
                item_id,
                "active".into(),
                crate::vm::VMValue::from_bool(active),
            ));
        }
        Ok(())
    }
    /// Reads or forces the light of an item or character.
    fn apply_emit_light(&mut self, actor: &Actor, mode: &str) -> Result<(), String> {
        let mode = mode.trim().to_ascii_lowercase();
        let forced = match mode.as_str() {
            "on" => Some(true),
            "off" => Some(false),
            _ => None,
        };
        if matches!(self.owner, EventOwner::Item(_)) {
            let item_id = actor.render_id;
            let item = self
                .ctx
                .map
                .items
                .iter_mut()
                .find(|item| item.id == item_id)
                .ok_or("Item no longer exists")?;
            let active =
                forced.unwrap_or_else(|| item.attributes.get_bool_default("active", false));
            if let Some(WorldValue::Light(light)) = item.attributes.get_mut("light") {
                light.active = active;
                item.mark_dirty_attribute("light");
            }
            return Ok(());
        }
        let entity = find_actor_entity_mut(self.ctx, &self.owner, actor)
            .ok_or("Character no longer exists")?;
        let active = forced.unwrap_or_else(|| entity.attributes.get_bool_default("active", false));
        if let Some(WorldValue::Light(light)) = entity.attributes.get_mut("light") {
            light.active = active;
            entity.mark_dirty_attribute("light");
        }
        Ok(())
    }
}
impl WorldServices for RegionServices<'_> {
    fn time(&self, _: &Actor) -> theframework::prelude::TheTime {
        self.ctx.time
    }
    fn say(&mut self, actor: &Actor, text: String) {
        let (entity, item) = match self.owner {
            EventOwner::Entity(_) | EventOwner::Area(_) => (Some(actor.render_id), None),
            EventOwner::Item(_) => (None, Some(actor.render_id)),
            _ => (None, None),
        };
        if let Some(sender) = self.ctx.from_sender.get() {
            let _ = sender.send(RegionMessage::Say(
                self.ctx.region_id,
                entity,
                item,
                text,
                String::new(),
            ));
        }
    }
    fn go_to(&mut self, actor: &Actor, destination: &str, speed: f32) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Go To requires a character".into());
        }
        let destination = destination.trim();
        let target = self
            .ctx
            .map
            .named_area_center_3d(destination)
            .ok_or_else(|| format!("Unknown destination '{destination}'"))?;
        let entity = find_actor_entity_mut(self.ctx, &self.owner, actor)
            .ok_or("Character no longer exists")?;
        // Use the existing continuous navigator, including its 3D route handling.
        entity
            .attributes
            .set("__goto_target_height", WorldValue::Float(target.y));
        for key in [
            "__goto_stall_ticks",
            "__goto_no_improve_ticks",
            "__goto_route_blocked_ticks",
        ] {
            entity.attributes.set(key, WorldValue::Int(0));
        }
        entity.attributes.set("__node_goto", WorldValue::Bool(true));
        entity
            .attributes
            .set("__node_goto_arrived", WorldValue::Bool(false));
        entity.action = crate::EntityAction::Goto(vek::Vec2::new(target.x, target.z), speed);
        Ok(())
    }
    fn go_to_result(&mut self, actor: &Actor) -> Option<Result<(), String>> {
        let entity = find_actor_entity_mut(self.ctx, &self.owner, actor)?;
        if entity
            .attributes
            .get_bool_default("__node_goto_arrived", false)
        {
            entity.attributes.remove("__node_goto");
            entity.attributes.remove("__node_goto_arrived");
            return Some(Ok(()));
        }
        if matches!(
            entity.action,
            crate::EntityAction::Goto(..) | crate::EntityAction::GotoRoute { .. }
        ) {
            return None;
        }
        entity.attributes.remove("__node_goto");
        Some(Err(
            "Go To was blocked or interrupted before reaching its destination".into(),
        ))
    }
    fn cancel_activity(&mut self, actor: &Actor) {
        let now = self.ctx.ticks;
        if let Some(entity) = find_actor_entity_mut(self.ctx, &self.owner, actor) {
            if entity.attributes.get_bool_default("__node_engage", false) {
                let delay = match entity.attributes.get("__node_lookout_delay") {
                    Some(WorldValue::Int64(delay)) => *delay,
                    _ => 0,
                };
                entity
                    .attributes
                    .set("__node_lookout_retry", WorldValue::Int64(now + delay));
                entity.attributes.remove("__node_engage");
                entity.attributes.remove("__node_engage_next_attack");
                entity.set_attribute("target", WorldValue::Str(String::new()));
                entity.set_attribute("attack_target", WorldValue::Str(String::new()));
                if matches!(entity.action, crate::EntityAction::CloseIn(..)) {
                    entity.action = crate::EntityAction::Off;
                }
            }
            if is_walk(&entity.action) || entity.attributes.get_bool_default("__node_goto", false) {
                entity.attributes.remove("__node_goto");
                entity.attributes.remove("__node_goto_arrived");
                entity.action = crate::EntityAction::Off;
            }
        }
    }
    fn refresh_random_walk(
        &mut self,
        actor: &Actor,
        area: &str,
        distance: f32,
        speed: f32,
        pause: i32,
    ) -> Result<(), String> {
        let previous = find_actor_entity(self.ctx, &self.owner, actor).map(|e| e.action.clone());
        let Some(mut previous) = previous.filter(is_walk) else {
            return Ok(());
        };
        // Validate the new area using the same service as initial execution.
        match self.random_walk(actor, area, distance, speed, pause) {
            Ok(true) => {}
            result => {
                self.cancel_activity(actor);
                return Err(result
                    .err()
                    .unwrap_or_else(|| "Character is outside the updated walk area".into()));
            }
        }
        refresh_walk(
            &mut previous,
            distance,
            speed,
            pause,
            self.ctx.ticks,
            self.ctx.ticks_per_minute,
        );
        if let Some(entity) = find_actor_entity_mut(self.ctx, &self.owner, actor) {
            entity.action = previous;
        }
        Ok(())
    }
    fn use_action(
        &mut self,
        actor: &Actor,
        action: &str,
        subject: Option<u32>,
    ) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Use Action requires a character".into());
        }
        let target = subject.or_else(|| self.ctx.entity_target(actor.render_id));
        if let Some(target) = target {
            if !self
                .ctx
                .map
                .entities
                .iter()
                .any(|entity| entity.id == target && entity.get_mode() != "dead")
            {
                return Err("Action target no longer exists or is dead".into());
            }
            self.ctx.set_entity_target(actor.render_id, Some(target));
        }
        if crate::server::region::execute_ruleset_action_with_target(
            self.ctx,
            actor.render_id,
            action,
            target.map(crate::server::region::RulesetActionTarget::Entity),
        ) {
            Ok(())
        } else {
            Err(format!("Ruleset action '{action}' could not be performed"))
        }
    }
    fn lookout(
        &mut self,
        actor: &Actor,
        profile: &str,
        reaction: Option<f32>,
        escape: Option<f32>,
    ) -> Result<bool, String> {
        crate::server::region::node_lookout(self.ctx, actor.render_id, profile, reaction, escape)
    }
    fn engage_start(&mut self, actor: &Actor, profile: &str) -> Result<(), String> {
        let result = crate::server::region::node_engage_start(self.ctx, actor.render_id, profile);
        if result.is_err() {
            self.ctx.set_entity_target(actor.render_id, None);
            if let Some(entity) = self
                .ctx
                .map
                .entities
                .iter_mut()
                .find(|e| e.id == actor.render_id)
            {
                let delay = match entity.attributes.get("__node_lookout_delay") {
                    Some(WorldValue::Int64(delay)) => *delay,
                    _ => 0,
                };
                entity.attributes.set(
                    "__node_lookout_retry",
                    WorldValue::Int64(self.ctx.ticks + delay),
                );
            }
        }
        result
    }
    fn engage_tick(
        &mut self,
        actor: &Actor,
        profile: &str,
    ) -> Result<Option<&'static str>, String> {
        let result = crate::server::region::node_engage_tick(self.ctx, actor.render_id, profile);
        if result.is_err() {
            self.cancel_activity(actor);
        }
        result
    }
    fn activity_status(&self, actor: &Actor) -> Option<String> {
        find_actor_entity(self.ctx, &self.owner, actor)
            .and_then(|e| e.attributes.get_str("__node_behavior_status"))
            .map(str::to_owned)
    }
    fn current_target(&self, actor: &Actor) -> Option<u32> {
        self.ctx.entity_target(actor.render_id)
    }
    fn random_walk_active(&self, actor: &Actor) -> bool {
        find_actor_entity(self.ctx, &self.owner, actor)
            .is_some_and(|entity| is_walk(&entity.action))
    }
    fn random_walk(
        &mut self,
        actor: &Actor,
        area: &str,
        distance: f32,
        speed: f32,
        pause: i32,
    ) -> Result<bool, String> {
        if !self.acts_on_entity() {
            return Err("Random Walk requires a character".into());
        }
        let entity =
            find_actor_entity(self.ctx, &self.owner, actor).ok_or("Character no longer exists")?;
        let position = entity.get_pos_xz();
        let area = area.trim();
        if !area.is_empty() {
            if self.ctx.map.named_area_center(area).is_none() {
                return Err(format!("Unknown area '{area}'"));
            }
            if self.ctx.map.named_area_name_at(position).as_deref() != Some(area) {
                return Ok(false);
            }
        }
        if self.ctx.map.find_sector_at(position).is_none()
            && self.ctx.map.geometry_area_bbox_at(position).is_none()
        {
            return Err("Character is not inside a walkable area".into());
        }
        let entity = self
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == actor.render_id)
            .ok_or("Character no longer exists")?;
        entity.action =
            crate::EntityAction::RandomWalkInSector(distance, speed, pause, 0, vek::Vec2::zero());
        Ok(true)
    }
    fn resume_routine(&mut self, actor: &Actor) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Resume Routine requires a character".into());
        }
        self.ctx.to_execute_entity.push((
            actor.render_id,
            "routine".into(),
            crate::vm::VMValue::zero(),
        ));
        Ok(())
    }
    fn player_camera(&mut self, actor: &Actor, camera: &str) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Player Camera requires a character".into());
        }
        let camera = match camera {
            "2d" => PlayerCamera::D2,
            "2d_grid" => PlayerCamera::D2Grid,
            "iso" => PlayerCamera::D3Iso,
            "firstp" => PlayerCamera::D3FirstP,
            "firstp_grid" => PlayerCamera::D3FirstPGrid,
            _ => return Err("Unknown player camera".into()),
        };
        let entity = find_actor_entity_mut(self.ctx, &self.owner, actor)
            .ok_or("Character no longer exists")?;
        entity.set_attribute("player_camera", WorldValue::PlayerCamera(camera));
        Ok(())
    }
    fn set_attribute(
        &mut self,
        actor: &Actor,
        attribute: &str,
        value: EventAttribute,
    ) -> Result<(), String> {
        if !self.acts_on_owned() {
            return Err("Set Attribute requires a character or item".into());
        }
        let attribute = attribute.trim();
        if attribute.is_empty() {
            return Err("Set Attribute needs an attribute name".into());
        }
        if matches!(self.owner, EventOwner::Item(_)) {
            let item_id = actor.render_id;
            let hint = self
                .ctx
                .map
                .items
                .iter()
                .find(|item| item.id == item_id)
                .and_then(|item| item.attributes.get(attribute))
                .cloned();
            let value = typed_attribute_value(
                attribute,
                value,
                hint.as_ref(),
                &self.ctx.health_attr,
            );
            return self.set_item_attribute(item_id, attribute, value);
        }
        let entity_id = find_actor_entity(self.ctx, &self.owner, actor)
            .ok_or("Character no longer exists")?
            .id;
        let hint = self
            .ctx
            .find_entity(entity_id)
            .and_then(|entity| entity.attributes.get(attribute))
            .cloned();
        let value =
            typed_attribute_value(attribute, value, hint.as_ref(), &self.ctx.health_attr);
        // `mode` is not just another attribute: it carries presence. Writing it
        // directly has to move the character in or out of the world, or a graph
        // that sets `mode` instead of using the State node would leave a dead
        // character solid, which is the bug this whole path exists to prevent.
        if attribute == "mode"
            && let WorldValue::Str(mode) = &value
        {
            let mode = mode.clone();
            if !apply_entity_mode(self.ctx, entity_id, &mode) {
                return Err("Character no longer exists".into());
            }
            return Ok(());
        }
        let entity = find_actor_entity_mut(self.ctx, &self.owner, actor)
            .ok_or("Character no longer exists")?;
        entity.set_attribute(attribute, value);
        Ok(())
    }
    fn attribute_display(&self, actor: &Actor, name: &str) -> Option<String> {
        let entity = find_actor_entity(self.ctx, &self.owner, actor)?;
        entity.attributes.get(name).map(|value| value.to_string())
    }
    fn toggle_attribute(&mut self, actor: &Actor, attribute: &str) -> Result<(), String> {
        if !self.acts_on_owned() {
            return Err("Toggle Attribute requires a character or item".into());
        }
        let attribute = attribute.trim();
        if attribute.is_empty() {
            return Err("Toggle Attribute needs an attribute name".into());
        }
        if matches!(self.owner, EventOwner::Item(_)) {
            let item_id = actor.render_id;
            let next = {
                let item = self
                    .ctx
                    .map
                    .items
                    .iter_mut()
                    .find(|item| item.id == item_id)
                    .ok_or("Item no longer exists")?;
                let next = !item.attributes.get_bool_default(attribute, false);
                item.set_attribute(attribute, WorldValue::Bool(next));
                next
            };
            if attribute == "active" {
                self.ctx.to_execute_item.push((
                    item_id,
                    "active".into(),
                    crate::vm::VMValue::from_bool(next),
                ));
            }
            return Ok(());
        }
        let entity = find_actor_entity_mut(self.ctx, &self.owner, actor)
            .ok_or("Character no longer exists")?;
        let next = !entity.attributes.get_bool_default(attribute, false);
        entity.set_attribute(attribute, WorldValue::Bool(next));
        Ok(())
    }
    fn set_emit_light(&mut self, actor: &Actor, mode: &str) -> Result<(), String> {
        if !self.acts_on_owned() {
            return Err("Set Emit Light requires a character or item".into());
        }
        self.apply_emit_light(actor, mode)
    }
    fn open_dialog(&mut self, actor: &Actor, target: u32, node: &str) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Dialog requires a character".into());
        }
        // The graph owner is the speaker whose class data holds the dialogue
        // tree; the acting target is the listener who triggered the event. The
        // two have to reach `open_dialog_node` in that order, or it looks the
        // tree up on the listener and finds nothing.
        if !crate::server::region::open_dialog_node(
            self.ctx,
            actor.render_id,
            target,
            node.trim(),
        ) {
            return Err(format!("Unknown dialogue node '{}'", node.trim()));
        }
        Ok(())
    }
    fn present_dialog(
        &mut self,
        actor: &Actor,
        target: u32,
        text: &str,
        choices: &[(String, Option<String>)],
    ) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Dialogue requires a character".into());
        }
        crate::server::region::present_node_dialogue(
            self.ctx,
            actor.render_id,
            target,
            text,
            choices,
        );
        Ok(())
    }
    fn take_dialog_choice(&mut self, actor: &Actor) -> Option<DialogChoiceMade> {
        self.ctx.dialog_choices.remove(&actor.render_id)
    }
    fn talk_step(&mut self, actor: &Actor) -> Option<String> {
        self.ctx.talk_steps.get(&actor.render_id).cloned()
    }
    fn set_talk_step(&mut self, actor: &Actor, step: Option<&str>) {
        match step {
            Some(step) => {
                self.ctx
                    .talk_steps
                    .insert(actor.render_id, step.to_string());
            }
            None => {
                self.ctx.talk_steps.remove(&actor.render_id);
            }
        }
    }
    fn take_talk_resume(&mut self, actor: &Actor) -> Option<String> {
        self.ctx.talk_resumes.remove(&actor.render_id)
    }
    fn set_talk_resume(&mut self, actor: &Actor, step: Option<&str>) {
        match step {
            Some(step) if !step.trim().is_empty() => {
                self.ctx
                    .talk_resumes
                    .insert(actor.render_id, step.trim().to_string());
            }
            _ => {
                self.ctx.talk_resumes.remove(&actor.render_id);
            }
        }
    }
    fn offer_inventory(&mut self, actor: &Actor, target: u32, filter: &str) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Offer Inventory requires a character".into());
        }
        if !crate::server::region::offer_inventory_to_entity(
            self.ctx,
            actor.render_id,
            target,
            filter.trim(),
        ) {
            return Err("The seller no longer exists".into());
        }
        Ok(())
    }
    fn inventory_has(&mut self, target: u32, item: &str) -> Result<bool, String> {
        let Some(entity) = self.ctx.map.entities.iter().find(|e| e.id == target) else {
            return Err("The target no longer exists".into());
        };
        let item = item.trim();
        Ok(entity.iter_inventory().any(|(_, carried)| {
            let name = carried.attributes.get_str("name").unwrap_or_default();
            let class_name = carried.attributes.get_str("class_name").unwrap_or_default();
            item.is_empty()
                || name.contains(item)
                || class_name.contains(item)
                || class_name.eq_ignore_ascii_case(item)
        }))
    }
    fn notify_in(&mut self, actor: &Actor, minutes: f32, event: &str) -> Result<(), String> {
        if !self.acts_on_owned() {
            return Err("Notify In requires a character or item".into());
        }
        let event = event.trim();
        if event.is_empty() {
            return Err("Notify In needs an event name".into());
        }
        let target = self.ctx.ticks
            + crate::server::region::RegionInstance::game_minutes_to_ticks(
                self.ctx,
                minutes.max(0.0),
            );
        if matches!(self.owner, EventOwner::Item(_)) {
            self.ctx
                .notifications_items
                .push((actor.render_id, target, event.to_string()));
        } else {
            self.ctx
                .notifications_entities
                .push((actor.render_id, target, event.to_string()));
        }
        Ok(())
    }
    fn entities_in_radius(&mut self, actor: &Actor) -> Result<bool, String> {
        if !self.acts_on_owned() {
            return Err("Entities In Radius requires a character or item".into());
        }
        let item_owner = matches!(self.owner, EventOwner::Item(_));
        let source = if item_owner {
            self.ctx
                .map
                .items
                .iter()
                .find(|item| item.id == actor.render_id)
                .map(|item| {
                    (
                        item.get_pos_xz(),
                        item.attributes.get_float_default("radius", 0.5),
                    )
                })
        } else {
            find_actor_entity(self.ctx, &self.owner, actor).map(|entity| {
                (
                    entity.get_pos_xz(),
                    entity.attributes.get_float_default("radius", 0.5),
                )
            })
        };
        let Some((position, own_radius)) = source else {
            return Err("The owner no longer exists".into());
        };
        // The radius an item or character carries is the one collisions use, so
        // the check stays coherent with gameplay instead of inventing its own.
        let radius = own_radius.max(0.5);
        let occupied = self.ctx.map.entities.iter().any(|other| {
            if other.get_mode() == "dead" {
                return false;
            }
            if !item_owner && actor_entity_matches(&self.owner, other, actor) {
                return false;
            }
            other.get_pos_xz().distance(position) <= radius
        });
        Ok(occupied)
    }
    fn add_item(&mut self, actor: &Actor, item: &str) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Add Item requires a character".into());
        }
        let name = item.trim();
        let Some(item) = self.ctx.create_item(name.to_string()) else {
            return Err(format!("Unknown item template '{name}'"));
        };
        let entity_id = actor.render_id;
        let Some(entity) = self.ctx.get_entity_mut(entity_id) else {
            return Err("Character no longer exists".into());
        };
        // The slot index is not interesting here; a full inventory is.
        entity.add_item(item).map(|_| ())
    }
    fn drop_items(&mut self, actor: &Actor, filter: &str) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Drop Items requires a character".into());
        }
        let entity_id = actor.render_id;
        if drop_items_into_ruleset_loot_container(self.ctx, entity_id, filter) {
            return Ok(());
        }
        let mut dropped = Vec::new();
        if let Some(entity) = self.ctx.get_entity_mut(entity_id) {
            let slots: Vec<usize> = entity
                .iter_inventory()
                .filter_map(|(slot, item)| {
                    let name = item.attributes.get_str("name").unwrap_or_default();
                    let class_name = item.attributes.get_str("class_name").unwrap_or_default();
                    if filter.is_empty() || name.contains(filter) || class_name.contains(filter) {
                        Some(slot)
                    } else {
                        None
                    }
                })
                .collect();
            for slot in slots {
                if let Some(mut item) = entity.remove_item_from_slot(slot) {
                    // Drop where the character stands, as the Eldrin host did.
                    item.position = entity.position;
                    item.mark_all_dirty();
                    dropped.push(item);
                }
            }
        }
        self.ctx.map.items.extend(dropped);
        Ok(())
    }
    fn message(&mut self, actor: &Actor, text: String, role: &str) -> Result<(), String> {
        let (entity, item) = match self.owner {
            EventOwner::Entity(_) | EventOwner::Area(_) => (Some(actor.render_id), None),
            EventOwner::Item(_) => (None, Some(actor.render_id)),
            _ => (None, None),
        };
        if let Some(sender) = self.ctx.from_sender.get() {
            let _ = sender.send(RegionMessage::Message(
                self.ctx.region_id,
                entity,
                item,
                actor.render_id,
                text,
                role.to_string(),
            ));
        }
        Ok(())
    }
    fn teleport(&mut self, actor: &Actor, area: &str, sector: &str) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("Teleport requires a character".into());
        }
        let area = area.trim();
        if area.is_empty() {
            return Err("Teleport needs a destination area".into());
        }
        if crate::server::region::teleport_entity_to_area(
            self.ctx,
            actor.render_id,
            area,
            sector.trim(),
        ) {
            Ok(())
        } else {
            Err(format!("Unknown destination area '{area}'"))
        }
    }
    fn set_state(&mut self, actor: &Actor, state: &str) -> Result<(), String> {
        if !self.acts_on_entity() {
            return Err("State requires a character".into());
        }
        let mode = match state {
            "Alive" => "active",
            "Dead" => "dead",
            "Sleeping" => "sleeping",
            "Unconscious" => "unconscious",
            _ => return Err("Unknown state".into()),
        };
        // Resolve first so a stashed character is still addressable, then let
        // the shared helper apply both the state and the presence it implies.
        let entity_id = find_actor_entity(self.ctx, &self.owner, actor)
            .ok_or("Character no longer exists")?
            .id;
        if !apply_entity_mode(self.ctx, entity_id, mode) {
            return Err("Character no longer exists".into());
        }
        Ok(())
    }
}

fn is_walk(action: &crate::EntityAction) -> bool {
    match action {
        crate::EntityAction::RandomWalk(..) | crate::EntityAction::RandomWalkInSector(..) => true,
        crate::EntityAction::SleepAndSwitch(_, next) => is_walk(next),
        _ => false,
    }
}
fn refresh_walk(
    action: &mut crate::EntityAction,
    distance: f32,
    speed: f32,
    pause: i32,
    now: i64,
    ticks_per_minute: u32,
) {
    use crate::EntityAction::*;
    match action {
        RandomWalk(d, s, p, _, _) | RandomWalkInSector(d, s, p, _, _) => {
            *d = distance;
            *s = speed;
            *p = pause;
        }
        SleepAndSwitch(wake, next) => {
            refresh_walk(next, distance, speed, pause, now, ticks_per_minute);
            if pause == 0 {
                *action = (**next).clone();
            } else {
                *wake = (*wake).min(now + pause as i64 * ticks_per_minute as i64);
            }
        }
        _ => {}
    }
}

/// Install valid edits at the region boundary. Existing one-shot effects are not
/// replayed; an active operation can opt into refreshing through its own module.
/// Called after movement is merged back into the authoritative map.
pub fn tick(ctx: &mut RegionCtx) {
    if ctx.paused {
        return;
    }
    let Some(mut behaviors) = ctx.node_behaviors.take() else {
        return;
    };
    let actors: Vec<_> = behaviors
        .runtime
        .actors
        .values()
        .filter_map(|actor| {
            behaviors
                .owners
                .get(&actor.handle.identity)
                .map(|owner| (actor.handle, owner.clone()))
        })
        .collect();
    for (handle, owner) in actors {
        let exists = match &owner {
            // A stashed body still owns its graph. Despawning here would tear
            // down the very actor that has to handle the death event and raise
            // the character again.
            EventOwner::Entity(id) => {
                let render_id = behaviors.runtime.actors[&handle.identity].render_id;
                ctx.map
                    .entities
                    .iter()
                    .chain(ctx.stashed_entities.values())
                    .any(|e| e.creator_id == *id && e.id == render_id)
            }
            EventOwner::Item(id) => ctx.map.items.iter().any(|e| e.creator_id == *id),
            _ => true,
        };
        if !exists {
            behaviors.runtime.despawn(handle);
            behaviors.owners.remove(&handle.identity);
            continue;
        }
        if matches!(owner, EventOwner::Entity(_))
            && crate::server::region::is_entity_dead_ctx(
                ctx,
                behaviors.runtime.actors[&handle.identity].render_id,
            )
        {
            behaviors.runtime.suspend_activity(handle);
            continue;
        }
        behaviors
            .runtime
            .tick(handle, ctx.ticks, &mut RegionServices { ctx, owner });
    }
    for event in behaviors.runtime.observations.drain(..) {
        super::super::event_observation::retain_observation(&mut ctx.event_observations, event);
    }
    let traces: Vec<_> = behaviors.runtime.traces.drain(..).collect();
    for trace in &traces {
        if let Some(error) = &trace.error {
            log(ctx, format!("[error] Node {}: {error}", trace.node));
        }
    }
    if !traces.is_empty() {
        if let Some(sender) = ctx.from_sender.get() {
            let _ = sender.send(RegionMessage::NodeTraces(traces));
        }
    }
    let highlights = behaviors.runtime.take_highlights();
    if !highlights.is_empty() {
        if let Some(sender) = ctx.from_sender.get() {
            let _ = sender.send(RegionMessage::NodeHighlights(highlights));
        }
    }
    ctx.node_behaviors = Some(behaviors);
}

pub fn refresh_graph(ctx: &mut RegionCtx, key: String, graph: Value) {
    let Some(assets) = ctx.assets.node_behaviors.as_ref() else {
        return;
    };
    if assets
        .graphs
        .get(&key)
        .is_some_and(|old| Registry::same_program(old, &graph))
    {
        ctx.assets
            .node_behaviors
            .as_mut()
            .unwrap()
            .graphs
            .insert(key, graph);
        return;
    }
    let plan = match Registry::shared().compile(&graph) {
        Ok(plan) => Arc::new(plan),
        Err(error) => {
            log(
                ctx,
                format!(
                    "[error] Live graph update rejected for {key}: {error}. Keeping the last valid behavior."
                ),
            );
            return;
        }
    };
    ctx.assets
        .node_behaviors
        .as_mut()
        .unwrap()
        .graphs
        .insert(key.clone(), graph.clone());
    let Some(mut behaviors) = ctx.node_behaviors.take() else {
        return;
    };
    // The refreshed graph may only cover some of its events, so drop the cached
    // plans and recompose per owner below.
    behaviors.plans.clear();
    let targets: Vec<_> = behaviors
        .runtime
        .actors
        .values()
        .filter_map(|actor| {
            let owner = behaviors.owners.get(&actor.handle.identity)?;
            let class = match owner {
                EventOwner::Entity(id) => ctx
                    .map
                    .entities
                    .iter()
                    .find(|e| e.creator_id == *id && e.id == actor.render_id)?
                    .get_attr_string("class_name")
                    .unwrap_or_default(),
                EventOwner::Item(id) => ctx
                    .map
                    .items
                    .iter()
                    .find(|i| i.creator_id == *id && i.id == actor.render_id)?
                    .get_attr_string("class_name")
                    .unwrap_or_default(),
                _ => String::new(),
            };
            (ctx.assets
                .node_behaviors
                .as_ref()?
                .graph_keys(ctx, owner, &class)
                .contains(&key))
            .then(|| (actor.handle, owner.clone(), class.clone()))
        })
        .collect();
    for (handle, owner, class) in targets {
        let recomposed = ctx
            .assets
            .node_behaviors
            .as_ref()
            .and_then(|assets| assets.owner_graph(ctx, &owner, &class))
            .and_then(|(plan_key, document)| {
                let plan = Arc::new(Registry::shared().compile(&document).ok()?);
                behaviors.plans.insert(plan_key, plan.clone());
                Some(plan)
            })
            .unwrap_or_else(|| plan.clone());
        if let Err(error) = behaviors.runtime.replace_graph(
            handle,
            recomposed,
            &mut RegionServices {
                ctx,
                owner: owner.clone(),
            },
        ) {
            log(ctx, format!("[error] Live behavior refresh: {error}"));
        }
        for (actor, event, _) in &mut behaviors.runtime.pending {
            if *actor == handle {
                event.tick = ctx.ticks;
                event.owner = owner.clone();
            }
        }
        behaviors.runtime.update(&mut RegionServices { ctx, owner });
    }
    for event in behaviors.runtime.observations.drain(..) {
        super::super::event_observation::retain_observation(&mut ctx.event_observations, event);
    }
    let traces: Vec<_> = behaviors.runtime.traces.drain(..).collect();
    for trace in &traces {
        if let Some(error) = &trace.error {
            log(ctx, format!("[error] Node {}: {error}", trace.node));
        }
    }
    if !traces.is_empty() {
        if let Some(sender) = ctx.from_sender.get() {
            let _ = sender.send(RegionMessage::NodeTraces(traces));
        }
    }
    let highlights = behaviors.runtime.take_highlights();
    if !highlights.is_empty() {
        if let Some(sender) = ctx.from_sender.get() {
            let _ = sender.send(RegionMessage::NodeHighlights(highlights));
        }
    }
    ctx.node_behaviors = Some(behaviors);
}

#[cfg(test)]
mod live_tests {
    use super::*;
    #[test]
    fn zero_pause_releases_existing_wait_and_keeps_walk_state() {
        use crate::EntityAction::*;
        let target = vek::Vec2::new(2., 3.);
        let mut action = SleepAndSwitch(500, Box::new(RandomWalkInSector(3., 1., 4, 0, target)));
        refresh_walk(&mut action, 5., 2., 0, 10, 60);
        assert!(matches!(action, RandomWalkInSector(5., 2., 0, 0, point) if point == target));
    }
    #[test]
    fn live_speed_update_preserves_current_target_and_movement_state() {
        use crate::EntityAction::*;
        let target = vek::Vec2::new(2., 3.);
        let mut action = RandomWalkInSector(3., 1., 4, 1, target);
        refresh_walk(&mut action, 5., 2., 1, 10, 60);
        assert!(matches!(action, RandomWalkInSector(5., 2., 1, 1, point) if point == target));
    }
}
