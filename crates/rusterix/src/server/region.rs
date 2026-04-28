use crate::server::message::DialogChoice;
use crate::server::py_fn::*;
use crate::server::region_host::{run_client_fn, run_server_fn, run_server_named_fn};
use crate::vm::*;
use crate::{
    Assets, Choice, Currency, Entity, EntityAction, Item, Map, MultipleChoice, PixelSource,
    PlayerCamera, RegionCtx, Value, ValueContainer,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use instant::{Duration, Instant};
use pathfinding::prelude::astar;
use rand::seq::SliceRandom;
use rand::*;

use std::sync::{Arc, Mutex};
use theframework::prelude::*;

use std::sync::atomic::{AtomicU32, Ordering};
use vek::{Vec2, Vec3};

use std::sync::{LazyLock, RwLock};

/// The global store of RegionCtx
static REGIONCTX: LazyLock<RwLock<FxHashMap<u32, Arc<Mutex<RegionCtx>>>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));

/// Register a new RegionCtx
pub fn register_regionctx(id: u32, instance: Arc<Mutex<RegionCtx>>) {
    REGIONCTX.write().unwrap().insert(id, instance);
}

/// Clear the store.
pub fn clear_regionctx_store() {
    REGIONCTX.write().unwrap().clear();
    RegionCtx::clear_world_state();
}

/// Get a specific RegionCtx
pub fn get_regionctx(id: u32) -> Option<Arc<Mutex<RegionCtx>>> {
    REGIONCTX.read().unwrap().get(&id).cloned()
}

/// Get gelper
pub fn with_regionctx<F, R>(region_id: u32, f: F) -> Option<R>
where
    F: FnOnce(&mut RegionCtx) -> R,
{
    get_regionctx(region_id).map(|arc| {
        let mut ctx = arc.lock().unwrap(); // Consider proper error handling if needed
        f(&mut ctx)
    })
}

/// Get the region id embedded in the VM
// pub fn get_region_id(vm: &VirtualMachine) -> Option<u32> {
//     let module = vm.import("__region_meta", 0).ok()?;
//     let obj = module.get_attr("__region_id", vm).ok()?;
//     obj.try_to_value::<u32>(vm).ok()
// }

// Global ID generator over all threads and regions.
// 0 is reserved as NO_ID / None sentinel.
static GLOBAL_ID_GEN: AtomicU32 = AtomicU32::new(1);

pub fn get_global_id() -> u32 {
    GLOBAL_ID_GEN.fetch_add(1, Ordering::Relaxed)
}

pub fn reset_global_id_gen() {
    GLOBAL_ID_GEN.store(1, Ordering::Relaxed);
}

fn map_spawn_height(map: &Map, pos: Vec2<f32>, preferred_y: Option<f32>) -> f32 {
    // Spawn on a walkable floor, not on overlapping roof sectors.
    if let Some(pref_y) = preferred_y {
        if let Some(h) = sector_floor_height_below_or_nearest(map, pos, pref_y) {
            return h;
        }
    } else {
        let mut highest_floor: Option<f32> = None;
        for sector in map
            .sectors
            .iter()
            .filter(|s| s.layer.is_none() && s.is_inside(map, pos))
        {
            if map
                .get_surface_for_sector_id(sector.id)
                .map(|surface| surface.plane.normal.y.abs() <= 0.7)
                .unwrap_or(true)
            {
                continue;
            }
            if sector.properties.get_float_default("roof_height", 0.0) > 0.0 {
                continue;
            }
            // Use average world-Y of sector boundary vertices for multi-level geometry.
            let mut vertex_ids: Vec<u32> = Vec::new();
            let mut sum_y = 0.0f32;
            let mut count = 0usize;
            for linedef_id in &sector.linedefs {
                if let Some(ld) = map.find_linedef(*linedef_id) {
                    if !vertex_ids.contains(&ld.start_vertex) {
                        vertex_ids.push(ld.start_vertex);
                        if let Some(v) = map.get_vertex_3d(ld.start_vertex) {
                            sum_y += v.y;
                            count += 1;
                        }
                    }
                    if !vertex_ids.contains(&ld.end_vertex) {
                        vertex_ids.push(ld.end_vertex);
                        if let Some(v) = map.get_vertex_3d(ld.end_vertex) {
                            sum_y += v.y;
                            count += 1;
                        }
                    }
                }
            }
            if count > 0 {
                let h = sum_y / count as f32;
                highest_floor = Some(highest_floor.map_or(h, |prev| prev.max(h)));
            }
        }
        if let Some(h) = highest_floor {
            return h;
        }
    }
    let config = crate::chunkbuilder::terrain_generator::TerrainConfig::default();
    crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(map, pos, &config)
}

fn sector_floor_height_below_or_nearest(
    map: &Map,
    pos: Vec2<f32>,
    reference_y: f32,
) -> Option<f32> {
    let mut best_below: Option<f32> = None;
    let mut best_above: Option<f32> = None;
    let mut best_above_dist = f32::INFINITY;
    const FLOOR_EPS: f32 = 0.05;

    for sector in map
        .sectors
        .iter()
        .filter(|s| s.layer.is_none() && s.is_inside(map, pos))
    {
        if map
            .get_surface_for_sector_id(sector.id)
            .map(|surface| surface.plane.normal.y.abs() <= 0.7)
            .unwrap_or(true)
        {
            continue;
        }
        // Roof sectors overlap the house footprint in XZ, but should not be used as walk floors.
        if sector.properties.get_float_default("roof_height", 0.0) > 0.0 {
            continue;
        }
        let mut vertex_ids: Vec<u32> = Vec::new();
        let mut sum_y = 0.0f32;
        let mut count = 0usize;
        for linedef_id in &sector.linedefs {
            if let Some(ld) = map.find_linedef(*linedef_id) {
                if !vertex_ids.contains(&ld.start_vertex) {
                    vertex_ids.push(ld.start_vertex);
                    if let Some(v) = map.get_vertex_3d(ld.start_vertex) {
                        sum_y += v.y;
                        count += 1;
                    }
                }
                if !vertex_ids.contains(&ld.end_vertex) {
                    vertex_ids.push(ld.end_vertex);
                    if let Some(v) = map.get_vertex_3d(ld.end_vertex) {
                        sum_y += v.y;
                        count += 1;
                    }
                }
            }
        }
        if count == 0 {
            continue;
        }

        let h = sum_y / count as f32;
        if h <= reference_y + FLOOR_EPS {
            best_below = Some(match best_below {
                Some(curr) => curr.max(h),
                None => h,
            });
        } else {
            let d = h - reference_y;
            if d < best_above_dist {
                best_above_dist = d;
                best_above = Some(h);
            } else if (d - best_above_dist).abs() < 1e-4 && h < best_above.unwrap_or(f32::INFINITY)
            {
                best_above = Some(h);
            }
        }
    }

    best_below.or(best_above)
}
use EntityAction::*;

use super::data::{apply_entity_data, apply_item_data};
use super::{AudioCommand, RegionMessage};
use crate::server::regionctx::{ChoiceSession, ScriptScope};
use RegionMessage::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CollisionMode {
    Tile,
    Mesh,
}

struct MovementResult {
    geometry_blocked: bool,
    dynamic_collision: bool,
}

struct DynamicCollisionProbe {
    blocking_collision: bool,
}

pub struct RegionInstance {
    pub id: u32,

    vm: VM,
    exec: Execution,

    name: String,

    /// Send messages to this region
    pub to_sender: Sender<RegionMessage>,
    /// Local receiver
    to_receiver: Receiver<RegionMessage>,

    /// Send messages from this region
    from_sender: Sender<RegionMessage>,
    /// Local receiver
    pub from_receiver: Receiver<RegionMessage>,

    /// Entity block mode
    entity_block_mode: i32,
    collision_mode: CollisionMode,
    last_redraw_at: Instant,
    last_simulation_advance_at: Instant,
    last_external_step_request_at: Instant,
    current_frame_has_turn_step: bool,
    simulation_step_pending: bool,
    pending_system_steps: u32,
    pending_redraw_steps: u32,
    movement_units_per_sec: f32,
}

impl RegionInstance {
    fn probe_dynamic_collisions_in_ctx(
        &self,
        ctx: &mut RegionCtx,
        entity: &Entity,
        test_position: Vec2<f32>,
    ) -> DynamicCollisionProbe {
        let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
        let mut blocking_collision = false;

        for other in ctx.map.entities.iter() {
            if other.id == entity.id || other.get_mode() == "dead" {
                continue;
            }

            let other_pos = other.get_pos_xz();
            let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
            let combined_radius = radius + other_radius;
            let combined_radius_sq = combined_radius * combined_radius;

            let dist_vec = test_position - other_pos;
            let dist_sq = dist_vec.magnitude_squared();
            if dist_sq < combined_radius_sq {
                blocking_collision = true;
                if let Some(_class_name) = ctx.entity_classes.get(&entity.id) {
                    ctx.to_execute_entity.push((
                        entity.id,
                        "bumped_into_entity".into(),
                        VMValue::broadcast(other.id as f32),
                    ));
                }
                if let Some(_class_name) = ctx.entity_classes.get(&other.id) {
                    ctx.to_execute_entity.push((
                        other.id,
                        "bumped_by_entity".into(),
                        VMValue::broadcast(entity.id as f32),
                    ));
                }
            }
        }

        for other in ctx.map.items.iter() {
            if !other.attributes.get_bool_default("visible", false) {
                continue;
            }

            let other_pos = other.get_pos_xz();
            let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
            let combined_radius = radius + other_radius;
            let combined_radius_sq = combined_radius * combined_radius;

            let dist_vec = test_position - other_pos;
            let dist_sq = dist_vec.magnitude_squared();
            if dist_sq < combined_radius_sq {
                if let Some(_class_name) = ctx.entity_classes.get(&entity.id) {
                    ctx.to_execute_entity.push((
                        entity.id,
                        "bumped_into_item".into(),
                        VMValue::broadcast(other.id as f32),
                    ));
                }
                if let Some(_class_name) = ctx.item_classes.get(&other.id) {
                    ctx.to_execute_item.push((
                        other.id,
                        "bumped_by_entity".into(),
                        VMValue::broadcast(entity.id as f32),
                    ));
                }

                if other.attributes.get_bool_default("blocking", false) {
                    blocking_collision = true;
                }
            }
        }

        DynamicCollisionProbe { blocking_collision }
    }

    fn is_first_person_camera(player_camera: &PlayerCamera) -> bool {
        matches!(
            player_camera,
            PlayerCamera::D3FirstP | PlayerCamera::D3FirstPGrid
        )
    }

    fn is_grid_camera(player_camera: &PlayerCamera) -> bool {
        matches!(
            player_camera,
            PlayerCamera::D2Grid | PlayerCamera::D3FirstPGrid
        )
    }

    fn should_keep_player_intent(ctx: &RegionCtx, entity: &Entity) -> bool {
        if !entity.is_player()
            || !get_config_bool_default(ctx, "game", "click_intents_2d", false)
                && !get_config_bool_default(ctx, "game", "persistent_2d_intents", false)
        {
            return false;
        }

        matches!(
            entity.attributes.get("player_camera"),
            Some(Value::PlayerCamera(PlayerCamera::D2 | PlayerCamera::D2Grid))
        )
    }

    fn is_movement_input_action(action: &EntityAction) -> bool {
        matches!(
            action,
            EntityAction::Off
                | EntityAction::Left
                | EntityAction::Forward
                | EntityAction::Right
                | EntityAction::Backward
                | EntityAction::StrafeLeft
                | EntityAction::StrafeRight
                | EntityAction::ForwardLeft
                | EntityAction::ForwardRight
                | EntityAction::BackwardLeft
                | EntityAction::BackwardRight
        )
    }

    fn should_use_directional_player_intent(entity: &Entity, click_intents_2d: bool) -> bool {
        let intent = entity.attributes.get_str_default("intent", "".into());
        if intent.is_empty() {
            return false;
        }

        if !click_intents_2d || !entity.is_player() {
            return true;
        }

        !matches!(
            entity.attributes.get("player_camera"),
            Some(Value::PlayerCamera(PlayerCamera::D2 | PlayerCamera::D2Grid))
        )
    }

    fn entity_click_distance(
        ctx: &RegionCtx,
        entity_id: u32,
        target_entity_id: u32,
    ) -> Option<f32> {
        let actor_pos = ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == entity_id)
            .map(|e| e.get_pos_xz())?;
        let target_pos = ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == target_entity_id)
            .map(|e| e.get_pos_xz())?;
        Some(actor_pos.distance(target_pos))
    }

    fn item_click_distance(
        ctx: &RegionCtx,
        entity_id: u32,
        item_id: u32,
        owner_entity_id: Option<u32>,
    ) -> Option<f32> {
        let actor = ctx.map.entities.iter().find(|e| e.id == entity_id)?;
        let actor_pos = actor.get_pos_xz();
        let actor_radius = actor.attributes.get_float_default("radius", 0.5).max(0.0);

        if let Some(item) = ctx.map.items.iter().find(|i| i.id == item_id) {
            let item_radius = item.attributes.get_float_default("radius", 0.5).max(0.0);
            return Some(
                (actor_pos.distance(item.get_pos_xz()) - actor_radius - item_radius).max(0.0),
            );
        }

        if let Some(owner_id) = owner_entity_id
            && let Some(owner) = ctx.map.entities.iter().find(|e| e.id == owner_id)
        {
            let owner_radius = owner.attributes.get_float_default("radius", 0.5).max(0.0);
            return Some(
                (actor_pos.distance(owner.get_pos_xz()) - actor_radius - owner_radius).max(0.0),
            );
        }

        Some(0.0)
    }

    fn resolve_named_sector_center(map: &Map, name: &str, from: Vec2<f32>) -> Option<Vec2<f32>> {
        let needle = name.trim();
        if needle.is_empty() {
            return None;
        }
        let needle_lower = needle.to_ascii_lowercase();

        map.sectors
            .iter()
            .filter(|sector| sector.name.trim().eq_ignore_ascii_case(&needle_lower))
            .filter_map(|sector| {
                sector
                    .center(map)
                    .map(|center| (center, from.distance_squared(center)))
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|entry| entry.0)
    }

    fn resolve_named_entity_target(
        ctx: &RegionCtx,
        actor_id: u32,
        name: &str,
    ) -> Option<(u32, f32)> {
        let actor = ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == actor_id)?;
        let actor_pos = actor.get_pos_xz();
        let needle = name.trim();
        if needle.is_empty() {
            return None;
        }

        ctx.map
            .entities
            .iter()
            .filter(|entity| entity.id != actor_id)
            .filter_map(|entity| {
                let entity_name = entity.attributes.get_str("name").unwrap_or_default();
                let class_name = entity.attributes.get_str("class_name").unwrap_or_default();
                if !entity_name.eq_ignore_ascii_case(needle)
                    && !class_name.eq_ignore_ascii_case(needle)
                {
                    return None;
                }
                let distance = Self::entity_click_distance(ctx, actor_id, entity.id)
                    .unwrap_or_else(|| (actor_pos - entity.get_pos_xz()).magnitude());
                Some((
                    entity.id,
                    distance,
                    actor_pos.distance_squared(entity.get_pos_xz()),
                ))
            })
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|entry| (entry.0, entry.1))
    }

    fn resolve_named_item_target(
        ctx: &RegionCtx,
        actor_id: u32,
        name: &str,
    ) -> Option<(u32, Option<u32>, f32)> {
        let actor = ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == actor_id)?;
        let actor_pos = actor.get_pos_xz();
        let needle = name.trim();
        if needle.is_empty() {
            return None;
        }

        ctx.map
            .items
            .iter()
            .filter_map(|item| {
                let item_name = item.attributes.get_str("name").unwrap_or_default();
                let class_name = item.attributes.get_str("class_name").unwrap_or_default();
                if !item_name.eq_ignore_ascii_case(needle)
                    && !class_name.eq_ignore_ascii_case(needle)
                {
                    return None;
                }
                let distance = Self::item_click_distance(ctx, actor_id, item.id, None)
                    .unwrap_or_else(|| (actor_pos - item.get_pos_xz()).magnitude());
                Some((
                    item.id,
                    None,
                    distance,
                    actor_pos.distance_squared(item.get_pos_xz()),
                ))
            })
            .min_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal))
            .map(|entry| (entry.0, entry.1, entry.2))
    }

    fn queue_named_goto(&self, entity: &mut Entity, coord: Vec2<f32>, speed: f32) {
        let position = entity.get_pos_xz();
        let start_center = Self::snapped_grid_center(position);
        let target_center = Self::snapped_grid_center(coord);
        let grid_aligned = (position - start_center).magnitude_squared() <= 0.001
            && (coord - target_center).magnitude_squared() <= 0.001;
        if grid_aligned {
            entity.action = EntityAction::GotoGrid(coord, speed);
        } else {
            entity.action = EntityAction::Goto(coord, speed);
        }
    }

    fn queue_sequence_use(
        &self,
        ctx: &RegionCtx,
        entity_id: u32,
        target: &str,
        intent: &str,
    ) -> bool {
        if let Some((item_id, owner_entity_id, distance)) =
            Self::resolve_named_item_target(ctx, entity_id, target)
        {
            let _ = self.to_sender.send(UserAction(
                entity_id,
                EntityAction::ItemClicked(
                    item_id,
                    distance,
                    Some(intent.to_string()),
                    owner_entity_id,
                ),
            ));
            return true;
        }

        if let Some((target_entity_id, distance)) =
            Self::resolve_named_entity_target(ctx, entity_id, target)
        {
            let _ = self.to_sender.send(UserAction(
                entity_id,
                EntityAction::EntityClicked(target_entity_id, distance, Some(intent.to_string())),
            ));
            return true;
        }

        false
    }

    fn resolve_named_item_id(ctx: &RegionCtx, actor_id: u32, target: &str) -> Option<u32> {
        Self::resolve_named_item_target(ctx, actor_id, target).map(|entry| entry.0)
    }

    fn advance_entity_sequence(&self, ctx: &mut RegionCtx, entity: &mut Entity) {
        let mut state = match entity.active_sequence.clone() {
            Some(state) => state,
            None => return,
        };

        loop {
            let Some(sequence) = entity.sequences.get(&state.name) else {
                entity.active_sequence = None;
                return;
            };

            if state.step_index >= sequence.steps.len() {
                entity.active_sequence = None;
                return;
            }

            if let Some(wait_until_tick) = state.wait_until_tick {
                if ctx.ticks < wait_until_tick {
                    entity.active_sequence = Some(state);
                    return;
                }
                state.wait_until_tick = None;
            }

            let step = &sequence.steps[state.step_index];
            match step.action.as_str() {
                "goto" => {
                    let Some(target) = Self::resolve_named_sector_center(
                        &ctx.map,
                        &step.target,
                        entity.get_pos_xz(),
                    ) else {
                        entity.active_sequence = None;
                        return;
                    };

                    if entity.get_pos_xz().distance(target) <= 0.1 {
                        state.step_index += 1;
                        continue;
                    }

                    if entity.action == EntityAction::Off {
                        self.queue_named_goto(entity, target, step.speed.unwrap_or(1.0));
                    }
                    entity.active_sequence = Some(state);
                    return;
                }
                "use" => {
                    let intent = step.intent.as_deref().unwrap_or("use");
                    if self.queue_sequence_use(ctx, entity.id, &step.target, intent) {
                        state.step_index += 1;
                        state.wait_until_tick = Some(ctx.ticks + 1);
                        continue;
                    }
                    entity.active_sequence = None;
                    return;
                }
                "ensure_active" => {
                    let desired = step.value.unwrap_or(true);
                    let Some(item_id) = Self::resolve_named_item_id(ctx, entity.id, &step.target)
                    else {
                        entity.active_sequence = None;
                        return;
                    };

                    let Some(item) = ctx.map.items.iter().find(|item| item.id == item_id) else {
                        entity.active_sequence = None;
                        return;
                    };

                    if item.attributes.get_bool_default("active", false) == desired {
                        state.step_index += 1;
                        continue;
                    }

                    if entity.action == EntityAction::Off
                        && self.queue_sequence_use(ctx, entity.id, &step.target, "use")
                    {
                        state.wait_until_tick = Some(ctx.ticks + 1);
                    }
                    entity.active_sequence = Some(state);
                    return;
                }
                "wait" => {
                    let seconds = step.seconds.unwrap_or(1.0).max(0.0);
                    let wait_ticks = (seconds * ctx.ticks_per_minute as f32 / 60.0).round() as i64;
                    state.step_index += 1;
                    if wait_ticks > 0 {
                        state.wait_until_tick = Some(ctx.ticks + wait_ticks.max(1));
                    }
                    continue;
                }
                _ => {
                    state.step_index += 1;
                    continue;
                }
            }
        }
    }

    fn snapped_cardinal_direction(direction: Vec2<f32>) -> Vec2<f32> {
        if direction.magnitude_squared() <= 1e-6 {
            return Vec2::new(1.0, 0.0);
        }

        if direction.x.abs() >= direction.y.abs() {
            Vec2::new(direction.x.signum(), 0.0)
        } else {
            Vec2::new(0.0, direction.y.signum())
        }
    }

    pub(crate) fn snapped_grid_center(pos: Vec2<f32>) -> Vec2<f32> {
        Vec2::new(pos.x.floor() + 0.5, pos.y.floor() + 0.5)
    }

    fn grid_press_speed(entity: &Entity) -> f32 {
        entity.attributes.get_float_default("speed", 1.0).max(0.01)
    }

    fn note_simulation_step_request(&mut self) {
        self.simulation_step_pending = true;
    }

    fn is_click_like_step_action(action: &EntityAction) -> bool {
        matches!(
            action,
            EntityAction::EntityClicked(_, _, _)
                | EntityAction::ItemClicked(_, _, _, _)
                | EntityAction::TerrainClicked(_)
                | EntityAction::Choice(_)
        )
    }

    fn should_accept_step_request(&self, ctx: &RegionCtx, action: &EntityAction) -> bool {
        if matches!(
            ctx.simulation_mode,
            crate::server::regionctx::SimulationMode::Realtime
        ) {
            return true;
        }
        if Self::is_click_like_step_action(action)
            && self.last_external_step_request_at.elapsed() < Duration::from_millis(150)
        {
            return false;
        }
        true
    }

    fn non_realtime_turn_dt(ctx: &RegionCtx) -> f32 {
        get_config_i32_default(ctx, "game", "game_tick_ms", 250).max(1) as f32 / 1000.0
    }

    fn autonomous_action_dt(ctx: &RegionCtx, entity: &Entity) -> f32 {
        if matches!(
            ctx.simulation_mode,
            crate::server::regionctx::SimulationMode::Realtime
        ) || entity.is_player()
        {
            ctx.delta_time
        } else {
            Self::non_realtime_turn_dt(ctx)
        }
    }

    fn close_in_step_distance(
        ctx: &RegionCtx,
        entity: &Entity,
        speed: f32,
        units_per_sec: f32,
    ) -> f32 {
        let base = units_per_sec * speed * Self::autonomous_action_dt(ctx, entity);
        if matches!(
            ctx.simulation_mode,
            crate::server::regionctx::SimulationMode::Realtime
        ) || entity.is_player()
        {
            base
        } else {
            base.min(1.0)
        }
    }

    fn close_in_arrived(
        &self,
        ctx: &RegionCtx,
        position: Vec2<f32>,
        target: Vec2<f32>,
        target_radius: f32,
    ) -> bool {
        if self.collision_mode == CollisionMode::Mesh
            || matches!(
                ctx.simulation_mode,
                crate::server::regionctx::SimulationMode::Realtime
            )
        {
            return (target - position).magnitude() <= target_radius;
        }

        let snapped_pos = Self::snapped_grid_center(position);
        let snapped_target = Self::snapped_grid_center(target);
        let delta = snapped_target - snapped_pos;
        let cardinal_distance = delta.x.abs() + delta.y.abs();
        cardinal_distance <= target_radius + 1e-4
    }

    fn follow_attack_cooldown_ticks(ctx: &RegionCtx, entity: &Entity) -> i64 {
        if matches!(
            ctx.simulation_mode,
            crate::server::regionctx::SimulationMode::Realtime
        ) {
            let attack_time = entity
                .attributes
                .get_float_default("avatar_attack_time", 0.35)
                .max(0.05);
            ((attack_time * ctx.ticks_per_minute as f32) / 60.0)
                .ceil()
                .max(1.0) as i64
        } else {
            1
        }
    }

    fn end_follow_attack(ctx: &mut RegionCtx, entity: &mut Entity, reason: &str) {
        entity.set_attribute("target", Value::Str(String::new()));
        entity.set_attribute("attack_target", Value::Str(String::new()));
        entity.set_attribute("__follow_attack_budget", Value::Float(0.0));
        entity.action = EntityAction::Off;

        if ctx.entity_classes.contains_key(&entity.id) {
            ctx.to_execute_entity.push((
                entity.id,
                "engagement_over".into(),
                VMValue::from_string(reason.to_string()),
            ));
        }
    }

    pub(crate) fn scheduled_delay_ticks(ctx: &RegionCtx, units: f32) -> i64 {
        let units = units.max(0.0);
        if units <= 0.0 {
            return 0;
        }
        if matches!(
            ctx.simulation_mode,
            crate::server::regionctx::SimulationMode::Realtime
        ) {
            (ctx.ticks_per_minute as f32 * units).round().max(1.0) as i64
        } else {
            units.round().max(1.0) as i64
        }
    }

    fn queue_simulation_step(&mut self) {
        self.pending_system_steps = self.pending_system_steps.saturating_add(1);
        self.pending_redraw_steps = self.pending_redraw_steps.saturating_add(1);
        self.last_simulation_advance_at = Instant::now();
    }

    fn grant_simulation_steps_if_due(&mut self, ctx: &RegionCtx) {
        match ctx.simulation_mode {
            crate::server::regionctx::SimulationMode::Realtime => {}
            crate::server::regionctx::SimulationMode::TurnBased => {
                if self.simulation_step_pending {
                    self.simulation_step_pending = false;
                    self.queue_simulation_step();
                }
            }
            crate::server::regionctx::SimulationMode::Hybrid => {
                let timeout_elapsed = self.last_simulation_advance_at.elapsed()
                    >= Duration::from_millis(ctx.turn_timeout_ms.max(1) as u64);
                if self.simulation_step_pending || timeout_elapsed {
                    self.simulation_step_pending = false;
                    self.queue_simulation_step();
                }
            }
        }
    }

    fn consume_system_step_if_allowed(&mut self, ctx: &RegionCtx) -> bool {
        match ctx.simulation_mode {
            crate::server::regionctx::SimulationMode::Realtime => true,
            crate::server::regionctx::SimulationMode::TurnBased
            | crate::server::regionctx::SimulationMode::Hybrid => {
                self.grant_simulation_steps_if_due(ctx);
                if self.pending_system_steps == 0 {
                    return false;
                }
                self.pending_system_steps -= 1;
                true
            }
        }
    }

    fn action_requests_simulation_step(action: &EntityAction) -> bool {
        !matches!(action, EntityAction::Off | EntityAction::Intent(_))
    }

    fn has_active_continuous_motion(ctx: &RegionCtx) -> bool {
        ctx.map.entities.iter().any(|entity| {
            matches!(
                entity.action,
                EntityAction::StepTo(_, _, _, _, _) | EntityAction::RotateTo(_)
            )
        })
    }

    fn entity_has_active_continuous_motion(entity: &Entity) -> bool {
        matches!(
            entity.action,
            EntityAction::StepTo(_, _, _, _, _) | EntityAction::RotateTo(_)
        )
    }

    fn simulation_dt_for_frame(&mut self, ctx: &RegionCtx, redraw_dt: f32) -> f32 {
        match ctx.simulation_mode {
            crate::server::regionctx::SimulationMode::Realtime => {
                self.current_frame_has_turn_step = true;
                self.last_simulation_advance_at = Instant::now();
                redraw_dt
            }
            crate::server::regionctx::SimulationMode::TurnBased => {
                self.grant_simulation_steps_if_due(ctx);
                if self.pending_redraw_steps == 0 {
                    if Self::has_active_continuous_motion(ctx) {
                        self.current_frame_has_turn_step = false;
                        redraw_dt
                    } else {
                        self.current_frame_has_turn_step = false;
                        0.0
                    }
                } else {
                    self.pending_redraw_steps -= 1;
                    self.current_frame_has_turn_step = true;
                    redraw_dt
                }
            }
            crate::server::regionctx::SimulationMode::Hybrid => {
                self.grant_simulation_steps_if_due(ctx);
                if self.pending_redraw_steps == 0 {
                    if Self::has_active_continuous_motion(ctx) {
                        self.current_frame_has_turn_step = false;
                        redraw_dt
                    } else {
                        self.current_frame_has_turn_step = false;
                        0.0
                    }
                } else {
                    self.pending_redraw_steps -= 1;
                    self.current_frame_has_turn_step = true;
                    redraw_dt
                }
            }
        }
    }

    fn grid_hold_speed(entity: &Entity) -> f32 {
        entity
            .attributes
            .get_float_default("hold_speed", Self::grid_press_speed(entity))
            .max(0.01)
    }

    fn queue_step_to_with_speed(
        &self,
        entity: &mut Entity,
        target: Vec2<f32>,
        facing: Vec2<f32>,
        speed: f32,
    ) {
        let facing = Self::snapped_cardinal_direction(facing);
        let start = entity.get_pos_xz();
        let target = Self::snapped_grid_center(target);
        entity.set_orientation(facing);
        let step_dir = target - Self::snapped_grid_center(start);
        entity.action = EntityAction::StepTo(target, speed, facing, start, step_dir);
    }

    fn queue_step_to(&self, entity: &mut Entity, target: Vec2<f32>, facing: Vec2<f32>) {
        self.queue_step_to_with_speed(entity, target, facing, Self::grid_press_speed(entity));
    }

    fn rotate_grid_left(&self, entity: &mut Entity) {
        let facing = Self::snapped_cardinal_direction(entity.orientation);
        let target = Vec2::new(facing.y, -facing.x);
        entity.action = EntityAction::RotateTo(target);
    }

    fn rotate_grid_right(&self, entity: &mut Entity) {
        let facing = Self::snapped_cardinal_direction(entity.orientation);
        let target = Vec2::new(-facing.y, facing.x);
        entity.action = EntityAction::RotateTo(target);
    }

    fn grid_desired_action(entity: &Entity) -> EntityAction {
        entity
            .attributes
            .get_str_default("__grid_desired_action", "none".into())
            .parse()
            .unwrap_or(EntityAction::Off)
    }

    fn set_grid_desired_action(entity: &mut Entity, action: &EntityAction) {
        entity.set_attribute("__grid_desired_action", Value::Str(action.to_string()));
    }

    fn clear_grid_blocked_action(entity: &mut Entity) {
        entity.set_attribute("__grid_blocked_action", Value::Str("none".into()));
    }

    fn blocked_grid_action(entity: &Entity) -> EntityAction {
        entity
            .attributes
            .get_str_default("__grid_blocked_action", "none".into())
            .parse()
            .unwrap_or(EntityAction::Off)
    }

    fn set_blocked_grid_action(entity: &mut Entity, action: &EntityAction) {
        entity.set_attribute("__grid_blocked_action", Value::Str(action.to_string()));
    }

    fn activate_grid_desired_action(&self, entity: &mut Entity) {
        let desired = Self::grid_desired_action(entity);
        let blocked = Self::blocked_grid_action(entity);
        if !matches!(
            desired,
            EntityAction::Off
                | EntityAction::Left
                | EntityAction::Forward
                | EntityAction::Right
                | EntityAction::Backward
                | EntityAction::StrafeLeft
                | EntityAction::StrafeRight
        ) || desired == blocked
        {
            entity.action = EntityAction::Off;
            return;
        }

        entity.action = desired;
    }

    fn queue_grid_action_from_desired(
        &self,
        entity: &mut Entity,
        player_camera: &PlayerCamera,
    ) -> bool {
        let desired = Self::grid_desired_action(entity);
        let blocked = Self::blocked_grid_action(entity);
        if desired == EntityAction::Off || desired == blocked {
            entity.action = EntityAction::Off;
            return false;
        }

        match desired {
            EntityAction::Forward => {
                if Self::is_first_person_camera(player_camera) {
                    let facing = Self::snapped_cardinal_direction(entity.orientation);
                    let target = entity.get_pos_xz() + facing;
                    self.queue_step_to_with_speed(
                        entity,
                        target,
                        facing,
                        Self::grid_hold_speed(entity),
                    );
                } else {
                    entity.face_north();
                    let target = entity.get_pos_xz() + Vec2::new(0.0, -1.0);
                    self.queue_step_to_with_speed(
                        entity,
                        target,
                        Vec2::new(0.0, -1.0),
                        Self::grid_hold_speed(entity),
                    );
                }
                true
            }
            EntityAction::Backward => {
                if Self::is_first_person_camera(player_camera) {
                    let facing = Self::snapped_cardinal_direction(entity.orientation);
                    let target = entity.get_pos_xz() - facing;
                    self.queue_step_to_with_speed(
                        entity,
                        target,
                        facing,
                        Self::grid_hold_speed(entity),
                    );
                } else {
                    entity.face_south();
                    let target = entity.get_pos_xz() + Vec2::new(0.0, 1.0);
                    self.queue_step_to_with_speed(
                        entity,
                        target,
                        Vec2::new(0.0, 1.0),
                        Self::grid_hold_speed(entity),
                    );
                }
                true
            }
            EntityAction::Left => {
                if Self::is_first_person_camera(player_camera) {
                    self.rotate_grid_left(entity);
                } else {
                    entity.face_west();
                    let target = entity.get_pos_xz() + Vec2::new(-1.0, 0.0);
                    self.queue_step_to_with_speed(
                        entity,
                        target,
                        Vec2::new(-1.0, 0.0),
                        Self::grid_hold_speed(entity),
                    );
                }
                true
            }
            EntityAction::Right => {
                if Self::is_first_person_camera(player_camera) {
                    self.rotate_grid_right(entity);
                } else {
                    entity.face_east();
                    let target = entity.get_pos_xz() + Vec2::new(1.0, 0.0);
                    self.queue_step_to_with_speed(
                        entity,
                        target,
                        Vec2::new(1.0, 0.0),
                        Self::grid_hold_speed(entity),
                    );
                }
                true
            }
            EntityAction::StrafeLeft => {
                if Self::is_first_person_camera(player_camera) {
                    let facing = Self::snapped_cardinal_direction(entity.orientation);
                    let step = Vec2::new(facing.y, -facing.x);
                    let target = entity.get_pos_xz() + step;
                    self.queue_step_to_with_speed(
                        entity,
                        target,
                        facing,
                        Self::grid_hold_speed(entity),
                    );
                    true
                } else {
                    entity.action = EntityAction::Off;
                    false
                }
            }
            EntityAction::StrafeRight => {
                if Self::is_first_person_camera(player_camera) {
                    let facing = Self::snapped_cardinal_direction(entity.orientation);
                    let step = Vec2::new(-facing.y, facing.x);
                    let target = entity.get_pos_xz() + step;
                    self.queue_step_to_with_speed(
                        entity,
                        target,
                        facing,
                        Self::grid_hold_speed(entity),
                    );
                    true
                } else {
                    entity.action = EntityAction::Off;
                    false
                }
            }
            _ => {
                entity.action = EntityAction::Off;
                false
            }
        }
    }

    fn update_grid_input_state(entity: &mut Entity, action: &EntityAction) {
        Self::set_grid_desired_action(entity, action);
        if *action == EntityAction::Off || *action != Self::blocked_grid_action(entity) {
            Self::clear_grid_blocked_action(entity);
        }
    }

    fn parse_vec2_attr(entity: &Entity, key: &str) -> Option<Vec2<f32>> {
        let raw = entity
            .attributes
            .get_str(key)
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        let (x, y) = raw.split_once(',')?;
        Some(Vec2::new(
            x.trim().parse::<f32>().ok()?,
            y.trim().parse::<f32>().ok()?,
        ))
    }

    fn compute_grid_goto_step_in_ctx(
        &self,
        ctx: &RegionCtx,
        position: Vec2<f32>,
        target: Vec2<f32>,
    ) -> Option<(Vec2<f32>, Vec2<f32>, Vec2<f32>)> {
        let position_cell = position.map(|value| value.floor());
        let target_cell = target.map(|value| value.floor());
        let anchor = position - position_cell;
        let target_pos = target_cell + anchor;

        if (target_pos - position).magnitude_squared() <= 0.001 {
            return None;
        }

        let from_tile = position_cell.map(|value| value as i32);
        let to_tile = target_cell.map(|value| value as i32);
        let blocked = &ctx.mapmini.blocked_tiles;
        let manhattan = (to_tile - from_tile).map(|x| x.abs()).sum();
        let padding = manhattan.clamp(8, 128);
        let min_bound = Vec2::new(
            from_tile.x.min(to_tile.x) - padding,
            from_tile.y.min(to_tile.y) - padding,
        );
        let max_bound = Vec2::new(
            from_tile.x.max(to_tile.x) + padding,
            from_tile.y.max(to_tile.y) + padding,
        );
        let successors = |pos: &Vec2<i32>| {
            [
                Vec2::new(-1, 0),
                Vec2::new(1, 0),
                Vec2::new(0, -1),
                Vec2::new(0, 1),
            ]
            .iter()
            .map(|d| *pos + *d)
            .filter(|p| {
                p.x >= min_bound.x && p.x <= max_bound.x && p.y >= min_bound.y && p.y <= max_bound.y
            })
            .filter(|p| !blocked.contains(p))
            .map(|p| (p, 1))
            .collect::<Vec<_>>()
        };
        let heuristic = |a: &Vec2<i32>| (to_tile - *a).map(|x| x.abs()).sum();
        let next_tile =
            astar(&from_tile, successors, heuristic, |p| *p == to_tile).and_then(|(path, _)| {
                if path.len() >= 2 { Some(path[1]) } else { None }
            });

        let Some(next_tile) = next_tile else {
            return None;
        };

        let next = next_tile.map(|value| value as f32) + anchor;
        let step = next - position;
        if step.magnitude_squared() <= 0.001 {
            return None;
        }

        let facing = Self::snapped_cardinal_direction(step);
        Some((next, facing, target_pos))
    }

    fn rotate_towards_cardinal(entity: &mut Entity, target: Vec2<f32>, step_deg: f32) -> bool {
        let current = if entity.orientation.magnitude_squared() <= 1e-6 {
            Vec2::new(1.0, 0.0)
        } else {
            entity.orientation.normalized()
        };
        let target = Self::snapped_cardinal_direction(target);
        let current_angle = current.y.atan2(current.x);
        let target_angle = target.y.atan2(target.x);
        let mut delta = target_angle - current_angle;
        while delta > std::f32::consts::PI {
            delta -= std::f32::consts::TAU;
        }
        while delta < -std::f32::consts::PI {
            delta += std::f32::consts::TAU;
        }

        if delta.abs() <= step_deg.to_radians() {
            entity.set_orientation(target);
            true
        } else {
            let angle = current_angle + step_deg.to_radians() * delta.signum();
            entity.set_orientation(Vec2::new(angle.cos(), angle.sin()).normalized());
            false
        }
    }

    fn move_entity_by_vector(
        &self,
        entity: &mut Entity,
        move_vector: Vec2<f32>,
        entity_block_mode: i32,
    ) -> bool {
        self.move_entity_by_vector_with_result(entity, move_vector, entity_block_mode)
            .geometry_blocked
    }

    fn move_entity_by_vector_with_result(
        &self,
        entity: &mut Entity,
        move_vector: Vec2<f32>,
        entity_block_mode: i32,
    ) -> MovementResult {
        with_regionctx(self.id, |ctx| {
            self.move_entity_by_vector_with_result_in_ctx(
                ctx,
                entity,
                move_vector,
                entity_block_mode,
            )
        })
        .unwrap()
    }

    fn move_entity_by_vector_with_result_in_ctx(
        &self,
        ctx: &mut RegionCtx,
        entity: &mut Entity,
        move_vector: Vec2<f32>,
        entity_block_mode: i32,
    ) -> MovementResult {
        let position = entity.get_pos_xz();
        let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;

        let mut new_position = position + move_vector;
        let mut dynamic_collision = false;

        const MAX_ITERATIONS: usize = 5;

        for _attempt in 0..MAX_ITERATIONS {
            let mut pushed = false;

            for other in ctx.map.entities.iter() {
                if other.id == entity.id || other.get_mode() == "dead" {
                    continue;
                }

                let other_pos = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
                let combined_radius = radius + other_radius;
                let combined_radius_sq = combined_radius * combined_radius;

                let dist_vec = new_position - other_pos;
                let dist_sq = dist_vec.magnitude_squared();
                if dist_sq < combined_radius_sq {
                    dynamic_collision = true;
                    if let Some(_class_name) = ctx.entity_classes.get(&entity.id) {
                        ctx.to_execute_entity.push((
                            entity.id,
                            "bumped_into_entity".into(),
                            VMValue::broadcast(other.id as f32),
                        ));
                    }
                    if let Some(_class_name) = ctx.entity_classes.get(&other.id) {
                        ctx.to_execute_entity.push((
                            other.id,
                            "bumped_by_entity".into(),
                            VMValue::broadcast(entity.id as f32),
                        ));
                    }

                    if entity_block_mode > 0 {
                        let normal = dist_vec.normalized();
                        let total_move = new_position - position;
                        let slide = total_move - normal * total_move.dot(normal);
                        let slide_pos = position + slide;
                        let slide_dist_sq = (slide_pos - other_pos).magnitude_squared();

                        if slide_dist_sq >= combined_radius_sq {
                            new_position = slide_pos;
                        } else {
                            let actual_dist = (slide_pos - other_pos).magnitude();
                            if actual_dist < combined_radius {
                                let push_amount = combined_radius - actual_dist;
                                new_position = slide_pos + normal * push_amount;
                            }
                        }
                        pushed = true;
                    }
                }
            }

            for other in ctx.map.items.iter() {
                if !other.attributes.get_bool_default("visible", false) {
                    continue;
                }

                let other_pos = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
                let combined_radius = radius + other_radius;
                let combined_radius_sq = combined_radius * combined_radius;

                let dist_vec = new_position - other_pos;
                let dist_sq = dist_vec.magnitude_squared();
                if dist_sq < combined_radius_sq {
                    dynamic_collision = true;
                    if let Some(_class_name) = ctx.entity_classes.get(&entity.id) {
                        ctx.to_execute_entity.push((
                            entity.id,
                            "bumped_into_item".into(),
                            VMValue::broadcast(other.id as f32),
                        ));
                    }
                    if let Some(_class_name) = ctx.item_classes.get(&other.id) {
                        ctx.to_execute_item.push((
                            other.id,
                            "bumped_by_entity".into(),
                            VMValue::broadcast(entity.id as f32),
                        ));
                    }

                    if other.attributes.get_bool_default("blocking", false) {
                        let normal = dist_vec.normalized();
                        let total_move = new_position - position;
                        let slide = total_move - normal * total_move.dot(normal);
                        let slide_pos = position + slide;
                        let slide_dist_sq = (slide_pos - other_pos).magnitude_squared();

                        if slide_dist_sq >= combined_radius_sq {
                            new_position = slide_pos;
                        } else {
                            let actual_dist = (slide_pos - other_pos).magnitude();
                            if actual_dist < combined_radius {
                                let push_amount = combined_radius - actual_dist;
                                new_position = slide_pos + normal * push_amount;
                            }
                        }
                        pushed = true;
                    }
                }
            }

            if !pushed {
                break;
            }
        }

        entity.set_pos_xz(new_position);

        let blocked = match self.collision_mode {
            CollisionMode::Tile => {
                let (end_position, geometry_blocked) =
                    ctx.mapmini
                        .move_distance(position, new_position - position, radius);
                entity.set_pos_xz(end_position);
                geometry_blocked
            }
            CollisionMode::Mesh => {
                if ctx.collision_world.has_collision_data() {
                    let move_vec = new_position - position;
                    let desired_dist = move_vec.magnitude();
                    if desired_dist > 1e-6 {
                        if let Some((end_pos, arrived)) =
                            ctx.collision_world.move_towards_on_floors_direct(
                                position,
                                new_position,
                                desired_dist,
                                radius,
                                1.0,
                                entity.position.y,
                            )
                        {
                            entity.set_pos_xz(vek::Vec2::new(end_pos.x, end_pos.z));
                            entity.position.y = end_pos.y;
                            !arrived
                        } else {
                            let start_pos =
                                vek::Vec3::new(position.x, entity.position.y, position.y);
                            let move_vec_3d = vek::Vec3::new(move_vec.x, 0.0, move_vec.y);
                            let (collision_pos, blocked) =
                                ctx.collision_world
                                    .move_distance(start_pos, move_vec_3d, radius);
                            entity.set_pos_xz(vek::Vec2::new(collision_pos.x, collision_pos.z));
                            blocked
                        }
                    } else {
                        false
                    }
                } else {
                    let (end_position, geometry_blocked) =
                        ctx.mapmini
                            .move_distance(position, new_position - position, radius);
                    entity.set_pos_xz(end_position);
                    geometry_blocked
                }
            }
        };

        let final_pos = entity.get_pos_xz();
        let mut base_y = None;
        if self.collision_mode == CollisionMode::Mesh && ctx.collision_world.has_collision_data() {
            base_y =
                ctx.collision_world
                    .get_floor_height_reachable(final_pos, entity.position.y, 1.0);
        }
        if base_y.is_none() {
            let config = crate::chunkbuilder::terrain_generator::TerrainConfig::default();
            base_y = Some(
                crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                    &ctx.map, final_pos, &config,
                ),
            );
        }

        if let Some(y) = base_y {
            entity.position.y = y;
        }

        ctx.check_player_for_section_change(entity);
        MovementResult {
            geometry_blocked: blocked,
            dynamic_collision,
        }
    }

    fn run_instance_setup_program(
        &mut self,
        source: &str,
        current_entity_id: Option<u32>,
        current_item_id: Option<u32>,
    ) -> Result<bool, String> {
        let program = self.vm.prepare_str(source).map_err(|err| err.to_string())?;

        Ok(with_regionctx(self.id, |ctx| {
            let prev_entity_id = ctx.curr_entity_id;
            let prev_item_id = ctx.curr_item_id;

            if let Some(entity_id) = current_entity_id {
                ctx.curr_entity_id = entity_id;
            }
            if current_item_id.is_some() || current_entity_id.is_none() {
                ctx.curr_item_id = current_item_id;
            }

            let ran = run_server_named_fn(&mut self.exec, "setup", &[], &program, ctx);

            ctx.curr_entity_id = prev_entity_id;
            ctx.curr_item_id = prev_item_id;

            ran
        })
        .unwrap_or(false))
    }

    fn is_legacy_python_instance_setup(source: &str) -> bool {
        source.trim_start().starts_with("def setup")
    }

    fn run_entity_instance_setup(&mut self, entity: &Entity, region_name: &str, context: &str) {
        if let Some(setup) = entity.get_attr_string("setup")
            && !setup.trim().is_empty()
        {
            if Self::is_legacy_python_instance_setup(&setup) {
                send_log_message(
                    self.id,
                    format!(
                        "{}: Ignoring legacy Python setup on '{}/{}' {}.",
                        region_name,
                        entity.get_attr_string("name").unwrap_or("Unknown".into()),
                        entity
                            .get_attr_string("class_name")
                            .unwrap_or("Unknown".into()),
                        context,
                    ),
                );
                return;
            }
            match self.run_instance_setup_program(&setup, Some(entity.id), None) {
                Ok(_) => {}
                Err(err) => {
                    send_log_message(
                        self.id,
                        format!(
                            "[error] {}: Setup '{}/{}' {}: {}",
                            region_name,
                            entity.get_attr_string("name").unwrap_or("Unknown".into()),
                            entity
                                .get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            context,
                            err,
                        ),
                    );
                    with_regionctx(self.id, |ctx| {
                        ctx.error_count += 1;
                    });
                }
            }
        }
    }

    fn run_item_instance_setup(&mut self, item: &Item, region_name: &str, context: &str) {
        if let Some(setup) = item.get_attr_string("setup")
            && !setup.trim().is_empty()
        {
            if Self::is_legacy_python_instance_setup(&setup) {
                send_log_message(
                    self.id,
                    format!(
                        "{}: Ignoring legacy Python item setup on '{}/{}' {}.",
                        region_name,
                        item.get_attr_string("name").unwrap_or("Unknown".into()),
                        item.get_attr_string("class_name")
                            .unwrap_or("Unknown".into()),
                        context,
                    ),
                );
                return;
            }
            match self.run_instance_setup_program(&setup, None, Some(item.id)) {
                Ok(_) => {}
                Err(err) => {
                    send_log_message(
                        self.id,
                        format!(
                            "[error] {}: Item Setup '{}/{}' {}: {}",
                            region_name,
                            item.get_attr_string("name").unwrap_or("Unknown".into()),
                            item.get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            context,
                            err,
                        ),
                    );
                    with_regionctx(self.id, |ctx| {
                        ctx.error_count += 1;
                    });
                }
            }
        }
    }

    pub fn new(region_id: u32) -> Self {
        let (to_sender, to_receiver) = unbounded::<RegionMessage>();
        let (from_sender, from_receiver) = unbounded::<RegionMessage>();

        Self {
            id: region_id,

            vm: VM::default(),
            exec: Execution::default(),

            name: String::new(),

            to_receiver,
            to_sender,
            from_receiver,
            from_sender,

            entity_block_mode: 0,
            collision_mode: CollisionMode::Tile,
            last_redraw_at: Instant::now(),
            last_simulation_advance_at: Instant::now(),
            last_external_step_request_at: Instant::now() - Duration::from_secs(1),
            current_frame_has_turn_step: false,
            simulation_step_pending: false,
            pending_system_steps: 0,
            pending_redraw_steps: 0,
            movement_units_per_sec: 4.0,
        }
    }

    /// Initializes the Python bases classes, sets the map and applies entities
    pub fn init(
        &mut self,
        name: String,
        map: Map,
        assets: &Assets,
        config_toml: String,
        debug_mode: bool,
    ) {
        self.name = name.clone();

        let mut ctx = RegionCtx::default();
        ctx.debug_mode = debug_mode;

        if let Ok(toml) = config_toml.parse::<toml::Table>() {
            ctx.config = toml;
        }
        if !assets.rules.trim().is_empty() {
            match assets.rules.parse::<toml::Table>() {
                Ok(toml) => ctx.rules = toml,
                Err(err) => ctx
                    .startup_errors
                    .push(format!("[warning] {}: Game Rules: {}", self.name, err)),
            }
        }

        ctx.map = map;
        ctx.blocking_tiles = assets.blocking_tiles();
        ctx.assets = assets.clone();

        if !assets.world_source.trim().is_empty() {
            match self.vm.prepare_str(&assets.world_source) {
                Ok(program) => ctx.world_program = Some(Arc::new(program)),
                Err(error) => ctx.startup_errors.push(format!(
                    "[error] {}: Compiling World Script: {}",
                    self.name, error
                )),
            }
        }

        if let Some(region_source) = assets.region_sources.get(&ctx.map.id)
            && !region_source.trim().is_empty()
        {
            match self.vm.prepare_str(region_source) {
                Ok(program) => ctx.region_program = Some(Arc::new(program)),
                Err(error) => ctx.startup_errors.push(format!(
                    "[error] {}: Compiling Region Script: {}",
                    self.name, error
                )),
            }
        }

        // Installing currencies

        _ = ctx.currencies.add_currency(Currency {
            name: "Gold".into(),
            symbol: "G".into(),
            exchange_rate: 1.0,
            max_limit: None,
        });
        ctx.currencies.base_currency = "G".to_string();

        // Compile Entity Template Scripts
        for (name, (entity_source, entity_data)) in &assets.entities {
            match self.vm.prepare_str(entity_source) {
                Ok(program) => {
                    ctx.entity_programs
                        .insert(name.clone(), std::sync::Arc::new(program));
                }
                Err(error) => {
                    ctx.startup_errors.push(format!(
                        "[error] {}: Compiling Character '{}': {}",
                        self.name,
                        name,
                        error.to_string(),
                    ));
                }
            }

            // Store entity classes which handle player
            match entity_data.parse::<toml::Table>() {
                Ok(data) => {
                    if let Some(game) = data.get("attributes").and_then(toml::Value::as_table) {
                        if let Some(value) = game.get("player") {
                            if let Some(v) = value.as_bool() {
                                if v {
                                    ctx.entity_player_classes.insert(name.clone());
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    ctx.startup_errors.push(format!(
                        "[error] {}: Character Attributes '{}': {}",
                        self.name, name, err,
                    ));
                }
            }

            ctx.entity_class_data
                .insert(name.clone(), entity_data.clone());
            if let Some(authoring) = assets.entity_authoring.get(name) {
                ctx.entity_authoring_data
                    .insert(name.clone(), authoring.clone());
            }
        }

        /*
        // Installing Entity Class Templates
        for (name, (entity_source, entity_data)) in &assets.entities {
            if let Err(err) = self.execute(entity_source) {
                ctx.startup_errors.push(format!(
                    "{}: Error Compiling {} Character Class: {}",
                    self.name, name, err,
                ));
            }
            if let Err(err) = self.execute(&format!("{} = {}()", name, name)) {
                ctx.startup_errors.push(format!(
                    "{}: Error Installing {} Character Class: {}",
                    self.name, name, err,
                ));
            }

            // Store entity classes which handle player
            match entity_data.parse::<toml::Table>() {
                Ok(data) => {
                    if let Some(game) = data.get("attributes").and_then(toml::Value::as_table) {
                        if let Some(value) = game.get("player") {
                            if let Some(v) = value.as_bool() {
                                if v {
                                    ctx.entity_player_classes.insert(name.clone());
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    ctx.startup_errors.push(format!(
                        "{}: Error Parsing {} Entity Class: {}",
                        self.name, name, err,
                    ));
                }
            }

            ctx.entity_class_data
                .insert(name.clone(), entity_data.clone());
        }*/

        // Installing Item Class Templates
        for (name, (item_source, item_data)) in &assets.items {
            match self.vm.prepare_str(item_source) {
                Ok(program) => {
                    ctx.item_programs
                        .insert(name.clone(), std::sync::Arc::new(program));
                }
                Err(error) => {
                    ctx.startup_errors.push(format!(
                        "[error] {}: Compiling Item '{}': {}",
                        self.name,
                        name,
                        error.to_string(),
                    ));
                }
            }

            // if let Err(err) = self.execute(item_source) {
            //     ctx.startup_errors.push(format!(
            //         "{}: Error Compiling {} Item Class: {}",
            //         self.name, name, err,
            //     ));
            // }
            // if let Err(err) = self.execute(&format!("{} = {}()", name, name)) {
            //     ctx.startup_errors.push(format!(
            //         "{}: Error Installing {} Item Class: {}",
            //         self.name, name, err,
            //     ));
            // }
            ctx.item_class_data.insert(name.clone(), item_data.clone());
            if let Some(authoring) = assets.item_authoring.get(name) {
                ctx.item_authoring_data
                    .insert(name.clone(), authoring.clone());
            }
        }

        // Remove player based entities, these only get created on demand from a client
        let player_classes = ctx.entity_player_classes.clone();
        ctx.map
            .entities
            .retain(|entity| match entity.get_attr_string("class_name") {
                Some(class_name) => !player_classes.contains(&class_name),
                None => true,
            });

        // Set an entity id and mark all fields dirty for the first transmission to the server.
        for e in ctx.map.entities.iter_mut() {
            e.id = get_global_id();
            // By default we set the sequence to idle.
            e.set_attribute(
                "_source_seq",
                Value::Source(PixelSource::Sequence("idle".into())),
            );
            e.set_attribute("mode", Value::Str("active".into()));
            e.mark_all_dirty();
        }

        // Set an item id and mark all fields dirty for the first transmission to the server.
        for i in ctx.map.items.iter_mut() {
            i.id = get_global_id();
            // By default we set the sequence to idle.
            i.attributes.set(
                "_source_seq",
                Value::Source(PixelSource::Sequence("_".into())),
            );
            i.mark_all_dirty();
        }

        // Create Items for Sectors
        let mut created_door_groups = std::collections::HashSet::new();
        for s in &ctx.map.sectors {
            if let Some(item_name) = s.properties.get_str("item") {
                if item_name.is_empty() {
                    continue;
                }
                if let Some(group_id) = s.properties.get_id("door_group_id")
                    && !created_door_groups.insert(group_id)
                {
                    continue;
                }
                if ctx.item_programs.contains_key(item_name) {
                    let mut item = Item::default();
                    item.id = get_global_id();
                    item.attributes.set("name", Value::Str(s.name.to_string()));
                    item.attributes
                        .set("class_name", Value::Str(item_name.to_string()));
                    item.attributes.set("static", Value::Bool(true));
                    item.attributes.set("sector_id", Value::UInt(s.id));
                    if let Some(group_id) = s.properties.get_id("door_group_id") {
                        item.attributes.set("door_group_id", Value::Id(group_id));
                    }
                    if let Some(mode) = s.properties.get_str("dungeon_door_mode") {
                        item.attributes
                            .set("door_mode", Value::Str(mode.to_string()));
                    }
                    if let Some(depth) = s.properties.get_float("dungeon_door_depth") {
                        item.attributes.set("door_depth", Value::Float(depth));
                    }
                    if let Some(height) = s.properties.get_float("dungeon_door_height") {
                        item.attributes.set("door_height", Value::Float(height));
                    }
                    item.attributes.set(
                        "blocking",
                        Value::Bool(s.properties.get_bool_default("blocking", true)),
                    );
                    if let Some(center) = s.center(&ctx.map) {
                        let world_y = s
                            .vertices_world(&ctx.map)
                            .map(|verts| {
                                verts.iter().map(|v| v.y).sum::<f32>() / verts.len() as f32
                            })
                            .unwrap_or(0.0);
                        item.set_position(Vec3::new(center.x, world_y, center.y));
                    }
                    item.mark_all_dirty();
                    ctx.map.items.push(item);
                } else {
                    ctx.startup_errors.push(format!(
                        "[error] {}: Sector Item '{}': Item does not exist '{}'",
                        self.name, name, item_name
                    ));
                }
            }
        }

        // Create Items for Profile Sectors (Doors, Gates)
        for (_, surface) in ctx.map.surfaces.iter_mut() {
            if let Some(profile_id) = surface.profile {
                if let Some(map) = ctx.map.profiles.get_mut(&profile_id) {
                    for s in &map.sectors {
                        if let Some(item_name) = s.properties.get_str("item") {
                            if item_name.is_empty() {
                                continue;
                            }

                            // Check if the given class name exists
                            if ctx.item_programs.contains_key(item_name) {
                                let mut item = Item::default();
                                item.id = get_global_id();
                                item.attributes.set("name", Value::Str(s.name.to_string()));
                                item.attributes
                                    .set("class_name", Value::Str(item_name.to_string()));
                                item.attributes.set("static", Value::Bool(true));
                                item.attributes
                                    .set("profile_host_sector_id", Value::UInt(surface.sector_id));
                                item.attributes.set("profile_sector_id", Value::UInt(s.id));
                                if let Some(pos) = s.center(map) {
                                    // Profile space uses -Y up; flip to UV and map onto the surface.
                                    let uv = Vec2::new(pos.x, -pos.y);
                                    let world_pos = surface.uv_to_world(uv);
                                    item.set_position(world_pos);
                                }
                                item.mark_all_dirty();
                                ctx.map.items.push(item);
                            } else {
                                ctx.startup_errors.push(format!(
                                    "[error] {}: Profile Sector Item '{}': Item does not exist '{}'",
                                    self.name,
                                    name,
                                    item_name
                                ));
                            }
                        }
                    }
                }
            }
        }

        // --- Startup

        ctx.from_sender.set(self.from_sender.clone()).unwrap();
        ctx.to_receiver.set(self.to_receiver.clone()).unwrap();
        ctx.region_id = self.id;
        ctx.mapmini = ctx.map.as_mini(&ctx.blocking_tiles);

        // Build collision geometry for all chunks (new collision system)
        use crate::chunkbuilder::{ChunkBuilder, d3chunkbuilder::D3ChunkBuilder};
        let mut chunk_builder = D3ChunkBuilder::new();
        let chunk_size = 10; // Match collision_world chunk size

        // Calculate chunk bounds from full map extents, not only surfaces.
        // Feature collisions (e.g. palisade/fence on linedefs) can extend beyond sector surfaces.
        let world_bbox = if ctx.map.vertices.is_empty() {
            None
        } else {
            Some(ctx.map.bbox())
        };
        if let Some(bbox) = world_bbox {
            let min_chunk = vek::Vec2::new(
                (bbox.min.x / chunk_size as f32).floor() as i32,
                (bbox.min.y / chunk_size as f32).floor() as i32,
            );
            let max_chunk = vek::Vec2::new(
                (bbox.max.x / chunk_size as f32).floor() as i32,
                (bbox.max.y / chunk_size as f32).floor() as i32,
            );

            // Build collision for each chunk
            for cy in min_chunk.y..=max_chunk.y {
                for cx in min_chunk.x..=max_chunk.x {
                    let chunk_origin = vek::Vec2::new(cx, cy);
                    let chunk_collision = chunk_builder.build_collision(
                        &ctx.map,
                        &ctx.assets,
                        chunk_origin,
                        chunk_size,
                    );

                    ctx.collision_world
                        .update_chunk(chunk_origin, chunk_collision);
                }
            }
        }

        ctx.ticks = 0;

        ctx.ticks_per_minute = 4;
        ctx.ticks_per_minute = get_config_i32_default(&ctx, "game", "ticks_per_minute", 4) as u32;
        ctx.simulation_mode = crate::server::regionctx::SimulationMode::from_config_value(
            &get_config_string_default(&ctx, "game", "simulation_mode", "realtime"),
        );
        ctx.turn_timeout_ms =
            get_config_i32_default(&ctx, "game", "turn_timeout_ms", 600).max(0) as u32;

        let target_fps = get_config_i32_default(&ctx, "game", "target_fps", 30).max(1) as f32;
        ctx.delta_time = 1.0 / target_fps;
        ctx.health_attr = get_config_string_default(&ctx, "game", "health", "HP").to_string();
        ctx.level_attr = get_config_string_default(&ctx, "game", "level", "LEVEL").to_string();
        ctx.experience_attr =
            get_config_string_default(&ctx, "game", "experience", "EXP").to_string();

        self.entity_block_mode = {
            let mode = get_config_string_default(&ctx, "game", "entity_block_mode", "always");
            if mode == "always" { 1 } else { 0 }
        };
        self.collision_mode = {
            let mode = get_config_string_default(&ctx, "game", "collision_mode", "tile");
            if mode.eq_ignore_ascii_case("mesh") {
                CollisionMode::Mesh
            } else {
                CollisionMode::Tile
            }
        };
        self.movement_units_per_sec =
            get_config_i32_default(&ctx, "game", "movement_units_per_sec", 4).max(1) as f32;

        let entities: Vec<Entity> = ctx.map.entities.clone();

        // Setting the data for the entities.
        for entity in entities.iter() {
            if let Some(class_name) = entity.get_attr_string("class_name") {
                if let Some(data) = ctx.entity_class_data.get(&class_name) {
                    let ground_y =
                        map_spawn_height(&ctx.map, entity.get_pos_xz(), Some(entity.position.y));
                    let mut spawn_entity_id: Option<u32> = None;
                    for e in ctx.map.entities.iter_mut() {
                        if e.id == entity.id {
                            apply_entity_data(e, data);
                            e.position.y = ground_y;

                            // Fill up the inventory slots
                            if let Some(Value::Int(inv_slots)) = e.attributes.get("inventory_slots")
                            {
                                e.inventory = vec![];
                                for _ in 0..*inv_slots {
                                    e.inventory.push(None);
                                }
                            }

                            // Set the wallet
                            if let Some(Value::Int(wealth)) = e.attributes.get("wealth") {
                                _ = e.add_base_currency(*wealth as i64, &ctx.currencies)
                            }
                            spawn_entity_id = Some(e.id);
                        }
                    }
                    if let Some(spawn_entity_id) = spawn_entity_id {
                        apply_spawn_item_lists_for_entity(spawn_entity_id, &mut ctx);
                    }
                }
            }
        }

        // Register the ctx, from here on we have to lock it
        register_regionctx(self.id, Arc::new(Mutex::new(ctx)));

        with_regionctx(self.id, |ctx: &mut RegionCtx| {
            let args = [VMValue::from_string("startup"), VMValue::zero()];
            if let Some(program) = ctx.world_program.clone() {
                ctx.current_script_scope = ScriptScope::World;
                run_server_fn(&mut self.exec, &args, &program, ctx);
            }
            if let Some(program) = ctx.region_program.clone() {
                ctx.current_script_scope = ScriptScope::Region;
                run_server_fn(&mut self.exec, &args, &program, ctx);
            }
            ctx.current_script_scope = ScriptScope::Entity;
        });

        // Send "startup" event to all entities.
        for entity in entities.iter() {
            if let Some(class_name) = entity.get_attr_string("class_name") {
                // let cmd = format!("{}.event(\"startup\", \"\")", class_name);
                with_regionctx(self.id, |ctx: &mut RegionCtx| {
                    ctx.entity_classes.insert(entity.id, class_name.clone());
                    ctx.curr_entity_id = entity.id;

                    if let Some(program) = ctx.entity_programs.get(&class_name).cloned() {
                        let args = [VMValue::from_string("startup"), VMValue::zero()];
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                        flush_pending_entity_transfers(ctx);
                    }
                });

                // if let Err(err) = self.execute(&cmd) {
                //     send_log_message(
                //         self.id,
                //         format!(
                //             "{}: Event Error ({}) for '{}': {}",
                //             name,
                //             "startup",
                //             self.get_entity_name(entity.id),
                //             err,
                //         ),
                //     );
                // }

                // Determine, set and notify the entity about the sector it is in.
                let mut sector_name = String::new();
                with_regionctx(self.id, |ctx| {
                    if let Some(sector) = ctx.map.find_sector_at(entity.get_pos_xz()) {
                        sector_name = sector.name.clone();
                        let sector_id = sector.id;
                        for e in ctx.map.entities.iter_mut() {
                            if e.id == entity.id {
                                e.attributes.set("sector", Value::Str(sector_name.clone()));
                                e.attributes
                                    .set("sector_id", Value::Int64(sector_id as i64));
                            }
                        }
                    } else {
                        for e in ctx.map.entities.iter_mut() {
                            if e.id == entity.id {
                                e.attributes.set("sector", Value::Str(String::new()));
                                e.attributes.set("sector_id", Value::Int64(-1));
                            }
                        }
                    }

                    if !sector_name.is_empty() {
                        // let cmd = format!("{}.event(\"entered\", \"{}\")", class_name, sector_name);
                        // _ = self.execute(&cmd);
                        if let Some(program) = ctx.entity_programs.get(&class_name).cloned() {
                            let args = [
                                VMValue::from_string("entered"),
                                VMValue::from_string(sector_name),
                            ];
                            run_server_fn(&mut self.exec, &args, &program, ctx);
                            flush_pending_entity_transfers(ctx);
                        }
                    }
                });
            }
        }

        /*
        // Send "startup" event to all items.
        for item in items.iter() {
            if let Some(class_name) = item.get_attr_string("class_name") {
                with_regionctx(self.id, |ctx| {
                    ctx.item_classes.insert(item.id, class_name.clone());
                    ctx.curr_item_id = Some(item.id);

                    if let Some(program) = ctx.item_programs.get(&class_name).cloned() {
                        let args = [VMValue::from_string("startup"), VMValue::zero()];
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                    }
                });
                // if let Err(err) = self.execute(&cmd) {
                //     send_log_message(
                //         self.id,
                //         format!(
                //             "{}: Item Event Error ({}) for '{}': {}",
                //             name,
                //             "startup",
                //             self.get_entity_name(item.id),
                //             err,
                //         ),
                //     );
                // }
            }
        }
        with_regionctx(self.id, |ctx| {
            ctx.curr_item_id = None;
        });*/

        // Running the character setup scripts for the class instances
        for entity in entities.iter() {
            self.run_entity_instance_setup(entity, &name, "for instance");
        }

        // Running the item setup scripts for the class instances
        let mut items = vec![];
        with_regionctx(self.id, |ctx| {
            items = ctx.map.items.clone();
        });
        for item in items.iter_mut() {
            self.run_item_instance_setup(item, &name, "for instance");

            // Setting the data for the item.
            if let Some(class_name) = item.get_attr_string("class_name") {
                with_regionctx(self.id, |ctx| {
                    if let Some(data) = ctx.item_class_data.get(&class_name) {
                        for i in ctx.map.items.iter_mut() {
                            if i.id == item.id {
                                apply_item_data(i, data);
                                *item = i.clone();
                            }
                        }
                    }

                    let state = if item.attributes.get_bool_default("active", false) {
                        true
                    } else {
                        false
                    };

                    if let Some(program) = ctx.item_programs.get(&class_name).cloned() {
                        let args = [VMValue::from_string("active"), VMValue::from_bool(state)];
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                    }
                });

                // Send startup to all items
                with_regionctx(self.id, |ctx| {
                    ctx.item_classes.insert(item.id, class_name.clone());
                    ctx.curr_item_id = Some(item.id);

                    if let Some(program) = ctx.item_programs.get(&class_name).cloned() {
                        let args = [VMValue::from_string("startup"), VMValue::zero()];
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                    }
                });
            }
        }

        // Wrapping up ...
        let mut error_count = 0;
        with_regionctx(self.id, |ctx| {
            ctx.curr_item_id = None;

            // Send startup messages
            ctx.error_count = ctx.startup_errors.len() as u32;
            error_count = ctx.error_count;

            let messages = ctx.startup_errors.clone();
            for l in messages {
                ctx.send_log_message(l);
            }
        });

        // Send startup log message
        send_log_message(
            self.id,
            format!("{}: Startup with {} errors.", name, error_count),
        );
    }

    /// System tick
    pub fn system_tick(&mut self) {
        let mut ticks = 0;
        let mut should_advance = true;

        with_regionctx(self.id, |ctx| {
            if ctx.paused {
                should_advance = false;
                return;
            }
            if !self.consume_system_step_if_allowed(ctx) {
                should_advance = false;
                return;
            }
            if ctx.debug_mode {
                ctx.debug.clear_execution();
                ctx.curr_debug_loc = None;
            }
            ctx.ticks += 1;
            ticks = ctx.ticks;

            let mins = ctx.time.total_minutes();
            ctx.time = TheTime::from_ticks(ticks, ctx.ticks_per_minute);

            if ctx.time.total_minutes() > mins {
                // If the time changed send to server
                self.from_sender
                    .send(RegionMessage::Time(self.id, ctx.time))
                    .unwrap();

                // Broadcast a server-side `time` event to all characters and items
                // whenever we cross a full in-game hour.
                if ctx.time.minutes == 0 {
                    let hour_24 = ctx.time.hours as i32;

                    let entity_ids: Vec<u32> = ctx.entity_classes.keys().copied().collect();
                    for id in entity_ids {
                        ctx.to_execute_entity
                            .push((id, "time".into(), VMValue::from_i32(hour_24)));
                    }

                    let item_ids: Vec<u32> = ctx.item_classes.keys().copied().collect();
                    for id in item_ids {
                        ctx.to_execute_item
                            .push((id, "time".into(), VMValue::from_i32(hour_24)));
                    }

                    let args = [VMValue::from_string("time"), VMValue::from_i32(hour_24)];
                    if let Some(program) = ctx.world_program.clone() {
                        ctx.current_script_scope = ScriptScope::World;
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                    }
                    if let Some(program) = ctx.region_program.clone() {
                        ctx.current_script_scope = ScriptScope::Region;
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                    }
                }
            }

            let expired_sessions: Vec<(u32, u32)> = ctx
                .active_choice_sessions
                .iter()
                .filter(|session| {
                    !choice_session_is_valid(
                        ctx,
                        session.from,
                        session.to,
                        session.expires_at_tick,
                        session.max_distance,
                    )
                })
                .map(|session| (session.from, session.to))
                .collect();

            for (from_id, to_id) in expired_sessions {
                clear_choice_session(ctx, from_id, to_id);
                if ctx.entity_classes.contains_key(&from_id) {
                    ctx.to_execute_entity.push((
                        from_id,
                        "goodbye".into(),
                        VMValue::broadcast(to_id as f32),
                    ));
                }
            }

            let tick_args = [
                VMValue::from_string("tick"),
                VMValue::from_i32(ctx.ticks as i32),
            ];
            if let Some(program) = ctx.world_program.clone() {
                ctx.current_script_scope = ScriptScope::World;
                run_server_fn(&mut self.exec, &tick_args, &program, ctx);
            }
            if let Some(program) = ctx.region_program.clone() {
                ctx.current_script_scope = ScriptScope::Region;
                run_server_fn(&mut self.exec, &tick_args, &program, ctx);
            }
            ctx.current_script_scope = ScriptScope::Entity;
        });

        if !should_advance {
            return;
        }

        // Process notifications for entities.
        let to_process = {
            let mut notifications = vec![];
            with_regionctx(self.id, |ctx| {
                notifications = ctx.notifications_entities.clone();
            });

            notifications
                .iter()
                .filter(|(_, tick, _)| *tick <= ticks)
                .cloned() // Clone only the relevant items
                .collect::<Vec<_>>() // Store them in a new list
        };
        for (id, _tick, notification) in &to_process {
            if !is_entity_dead(self.id, *id) {
                // let mut cmd = String::new();
                with_regionctx(self.id, |ctx| {
                    if notification == "attack" {
                        let parse_target_attr = |value: Option<&Value>| -> Option<u32> {
                            match value {
                                Some(Value::UInt(id)) => Some(*id),
                                Some(Value::Int(id)) if *id >= 0 => Some(*id as u32),
                                Some(Value::Int64(id)) if *id >= 0 => Some(*id as u32),
                                Some(Value::Str(id)) => id.trim().parse::<u32>().ok(),
                                _ => None,
                            }
                        };

                        let Some(attacker) = ctx.map.entities.iter().find(|e| e.id == *id) else {
                            return;
                        };
                        let target_id = parse_target_attr(attacker.attributes.get("attack_target"))
                            .or_else(|| parse_target_attr(attacker.attributes.get("target")));
                        let Some(target_id) = target_id else {
                            return;
                        };

                        let Some(target) = ctx.map.entities.iter().find(|e| e.id == target_id)
                        else {
                            return;
                        };
                        let target_mode =
                            target.attributes.get_str_default("mode", "active".into());
                        if target_mode == "dead" {
                            return;
                        }

                        let attacker_sector = attacker
                            .attributes
                            .get("sector_id")
                            .and_then(|value| match value {
                                Value::Int64(v) if *v >= 0 => Some(*v as u32),
                                Value::Int(v) if *v >= 0 => Some(*v as u32),
                                _ => None,
                            })
                            .or_else(|| {
                                ctx.map.find_sector_at(attacker.get_pos_xz()).map(|s| s.id)
                            });
                        let target_sector = target
                            .attributes
                            .get("sector_id")
                            .and_then(|value| match value {
                                Value::Int64(v) if *v >= 0 => Some(*v as u32),
                                Value::Int(v) if *v >= 0 => Some(*v as u32),
                                _ => None,
                            })
                            .or_else(|| ctx.map.find_sector_at(target.get_pos_xz()).map(|s| s.id));

                        if attacker_sector.is_some()
                            && target_sector.is_some()
                            && attacker_sector != target_sector
                        {
                            return;
                        }
                    }

                    if let Some(class_name) = ctx.entity_classes.get(id) {
                        // cmd = format!("{}.event(\"{}\", \"\")", class_name, notification);
                        ctx.curr_entity_id = *id;
                        ctx.curr_item_id = None;

                        if let Some(program) = ctx.entity_programs.get(class_name).cloned() {
                            let payload = if notification == "closed_in" {
                                let target_id = ctx
                                    .map
                                    .entities
                                    .iter()
                                    .find(|entity| entity.id == *id)
                                    .and_then(|entity| match entity.attributes.get("target") {
                                        Some(Value::UInt(target_id)) => Some(*target_id),
                                        Some(Value::Int(target_id)) if *target_id >= 0 => {
                                            Some(*target_id as u32)
                                        }
                                        Some(Value::Int64(target_id)) if *target_id >= 0 => {
                                            Some(*target_id as u32)
                                        }
                                        Some(Value::Str(target_id)) => {
                                            target_id.trim().parse::<u32>().ok()
                                        }
                                        _ => None,
                                    })
                                    .unwrap_or(0);
                                VMValue::broadcast(target_id as f32)
                            } else {
                                VMValue::zero()
                            };
                            let args = [VMValue::from_string(notification), payload];
                            run_server_fn(&mut self.exec, &args, &program, ctx);
                            flush_pending_entity_transfers(ctx);
                        }
                    }
                });

                // let _ = self.execute(&cmd);
            }
        }

        with_regionctx(self.id, |ctx| {
            ctx.notifications_entities.retain(|(id, tick, _)| {
                !to_process
                    .iter()
                    .any(|(pid, _, _)| pid == id && *tick <= ticks)
            });
        });

        // Process notifications for items.
        let to_process = {
            let mut notifications = vec![];
            with_regionctx(self.id, |ctx| {
                notifications = ctx.notifications_items.clone();
            });

            notifications
                .iter()
                .filter(|(_, tick, _)| *tick <= ticks)
                .cloned()
                .collect::<Vec<_>>()
        };
        for (id, _tick, notification) in &to_process {
            // let mut cmd = String::new();
            with_regionctx(self.id, |ctx| {
                if let Some(class_name) = ctx.item_classes.get(id) {
                    // cmd = format!("{}.event(\"{}\", \"\")", class_name, notification);
                    ctx.curr_item_id = Some(*id);

                    if let Some(program) = ctx.item_programs.get(class_name).cloned() {
                        let args = [VMValue::from_string(notification), VMValue::zero()];
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                        ctx.curr_item_id = None;
                    }
                }
            });
            // let _ = self.execute(&cmd);
            // with_regionctx(self.id, |ctx| {
            //     ctx.curr_item_id = None;
            // });
        }

        with_regionctx(self.id, |ctx| {
            ctx.notifications_items.retain(|(id, tick, _)| {
                !to_process
                    .iter()
                    .any(|(pid, _, _)| pid == id && *tick <= ticks)
            });
        });

        // Check Proximity Alerts
        with_regionctx(self.id, |ctx| {
            for (id, radius) in ctx.entity_proximity_alerts.iter() {
                let entities = self.entities_in_radius(ctx, Some(*id), None, *radius);
                if !entities.is_empty() {
                    // if let Some(class_name) = ctx.entity_classes.get(id) {
                    // let cmd = format!(
                    //     "{}.event(\"{}\", [{}])",
                    //     class_name,
                    //     "proximity_warning",
                    //     entities
                    //         .iter()
                    //         .map(|e| e.to_string())
                    //         .collect::<Vec<_>>()
                    //         .join(",")
                    // );
                    // }
                    ctx.to_execute_entity.push((
                        *id,
                        "proximity_warning".into(),
                        VMValue::from(entities[0]),
                    ));
                }
            }
        });
    }

    /// Redraw tick
    pub fn redraw_tick(&mut self) {
        // Catch up with the server messages
        while let Ok(msg) = self.to_receiver.try_recv() {
            match msg {
                Pause => {
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        ctx.paused = true;
                    });
                }
                Continue => {
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        ctx.paused = false;
                    });
                }
                Event(entity_id, event, value) => {
                    // let mut cmd = String::new();
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        if let Some(class_name) = ctx.entity_classes.get(&entity_id) {
                            // cmd = format!("{}.event('{}', {})", class_name, event, value);
                            ctx.curr_entity_id = entity_id;
                            ctx.curr_item_id = None;

                            if let Some(program) = ctx.entity_programs.get(class_name).cloned() {
                                let args =
                                    [VMValue::from_string(event), VMValue::from_value(&value)];
                                run_server_fn(&mut self.exec, &args, &program, ctx);
                                flush_pending_entity_transfers(ctx);
                            }
                        }
                    });

                    // if let Err(err) = self.execute(&cmd) {
                    //     send_log_message(
                    //         self.id,
                    //         format!(
                    //             "{}: Event Error for '{}': {}",
                    //             self.name,
                    //             self.get_entity_name(entity_id),
                    //             err,
                    //         ),
                    //     );
                    // }
                }
                UserEvent(entity_id, event, value) => {
                    // let mut cmd = String::new();
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        if let Some(class_name) = ctx.entity_classes.get(&entity_id) {
                            // cmd = format!("{}.user_event('{}', '{}')", class_name, event, value);
                            ctx.curr_entity_id = entity_id;
                            ctx.curr_item_id = None;

                            if let Some(program) = ctx.entity_programs.get(class_name).cloned() {
                                let args =
                                    [VMValue::from_string(event), VMValue::from_value(&value)];
                                run_client_fn(&mut self.exec, &args, &program, ctx);
                            }
                        }
                    });
                    // if let Err(err) = self.execute(&cmd) {
                    //     send_log_message(
                    //         self.id,
                    //         format!(
                    //             "{}: User Event Error for '{}': {}",
                    //             self.name,
                    //             self.get_entity_name(entity_id),
                    //             err,
                    //         ),
                    //     );
                    // }
                }
                UserAction(entity_id, action) => {
                    if Self::action_requests_simulation_step(&action) {
                        with_regionctx(self.id, |ctx: &mut RegionCtx| {
                            if let Some(entity) = ctx
                                .map
                                .entities
                                .iter()
                                .find(|entity| entity.id == entity_id)
                                && entity.is_player()
                                && self.should_accept_step_request(ctx, &action)
                                && !(Self::is_movement_input_action(&action)
                                    && Self::entity_has_active_continuous_motion(entity))
                            {
                                if Self::is_click_like_step_action(&action) {
                                    self.last_external_step_request_at = Instant::now();
                                }
                                self.note_simulation_step_request();
                            }
                        });
                    }
                    match action {
                        Intent(intent) => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                if let Some(entity) = ctx
                                    .map
                                    .entities
                                    .iter_mut()
                                    .find(|entity| entity.id == entity_id)
                                {
                                    entity.set_attribute("intent", Value::Str(intent));
                                }
                            });
                        }
                        action
                            if Self::is_movement_input_action(&action)
                                && action != EntityAction::Off =>
                        {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                if let Some(entity) = ctx
                                    .map
                                    .entities
                                    .iter_mut()
                                    .find(|entity| entity.id == entity_id)
                                {
                                    entity.set_attribute(
                                        "__grid_goto_target",
                                        Value::Str(String::new()),
                                    );
                                    let is_grid_player = matches!(
                                        entity.attributes.get("player_camera"),
                                        Some(Value::PlayerCamera(camera))
                                            if Self::is_grid_camera(camera)
                                    );
                                    if is_grid_player {
                                        Self::update_grid_input_state(entity, &action);
                                        if matches!(
                                            entity.action,
                                            EntityAction::StepTo(_, _, _, _, _)
                                                | EntityAction::RotateTo(_)
                                        ) {
                                            return;
                                        }
                                        if action == Self::blocked_grid_action(entity) {
                                            return;
                                        }
                                    }
                                    entity.action = action;
                                }
                            });
                        }
                        EntityClicked(clicked_entity_id, distance, explicit_intent) => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                if ctx.entity_classes.get(&entity_id).is_none() {
                                    return;
                                }
                                let distance =
                                    Self::entity_click_distance(ctx, entity_id, clicked_entity_id)
                                        .unwrap_or(distance);

                                let intent_raw = if let Some(int) = explicit_intent {
                                    int
                                } else {
                                    ctx.map
                                        .entities
                                        .iter()
                                        .find(|e| e.id == entity_id)
                                        .map(|e| e.attributes.get_str_default("intent", "".into()))
                                        .unwrap_or_default()
                                };
                                let intent = intent_raw.trim().to_string();
                                let intent_lower = intent.to_ascii_lowercase();
                                let mut handled_shortcut = false;
                                let keep_intent = ctx
                                    .map
                                    .entities
                                    .iter()
                                    .find(|e| e.id == entity_id)
                                    .map(|entity| Self::should_keep_player_intent(ctx, entity))
                                    .unwrap_or(false);
                                let subject = ctx.map.entities.iter().find(|e| e.id == entity_id);
                                let target_entity =
                                    ctx.map.entities.iter().find(|e| e.id == clicked_entity_id);
                                let rules = intent_rule_config(ctx, entity_id, &intent_lower);

                                if !intent.is_empty()
                                    && let Some(max_distance) =
                                        entity_intent_distance_limit(ctx, entity_id, &intent_lower)
                                    && distance > max_distance
                                {
                                    send_message(
                                        ctx,
                                        entity_id,
                                        "{system.too_far_away}".into(),
                                        "warning",
                                    );
                                    if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
                                        && !keep_intent
                                    {
                                        entity.set_attribute("intent", Value::Str(String::new()));
                                    }
                                    return;
                                }

                                if !intent.is_empty()
                                    && let Some(allowed) = rules.allowed.as_deref()
                                    && !evaluate_intent_allowed(
                                        ctx,
                                        allowed,
                                        distance,
                                        subject,
                                        target_entity,
                                        None,
                                    )
                                {
                                    send_message(
                                        ctx,
                                        entity_id,
                                        rules
                                            .deny_message
                                            .clone()
                                            .unwrap_or_else(|| "{system.cant_do_that}".to_string()),
                                        "warning",
                                    );
                                    if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
                                        && !keep_intent
                                    {
                                        entity.set_attribute("intent", Value::Str(String::new()));
                                    }
                                    return;
                                }

                                if let Some(spell_template) = intent.strip_prefix("spell:") {
                                    let spell_template = spell_template.trim();
                                    if !spell_template.is_empty() {
                                        let spell_id = cast_spell_for_entity(
                                            ctx,
                                            entity_id,
                                            spell_template,
                                            clicked_entity_id,
                                            100.0,
                                        );
                                        handled_shortcut = spell_id >= 0;
                                    }
                                }

                                // Optional character-level shortcuts for common intents.
                                if !handled_shortcut
                                    && intent_lower == "look"
                                    && let Some(target) =
                                        ctx.map.entities.iter().find(|e| e.id == clicked_entity_id)
                                {
                                    if let Some(msg) = target.attributes.get_str("on_look") {
                                        let msg = msg.trim();
                                        if !msg.is_empty() {
                                            send_message(ctx, entity_id, msg.to_string(), "system");
                                            handled_shortcut = true;
                                        }
                                    }
                                    if !handled_shortcut
                                        && let Some(msg) = entity_look_description(ctx, target)
                                    {
                                        send_message(ctx, entity_id, msg, "system");
                                        handled_shortcut = true;
                                    }
                                }

                                if !handled_shortcut {
                                    // Send default script-driven intent events.
                                    ctx.to_execute_entity.push((
                                        entity_id,
                                        "intent".to_string(),
                                        VMValue::new_with_string(
                                            clicked_entity_id as f32,
                                            distance as f32,
                                            0.0,
                                            &intent,
                                        ),
                                    ));

                                    if ctx.entity_classes.get(&clicked_entity_id).is_some() {
                                        ctx.to_execute_entity.push((
                                            clicked_entity_id,
                                            "intent".to_string(),
                                            VMValue::new_with_string(
                                                entity_id as f32,
                                                distance as f32,
                                                0.0,
                                                &intent,
                                            ),
                                        ));
                                    }
                                }

                                queue_intent_cooldown(
                                    ctx,
                                    entity_id,
                                    &intent_lower,
                                    rules.cooldown_minutes,
                                );

                                if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
                                    && !keep_intent
                                {
                                    entity.set_attribute("intent", Value::Str(String::new()));
                                }
                            });
                        }
                        ItemClicked(
                            clicked_item_id,
                            distance,
                            explicit_intent,
                            owner_entity_id,
                        ) => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                if ctx.entity_classes.get(&entity_id).is_none() {
                                    return;
                                }

                                let item_owner_id = owner_entity_id.unwrap_or(entity_id);
                                let distance = Self::item_click_distance(
                                    ctx,
                                    entity_id,
                                    clicked_item_id,
                                    owner_entity_id,
                                )
                                .unwrap_or(distance);

                                let intent_raw = if let Some(int) = explicit_intent {
                                    int
                                } else {
                                    ctx.map
                                        .entities
                                        .iter()
                                        .find(|e| e.id == entity_id)
                                        .map(|e| e.attributes.get_str_default("intent", "".into()))
                                        .unwrap_or_default()
                                };
                                let intent = intent_raw.trim().to_string();
                                let intent_lower = intent.to_ascii_lowercase();
                                let mut handled_shortcut = false;
                                let keep_intent = ctx
                                    .map
                                    .entities
                                    .iter()
                                    .find(|e| e.id == entity_id)
                                    .map(|entity| Self::should_keep_player_intent(ctx, entity))
                                    .unwrap_or(false);
                                let subject = ctx.map.entities.iter().find(|e| e.id == entity_id);
                                let target_item = ctx
                                    .map
                                    .items
                                    .iter()
                                    .find(|i| i.id == clicked_item_id)
                                    .or_else(|| {
                                        ctx.map
                                            .entities
                                            .iter()
                                            .find(|e| e.id == item_owner_id)
                                            .and_then(|e| e.get_item(clicked_item_id))
                                    })
                                    .or_else(|| {
                                        ctx.map
                                            .entities
                                            .iter()
                                            .find(|e| e.id == item_owner_id)
                                            .and_then(|e| {
                                                e.equipped
                                                    .values()
                                                    .find(|item| item.id == clicked_item_id)
                                            })
                                    });
                                let authored_use_message = if intent_lower == "use" {
                                    target_item.and_then(|item| item_use_message(ctx, item))
                                } else {
                                    None
                                };
                                let rules = intent_rule_config(ctx, entity_id, &intent_lower);

                                if !intent.is_empty()
                                    && let Some(max_distance) =
                                        entity_intent_distance_limit(ctx, entity_id, &intent_lower)
                                    && distance > max_distance
                                {
                                    send_message(
                                        ctx,
                                        entity_id,
                                        "{system.too_far_away}".into(),
                                        "warning",
                                    );
                                    if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
                                        && !keep_intent
                                    {
                                        entity.set_attribute("intent", Value::Str(String::new()));
                                    }
                                    return;
                                }

                                if !intent.is_empty()
                                    && let Some(allowed) = rules.allowed.as_deref()
                                    && !evaluate_intent_allowed(
                                        ctx,
                                        allowed,
                                        distance,
                                        subject,
                                        None,
                                        target_item,
                                    )
                                {
                                    send_message(
                                        ctx,
                                        entity_id,
                                        rules
                                            .deny_message
                                            .clone()
                                            .unwrap_or_else(|| "{system.cant_do_that}".to_string()),
                                        "warning",
                                    );
                                    if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
                                        && !keep_intent
                                    {
                                        entity.set_attribute("intent", Value::Str(String::new()));
                                    }
                                    return;
                                }

                                // Optional item-level shortcuts for common intents.
                                let item_attrs = ctx
                                    .map
                                    .items
                                    .iter()
                                    .find(|i| i.id == clicked_item_id)
                                    .map(|i| &i.attributes)
                                    .or_else(|| {
                                        ctx.map
                                            .entities
                                            .iter()
                                            .find(|e| e.id == entity_id)
                                            .and_then(|e| e.get_item(clicked_item_id))
                                            .map(|i| &i.attributes)
                                    })
                                    .or_else(|| {
                                        ctx.map
                                            .entities
                                            .iter()
                                            .find(|e| e.id == entity_id)
                                            .and_then(|e| {
                                                e.equipped
                                                    .values()
                                                    .find(|item| item.id == clicked_item_id)
                                            })
                                            .map(|i| &i.attributes)
                                    });
                                if intent_lower == "drop" {
                                    if let Some(attrs) = item_attrs {
                                        if let Some(action) = attrs.get_str("on_drop") {
                                            let action = action.trim();
                                            if action.is_empty()
                                                || action.eq_ignore_ascii_case("drop")
                                            {
                                                handled_shortcut = drop_item_for_entity(
                                                    ctx,
                                                    entity_id,
                                                    clicked_item_id,
                                                );
                                            } else if action
                                                .eq_ignore_ascii_case("you cannot drop that")
                                            {
                                                send_message(
                                                    ctx,
                                                    entity_id,
                                                    action.to_string(),
                                                    "system",
                                                );
                                                handled_shortcut = true;
                                            } else {
                                                send_message(
                                                    ctx,
                                                    entity_id,
                                                    action.to_string(),
                                                    "system",
                                                );
                                                handled_shortcut = drop_item_for_entity(
                                                    ctx,
                                                    entity_id,
                                                    clicked_item_id,
                                                );
                                            }
                                        } else {
                                            handled_shortcut = drop_item_for_entity(
                                                ctx,
                                                entity_id,
                                                clicked_item_id,
                                            );
                                        }
                                    } else {
                                        handled_shortcut =
                                            drop_item_for_entity(ctx, entity_id, clicked_item_id);
                                    }
                                } else if let Some(attrs) = item_attrs {
                                    if intent_lower == "look" {
                                        if let Some(msg) = attrs.get_str("on_look") {
                                            let msg = msg.trim();
                                            if !msg.is_empty() {
                                                send_message(
                                                    ctx,
                                                    entity_id,
                                                    msg.to_string(),
                                                    "system",
                                                );
                                                handled_shortcut = true;
                                            }
                                        }
                                        if !handled_shortcut
                                            && let Some(item) = ctx
                                                .map
                                                .items
                                                .iter()
                                                .find(|i| i.id == clicked_item_id)
                                                .or_else(|| {
                                                    ctx.map
                                                        .entities
                                                        .iter()
                                                        .find(|e| e.id == entity_id)
                                                        .and_then(|e| e.get_item(clicked_item_id))
                                                })
                                                .or_else(|| {
                                                    ctx.map
                                                        .entities
                                                        .iter()
                                                        .find(|e| e.id == entity_id)
                                                        .and_then(|e| {
                                                            e.equipped.values().find(|item| {
                                                                item.id == clicked_item_id
                                                            })
                                                        })
                                                })
                                            && let Some(msg) = item_look_description(ctx, item)
                                        {
                                            send_message(ctx, entity_id, msg, "system");
                                            handled_shortcut = true;
                                        }
                                    } else if intent_lower == "use" {
                                        if let Some(msg) = attrs.get_str("on_use") {
                                            let msg = msg.trim();
                                            if !msg.is_empty() {
                                                send_message(
                                                    ctx,
                                                    entity_id,
                                                    msg.to_string(),
                                                    "system",
                                                );
                                                handled_shortcut = true;
                                            }
                                        }
                                    } else if intent_lower == "pickup" || intent_lower == "take" {
                                        if let Some(action) = attrs
                                            .get_str("on_pickup")
                                            .or_else(|| attrs.get_str("on_take"))
                                        {
                                            let action = action.trim();
                                            if !action.is_empty() {
                                                if action.eq_ignore_ascii_case("pickup")
                                                    || action.eq_ignore_ascii_case("take")
                                                {
                                                    take_item_for_entity(
                                                        ctx,
                                                        entity_id,
                                                        clicked_item_id,
                                                    );
                                                } else {
                                                    send_message(
                                                        ctx,
                                                        entity_id,
                                                        action.to_string(),
                                                        "system",
                                                    );
                                                }
                                                handled_shortcut = true;
                                            }
                                        }
                                    }
                                }

                                if !handled_shortcut
                                    && intent_lower == "use"
                                    && let Some(msg) = authored_use_message
                                {
                                    send_message(ctx, entity_id, msg, "system");
                                }

                                if !handled_shortcut {
                                    // Send default script-driven intent events.
                                    ctx.to_execute_entity.push((
                                        entity_id,
                                        "intent".to_string(),
                                        VMValue::new_with_string(
                                            clicked_item_id as f32,
                                            distance as f32,
                                            0.0,
                                            &intent,
                                        ),
                                    ));

                                    if ctx.item_classes.get(&clicked_item_id).is_some() {
                                        ctx.to_execute_item.push((
                                            clicked_item_id,
                                            "intent".to_string(),
                                            VMValue::new_with_string(
                                                entity_id as f32,
                                                distance as f32,
                                                0.0,
                                                &intent,
                                            ),
                                        ));
                                    }
                                }

                                queue_intent_cooldown(
                                    ctx,
                                    entity_id,
                                    &intent_lower,
                                    rules.cooldown_minutes,
                                );

                                if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
                                    && !keep_intent
                                {
                                    entity.set_attribute("intent", Value::Str(String::new()));
                                }
                            });
                        }
                        TerrainClicked(position) => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                if !get_config_bool_default(ctx, "game", "auto_walk_2d", false) {
                                    return;
                                }

                                let Some(snapshot) = ctx
                                    .map
                                    .entities
                                    .iter()
                                    .find(|entity| entity.id == entity_id)
                                    .cloned()
                                else {
                                    return;
                                };
                                if !snapshot.is_player() {
                                    return;
                                }

                                if !matches!(
                                    snapshot.attributes.get("player_camera"),
                                    Some(Value::PlayerCamera(
                                        PlayerCamera::D2 | PlayerCamera::D2Grid
                                    ))
                                ) {
                                    return;
                                }

                                let intent =
                                    snapshot.attributes.get_str_default("intent", "".into());
                                if !intent.trim().is_empty() {
                                    return;
                                }
                                if matches!(
                                    snapshot.attributes.get("player_camera"),
                                    Some(Value::PlayerCamera(PlayerCamera::D2Grid))
                                ) {
                                    let step = self.compute_grid_goto_step_in_ctx(
                                        ctx,
                                        snapshot.get_pos_xz(),
                                        position,
                                    );
                                    let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
                                    else {
                                        return;
                                    };
                                    if let Some((next, facing, target)) = step {
                                        let start = entity.get_pos_xz();
                                        let step_dir = next - start;
                                        entity.set_orientation(facing);
                                        entity.action = EntityAction::StepTo(
                                            next, 1.0, facing, start, step_dir,
                                        );
                                        entity.set_attribute(
                                            "__grid_goto_target",
                                            Value::Str(format!("{},{}", target.x, target.y)),
                                        );
                                    } else {
                                        entity.set_attribute(
                                            "__grid_goto_target",
                                            Value::Str(String::new()),
                                        );
                                        entity.action = EntityAction::Off;
                                    }
                                } else {
                                    let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
                                    else {
                                        return;
                                    };
                                    entity.action = Goto(position, 1.0);
                                }
                            });
                        }
                        SetPlayerCamera(player_camera) => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                if let Some(entity) = ctx
                                    .map
                                    .entities
                                    .iter_mut()
                                    .find(|entity| entity.id == entity_id)
                                {
                                    entity.set_attribute(
                                        "player_camera",
                                        Value::PlayerCamera(player_camera),
                                    );
                                }
                            });
                        }
                        MoveItem {
                            item_id,
                            owner_entity_id,
                            target_entity_id,
                            to_inventory_index,
                            to_equipped_slot,
                        } => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                _ = move_item_for_entity(
                                    ctx,
                                    owner_entity_id.unwrap_or(entity_id),
                                    target_entity_id.unwrap_or(entity_id),
                                    item_id,
                                    to_inventory_index,
                                    to_equipped_slot,
                                );
                            });
                        }
                        DropItemAt {
                            item_id,
                            owner_entity_id,
                            position,
                        } => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                _ = drop_item_for_entity_at(
                                    ctx,
                                    owner_entity_id.unwrap_or(entity_id),
                                    item_id,
                                    Some(position),
                                );
                            });
                        }
                        Choice(choice) => match &choice {
                            Choice::ItemToSell(
                                item_id,
                                seller_id,
                                buyer_id,
                                expires_at_tick,
                                max_distance,
                            ) => {
                                with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                    clear_choice_session(ctx, *seller_id, *buyer_id);
                                    if !choice_session_is_valid(
                                        ctx,
                                        *seller_id,
                                        *buyer_id,
                                        *expires_at_tick,
                                        *max_distance,
                                    ) {
                                        if let Some(_class_name) = ctx.entity_classes.get(seller_id)
                                        {
                                            ctx.to_execute_entity.push((
                                                *seller_id,
                                                "goodbye".into(),
                                                VMValue::broadcast(*buyer_id as f32),
                                            ));
                                        }
                                        return;
                                    }

                                    let mut msg_to_buyer: Option<String> = None;
                                    let mut msg_role = "system";

                                    // Get the price of the item.
                                    let mut price = 0;
                                    let mut can_afford = false;
                                    if let Some(entity) = get_entity_mut(&mut ctx.map, *seller_id) {
                                        if let Some(item) = entity.get_item(*item_id) {
                                            if let Some(w) = item.get_attribute("worth") {
                                                if let Some(worth) = w.to_i32() {
                                                    price = worth as i64;
                                                }
                                            }
                                        }
                                    }

                                    // Check if the buyer can afford
                                    if let Some(entity) = get_entity_mut(&mut ctx.map, *buyer_id) {
                                        can_afford =
                                            entity.wallet.can_afford(price, &ctx.currencies);
                                    }

                                    if can_afford {
                                        let mut item_to_sell: Option<Item> = None;
                                        if let Some(entity) =
                                            get_entity_mut(&mut ctx.map, *seller_id)
                                        {
                                            if let Some(item) = entity.remove_item(*item_id) {
                                                item_to_sell = Some(item);
                                                _ = entity
                                                    .add_base_currency(price, &ctx.currencies);
                                            }
                                        }
                                        if let Some(item) = item_to_sell {
                                            if let Some(entity) =
                                                get_entity_mut(&mut ctx.map, *buyer_id)
                                            {
                                                msg_to_buyer = Some(format!(
                                                    "{{system.you_bought}} {{I:{}.name, article=indef, case=lower}}",
                                                    item.id
                                                ));
                                                _ = entity.add_item(item);
                                                _ = entity.spend_currency(price, &ctx.currencies);
                                            }
                                        }
                                    } else {
                                        msg_to_buyer = Some("{system.cant_afford}".into());
                                        msg_role = "warning";
                                    }

                                    if let Some(msg_to_buyer) = msg_to_buyer {
                                        send_message(ctx, *buyer_id, msg_to_buyer, msg_role);
                                    }
                                });
                            }
                            Choice::Cancel(from_id, to_id, _, _) => {
                                with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                    clear_choice_session(ctx, *from_id, *to_id);
                                    if let Some(_class_name) = ctx.entity_classes.get(from_id) {
                                        // let cmd = format!("{}.event('goodbye', {})", class_name, to_id);
                                        ctx.to_execute_entity.push((
                                            *from_id,
                                            "goodbye".into(),
                                            VMValue::broadcast(*to_id as f32),
                                        ));
                                    }
                                });
                            }
                            Choice::ScriptChoice(
                                label,
                                choice_attr,
                                from_id,
                                to_id,
                                index,
                                expires_at_tick,
                                max_distance,
                            ) => {
                                with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                    clear_choice_session(ctx, *from_id, *to_id);
                                    if !choice_session_is_valid(
                                        ctx,
                                        *from_id,
                                        *to_id,
                                        *expires_at_tick,
                                        *max_distance,
                                    ) {
                                        if let Some(_class_name) = ctx.entity_classes.get(from_id) {
                                            ctx.to_execute_entity.push((
                                                *from_id,
                                                "goodbye".into(),
                                                VMValue::broadcast(*to_id as f32),
                                            ));
                                        }
                                        return;
                                    }

                                    if let Some(_class_name) = ctx.entity_classes.get(from_id) {
                                        let value = VMValue::new_with_string(
                                            *to_id as f32,
                                            *index as f32,
                                            0.0,
                                            label.clone(),
                                        );
                                        ctx.to_execute_entity.push((
                                            *from_id,
                                            choice_attr.clone(),
                                            value.clone(),
                                        ));
                                        ctx.to_execute_entity.push((
                                            *from_id,
                                            format!("{choice_attr}:{index}"),
                                            value,
                                        ));
                                    }
                                });
                            }
                            Choice::DialogChoice(dialog_choice) => {
                                with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                    clear_choice_session(ctx, dialog_choice.from, dialog_choice.to);
                                    if !choice_session_is_valid(
                                        ctx,
                                        dialog_choice.from,
                                        dialog_choice.to,
                                        dialog_choice.expires_at_tick,
                                        dialog_choice.max_distance,
                                    ) {
                                        if ctx.entity_classes.contains_key(&dialog_choice.from) {
                                            ctx.to_execute_entity.push((
                                                dialog_choice.from,
                                                "goodbye".into(),
                                                VMValue::broadcast(dialog_choice.to as f32),
                                            ));
                                        }
                                        return;
                                    }

                                    if ctx.entity_classes.contains_key(&dialog_choice.from) {
                                        let value = VMValue::new_with_string(
                                            dialog_choice.to as f32,
                                            dialog_choice.index as f32,
                                            0.0,
                                            dialog_choice.label.clone(),
                                        );
                                        if let Some(event) = &dialog_choice.event
                                            && !event.trim().is_empty()
                                        {
                                            ctx.to_execute_entity.push((
                                                dialog_choice.from,
                                                event.trim().to_string(),
                                                value.clone(),
                                            ));
                                        }
                                    }

                                    if !dialog_choice.end
                                        && let Some(next) = &dialog_choice.next
                                        && !next.trim().is_empty()
                                    {
                                        open_dialog_node(
                                            ctx,
                                            dialog_choice.from,
                                            dialog_choice.to,
                                            next,
                                        );
                                    }
                                });
                            }
                        },
                        _ => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                if let Some(entity) = ctx
                                    .map
                                    .entities
                                    .iter_mut()
                                    .find(|entity| entity.id == entity_id)
                                {
                                    let is_grid_player = matches!(
                                        entity.attributes.get("player_camera"),
                                        Some(Value::PlayerCamera(camera))
                                            if Self::is_grid_camera(camera)
                                    );
                                    if is_grid_player && Self::is_movement_input_action(&action) {
                                        Self::update_grid_input_state(entity, &action);
                                    }
                                    if is_grid_player
                                        && matches!(
                                            entity.action,
                                            EntityAction::StepTo(_, _, _, _, _)
                                                | EntityAction::RotateTo(_)
                                        )
                                        && Self::is_movement_input_action(&action)
                                    {
                                        return;
                                    }
                                    if is_grid_player
                                        && Self::is_movement_input_action(&action)
                                        && action == Self::blocked_grid_action(entity)
                                    {
                                        return;
                                    }
                                    if is_grid_player
                                        && Self::is_movement_input_action(&action)
                                        && action == EntityAction::Off
                                    {
                                        entity.action = EntityAction::Off;
                                        return;
                                    }
                                    entity.action = action;
                                }
                            });
                        }
                    }
                }
                CreateEntity(_id, entity) => self.create_entity_instance(entity),
                ShowStartupSectorDescription(entity_id) => {
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        if let Some(entity) =
                            ctx.map.entities.iter().find(|e| e.id == entity_id).cloned()
                            && let Some(sector) =
                                ctx.map.find_sector_at(entity.get_pos_xz()).cloned()
                        {
                            ctx.send_player_sector_description(&entity, &sector, true);
                        }
                    });
                }
                TeleportEntity(entity_id, dest_sector_name, dest_region_name) => {
                    if dest_region_name.is_empty() {
                        with_regionctx(self.id, |ctx: &mut RegionCtx| {
                            let center = {
                                let map = &ctx.map;
                                map.sectors
                                    .iter()
                                    .find(|s| s.name == *dest_sector_name)
                                    .and_then(|s| s.center(map))
                            };

                            if let Some(center) = center {
                                if let Some(entity) =
                                    ctx.map.entities.iter_mut().find(|e| e.id == entity_id)
                                {
                                    let id = entity.id;
                                    entity.set_pos_xz(center);
                                    ctx.check_player_for_section_change_id(id);
                                }
                            }
                        });
                    } else {
                        with_regionctx(self.id, |ctx: &mut RegionCtx| {
                            if let Some(pos) =
                                ctx.map.entities.iter().position(|e| e.id == entity_id)
                            {
                                let removed = ctx.map.entities.remove(pos);
                                ctx.entity_classes.remove(&removed.id);

                                if let Some(sender) = ctx.from_sender.get() {
                                    let _ = sender.send(RegionMessage::TransferEntity(
                                        ctx.region_id,
                                        removed,
                                        dest_region_name.clone(),
                                        dest_sector_name.clone(),
                                    ));
                                }
                            }
                        });
                    }
                }
                TeleportEntityPos(entity_id, position) => {
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        if let Some(entity) =
                            ctx.map.entities.iter_mut().find(|e| e.id == entity_id)
                        {
                            let id = entity.id;
                            entity.set_pos_xz(position.clone());
                            ctx.check_player_for_section_change_id(id);
                        }
                    });
                }
                TransferEntity(_region_id, entity, _dest_region_name, dest_sector_name) => {
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        receive_entity(ctx, entity, dest_sector_name);
                    });
                }
                Time(_id, time) => {
                    // User manually set the server time
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        ctx.ticks = time.to_ticks(ctx.ticks_per_minute);
                        ctx.time = time;
                    });
                }
                Quit => {
                    println!("Shutting down '{}'. Goodbye.", self.name);
                }
                _ => {}
            }
        }

        // ---

        let mut updates: Vec<Vec<u8>> = vec![];
        let mut item_updates: Vec<Vec<u8>> = vec![];
        let now = Instant::now();
        let redraw_dt = now
            .saturating_duration_since(self.last_redraw_at)
            .as_secs_f32()
            .clamp(1.0 / 240.0, 0.1);
        self.last_redraw_at = now;
        let mut turn_step_deg: f32 = 4.0;
        let mut click_intents_2d = false;
        let mut sim_dt = redraw_dt;

        let mut entities = vec![];
        with_regionctx(self.id, |ctx: &mut RegionCtx| {
            if ctx.paused {
                return;
            }
            sim_dt = self.simulation_dt_for_frame(ctx, redraw_dt);
            ctx.delta_time = sim_dt;
            update_spell_cooldowns(ctx, sim_dt);
            entities = ctx.map.entities.clone();
            let turn_speed_deg_per_sec =
                get_config_i32_default(ctx, "game", "turn_speed_deg_per_sec", 120).max(1) as f32;
            turn_step_deg = turn_speed_deg_per_sec * sim_dt;
            click_intents_2d = get_config_bool_default(ctx, "game", "click_intents_2d", false)
                || get_config_bool_default(ctx, "game", "persistent_2d_intents", false);
        });

        for entity in &mut entities {
            if sim_dt <= 0.0 {
                if entity.is_dirty() {
                    updates.push(entity.get_update().pack());
                    entity.clear_dirty();
                }
                continue;
            }

            if !self.current_frame_has_turn_step
                && !matches!(
                    entity.action,
                    EntityAction::StepTo(_, _, _, _, _) | EntityAction::RotateTo(_)
                )
            {
                if entity.is_dirty() {
                    updates.push(entity.get_update().pack());
                    entity.clear_dirty();
                }
                continue;
            }

            let action_start_pos = entity.get_pos_xz();
            match &entity.action.clone() {
                EntityAction::Forward => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_grid_camera(player_camera) {
                                    if Self::is_first_person_camera(player_camera) {
                                        let facing =
                                            Self::snapped_cardinal_direction(entity.orientation);
                                        let target = entity.get_pos_xz() + facing;
                                        self.queue_step_to(entity, target, facing);
                                    } else {
                                        entity.face_north();
                                        let target = entity.get_pos_xz() + Vec2::new(0.0, -1.0);
                                        self.queue_step_to(entity, target, Vec2::new(0.0, -1.0));
                                    }
                                } else {
                                    if !Self::is_first_person_camera(player_camera) {
                                        entity.face_north();
                                    }
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_north();
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, 1.0, self.entity_block_mode);
                    }
                }
                EntityAction::Left => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_grid_camera(player_camera) {
                                    if Self::is_first_person_camera(player_camera) {
                                        self.rotate_grid_left(entity);
                                    } else {
                                        entity.face_west();
                                        let target = entity.get_pos_xz() + Vec2::new(-1.0, 0.0);
                                        self.queue_step_to(entity, target, Vec2::new(-1.0, 0.0));
                                    }
                                } else if !Self::is_first_person_camera(player_camera) {
                                    entity.face_west();
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    entity.turn_left(turn_step_deg);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_west();
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        entity.turn_left(turn_step_deg);
                    }
                }
                EntityAction::Right => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            // If no intent we walk
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_grid_camera(player_camera) {
                                    if Self::is_first_person_camera(player_camera) {
                                        self.rotate_grid_right(entity);
                                    } else {
                                        entity.face_east();
                                        let target = entity.get_pos_xz() + Vec2::new(1.0, 0.0);
                                        self.queue_step_to(entity, target, Vec2::new(1.0, 0.0));
                                    }
                                } else if !Self::is_first_person_camera(player_camera) {
                                    entity.face_east();
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    entity.turn_right(turn_step_deg);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_east();
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        entity.turn_right(turn_step_deg);
                    }
                }
                EntityAction::Backward => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_grid_camera(player_camera) {
                                    if Self::is_first_person_camera(player_camera) {
                                        let facing =
                                            Self::snapped_cardinal_direction(entity.orientation);
                                        let target = entity.get_pos_xz() - facing;
                                        self.queue_step_to(entity, target, facing);
                                    } else {
                                        entity.face_south();
                                        let target = entity.get_pos_xz() + Vec2::new(0.0, 1.0);
                                        self.queue_step_to(entity, target, Vec2::new(0.0, 1.0));
                                    }
                                } else if !Self::is_first_person_camera(player_camera) {
                                    entity.face_south();
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    self.move_entity(entity, -1.0, self.entity_block_mode);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_south();
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, -1.0, self.entity_block_mode);
                    }
                }
                EntityAction::StrafeLeft => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_first_person_camera(player_camera) {
                                    if Self::is_grid_camera(player_camera) {
                                        let facing =
                                            Self::snapped_cardinal_direction(entity.orientation);
                                        let step = Vec2::new(facing.y, -facing.x);
                                        let target = entity.get_pos_xz() + step;
                                        self.queue_step_to(entity, target, facing);
                                    } else {
                                        let right =
                                            Vec2::new(-entity.orientation.y, entity.orientation.x)
                                                .normalized();
                                        self.move_entity_by_vector(
                                            entity,
                                            -right * (self.movement_units_per_sec * redraw_dt),
                                            self.entity_block_mode,
                                        );
                                    }
                                } else {
                                    entity.action = EntityAction::Off;
                                }
                            }
                        } else {
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        entity.action = EntityAction::Off;
                    }
                }
                EntityAction::StrafeRight => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_first_person_camera(player_camera) {
                                    if Self::is_grid_camera(player_camera) {
                                        let facing =
                                            Self::snapped_cardinal_direction(entity.orientation);
                                        let step = Vec2::new(-facing.y, facing.x);
                                        let target = entity.get_pos_xz() + step;
                                        self.queue_step_to(entity, target, facing);
                                    } else {
                                        let right =
                                            Vec2::new(-entity.orientation.y, entity.orientation.x)
                                                .normalized();
                                        self.move_entity_by_vector(
                                            entity,
                                            right * (self.movement_units_per_sec * redraw_dt),
                                            self.entity_block_mode,
                                        );
                                    }
                                } else {
                                    entity.action = EntityAction::Off;
                                }
                            }
                        } else {
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        entity.action = EntityAction::Off;
                    }
                }
                EntityAction::ForwardLeft => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_grid_camera(player_camera) {
                                    self.activate_grid_desired_action(entity);
                                } else if !Self::is_first_person_camera(player_camera) {
                                    entity.set_orientation(vek::Vec2::new(-1.0, 1.0).normalized());
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    entity.turn_left(turn_step_deg);
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                }
                            }
                        } else {
                            entity.set_orientation(vek::Vec2::new(-1.0, 1.0).normalized());
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, 1.0, self.entity_block_mode);
                    }
                }
                EntityAction::ForwardRight => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_grid_camera(player_camera) {
                                    self.activate_grid_desired_action(entity);
                                } else if !Self::is_first_person_camera(player_camera) {
                                    entity.set_orientation(vek::Vec2::new(1.0, 1.0).normalized());
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    entity.turn_right(turn_step_deg);
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                }
                            }
                        } else {
                            entity.set_orientation(vek::Vec2::new(1.0, 1.0).normalized());
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, 1.0, self.entity_block_mode);
                    }
                }
                EntityAction::BackwardLeft => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_grid_camera(player_camera) {
                                    self.activate_grid_desired_action(entity);
                                } else if !Self::is_first_person_camera(player_camera) {
                                    entity.set_orientation(vek::Vec2::new(-1.0, -1.0).normalized());
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    entity.turn_left(turn_step_deg);
                                    self.move_entity(entity, -1.0, self.entity_block_mode);
                                }
                            }
                        } else {
                            entity.set_orientation(vek::Vec2::new(-1.0, -1.0).normalized());
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, -1.0, self.entity_block_mode);
                    }
                }
                EntityAction::BackwardRight => {
                    if entity.is_player() {
                        if !Self::should_use_directional_player_intent(entity, click_intents_2d) {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if Self::is_grid_camera(player_camera) {
                                    self.activate_grid_desired_action(entity);
                                } else if !Self::is_first_person_camera(player_camera) {
                                    entity.set_orientation(vek::Vec2::new(1.0, -1.0).normalized());
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    entity.turn_right(turn_step_deg);
                                    self.move_entity(entity, -1.0, self.entity_block_mode);
                                }
                            }
                        } else {
                            entity.set_orientation(vek::Vec2::new(1.0, -1.0).normalized());
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, -1.0, self.entity_block_mode);
                    }
                }
                EntityAction::CloseIn(target, target_radius, speed) => {
                    if is_entity_dead(self.id, *target) {
                        continue;
                    }

                    let position = entity.get_pos_xz();
                    let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
                    let target_id = *target;

                    let mut coord: Option<vek::Vec2<f32>> = None;

                    with_regionctx(self.id, |ctx| {
                        let speed: f32 = Self::close_in_step_distance(
                            ctx,
                            entity,
                            *speed,
                            self.movement_units_per_sec,
                        );

                        if let Some(entity) =
                            ctx.map.entities.iter().find(|entity| entity.id == *target)
                        {
                            coord = Some(entity.get_pos_xz());
                        }

                        if let Some(coord) = coord {
                            let use_3d_nav = self.collision_mode == CollisionMode::Mesh
                                && ctx.collision_world.has_collision_data();
                            let (new_position, new_y, arrived) = if use_3d_nav {
                                let (p, arrived) = ctx
                                    .collision_world
                                    .close_in_on_floors(
                                        position,
                                        coord,
                                        *target_radius,
                                        speed,
                                        radius,
                                        1.0,
                                        entity.position.y,
                                    )
                                    .unwrap_or_else(|| {
                                        let to_target = coord - position;
                                        let dist = to_target.magnitude();
                                        if dist <= *target_radius {
                                            (
                                                Vec3::new(
                                                    position.x,
                                                    entity.position.y,
                                                    position.y,
                                                ),
                                                true,
                                            )
                                        } else if dist <= f32::EPSILON {
                                            (
                                                Vec3::new(
                                                    position.x,
                                                    entity.position.y,
                                                    position.y,
                                                ),
                                                false,
                                            )
                                        } else {
                                            let step = to_target.normalized() * speed.min(dist);
                                            let start_3d = vek::Vec3::new(
                                                position.x,
                                                entity.position.y,
                                                position.y,
                                            );
                                            let step_3d = vek::Vec3::new(step.x, 0.0, step.y);
                                            let (end_3d, _) = ctx
                                                .collision_world
                                                .move_distance(start_3d, step_3d, radius);
                                            let end_2d = vek::Vec2::new(end_3d.x, end_3d.z);
                                            let arrived = self.close_in_arrived(
                                                ctx,
                                                end_2d,
                                                coord,
                                                *target_radius,
                                            );
                                            (end_3d, arrived)
                                        }
                                    });
                                (Vec2::new(p.x, p.z), p.y, arrived)
                            } else {
                                let (p, _arrived) = ctx.mapmini.close_in(
                                    position,
                                    coord,
                                    *target_radius,
                                    speed,
                                    radius,
                                    1.0,
                                );
                                let arrived = self.close_in_arrived(ctx, p, coord, *target_radius);
                                (p, entity.position.y, arrived)
                            };

                            let move_delta = new_position - position;
                            let moved_this_turn = move_delta.magnitude_squared() > 1e-6;
                            if moved_this_turn {
                                entity.set_orientation(move_delta.normalized());
                            }
                            entity.set_pos_xz(new_position);
                            entity.position.y = new_y;
                            if arrived {
                                entity.action = EntityAction::Off;

                                // Send closed in event
                                if let Some(_class_name) = ctx.entity_classes.get(&entity.id) {
                                    if moved_this_turn
                                        && !matches!(
                                            ctx.simulation_mode,
                                            crate::server::regionctx::SimulationMode::Realtime
                                        )
                                    {
                                        entity.set_attribute("target", Value::UInt(target_id));
                                        ctx.notifications_entities.push((
                                            entity.id,
                                            ctx.ticks + 1,
                                            "closed_in".into(),
                                        ));
                                    } else {
                                        ctx.to_execute_entity.push((
                                            entity.id,
                                            "closed_in".into(),
                                            VMValue::broadcast(target_id as f32),
                                        ));
                                    }
                                }
                            }

                            ctx.check_player_for_section_change(entity);
                        }
                    });
                }
                EntityAction::FollowAttack(target, speed, next_attack_tick) => {
                    let position = entity.get_pos_xz();
                    let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
                    let attacker_id = entity.id;
                    let target_id = *target;

                    with_regionctx(self.id, |ctx| {
                        let Some(target_entity) = ctx
                            .map
                            .entities
                            .iter()
                            .find(|candidate| {
                                candidate.id == target_id
                                    && candidate.get_mode() != "dead"
                                    && candidate.attributes.get_bool_default("visible", true)
                            })
                            .cloned()
                        else {
                            Self::end_follow_attack(ctx, entity, "lost");
                            return;
                        };

                        let target_pos = target_entity.get_pos_xz();
                        let leash_distance = ctx
                            .entity_proximity_alerts
                            .get(&attacker_id)
                            .copied()
                            .unwrap_or(5.0)
                            .max(1.5)
                            + 1.0;

                        if (target_pos - position).magnitude() > leash_distance {
                            Self::end_follow_attack(ctx, entity, "too_far");
                            return;
                        }

                        entity.set_attribute("target", Value::UInt(target_id));
                        entity.set_attribute("attack_target", Value::UInt(target_id));

                        if self.close_in_arrived(ctx, position, target_pos, 1.0) {
                            if ctx.ticks >= *next_attack_tick {
                                queue_entity_attack_damage(ctx, attacker_id, target_id);

                                let attack_time = entity
                                    .attributes
                                    .get_float_default("avatar_attack_time", 0.35)
                                    .max(0.05);
                                entity
                                    .set_attribute("avatar_attack_left", Value::Float(attack_time));

                                let next_tick =
                                    ctx.ticks + Self::follow_attack_cooldown_ticks(ctx, entity);
                                entity.action =
                                    EntityAction::FollowAttack(target_id, *speed, next_tick);
                            } else {
                                entity.action = EntityAction::FollowAttack(
                                    target_id,
                                    *speed,
                                    *next_attack_tick,
                                );
                            }
                            return;
                        }

                        let non_realtime_grid = !matches!(
                            ctx.simulation_mode,
                            crate::server::regionctx::SimulationMode::Realtime
                        ) && self.collision_mode != CollisionMode::Mesh;

                        let step_speed = if non_realtime_grid {
                            let speed_per_turn = (*speed).max(0.0);
                            let mut budget = entity
                                .attributes
                                .get_float_default("__follow_attack_budget", 0.0)
                                .max(0.0);
                            budget += speed_per_turn;
                            if budget + 1e-6 < 1.0 {
                                entity
                                    .set_attribute("__follow_attack_budget", Value::Float(budget));
                                entity.action = EntityAction::FollowAttack(
                                    target_id,
                                    *speed,
                                    *next_attack_tick,
                                );
                                return;
                            }
                            budget = (budget - 1.0).max(0.0);
                            entity.set_attribute("__follow_attack_budget", Value::Float(budget));
                            1.0
                        } else {
                            Self::close_in_step_distance(
                                ctx,
                                entity,
                                *speed,
                                self.movement_units_per_sec,
                            )
                        };

                        let use_3d_nav = self.collision_mode == CollisionMode::Mesh
                            && ctx.collision_world.has_collision_data();
                        let (new_position, new_y) = if use_3d_nav {
                            let (p, _) = ctx
                                .collision_world
                                .close_in_on_floors(
                                    position,
                                    target_pos,
                                    1.0,
                                    step_speed,
                                    radius,
                                    1.0,
                                    entity.position.y,
                                )
                                .unwrap_or_else(|| {
                                    let to_target = target_pos - position;
                                    let dist = to_target.magnitude();
                                    if dist <= f32::EPSILON {
                                        (
                                            Vec3::new(position.x, entity.position.y, position.y),
                                            false,
                                        )
                                    } else {
                                        let step = to_target.normalized() * step_speed.min(dist);
                                        let start_3d = vek::Vec3::new(
                                            position.x,
                                            entity.position.y,
                                            position.y,
                                        );
                                        let step_3d = vek::Vec3::new(step.x, 0.0, step.y);
                                        ctx.collision_world.move_distance(start_3d, step_3d, radius)
                                    }
                                });
                            (Vec2::new(p.x, p.z), p.y)
                        } else {
                            let (p, _) = ctx
                                .mapmini
                                .close_in(position, target_pos, 1.0, step_speed, radius, 1.0);
                            (p, entity.position.y)
                        };

                        let move_delta = new_position - position;
                        if move_delta.magnitude_squared() > 1e-6 {
                            entity.set_orientation(move_delta.normalized());
                        }
                        entity.set_pos_xz(new_position);
                        entity.position.y = new_y;
                        entity.action =
                            EntityAction::FollowAttack(target_id, *speed, *next_attack_tick);

                        ctx.check_player_for_section_change(entity);
                    });
                }
                EntityAction::Goto(coord, speed) => {
                    let position = entity.get_pos_xz();
                    let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
                    with_regionctx(self.id, |ctx| {
                        let speed = self.movement_units_per_sec
                            * speed
                            * Self::autonomous_action_dt(ctx, entity);

                        let use_3d_nav = self.collision_mode == CollisionMode::Mesh
                            && ctx.collision_world.has_collision_data();
                        let (new_position, new_y, mut arrived) = if use_3d_nav {
                            let (p, arrived) = ctx
                                .collision_world
                                .move_towards_on_floors(
                                    position,
                                    *coord,
                                    speed,
                                    radius,
                                    1.0,
                                    entity.position.y,
                                )
                                .unwrap_or_else(|| {
                                    let to_target = *coord - position;
                                    let dist = to_target.magnitude();
                                    if dist <= 0.05 {
                                        (Vec3::new(position.x, entity.position.y, position.y), true)
                                    } else if dist <= f32::EPSILON {
                                        (
                                            Vec3::new(position.x, entity.position.y, position.y),
                                            false,
                                        )
                                    } else {
                                        let step = to_target.normalized() * speed.min(dist);
                                        let start_3d = vek::Vec3::new(
                                            position.x,
                                            entity.position.y,
                                            position.y,
                                        );
                                        let step_3d = vek::Vec3::new(step.x, 0.0, step.y);
                                        let (end_3d, _) = ctx
                                            .collision_world
                                            .move_distance(start_3d, step_3d, radius);
                                        let end_2d = vek::Vec2::new(end_3d.x, end_3d.z);
                                        let arrived = (*coord - end_2d).magnitude() <= 0.05;
                                        (end_3d, arrived)
                                    }
                                });
                            (Vec2::new(p.x, p.z), p.y, arrived)
                        } else {
                            let (p, arrived) = ctx
                                .mapmini
                                .move_towards(position, *coord, speed, radius, 1.0);
                            (p, entity.position.y, arrived)
                        };

                        let mut resolved_position = new_position;
                        let probe =
                            self.probe_dynamic_collisions_in_ctx(ctx, entity, resolved_position);
                        if probe.blocking_collision {
                            resolved_position = position;
                            arrived = false;
                        }

                        let move_delta = resolved_position - position;
                        let old_dist = (*coord - position).magnitude();
                        let new_dist = (*coord - resolved_position).magnitude();
                        let progress = old_dist - new_dist;
                        let attempted_step = speed.min(old_dist).max(1e-4);
                        let min_movement = (attempted_step * 0.1).max(1e-4);
                        let min_movement_sq = min_movement * min_movement;

                        // Prevent facing jitter when repeatedly colliding/sliding near blockers.
                        if move_delta.magnitude_squared() > min_movement_sq && progress.abs() > 1e-6
                        {
                            entity.set_orientation(move_delta.normalized());
                        }
                        entity.set_pos_xz(resolved_position);
                        entity.position.y = new_y;

                        // Track long-running no-improvement oscillations near blockers.
                        // This catches "left/right flicker forever" where tiny movement happens
                        // but distance-to-goal never materially decreases.
                        let prev_tx = entity
                            .attributes
                            .get_float_default("__goto_target_x", coord.x);
                        let prev_ty = entity
                            .attributes
                            .get_float_default("__goto_target_y", coord.y);
                        let target_changed =
                            (prev_tx - coord.x).abs() > 0.01 || (prev_ty - coord.y).abs() > 0.01;
                        entity
                            .attributes
                            .set("__goto_target_x", Value::Float(coord.x));
                        entity
                            .attributes
                            .set("__goto_target_y", Value::Float(coord.y));

                        let mut best_dist = if target_changed {
                            old_dist
                        } else {
                            entity
                                .attributes
                                .get_float_default("__goto_best_dist", old_dist)
                        };
                        let mut no_improve_ticks = if target_changed {
                            0
                        } else {
                            entity
                                .attributes
                                .get_int_default("__goto_no_improve_ticks", 0)
                                .max(0)
                        };

                        if probe.blocking_collision {
                            no_improve_ticks = 0;
                        } else if new_dist + attempted_step < best_dist {
                            best_dist = new_dist;
                            no_improve_ticks = 0;
                        } else if move_delta.magnitude_squared() <= min_movement_sq {
                            no_improve_ticks += 1;
                        } else {
                            no_improve_ticks = 0;
                        }
                        entity
                            .attributes
                            .set("__goto_best_dist", Value::Float(best_dist));
                        entity
                            .attributes
                            .set("__goto_no_improve_ticks", Value::Int(no_improve_ticks));
                        let mut stall_ticks = entity
                            .attributes
                            .get_int_default("__goto_stall_ticks", 0)
                            .max(0);
                        if probe.blocking_collision {
                            stall_ticks = 0;
                        } else if move_delta.magnitude_squared() <= min_movement_sq {
                            stall_ticks += 2;
                        } else {
                            stall_ticks = 0;
                        }
                        entity
                            .attributes
                            .set("__goto_stall_ticks", Value::Int(stall_ticks));

                        if arrived {
                            entity.attributes.set("__goto_stall_ticks", Value::Int(0));
                            entity
                                .attributes
                                .set("__goto_no_improve_ticks", Value::Int(0));
                            entity.action = EntityAction::Off;

                            let mut sector_name: String = String::new();
                            {
                                if let Some(s) = ctx.map.find_sector_at(resolved_position) {
                                    sector_name = s.name.clone();
                                }
                            }

                            // Send arrived event
                            if let Some(_class_name) = ctx.entity_classes.get(&entity.id) {
                                // let cmd =
                                //     format!("{}.event('arrived', \"{}\")", class_name, sector_name);
                                ctx.to_execute_entity.push((
                                    entity.id,
                                    "arrived".into(),
                                    VMValue::from(sector_name),
                                ));
                            }
                        } else if stall_ticks >= 8 || no_improve_ticks >= 16 {
                            // Give up this goto target when we are clearly oscillating/stuck.
                            entity.attributes.set("__goto_stall_ticks", Value::Int(0));
                            entity
                                .attributes
                                .set("__goto_no_improve_ticks", Value::Int(0));
                            entity.action = EntityAction::Off;
                        };
                        ctx.check_player_for_section_change(entity);
                    });
                }
                EntityAction::GotoGrid(coord, speed) => {
                    with_regionctx(self.id, |ctx| {
                        let step =
                            self.compute_grid_goto_step_in_ctx(ctx, entity.get_pos_xz(), *coord);
                        entity.set_attribute("__grid_goto_speed", Value::Float(*speed));
                        if let Some((next, facing, target)) = step {
                            let start = entity.get_pos_xz();
                            let step_dir = next - start;
                            entity.set_orientation(facing);
                            entity.action =
                                EntityAction::StepTo(next, *speed, facing, start, step_dir);
                            entity.set_attribute(
                                "__grid_goto_target",
                                Value::Str(format!("{},{}", target.x, target.y)),
                            );
                        } else {
                            entity.set_attribute("__grid_goto_target", Value::Str(String::new()));
                            entity.action = EntityAction::Off;
                        }
                    });
                }
                EntityAction::StepTo(coord, speed, facing, start, step_dir) => {
                    with_regionctx(self.id, |ctx| {
                        let mut remaining_speed =
                            self.movement_units_per_sec * speed * ctx.delta_time;
                        let mut curr_coord = *coord;
                        let mut curr_facing = *facing;
                        let mut curr_start = *start;
                        let mut curr_step_dir = *step_dir;

                        for _ in 0..4 {
                            let position = entity.get_pos_xz();
                            let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
                            let use_3d_nav = self.collision_mode == CollisionMode::Mesh
                                && ctx.collision_world.has_collision_data();
                            let (new_position, new_y, arrived, geometry_blocked, dynamic_collision) =
                                if use_3d_nav {
                                    let to_target = curr_coord - position;
                                    let dist = to_target.magnitude();
                                    if dist <= 0.05 {
                                        (position, entity.position.y, true, false, false)
                                    } else {
                                        let step =
                                            to_target.normalized() * remaining_speed.min(dist);
                                        let probe = self.probe_dynamic_collisions_in_ctx(
                                            ctx,
                                            entity,
                                            position + step,
                                        );
                                        if probe.blocking_collision {
                                            (position, entity.position.y, false, false, true)
                                        } else {
                                            let (p, arrived) = ctx
                                                .collision_world
                                                .move_towards_on_floors(
                                                    position,
                                                    curr_coord,
                                                    remaining_speed,
                                                    radius,
                                                    1.0,
                                                    entity.position.y,
                                                )
                                                .unwrap_or_else(|| {
                                                    let to_target = curr_coord - position;
                                                    let dist = to_target.magnitude();
                                                    if dist <= 0.05 {
                                                        (
                                                            Vec3::new(
                                                                position.x,
                                                                entity.position.y,
                                                                position.y,
                                                            ),
                                                            true,
                                                        )
                                                    } else if dist <= f32::EPSILON {
                                                        (
                                                            Vec3::new(
                                                                position.x,
                                                                entity.position.y,
                                                                position.y,
                                                            ),
                                                            false,
                                                        )
                                                    } else {
                                                        let step = to_target.normalized()
                                                            * remaining_speed.min(dist);
                                                        let start_3d = vek::Vec3::new(
                                                            position.x,
                                                            entity.position.y,
                                                            position.y,
                                                        );
                                                        let step_3d =
                                                            vek::Vec3::new(step.x, 0.0, step.y);
                                                        let (end_3d, _) =
                                                            ctx.collision_world.move_distance(
                                                                start_3d, step_3d, radius,
                                                            );
                                                        let end_2d =
                                                            vek::Vec2::new(end_3d.x, end_3d.z);
                                                        let arrived = (curr_coord - end_2d)
                                                            .magnitude()
                                                            <= 0.05;
                                                        (end_3d, arrived)
                                                    }
                                                });
                                            (Vec2::new(p.x, p.z), p.y, arrived, false, false)
                                        }
                                    }
                                } else {
                                    let to_target = curr_coord - position;
                                    let dist = to_target.magnitude();
                                    if dist <= 0.05 {
                                        (position, entity.position.y, true, false, false)
                                    } else {
                                        let step =
                                            to_target.normalized() * remaining_speed.min(dist);
                                        let move_result = self
                                            .move_entity_by_vector_with_result_in_ctx(
                                                ctx,
                                                entity,
                                                step,
                                                self.entity_block_mode,
                                            );
                                        let p = entity.get_pos_xz();
                                        let y = entity.position.y;
                                        let arrived = (curr_coord - p).magnitude() <= 0.05;
                                        (
                                            p,
                                            y,
                                            arrived,
                                            move_result.geometry_blocked,
                                            move_result.dynamic_collision,
                                        )
                                    }
                                };

                            let move_delta = new_position - position;
                            let progress = (curr_coord - position).magnitude()
                                - (curr_coord - new_position).magnitude();
                            let axis = if curr_step_dir.magnitude_squared() > 1e-6 {
                                curr_step_dir.normalized()
                            } else {
                                curr_facing
                            };
                            let start_anchor = Self::snapped_grid_center(curr_start);
                            let axis_stable = if axis.x.abs() > axis.y.abs() {
                                let start_lateral = (curr_start.y - start_anchor.y).abs();
                                let new_lateral = (new_position.y - start_anchor.y).abs();
                                new_lateral <= (start_lateral + 0.05)
                            } else {
                                let start_lateral = (curr_start.x - start_anchor.x).abs();
                                let new_lateral = (new_position.x - start_anchor.x).abs();
                                new_lateral <= (start_lateral + 0.05)
                            };
                            let blocked = geometry_blocked
                                || (!arrived && progress <= 0.0005)
                                || (!arrived && !axis_stable)
                                || (move_delta.magnitude_squared() <= 1e-8 && !arrived);

                            if blocked {
                                entity.set_pos_xz(curr_start);
                                entity.set_orientation(curr_facing);
                                if dynamic_collision {
                                    if let Some(target) =
                                        Self::parse_vec2_attr(entity, "__grid_goto_target")
                                    {
                                        let goto_speed = entity
                                            .attributes
                                            .get_float_default("__grid_goto_speed", *speed);
                                        entity.action = EntityAction::GotoGrid(target, goto_speed);
                                    } else {
                                        entity.action = EntityAction::Off;
                                    }
                                    Self::clear_grid_blocked_action(entity);
                                } else {
                                    entity.action = EntityAction::Off;
                                    Self::set_blocked_grid_action(
                                        entity,
                                        &Self::grid_desired_action(entity),
                                    );
                                }
                                break;
                            }

                            entity.set_pos_xz(new_position);
                            entity.position.y = new_y;
                            entity.set_orientation(curr_facing);

                            if !arrived {
                                break;
                            }

                            let traveled = move_delta.magnitude();
                            remaining_speed = (remaining_speed - traveled).max(0.0);

                            entity.set_pos_xz(curr_coord);
                            entity.set_orientation(curr_facing);
                            Self::clear_grid_blocked_action(entity);
                            let grid_goto_target =
                                Self::parse_vec2_attr(entity, "__grid_goto_target");
                            let player_camera = match entity.attributes.get("player_camera") {
                                Some(Value::PlayerCamera(player_camera)) => {
                                    Some(player_camera.clone())
                                }
                                _ => None,
                            };
                            let mut continue_grid_chain = true;
                            if let Some(target) = grid_goto_target {
                                if (target - curr_coord).magnitude_squared() <= 0.001 {
                                    entity.set_attribute(
                                        "__grid_goto_target",
                                        Value::Str(String::new()),
                                    );
                                    entity.set_attribute("__grid_goto_speed", Value::Float(1.0));
                                    entity.action = EntityAction::Off;
                                } else {
                                    let goto_speed = entity
                                        .attributes
                                        .get_float_default("__grid_goto_speed", *speed);
                                    entity.action = EntityAction::GotoGrid(target, goto_speed);
                                }
                            } else if let Some(player_camera) = player_camera {
                                if matches!(
                                    ctx.simulation_mode,
                                    crate::server::regionctx::SimulationMode::Realtime
                                ) {
                                    self.queue_grid_action_from_desired(entity, &player_camera);
                                } else if self
                                    .queue_grid_action_from_desired(entity, &player_camera)
                                {
                                    self.queue_simulation_step();
                                    continue_grid_chain = false;
                                } else {
                                    entity.action = EntityAction::Off;
                                }
                            } else {
                                entity.action = EntityAction::Off;
                            }

                            if remaining_speed <= 0.0001 || !continue_grid_chain {
                                break;
                            }

                            match entity.action.clone() {
                                EntityAction::StepTo(
                                    next_coord,
                                    _,
                                    next_facing,
                                    next_start,
                                    next_step_dir,
                                ) => {
                                    curr_coord = next_coord;
                                    curr_facing = next_facing;
                                    curr_start = next_start;
                                    curr_step_dir = next_step_dir;
                                }
                                _ => break,
                            }
                        }

                        ctx.check_player_for_section_change(entity);
                    });
                }
                EntityAction::RotateTo(target) => {
                    let finished = Self::rotate_towards_cardinal(entity, *target, turn_step_deg);
                    if finished {
                        let simulation_mode = with_regionctx(self.id, |ctx| ctx.simulation_mode)
                            .unwrap_or(crate::server::regionctx::SimulationMode::Realtime);
                        Self::clear_grid_blocked_action(entity);
                        let player_camera = match entity.attributes.get("player_camera") {
                            Some(Value::PlayerCamera(player_camera)) => Some(player_camera.clone()),
                            _ => None,
                        };
                        let grid_goto_target = Self::parse_vec2_attr(entity, "__grid_goto_target");
                        if let Some(target) = grid_goto_target {
                            if (target - entity.get_pos_xz().map(|value| value.floor()))
                                .magnitude_squared()
                                <= 0.001
                            {
                                entity
                                    .set_attribute("__grid_goto_target", Value::Str(String::new()));
                                entity.set_attribute("__grid_goto_speed", Value::Float(1.0));
                                entity.action = EntityAction::Off;
                            } else {
                                let goto_speed = entity
                                    .attributes
                                    .get_float_default("__grid_goto_speed", 1.0);
                                entity.action = EntityAction::GotoGrid(target, goto_speed);
                            }
                        } else if let Some(player_camera) = player_camera {
                            if matches!(
                                simulation_mode,
                                crate::server::regionctx::SimulationMode::Realtime
                            ) {
                                self.queue_grid_action_from_desired(entity, &player_camera);
                            } else if self.queue_grid_action_from_desired(entity, &player_camera) {
                                self.queue_simulation_step();
                            } else {
                                entity.action = EntityAction::Off;
                            }
                        } else {
                            entity.action = EntityAction::Off;
                        }
                    }
                }
                EntityAction::RandomWalk(distance, speed, max_sleep, state, target) => {
                    if *state == 0 {
                        // State 0: Uninitialized, find a target location.
                        let curr_pos = entity.get_pos_xz();
                        let mut next_pos = curr_pos;
                        let mut found = false;

                        with_regionctx(self.id, |ctx| {
                            let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;

                            // Prefer discrete nearby tile centers first. In narrow 2D spaces
                            // (for example behind counters) arbitrary points on a distance-radius
                            // circle are almost always invalid, even though left/right tile moves
                            // are perfectly fine.
                            let curr_tile = curr_pos.map(|c| c.floor() as i32);
                            let curr_center = curr_tile.map(|i| i as f32) + Vec2::broadcast(0.5);
                            let max_steps = (*distance).ceil().max(1.0) as i32;
                            let mut center_candidates = Vec::new();

                            if (curr_center - curr_pos).magnitude() > 0.05
                                && ctx.mapmini.is_walkable_position(curr_center, radius)
                            {
                                center_candidates.push(curr_center);
                            }

                            for y in -max_steps..=max_steps {
                                for x in -max_steps..=max_steps {
                                    let manhattan = x.abs() + y.abs();
                                    if manhattan == 0 || manhattan > max_steps {
                                        continue;
                                    }
                                    let tile = curr_tile + Vec2::new(x, y);
                                    let center = tile.map(|i| i as f32) + Vec2::broadcast(0.5);
                                    if ctx.mapmini.is_walkable_position(center, radius) {
                                        center_candidates.push(center);
                                    }
                                }
                            }

                            let mut rng = rand::rng();
                            center_candidates.shuffle(&mut rng);

                            if let Some(candidate) = center_candidates.into_iter().next() {
                                next_pos = candidate;
                                found = true;
                                return;
                            }

                            for _ in 0..16 {
                                let candidate = find_random_position(curr_pos, *distance);
                                if ctx.mapmini.is_walkable_position(candidate, radius) {
                                    next_pos = candidate;
                                    found = true;
                                    break;
                                }
                            }

                            if !found {
                                let min_sleep = (*max_sleep / 2).max(1);
                                let max_sleep_guard = (*max_sleep).max(1);
                                let sleep_minutes =
                                    rng.random_range(min_sleep..=max_sleep_guard) as u32;
                                let wake_tick = ctx.ticks
                                    + Self::scheduled_delay_ticks(ctx, sleep_minutes as f32);
                                entity.action = SleepAndSwitch(
                                    wake_tick,
                                    Box::new(RandomWalk(
                                        *distance, *speed, *max_sleep, 0, curr_pos,
                                    )),
                                );
                            }
                        });

                        if found {
                            entity.action = RandomWalk(*distance, *speed, *max_sleep, 1, next_pos);
                            entity.face_at(next_pos);
                        }
                    } else if *state == 1 {
                        // State 1: Walk towards
                        if target.distance(entity.get_pos_xz()) < 0.1 {
                            // Arrived, Sleep
                            let mut rng = rand::rng();
                            entity.action = self.create_sleep_switch_action(
                                rng.random_range(*max_sleep / 2..=*max_sleep) as u32,
                                RandomWalk(*distance, *speed, *max_sleep, 0, *target),
                            );
                        } else {
                            let max_sleep = *max_sleep;
                            with_regionctx(self.id, |ctx| {
                                let position = entity.get_pos_xz();
                                let radius =
                                    entity.attributes.get_float_default("radius", 0.5) - 0.01;
                                // Keep RandomWalk speed behavior aligned with legacy move_entity().
                                let step_speed = self.movement_units_per_sec
                                    * Self::autonomous_action_dt(ctx, entity);
                                let terrain_cfg =
                                    crate::chunkbuilder::terrain_generator::TerrainConfig::default(
                                    );
                                let terrain_y =
                                    crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                                        &ctx.map,
                                        position,
                                        &terrain_cfg,
                                    );
                                let is_elevated_floor =
                                    (entity.position.y - terrain_y).abs() > 0.25;
                                let use_3d_nav = ctx.collision_world.has_collision_data()
                                    && (self.collision_mode == CollisionMode::Mesh
                                        || is_elevated_floor);

                                let mut mesh_blocked = false;
                                let (new_position, new_y, mut arrived) = if use_3d_nav {
                                    let (desired_position, arrived_hint) = ctx
                                        .collision_world
                                        .move_towards_on_floors(
                                            position,
                                            *target,
                                            step_speed,
                                            radius,
                                            1.0,
                                            entity.position.y,
                                        )
                                        .unwrap_or_else(|| {
                                            let to_target = *target - position;
                                            let dist = to_target.magnitude();
                                            if dist <= 0.1 {
                                                (
                                                    Vec3::new(
                                                        position.x,
                                                        entity.position.y,
                                                        position.y,
                                                    ),
                                                    true,
                                                )
                                            } else if dist <= f32::EPSILON {
                                                (
                                                    Vec3::new(
                                                        position.x,
                                                        entity.position.y,
                                                        position.y,
                                                    ),
                                                    false,
                                                )
                                            } else {
                                                let step =
                                                    to_target.normalized() * step_speed.min(dist);
                                                (
                                                    Vec3::new(
                                                        position.x + step.x,
                                                        entity.position.y,
                                                        position.y + step.y,
                                                    ),
                                                    false,
                                                )
                                            }
                                        });

                                    let desired_move =
                                        Vec2::new(desired_position.x, desired_position.z)
                                            - position;
                                    let start_3d =
                                        vek::Vec3::new(position.x, entity.position.y, position.y);
                                    let step_3d =
                                        vek::Vec3::new(desired_move.x, 0.0, desired_move.y);
                                    let (end_3d, blocked) = ctx
                                        .collision_world
                                        .move_distance(start_3d, step_3d, radius);
                                    mesh_blocked = blocked;
                                    let end_2d = vek::Vec2::new(end_3d.x, end_3d.z);
                                    let arrived = arrived_hint
                                        && !blocked
                                        && (*target - end_2d).magnitude() <= 0.1;
                                    (end_2d, end_3d.y, arrived)
                                } else {
                                    let (p, arrived) = ctx
                                        .mapmini
                                        .move_towards(position, *target, step_speed, radius, 1.0);
                                    (p, entity.position.y, arrived)
                                };

                                let mut dynamic_blocked = false;
                                let mut resolved_position = new_position;

                                if self.entity_block_mode > 0 {
                                    for other in ctx.map.entities.iter() {
                                        if other.id == entity.id || other.get_mode() == "dead" {
                                            continue;
                                        }
                                        let other_pos = other.get_pos_xz();
                                        let other_radius =
                                            other.attributes.get_float_default("radius", 0.5)
                                                - 0.01;
                                        let combined = radius + other_radius;
                                        if (resolved_position - other_pos).magnitude_squared()
                                            < combined * combined
                                        {
                                            dynamic_blocked = true;
                                            resolved_position = position;
                                            break;
                                        }
                                    }
                                }

                                if !dynamic_blocked {
                                    for other in ctx.map.items.iter() {
                                        if !other.attributes.get_bool_default("visible", false)
                                            || !other.attributes.get_bool_default("blocking", false)
                                        {
                                            continue;
                                        }
                                        let other_pos = other.get_pos_xz();
                                        let other_radius =
                                            other.attributes.get_float_default("radius", 0.5)
                                                - 0.01;
                                        let combined = radius + other_radius;
                                        if (resolved_position - other_pos).magnitude_squared()
                                            < combined * combined
                                        {
                                            dynamic_blocked = true;
                                            resolved_position = position;
                                            break;
                                        }
                                    }
                                }

                                if dynamic_blocked {
                                    arrived = false;
                                }

                                let move_delta = resolved_position - position;
                                let old_dist = (*target - position).magnitude();
                                let new_dist = (*target - resolved_position).magnitude();
                                let progress = old_dist - new_dist;

                                if move_delta.magnitude_squared() > 1e-6 && progress > 0.002 {
                                    entity.set_orientation(move_delta.normalized());
                                }
                                entity.set_pos_xz(resolved_position);
                                entity.position.y = new_y;

                                let floor_ref_y = entity.position.y;
                                let sector_floor = sector_floor_height_below_or_nearest(
                                    &ctx.map,
                                    resolved_position,
                                    floor_ref_y,
                                );
                                let collision_floor = if use_3d_nav {
                                    ctx.collision_world
                                        .get_floor_height_nearest(resolved_position, floor_ref_y)
                                } else {
                                    None
                                };
                                let terrain_floor = {
                                    let config =
                                        crate::chunkbuilder::terrain_generator::TerrainConfig::default();
                                    crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                                        &ctx.map,
                                        resolved_position,
                                        &config,
                                    )
                                };

                                let base_y =
                                    sector_floor.or(collision_floor).or(Some(terrain_floor));
                                if let Some(y) = base_y {
                                    entity.position.y = y;
                                }

                                let mut stall_ticks = entity
                                    .attributes
                                    .get_int_default("__rw_stall_ticks", 0)
                                    .max(0);
                                if progress < 0.01 {
                                    stall_ticks += 1;
                                } else {
                                    stall_ticks = 0;
                                }
                                if mesh_blocked || dynamic_blocked {
                                    stall_ticks += 2;
                                }
                                entity
                                    .attributes
                                    .set("__rw_stall_ticks", Value::Int(stall_ticks));

                                if arrived
                                    || move_delta.magnitude_squared() <= 1e-8
                                    || stall_ticks >= 8
                                {
                                    entity.attributes.set("__rw_stall_ticks", Value::Int(0));
                                    let mut rng = rand::rng();
                                    let min_sleep = (max_sleep / 2).max(1);
                                    let max_sleep_guard = max_sleep.max(1);
                                    let sleep_minutes =
                                        rng.random_range(min_sleep..=max_sleep_guard) as u32;
                                    let wake_tick = ctx.ticks
                                        + Self::scheduled_delay_ticks(ctx, sleep_minutes as f32);
                                    entity.action = SleepAndSwitch(
                                        wake_tick,
                                        Box::new(RandomWalk(
                                            *distance, *speed, max_sleep, 0, *target,
                                        )),
                                    );
                                }

                                ctx.check_player_for_section_change(entity);
                            });
                        }
                    }
                }
                EntityAction::RandomWalkInSector(distance, speed, max_sleep, state, target) => {
                    if *state == 0 {
                        // State 0: Uninitialized, find a target location.
                        let curr_pos = entity.get_pos_xz().clone();
                        with_regionctx(self.id, |ctx| {
                            if let Some(sector) = ctx.map.find_sector_at(curr_pos) {
                                let radius =
                                    entity.attributes.get_float_default("radius", 0.5) - 0.01;
                                let mut new_pos = find_random_position(curr_pos, *distance);
                                let mut found = false;

                                for _ in 0..16 {
                                    if sector.is_inside(&ctx.map, new_pos)
                                        && ctx.mapmini.is_walkable_position(new_pos, radius)
                                    {
                                        found = true;
                                        break;
                                    } else {
                                        new_pos = find_random_position(curr_pos, *distance);
                                    }
                                }

                                if found {
                                    entity.action = RandomWalkInSector(
                                        *distance, *speed, *max_sleep, 1, new_pos,
                                    );
                                    entity.face_at(new_pos);
                                } else {
                                    entity.action = RandomWalkInSector(
                                        *distance, *speed, *max_sleep, 0, curr_pos,
                                    );
                                }
                            }
                        });
                    } else if *state == 1 {
                        // State 1: Walk towards
                        if target.distance(entity.get_pos_xz()) < 0.1 {
                            // Arrived, Sleep
                            let mut rng = rand::rng();
                            entity.action = self.create_sleep_switch_action(
                                rng.random_range(*max_sleep / 2..=*max_sleep) as u32,
                                RandomWalkInSector(*distance, *speed, *max_sleep, 0, *target),
                            );
                        } else {
                            let max_sleep = *max_sleep;
                            with_regionctx(self.id, |ctx| {
                                let position = entity.get_pos_xz();
                                let radius =
                                    entity.attributes.get_float_default("radius", 0.5) - 0.01;
                                // Keep RandomWalkInSector speed behavior aligned with legacy move_entity().
                                let step_speed = self.movement_units_per_sec
                                    * Self::autonomous_action_dt(ctx, entity);
                                let terrain_cfg =
                                    crate::chunkbuilder::terrain_generator::TerrainConfig::default(
                                    );
                                let terrain_y =
                                    crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                                        &ctx.map,
                                        position,
                                        &terrain_cfg,
                                    );
                                let is_elevated_floor =
                                    (entity.position.y - terrain_y).abs() > 0.25;
                                // Use mesh nav either when explicitly configured, or when clearly on an
                                // elevated/interior floor where tile/terrain movement is invalid.
                                let use_3d_nav = ctx.collision_world.has_collision_data()
                                    && (self.collision_mode == CollisionMode::Mesh
                                        || is_elevated_floor);

                                let mut mesh_blocked = false;
                                let (new_position, new_y, mut arrived) = if use_3d_nav {
                                    let (desired_position, arrived_hint) = ctx
                                        .collision_world
                                        .move_towards_on_floors(
                                            position,
                                            *target,
                                            step_speed,
                                            radius,
                                            1.0,
                                            entity.position.y,
                                        )
                                        .unwrap_or_else(|| {
                                            let to_target = *target - position;
                                            let dist = to_target.magnitude();
                                            if dist <= 0.1 {
                                                (
                                                    Vec3::new(
                                                        position.x,
                                                        entity.position.y,
                                                        position.y,
                                                    ),
                                                    true,
                                                )
                                            } else if dist <= f32::EPSILON {
                                                (
                                                    Vec3::new(
                                                        position.x,
                                                        entity.position.y,
                                                        position.y,
                                                    ),
                                                    false,
                                                )
                                            } else {
                                                let step =
                                                    to_target.normalized() * step_speed.min(dist);
                                                (
                                                    Vec3::new(
                                                        position.x + step.x,
                                                        entity.position.y,
                                                        position.y + step.y,
                                                    ),
                                                    false,
                                                )
                                            }
                                        });

                                    // Always clamp the nav step against full mesh collision so
                                    // walls/furniture cannot be crossed.
                                    let desired_move =
                                        Vec2::new(desired_position.x, desired_position.z)
                                            - position;
                                    let start_3d =
                                        vek::Vec3::new(position.x, entity.position.y, position.y);
                                    let step_3d =
                                        vek::Vec3::new(desired_move.x, 0.0, desired_move.y);
                                    let (end_3d, blocked) = ctx
                                        .collision_world
                                        .move_distance(start_3d, step_3d, radius);
                                    mesh_blocked = blocked;
                                    let end_2d = vek::Vec2::new(end_3d.x, end_3d.z);
                                    let arrived = arrived_hint
                                        && !blocked
                                        && (*target - end_2d).magnitude() <= 0.1;
                                    (end_2d, end_3d.y, arrived)
                                } else {
                                    let (p, arrived) = ctx
                                        .mapmini
                                        .move_towards(position, *target, step_speed, radius, 1.0);
                                    (p, entity.position.y, arrived)
                                };

                                // Keep dynamic blocking (entities/items) behavior:
                                // prevent entering blocking actor/item circles even when mesh nav says clear.
                                let mut dynamic_blocked = false;
                                let mut resolved_position = new_position;

                                // Entity blocking (depends on entity_block_mode)
                                if self.entity_block_mode > 0 {
                                    for other in ctx.map.entities.iter() {
                                        if other.id == entity.id || other.get_mode() == "dead" {
                                            continue;
                                        }
                                        let other_pos = other.get_pos_xz();
                                        let other_radius =
                                            other.attributes.get_float_default("radius", 0.5)
                                                - 0.01;
                                        let combined = radius + other_radius;
                                        if (resolved_position - other_pos).magnitude_squared()
                                            < combined * combined
                                        {
                                            dynamic_blocked = true;
                                            resolved_position = position;
                                            break;
                                        }
                                    }
                                }

                                // Item blocking
                                if !dynamic_blocked {
                                    for other in ctx.map.items.iter() {
                                        if !other.attributes.get_bool_default("visible", false)
                                            || !other.attributes.get_bool_default("blocking", false)
                                        {
                                            continue;
                                        }
                                        let other_pos = other.get_pos_xz();
                                        let other_radius =
                                            other.attributes.get_float_default("radius", 0.5)
                                                - 0.01;
                                        let combined = radius + other_radius;
                                        if (resolved_position - other_pos).magnitude_squared()
                                            < combined * combined
                                        {
                                            dynamic_blocked = true;
                                            resolved_position = position;
                                            break;
                                        }
                                    }
                                }

                                if dynamic_blocked {
                                    arrived = false;
                                }

                                let move_delta = resolved_position - position;
                                let old_dist = (*target - position).magnitude();
                                let new_dist = (*target - resolved_position).magnitude();
                                let progress = old_dist - new_dist;

                                // Avoid rapid facing flips when colliding/sliding with near-zero
                                // progress (classic jitter case in tight interiors).
                                if move_delta.magnitude_squared() > 1e-6 && progress > 0.002 {
                                    entity.set_orientation(move_delta.normalized());
                                }
                                entity.set_pos_xz(resolved_position);
                                entity.position.y = new_y;

                                // Keep Y aligned to walking sector first (RPG behavior),
                                // then fall back to collision floor/terrain.
                                let floor_ref_y = entity.position.y;
                                let sector_floor = sector_floor_height_below_or_nearest(
                                    &ctx.map,
                                    resolved_position,
                                    floor_ref_y,
                                );
                                let collision_floor = if use_3d_nav {
                                    ctx.collision_world
                                        .get_floor_height_nearest(resolved_position, floor_ref_y)
                                } else {
                                    None
                                };
                                let terrain_floor = {
                                    let config =
                                        crate::chunkbuilder::terrain_generator::TerrainConfig::default();
                                    crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                                        &ctx.map,
                                        resolved_position,
                                        &config,
                                    )
                                };

                                let base_y =
                                    sector_floor.or(collision_floor).or(Some(terrain_floor));
                                if let Some(y) = base_y {
                                    entity.position.y = y;
                                }

                                // Track repeated no-progress frames and abandon this waypoint if
                                // we keep oscillating near obstacles.
                                let mut stall_ticks = entity
                                    .attributes
                                    .get_int_default("__rwis_stall_ticks", 0)
                                    .max(0);
                                if progress < 0.01 {
                                    stall_ticks += 1;
                                } else {
                                    stall_ticks = 0;
                                }
                                if mesh_blocked || dynamic_blocked {
                                    stall_ticks += 2;
                                }
                                entity
                                    .attributes
                                    .set("__rwis_stall_ticks", Value::Int(stall_ticks));

                                if arrived {
                                    entity.attributes.set("__rwis_stall_ticks", Value::Int(0));
                                    let mut rng = rand::rng();
                                    let min_sleep = (max_sleep / 2).max(1);
                                    let max_sleep_guard = max_sleep.max(1);
                                    let sleep_minutes =
                                        rng.random_range(min_sleep..=max_sleep_guard) as u32;
                                    let wake_tick = ctx.ticks
                                        + Self::scheduled_delay_ticks(ctx, sleep_minutes as f32);
                                    entity.action = SleepAndSwitch(
                                        wake_tick,
                                        Box::new(RandomWalkInSector(
                                            *distance, *speed, max_sleep, 0, *target,
                                        )),
                                    );
                                } else if move_delta.magnitude_squared() <= 1e-8 || stall_ticks >= 8
                                {
                                    // Stuck against geometry/obstacle: pause, then pick a fresh target.
                                    entity.attributes.set("__rwis_stall_ticks", Value::Int(0));
                                    let mut rng = rand::rng();
                                    let min_sleep = (max_sleep / 2).max(1);
                                    let max_sleep_guard = max_sleep.max(1);
                                    let sleep_minutes =
                                        rng.random_range(min_sleep..=max_sleep_guard) as u32;
                                    let wake_tick = ctx.ticks
                                        + Self::scheduled_delay_ticks(ctx, sleep_minutes as f32);
                                    entity.action = SleepAndSwitch(
                                        wake_tick,
                                        Box::new(RandomWalkInSector(
                                            *distance, *speed, max_sleep, 0, *target,
                                        )),
                                    );
                                }

                                ctx.check_player_for_section_change(entity);
                            });
                        }
                    }
                }
                EntityAction::Patrol {
                    points,
                    route_wait,
                    route_speed,
                    route_mode,
                    point_index,
                    forward,
                    wait_until_tick,
                } => {
                    if points.is_empty() {
                        entity.action = EntityAction::Off;
                    } else {
                        with_regionctx(self.id, |ctx| {
                            let points = points.clone();
                            if points.is_empty() {
                                entity.action = EntityAction::Off;
                                return;
                            }

                            let len = points.len();
                            let mut idx = (*point_index).min(len - 1);
                            let mut fwd = *forward;
                            let mut wait_until = *wait_until_tick;

                            if wait_until > ctx.ticks {
                                entity.action = EntityAction::Patrol {
                                    points,
                                    route_wait: *route_wait,
                                    route_speed: *route_speed,
                                    route_mode: route_mode.clone(),
                                    point_index: idx,
                                    forward: fwd,
                                    wait_until_tick: wait_until,
                                };
                                return;
                            }

                            let target = points[idx];
                            let position = entity.get_pos_xz();
                            let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
                            let speed = self.movement_units_per_sec
                                * *route_speed
                                * Self::autonomous_action_dt(ctx, entity);

                            let use_3d_nav = self.collision_mode == CollisionMode::Mesh
                                && ctx.collision_world.has_collision_data();
                            let (new_position, new_y, arrived) = if use_3d_nav {
                                let (p, arrived) = ctx
                                    .collision_world
                                    .move_towards_on_floors(
                                        position,
                                        target,
                                        speed,
                                        radius,
                                        1.0,
                                        entity.position.y,
                                    )
                                    .unwrap_or_else(|| {
                                        let to_target = target - position;
                                        let dist = to_target.magnitude();
                                        if dist <= 0.05 {
                                            (
                                                Vec3::new(
                                                    position.x,
                                                    entity.position.y,
                                                    position.y,
                                                ),
                                                true,
                                            )
                                        } else if dist <= f32::EPSILON {
                                            (
                                                Vec3::new(
                                                    position.x,
                                                    entity.position.y,
                                                    position.y,
                                                ),
                                                false,
                                            )
                                        } else {
                                            let step = to_target.normalized() * speed.min(dist);
                                            let start_3d = vek::Vec3::new(
                                                position.x,
                                                entity.position.y,
                                                position.y,
                                            );
                                            let step_3d = vek::Vec3::new(step.x, 0.0, step.y);
                                            let (end_3d, _) = ctx
                                                .collision_world
                                                .move_distance(start_3d, step_3d, radius);
                                            let end_2d = vek::Vec2::new(end_3d.x, end_3d.z);
                                            let arrived = (target - end_2d).magnitude() <= 0.05;
                                            (end_3d, arrived)
                                        }
                                    });
                                (Vec2::new(p.x, p.z), p.y, arrived)
                            } else {
                                let (p, arrived) = ctx
                                    .mapmini
                                    .move_towards(position, target, speed, radius, 1.0);
                                (p, entity.position.y, arrived)
                            };

                            let move_delta = new_position - position;
                            if move_delta.magnitude_squared() > 1e-6 {
                                entity.set_orientation(move_delta.normalized());
                            }
                            entity.set_pos_xz(new_position);
                            entity.position.y = new_y;
                            if arrived {
                                let wait_ticks = Self::scheduled_delay_ticks(ctx, *route_wait);
                                wait_until = ctx.ticks + wait_ticks;
                                if len > 1 {
                                    let pingpong = route_mode.eq_ignore_ascii_case("pingpong");
                                    if pingpong {
                                        if fwd {
                                            if idx + 1 >= len {
                                                fwd = false;
                                                idx = idx.saturating_sub(1);
                                            } else {
                                                idx += 1;
                                            }
                                        } else if idx == 0 {
                                            fwd = true;
                                            idx = (idx + 1).min(len - 1);
                                        } else {
                                            idx -= 1;
                                        }
                                    } else {
                                        idx = (idx + 1) % len;
                                    }
                                }
                            }

                            ctx.check_player_for_section_change(entity);
                            entity.action = EntityAction::Patrol {
                                points,
                                route_wait: *route_wait,
                                route_speed: *route_speed,
                                route_mode: route_mode.clone(),
                                point_index: idx,
                                forward: fwd,
                                wait_until_tick: wait_until,
                            };
                        });
                    }
                }
                SleepAndSwitch(tick, action) => {
                    with_regionctx(self.id, |ctx| {
                        if *tick <= ctx.ticks {
                            entity.action = *action.clone();
                        }
                    });
                }
                _ => {}
            }

            with_regionctx(self.id, |ctx| {
                self.advance_entity_sequence(ctx, entity);
            });

            // Keep avatar animation state in sync with actual movement this update.
            let moved = (entity.get_pos_xz() - action_start_pos).magnitude_squared() > 1e-6;
            let mut attack_left = entity
                .attributes
                .get_float_default("avatar_attack_left", 0.0);
            if attack_left > 0.0 {
                attack_left = (attack_left - redraw_dt).max(0.0);
                entity.set_attribute("avatar_attack_left", Value::Float(attack_left));
            }
            let is_attacking = attack_left > 0.0;
            let is_casting = entity.attributes.get_bool_default("spell_casting", false);
            let desired_anim = if is_attacking {
                "Attack"
            } else if is_casting {
                "Cast"
            } else if moved {
                "Walk"
            } else {
                "Idle"
            };
            let current_anim = entity
                .attributes
                .get_str_default("avatar_animation", String::new());
            if !current_anim.eq_ignore_ascii_case(desired_anim) {
                entity.set_attribute("avatar_animation", Value::Str(desired_anim.to_string()));
            }

            if entity.is_dirty() {
                updates.push(entity.get_update().pack());
                entity.clear_dirty();
            }
        }

        with_regionctx(self.id, |ctx| {
            ctx.map.entities = entities;
            update_spell_items(ctx);

            // Send the entity updates if non empty
            if !updates.is_empty() {
                self.from_sender
                    .send(RegionMessage::EntitiesUpdate(self.id, updates))
                    .unwrap();
            }

            // let mut items = MAP.borrow().items.clone();
            for item in &mut ctx.map.items {
                if item.is_dirty() {
                    item_updates.push(item.get_update().pack());
                    item.clear_dirty();
                }
            }

            // Send the item updates if non empty
            if !item_updates.is_empty() {
                self.from_sender
                    .send(RegionMessage::ItemsUpdate(self.id, item_updates))
                    .unwrap();
            }
        });

        // Execute delayed scripts for entities
        let mut to_execute_entity = vec![];
        with_regionctx(self.id, |ctx| {
            to_execute_entity = ctx.to_execute_entity.clone();
            ctx.to_execute_entity.clear();
        });
        for todo in to_execute_entity {
            let entity_is_dead = if todo.1 == "death" {
                false
            } else {
                let mut dead = false;
                with_regionctx(self.id, |ctx| {
                    dead = is_entity_dead_ctx(ctx, todo.0);
                });
                dead
            };
            if entity_is_dead {
                continue;
            }

            if todo.1 == "__grant_xp" {
                with_regionctx(self.id, |ctx| {
                    let _ = grant_experience(ctx, todo.0, todo.2.x.max(0.0).round() as i32);
                });
                continue;
            }

            let mut ticks = 0;
            let mut state_data = FxHashMap::default();

            with_regionctx(self.id, |ctx| {
                ctx.curr_entity_id = todo.0;
                ctx.curr_item_id = None;
                state_data = ctx.entity_state_data.clone();
                ticks = ctx.ticks;
            });

            if let Some(state_data) = state_data.get_mut(&todo.0) {
                let specific_intent_key = if todo.1 == "intent" {
                    todo.2
                        .as_string()
                        .map(|intent| format!("intent: {}", intent.trim().to_ascii_lowercase()))
                } else {
                    None
                };

                // Check if we have already executed this script in this tick
                if let Some(Value::Int64(tick)) = state_data.get(&todo.1) {
                    if *tick >= ticks {
                        if todo.1.starts_with("intent") {
                            with_regionctx(self.id, |ctx| {
                                send_message(
                                    ctx,
                                    todo.0,
                                    "{system.cant_do_that_yet}".into(),
                                    "warning",
                                );
                            });
                        }
                        continue;
                    }
                }
                if let Some(specific_intent_key) = &specific_intent_key
                    && let Some(Value::Int64(tick)) = state_data.get(specific_intent_key)
                    && *tick >= ticks
                {
                    with_regionctx(self.id, |ctx| {
                        send_message(ctx, todo.0, "{system.cant_do_that_yet}".into(), "warning");
                    });
                    continue;
                }
                // Store the tick we executed this in
                state_data.set(&todo.1, Value::Int64(ticks));

                if let Some(specific_intent_key) = &specific_intent_key {
                    let pending_key = format!(
                        "__pending_intent_cooldown:{}",
                        todo.2
                            .as_string()
                            .map(|intent| intent.trim().to_ascii_lowercase())
                            .unwrap_or_default()
                    );
                    if let Some(value) = state_data.get(&pending_key).cloned() {
                        state_data.set(specific_intent_key, value);
                        state_data.remove(&pending_key);
                    }
                }
            } else {
                let mut vc = ValueContainer::default();
                vc.set(&todo.1, Value::Int64(ticks));
                state_data.insert(todo.0, vc);
            }

            with_regionctx(self.id, |ctx| {
                ctx.entity_state_data = state_data;
                ctx.damage_committed = false;
                ctx.current_damage_kind = if todo.1 == "take_damage" {
                    todo.2.as_string().map(|s| s.to_string())
                } else {
                    None
                };
                ctx.current_damage_source_item = if todo.1 == "take_damage" {
                    let source_item_id = todo.2.z.max(0.0) as u32;
                    if source_item_id > 0 {
                        Some(source_item_id)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(class_name) = ctx.entity_classes.get(&todo.0) {
                    if let Some(program) = ctx.entity_programs.get(class_name).cloned() {
                        let event_name = todo.1.clone();
                        let payload = todo.2.clone();
                        let args = [VMValue::from_string(event_name.clone()), payload.clone()];
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                        if event_name == "take_damage" && !ctx.damage_committed {
                            let from_id = payload.x.max(0.0) as u32;
                            let amount = payload.y.max(0.0) as i32;
                            if amount > 0 {
                                let kind = ctx
                                    .current_damage_kind
                                    .as_deref()
                                    .unwrap_or("physical")
                                    .to_string();
                                let _ = apply_damage_direct(
                                    ctx,
                                    todo.0,
                                    from_id,
                                    amount,
                                    &kind,
                                    ctx.current_damage_source_item,
                                );
                            }
                        }
                        flush_pending_entity_transfers(ctx);
                    }
                }
                ctx.current_damage_kind = None;
                ctx.current_damage_source_item = None;
            });

            // if let Err(err) = self.execute(&todo.2) {
            //     send_log_message(
            //         self.id,
            //         format!(
            //             "TO_EXECUTE_ENTITY: Error for '{}': {}: {}",
            //             todo.0, todo.1, err,
            //         ),
            //     );
            // }
        }

        // Execute delayed scrips for items.
        // This is because we can only borrow REGION once.

        let mut to_execute_item = vec![];
        with_regionctx(self.id, |ctx| {
            to_execute_item = ctx.to_execute_item.clone();
            ctx.to_execute_item.clear();
        });

        for todo in to_execute_item {
            let mut ticks = 0;
            let mut state_data = FxHashMap::default();
            with_regionctx(self.id, |ctx| {
                ctx.curr_item_id = Some(todo.0);
                state_data = ctx.item_state_data.clone();
                ticks = ctx.ticks;
            });

            if let Some(state_data) = state_data.get_mut(&todo.0) {
                let specific_intent_key = if todo.1 == "intent" {
                    todo.2
                        .as_string()
                        .map(|intent| format!("intent: {}", intent.trim().to_ascii_lowercase()))
                } else {
                    None
                };

                // Check if we have already executed this script in this tick
                if let Some(Value::Int64(tick)) = state_data.get(&todo.1) {
                    if *tick >= ticks {
                        continue;
                    }
                }
                if let Some(specific_intent_key) = &specific_intent_key
                    && let Some(Value::Int64(tick)) = state_data.get(specific_intent_key)
                    && *tick >= ticks
                {
                    continue;
                }
                // Store the tick we executed this in
                state_data.set(&todo.1, Value::Int64(ticks));
            } else {
                let mut vc = ValueContainer::default();
                vc.set(&todo.1, Value::Int64(ticks));
                state_data.insert(todo.0, vc);
            }

            with_regionctx(self.id, |ctx| {
                ctx.item_state_data = state_data;
                if let Some(class_name) = ctx.item_classes.get(&todo.0) {
                    if let Some(program) = ctx.item_programs.get(class_name).cloned() {
                        let args = [VMValue::from_string(todo.1), todo.2];
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                    }
                }
            });

            // if let Err(err) = self.execute(&todo.2) {
            //     send_log_message(
            //         self.id,
            //         format!(
            //             "TO_EXECUTE_ITEM: Error for '{}': {}: {}",
            //             todo.0, todo.1, err,
            //         ),
            //     );
            // }
        }

        with_regionctx(self.id, |ctx| {
            if ctx.debug_mode {
                self.from_sender
                    .send(RegionMessage::DebugData(ctx.debug.clone()))
                    .unwrap();
            }
        });
    }

    /*
    /// Execute a script.
    pub fn execute(&self, source: &str) -> Result<PyObjectRef, String> {
        let scope = self.scope.lock().unwrap();

        self.interp.enter(|vm| {
            let rc = vm.run_block_expr(scope.clone(), source);
            match rc {
                Ok(obj) => Ok(obj),
                Err(error) => {
                    let mut err_line: Option<u32> = None;

                    if let Some(tb) = error.__traceback__() {
                        // let file_name = tb.frame.code.source_path.as_str();
                        let instruction_index =
                            tb.frame.lasti.load(std::sync::atomic::Ordering::Relaxed);
                        err_line = Some(instruction_index / 2);
                        // let function_name = tb.frame.code.obj_name.as_str();
                    }

                    let mut err_string = String::new();
                    if let Some(err) = error.args().first() {
                        if let Ok(msg) = err.str(vm) {
                            err_string = msg.to_string();
                        }
                    }

                    if let Some(err_line) = err_line {
                        err_string = format!("{} at line {}.", err_string, err_line);
                    }
                    println!("err {}", err_string);
                    Err(err_string)
                }
            }
        })
    }*/

    /// Create a sleep action which switches back to the previous action.
    fn create_sleep_switch_action(&self, minutes: u32, switchback: EntityAction) -> EntityAction {
        with_regionctx(self.id, |ctx| {
            let tick = ctx.ticks + Self::scheduled_delay_ticks(ctx, minutes as f32);
            SleepAndSwitch(tick, Box::new(switchback))
        })
        .unwrap()
    }

    /// Moves an entity forward or backward. Returns true if blocked.
    fn move_entity(&self, entity: &mut Entity, dir: f32, entity_block_mode: i32) -> bool {
        with_regionctx(self.id, |ctx| {
            let speed = self.movement_units_per_sec * ctx.delta_time;
            entity.orientation * speed * dir
        })
        .map(|move_vector| self.move_entity_by_vector(entity, move_vector, entity_block_mode))
        .unwrap()
    }

    /// Create a new entity instance.
    pub fn create_entity_instance(&mut self, mut entity: Entity) {
        entity.id = get_global_id();
        entity.set_attribute(
            "_source_seq",
            Value::Source(PixelSource::Sequence("idle".into())),
        );
        entity.set_attribute("mode", Value::Str("active".into()));
        entity.mark_all_dirty();

        if let Some(class_name) = entity.get_attr_string("class_name") {
            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                ctx.map.entities.push(entity.clone());

                // Setting the data for the entity
                if let Some(data) = ctx.entity_class_data.get(&class_name) {
                    let ground_y =
                        map_spawn_height(&ctx.map, entity.get_pos_xz(), Some(entity.position.y));
                    let mut spawn_entity_id: Option<u32> = None;
                    for e in ctx.map.entities.iter_mut() {
                        if e.id == entity.id {
                            apply_entity_data(e, data);
                            e.position.y = ground_y;

                            // Fill up the inventory slots
                            if let Some(Value::Int(inv_slots)) = e.attributes.get("inventory_slots")
                            {
                                e.inventory = vec![];
                                for _ in 0..*inv_slots {
                                    e.inventory.push(None);
                                }
                            }

                            // Set the wallet
                            if let Some(Value::Int(wealth)) = e.attributes.get("wealth") {
                                _ = e.add_base_currency(*wealth as i64, &ctx.currencies)
                            }
                            spawn_entity_id = Some(e.id);
                        }
                    }
                    if let Some(spawn_entity_id) = spawn_entity_id {
                        apply_spawn_item_lists_for_entity(spawn_entity_id, ctx);
                    }
                }

                ctx.curr_entity_id = entity.id;

                // Register player
                if ctx.entity_player_classes.contains(&class_name) {
                    if let Some(entity) = get_entity_mut(&mut ctx.map, ctx.curr_entity_id) {
                        entity
                            .set_attribute("player_camera", Value::PlayerCamera(PlayerCamera::D2));
                    }

                    self.from_sender
                        .send(RegisterPlayer(self.id, ctx.curr_entity_id))
                        .unwrap();
                }

                // Register the class for the entity
                ctx.entity_classes.insert(entity.id, class_name.clone());
            });

            // Send "startup" event
            // let cmd = format!("{}.event(\"startup\", \"\")", class_name);
            // if let Err(err) = self.execute(&cmd) {
            //     send_log_message(
            //         0,
            //         format!(
            //             "{}: Event Error ({}) for '{}': {}",
            //             self.name,
            //             "startup",
            //             self.get_entity_name(entity.id),
            //             err,
            //         ),
            //     );
            // }
            //

            // Determine, set and notify the entity about the sector it is in.
            let mut sector_name = String::new();

            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                // Send startup event
                if let Some(program) = ctx.entity_programs.get(&class_name).cloned() {
                    let args = [VMValue::from_string("startup"), VMValue::zero()];
                    run_server_fn(&mut self.exec, &args, &program, ctx);
                    flush_pending_entity_transfers(ctx);
                }

                if let Some(sector) = ctx.map.find_sector_at(entity.get_pos_xz()) {
                    sector_name = sector.name.clone();
                }
                {
                    for e in ctx.map.entities.iter_mut() {
                        if e.id == entity.id {
                            e.attributes.set("sector", Value::Str(sector_name.clone()));
                        }
                    }
                }

                if !sector_name.is_empty() {
                    if let Some(program) = ctx.entity_programs.get(&class_name).cloned() {
                        let args = [
                            VMValue::from_string("entered"),
                            VMValue::from_string(sector_name),
                        ];
                        run_server_fn(&mut self.exec, &args, &program, ctx);
                        flush_pending_entity_transfers(ctx);
                    }
                }
            });
            // if !sector_name.is_empty() {
            //     let cmd = format!("{}.event(\"entered\", \"{}\")", class_name, sector_name);
            //     _ = self.execute(&cmd);
            // }
        }

        let region_name = self.name.clone();
        self.run_entity_instance_setup(&entity, &region_name, "for spawned instance");

        send_log_message(
            self.id,
            format!(
                "{}: Spawned `{}`",
                self.name,
                self.get_entity_name(entity.id),
            ),
        );
    }

    /// Get the name of the entity with the given id.
    fn get_entity_name(&self, id: u32) -> String {
        let mut name = "Unknown".to_string();
        with_regionctx(self.id, |ctx: &mut RegionCtx| {
            for entity in ctx.map.entities.iter() {
                if entity.id == id {
                    if let Some(n) = entity.attributes.get_str("name") {
                        name = n.to_string();
                    }
                }
            }
        });
        name
    }

    /// Send "intent" events for the entity or item at the given position.
    fn send_entity_intent_events(&self, entity: &mut Entity, position: Vec2<f32>) {
        with_regionctx(self.id, |ctx: &mut RegionCtx| {
            // Send "intent" event for the entity
            let keep_intent = Self::should_keep_player_intent(ctx, entity);

            let mut value = VMValue::zero();
            value.y = 1.0; // Distance

            let mut target_item_id = None;
            let mut target_entity_id = None;

            // TODO

            let mut found_target = false;
            if let Some(entity_id) = get_entity_at(ctx, position, entity.id) {
                if entity_id != entity.id && !ctx.is_entity_dead_ctx(entity_id) {
                    value.x = entity_id as f32;
                    target_entity_id = Some(entity_id);
                    found_target = true;
                }
            }
            if !found_target {
                if let Some(i_id) = get_item_at(ctx, position) {
                    value.x = i_id as f32;
                    target_item_id = Some(i_id);
                    found_target = true;
                }
            }

            let intent = entity.attributes.get_str_default("intent", "".into());
            let intent_lower = intent.trim().to_ascii_lowercase();
            let rules = intent_rule_config(ctx, entity.id, &intent_lower);

            if let Some(spell_template) = intent.trim().strip_prefix("spell:") {
                let spell_template = spell_template.trim();
                if spell_template.is_empty() {
                    return;
                }

                if let Some(target_entity_id) = target_entity_id {
                    _ = cast_spell_for_entity(
                        ctx,
                        entity.id,
                        spell_template,
                        target_entity_id,
                        100.0,
                    );
                } else {
                    // In 2D directional intent mode, cast towards the chosen direction
                    // even if no entity is currently at that tile.
                    _ = cast_spell_for_entity_to_pos(
                        ctx,
                        entity.id,
                        spell_template,
                        position,
                        100.0,
                    );
                }
                return;
            }

            if !found_target {
                if !keep_intent {
                    entity.set_attribute("intent", Value::Str(String::new()));
                }
                send_message(ctx, entity.id, "{system.cant_do_that}".into(), "warning");
                return;
            }

            let target_entity = target_entity_id
                .and_then(|id| ctx.map.entities.iter().find(|candidate| candidate.id == id));
            let target_item = target_item_id
                .and_then(|id| ctx.map.items.iter().find(|candidate| candidate.id == id));

            if !intent.trim().is_empty()
                && let Some(allowed) = rules.allowed.as_deref()
                && !evaluate_intent_allowed(
                    ctx,
                    allowed,
                    value.y,
                    Some(entity),
                    target_entity,
                    target_item,
                )
            {
                send_message(
                    ctx,
                    entity.id,
                    rules
                        .deny_message
                        .clone()
                        .unwrap_or_else(|| "{system.cant_do_that}".to_string()),
                    "warning",
                );
                if !keep_intent {
                    entity.set_attribute("intent", Value::Str(String::new()));
                }
                return;
            }

            if intent_lower == "look" {
                if let Some(target_entity) = target_entity {
                    if let Some(msg) = target_entity.attributes.get_str("on_look") {
                        let msg = msg.trim();
                        if !msg.is_empty() {
                            send_message(ctx, entity.id, msg.to_string(), "system");
                            if !keep_intent {
                                entity.set_attribute("intent", Value::Str(String::new()));
                            }
                            return;
                        }
                    }
                    if let Some(msg) = entity_look_description(ctx, target_entity) {
                        send_message(ctx, entity.id, msg, "system");
                        if !keep_intent {
                            entity.set_attribute("intent", Value::Str(String::new()));
                        }
                        return;
                    }
                }
                if let Some(target_item) = target_item {
                    if let Some(msg) = target_item.attributes.get_str("on_look") {
                        let msg = msg.trim();
                        if !msg.is_empty() {
                            send_message(ctx, entity.id, msg.to_string(), "system");
                            if !keep_intent {
                                entity.set_attribute("intent", Value::Str(String::new()));
                            }
                            return;
                        }
                    }
                    if let Some(msg) = item_look_description(ctx, target_item) {
                        send_message(ctx, entity.id, msg, "system");
                        if !keep_intent {
                            entity.set_attribute("intent", Value::Str(String::new()));
                        }
                        return;
                    }
                }
            }

            if intent_lower == "use" {
                if let Some(target_item) = target_item {
                    if let Some(msg) = target_item.attributes.get_str("on_use") {
                        let msg = msg.trim();
                        if !msg.is_empty() {
                            send_message(ctx, entity.id, msg.to_string(), "system");
                        }
                    } else if let Some(msg) = item_use_message(ctx, target_item) {
                        send_message(ctx, entity.id, msg, "system");
                    }
                }
            }

            value.string = Some(intent.clone());

            ctx.to_execute_entity
                .push((entity.id, "intent".to_string(), value.clone()));

            value.x = entity.id as f32;

            if let Some(target_entity_id) = target_entity_id {
                ctx.to_execute_entity
                    .push((target_entity_id, "intent".to_string(), value));
            } else if let Some(item_id) = target_item_id {
                ctx.to_execute_item
                    .push((item_id, "intent".to_string(), value));
            }

            queue_intent_cooldown(ctx, entity.id, &intent_lower, rules.cooldown_minutes);

            if !keep_intent {
                entity.set_attribute("intent", Value::Str(String::new()));
            }
        });
    }

    /// Returns the entities in the radius of the character or item.
    fn entities_in_radius(
        &self,
        ctx: &RegionCtx,
        entity_id: Option<u32>,
        item_id: Option<u32>,
        radius: f32,
    ) -> Vec<u32> {
        let mut position = None;
        let mut is_entity = false;
        let mut id = 0;

        if let Some(item_id) = item_id {
            if let Some(item) = ctx.map.items.iter().find(|item| item.id == item_id) {
                id = item_id;
                position = Some(item.get_pos_xz());
            }
        } else if let Some(entity_id) = entity_id {
            is_entity = true;
            if let Some(entity) = ctx
                .map
                .entities
                .iter()
                .find(|entity| entity.id == entity_id)
            {
                id = entity.id;
                position = Some(entity.get_pos_xz());
            }
        }

        let mut entities: Vec<(u32, f32)> = Vec::new();

        if let Some(position) = position {
            for other in ctx.map.entities.iter() {
                if is_entity && other.id == id {
                    continue;
                }
                if other.get_mode() == "dead" {
                    continue;
                }
                let other_position = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5);

                let distance_squared = (position - other_position).magnitude_squared();
                let combined_radius = radius + other_radius;
                let combined_radius_squared = combined_radius * combined_radius;

                // Entity is inside the radius
                if distance_squared < combined_radius_squared {
                    entities.push((other.id, distance_squared));
                }
            }
        }

        entities.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        entities.into_iter().map(|(id, _)| id).collect()
    }
}

fn collect_spawn_item_list(attrs: &ValueContainer, keys: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for key in keys {
        if let Some(value) = attrs.get(key) {
            match value {
                Value::StrArray(values) => {
                    for entry in values {
                        let trimmed = entry.trim();
                        if !trimmed.is_empty() && !out.iter().any(|v| v == trimmed) {
                            out.push(trimmed.to_string());
                        }
                    }
                }
                Value::Str(raw) => {
                    for entry in raw.split(',') {
                        let trimmed = entry.trim();
                        if !trimmed.is_empty() && !out.iter().any(|v| v == trimmed) {
                            out.push(trimmed.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn apply_spawn_item_entries_for_entity(
    entity_id: u32,
    entity_name: &str,
    ctx: &mut RegionCtx,
    class_names: &[String],
    equip: bool,
) {
    for class_name in class_names {
        let Some(item) = ctx.create_item(class_name.clone()) else {
            ctx.send_log_message(format!(
                "[warn] {} ({}) => unknown startup item template '{}'",
                entity_name, entity_id, class_name
            ));
            continue;
        };

        let item_id = item.id;
        let item_slot = item.attributes.get_str("slot").map(str::to_string);

        let mut added = false;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            added = entity.add_item(item).is_ok();
        }
        if !added {
            ctx.send_log_message(format!(
                "[warn] {} ({}) => startup item '{}' skipped: inventory full",
                entity_name, entity_id, class_name
            ));
            continue;
        }

        if !equip {
            continue;
        }

        if let Some(slot) = item_slot {
            let mut _equip_ok = false;
            if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                _equip_ok = entity.equip_item(item_id, &slot).is_ok();
            }
        } else {
            ctx.send_log_message(format!(
                "[warn] {} ({}) => startup equip item '{}' has no slot attribute",
                entity_name, entity_id, class_name
            ));
        }
    }
}

fn apply_spawn_item_lists_for_entity(entity_id: u32, ctx: &mut RegionCtx) {
    let mut entity_name = "Unknown".to_string();
    let mut add_only: Vec<String> = Vec::new();
    let mut add_and_equip: Vec<String> = Vec::new();
    if let Some(entity) = ctx.map.entities.iter().find(|e| e.id == entity_id) {
        entity_name = entity
            .attributes
            .get_str_default("name", "Unknown".to_string());

        // Preferred names plus backward-compatible aliases.
        add_only = collect_spawn_item_list(
            &entity.attributes,
            &["start_items", "startup_items", "add_items"],
        );
        add_and_equip = collect_spawn_item_list(
            &entity.attributes,
            &[
                "start_equipped_items",
                "startup_equipped_items",
                "add_equip_items",
            ],
        );
    }

    if add_only.is_empty() && add_and_equip.is_empty() {
        return;
    }

    apply_spawn_item_entries_for_entity(entity_id, &entity_name, ctx, &add_only, false);
    apply_spawn_item_entries_for_entity(entity_id, &entity_name, ctx, &add_and_equip, true);
}

/// Set Player Camera
/*
fn set_player_camera(camera: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let player_camera = match camera.as_str() {
            "2d_grid" => PlayerCamera::D2Grid,
            "iso" => PlayerCamera::D3Iso,
            "iso_grid" => PlayerCamera::D2Grid,
            "firstp" => PlayerCamera::D3FirstP,
            "firstp_grid" => PlayerCamera::D3FirstPGrid,
            _ => PlayerCamera::D2,
        };

        if let Some(entity) = get_entity_mut(&mut ctx.map, ctx.curr_entity_id) {
            entity.set_attribute("player_camera", Value::PlayerCamera(player_camera));
        }
    });
}*/

/// Is the given entity dead.
pub fn is_entity_dead(region_id: u32, id: u32) -> bool {
    let mut v = false;
    with_regionctx(region_id, |ctx: &mut RegionCtx| {
        for entity in &ctx.map.entities {
            if entity.id == id {
                v = entity.attributes.get_str_default("mode", "active".into()) == "dead";
            }
        }
    });
    v
}

/// Is the given entity dead.
pub fn is_entity_dead_ctx(ctx: &RegionCtx, id: u32) -> bool {
    let mut v = false;
    for entity in &ctx.map.entities {
        if entity.id == id {
            v = entity.attributes.get_str_default("mode", "active".into()) == "dead";
        }
    }
    v
}

/// Search for a mutable reference to an entity with the given ID.
fn get_entity_mut<'a>(map: &'a mut Map, entity_id: u32) -> Option<&'a mut Entity> {
    // Look in the top-level items
    if let Some(entity) = map
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        return Some(entity);
    }
    None
}

pub fn send_log_message(id: u32, message: String) {
    with_regionctx(id, |ctx| {
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::LogMessage(message))
            .unwrap();
    });
}

/// Get an i32 config value
fn get_config_i32_default(ctx: &RegionCtx, table: &str, key: &str, default: i32) -> i32 {
    let mut value = default;
    let tab = &ctx.config;
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(val) = game.get(key) {
            if let Some(v) = val.as_integer() {
                value = v as i32;
            }
        }
    }
    value
}

/// Get a bool config value
fn get_config_bool_default(ctx: &RegionCtx, table: &str, key: &str, default: bool) -> bool {
    let mut value = default;
    let tab = &ctx.config;
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table)
        && let Some(val) = game.get(key)
        && let Some(v) = val.as_bool()
    {
        value = v;
    }
    value
}

/// Returns the entity at the given position (if any)
fn get_entity_at(ctx: &RegionCtx, position: Vec2<f32>, but_not: u32) -> Option<u32> {
    let mut entity = None;

    for other in ctx.map.entities.iter() {
        if other.id == but_not {
            continue;
        }
        let other_position = other.get_pos_xz();

        let distance = position.distance(other_position);

        // Item is inside the radius
        if distance < 1.0 {
            entity = Some(other.id);
            break; // We only need the first item found
        }
    }

    entity
}

/// Returns the item at the given position (if any)
fn get_item_at(ctx: &RegionCtx, position: Vec2<f32>) -> Option<u32> {
    let mut item = None;

    for other in ctx.map.items.iter() {
        let other_position = other.get_pos_xz();

        let distance = position.distance(other_position);

        // Item is inside the radius
        if distance < 1.0 {
            item = Some(other.id);
            break; // We only need the first item found
        }
    }

    item
}

/// Received an entity from another region
pub fn receive_entity(ctx: &mut RegionCtx, mut entity: Entity, dest_sector_name: String) {
    entity.action = EntityAction::Off;
    let entity_id = entity.id;
    if entity.is_player() {
        entity.set_attribute("mode", Value::Str("active".into()));
        entity.set_attribute("visible", Value::Bool(true));
    }

    let mut new_pos: Option<vek::Vec2<f32>> = None;
    for sector in &ctx.map.sectors {
        if sector.name == dest_sector_name {
            new_pos = sector.center(&ctx.map);
        }
    }

    if let Some(new_pos) = new_pos {
        entity.set_pos_xz(new_pos);
        entity.position.y =
            map_spawn_height(&ctx.map, entity.get_pos_xz(), Some(entity.position.y));
    }

    if let Some(class_name) = entity.get_attr_string("class_name") {
        ctx.entity_classes.insert(entity_id, class_name.clone());
    }

    ctx.map.entities.retain(|existing| existing.id != entity_id);
    ctx.map.entities.push(entity);
    ctx.check_player_for_section_change_id(entity_id);
}

fn flush_pending_entity_transfers(ctx: &mut RegionCtx) {
    if ctx.pending_entity_transfers.is_empty() {
        return;
    }

    let pending = std::mem::take(&mut ctx.pending_entity_transfers);
    for (entity_id, dest_region_name, dest_sector_name) in pending {
        if let Some(pos) = ctx.map.entities.iter().position(|e| e.id == entity_id) {
            let removed = ctx.map.entities.remove(pos);
            ctx.entity_classes.remove(&removed.id);

            if let Some(sender) = ctx.from_sender.get() {
                let _ = sender.send(RegionMessage::TransferEntity(
                    ctx.region_id,
                    removed,
                    dest_region_name,
                    dest_sector_name,
                ));
            }
        }
    }
}

/// Add a debug value at the current debug position
#[inline(always)]
pub fn add_debug_value(ctx: &mut RegionCtx, value: TheValue, error: bool) {
    if let Some((event, x, y)) = &ctx.curr_debug_loc {
        if let Some(item_id) = ctx.curr_item_id {
            ctx.debug.add_value(item_id, event, *x, *y, value);
            if error {
                ctx.debug.add_error(item_id, event, *x, *y);
            } else {
                ctx.debug.remove_error(item_id, event, *x, *y);
            }
        } else {
            ctx.debug
                .add_value(ctx.curr_entity_id, event, *x, *y, value);
            if error {
                ctx.debug.add_error(ctx.curr_entity_id, event, *x, *y);
            } else {
                ctx.debug.remove_error(ctx.curr_entity_id, event, *x, *y);
            }
        }

        ctx.curr_debug_loc = None;
    }
}

/*
fn _get_config_f32_default(table: &str, key: &str, default: f32) -> f32 {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_float() {
                return v as f32;
            }
        }
    }
    default
}

fn _get_config_bool_default(table: &str, key: &str, default: bool) -> bool {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_bool() {
                return v;
            }
        }
    }
    default
}
*/

fn get_config_string_default(ctx: &RegionCtx, table: &str, key: &str, default: &str) -> String {
    let mut value = default.to_string();
    let tab = &ctx.config;
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(val) = game.get(key) {
            if let Some(v) = val.as_str() {
                value = v.to_string();
            }
        }
    }
    value
}

/*
/// Sets light emission to on / off
fn set_emit_light(value: bool, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                if let Some(Value::Light(light)) = item.attributes.get_mut("light") {
                    light.active = value;
                    item.mark_dirty_attribute("light");
                }
            }
        } else {
            let entity_id = ctx.curr_entity_id;
            if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                if let Some(Value::Light(light)) = entity.attributes.get_mut("light") {
                    light.active = value;
                    entity.mark_dirty_attribute("light");
                }
            }
        }
    });
}

/// Set the tile_id of the current entity or item.
fn set_tile(id: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Ok(uuid) = Uuid::try_parse(&id) {
            if let Some(item_id) = ctx.curr_item_id {
                if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                    item.set_attribute("source", Value::Source(PixelSource::TileId(uuid)));
                }
            } else {
                let entity_id = ctx.curr_entity_id;
                if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                    entity.set_attribute("source", Value::Source(PixelSource::TileId(uuid)));
                }
            }
        }
    });
}

/// Set rigging sequence
pub fn set_rig_sequence(
    args: rustpython_vm::function::FuncArgs,
    vm: &VirtualMachine,
) -> PyResult<()> {
    let mut sequence = vec![];

    for arg in args.args.iter() {
        if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
            sequence.push(v);
        }
    }

    Ok(())
}

fn take_item_for_entity(ctx: &mut RegionCtx, entity_id: u32, item_id: u32) -> bool {
    let mut rc = true;

    if let Some(pos) = ctx
        .map
        .items
        .iter()
        .position(|item| item.id == item_id && !item.attributes.get_bool_default("static", false))
    {
        let item = ctx.map.items.remove(pos);
        if item.attributes.get_bool_default("is_spell", false) {
            return false;
        }

        if let Some(entity) = ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
        {
            let mut item_name = "Unknown".to_string();
            if let Some(name) = item.attributes.get_str("name") {
                item_name = name.to_string();
            }

            fn article_for(item_name: &str) -> (&'static str, String) {
                let name = item_name.to_ascii_lowercase();

                let pair_items = ["trousers", "pants", "gloves", "boots", "scissors"];
                let mass_items = ["armor", "cloth", "water", "meat"];

                if pair_items.contains(&name.as_str()) {
                    ("a pair of", item_name.to_string())
                } else if mass_items.contains(&name.as_str()) {
                    ("some", item_name.to_string())
                } else {
                    let first = name.chars().next().unwrap_or('x');
                    let article = match first {
                        'a' | 'e' | 'i' | 'o' | 'u' => "an",
                        _ => "a",
                    };
                    (article, item_name.to_string())
                }
            }

            let mut message = format!(
                "You take {} {}",
                article_for(&item_name.to_lowercase()).0,
                item_name.to_lowercase()
            );

            if item.attributes.get_bool_default("monetary", false) {
                // This is not a standalone item but money
                let amount = item.attributes.get_int_default("worth", 0);
                if amount > 0 {
                    message = format!("You take {} gold.", amount);
                    _ = entity.add_base_currency(amount as i64, &ctx.currencies);
                }
            } else if entity.add_item(item).is_err() {
                println!("Take: Too many items");
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Text("Inventory Full".into()), true);
                }
                rc = false;
            }

            if ctx.debug_mode && rc {
                add_debug_value(ctx, TheValue::Text("Ok".into()), false);
            }

            ctx.from_sender
                .get()
                .unwrap()
                .send(RegionMessage::RemoveItem(ctx.region_id, item_id))
                .unwrap();

            let msg = RegionMessage::Message(
                ctx.region_id,
                Some(entity_id),
                None,
                entity_id,
                message,
                "system".into(),
            );
            ctx.from_sender.get().unwrap().send(msg).unwrap();
        }
    } else if ctx.debug_mode {
        add_debug_value(ctx, TheValue::Text("Unknown Item".into()), true);
    }
    rc
}

/// Take the given item.
fn take(item_id: u32, vm: &VirtualMachine) -> bool {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        take_item_for_entity(ctx, ctx.curr_entity_id, item_id)
    })
    .unwrap()
}

/// Block the events for the entity / item for the given amount of minutes.
pub fn block_events(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let mut minutes: f32 = 4.0;
        let mut events: Vec<String> = Vec::new();

        for (i, arg) in args.args.iter().enumerate() {
            if i == 0 {
                if let Some(v) = Value::from_pyobject(arg.clone(), vm).and_then(|v| v.to_f32()) {
                    minutes = v;
                }
            } else if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                events.push(v);
            }
        }

        let target_tick = Value::Int64(ctx.ticks + Self::scheduled_delay_ticks(ctx, minutes));

        if let Some(item_id) = ctx.curr_item_id {
            let state_data = &mut ctx.item_state_data;
            if let Some(state_data) = state_data.get_mut(&item_id) {
                for event in events {
                    state_data.set(&event, target_tick.clone());
                }
            } else {
                let mut vc = ValueContainer::default();
                for event in events {
                    vc.set(&event, target_tick.clone());
                }
                state_data.insert(item_id, vc);
            }
        } else {
            let entity_id = ctx.curr_entity_id;

            let state_data = &mut ctx.entity_state_data;
            if let Some(state_data) = state_data.get_mut(&entity_id) {
                for event in events {
                    state_data.set(&event, target_tick.clone());
                }
            } else {
                let mut vc = ValueContainer::default();
                for event in events {
                    vc.set(&event, target_tick.clone());
                }
                state_data.insert(entity_id, vc);
            }
        }
    });
}

/// Deal damage to the given entity. Sends an "take_damage" event to the other entity.
fn deal_damage(id: u32, dict: PyObjectRef, vm: &VirtualMachine) {
    /*
    let dict = extract_dictionary(dict, vm);

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Ok(dict) = dict {
            if let Some(entity) = ctx.map.entities.iter().find(|entity| entity.id == id) {
                if let Some(class_name) = entity.attributes.get_str("class_name") {
                    let cmd = format!("{}.event('{}', {})", class_name, "take_damage", dict);
                    ctx.to_execute_entity.push((id, "take_damage".into(), cmd));
                }
            } else if let Some(item) = ctx.map.items.iter_mut().find(|item| item.id == id) {
                if let Some(class_name) = item.attributes.get_str("class_name") {
                    let cmd = format!("{}.event('{}', {})", class_name, "take_damage", dict);
                    ctx.to_execute_item.push((id, "take_damage".into(), cmd));
                }
            }
        }
    });*/
}

*/

/// Send a message to the entity.
fn send_message(ctx: &RegionCtx, id: u32, message: String, role: &str) {
    let msg = RegionMessage::Message(ctx.region_id, Some(id), None, id, message, role.to_string());
    ctx.from_sender.get().unwrap().send(msg).unwrap();
}

fn send_message_from(
    ctx: &RegionCtx,
    sender_entity_id: u32,
    receiver_id: u32,
    message: String,
    role: &str,
) {
    let msg = RegionMessage::Message(
        ctx.region_id,
        Some(sender_entity_id),
        None,
        receiver_id,
        message,
        role.to_string(),
    );
    ctx.from_sender.get().unwrap().send(msg).unwrap();
}

pub(crate) fn spell_cooldown_key(template: &str) -> String {
    format!("__spell_cd_{}", template.trim().to_ascii_lowercase())
}

pub(crate) fn is_spell_on_cooldown(ctx: &RegionCtx, caster_id: u32, template: &str) -> bool {
    let key = spell_cooldown_key(template);
    if let Some(state) = ctx.entity_state_data.get(&caster_id)
        && let Some(value) = state.get(&key)
    {
        return match value {
            Value::Float(left) => *left > 0.0,
            Value::Int64(until_tick) => ctx.ticks < *until_tick,
            Value::Int(until_tick) => ctx.ticks < *until_tick as i64,
            _ => false,
        };
    }
    false
}

pub(crate) fn set_spell_cooldown(
    ctx: &mut RegionCtx,
    caster_id: u32,
    template: &str,
    cooldown_seconds: f32,
) {
    if cooldown_seconds <= 0.0 {
        return;
    }
    let key = spell_cooldown_key(template);
    if let Some(state) = ctx.entity_state_data.get_mut(&caster_id) {
        state.set(&key, Value::Float(cooldown_seconds));
    } else {
        let mut vc = ValueContainer::default();
        vc.set(&key, Value::Float(cooldown_seconds));
        ctx.entity_state_data.insert(caster_id, vc);
    }
}

fn update_spell_cooldowns(ctx: &mut RegionCtx, dt: f32) {
    if dt <= 0.0 {
        return;
    }
    for state in ctx.entity_state_data.values_mut() {
        let keys: Vec<String> = state
            .keys()
            .filter(|k| k.starts_with("__spell_cd_"))
            .cloned()
            .collect();
        for key in keys {
            if let Some(Value::Float(left)) = state.get(&key).cloned() {
                state.set(&key, Value::Float((left - dt).max(0.0)));
            }
        }
    }
}

pub(crate) fn apply_spell_default_attrs(spell_item: &mut Item) {
    if spell_item.attributes.get("spell_mode").is_none() {
        spell_item.set_attribute("spell_mode", Value::Str("projectile".into()));
    }
    if spell_item.attributes.get("spell_effect").is_none() {
        spell_item.set_attribute("spell_effect", Value::Str("damage".into()));
    }
    if spell_item.attributes.get("spell_target_filter").is_none() {
        spell_item.set_attribute("spell_target_filter", Value::Str("any".into()));
    }
    if spell_item.attributes.get("spell_amount").is_none() {
        spell_item.set_attribute("spell_amount", Value::Int(1));
    }
    if spell_item.attributes.get("spell_kind").is_none() {
        spell_item.set_attribute("spell_kind", Value::Str("spell".into()));
    }
    if spell_item.attributes.get("spell_speed").is_none() {
        spell_item.set_attribute("spell_speed", Value::Float(6.0));
    }
    if spell_item.attributes.get("spell_cast_time").is_none() {
        spell_item.set_attribute("spell_cast_time", Value::Float(0.0));
    }
    if spell_item.attributes.get("spell_cast_offset").is_none() {
        spell_item.set_attribute("spell_cast_offset", Value::Float(0.6));
    }
    if spell_item.attributes.get("spell_cast_height").is_none() {
        spell_item.set_attribute("spell_cast_height", Value::Float(0.5));
    }
    if spell_item.attributes.get("spell_flight_height").is_none() {
        spell_item.set_attribute("spell_flight_height", Value::Float(0.5));
    }
    if spell_item.attributes.get("spell_cooldown").is_none() {
        spell_item.set_attribute("spell_cooldown", Value::Float(0.0));
    }
    if spell_item.attributes.get("spell_max_range").is_none() {
        spell_item.set_attribute("spell_max_range", Value::Float(0.0));
    }
    if spell_item.attributes.get("spell_lifetime").is_none() {
        spell_item.set_attribute("spell_lifetime", Value::Float(3.0));
    }
    if spell_item.attributes.get("spell_radius").is_none() {
        spell_item.set_attribute("spell_radius", Value::Float(0.4));
    }
}

fn parse_filter_expr(filter: &str) -> Option<(&str, &str, f32)> {
    let ops = ["<=", ">=", "==", "!=", "<", ">"];
    let trimmed = filter.trim();
    for op in ops {
        if let Some(idx) = trimmed.find(op) {
            let lhs = trimmed[..idx].trim();
            let rhs = trimmed[idx + op.len()..].trim();
            if lhs.is_empty() || rhs.is_empty() {
                return None;
            }
            if let Ok(v) = rhs.parse::<f32>() {
                return Some((lhs, op, v));
            }
        }
    }
    None
}

fn numeric_attr(attrs: &ValueContainer, key: &str) -> Option<f32> {
    match attrs.get(key) {
        Some(Value::Float(v)) => Some(*v),
        Some(Value::Int(v)) => Some(*v as f32),
        Some(Value::UInt(v)) => Some(*v as f32),
        Some(Value::Int64(v)) => Some(*v as f32),
        Some(Value::Bool(v)) => Some(if *v { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn cast_spell_for_entity(
    ctx: &mut RegionCtx,
    caster_id: u32,
    template: &str,
    target_id: u32,
    success_pct: f32,
) -> i32 {
    if is_spell_on_cooldown(ctx, caster_id, template) {
        return -1;
    }

    let success_pct = success_pct.clamp(0.0, 100.0);
    let mut rng = rand::rng();
    let roll = rng.random_range(0.0..100.0);
    if roll >= success_pct {
        ctx.to_execute_entity
            .push((caster_id, "cast_failed".into(), VMValue::zero()));
        return -1;
    }

    let Some(mut spell_item) = ctx.create_item(template.to_string()) else {
        return -1;
    };
    let Some(caster) = ctx.map.entities.iter().find(|e| e.id == caster_id) else {
        return -1;
    };
    let Some(target) = ctx.map.entities.iter().find(|e| e.id == target_id) else {
        return -1;
    };
    let caster_pos = caster.position;
    let caster_orientation = caster.orientation;
    let caster_is_firstp = matches!(
        caster.attributes.get("player_camera"),
        Some(Value::PlayerCamera(
            PlayerCamera::D3FirstP | PlayerCamera::D3FirstPGrid
        ))
    );
    let target_pos = target.position;
    let had_cast_height = spell_item.attributes.contains("spell_cast_height");

    spell_item.set_attribute("is_spell", Value::Bool(true));
    if spell_item.attributes.get("visible").is_none() {
        spell_item.set_attribute("visible", Value::Bool(true));
    }
    apply_spell_default_attrs(&mut spell_item);
    spell_item.set_attribute("spell_caster_id", Value::UInt(caster_id));
    spell_item.set_attribute("spell_target_id", Value::UInt(target_id));

    let flight_height = spell_item
        .attributes
        .get_float_default("spell_flight_height", 0.5);
    let spawn_pos = Vec3::new(caster_pos.x, flight_height, caster_pos.z);
    let cast_time = spell_item
        .attributes
        .get_float_default("spell_cast_time", 0.0)
        .max(0.0);
    let cast_offset = spell_item
        .attributes
        .get_float_default("spell_cast_offset", 0.6)
        .max(0.0);
    let mut cast_height = spell_item
        .attributes
        .get_float_default("spell_cast_height", flight_height);
    if caster_is_firstp {
        if !had_cast_height {
            cast_height = cast_height.max(1.4);
        }
    }
    let mut dir = Vec2::new(target_pos.x - spawn_pos.x, target_pos.z - spawn_pos.z);
    if dir.magnitude_squared() <= 1e-6 {
        dir = caster_orientation;
    }
    if dir.magnitude_squared() <= 1e-6 {
        dir = Vec2::new(1.0, 0.0);
    } else {
        dir = dir.normalized();
    }
    if let Some(caster_mut) = ctx.map.entities.iter_mut().find(|e| e.id == caster_id) {
        caster_mut.set_orientation(dir);
    }

    spell_item.set_attribute("spell_dir_x", Value::Float(dir.x));
    spell_item.set_attribute("spell_dir_z", Value::Float(dir.y));
    spell_item.set_attribute("spell_travel", Value::Float(0.0));
    let lifetime = spell_item
        .attributes
        .get_float_default("spell_lifetime", 3.0);
    spell_item.set_attribute("spell_lifetime_left", Value::Float(lifetime));
    if cast_time > 0.0 {
        let hold_pos = Vec3::new(
            spawn_pos.x + dir.x * cast_offset,
            cast_height,
            spawn_pos.z + dir.y * cast_offset,
        );
        spell_item.set_attribute("spell_casting", Value::Bool(true));
        spell_item.set_attribute("spell_cast_left", Value::Float(cast_time));
        spell_item.set_attribute("spell_cast_height", Value::Float(cast_height));
        spell_item.set_attribute("spell_cast_offset", Value::Float(cast_offset));
        spell_item.set_position(hold_pos);
        if let Some(caster_mut) = ctx.map.entities.iter_mut().find(|e| e.id == caster_id) {
            caster_mut.set_attribute("spell_casting", Value::Bool(true));
        }
    } else {
        spell_item.set_position(spawn_pos);
    }
    spell_item.mark_all_dirty();
    let spell_id = spell_item.id as i32;
    let cooldown = spell_item
        .attributes
        .get_float_default("spell_cooldown", 0.0)
        .max(0.0);
    let on_cast_message = spell_item
        .attributes
        .get_str("on_cast")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    ctx.map.items.push(spell_item);
    set_spell_cooldown(ctx, caster_id, template, cooldown);
    if let Some(message) = on_cast_message {
        send_message(ctx, caster_id, message, "system");
    }
    spell_id
}

fn cast_spell_for_entity_to_pos(
    ctx: &mut RegionCtx,
    caster_id: u32,
    template: &str,
    target_pos_2d: Vec2<f32>,
    success_pct: f32,
) -> i32 {
    if is_spell_on_cooldown(ctx, caster_id, template) {
        return -1;
    }

    let success_pct = success_pct.clamp(0.0, 100.0);
    let mut rng = rand::rng();
    let roll = rng.random_range(0.0..100.0);
    if roll >= success_pct {
        ctx.to_execute_entity
            .push((caster_id, "cast_failed".into(), VMValue::zero()));
        return -1;
    }

    let Some(mut spell_item) = ctx.create_item(template.to_string()) else {
        return -1;
    };
    let Some(caster) = ctx.map.entities.iter().find(|e| e.id == caster_id) else {
        return -1;
    };
    let caster_pos = caster.position;
    let caster_orientation = caster.orientation;
    let caster_is_firstp = matches!(
        caster.attributes.get("player_camera"),
        Some(Value::PlayerCamera(
            PlayerCamera::D3FirstP | PlayerCamera::D3FirstPGrid
        ))
    );
    let had_cast_height = spell_item.attributes.contains("spell_cast_height");

    spell_item.set_attribute("is_spell", Value::Bool(true));
    if spell_item.attributes.get("visible").is_none() {
        spell_item.set_attribute("visible", Value::Bool(true));
    }
    apply_spell_default_attrs(&mut spell_item);
    spell_item.set_attribute("spell_caster_id", Value::UInt(caster_id));
    spell_item.set_attribute("spell_target_x", Value::Float(target_pos_2d.x));
    let flight_height = spell_item
        .attributes
        .get_float_default("spell_flight_height", 0.5);
    spell_item.set_attribute("spell_target_y", Value::Float(flight_height));
    spell_item.set_attribute("spell_target_z", Value::Float(target_pos_2d.y));

    let spawn_pos = Vec3::new(caster_pos.x, flight_height, caster_pos.z);
    let cast_time = spell_item
        .attributes
        .get_float_default("spell_cast_time", 0.0)
        .max(0.0);
    let cast_offset = spell_item
        .attributes
        .get_float_default("spell_cast_offset", 0.6)
        .max(0.0);
    let mut cast_height = spell_item
        .attributes
        .get_float_default("spell_cast_height", flight_height);
    if caster_is_firstp {
        if !had_cast_height {
            cast_height = cast_height.max(1.4);
        }
    }
    let mut dir = Vec2::new(target_pos_2d.x - spawn_pos.x, target_pos_2d.y - spawn_pos.z);
    if dir.magnitude_squared() <= 1e-6 {
        dir = caster_orientation;
    }
    if dir.magnitude_squared() <= 1e-6 {
        dir = Vec2::new(1.0, 0.0);
    } else {
        dir = dir.normalized();
    }
    if let Some(caster_mut) = ctx.map.entities.iter_mut().find(|e| e.id == caster_id) {
        caster_mut.set_orientation(dir);
    }

    spell_item.set_attribute("spell_dir_x", Value::Float(dir.x));
    spell_item.set_attribute("spell_dir_z", Value::Float(dir.y));
    spell_item.set_attribute("spell_travel", Value::Float(0.0));
    let lifetime = spell_item
        .attributes
        .get_float_default("spell_lifetime", 3.0);
    spell_item.set_attribute("spell_lifetime_left", Value::Float(lifetime));
    if cast_time > 0.0 {
        let hold_pos = Vec3::new(
            spawn_pos.x + dir.x * cast_offset,
            cast_height,
            spawn_pos.z + dir.y * cast_offset,
        );
        spell_item.set_attribute("spell_casting", Value::Bool(true));
        spell_item.set_attribute("spell_cast_left", Value::Float(cast_time));
        spell_item.set_attribute("spell_cast_height", Value::Float(cast_height));
        spell_item.set_attribute("spell_cast_offset", Value::Float(cast_offset));
        spell_item.set_position(hold_pos);
        if let Some(caster_mut) = ctx.map.entities.iter_mut().find(|e| e.id == caster_id) {
            caster_mut.set_attribute("spell_casting", Value::Bool(true));
        }
    } else {
        spell_item.set_position(spawn_pos);
    }
    spell_item.mark_all_dirty();
    let spell_id = spell_item.id as i32;
    let cooldown = spell_item
        .attributes
        .get_float_default("spell_cooldown", 0.0)
        .max(0.0);
    let on_cast_message = spell_item
        .attributes
        .get_str("on_cast")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    ctx.map.items.push(spell_item);
    set_spell_cooldown(ctx, caster_id, template, cooldown);
    if let Some(message) = on_cast_message {
        send_message(ctx, caster_id, message, "system");
    }
    spell_id
}

fn spell_target_filter_allows(
    filter: &str,
    caster_id: u32,
    target_id: u32,
    entity_attrs: &FxHashMap<u32, ValueContainer>,
    alignment: &FxHashMap<u32, i32>,
) -> bool {
    let trimmed = filter.trim();
    let filter = trimmed.to_ascii_lowercase();
    match filter.as_str() {
        "self" => caster_id == target_id,
        "ally" => {
            if caster_id == 0 {
                return false;
            }
            let caster_alignment = alignment.get(&caster_id).copied().unwrap_or(0);
            let target_alignment = alignment.get(&target_id).copied().unwrap_or(0);
            caster_alignment != 0 && caster_alignment == target_alignment
        }
        "enemy" => {
            if caster_id == 0 {
                return true;
            }
            let caster_alignment = alignment.get(&caster_id).copied().unwrap_or(0);
            let target_alignment = alignment.get(&target_id).copied().unwrap_or(0);
            caster_alignment == 0 || target_alignment == 0 || caster_alignment != target_alignment
        }
        _ => {
            if let Some((lhs, op, rhs)) = parse_filter_expr(trimmed)
                && let Some(attrs) = entity_attrs.get(&target_id)
                && let Some(lhs_v) = numeric_attr(attrs, lhs)
            {
                return match op {
                    "<" => lhs_v < rhs,
                    "<=" => lhs_v <= rhs,
                    ">" => lhs_v > rhs,
                    ">=" => lhs_v >= rhs,
                    "==" => (lhs_v - rhs).abs() <= f32::EPSILON,
                    "!=" => (lhs_v - rhs).abs() > f32::EPSILON,
                    _ => false,
                };
            }
            true // "any" and unknown values
        }
    }
}

fn close_visible_damage_allowed(ctx: &RegionCtx, from_id: u32, target_id: u32) -> bool {
    let Some(attacker) = ctx.map.entities.iter().find(|e| e.id == from_id) else {
        return false;
    };
    let Some(target) = ctx.map.entities.iter().find(|e| e.id == target_id) else {
        return false;
    };

    let attacker_pos = attacker.get_pos_xz();
    let target_pos = target.get_pos_xz();
    if attacker_pos.distance(target_pos) > 2.1 {
        return false;
    }

    let attacker_tile = attacker_pos.floor().as_::<i32>();
    let target_tile = target_pos.floor().as_::<i32>();
    ctx.mapmini.is_tile_visible(attacker_tile, target_tile)
        && ctx.mapmini.is_visible(attacker_pos, target_pos)
}

pub(crate) fn apply_damage_direct(
    ctx: &mut RegionCtx,
    target_id: u32,
    from_id: u32,
    amount: i32,
    kind: &str,
    source_item_id: Option<u32>,
) -> bool {
    if amount <= 0 {
        return false;
    }

    let health_attr = ctx.health_attr.clone();
    let mut kill = false;
    let mut enqueue_death = false;
    let mut should_autodrop = false;
    let attacker_name = ctx.get_entity_name(from_id);
    let defender_name = ctx.get_entity_name(target_id);

    let attr_matches_target_id = |value: &Value, target_id: u32| match value {
        Value::UInt(id) => *id == target_id,
        Value::Int(id) => *id >= 0 && *id as u32 == target_id,
        Value::Int64(id) => *id >= 0 && *id as u32 == target_id,
        Value::Str(id) => id.trim().parse::<u32>().ok() == Some(target_id),
        _ => false,
    };

    let attacker_sector = ctx
        .map
        .entities
        .iter()
        .find(|e| e.id == from_id)
        .and_then(|entity| {
            entity
                .attributes
                .get("sector_id")
                .and_then(|value| match value {
                    Value::Int64(v) if *v >= 0 => Some(*v as u32),
                    Value::Int(v) if *v >= 0 => Some(*v as u32),
                    _ => None,
                })
                .or_else(|| ctx.map.find_sector_at(entity.get_pos_xz()).map(|s| s.id))
        })
        .map(|id| id.to_string())
        .unwrap_or_else(|| "<none>".to_string());
    let target_sector_before = ctx
        .map
        .entities
        .iter()
        .find(|e| e.id == target_id)
        .and_then(|entity| {
            entity
                .attributes
                .get("sector_id")
                .and_then(|value| match value {
                    Value::Int64(v) if *v >= 0 => Some(*v as u32),
                    Value::Int(v) if *v >= 0 => Some(*v as u32),
                    _ => None,
                })
                .or_else(|| ctx.map.find_sector_at(entity.get_pos_xz()).map(|s| s.id))
        })
        .map(|id| id.to_string())
        .unwrap_or_else(|| "<none>".to_string());

    if from_id != 0
        && attacker_sector != "<none>"
        && target_sector_before != "<none>"
        && attacker_sector != target_sector_before
        && !close_visible_damage_allowed(ctx, from_id, target_id)
    {
        return false;
    }

    if let Some(entity) = ctx.map.entities.iter_mut().find(|e| e.id == target_id)
        && let Some(mut health) = entity.attributes.get_int(&health_attr)
    {
        health -= amount;
        health = health.max(0);
        entity.set_attribute(&health_attr, Value::Int(health));

        let mode = entity.attributes.get_str_default("mode", "".into());
        if health <= 0 && mode != "dead" {
            enqueue_death = true;
            entity.set_attribute("mode", Value::Str("dead".into()));
            entity.set_attribute("visible", Value::Bool(false));
            entity.action = EntityAction::Off;
            ctx.entity_proximity_alerts.remove(&target_id);
            should_autodrop = entity.attributes.get_bool_default("autodrop", false);
            kill = true;
        }
    }

    if kill {
        ctx.to_execute_entity.retain(|(id, event, payload)| {
            if *id == target_id {
                return event == "death";
            }
            // Drop any already-queued direct damage payloads still targeting the dead entity.
            if event == "__apply_damage" && payload.x.max(0.0) as u32 == target_id {
                return false;
            }
            // Guard against stale queued broadcasts encoding the dead entity as a target.
            if payload.x.max(0.0) as u32 == target_id && event == "take_damage" {
                return false;
            }
            true
        });
        ctx.notifications_entities
            .retain(|(id, _, _)| *id != target_id);

        for entity in &mut ctx.map.entities {
            let target_matches = entity
                .attributes
                .get("target")
                .map(|value| attr_matches_target_id(value, target_id))
                .unwrap_or(false);
            if target_matches {
                entity.set_attribute("target", Value::Str(String::new()));
                ctx.notifications_entities
                    .retain(|(id, _, event)| *id != entity.id || event != "attack");
            }

            let attack_target_matches = entity
                .attributes
                .get("attack_target")
                .map(|value| attr_matches_target_id(value, target_id))
                .unwrap_or(false);
            if attack_target_matches {
                entity.set_attribute("attack_target", Value::Str(String::new()));
                ctx.notifications_entities
                    .retain(|(id, _, event)| *id != entity.id || event != "attack");
            }
        }

        for item in &mut ctx.map.items {
            let target_matches = item
                .attributes
                .get("target")
                .map(|value| attr_matches_target_id(value, target_id))
                .unwrap_or(false);
            if target_matches {
                item.set_attribute("target", Value::Str(String::new()));
                ctx.notifications_items
                    .retain(|(id, _, event)| *id != item.id || event != "attack");
            }

            let attack_target_matches = item
                .attributes
                .get("attack_target")
                .map(|value| attr_matches_target_id(value, target_id))
                .unwrap_or(false);
            if attack_target_matches {
                item.set_attribute("attack_target", Value::Str(String::new()));
                ctx.notifications_items
                    .retain(|(id, _, event)| *id != item.id || event != "attack");
            }
        }

        if let Some(state) = ctx.entity_state_data.get_mut(&target_id) {
            state.remove("__under_attack_by");
        }
    }

    if kill && should_autodrop {
        drop_all_items_for_entity(ctx, target_id);
    }

    if enqueue_death {
        ctx.to_execute_entity
            .push((target_id, "death".into(), VMValue::zero()));
    }

    send_damage_rule_messages(
        ctx,
        from_id,
        target_id,
        amount,
        kind,
        source_item_id,
        &attacker_name,
        &defender_name,
    );

    if kill {
        let attacker_can_receive_kill = from_id != 0
            && ctx
                .map
                .entities
                .iter()
                .any(|entity| entity.id == from_id && entity.get_mode() != "dead");
        if attacker_can_receive_kill {
            let xp = progression_kill_xp(ctx, from_id, target_id);
            ctx.to_execute_entity.push((
                from_id,
                "kill".into(),
                VMValue::broadcast(target_id as f32),
            ));
            if xp > 0 {
                ctx.to_execute_entity.push((
                    from_id,
                    "__grant_xp".into(),
                    VMValue::broadcast(xp as f32),
                ));
            }
        }
    }

    kill
}

fn combat_rule_expr_from_root<'a>(
    root: &'a toml::value::Table,
    kind: &str,
    key: &str,
) -> Option<&'a str> {
    let kind_key = if key == "incoming_damage" {
        Some(["incoming_damage", "received_damage"])
    } else {
        None
    };
    if !kind.is_empty()
        && let Some(expr) = root
            .get("combat")
            .and_then(toml::Value::as_table)
            .and_then(|combat| combat.get("kinds"))
            .and_then(toml::Value::as_table)
            .and_then(|kinds| kinds.get(kind))
            .and_then(toml::Value::as_table)
            .and_then(|kind_table| {
                if let Some(keys) = kind_key {
                    keys.iter().find_map(|key| kind_table.get(*key))
                } else {
                    kind_table.get(key)
                }
            })
            .and_then(toml::Value::as_str)
    {
        return Some(expr);
    }
    root.get("combat")
        .and_then(toml::Value::as_table)
        .and_then(|combat| {
            if let Some(keys) = kind_key {
                keys.iter().find_map(|key| combat.get(*key))
            } else {
                combat.get(key)
            }
        })
        .and_then(toml::Value::as_str)
}

fn combat_rule_expr<'a>(ctx: &'a RegionCtx, kind: &str, key: &str) -> Option<&'a str> {
    combat_rule_expr_from_root(&ctx.rules, kind, key)
}

fn combat_rule_exprs<'a>(
    ctx: &'a RegionCtx,
    attacker: Option<&Entity>,
    kind: &str,
    key: &str,
) -> Vec<&'a str> {
    let mut exprs = Vec::new();
    if let Some(expr) = combat_rule_expr(ctx, kind, key) {
        exprs.push(expr);
    }
    if let Some(attacker) = attacker
        && let Some(root) = race_rule_root(ctx, attacker)
        && let Some(expr) = combat_rule_expr_from_root(root, kind, key)
    {
        exprs.push(expr);
    }
    if let Some(attacker) = attacker
        && let Some(root) = class_rule_root(ctx, attacker)
        && let Some(expr) = combat_rule_expr_from_root(root, kind, key)
    {
        exprs.push(expr);
    }
    exprs
}

fn active_locale(ctx: &RegionCtx) -> &str {
    let configured = ctx
        .config
        .get("game")
        .and_then(toml::Value::as_table)
        .and_then(|game| game.get("locale"))
        .and_then(toml::Value::as_str)
        .filter(|locale| !locale.trim().is_empty())
        .unwrap_or("en");

    resolve_runtime_locale(&ctx.assets, configured)
}

fn normalize_locale(locale: &str) -> String {
    locale
        .trim()
        .replace('-', "_")
        .split('.')
        .next()
        .unwrap_or("en")
        .to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_system_locale() -> Option<String> {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(normalize_locale(value));
            }
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn detect_system_locale() -> Option<String> {
    None
}

fn locale_candidates(locale: &str) -> Vec<String> {
    let normalized = normalize_locale(locale);
    let mut candidates = vec![normalized.clone()];
    if let Some((base, _)) = normalized.split_once('_')
        && base != normalized
    {
        candidates.push(base.to_string());
    }
    if !candidates.iter().any(|candidate| candidate == "en") {
        candidates.push("en".to_string());
    }
    candidates
}

fn resolve_runtime_locale<'a>(assets: &'a Assets, configured: &str) -> &'a str {
    let requested = if configured.eq_ignore_ascii_case("auto") {
        detect_system_locale().unwrap_or_else(|| "en".to_string())
    } else {
        configured.to_string()
    };

    for candidate in locale_candidates(&requested) {
        if assets.locales.contains_key(&candidate) {
            return assets
                .locales
                .get_key_value(&candidate)
                .map(|(key, _)| key.as_str())
                .unwrap();
        }
    }

    "en"
}

fn parse_intent_distance_limit(data: &str, intent: &str) -> Option<f32> {
    let table = data.parse::<toml::Table>().ok()?;
    let distances = table.get("intent_distance")?.as_table()?;
    let intent_key = intent.trim().to_ascii_lowercase();

    let specific = distances.get(&intent_key).and_then(|value| {
        value
            .as_float()
            .or_else(|| value.as_integer().map(|v| v as f64))
    });
    let default = distances.get("default").and_then(|value| {
        value
            .as_float()
            .or_else(|| value.as_integer().map(|v| v as f64))
    });

    specific
        .or(default)
        .map(|value| value as f32)
        .filter(|v| *v >= 0.0)
}

fn entity_intent_distance_limit(ctx: &RegionCtx, entity_id: u32, intent: &str) -> Option<f32> {
    let class_name = ctx.entity_classes.get(&entity_id)?;
    let data = ctx.entity_class_data.get(class_name)?;
    Some(parse_intent_distance_limit(data, intent).unwrap_or(2.0))
}

fn choice_session_is_valid(
    ctx: &RegionCtx,
    from_id: u32,
    to_id: u32,
    expires_at_tick: i64,
    max_distance: f32,
) -> bool {
    if ctx.ticks > expires_at_tick {
        return false;
    }

    let Some(from_entity) = ctx.map.entities.iter().find(|entity| entity.id == from_id) else {
        return false;
    };
    let Some(to_entity) = ctx.map.entities.iter().find(|entity| entity.id == to_id) else {
        return false;
    };

    from_entity.get_pos_xz().distance(to_entity.get_pos_xz()) <= max_distance
}

fn clear_choice_session(ctx: &mut RegionCtx, from_id: u32, to_id: u32) {
    ctx.active_choice_sessions
        .retain(|session| !(session.from == from_id && session.to == to_id));
}

fn dialog_condition_met(ctx: &RegionCtx, from_id: u32, to_id: u32, condition: &str) -> bool {
    let condition = condition.trim();
    if condition.is_empty() {
        return true;
    }

    fn value_truthy(value: &Value) -> bool {
        match value {
            Value::Bool(value) => *value,
            Value::Int(value) => *value != 0,
            Value::UInt(value) => *value != 0,
            Value::Int64(value) => *value != 0,
            Value::Float(value) => *value != 0.0,
            Value::Str(value) => !value.trim().is_empty() && value != "false" && value != "0",
            Value::StrArray(value) => !value.is_empty(),
            _ => false,
        }
    }

    let (scope, key) = condition
        .split_once('.')
        .map(|(scope, key)| (Some(scope.trim()), key.trim()))
        .unwrap_or((None, condition));

    let resolve_entity = |id| {
        ctx.map
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .and_then(|entity| entity.attributes.get(key))
            .is_some_and(value_truthy)
    };

    match scope {
        Some("self") => resolve_entity(from_id),
        Some("target") | Some("player") => resolve_entity(to_id),
        Some("region") | Some("world") => ctx.region_state.get(key).is_some_and(value_truthy),
        _ => {
            resolve_entity(from_id)
                || resolve_entity(to_id)
                || ctx.region_state.get(key).is_some_and(value_truthy)
        }
    }
}

fn dialog_choice_visible(
    ctx: &RegionCtx,
    from_id: u32,
    to_id: u32,
    choice: &toml::value::Table,
) -> bool {
    if let Some(condition) = choice.get("if").and_then(toml::Value::as_str)
        && !dialog_condition_met(ctx, from_id, to_id, condition)
    {
        return false;
    }
    if let Some(condition) = choice.get("unless").and_then(toml::Value::as_str)
        && dialog_condition_met(ctx, from_id, to_id, condition)
    {
        return false;
    }
    true
}

fn dialog_node_table<'a>(
    dialog: &'a toml::value::Table,
    node_name: &str,
) -> Option<&'a toml::value::Table> {
    dialog
        .get("nodes")
        .and_then(toml::Value::as_table)
        .and_then(|nodes| nodes.get(node_name))
        .and_then(toml::Value::as_table)
        .or_else(|| dialog.get(node_name).and_then(toml::Value::as_table))
}

pub fn open_dialog_node(ctx: &mut RegionCtx, from_id: u32, to_id: u32, node_name: &str) -> bool {
    let Some(class_name) = ctx.entity_classes.get(&from_id).cloned() else {
        return false;
    };
    let Some(class_data) = ctx.entity_class_data.get(&class_name) else {
        return false;
    };
    let Ok(data) = class_data.parse::<toml::Table>() else {
        return false;
    };
    let Some(dialog) = data.get("dialog").and_then(toml::Value::as_table) else {
        return false;
    };

    let node_name = if node_name.trim().is_empty() {
        dialog
            .get("start")
            .and_then(toml::Value::as_str)
            .unwrap_or("start")
    } else {
        node_name.trim()
    };
    let Some(node) = dialog_node_table(dialog, node_name) else {
        return false;
    };

    let timeout_minutes = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == from_id)
        .map(|entity| {
            entity
                .attributes
                .get_float_default("timeout", 10.0)
                .max(0.0)
        })
        .unwrap_or(10.0);
    let expires_at_tick = ctx.ticks + (ctx.ticks_per_minute as f32 * timeout_minutes) as i64;
    let max_distance = entity_intent_distance_limit(ctx, from_id, "talk").unwrap_or(2.0);

    let text = node
        .get("text")
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .trim();
    if !text.is_empty()
        && let Some(sender) = ctx.from_sender.get()
    {
        let _ = sender.send(RegionMessage::Message(
            ctx.region_id,
            Some(from_id),
            None,
            to_id,
            text.to_string(),
            "dialog".into(),
        ));
    }

    let mut choices =
        MultipleChoice::new(ctx.region_id, from_id, to_id, expires_at_tick, max_distance);

    if let Some(choice_values) = node.get("choices").and_then(toml::Value::as_array) {
        for choice_value in choice_values {
            let Some(choice) = choice_value.as_table() else {
                continue;
            };
            if !dialog_choice_visible(ctx, from_id, to_id, choice) {
                continue;
            }
            let label = choice
                .get("label")
                .and_then(toml::Value::as_str)
                .unwrap_or("{dialog.continue}")
                .trim();
            if label.is_empty() {
                continue;
            }
            choices.add(Choice::DialogChoice(DialogChoice {
                label: label.to_string(),
                dialog: node_name.to_string(),
                from: from_id,
                to: to_id,
                index: choices.choices.len() as u32,
                next: choice
                    .get("next")
                    .and_then(toml::Value::as_str)
                    .map(str::to_string),
                event: choice
                    .get("event")
                    .and_then(toml::Value::as_str)
                    .map(str::to_string),
                end: choice
                    .get("end")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(false),
                expires_at_tick,
                max_distance,
            }));
        }
    }

    if !choices.choices.is_empty()
        && let Some(sender) = ctx.from_sender.get().cloned()
    {
        clear_choice_session(ctx, from_id, to_id);
        ctx.active_choice_sessions.push(ChoiceSession {
            from: from_id,
            to: to_id,
            expires_at_tick,
            max_distance,
        });
        let _ = sender.send(RegionMessage::MultipleChoice(choices));
    } else {
        clear_choice_session(ctx, from_id, to_id);
    }

    true
}

fn queue_intent_cooldown(
    ctx: &mut RegionCtx,
    entity_id: u32,
    intent: &str,
    cooldown_minutes: Option<f32>,
) {
    let Some(minutes) = cooldown_minutes else {
        return;
    };
    let intent = intent.trim().to_ascii_lowercase();
    if intent.is_empty() {
        return;
    }
    let target_tick = ctx.ticks + RegionInstance::scheduled_delay_ticks(ctx, minutes);
    let state = ctx.entity_state_data.entry(entity_id).or_default();
    state.set(
        &format!("__pending_intent_cooldown:{}", intent),
        Value::Int64(target_tick),
    );
}

#[derive(Default)]
struct IntentRuleConfig {
    allowed: Option<String>,
    deny_message: Option<String>,
    cooldown_minutes: Option<f32>,
}

fn merge_intent_rule_config(config: &mut IntentRuleConfig, table: &toml::value::Table) {
    if let Some(value) = table.get("allowed").and_then(toml::Value::as_str)
        && !value.trim().is_empty()
    {
        config.allowed = Some(value.trim().to_string());
    }
    if let Some(value) = table.get("deny_message").and_then(toml::Value::as_str)
        && !value.trim().is_empty()
    {
        config.deny_message = Some(value.trim().to_string());
    }
    if let Some(value) = table.get("cooldown").and_then(|value| {
        value
            .as_float()
            .or_else(|| value.as_integer().map(|v| v as f64))
    }) {
        config.cooldown_minutes = Some(value as f32);
    }
}

fn intent_rule_config_from_data(data: &str, intent: &str) -> Option<IntentRuleConfig> {
    let table = data.parse::<toml::Table>().ok()?;
    let intents = table.get("intents")?.as_table()?;
    let intent_table = intents.get(intent)?.as_table()?;
    let mut config = IntentRuleConfig::default();
    merge_intent_rule_config(&mut config, intent_table);
    Some(config)
}

fn intent_rule_config(ctx: &RegionCtx, entity_id: u32, intent: &str) -> IntentRuleConfig {
    let mut config = IntentRuleConfig::default();
    if let Some(global) = ctx
        .rules
        .get("intents")
        .and_then(toml::Value::as_table)
        .and_then(|intents| intents.get(intent))
        .and_then(toml::Value::as_table)
    {
        merge_intent_rule_config(&mut config, global);
    }

    if let Some(class_name) = ctx.entity_classes.get(&entity_id)
        && let Some(data) = ctx.entity_class_data.get(class_name)
        && let Some(local) = intent_rule_config_from_data(data, intent)
    {
        if local.allowed.is_some() {
            config.allowed = local.allowed;
        }
        if local.deny_message.is_some() {
            config.deny_message = local.deny_message;
        }
        if local.cooldown_minutes.is_some() {
            config.cooldown_minutes = local.cooldown_minutes;
        }
    }

    config
}

fn localized_template(ctx: &RegionCtx, key: &str) -> Option<String> {
    let locale = active_locale(ctx);
    ctx.assets
        .locales
        .get(locale)
        .and_then(|translations| translations.get(key))
        .cloned()
        .or_else(|| {
            if let Some((base, _)) = locale.split_once('_') {
                return ctx
                    .assets
                    .locales
                    .get(base)
                    .and_then(|translations| translations.get(key))
                    .cloned();
            }
            None
        })
        .or_else(|| {
            ctx.assets
                .locales
                .get("en")
                .and_then(|translations| translations.get(key))
                .cloned()
        })
}

fn authored_description_from_entry(value: &toml::Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(table) = value.as_table()
        && let Some(text) = table.get("description").and_then(toml::Value::as_str)
    {
        let text = text.trim();
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    None
}

fn authored_text_from_entry(value: &toml::Value, key: &str) -> Option<String> {
    if let Some(table) = value.as_table()
        && let Some(text) = table.get(key).and_then(toml::Value::as_str)
    {
        let text = text.trim();
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    None
}

fn authored_description_from_data(
    data: &str,
    mode: Option<&str>,
    state: Option<&str>,
) -> Option<String> {
    let table = data.parse::<toml::Table>().ok()?;

    if let Some(mode) = mode
        && let Some(entries) = table.get("mode").and_then(toml::Value::as_table)
        && let Some(value) = entries.get(mode)
        && let Some(description) = authored_description_from_entry(value)
    {
        return Some(description);
    }

    if let Some(state) = state
        && let Some(entries) = table.get("state").and_then(toml::Value::as_table)
        && let Some(value) = entries.get(state)
        && let Some(description) = authored_description_from_entry(value)
    {
        return Some(description);
    }

    table
        .get("description")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn authored_state_text_from_data(
    data: &str,
    mode: Option<&str>,
    state: Option<&str>,
    key: &str,
) -> Option<String> {
    let table = data.parse::<toml::Table>().ok()?;

    if let Some(mode) = mode
        && let Some(entries) = table.get("mode").and_then(toml::Value::as_table)
        && let Some(value) = entries.get(mode)
        && let Some(text) = authored_text_from_entry(value, key)
    {
        return Some(text);
    }

    if let Some(state) = state
        && let Some(entries) = table.get("state").and_then(toml::Value::as_table)
        && let Some(value) = entries.get(state)
        && let Some(text) = authored_text_from_entry(value, key)
    {
        return Some(text);
    }

    None
}

fn entity_look_description(ctx: &RegionCtx, entity: &Entity) -> Option<String> {
    let class_name = entity.get_attr_string("class_name")?;
    let data = ctx.entity_authoring_data.get(&class_name)?;
    let mode = entity.get_attr_string("mode");
    authored_description_from_data(data, mode.as_deref(), None)
}

fn item_look_description(ctx: &RegionCtx, item: &Item) -> Option<String> {
    let class_name = item.get_attr_string("class_name")?;
    let data = ctx.item_authoring_data.get(&class_name)?;
    let state = item
        .get_attr_string("state")
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            if item.attributes.get_bool_default("active", false) {
                Some("on".to_string())
            } else {
                Some("off".to_string())
            }
        });
    authored_description_from_data(data, None, state.as_deref())
}

fn item_use_message(ctx: &RegionCtx, item: &Item) -> Option<String> {
    let class_name = item.get_attr_string("class_name")?;
    let data = ctx.item_authoring_data.get(&class_name)?;
    let state = item
        .get_attr_string("state")
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            if item.attributes.get_bool_default("active", false) {
                Some("on".to_string())
            } else {
                Some("off".to_string())
            }
        });
    authored_state_text_from_data(data, None, state.as_deref(), "on_use")
}

fn combat_message_template(ctx: &RegionCtx, key: &str) -> Option<String> {
    let messages = ctx
        .rules
        .get("combat")
        .and_then(toml::Value::as_table)
        .and_then(|combat| combat.get("messages"))
        .and_then(toml::Value::as_table)?;

    if let Some(locale_key) = messages
        .get(&format!("{}_key", key))
        .and_then(toml::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        && let Some(template) = localized_template(ctx, locale_key)
    {
        return Some(template);
    }

    messages
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
}

fn progression_message_template(ctx: &RegionCtx, key: &str) -> Option<String> {
    let messages = ctx
        .rules
        .get("progression")
        .and_then(toml::Value::as_table)
        .and_then(|progression| progression.get("messages"))
        .and_then(toml::Value::as_table)?;

    if let Some(locale_key) = messages
        .get(&format!("{}_key", key))
        .and_then(toml::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        && let Some(template) = localized_template(ctx, locale_key)
    {
        return Some(template);
    }

    messages
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
}

fn combat_audio_string(ctx: &RegionCtx, kind: &str, key: &str) -> Option<String> {
    if !kind.is_empty()
        && let Some(value) = ctx
            .rules
            .get("combat")
            .and_then(toml::Value::as_table)
            .and_then(|combat| combat.get("kinds"))
            .and_then(toml::Value::as_table)
            .and_then(|kinds| kinds.get(kind))
            .and_then(toml::Value::as_table)
            .and_then(|kind_table| kind_table.get("audio"))
            .and_then(toml::Value::as_table)
            .and_then(|audio| audio.get(key))
            .and_then(toml::Value::as_str)
            .filter(|value| !value.trim().is_empty())
    {
        return Some(value.to_string());
    }

    ctx.rules
        .get("combat")
        .and_then(toml::Value::as_table)
        .and_then(|combat| combat.get("audio"))
        .and_then(toml::Value::as_table)
        .and_then(|audio| audio.get(key))
        .and_then(toml::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

fn combat_audio_gain(ctx: &RegionCtx, kind: &str, key: &str) -> f32 {
    if !kind.is_empty()
        && let Some(value) = ctx
            .rules
            .get("combat")
            .and_then(toml::Value::as_table)
            .and_then(|combat| combat.get("kinds"))
            .and_then(toml::Value::as_table)
            .and_then(|kinds| kinds.get(kind))
            .and_then(toml::Value::as_table)
            .and_then(|kind_table| kind_table.get("audio"))
            .and_then(toml::Value::as_table)
            .and_then(|audio| audio.get(key))
            .and_then(toml::Value::as_float)
    {
        return value as f32;
    }

    ctx.rules
        .get("combat")
        .and_then(toml::Value::as_table)
        .and_then(|combat| combat.get("audio"))
        .and_then(toml::Value::as_table)
        .and_then(|audio| audio.get(key))
        .and_then(toml::Value::as_float)
        .map(|value| value as f32)
        .unwrap_or(1.0)
}

fn combat_message_category(ctx: &RegionCtx, key: &str) -> String {
    ctx.rules
        .get("combat")
        .and_then(toml::Value::as_table)
        .and_then(|combat| combat.get("messages"))
        .and_then(toml::Value::as_table)
        .and_then(|messages| messages.get(key))
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "system".to_string())
}

fn progression_message_category(ctx: &RegionCtx, key: &str) -> String {
    ctx.rules
        .get("progression")
        .and_then(toml::Value::as_table)
        .and_then(|progression| progression.get("messages"))
        .and_then(toml::Value::as_table)
        .and_then(|messages| messages.get(key))
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "system".to_string())
}

fn render_damage_message(
    template: &str,
    attacker: &str,
    defender: &str,
    amount: i32,
    kind: &str,
    from_id: u32,
    target_id: u32,
) -> String {
    template
        .replace("{attacker}", attacker)
        .replace("{defender}", defender)
        .replace("{amount}", &amount.to_string())
        .replace("{kind}", kind)
        .replace("{from_id}", &from_id.to_string())
        .replace("{target_id}", &target_id.to_string())
}

fn render_progression_message(
    template: &str,
    amount: i32,
    level: Option<u32>,
    xp_total: i32,
) -> String {
    let mut rendered = template
        .replace("{amount}", &amount.to_string())
        .replace("{xp_total}", &xp_total.to_string());
    if let Some(level) = level {
        rendered = rendered.replace("{level}", &level.to_string());
    }
    rendered
}

fn is_player_message_recipient(ctx: &RegionCtx, entity_id: u32) -> bool {
    ctx.map
        .entities
        .iter()
        .find(|entity| entity.id == entity_id)
        .map(|entity| entity.is_player())
        .unwrap_or(false)
}

fn item_by_id<'a>(ctx: &'a RegionCtx, item_id: u32) -> Option<&'a Item> {
    if let Some(item) = ctx.map.items.iter().find(|item| item.id == item_id) {
        return Some(item);
    }
    for entity in &ctx.map.entities {
        if let Some(item) = entity
            .inventory
            .iter()
            .flatten()
            .find(|item| item.id == item_id)
        {
            return Some(item);
        }
        for slot in entity.equipped.values() {
            if slot.id == item_id {
                return Some(slot);
            }
        }
    }
    None
}

fn progression_kill_xp(ctx: &RegionCtx, from_id: u32, target_id: u32) -> i32 {
    let attacker = ctx.map.entities.iter().find(|entity| entity.id == from_id);
    let defender = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == target_id);

    let mut exprs = Vec::new();
    if let Some(expr) = ctx
        .rules
        .get("progression")
        .and_then(toml::Value::as_table)
        .and_then(|progression| progression.get("xp"))
        .and_then(toml::Value::as_table)
        .and_then(|xp| xp.get("kill"))
        .and_then(toml::Value::as_str)
    {
        exprs.push(expr);
    }
    if let Some(attacker) = attacker
        && let Some(root) = race_rule_root(ctx, attacker)
        && let Some(expr) = root
            .get("progression")
            .and_then(toml::Value::as_table)
            .and_then(|progression| progression.get("xp"))
            .and_then(toml::Value::as_table)
            .and_then(|xp| xp.get("kill"))
            .and_then(toml::Value::as_str)
    {
        exprs.push(expr);
    }
    if let Some(attacker) = attacker
        && let Some(root) = class_rule_root(ctx, attacker)
        && let Some(expr) = root
            .get("progression")
            .and_then(toml::Value::as_table)
            .and_then(|progression| progression.get("xp"))
            .and_then(toml::Value::as_table)
            .and_then(|xp| xp.get("kill"))
            .and_then(toml::Value::as_str)
    {
        exprs.push(expr);
    }

    if exprs.is_empty() {
        return 0;
    }

    let mut current_value = 0.0;
    for expr in exprs {
        let Some(parsed) = FormulaParser::new(expr, |name| {
            resolve_combat_var(ctx, name, current_value, attacker, defender, None)
        })
        .parse() else {
            return 0;
        };
        if !parsed.is_finite() {
            return 0;
        }
        current_value = parsed.max(0.0);
    }

    current_value.round().max(0.0) as i32
}

pub(crate) fn grant_experience(ctx: &mut RegionCtx, entity_id: u32, amount: i32) -> Vec<u32> {
    if amount <= 0 {
        return Vec::new();
    }

    let amount_f = amount as f32;
    let level_attr = ctx.level_attr.clone();
    let experience_attr = ctx.experience_attr.clone();

    let (new_xp, mut level) = if let Some(entity) = ctx.get_entity_mut(entity_id) {
        let new_xp = entity.attributes.get_float_default(&experience_attr, 0.0) + amount_f;
        let level = entity
            .attributes
            .get_float_default(&level_attr, 1.0)
            .round()
            .max(1.0) as u32;
        entity.set_attribute(&experience_attr, Value::Float(new_xp));
        (new_xp, level)
    } else {
        return Vec::new();
    };

    let mut level_ups = Vec::new();
    loop {
        let Some(required_xp) = progression_xp_for_level(ctx, entity_id, level + 1) else {
            break;
        };
        if new_xp + f32::EPSILON < required_xp {
            break;
        }
        level += 1;
        level_ups.push(level);
    }

    if !level_ups.is_empty() {
        if let Some(entity) = ctx.get_entity_mut(entity_id) {
            entity.set_attribute(&level_attr, Value::Int(level as i32));
        }
        for level in &level_ups {
            ctx.to_execute_entity.push((
                entity_id,
                "level_up".into(),
                VMValue::broadcast(*level as f32),
            ));
        }
    }

    if is_player_message_recipient(ctx, entity_id) {
        let xp_total = new_xp.round() as i32;
        if let Some(template) = progression_message_template(ctx, "xp") {
            let category = progression_message_category(ctx, "xp_category");
            let message =
                render_progression_message(&template, amount, level_ups.last().copied(), xp_total);
            if !message.trim().is_empty() {
                send_message(ctx, entity_id, message, &category);
            }
        }
        for level in &level_ups {
            if let Some(template) = progression_message_template(ctx, "level_up") {
                let category = progression_message_category(ctx, "level_up_category");
                let message = render_progression_message(&template, amount, Some(*level), xp_total);
                if !message.trim().is_empty() {
                    send_message(ctx, entity_id, message, &category);
                }
            }
        }
    }

    level_ups
}

fn equipped_audio_item<'a>(ctx: &'a RegionCtx, attacker_id: u32) -> Option<&'a Item> {
    ctx.map
        .entities
        .iter()
        .find(|entity| entity.id == attacker_id)
        .and_then(|entity| {
            for slot in ["main_hand", "mainhand", "weapon", "hand_main", "off_hand"] {
                if let Some(item) = entity.get_equipped_item(slot) {
                    return Some(item);
                }
            }
            None
        })
}

fn item_audio_override(
    item: &Item,
    key_prefix: &str,
) -> Option<(String, Option<String>, Option<f32>)> {
    let name = item
        .attributes
        .get_str(&format!("{}_fx", key_prefix))
        .filter(|value| !value.trim().is_empty())?;
    let bus = item
        .attributes
        .get_str(&format!("{}_bus", key_prefix))
        .filter(|value| !value.trim().is_empty());
    let gain = item.attributes.get_float(&format!("{}_gain", key_prefix));
    Some((name.to_string(), bus.map(ToString::to_string), gain))
}

fn send_damage_rule_audio_with_source(
    ctx: &RegionCtx,
    entity_id: u32,
    attacker_id: u32,
    kind: &str,
    source_item_id: Option<u32>,
    key_prefix: &str,
) {
    if !is_player_message_recipient(ctx, entity_id) {
        return;
    }

    let item_override = source_item_id
        .and_then(|item_id| item_by_id(ctx, item_id))
        .and_then(|item| item_audio_override(item, key_prefix))
        .or_else(|| {
            equipped_audio_item(ctx, attacker_id)
                .and_then(|item| item_audio_override(item, key_prefix))
        });

    let (name, bus, gain) = if let Some((name, bus, gain)) = item_override {
        (
            name,
            bus.unwrap_or_else(|| "sfx".to_string()),
            gain.unwrap_or(1.0),
        )
    } else {
        let Some(name) = combat_audio_string(ctx, kind, &format!("{}_fx", key_prefix)) else {
            return;
        };
        let bus = combat_audio_string(ctx, kind, &format!("{}_bus", key_prefix))
            .unwrap_or_else(|| "sfx".to_string());
        let gain = combat_audio_gain(ctx, kind, &format!("{}_gain", key_prefix));
        (name, bus, gain)
    };

    if name.trim().is_empty() {
        return;
    }

    let cmd = RegionMessage::AudioCmd(
        ctx.region_id,
        AudioCommand::Play {
            name,
            bus,
            gain,
            looping: false,
        },
    );
    let _ = ctx.from_sender.get().unwrap().send(cmd);
}

fn send_damage_rule_messages(
    ctx: &mut RegionCtx,
    from_id: u32,
    target_id: u32,
    amount: i32,
    kind: &str,
    source_item_id: Option<u32>,
    attacker_name: &str,
    defender_name: &str,
) {
    if is_player_message_recipient(ctx, target_id)
        && let Some(template) = combat_message_template(ctx, "incoming")
    {
        let category = combat_message_category(ctx, "incoming_category");
        let message = render_damage_message(
            &template,
            attacker_name,
            defender_name,
            amount,
            kind,
            from_id,
            target_id,
        );
        if !message.trim().is_empty() {
            send_message_from(ctx, from_id, target_id, message, &category);
        }

        let under_attack_key = "__under_attack_by";
        let previous_attacker = ctx
            .entity_state_data
            .get(&target_id)
            .and_then(|state| state.get(under_attack_key))
            .and_then(|value| match value {
                Value::UInt(v) => Some(*v),
                Value::Int(v) => Some(*v as u32),
                Value::Int64(v) => Some(*v as u32),
                _ => None,
            });
        let target_still_alive = ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == target_id)
            .map(|entity| entity.get_attr_string("mode").unwrap_or_default() != "dead")
            .unwrap_or(false);
        if target_still_alive && previous_attacker != Some(from_id) {
            let state = ctx.entity_state_data.entry(target_id).or_default();
            state.set(under_attack_key, Value::UInt(from_id));
            send_message_from(
                ctx,
                from_id,
                target_id,
                format!("You are under attack by {}!", attacker_name),
                "warning",
            );
        }
    }

    if is_player_message_recipient(ctx, from_id)
        && let Some(template) = combat_message_template(ctx, "outgoing")
    {
        let category = combat_message_category(ctx, "outgoing_category");
        let message = render_damage_message(
            &template,
            attacker_name,
            defender_name,
            amount,
            kind,
            from_id,
            target_id,
        );
        if !message.trim().is_empty() {
            send_message_from(ctx, from_id, from_id, message, &category);
        }
    }

    send_damage_rule_audio_with_source(ctx, target_id, from_id, kind, source_item_id, "incoming");
    send_damage_rule_audio_with_source(ctx, from_id, from_id, kind, source_item_id, "outgoing");
}

fn configured_slot_names(ctx: &RegionCtx, key: &str) -> Vec<String> {
    ctx.config
        .get("game")
        .and_then(toml::Value::as_table)
        .and_then(|game| game.get(key))
        .and_then(toml::Value::as_array)
        .map(|slots| {
            slots
                .iter()
                .filter_map(toml::Value::as_str)
                .map(|slot| slot.trim().to_ascii_lowercase())
                .filter(|slot| !slot.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn is_weapon_slot(ctx: &RegionCtx, slot: &str) -> bool {
    let normalized = slot.trim().to_ascii_lowercase();
    let configured = configured_slot_names(ctx, "weapon_slots");
    if !configured.is_empty() {
        return configured
            .iter()
            .any(|configured| configured == &normalized);
    }

    matches!(
        normalized.as_str(),
        "main_hand" | "mainhand" | "weapon" | "hand_main" | "off_hand" | "offhand" | "hand_off"
    )
}

fn is_gear_slot(ctx: &RegionCtx, slot: &str) -> bool {
    let normalized = slot.trim().to_ascii_lowercase();
    let configured = configured_slot_names(ctx, "gear_slots");
    if !configured.is_empty() {
        return configured
            .iter()
            .any(|configured| configured == &normalized);
    }

    !is_weapon_slot(ctx, slot)
}

fn equipped_attr(ctx: &RegionCtx, entity: &Entity, attr: &str) -> f32 {
    entity
        .equipped
        .iter()
        .filter(|(slot, _)| is_weapon_slot(ctx, slot))
        .map(|(_, item)| item.attributes.get_float_default(attr, 0.0))
        .sum()
}

fn all_equipped_attr(entity: &Entity, attr: &str) -> f32 {
    entity
        .equipped
        .values()
        .map(|item| item.attributes.get_float_default(attr, 0.0))
        .sum()
}

fn armor_equipped_attr(ctx: &RegionCtx, entity: &Entity, attr: &str) -> f32 {
    entity
        .equipped
        .iter()
        .filter(|(slot, _)| is_gear_slot(ctx, slot))
        .map(|(_, item)| item.attributes.get_float_default(attr, 0.0))
        .sum()
}

fn resolve_combat_var(
    ctx: &RegionCtx,
    name: &str,
    value: f32,
    attacker: Option<&Entity>,
    defender: Option<&Entity>,
    source_item: Option<&Item>,
) -> f32 {
    if name == "value" {
        return value;
    }
    if let Some(attr) = name.strip_prefix("attacker.source.") {
        return source_item.map_or(0.0, |item| item.attributes.get_float_default(attr, 0.0));
    }
    if let Some(attr) = name.strip_prefix("source.") {
        return source_item.map_or(0.0, |item| item.attributes.get_float_default(attr, 0.0));
    }
    if let Some(attr) = name.strip_prefix("attacker.equipped.") {
        return attacker.map_or(0.0, |entity| all_equipped_attr(entity, attr));
    }
    if let Some(attr) = name.strip_prefix("defender.equipped.") {
        return defender.map_or(0.0, |entity| all_equipped_attr(entity, attr));
    }
    if let Some(attr) = name.strip_prefix("equipped.") {
        return attacker.map_or(0.0, |entity| all_equipped_attr(entity, attr));
    }
    if let Some(attr) = name.strip_prefix("attacker.armor.") {
        return attacker.map_or(0.0, |entity| armor_equipped_attr(ctx, entity, attr));
    }
    if let Some(attr) = name.strip_prefix("defender.armor.") {
        return defender.map_or(0.0, |entity| armor_equipped_attr(ctx, entity, attr));
    }
    if let Some(attr) = name.strip_prefix("armor.") {
        return defender.map_or(0.0, |entity| armor_equipped_attr(ctx, entity, attr));
    }
    if let Some(attr) = name.strip_prefix("attacker.weapon.") {
        return attacker.map_or(0.0, |entity| equipped_attr(ctx, entity, attr));
    }
    if let Some(attr) = name.strip_prefix("defender.weapon.") {
        return defender.map_or(0.0, |entity| equipped_attr(ctx, entity, attr));
    }
    if let Some(attr) = name.strip_prefix("weapon.") {
        return attacker.map_or(0.0, |entity| equipped_attr(ctx, entity, attr));
    }
    if let Some(attr) = name.strip_prefix("attacker.") {
        return attacker.map_or(0.0, |entity| {
            let default = if attr == ctx.level_attr { 1.0 } else { 0.0 };
            entity.attributes.get_float_default(attr, default)
        });
    }
    if let Some(attr) = name.strip_prefix("defender.") {
        return defender.map_or(0.0, |entity| {
            let default = if attr == ctx.level_attr { 1.0 } else { 0.0 };
            entity.attributes.get_float_default(attr, default)
        });
    }
    0.0
}

fn progression_stat_table<'a>(ctx: &'a RegionCtx, stat: &str) -> Option<&'a toml::value::Table> {
    ctx.rules
        .get("progression")
        .and_then(toml::Value::as_table)
        .and_then(|progression| progression.get(stat))
        .and_then(toml::Value::as_table)
}

fn entity_rule_identity(entity: &Entity, key: &str) -> Option<String> {
    entity
        .get_attr_string(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn race_rule_root<'a>(ctx: &'a RegionCtx, entity: &Entity) -> Option<&'a toml::value::Table> {
    let race = entity_rule_identity(entity, "race")?;
    ctx.rules
        .get("races")
        .and_then(toml::Value::as_table)
        .and_then(|races| races.get(&race))
        .and_then(toml::Value::as_table)
}

fn class_rule_root<'a>(ctx: &'a RegionCtx, entity: &Entity) -> Option<&'a toml::value::Table> {
    let class = entity_rule_identity(entity, "class")?;
    ctx.rules
        .get("classes")
        .and_then(toml::Value::as_table)
        .and_then(|classes| classes.get(&class))
        .and_then(toml::Value::as_table)
}

fn progression_stat_table_from_root<'a>(
    root: &'a toml::value::Table,
    stat: &str,
) -> Option<&'a toml::value::Table> {
    root.get("progression")
        .and_then(toml::Value::as_table)
        .and_then(|progression| progression.get(stat))
        .and_then(toml::Value::as_table)
}

fn progression_stat_tables<'a>(
    ctx: &'a RegionCtx,
    entity: &Entity,
    stat: &str,
) -> Vec<&'a toml::value::Table> {
    let mut tables = Vec::new();
    if let Some(table) = progression_stat_table(ctx, stat) {
        tables.push(table);
    }
    if let Some(root) = race_rule_root(ctx, entity)
        && let Some(table) = progression_stat_table_from_root(root, stat)
    {
        tables.push(table);
    }
    if let Some(root) = class_rule_root(ctx, entity)
        && let Some(table) = progression_stat_table_from_root(root, stat)
    {
        tables.push(table);
    }
    tables
}

fn progression_level_for_entity(ctx: &RegionCtx, entity: &Entity) -> f32 {
    entity
        .attributes
        .get_float_default(&ctx.level_attr, 1.0)
        .round()
        .max(1.0)
}

fn resolve_progression_var(ctx: &RegionCtx, entity: &Entity, name: &str) -> f32 {
    if name == "level" {
        return progression_level_for_entity(ctx, entity);
    }

    entity.attributes.get_float_default(name, 0.0)
}

fn progression_number(value: Option<&toml::Value>, default: f32) -> f32 {
    match value {
        Some(toml::Value::Integer(value)) => *value as f32,
        Some(toml::Value::Float(value)) => *value as f32,
        _ => default,
    }
}

pub(crate) fn progression_xp_for_level(ctx: &RegionCtx, entity_id: u32, level: u32) -> Option<f32> {
    let entity = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == entity_id)?;
    let tables = progression_stat_tables(ctx, entity, "level");
    let expr = tables
        .iter()
        .rev()
        .find_map(|table| table.get("xp_for_level").and_then(toml::Value::as_str))?;
    FormulaParser::new(expr, |name| {
        if name == "level" {
            level as f32
        } else {
            resolve_progression_var(ctx, entity, name)
        }
    })
    .parse()
    .filter(|value| value.is_finite())
    .map(|value| value.max(0.0))
}

pub(crate) fn progression_stat_value(ctx: &RegionCtx, entity_id: u32, stat: &str) -> Option<f32> {
    let entity = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == entity_id)?;
    let tables = progression_stat_tables(ctx, entity, stat);
    if tables.is_empty() {
        return None;
    }
    let base = tables
        .iter()
        .map(|table| progression_number(table.get("base"), 0.0))
        .sum::<f32>();
    let per_level = tables
        .iter()
        .map(|table| progression_number(table.get("per_level"), 0.0))
        .sum::<f32>();
    let level = progression_level_for_entity(ctx, entity);
    let levels_gained = (level - 1.0).max(0.0);
    let gain = tables
        .iter()
        .map(|table| {
            table
                .get("gain")
                .and_then(toml::Value::as_str)
                .and_then(|expr| {
                    FormulaParser::new(expr, |name| resolve_progression_var(ctx, entity, name))
                        .parse()
                })
                .unwrap_or(0.0)
        })
        .sum::<f32>();

    Some((base + levels_gained * (per_level + gain)).max(0.0))
}

fn item_numeric_attr(item: &Item, attr: &str) -> f32 {
    item.attributes.get_float_default(attr, 0.0)
}

fn configured_weapon_slots(ctx: &RegionCtx) -> Vec<String> {
    ctx.config
        .get("game")
        .and_then(toml::Value::as_table)
        .and_then(|game| game.get("weapon_slots"))
        .and_then(toml::Value::as_array)
        .map(|slots| {
            slots
                .iter()
                .filter_map(toml::Value::as_str)
                .map(|slot| slot.trim().to_ascii_lowercase())
                .filter(|slot| !slot.is_empty())
                .collect()
        })
        .unwrap_or_else(|| vec!["main_hand".into(), "off_hand".into()])
}

fn current_attack_source_item_id_for_entity(ctx: &RegionCtx, entity_id: u32) -> Option<u32> {
    let entity = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == entity_id)?;

    let configured_slots = configured_weapon_slots(ctx);
    for slot in &configured_slots {
        if let Some((_, item)) = entity
            .equipped
            .iter()
            .find(|(equipped_slot, _)| equipped_slot.trim().eq_ignore_ascii_case(slot))
        {
            return Some(item.id);
        }
    }

    entity
        .equipped
        .iter()
        .find(|(slot, _)| {
            matches!(
                slot.trim().to_ascii_lowercase().as_str(),
                "main_hand"
                    | "mainhand"
                    | "weapon"
                    | "hand_main"
                    | "off_hand"
                    | "offhand"
                    | "hand_off"
            )
        })
        .map(|(_, item)| item.id)
}

fn current_attack_base_damage_for_entity(ctx: &RegionCtx, entity_id: u32) -> i32 {
    progression_stat_value(ctx, entity_id, "damage")
        .or_else(|| {
            ctx.map
                .entities
                .iter()
                .find(|entity| entity.id == entity_id)
                .map(|entity| entity.attributes.get_float_default("DMG", 1.0))
        })
        .unwrap_or(1.0)
        .round()
        .max(0.0) as i32
}

fn current_attack_kind_for_entity(
    ctx: &RegionCtx,
    entity_id: u32,
    source_item_id: Option<u32>,
) -> String {
    let attacker = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == entity_id);

    if let Some(kind) = source_item_id
        .and_then(|item_id| attacker.and_then(|entity| entity.get_item(item_id)))
        .and_then(|item| item.attributes.get_str("damage_kind"))
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
    {
        return kind.to_string();
    }

    "physical".to_string()
}

fn queue_entity_attack_damage(ctx: &mut RegionCtx, attacker_id: u32, target_id: u32) {
    let source_item_id = current_attack_source_item_id_for_entity(ctx, attacker_id);
    let kind = current_attack_kind_for_entity(ctx, attacker_id, source_item_id);
    let base_dmg = current_attack_base_damage_for_entity(ctx, attacker_id);
    let dmg = apply_damage_rules(
        ctx,
        target_id,
        attacker_id,
        base_dmg,
        &kind,
        source_item_id.unwrap_or(0),
    );

    if dmg > 0
        && let Some(attacker) = ctx.map.entities.iter_mut().find(|e| e.id == attacker_id)
    {
        let attack_time = attacker
            .attributes
            .get_float_default("avatar_attack_time", 0.35)
            .max(0.05);
        attacker.set_attribute("avatar_attack_left", Value::Float(attack_time));
    }

    let autodamage = ctx
        .map
        .entities
        .iter()
        .find(|e| e.id == target_id)
        .map(|e| e.attributes.get_bool_default("autodamage", false))
        .unwrap_or(false);

    if autodamage {
        _ = apply_damage_direct(ctx, target_id, attacker_id, dmg, &kind, source_item_id);
    } else {
        let source_item_id = source_item_id.unwrap_or(0) as f32;
        ctx.to_execute_entity.push((
            target_id,
            "take_damage".into(),
            VMValue::new_with_string(attacker_id as f32, dmg as f32, source_item_id, &kind),
        ));
    }
}

fn entity_numeric_attr(ctx: &RegionCtx, entity: &Entity, attr: &str) -> f32 {
    let default = if attr == ctx.level_attr { 1.0 } else { 0.0 };
    entity.attributes.get_float_default(attr, default)
}

fn resolve_intent_rule_var(
    ctx: &RegionCtx,
    name: &str,
    distance: f32,
    subject: Option<&Entity>,
    target_entity: Option<&Entity>,
    target_item: Option<&Item>,
) -> f32 {
    if name == "distance" {
        return distance;
    }
    if let Some(attr) = name.strip_prefix("subject.") {
        return subject.map_or(0.0, |entity| entity_numeric_attr(ctx, entity, attr));
    }
    if let Some(attr) = name.strip_prefix("actor.") {
        return subject.map_or(0.0, |entity| entity_numeric_attr(ctx, entity, attr));
    }
    if let Some(attr) = name.strip_prefix("target.") {
        if let Some(entity) = target_entity {
            return entity_numeric_attr(ctx, entity, attr);
        }
        if let Some(item) = target_item {
            return item_numeric_attr(item, attr);
        }
    }
    0.0
}

fn evaluate_intent_allowed(
    ctx: &RegionCtx,
    expr: &str,
    distance: f32,
    subject: Option<&Entity>,
    target_entity: Option<&Entity>,
    target_item: Option<&Item>,
) -> bool {
    FormulaParser::new(expr, |name| {
        resolve_intent_rule_var(ctx, name, distance, subject, target_entity, target_item)
    })
    .parse()
    .filter(|value| value.is_finite())
    .map(|value| value != 0.0)
    .unwrap_or(false)
}

struct FormulaParser<'a, F>
where
    F: Fn(&str) -> f32,
{
    src: &'a [u8],
    idx: usize,
    resolve: F,
}

impl<'a, F> FormulaParser<'a, F>
where
    F: Fn(&str) -> f32,
{
    fn new(src: &'a str, resolve: F) -> Self {
        Self {
            src: src.as_bytes(),
            idx: 0,
            resolve,
        }
    }

    fn parse(mut self) -> Option<f32> {
        let value = self.parse_or()?;
        self.skip_ws();
        if self.idx == self.src.len() {
            Some(value)
        } else {
            None
        }
    }

    fn skip_ws(&mut self) {
        while self.idx < self.src.len() && self.src[self.idx].is_ascii_whitespace() {
            self.idx += 1;
        }
    }

    fn consume(&mut self, ch: u8) -> bool {
        self.skip_ws();
        if self.idx < self.src.len() && self.src[self.idx] == ch {
            self.idx += 1;
            true
        } else {
            false
        }
    }

    fn parse_or(&mut self) -> Option<f32> {
        let mut value = self.parse_and()?;
        loop {
            self.skip_ws();
            if self.idx + 1 < self.src.len()
                && self.src[self.idx] == b'|'
                && self.src[self.idx + 1] == b'|'
            {
                self.idx += 2;
                let rhs = self.parse_and()?;
                value = if value != 0.0 || rhs != 0.0 { 1.0 } else { 0.0 };
            } else {
                break;
            }
        }
        Some(value)
    }

    fn parse_and(&mut self) -> Option<f32> {
        let mut value = self.parse_comparison()?;
        loop {
            self.skip_ws();
            if self.idx + 1 < self.src.len()
                && self.src[self.idx] == b'&'
                && self.src[self.idx + 1] == b'&'
            {
                self.idx += 2;
                let rhs = self.parse_comparison()?;
                value = if value != 0.0 && rhs != 0.0 { 1.0 } else { 0.0 };
            } else {
                break;
            }
        }
        Some(value)
    }

    fn parse_comparison(&mut self) -> Option<f32> {
        let mut value = self.parse_expr()?;
        loop {
            self.skip_ws();
            let next = if self.idx + 1 < self.src.len() {
                Some((self.src[self.idx], self.src[self.idx + 1]))
            } else {
                None
            };
            let result = match next {
                Some((b'=', b'=')) => {
                    self.idx += 2;
                    let rhs = self.parse_expr()?;
                    Some(if (value - rhs).abs() <= f32::EPSILON {
                        1.0
                    } else {
                        0.0
                    })
                }
                Some((b'!', b'=')) => {
                    self.idx += 2;
                    let rhs = self.parse_expr()?;
                    Some(if (value - rhs).abs() > f32::EPSILON {
                        1.0
                    } else {
                        0.0
                    })
                }
                Some((b'<', b'=')) => {
                    self.idx += 2;
                    let rhs = self.parse_expr()?;
                    Some(if value <= rhs { 1.0 } else { 0.0 })
                }
                Some((b'>', b'=')) => {
                    self.idx += 2;
                    let rhs = self.parse_expr()?;
                    Some(if value >= rhs { 1.0 } else { 0.0 })
                }
                _ if self.consume(b'<') => {
                    let rhs = self.parse_expr()?;
                    Some(if value < rhs { 1.0 } else { 0.0 })
                }
                _ if self.consume(b'>') => {
                    let rhs = self.parse_expr()?;
                    Some(if value > rhs { 1.0 } else { 0.0 })
                }
                _ => None,
            };

            if let Some(result) = result {
                value = result;
            } else {
                break;
            }
        }
        Some(value)
    }

    fn parse_expr(&mut self) -> Option<f32> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_ws();
            if self.consume(b'+') {
                value += self.parse_term()?;
            } else if self.consume(b'-') {
                value -= self.parse_term()?;
            } else {
                break;
            }
        }
        Some(value)
    }

    fn parse_term(&mut self) -> Option<f32> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_ws();
            if self.consume(b'*') {
                value *= self.parse_factor()?;
            } else if self.consume(b'/') {
                let rhs = self.parse_factor()?;
                if rhs.abs() <= f32::EPSILON {
                    return None;
                }
                value /= rhs;
            } else {
                break;
            }
        }
        Some(value)
    }

    fn parse_factor(&mut self) -> Option<f32> {
        self.skip_ws();
        if self.consume(b'+') {
            return self.parse_factor();
        }
        if self.consume(b'-') {
            return self.parse_factor().map(|v| -v);
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<f32> {
        self.skip_ws();
        if self.consume(b'(') {
            let value = self.parse_or()?;
            if !self.consume(b')') {
                return None;
            }
            return Some(value);
        }
        if self.idx >= self.src.len() {
            return None;
        }
        let ch = self.src[self.idx];
        if ch.is_ascii_digit() || ch == b'.' {
            return self.parse_number();
        }
        if ch.is_ascii_alphabetic() || ch == b'_' {
            let ident = self.parse_identifier()?;
            self.skip_ws();
            if self.consume(b'(') {
                let value = self.parse_call(&ident)?;
                if !self.consume(b')') {
                    return None;
                }
                return Some(value);
            }
            return Some((self.resolve)(&ident));
        }
        None
    }

    fn parse_identifier(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.idx;
        while self.idx < self.src.len() {
            let ch = self.src[self.idx];
            if ch.is_ascii_alphanumeric() || matches!(ch, b'_' | b'.') {
                self.idx += 1;
            } else {
                break;
            }
        }
        if self.idx == start {
            None
        } else {
            std::str::from_utf8(&self.src[start..self.idx])
                .ok()
                .map(ToString::to_string)
        }
    }

    fn parse_number(&mut self) -> Option<f32> {
        self.skip_ws();
        let start = self.idx;
        let mut seen_dot = false;
        while self.idx < self.src.len() {
            let ch = self.src[self.idx];
            if ch.is_ascii_digit() {
                self.idx += 1;
            } else if ch == b'.' && !seen_dot {
                seen_dot = true;
                self.idx += 1;
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.src[start..self.idx])
            .ok()?
            .parse::<f32>()
            .ok()
    }

    fn parse_args(&mut self) -> Option<Vec<f32>> {
        let mut args = Vec::new();
        self.skip_ws();
        if self.idx < self.src.len() && self.src[self.idx] == b')' {
            return Some(args);
        }
        loop {
            args.push(self.parse_expr()?);
            self.skip_ws();
            if self.consume(b',') {
                continue;
            }
            break;
        }
        Some(args)
    }

    fn parse_call(&mut self, ident: &str) -> Option<f32> {
        let args = self.parse_args()?;
        match ident {
            "min" if args.len() == 2 => Some(args[0].min(args[1])),
            "max" if args.len() == 2 => Some(args[0].max(args[1])),
            "clamp" if args.len() == 3 => Some(args[0].clamp(args[1], args[2])),
            "abs" if args.len() == 1 => Some(args[0].abs()),
            "floor" if args.len() == 1 => Some(args[0].floor()),
            "ceil" if args.len() == 1 => Some(args[0].ceil()),
            "round" if args.len() == 1 => Some(args[0].round()),
            _ => None,
        }
    }
}

fn evaluate_damage_rule(
    ctx: &RegionCtx,
    target_id: u32,
    from_id: u32,
    amount: i32,
    kind: &str,
    source_item_id: u32,
    key: &str,
) -> Option<i32> {
    let attacker = ctx.map.entities.iter().find(|entity| entity.id == from_id);
    let exprs = combat_rule_exprs(ctx, attacker, kind, key);
    if exprs.is_empty() {
        return None;
    }
    let defender = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == target_id);
    let source_item = attacker.and_then(|entity| {
        if source_item_id > 0 {
            entity.get_item(source_item_id)
        } else {
            None
        }
    });
    let mut current_value = amount as f32;

    for expr in exprs {
        let parsed = FormulaParser::new(expr, |name| {
            resolve_combat_var(ctx, name, current_value, attacker, defender, source_item)
        })
        .parse()?;
        if !parsed.is_finite() {
            return None;
        }
        current_value = parsed.max(0.0);
    }

    Some(current_value.round().max(0.0) as i32)
}

pub(crate) fn apply_damage_rules(
    ctx: &RegionCtx,
    target_id: u32,
    from_id: u32,
    amount: i32,
    kind: &str,
    source_item_id: u32,
) -> i32 {
    let amount = amount.max(0);
    let outgoing = evaluate_damage_rule(
        ctx,
        target_id,
        from_id,
        amount,
        kind,
        source_item_id,
        "outgoing_damage",
    )
    .unwrap_or(amount);
    evaluate_damage_rule(
        ctx,
        target_id,
        from_id,
        outgoing,
        kind,
        source_item_id,
        "incoming_damage",
    )
    .unwrap_or(outgoing)
    .max(0)
}

pub(crate) fn drop_all_items_for_entity(ctx: &mut RegionCtx, entity_id: u32) {
    let drop_position = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == entity_id)
        .map(|entity| entity.get_pos_xz());

    let removed_items = if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
        let mut removed_items = Vec::new();

        let slots: Vec<usize> = entity
            .inventory
            .iter()
            .enumerate()
            .filter_map(|(slot, item)| item.as_ref().map(|_| slot))
            .collect();

        for slot in slots {
            if let Some(mut item) = entity.remove_item_from_slot(slot) {
                item.position = entity.position;
                item.mark_all_dirty();
                removed_items.push(item);
            }
        }

        let equipped_slots: Vec<String> = entity.equipped.keys().cloned().collect();
        for slot in equipped_slots {
            if let Ok(mut item) = entity.unequip_item(&slot) {
                item.position = entity.position;
                item.mark_all_dirty();
                removed_items.push(item);
            }
        }

        removed_items
    } else {
        Vec::new()
    };

    if !removed_items.is_empty() {
        let count = removed_items.len();
        ctx.map.items.extend(removed_items);
        if let Some(drop_position) = drop_position {
            ctx.send_item_drop_message_for_position(drop_position, count);
        }
    }
}

fn update_spell_items(ctx: &mut RegionCtx) {
    let dt = ctx.delta_time.max(0.0);
    if dt <= 0.0 || ctx.map.items.is_empty() {
        return;
    }

    let target_fps = get_config_i32_default(ctx, "game", "target_fps", 30).max(1) as f32;
    let default_effect_frame_time = 1.0 / target_fps;
    let mut tile_frame_counts: FxHashMap<Uuid, usize> = FxHashMap::default();
    for (tile_id, tile) in &ctx.assets.tiles {
        tile_frame_counts.insert(*tile_id, tile.textures.len().max(1));
    }

    let mut entity_pos: FxHashMap<u32, Vec2<f32>> = FxHashMap::default();
    let mut entity_dead: FxHashMap<u32, bool> = FxHashMap::default();
    let mut entity_alignment: FxHashMap<u32, i32> = FxHashMap::default();
    let mut entity_orientation: FxHashMap<u32, Vec2<f32>> = FxHashMap::default();
    let mut entity_attrs: FxHashMap<u32, ValueContainer> = FxHashMap::default();
    for entity in &ctx.map.entities {
        entity_pos.insert(entity.id, entity.get_pos_xz());
        entity_dead.insert(
            entity.id,
            entity.attributes.get_str_default("mode", "active".into()) == "dead",
        );
        entity_alignment.insert(entity.id, entity.attributes.get_int_default("ALIGNMENT", 0));
        entity_orientation.insert(entity.id, entity.orientation);
        entity_attrs.insert(entity.id, entity.attributes.clone());
    }

    let mut despawn_item_ids: Vec<u32> = Vec::new();
    let mut casting_casters: FxHashSet<u32> = FxHashSet::default();
    let mut pending_damage: Vec<(u32, u32, i32, String, u32)> = Vec::new(); // (target_id, caster_id, amount, kind, source_item_id)
    let mut pending_heal: Vec<(u32, i32)> = Vec::new(); // (target_id, amount)
    let mut pending_item_events: Vec<(u32, String, VMValue)> = Vec::new();

    for item in &mut ctx.map.items {
        if !item.attributes.get_bool_default("is_spell", false) {
            continue;
        }

        let mode = item
            .attributes
            .get_str_default("spell_mode", "projectile".into())
            .to_ascii_lowercase();
        if mode != "projectile" {
            continue;
        }

        // Impact phase: keep the projectile alive for a short effect display
        // after a hit (same item, switched to effect_id tile/source).
        if item.attributes.get_bool_default("spell_impacting", false) {
            let impact_tile_id = item.attributes.get_id("tile_id").or_else(|| {
                item.attributes
                    .get_str("tile_id")
                    .and_then(|s| Uuid::parse_str(s).ok())
            });
            let impact_default = if item.attributes.contains("effect_duration") {
                item.attributes.get_float_default("effect_duration", 0.25)
            } else {
                let frame_time = if item.attributes.contains("effect_frame_time") {
                    item.attributes
                        .get_float_default("effect_frame_time", default_effect_frame_time)
                } else {
                    default_effect_frame_time
                }
                .max(0.01);
                let frames = impact_tile_id
                    .and_then(|id| tile_frame_counts.get(&id).copied())
                    .unwrap_or(1) as f32;
                (frames * frame_time).max(frame_time)
            }
            .max(0.0);
            let mut impact_left = item
                .attributes
                .get_float_default("spell_impact_left", impact_default);
            impact_left -= dt;
            item.set_attribute("spell_impact_left", Value::Float(impact_left));
            if impact_left <= 0.0 {
                despawn_item_ids.push(item.id);
            }
            continue;
        }

        if item.attributes.get_bool_default("spell_casting", false) {
            let caster_id = item.attributes.get_uint("spell_caster_id").unwrap_or(0);
            let cast_height = item.attributes.get_float_default("spell_cast_height", 0.5);
            let cast_offset = item
                .attributes
                .get_float_default("spell_cast_offset", 0.6)
                .max(0.0);
            let flight_height = item
                .attributes
                .get_float_default("spell_flight_height", 0.5);

            if let Some(caster_pos) = entity_pos.get(&caster_id) {
                let mut dir = entity_orientation
                    .get(&caster_id)
                    .copied()
                    .unwrap_or(Vec2::new(1.0, 0.0));
                if dir.magnitude_squared() <= 1e-6 {
                    dir = Vec2::new(1.0, 0.0);
                } else {
                    dir = dir.normalized();
                }
                item.set_attribute("spell_dir_x", Value::Float(dir.x));
                item.set_attribute("spell_dir_z", Value::Float(dir.y));
                item.set_position(Vec3::new(
                    caster_pos.x + dir.x * cast_offset,
                    cast_height,
                    caster_pos.y + dir.y * cast_offset,
                ));
            }

            let mut cast_left = item.attributes.get_float_default("spell_cast_left", 0.0);
            cast_left -= dt;
            item.set_attribute("spell_cast_left", Value::Float(cast_left));
            if cast_left > 0.0 {
                if caster_id != 0 {
                    casting_casters.insert(caster_id);
                }
                continue;
            }
            item.set_attribute("spell_casting", Value::Bool(false));
            item.set_position(Vec3::new(item.position.x, flight_height, item.position.z));
        }

        let mut lifetime_left = item.attributes.get_float_default(
            "spell_lifetime_left",
            item.attributes.get_float_default("spell_lifetime", 3.0),
        );
        lifetime_left -= dt;
        item.set_attribute("spell_lifetime_left", Value::Float(lifetime_left));
        if lifetime_left <= 0.0 {
            pending_item_events.push((item.id, "expire".into(), VMValue::zero()));
            despawn_item_ids.push(item.id);
            continue;
        }

        let speed = item
            .attributes
            .get_float_default("spell_speed", 6.0)
            .max(0.0);

        let target_id = item.attributes.get_uint("spell_target_id");
        let mut target_pos = target_id.and_then(|id| entity_pos.get(&id).copied());
        if target_pos.is_none() {
            let tx = item
                .attributes
                .get_float_default("spell_target_x", item.position.x);
            let tz = item
                .attributes
                .get_float_default("spell_target_z", item.position.z);
            target_pos = Some(Vec2::new(tx, tz));
        }

        let mut direction = Vec2::new(
            item.attributes.get_float_default("spell_dir_x", 1.0),
            item.attributes.get_float_default("spell_dir_z", 0.0),
        );
        if let Some(tp) = target_pos {
            let to_target = tp - item.get_pos_xz();
            if to_target.magnitude_squared() > 1e-6 {
                direction = to_target.normalized();
            }
        }
        if direction.magnitude_squared() <= 1e-6 {
            direction = Vec2::new(1.0, 0.0);
        } else {
            direction = direction.normalized();
        }
        item.set_attribute("spell_dir_x", Value::Float(direction.x));
        item.set_attribute("spell_dir_z", Value::Float(direction.y));

        let step = speed * dt;
        let flight_height = item
            .attributes
            .get_float_default("spell_flight_height", 0.5);
        if step > 0.0 {
            item.set_position(Vec3::new(
                item.position.x + direction.x * step,
                flight_height,
                item.position.z + direction.y * step,
            ));
        }

        let travel = item.attributes.get_float_default("spell_travel", 0.0) + step;
        item.set_attribute("spell_travel", Value::Float(travel));
        let max_range = item.attributes.get_float_default("spell_max_range", 0.0);
        if max_range > 0.0 && travel >= max_range {
            pending_item_events.push((item.id, "expire".into(), VMValue::zero()));
            despawn_item_ids.push(item.id);
            continue;
        }

        let caster_id = item.attributes.get_uint("spell_caster_id").unwrap_or(0);
        let filter = item
            .attributes
            .get_str_default("spell_target_filter", "any".into());
        let hit_radius = item
            .attributes
            .get_float_default("spell_radius", 0.4)
            .max(0.05);

        let mut hit_target: Option<u32> = None;
        if let Some(tid) = target_id {
            if !entity_dead.get(&tid).copied().unwrap_or(true)
                && spell_target_filter_allows(
                    &filter,
                    caster_id,
                    tid,
                    &entity_attrs,
                    &entity_alignment,
                )
                && let Some(tp) = entity_pos.get(&tid)
                && tp.distance(item.get_pos_xz()) <= hit_radius
            {
                hit_target = Some(tid);
            }
        } else {
            for (eid, pos) in &entity_pos {
                if *eid == caster_id {
                    continue;
                }
                if entity_dead.get(eid).copied().unwrap_or(true) {
                    continue;
                }
                if !spell_target_filter_allows(
                    &filter,
                    caster_id,
                    *eid,
                    &entity_attrs,
                    &entity_alignment,
                ) {
                    continue;
                }
                if pos.distance(item.get_pos_xz()) <= hit_radius {
                    hit_target = Some(*eid);
                    break;
                }
            }
        }

        if let Some(target_id) = hit_target {
            let effect = item
                .attributes
                .get_str_default("spell_effect", "damage".into())
                .to_ascii_lowercase();
            let amount = item.attributes.get_int_default("spell_amount", 1).max(0);
            let kind = item
                .attributes
                .get_str_default("spell_kind", "spell".into())
                .to_string();

            if effect == "heal" {
                pending_heal.push((target_id, amount));
            } else {
                pending_damage.push((target_id, caster_id, amount, kind, item.id));
            }

            pending_item_events.push((item.id, "hit".into(), VMValue::broadcast(target_id as f32)));
            // Optional impact visual on the same projectile item.
            // If effect_id is present and valid, switch source and hold for effect_duration.
            let effect_uuid = item.attributes.get_id("effect_id").or_else(|| {
                item.attributes
                    .get_str("effect_id")
                    .and_then(|s| Uuid::parse_str(s).ok())
            });
            if let Some(uuid) = effect_uuid {
                item.set_attribute("source", Value::Source(PixelSource::TileId(uuid)));
                item.set_attribute("tile_id", Value::Id(uuid));
                item.set_attribute("spell_impacting", Value::Bool(true));
                item.set_attribute("spell_speed", Value::Float(0.0));
                item.set_attribute("spell_dir_x", Value::Float(0.0));
                item.set_attribute("spell_dir_z", Value::Float(0.0));
                let impact_duration = if item.attributes.contains("effect_duration") {
                    item.attributes.get_float_default("effect_duration", 0.25)
                } else {
                    let frame_time = if item.attributes.contains("effect_frame_time") {
                        item.attributes
                            .get_float_default("effect_frame_time", default_effect_frame_time)
                    } else {
                        default_effect_frame_time
                    }
                    .max(0.01);
                    let frames = tile_frame_counts.get(&uuid).copied().unwrap_or(1) as f32;
                    (frames * frame_time).max(frame_time)
                }
                .max(0.0);
                item.set_attribute("spell_impact_left", Value::Float(impact_duration));
                let impact_height = item
                    .attributes
                    .get_float_default("effect_height", item.position.y);
                item.set_position(Vec3::new(item.position.x, impact_height, item.position.z));
                item.mark_dirty_attribute("source");
            } else {
                despawn_item_ids.push(item.id);
            }
        }
    }

    if !pending_heal.is_empty() {
        let health_attr = ctx.health_attr.clone();
        for (target_id, amount) in pending_heal {
            if amount <= 0 {
                continue;
            }
            if let Some(entity) = ctx.map.entities.iter_mut().find(|e| e.id == target_id) {
                let hp = entity.attributes.get_int_default(&health_attr, 0);
                let max_hp = entity.attributes.get_int_default("max_health", hp.max(1));
                entity.set_attribute(&health_attr, Value::Int((hp + amount).min(max_hp)));
            }
        }
    }

    for entity in &mut ctx.map.entities {
        let is_casting = casting_casters.contains(&entity.id);
        let was_casting = entity.attributes.get_bool_default("spell_casting", false);
        if is_casting != was_casting {
            entity.set_attribute("spell_casting", Value::Bool(is_casting));
        }
    }

    for (target_id, caster_id, amount, kind, source_item_id) in pending_damage {
        if amount <= 0 {
            continue;
        }
        let final_amount =
            apply_damage_rules(ctx, target_id, caster_id, amount, &kind, source_item_id);
        if final_amount <= 0 {
            continue;
        }
        let autodamage = ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == target_id)
            .map(|e| e.attributes.get_bool_default("autodamage", false))
            .unwrap_or(false);

        if autodamage {
            _ = apply_damage_direct(
                ctx,
                target_id,
                caster_id,
                final_amount,
                &kind,
                Some(source_item_id),
            );
        } else {
            ctx.to_execute_entity.push((
                target_id,
                "take_damage".into(),
                VMValue::new_with_string(
                    caster_id as f32,
                    final_amount as f32,
                    source_item_id as f32,
                    kind,
                ),
            ));
        }
    }

    ctx.to_execute_item.extend(pending_item_events);

    if !despawn_item_ids.is_empty() {
        ctx.map
            .items
            .retain(|item| !despawn_item_ids.iter().any(|id| *id == item.id));
        for item_id in despawn_item_ids {
            ctx.item_classes.remove(&item_id);
            ctx.item_state_data.remove(&item_id);
            let _ = ctx
                .from_sender
                .get()
                .unwrap()
                .send(RegionMessage::RemoveItem(ctx.region_id, item_id));
        }
    }
}

fn drop_item_for_entity(ctx: &mut RegionCtx, entity_id: u32, item_id: u32) -> bool {
    drop_item_for_entity_at(ctx, entity_id, item_id, None)
}

fn quantize_item_drop_position(pos: Vec2<f32>) -> Vec2<f32> {
    Vec2::new(pos.x.floor() + 0.5, pos.y.floor() + 0.5)
}

fn drop_item_for_entity_at(
    ctx: &mut RegionCtx,
    entity_id: u32,
    item_id: u32,
    drop_position_override: Option<Vec2<f32>>,
) -> bool {
    let drop_position = ctx
        .map
        .entities
        .iter()
        .find(|entity| entity.id == entity_id)
        .map(|entity| entity.get_pos_xz());
    let final_drop_position = drop_position_override
        .or(drop_position)
        .map(quantize_item_drop_position);

    if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
        // Drop from inventory.
        if let Some(slot) = entity.get_item_slot(item_id)
            && let Some(mut item) = entity.remove_item_from_slot(slot)
        {
            if item.attributes.get_bool_default("is_spell", false) {
                return false;
            }
            item.position = entity.position;
            if let Some(drop_position) = final_drop_position {
                item.position.x = drop_position.x;
                item.position.z = drop_position.y;
            }
            item.mark_all_dirty();
            ctx.map.items.push(item);
            if let Some(drop_position) = final_drop_position {
                ctx.send_item_drop_message_for_position(drop_position, 1);
            }
            return true;
        }

        // Drop from equipped slots.
        let equipped_slot = entity.equipped.iter().find_map(|(slot, item)| {
            if item.id == item_id {
                Some(slot.clone())
            } else {
                None
            }
        });
        if let Some(slot) = equipped_slot
            && let Ok(mut item) = entity.unequip_item(&slot)
        {
            if item.attributes.get_bool_default("is_spell", false) {
                return false;
            }
            item.position = entity.position;
            if let Some(drop_position) = final_drop_position {
                item.position.x = drop_position.x;
                item.position.z = drop_position.y;
            }
            item.mark_all_dirty();
            ctx.map.items.push(item);
            if let Some(drop_position) = final_drop_position {
                ctx.send_item_drop_message_for_position(drop_position, 1);
            }
            return true;
        }
    }
    false
}

fn move_item_for_entity(
    ctx: &mut RegionCtx,
    source_entity_id: u32,
    target_entity_id: u32,
    item_id: u32,
    to_inventory_index: Option<usize>,
    to_equipped_slot: Option<String>,
) -> bool {
    #[derive(Clone)]
    enum Source {
        Inventory(usize),
        Equipped(String),
        World(usize),
    }

    fn entity_pair_mut(
        entities: &mut [Entity],
        source_index: usize,
        target_index: usize,
    ) -> (&mut Entity, Option<&mut Entity>) {
        if source_index == target_index {
            (&mut entities[source_index], None)
        } else if source_index < target_index {
            let (left, right) = entities.split_at_mut(target_index);
            (&mut left[source_index], Some(&mut right[0]))
        } else {
            let (left, right) = entities.split_at_mut(source_index);
            (&mut right[0], Some(&mut left[target_index]))
        }
    }

    let source_entity_index = ctx
        .map
        .entities
        .iter()
        .position(|entity| entity.id == source_entity_id);
    let Some(target_entity_index) = ctx
        .map
        .entities
        .iter()
        .position(|entity| entity.id == target_entity_id)
    else {
        return false;
    };

    let source =
        if let Some(source_entity_index) = source_entity_index
            && let Some(slot) = ctx.map.entities[source_entity_index].get_item_slot(item_id)
        {
            Source::Inventory(slot)
        } else if let Some(source_entity_index) = source_entity_index
            && let Some(slot) = ctx.map.entities[source_entity_index]
                .equipped
                .iter()
                .find_map(|(slot, item)| {
                    if item.id == item_id {
                        Some(slot.clone())
                    } else {
                        None
                    }
                })
        {
            Source::Equipped(slot)
        } else if let Some(index) = ctx.map.items.iter().position(|item| {
            item.id == item_id && !item.attributes.get_bool_default("static", false)
        }) {
            Source::World(index)
        } else {
            return false;
        };
    let from_world = matches!(source, Source::World(_));

    let moving_item_slot = match (&source, source_entity_index) {
        (Source::Inventory(source_index), Some(source_entity_index)) => ctx.map.entities
            [source_entity_index]
            .inventory
            .get(*source_index)
            .and_then(|item| item.as_ref())
            .and_then(|item| item.attributes.get_str("slot"))
            .map(|slot| slot.trim().to_ascii_lowercase()),
        (Source::Equipped(source_slot), Some(source_entity_index)) => ctx.map.entities
            [source_entity_index]
            .equipped
            .get(source_slot)
            .and_then(|item| item.attributes.get_str("slot"))
            .map(|slot| slot.trim().to_ascii_lowercase()),
        (Source::World(source_index), _) => ctx
            .map
            .items
            .get(*source_index)
            .and_then(|item| item.attributes.get_str("slot"))
            .map(|slot| slot.trim().to_ascii_lowercase()),
        _ => return false,
    };

    let moving_is_spell = match (&source, source_entity_index) {
        (Source::Inventory(source_index), Some(source_entity_index)) => ctx.map.entities
            [source_entity_index]
            .inventory
            .get(*source_index)
            .and_then(|item| item.as_ref())
            .map(|item| item.attributes.get_bool_default("is_spell", false))
            .unwrap_or(false),
        (Source::Equipped(source_slot), Some(source_entity_index)) => ctx.map.entities
            [source_entity_index]
            .equipped
            .get(source_slot)
            .map(|item| item.attributes.get_bool_default("is_spell", false))
            .unwrap_or(false),
        (Source::World(source_index), _) => ctx
            .map
            .items
            .get(*source_index)
            .map(|item| item.attributes.get_bool_default("is_spell", false))
            .unwrap_or(false),
        _ => return false,
    };
    if moving_is_spell {
        return false;
    }

    if let Some(target_index) = to_inventory_index {
        let target_entity = &ctx.map.entities[target_entity_index];
        if target_index >= target_entity.inventory.len() {
            return false;
        }
        if source_entity_index == Some(target_entity_index)
            && let Source::Inventory(source_index) = source
            && source_index == target_index
        {
            return true;
        }

        if from_world
            && target_entity
                .inventory
                .get(target_index)
                .and_then(|item| item.as_ref())
                .is_some()
            && !target_entity
                .inventory
                .iter()
                .enumerate()
                .any(|(index, item)| index != target_index && item.is_none())
        {
            return false;
        }

        let moving = match &source {
            Source::Inventory(source_index) => source_entity_index
                .and_then(|index| ctx.map.entities[index].remove_item_from_slot(*source_index)),
            Source::Equipped(source_slot) => source_entity_index
                .and_then(|index| ctx.map.entities[index].unequip_item(source_slot).ok()),
            Source::World(source_index) => Some(ctx.map.items.remove(*source_index)),
        };
        let Some(moving) = moving else {
            return false;
        };

        let (source_entity, maybe_target_entity) = entity_pair_mut(
            &mut ctx.map.entities,
            source_entity_index.unwrap_or(target_entity_index),
            target_entity_index,
        );
        let target_entity = maybe_target_entity.unwrap_or(source_entity);
        let displaced = target_entity.remove_item_from_slot(target_index);
        target_entity.inventory[target_index] = Some(moving.clone());
        target_entity
            .inventory_additions
            .insert(target_index, moving);
        target_entity.inventory_removals.remove(&target_index);
        target_entity.dirty_flags |= 0b1000;

        if let Some(displaced) = displaced {
            match &source {
                Source::Inventory(source_index) => {
                    source_entity.inventory[*source_index] = Some(displaced.clone());
                    source_entity
                        .inventory_additions
                        .insert(*source_index, displaced);
                    source_entity.inventory_removals.remove(source_index);
                    source_entity.dirty_flags |= 0b1000;
                }
                Source::Equipped(source_slot) => {
                    source_entity
                        .equipped
                        .insert(source_slot.clone(), displaced);
                    source_entity.dirty_flags |= 0b10000;
                }
                Source::World(_) => {
                    if target_entity.add_item(displaced).is_err() {
                        return false;
                    }
                }
            }
        }

        if from_world {
            ctx.from_sender
                .get()
                .unwrap()
                .send(RegionMessage::RemoveItem(ctx.region_id, item_id))
                .unwrap();
        }
        return true;
    }

    if let Some(target_slot) = to_equipped_slot {
        if moving_item_slot.as_deref() != Some(target_slot.trim().to_ascii_lowercase().as_str()) {
            return false;
        }

        if source_entity_index == Some(target_entity_index)
            && let Source::Equipped(source_slot) = &source
            && source_slot == &target_slot
        {
            return true;
        }

        let target_entity = &ctx.map.entities[target_entity_index];
        if from_world
            && target_entity.get_equipped_item(&target_slot).is_some()
            && !target_entity.inventory.iter().any(|item| item.is_none())
        {
            return false;
        }

        let moving = match &source {
            Source::Inventory(source_index) => source_entity_index
                .and_then(|index| ctx.map.entities[index].remove_item_from_slot(*source_index)),
            Source::Equipped(source_slot) => source_entity_index
                .and_then(|index| ctx.map.entities[index].unequip_item(source_slot).ok()),
            Source::World(source_index) => Some(ctx.map.items.remove(*source_index)),
        };
        let Some(moving) = moving else {
            return false;
        };

        let (source_entity, maybe_target_entity) = entity_pair_mut(
            &mut ctx.map.entities,
            source_entity_index.unwrap_or(target_entity_index),
            target_entity_index,
        );
        let target_entity = maybe_target_entity.unwrap_or(source_entity);
        let displaced = target_entity.unequip_item(&target_slot).ok();
        target_entity.equipped.insert(target_slot, moving);
        target_entity.dirty_flags |= 0b10000;

        if let Some(displaced) = displaced {
            match &source {
                Source::Inventory(source_index) => {
                    source_entity.inventory[*source_index] = Some(displaced.clone());
                    source_entity
                        .inventory_additions
                        .insert(*source_index, displaced);
                    source_entity.inventory_removals.remove(source_index);
                    source_entity.dirty_flags |= 0b1000;
                }
                Source::Equipped(source_slot) => {
                    source_entity
                        .equipped
                        .insert(source_slot.clone(), displaced);
                    source_entity.dirty_flags |= 0b10000;
                }
                Source::World(_) => {
                    if target_entity.add_item(displaced).is_err() {
                        return false;
                    }
                }
            }
        }

        if from_world {
            ctx.from_sender
                .get()
                .unwrap()
                .send(RegionMessage::RemoveItem(ctx.region_id, item_id))
                .unwrap();
        }
        return true;
    }

    false
}

fn take_item_for_entity(ctx: &mut RegionCtx, entity_id: u32, item_id: u32) -> bool {
    let mut rc = true;

    if let Some(pos) =
        ctx.map.items.iter().position(|item| {
            item.id == item_id && !item.attributes.get_bool_default("static", false)
        })
    {
        let item = ctx.map.items.remove(pos);
        if item.attributes.get_bool_default("is_spell", false) {
            return false;
        }

        if let Some(entity) = ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
        {
            let item_name = item
                .attributes
                .get_str("name")
                .map(str::to_string)
                .unwrap_or_else(|| "Unknown".to_string());

            fn article_for(item_name: &str) -> (&'static str, String) {
                let name = item_name.to_ascii_lowercase();

                let pair_items = ["trousers", "pants", "gloves", "boots", "scissors"];
                let mass_items = ["armor", "cloth", "water", "meat"];

                if pair_items.contains(&name.as_str()) {
                    ("a pair of", item_name.to_string())
                } else if mass_items.contains(&name.as_str()) {
                    ("some", item_name.to_string())
                } else {
                    let first = name.chars().next().unwrap_or('x');
                    let article = match first {
                        'a' | 'e' | 'i' | 'o' | 'u' => "an",
                        _ => "a",
                    };
                    (article, item_name.to_string())
                }
            }

            let mut message = format!(
                "You take {} {}",
                article_for(&item_name.to_lowercase()).0,
                item_name.to_lowercase()
            );

            if item.attributes.get_bool_default("monetary", false) {
                let amount = item.attributes.get_int_default("worth", 0);
                if amount > 0 {
                    message = format!("You take {} gold.", amount);
                    _ = entity.add_base_currency(amount as i64, &ctx.currencies);
                }
            } else if entity.add_item(item).is_err() {
                println!("Take: Too many items");
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Text("Inventory Full".into()), true);
                }
                rc = false;
            }

            if ctx.debug_mode && rc {
                add_debug_value(ctx, TheValue::Text("Ok".into()), false);
            }

            ctx.from_sender
                .get()
                .unwrap()
                .send(RegionMessage::RemoveItem(ctx.region_id, item_id))
                .unwrap();

            let msg = RegionMessage::Message(
                ctx.region_id,
                Some(entity_id),
                None,
                entity_id,
                message,
                "system".into(),
            );
            ctx.from_sender.get().unwrap().send(msg).unwrap();
        }
    } else if ctx.debug_mode {
        add_debug_value(ctx, TheValue::Text("Unknown Item".into()), true);
    }
    rc
}

/*
/// An entity took damage. Send out messages and check for death.
fn took_damage(from: u32, mut amount: i32, vm: &VirtualMachine) {
    let mut kill = false;

    // Make sure we don't heal by accident
    amount = amount.max(0);
    if amount == 0 {
        return;
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let id = ctx.curr_entity_id;

        // Check for death
        if let Some(entity) = get_entity_mut(&mut ctx.map, id) {
            let health_attr = ctx.health_attr.clone();
            if let Some(mut health) = entity.attributes.get_int(&health_attr) {
                // Reduce the health of the target
                health -= amount;
                health = health.max(0);
                // Set the new health
                entity.set_attribute(&health_attr, Value::Int(health));

                /*
                let mode = entity.attributes.get_str_default("mode", "".into());
                if health <= 0 && mode != "dead" {
                    // Send "death" event
                    if let Some(class_name) = entity.attributes.get_str("class_name") {
                        let cmd = format!("{}.event(\"death\", \"\")", class_name);
                        ctx.to_execute_entity.push((entity.id, "death".into(), cmd));

                        entity.set_attribute("mode", Value::Str("dead".into()));
                        entity.action = EntityAction::Off;
                        ctx.entity_proximity_alerts.remove(&entity.id);

                        kill = true;
                    }
                }*/
            }
        }

        /*
        // if receiver got killed, send a "kill" event to the attacker
        if kill {
            if let Some(entity) = get_entity_mut(&mut ctx.map, from) {
                // Send "kill" event
                if let Some(class_name) = entity.attributes.get_str("class_name") {
                    let cmd = format!("{}.event(\"kill\", {})", class_name, id);
                    ctx.to_execute_entity.push((from, "kill".into(), cmd));
                }
            }
        }*/
    });
}

/// Get an attribute from the given entity.
fn get_attr_of(id: u32, key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            if let Some(v) = entity.attributes.get(&key) {
                value = v.clone();
            }
        }

        if ctx.debug_mode {
            if value != Value::NoValue {
                add_debug_value(ctx, TheValue::Text(value.to_string()), false);
            }
        }
    });

    if value == Value::NoValue {
        let item_id = id;
        with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
            if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                if let Some(v) = item.get_attribute(&key) {
                    value = v.clone();
                }
            }

            if ctx.debug_mode {
                if value == Value::NoValue {
                    add_debug_value(ctx, TheValue::Text("Not Found".into()), true);
                } else {
                    add_debug_value(ctx, TheValue::Text(value.to_string()), false);
                }
            }
        });
    }

    Ok(value.to_pyobject(vm))
}

/*
/// Get an attribute from the given entity.
fn get_entity_attr(entity_id: u32, key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            if let Some(v) = entity.attributes.get(&key) {
                value = v.clone();
            }
        }

        if ctx.debug_mode {
            if value == Value::NoValue {
                add_debug_value(ctx, Value::Str("Not Found".into()), true);
            }
        }
    });

    Ok(value.to_pyobject(vm))
}

/// Get an attribute from the given item.
fn get_item_attr(item_id: u32, key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
            if let Some(v) = item.get_attribute(&key) {
                value = v.clone();
            }
        }

        if ctx.debug_mode {
            if value == Value::NoValue {
                add_debug_value(ctx, Value::Str("Not Found".into()), true);
            }
        }
    });

    Ok(value.to_pyobject(vm))
}
*/

/// Get an attribute from the current item or entity.
fn get_attr(key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                if let Some(v) = item.get_attribute(&key) {
                    value = v.clone();
                }
            }
        } else {
            let entity_id = ctx.curr_entity_id;
            if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                if let Some(v) = entity.attributes.get(&key) {
                    value = v.clone();
                }
            }
        }

        if ctx.debug_mode {
            if value == Value::NoValue {
                add_debug_value(ctx, TheValue::Text("Not Found".into()), true);
            } else {
                add_debug_value(ctx, TheValue::Text(value.to_string()), false);
            }
        }
    });

    Ok(value.to_pyobject(vm))
}

/// Toggles a boolean attribute of the current entity or item.
fn toggle_attr(key: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                item.attributes.toggle(&key);
                if key == "active" {
                    // Send active state
                    if let Some(class_name) = item.attributes.get_str("class_name") {
                        let cmd = format!(
                            "{}.event(\"active\", {})",
                            class_name,
                            if item.attributes.get_bool_default("active", false) {
                                "True"
                            } else {
                                "False"
                            }
                        );
                        // ctx.to_execute_item.push((item.id, "active".into(), cmd));
                    }
                }
            } else {
                let entity_id = ctx.curr_entity_id;
                if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                    entity.attributes.toggle(&key);
                }
            }
        }
    });
}

/// Set the attribute of the current entity or item.
fn set_attr(key: PyObjectRef, value: PyObjectRef, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Ok(key) = String::try_from_object(vm, key) {
            if let Some(value) = Value::from_pyobject(value, vm) {
                if let Some(item_id) = ctx.curr_item_id {
                    if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                        item.set_attribute(&key, value);

                        if key == "active" {
                            // Send active state
                            if let Some(class_name) = item.attributes.get_str("class_name") {
                                let cmd = format!(
                                    "{}.event(\"active\", {})",
                                    class_name,
                                    if item.attributes.get_bool_default("active", false) {
                                        "True"
                                    } else {
                                        "False"
                                    }
                                );
                                // ctx.to_execute_item.push((item.id, "active".into(), cmd));
                            }
                        }
                    }
                } else {
                    let entity_id = ctx.curr_entity_id;
                    if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                        entity.set_attribute(&key, value);
                    }
                }
            }
        }
    });
}

/// Returns a list of filtered inventory items.
fn inventory_items_of(
    entity_id: u32,
    filter: String,
    vm: &VirtualMachine,
) -> PyResult<PyObjectRef> {
    let mut items = Vec::new();

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(entity) = ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == entity_id)
        {
            for (_, item) in entity.iter_inventory() {
                let name = item.attributes.get_str("name").unwrap_or_default();
                let class_name = item.attributes.get_str("class_name").unwrap_or_default();

                if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                    items.push(item.id);
                }
            }
        }
    });

    let py_list = vm.ctx.new_list(
        items
            .iter()
            .map(|&id| vm.ctx.new_int(id).into())
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Returns a list of filtered inventory items.
fn inventory_items(filter: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut items = Vec::new();

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;

        if let Some(entity) = ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == entity_id)
        {
            for (_, item) in entity.iter_inventory() {
                let name = item.attributes.get_str("name").unwrap_or_default();
                let class_name = item.attributes.get_str("class_name").unwrap_or_default();

                if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                    items.push(item.id);
                }
            }
        }
    });

    let py_list = vm.ctx.new_list(
        items
            .iter()
            .map(|&id| vm.ctx.new_int(id).into())
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Drop the item with the given id.
fn drop(item_id: u32, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        let mut slot = None;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            // Collect matching slot indices
            for (index, item) in entity.inventory.iter().enumerate() {
                if let Some(item) = item {
                    if item.id == item_id {
                        slot = Some(index);
                        break;
                    }
                }
            }

            let mut removed_items = Vec::new();
            if let Some(slot) = slot {
                if let Some(mut item) = entity.remove_item_from_slot(slot) {
                    item.position = entity.position;
                    item.mark_all_dirty();
                    removed_items.push(item);
                }
            }

            for item in removed_items {
                ctx.map.items.push(item);
            }
        }
    });
}

/// Drop the given items.
fn drop_items(filter: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            // Collect matching slot indices
            let matching_slots: Vec<usize> = entity
                .iter_inventory()
                .filter_map(|(slot, item)| {
                    let name = item.attributes.get_str("name").unwrap_or_default();
                    let class_name = item.attributes.get_str("class_name").unwrap_or_default();

                    if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                        Some(slot)
                    } else {
                        None
                    }
                })
                .collect();

            // Remove matching items from slots
            let mut removed_items = Vec::new();
            for slot in matching_slots {
                if let Some(mut item) = entity.remove_item_from_slot(slot) {
                    item.position = entity.position;
                    item.mark_all_dirty();
                    removed_items.push(item);
                }
            }

            for item in removed_items {
                ctx.map.items.push(item);
            }
        }
    });
}

/// Offer inventory.
fn offer_inventory(to: u32, filter: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            // Collect matching slot indices
            let matching_item_ids: Vec<u32> = entity
                .iter_inventory()
                .filter_map(|(_, item)| {
                    let name = item.attributes.get_str("name").unwrap_or_default();
                    let class_name = item.attributes.get_str("class_name").unwrap_or_default();

                    if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                        Some(item.id)
                    } else {
                        None
                    }
                })
                .collect();

            let timeout_minutes = ctx
                .map
                .entities
                .iter()
                .find(|entity| entity.id == entity_id)
                .map(|entity| entity.attributes.get_float_default("timeout", 10.0))
                .unwrap_or(10.0)
                .max(0.0);
            let expires_at_tick = ctx.ticks + (ctx.ticks_per_minute as f32 * timeout_minutes) as i64;
            let max_distance = entity_intent_distance_limit(ctx, entity_id, "talk")
                .or_else(|| entity_intent_distance_limit(ctx, entity_id, "use"))
                .unwrap_or(2.0)
                .max(0.0);
            let mut choices =
                MultipleChoice::new(ctx.region_id, entity_id, to, expires_at_tick, max_distance);
            clear_choice_session(ctx, entity_id, to);
            ctx.active_choice_sessions.push(ChoiceSession {
                from: entity_id,
                to,
                expires_at_tick,
                max_distance,
            });
            for item_id in matching_item_ids {
                let choice =
                    Choice::ItemToSell(item_id, entity_id, to, expires_at_tick, max_distance);
                choices.add(choice);
            }

            ctx.from_sender
                .get()
                .unwrap()
                .send(RegionMessage::MultipleChoice(choices))
                .unwrap();
        }
    });
}

/// Returns the entity at the given position (if any)
fn get_entity_at(ctx: &RegionCtx, position: Vec2<f32>, but_not: u32) -> Option<u32> {
    let mut entity = None;

    for other in ctx.map.entities.iter() {
        if other.id == but_not {
            continue;
        }
        let other_position = other.get_pos_xz();

        let distance = position.distance(other_position);

        // Item is inside the radius
        if distance < 1.0 {
            entity = Some(other.id);
            break; // We only need the first item found
        }
    }

    entity
}

/// Returns the item at the given position (if any)
fn get_item_at(ctx: &RegionCtx, position: Vec2<f32>) -> Option<u32> {
    let mut item = None;

    for other in ctx.map.items.iter() {
        let other_position = other.get_pos_xz();

        let distance = position.distance(other_position);

        // Item is inside the radius
        if distance < 1.0 {
            item = Some(other.id);
            break; // We only need the first item found
        }
    }

    item
}

/// Returns the entities in the radius of the character or item.
fn entities_in_radius(vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut radius = 0.5;
    let mut position = None;
    let mut is_entity = false;
    let mut id = 0;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if let Some(item) = ctx.map.items.iter().find(|item| item.id == item_id) {
                id = item_id;
                position = Some(item.get_pos_xz());
                radius = item.attributes.get_float_default("radius", 0.5);
            }
        } else {
            let entity_id = ctx.curr_entity_id;
            is_entity = true;
            if let Some(entity) = ctx
                .map
                .entities
                .iter()
                .find(|entity| entity.id == entity_id)
            {
                id = entity.id;
                position = Some(entity.get_pos_xz());
                radius = entity.attributes.get_float_default("radius", 0.5);
            }
        }
    });

    let mut entities = Vec::new();

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(position) = position {
            for other in ctx.map.entities.iter() {
                if is_entity && other.id == id {
                    continue;
                }
                let other_position = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5);

                let distance_squared = (position - other_position).magnitude_squared();
                let combined_radius = radius + other_radius;
                let combined_radius_squared = combined_radius * combined_radius;

                // Entity is inside the radius
                if distance_squared < combined_radius_squared {
                    entities.push(other.id);
                }
            }
        }
    });

    let py_list = vm.ctx.new_list(
        entities
            .iter()
            .map(|&id| vm.ctx.new_int(id).into())
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Add an item to the characters inventory
fn add_item(class_name: String, vm: &VirtualMachine) -> i32 {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item) = create_item(ctx, class_name.clone()) {
            let id = ctx.curr_entity_id;
            if let Some(entity) = ctx.map.entities.iter_mut().find(|entity| entity.id == id) {
                let item_id = item.id;
                if entity.add_item(item).is_ok() {
                    if ctx.debug_mode {
                        add_debug_value(ctx, TheValue::Text("Ok".into()), false);
                    }
                    item_id as i32
                } else {
                    if ctx.debug_mode {
                        add_debug_value(ctx, TheValue::Text("Inventory Full".into()), true);
                    }
                    println!("add_item ({}): Inventory is full", class_name);
                    -1
                }
            } else {
                -1
            }
        } else {
            if ctx.debug_mode {
                add_debug_value(ctx, TheValue::Text("Unknown Item".into()), true);
            }
            -1
        }
    })
    .unwrap()
}

/// Add a debug value at the current debug position
#[inline(always)]
pub fn add_debug_value(ctx: &mut RegionCtx, value: TheValue, error: bool) {
    if let Some((event, x, y)) = &ctx.curr_debug_loc {
        if let Some(item_id) = ctx.curr_item_id {
            ctx.debug.add_value(item_id, event, *x, *y, value);
            if error {
                ctx.debug.add_error(item_id, event, *x, *y);
            } else {
                ctx.debug.remove_error(item_id, event, *x, *y);
            }
        } else {
            ctx.debug
                .add_value(ctx.curr_entity_id, event, *x, *y, value);
            if error {
                ctx.debug.add_error(ctx.curr_entity_id, event, *x, *y);
            } else {
                ctx.debug.remove_error(ctx.curr_entity_id, event, *x, *y);
            }
        }

        ctx.curr_debug_loc = None;
    }
}

/// Equip the item with the given item id.
fn equip(item_id: u32, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let id = ctx.curr_entity_id;
        if let Some(entity) = ctx.map.entities.iter_mut().find(|entity| entity.id == id) {
            let mut slot: Option<String> = None;
            if let Some(item) = entity.get_item(item_id) {
                if let Some(sl) = item.attributes.get_str("slot") {
                    slot = Some(sl.to_string());
                }
            }

            if let Some(slot) = slot {
                if entity.equip_item(item_id, &slot).is_err() {
                    println!("Equipped failure");
                } else {
                    if ctx.debug_mode {
                        add_debug_value(ctx, TheValue::Text("Ok".into()), false);
                    }
                }
            } else {
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Text("Unknown Item".into()), true);
                }
            }
        }
    });
}

/// Notify the entity / item in the given amount of minutes.
fn notify_in(minutes: i32, notification: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let tick = ctx.ticks + RegionInstance::scheduled_delay_ticks(ctx, minutes as f32);
        if let Some(item_id) = ctx.curr_item_id {
            ctx.notifications_items.push((item_id, tick, notification));
        } else {
            if !is_entity_dead_ctx(ctx, ctx.curr_entity_id) {
                ctx.notifications_entities
                    .push((ctx.curr_entity_id, tick, notification));
            }
        }
    });
}

/*
/// Returns the name of the sector the entity or item is currently in.
fn get_sector_name() -> String {
    let map = MAP.borrow();

    if let Some(item_id) = *CURR_ITEMID.borrow() {
        for e in map.items.iter() {
            if e.id == item_id {
                let pos = e.get_pos_xz();
                if let Some(s) = map.find_sector_at(pos) {
                    if s.name.is_empty() {
                        return "Unnamed Sector".to_string();
                    } else {
                        return s.name.clone();
                    }
                }
            }
        }
    } else {
        for e in map.entities.iter() {
            if e.id == *CURR_ENTITYID.borrow() {
                let pos = e.get_pos_xz();
                if let Some(s) = map.find_sector_at(pos) {
                    if s.name.is_empty() {
                        return "Unnamed Sector".to_string();
                    } else {
                        return s.name.clone();
                    }
                }
            }
        }
    }

    "Not inside any sector".to_string()
}

/// Faces the entity at a random direction.
fn face_random() {
    let entity_id = *CURR_ENTITYID.borrow();
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        entity.face_random();
    }
}*/

/// Goto a destination sector with the given speed.
fn goto(destination: String, speed: f32, vm: &VirtualMachine) {
    let mut coord: Option<vek::Vec2<f32>> = None;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        for sector in &ctx.map.sectors {
            if sector.name == destination {
                coord = sector.center(&ctx.map);
            }
        }

        if let Some(coord) = coord {
            let entity_id = ctx.curr_entity_id;
            if let Some(entity) = ctx
                .map
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
            {
                let position = entity.get_pos_xz();
                let start_center = RegionInstance::snapped_grid_center(position);
                let target_center = RegionInstance::snapped_grid_center(coord);
                let grid_aligned =
                    (position - start_center).magnitude_squared() <= 0.001
                        && (coord - target_center).magnitude_squared() <= 0.001;
                if grid_aligned {
                    entity.action = GotoGrid(coord, speed);
                } else {
                    entity.action = Goto(coord, speed);
                }
            }
        } else {
            if ctx.debug_mode {
                add_debug_value(ctx, TheValue::Text("Unknown Sector".into()), true);
            }
        }
    });
}

fn run_sequence(name: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            let sequence_name = name.trim();
            if entity.sequences.contains_key(sequence_name) {
                entity.active_sequence = Some(crate::server::entity::EntitySequenceState {
                    name: sequence_name.to_string(),
                    step_index: 0,
                    wait_until_tick: None,
                });
                entity.paused_sequence = None;
                entity.action = EntityAction::Off;
            }
        }
    });
}

fn pause_sequence(vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
            && let Some(active) = entity.active_sequence.take()
        {
            entity.paused_sequence = Some(active);
            entity.action = EntityAction::Off;
        }
    });
}

fn resume_sequence(vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id)
            && entity.active_sequence.is_none()
            && let Some(paused) = entity.paused_sequence.take()
        {
            entity.active_sequence = Some(paused);
            entity.action = EntityAction::Off;
        }
    });
}

fn cancel_sequence(vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            entity.active_sequence = None;
            entity.paused_sequence = None;
            entity.action = EntityAction::Off;
        }
    });
}

/// CloseIn: Move within a radius of a target entity with a given speed
fn close_in(target: u32, target_radius: f32, speed: f32, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            entity.action = CloseIn(target, target_radius, speed);
        }
    });
}

fn follow_attack(target: u32, speed: f32, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            let next_attack_tick = match entity.action {
                EntityAction::FollowAttack(existing_target, _, next_tick) if existing_target == target => {
                    next_tick
                }
                _ => 0,
            };
            entity.set_attribute("target", Value::UInt(target));
            entity.set_attribute("attack_target", Value::UInt(target));
            entity.action = EntityAction::FollowAttack(target, speed, next_attack_tick);
        }
    });
}

/// Randomly walks
fn random_walk(
    distance: PyObjectRef,
    speed: PyObjectRef,
    max_sleep: PyObjectRef,
    vm: &VirtualMachine,
) {
    let distance: f32 = get_f32(distance, 1.0, vm);
    let speed: f32 = get_f32(speed, 1.0, vm);
    let max_sleep: i32 = get_i32(max_sleep, 0, vm);

    with_regionctx(get_region_id(vm).unwrap(), |ctx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            entity.action = RandomWalk(distance, speed, max_sleep, 0, zero());
        }
    });
}

/// Randomly walks within the current sector.
fn random_walk_in_sector(
    distance: PyObjectRef,
    speed: PyObjectRef,
    max_sleep: PyObjectRef,
    vm: &VirtualMachine,
) {
    let distance: f32 = get_f32(distance, 1.0, vm); // Default distance: 1.0
    let speed: f32 = get_f32(speed, 1.0, vm); // Default speed: 1.0
    let max_sleep: i32 = get_i32(max_sleep, 0, vm); // Default max_sleep: 0

    with_regionctx(get_region_id(vm).unwrap(), |ctx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            entity.action = RandomWalkInSector(distance, speed, max_sleep, 0, zero());
        }
    });
}

/// Set Proximity Tracking
pub fn set_proximity_tracking(
    args: rustpython_vm::function::FuncArgs,
    vm: &VirtualMachine,
) -> PyResult<()> {
    let mut turn_on = false;
    let mut distance = 5.0;

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::Bool(v)) = Value::from_pyobject(arg.clone(), vm) {
                turn_on = v;
            }
        } else if i == 1 {
            if let Some(Value::Float(v)) = Value::from_pyobject(arg.clone(), vm) {
                distance = v;
            }
        }
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if turn_on {
                ctx.item_proximity_alerts.insert(item_id, distance);
            } else {
                ctx.item_proximity_alerts.remove(&item_id);
            }
        } else {
            let entity_id = ctx.curr_entity_id;
            if turn_on {
                ctx.entity_proximity_alerts.insert(entity_id, distance);
            } else {
                ctx.entity_proximity_alerts.remove(&entity_id);
            }
        }
    });

    Ok(())
}

/// Teleport
pub fn teleport(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut sector_name = String::new();
    let mut region_name = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                sector_name = v.clone();
            }
        } else if i == 1 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                region_name = v.clone();
            }
        }
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if region_name.is_empty() {
            // Teleport entity in this region to the given sector.

            let mut new_pos: Option<vek::Vec2<f32>> = None;
            for sector in &ctx.map.sectors {
                if sector.name == sector_name {
                    new_pos = sector.center(&ctx.map);
                }
            }

            if let Some(new_pos) = new_pos {
                let entity_id = ctx.curr_entity_id;
                let mut entities = ctx.map.entities.clone();
                if let Some(entity) = entities.iter_mut().find(|entity| entity.id == entity_id) {
                    entity.set_pos_xz(new_pos);
                    check_player_for_section_change(ctx, entity);
                }
                ctx.map.entities = entities;
            } else {
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Text("Unknown Sector".into()), true);
                }
            }
        } else {
            // Remove the entity from this region and send it to the server to be moved
            // into a new region.

            let entity_id = ctx.curr_entity_id;
            if let Some(pos) = ctx.map.entities.iter().position(|e| e.id == entity_id) {
                let removed = ctx.map.entities.remove(pos);

                ctx.entity_classes.remove(&removed.id);

                let msg =
                    RegionMessage::TransferEntity(ctx.region_id, removed, region_name, sector_name);
                ctx.from_sender.get().unwrap().send(msg).unwrap();
            }
        }
    });

    Ok(())
}

/// Message
pub fn message(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut receiver = None;
    let mut message = None;
    let mut category = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::UInt(v)) = Value::from_pyobject(arg.clone(), vm) {
                receiver = Some(v);
            } else if let Some(Value::Int(v)) = Value::from_pyobject(arg.clone(), vm) {
                receiver = Some(v as u32);
            }
        } else if i == 1 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                message = Some(v);
            }
        } else if i == 2 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                category = v.clone();
            }
        }
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if receiver.is_some() && message.is_some() {
            let mut entity_id = Some(ctx.curr_entity_id);
            let item_id = ctx.curr_item_id;
            if item_id.is_some() {
                entity_id = None;
            }

            let message = message.unwrap();
            let msg = RegionMessage::Message(
                ctx.region_id,
                entity_id,
                item_id,
                receiver.unwrap() as u32,
                message,
                category,
            );
            ctx.from_sender.get().unwrap().send(msg).unwrap();

            if ctx.debug_mode {
                add_debug_value(ctx, TheValue::Text("Ok".into()), false);
            }
        }
    });

    Ok(())
}

/// Say
pub fn say(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut message = None;
    let mut category = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                message = Some(v);
            }
        } else if i == 1 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                category = v.clone();
            }
        }
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if message.is_some() {
            let mut entity_id = Some(ctx.curr_entity_id);
            let item_id = ctx.curr_item_id;
            if item_id.is_some() {
                entity_id = None;
            }

            let message = message.unwrap();
            let msg = RegionMessage::Say(
                ctx.region_id,
                entity_id,
                item_id,
                message,
                category,
            );
            ctx.from_sender.get().unwrap().send(msg).unwrap();

            if ctx.debug_mode {
                add_debug_value(ctx, TheValue::Text("Ok".into()), false);
            }
        }
    });

    Ok(())
}

/// Debug
pub fn debug(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut output = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        let arg_str = match vm.call_method(arg.as_object(), "__repr__", ()) {
            Ok(repr_obj) => match repr_obj.str(vm) {
                Ok(s) => s.as_str().to_owned(),
                Err(_) => "<error converting repr to str>".to_owned(),
            },
            Err(_) => "<error calling __repr__>".to_owned(),
        };

        if i > 0 {
            output.push(' ');
        }
        output.push_str(&arg_str);
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(name) = get_attr_internal(ctx, "name") {
            output = format!("{}: {}", name, output);
        }
    });

    send_log_message(get_region_id(vm).unwrap(), output);
    Ok(())
}

/// Send a log message.
pub fn send_log_message(id: u32, message: String) {
    with_regionctx(id, |ctx| {
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::LogMessage(message))
            .unwrap();
    });
}

/// Get an i32 config value
fn get_config_i32_default(ctx: &RegionCtx, table: &str, key: &str, default: i32) -> i32 {
    let mut value = default;
    let tab = &ctx.config;
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(val) = game.get(key) {
            if let Some(v) = val.as_integer() {
                value = v as i32;
            }
        }
    }
    value
}

/*
fn _get_config_f32_default(table: &str, key: &str, default: f32) -> f32 {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_float() {
                return v as f32;
            }
        }
    }
    default
}

fn _get_config_bool_default(table: &str, key: &str, default: bool) -> bool {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_bool() {
                return v;
            }
        }
    }
    default
}
*/

fn get_config_string_default(ctx: &RegionCtx, table: &str, key: &str, default: &str) -> String {
    let mut value = default.to_string();
    let tab = &ctx.config;
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(val) = game.get(key) {
            if let Some(v) = val.as_str() {
                value = v.to_string();
            }
        }
    }
    value
}

/// Get an attribute value from the current item or entity.
fn get_attr_internal(ctx: &mut RegionCtx, key: &str) -> Option<Value> {
    if let Some(id) = ctx.curr_item_id {
        if let Some(item) = get_item_mut(&mut ctx.map, id) {
            return item.attributes.get(key).cloned();
        }
    } else {
        let id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, id) {
            return entity.attributes.get(key).cloned();
        }
    };

    None
}

/// Received an entity from another region
pub fn receive_entity(ctx: &mut RegionCtx, mut entity: Entity, dest_sector_name: String) {
    entity.action = EntityAction::Off;
    let entity_id = entity.id;
    if entity.is_player() {
        entity.set_attribute("mode", Value::Str("active".into()));
        entity.set_attribute("visible", Value::Bool(true));
    }

    let mut new_pos: Option<vek::Vec2<f32>> = None;
    for sector in &ctx.map.sectors {
        if sector.name == dest_sector_name {
            new_pos = sector.center(&ctx.map);
        }
    }

    if let Some(new_pos) = new_pos {
        entity.set_pos_xz(new_pos);
        entity.position.y =
            map_spawn_height(&ctx.map, entity.get_pos_xz(), Some(entity.position.y));
    }

    if let Some(class_name) = entity.get_attr_string("class_name") {
        ctx.entity_classes.insert(entity_id, class_name.clone());
    }

    ctx.map.entities.retain(|existing| existing.id != entity_id);
    ctx.map.entities.push(entity);
    ctx.check_player_for_section_change_id(entity_id);
}

fn id(vm: &VirtualMachine) -> u32 {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        ctx.curr_entity_id
    })
    .unwrap()
}

/// Used only for local, Eldiron Creator emitted commands.
fn player_action(action: String, vm: &VirtualMachine) {
    if let Ok(parsed_action) = action.parse::<EntityAction>() {
        with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
            let entity_id = ctx.curr_entity_id;
            if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                entity.action = parsed_action;
            }
        });
    }
}

/// Dummy. Only used on the client side.
fn player_intent(_intent: String, _vm: &VirtualMachine) {}

/// Set the current debug location in the grid.
fn set_debug_loc(event: String, x: u32, y: u32, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        ctx.curr_debug_loc = Some((event, x, y));
    });
}

*/
