//! Behavior execution adapter for the existing region lifecycle. This module never
//! spawns map entities, resets the client, or changes rendering initialization.
use super::*;
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
}

impl BehaviorAssets {
    fn graph<'a>(&'a self, ctx: &RegionCtx, owner: &EventOwner, class: &str) -> Option<&'a Value> {
        self.graph_key(ctx, owner, class)
            .and_then(|key| self.graphs.get(&key))
    }
    fn graph_key(&self, ctx: &RegionCtx, owner: &EventOwner, class: &str) -> Option<String> {
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
        };
        keys.into_iter().find(|key| self.graphs.contains_key(key))
    }
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
            let Some(entity) = ctx.map.entities.iter().find(|e| e.creator_id == identity) else {
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
    };
    let mut behaviors = ctx.node_behaviors.take().unwrap_or_default();
    // Follow authoritative map presence. Visibility is deliberately not consulted.
    let stale: Vec<_> = behaviors
        .runtime
        .actors
        .values()
        .filter(|actor| match behaviors.owners.get(&actor.handle.identity) {
            Some(EventOwner::Entity(id)) => !ctx
                .map
                .entities
                .iter()
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
        let graph = assets.graph(ctx, &event.owner, &class);
        let plan = match graph {
            Some(graph) => match Registry::builtin().compile(graph) {
                Ok(plan) => Some(plan),
                Err(error) => {
                    log(ctx, format!("[error] Node behavior {class}: {error}"));
                    None
                }
            },
            None => {
                if matches!(event.owner, EventOwner::Entity(_) | EventOwner::Item(_)) {
                    log(
                        ctx,
                        format!("[nodes] {class}: no behavior graph; Eldrin behavior is disabled"),
                    );
                }
                None
            }
        };
        // Attaching is not spawning. Startup arrives from the normal engine path.
        if let Err(error) = behaviors
            .runtime
            .attach(identity, ctx.map.id, render_id, plan)
        {
            log(ctx, format!("[error] Node attachment: {error}"));
            ctx.node_behaviors = Some(behaviors);
            return;
        }
        behaviors.owners.insert(identity, event.owner.clone());
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
struct RegionServices<'a> {
    ctx: &'a mut RegionCtx,
    owner: EventOwner,
}
impl WorldServices for RegionServices<'_> {
    fn time(&self, _: &Actor) -> theframework::prelude::TheTime {
        self.ctx.time
    }
    fn say(&mut self, actor: &Actor, text: String) {
        let (entity, item) = match self.owner {
            EventOwner::Entity(_) => (Some(actor.render_id), None),
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
        if !matches!(self.owner, EventOwner::Entity(_)) {
            return Err("Go To requires a character".into());
        }
        let destination = destination.trim();
        let target = self
            .ctx
            .map
            .named_area_center_3d(destination)
            .ok_or_else(|| format!("Unknown destination '{destination}'"))?;
        let entity = self
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == actor.render_id && e.creator_id == actor.handle.identity)
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
        let entity = self
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == actor.render_id && e.creator_id == actor.handle.identity)?;
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
        if let Some(entity) = self
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == actor.render_id && e.creator_id == actor.handle.identity)
        {
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
        let previous = self
            .ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == actor.render_id && e.creator_id == actor.handle.identity)
            .map(|e| e.action.clone());
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
        if let Some(entity) = self
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == actor.render_id && e.creator_id == actor.handle.identity)
        {
            entity.action = previous;
        }
        Ok(())
    }
    fn random_walk_active(&self, actor: &Actor) -> bool {
        self.ctx
            .map
            .entities
            .iter()
            .find(|entity| {
                entity.id == actor.render_id && entity.creator_id == actor.handle.identity
            })
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
        if !matches!(self.owner, EventOwner::Entity(_)) {
            return Err("Random Walk requires a character".into());
        }
        let entity = self
            .ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == actor.render_id && e.creator_id == actor.handle.identity)
            .ok_or("Character no longer exists")?;
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
        if !matches!(self.owner, EventOwner::Entity(_)) {
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
        if !matches!(self.owner, EventOwner::Entity(_)) {
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
        let entity = self
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == actor.render_id && e.creator_id == actor.handle.identity)
            .ok_or("Character no longer exists")?;
        entity.set_attribute("player_camera", WorldValue::PlayerCamera(camera));
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
            EventOwner::Entity(id) => ctx.map.entities.iter().any(|e| {
                e.creator_id == *id && e.id == behaviors.runtime.actors[&handle.identity].render_id
            }),
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
    let registry = Registry::builtin();
    if let Err(error) = registry.compile(&graph) {
        log(
            ctx,
            format!(
                "[error] Live graph update rejected for {key}: {error}. Keeping the last valid behavior."
            ),
        );
        return;
    }
    ctx.assets
        .node_behaviors
        .as_mut()
        .unwrap()
        .graphs
        .insert(key.clone(), graph.clone());
    let Some(mut behaviors) = ctx.node_behaviors.take() else {
        return;
    };
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
                .graph_key(ctx, owner, &class)
                .as_deref()
                == Some(key.as_str()))
            .then(|| (actor.handle, owner.clone()))
        })
        .collect();
    for (handle, owner) in targets {
        match registry.compile(&graph) {
            Ok(plan) => {
                if let Err(error) = behaviors.runtime.replace_graph(
                    handle,
                    plan,
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
            Err(error) => log(ctx, format!("[error] Live behavior compile: {error}")),
        }
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
