//! Opt-in host policy. Embedded hosts default to all regions active. Region-local
//! time freezes while suspended; wake resumes from that time without catch-up.
use super::*;
use std::{collections::BTreeSet, time::Duration};

#[derive(Clone, Debug)]
pub struct RegionSchedulingPolicy {
    pub pause_unoccupied_regions: BTreeSet<Uuid>,
    pub idle_after: Duration,
}
impl Default for RegionSchedulingPolicy {
    fn default() -> Self {
        Self {
            pause_unoccupied_regions: Default::default(),
            idle_after: Duration::from_secs(5),
        }
    }
}
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct RegionScheduleSummary {
    pub region: u32,
    pub eligible: bool,
    pub suspended: bool,
    pub idle_ms: u64,
    pub sleeps: u64,
    pub wakes: u64,
    pub skipped_movement: u64,
    pub skipped_game: u64,
}
#[derive(Default)]
pub(super) struct RegionIdle {
    pub eligible: bool,
    pub suspended: bool,
    pub elapsed: Duration,
    pub sleeps: u64,
    pub wakes: u64,
    pub skipped_movement: u64,
    pub skipped_game: u64,
}
impl RegionIdle {
    pub fn wake(&mut self) {
        if self.suspended {
            self.wakes = self.wakes.saturating_add(1);
        }
        self.suspended = false;
        self.elapsed = Duration::ZERO;
    }
    pub fn summary(&self, region: u32) -> RegionScheduleSummary {
        RegionScheduleSummary {
            region,
            eligible: self.eligible,
            suspended: self.suspended,
            idle_ms: self.elapsed.as_millis().min(u64::MAX as u128) as u64,
            sleeps: self.sleeps,
            wakes: self.wakes,
            skipped_movement: self.skipped_movement,
            skipped_game: self.skipped_game,
        }
    }
}
impl Server {
    pub fn set_region_scheduling(&mut self, policy: RegionSchedulingPolicy) -> Result<(), String> {
        if !(Duration::from_millis(20)..=Duration::from_secs(600)).contains(&policy.idle_after)
            || policy.pause_unoccupied_regions.len() > 1024
            || policy
                .pause_unoccupied_regions
                .iter()
                .any(|id| !self.region_id_map.contains_key(id))
        {
            return Err(
                "Invalid region scheduling: known map IDs and idle interval 20..600000 ms required"
                    .into(),
            );
        }
        self.scheduling = policy;
        for instance in &self.instances {
            instance.lock().unwrap().idle = Default::default();
        }
        Ok(())
    }
    /// Called once per host advance (zero for command-only polls). Incoming
    /// commands, spawns and ownership wake before consuming any input/time.
    pub fn prepare_region_schedule(&self, elapsed: Duration) {
        if self.scheduling.pause_unoccupied_regions.is_empty() {
            return;
        }
        let occupied: BTreeSet<_> = self
            .connections
            .values()
            .filter_map(|state| match state {
                PlayerConnectionState::Active(c) => Some(c.region_id),
                PlayerConnectionState::Spawning { region_id } => Some(*region_id),
                _ => None,
            })
            .collect();
        for instance in &self.instances {
            let mut region = instance.lock().unwrap();
            let Some(context) = region.context() else {
                continue;
            };
            let ctx = context.lock().unwrap();
            let eligible = self
                .scheduling
                .pause_unoccupied_regions
                .contains(&ctx.map.id);
            let pending = region.has_pending_input()
                || !ctx.pending_entity_transfers.is_empty()
                || !ctx.pending_entity_removals.is_empty()
                || !ctx.to_execute_entity.is_empty()
                || !ctx.to_execute_item.is_empty()
                || !ctx.to_execute_world.is_empty();
            let awake = !eligible || occupied.contains(&region.id) || pending || ctx.debug_mode;
            drop(ctx);
            region.idle.eligible = eligible;
            if awake {
                region.idle.wake();
            } else {
                region.idle.elapsed = region
                    .idle
                    .elapsed
                    .saturating_add(elapsed)
                    .min(self.scheduling.idle_after);
                if !region.idle.suspended && region.idle.elapsed >= self.scheduling.idle_after {
                    region.idle.suspended = true;
                    region.idle.sleeps = region.idle.sleeps.saturating_add(1);
                }
            }
        }
    }
    pub fn region_schedule(&self) -> Vec<RegionScheduleSummary> {
        self.instances
            .iter()
            .map(|r| {
                let r = r.lock().unwrap();
                r.idle.summary(r.id)
            })
            .collect()
    }
    pub fn has_region_scheduling(&self) -> bool {
        !self.scheduling.pause_unoccupied_regions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Entity, EntityAction, RegionCtx, Value};
    fn fixture() -> (Server, Arc<Mutex<RegionCtx>>, Arc<Mutex<RegionCtx>>) {
        let mut server = Server::default();
        server.state = ServerState::Running;
        let mut contexts = vec![];
        for id in 1..=2 {
            let mut ctx = RegionCtx::default();
            ctx.ticks_per_minute = 10;
            ctx.region_id = id;
            ctx.map.id = Uuid::new_v4();
            let mut actor = Entity::default();
            actor.id = ctx.world.next_id();
            actor.position = vek::Vec3::new(0.5, 0., 0.5);
            actor.action = EntityAction::Forward;
            actor.set_attribute("avatar_attack_left", Value::Float(1.));
            ctx.map.entities.push(actor);
            let ctx = Arc::new(Mutex::new(ctx));
            let region = region::RegionInstance::with_context(id, ctx.clone());
            ctx.lock()
                .unwrap()
                .from_sender
                .set(region.test_from_sender())
                .unwrap();
            server.region_id_map.insert(ctx.lock().unwrap().map.id, id);
            server.instances.push(Arc::new(Mutex::new(region)));
            contexts.push(ctx);
        }
        (server, contexts.remove(0), contexts.remove(0))
    }
    fn opt_in(server: &mut Server, map: Uuid) {
        server
            .set_region_scheduling(RegionSchedulingPolicy {
                pause_unoccupied_regions: [map].into(),
                idle_after: Duration::from_millis(100),
            })
            .unwrap();
    }
    #[test]
    fn default_ticks_every_region_and_opt_in_freezes_only_unoccupied_region_time() {
        let (mut server, a, b) = fixture();
        server.prepare_region_schedule(Duration::from_secs(100));
        server.movement_tick();
        server.system_tick();
        assert!(server.region_schedule().iter().all(|r| !r.suspended));
        opt_in(&mut server, a.lock().unwrap().map.id);
        let ticks = a.lock().unwrap().ticks;
        let position = a.lock().unwrap().map.entities[0].position;
        let left = a.lock().unwrap().map.entities[0]
            .attributes
            .get_float("avatar_attack_left")
            .unwrap();
        // Outbound projection/time messages cannot keep an idle region awake.
        server.prepare_region_schedule(Duration::from_millis(100));
        for _ in 0..20 {
            server.redraw_tick();
            server.movement_tick();
            server.system_tick();
            server.prepare_region_schedule(Duration::ZERO);
        }
        let a = a.lock().unwrap();
        assert_eq!(a.ticks, ticks);
        assert_eq!(a.map.entities[0].position, position);
        assert_eq!(
            a.map.entities[0].attributes.get_float("avatar_attack_left"),
            Some(left)
        );
        assert!(b.lock().unwrap().ticks > ticks);
        let stats = server.region_schedule();
        assert!(stats[0].suspended);
        assert_eq!(stats[0].skipped_movement, 20);
    }
    #[test]
    fn players_spawns_and_addressed_commands_wake_without_elapsed_time_catchup() {
        let (mut server, a, _) = fixture();
        let map = a.lock().unwrap().map.id;
        opt_in(&mut server, map);
        server.prepare_region_schedule(Duration::from_secs(100));
        assert!(server.region_schedule()[0].suspended);
        let before = a.lock().unwrap().ticks;
        server.instances[0]
            .lock()
            .unwrap()
            .to_sender
            .send(RegionMessage::TeleportEntityPos(
                1,
                vek::Vec3::new(3., 0., 3.).into(),
            ))
            .unwrap();
        // Even a direct redraw notices commands between host scheduling cycles.
        server.redraw_tick();
        assert!(!server.region_schedule()[0].suspended);
        assert_eq!(a.lock().unwrap().ticks, before);
        assert_eq!(a.lock().unwrap().map.entities[0].position.x, 3.);
        server.system_tick();
        assert_eq!(a.lock().unwrap().ticks, before + 1);
        let id = server.open_player_connection();
        for state in [
            PlayerConnectionState::Spawning { region_id: 1 },
            PlayerConnectionState::Active(PlayerControl {
                region_id: 1,
                entity_id: 1,
            }),
        ] {
            server.connections.insert(id, state);
            server.prepare_region_schedule(Duration::from_secs(100));
            assert!(!server.region_schedule()[0].suspended);
        }
        server.connections.remove(&id);
        server.prepare_region_schedule(Duration::from_millis(99));
        assert!(!server.region_schedule()[0].suspended);
        server.prepare_region_schedule(Duration::from_millis(1));
        assert!(server.region_schedule()[0].suspended);
        assert_eq!(server.region_schedule()[0].wakes, 1);
    }
    #[test]
    fn queued_transfers_and_effects_hold_wake_and_invalid_policies_do_not_replace_policy() {
        let (mut server, a, _) = fixture();
        opt_in(&mut server, a.lock().unwrap().map.id);
        a.lock()
            .unwrap()
            .pending_entity_transfers
            .push((1, "other".into(), "arrival".into()));
        server.prepare_region_schedule(Duration::from_secs(100));
        assert!(!server.region_schedule()[0].suspended);
        a.lock().unwrap().pending_entity_transfers.clear();
        a.lock()
            .unwrap()
            .to_execute_entity
            .push((1, "effect".into(), crate::vm::VMValue::zero()));
        server.prepare_region_schedule(Duration::from_secs(100));
        assert!(!server.region_schedule()[0].suspended);
        assert!(
            server
                .set_region_scheduling(RegionSchedulingPolicy {
                    pause_unoccupied_regions: [Uuid::new_v4()].into(),
                    ..Default::default()
                })
                .is_err()
        );
        assert!(server.has_region_scheduling());
        server.clear();
        assert!(!server.has_region_scheduling());
    }
}
