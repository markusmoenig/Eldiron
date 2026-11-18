//! Storage reconstruction never emits Startup or starting grants.
use super::*;
use crate::server::{nodes::region::Behaviors, persistence::*};

pub struct PreparedRegion {
    record: RegionRecord,
    map: Map,
    mini: crate::map::mini::MapMini,
    collision: crate::collision_world::CollisionWorld,
    behaviors: Behaviors,
}
impl RegionInstance {
    pub fn durable_commands_settled(&self) -> bool {
        self.to_receiver.is_empty() && self.from_receiver.is_empty()
    }

    pub fn capture_durable_region(&self) -> Result<RegionRecord, String> {
        let context = self.context().ok_or("Region unavailable")?;
        let ctx = context.lock().unwrap();
        Ok(RegionRecord {
            geometry: SavedGeometry::capture(&ctx.map),
            entities: ctx.map.entities.iter().map(SavedEntity::capture).collect(),
            items: ctx.map.items.iter().map(SavedItem::capture).collect(),
            stashed: ctx
                .stashed_entities
                .values()
                .map(SavedEntity::capture)
                .collect(),
            respawns: ctx
                .entity_respawn_snapshots
                .iter()
                .map(|(k, e)| (*k, SavedEntity::capture(e)))
                .collect(),
            values: ctx.region_state.clone(),
            ticks: ctx.ticks,
            time: ctx.time,
            entity_classes: ctx
                .entity_classes
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            item_classes: ctx
                .item_classes
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            entity_state: ctx
                .entity_state_data
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            item_state: ctx
                .item_state_data
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            entity_alerts: ctx
                .entity_proximity_alerts
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            item_alerts: ctx
                .item_proximity_alerts
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            entity_notifications: ctx.notifications_entities.clone(),
            item_notifications: ctx.notifications_items.clone(),
            talk_steps: ctx
                .talk_steps
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            talk_resumes: ctx
                .talk_resumes
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            dismiss_dialogues: ctx.active_choice_sessions.iter().map(|s| s.from).collect(),
            entity_events: ctx.to_execute_entity.clone(),
            item_events: ctx.to_execute_item.clone(),
            world_events: ctx.to_execute_world.clone(),
            behaviors: ctx
                .node_behaviors
                .as_ref()
                .map(Behaviors::save)
                .unwrap_or_default(),
            globals: self.exec.globals.clone(),
            simulation_time: self.simulation_time,
            last_advance: self.last_simulation_advance_at,
            last_request: self.last_external_step_request_at,
            pending_system: self.pending_system_steps,
            pending_redraw: self.pending_redraw_steps,
            step_pending: self.simulation_step_pending,
        })
    }
    pub fn prepare_durable_region(&self, record: &RegionRecord) -> Result<PreparedRegion, String> {
        let context = self.context().ok_or("Region unavailable")?;
        let ctx = context.lock().unwrap();
        let mut map = record.geometry.materialize();
        map.entities = record
            .entities
            .iter()
            .map(SavedEntity::materialize)
            .collect();
        map.items = record.items.iter().map(SavedItem::materialize).collect();
        let mini = map.as_mini(&ctx.blocking_tiles);
        let collision = crate::chunkbuilder::d3chunkbuilder::D3ChunkBuilder::build_collision_world(
            &map,
            &ctx.assets,
        );
        let behaviors = Behaviors::restore(&record.behaviors, ctx.assets.node_behaviors.as_ref())?;
        resources::accesses(&map, &ctx.rules)?;
        Ok(PreparedRegion {
            record: record.clone(),
            map,
            mini,
            collision,
            behaviors,
        })
    }
    pub fn apply_durable_region(&mut self, prepared: PreparedRegion) {
        let record = prepared.record;
        let frames: Vec<_> = self.frame_motion.keys().copied().collect();
        self.frame_motion.clear();
        let context = self.context().unwrap();
        {
            let mut ctx = context.lock().unwrap();
            ctx.map = prepared.map;
            ctx.resource_access = resources::accesses(&ctx.map, &ctx.rules).expect("Validated saved resource bindings");
            ctx.mapmini = prepared.mini;
            ctx.collision_world = prepared.collision;
            ctx.node_behaviors = Some(prepared.behaviors);
            ctx.region_state = record.values;
            ctx.ticks = record.ticks;
            ctx.time = record.time;
            ctx.entity_classes = record.entity_classes.into_iter().collect();
            ctx.item_classes = record.item_classes.into_iter().collect();
            ctx.entity_state_data = record.entity_state.into_iter().collect();
            ctx.item_state_data = record.item_state.into_iter().collect();
            ctx.entity_proximity_alerts = record.entity_alerts.into_iter().collect();
            ctx.item_proximity_alerts = record.item_alerts.into_iter().collect();
            ctx.stashed_entities = record
                .stashed
                .iter()
                .map(|e| (e.id, e.materialize()))
                .collect();
            ctx.entity_respawn_snapshots = record
                .respawns
                .iter()
                .map(|(k, e)| (*k, e.materialize()))
                .collect();
            ctx.notifications_entities = record.entity_notifications;
            ctx.notifications_items = record.item_notifications;
            ctx.talk_steps = record.talk_steps.into_iter().collect();
            ctx.talk_resumes = record.talk_resumes.into_iter().collect();
            ctx.dialog_choices.clear();
            for id in record.dismiss_dialogues {
                ctx.dialog_choices
                    .insert(id, crate::server::nodes::DialogChoiceMade::Dismissed);
            }
            ctx.active_choice_sessions.clear();
            ctx.active_container_sessions.clear();
            ctx.entity_tile_contacts.clear();
            ctx.pending_entity_removals.clear();
            ctx.pending_entity_transfers.clear();
            ctx.to_execute_entity = record.entity_events;
            ctx.to_execute_item = record.item_events;
            ctx.to_execute_world = record.world_events;
            // Private condition/timer state and active FX were saved together.
            // Re-running condition restoration here would duplicate FX entities.
        }
        self.exec.globals = record.globals;
        self.simulation_time = record.simulation_time;
        self.last_simulation_advance_at = record.last_advance;
        self.last_external_step_request_at = record.last_request;
        self.pending_system_steps = record.pending_system;
        self.pending_redraw_steps = record.pending_redraw;
        self.simulation_step_pending = record.step_pending;
        self.current_frame_has_turn_step = false;
        for actor in frames {
            self.set_frame_motion(actor, true);
        }
    }
}
