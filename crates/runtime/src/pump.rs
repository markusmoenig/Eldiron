//! CPU update and addressed output ownership, independent of storage/renderer.
use crate::{
    ClientFrame, ClientWorld, PlayerConnectionId, ReplicationError, replication::ReplicationHub,
};
use crate::{EventInterests, Project, RegionEvents, SessionClock, SessionUpdate};
use rustc_hash::FxHashMap;
use rusterix::{Assets, Server, ServerState};
use std::time::Duration;
use theframework_core::Uuid;

/// One pump per local runtime incarnation. GameSession owns all source storage;
/// migration hosts may lend explicit CPU parts while retiring their facade.
/// Presentation never calls this API or drains Server output channels.
pub struct SessionPump {
    diagnostics: Option<crate::diagnostics::PumpTimings>,
    session_id: Uuid,
    tick_sequence: u64,
    movement_tick_sequence: u64,
    explicit_clock: SessionClock,
    interests: EventInterests,
    events: FxHashMap<Uuid, RegionEvents>,
    replication: Option<ReplicationHub>,
    local_connection: Option<PlayerConnectionId>,
    movement_inputs: FxHashMap<PlayerConnectionId, crate::movement::PendingMovement>,
    frame_inputs: crate::frame_input::FrameInputs,
}
impl SessionPump {
    pub(crate) fn restore_counters(&mut self, tick: u64, movement: u64) {
        self.tick_sequence = tick;
        self.movement_tick_sequence = movement;
    }
    pub fn new(interests: EventInterests) -> Self {
        Self {
            diagnostics: None,
            session_id: Uuid::new_v4(),
            tick_sequence: 0,
            movement_tick_sequence: 0,
            explicit_clock: SessionClock::new(Duration::from_millis(250)),
            interests,
            events: FxHashMap::default(),
            replication: None,
            local_connection: None,
            movement_inputs: Default::default(),
            frame_inputs: Default::default(),
        }
    }
    pub(crate) fn enable_diagnostics(&mut self, enabled: bool) {
        self.diagnostics = enabled.then(Default::default);
    }
    pub(crate) fn diagnostics(
        &self,
        server: &Server,
    ) -> Option<crate::diagnostics::SessionDiagnostics> {
        let d = self.diagnostics.as_ref()?;
        Some(crate::diagnostics::SessionDiagnostics {
            simulation: d.simulation.summary(),
            collection: d.collection.summary(),
            replication: d.replication.summary(),
            regions: server.region_diagnostics(),
            queues: self
                .replication
                .as_ref()
                .map(|h| h.diagnostics())
                .unwrap_or_default(),
        })
    }
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
    pub fn tick_sequence(&self) -> u64 {
        self.tick_sequence
    }
    pub fn movement_tick_sequence(&self) -> u64 {
        self.movement_tick_sequence
    }
    pub(crate) fn renew_identity(&mut self) {
        self.frame_inputs.clear();
        self.movement_inputs.clear();
        self.session_id = Uuid::new_v4();
        if let Some(hub) = &mut self.replication {
            hub.clear();
        }
    }
    pub(crate) fn restart_collection(&mut self, interests: EventInterests) {
        self.interests = interests;
        self.tick_sequence = 0;
        self.movement_tick_sequence = 0;
        self.explicit_clock.reset_elapsed();
        self.events.clear();
    }
    pub fn invalidate(&mut self) {
        self.frame_inputs.clear();
        self.movement_inputs.clear();
        self.session_id = Uuid::new_v4();
        self.events.clear();
        if let Some(hub) = &mut self.replication {
            hub.clear();
        }
    }
    pub fn clear_events(&mut self) {
        self.events.clear();
        if let Some(hub) = &mut self.replication {
            hub.clear_events();
        }
    }
    pub fn take_events(&mut self, id: &Uuid) -> RegionEvents {
        if let Some(connection) = self.local_connection
            && let Some(client) = self.client_world_mut(connection)
        {
            return client.take_events(*id);
        }
        self.events.remove(id).unwrap_or_default()
    }
    pub fn pending_events(&mut self, id: &Uuid) -> Option<&mut RegionEvents> {
        if self.replication.is_some() {
            return self
                .client_world_mut(self.local_connection?)
                .and_then(|client| client.pending_events(id));
        }
        self.events.get_mut(id)
    }
    pub(crate) fn enable_replication(&mut self) {
        self.replication = Some(Default::default());
    }
    pub(crate) fn set_replication_policy(
        &mut self,
        policy: crate::ReplicationPolicy,
    ) -> Result<(), ReplicationError> {
        self.replication
            .as_mut()
            .ok_or(ReplicationError::UnknownConnection)?
            .set_policy(policy);
        Ok(())
    }
    pub(crate) fn set_spatial(
        &mut self,
        id: PlayerConnectionId,
        config: Option<crate::replication::SpatialSubscriptions>,
    ) -> Result<(), ReplicationError> {
        self.replication
            .as_mut()
            .ok_or(ReplicationError::UnknownConnection)?
            .set_spatial(id, config)
    }
    pub(crate) fn prefetch(&self, id: PlayerConnectionId) -> Vec<Uuid> {
        self.replication
            .as_ref()
            .map(|hub| hub.prefetch(id).collect())
            .unwrap_or_default()
    }
    pub fn client_world(&self, id: PlayerConnectionId) -> Option<&ClientWorld> {
        self.replication.as_ref()?.client(id)
    }
    pub(crate) fn client_world_mut(&mut self, id: PlayerConnectionId) -> Option<&mut ClientWorld> {
        self.replication.as_mut()?.client_mut(id)
    }
    pub(crate) fn remove_replica(&mut self, id: PlayerConnectionId) {
        self.frame_inputs.remove(id);
        self.movement_inputs.remove(&id);
        if let Some(hub) = &mut self.replication {
            hub.remove(id);
        }
    }
    pub(crate) fn subscribe_replication(
        &mut self,
        id: PlayerConnectionId,
    ) -> Result<(), ReplicationError> {
        self.replication
            .as_mut()
            .ok_or(ReplicationError::UnknownConnection)?
            .subscribe(
                id,
                self.session_id,
                self.tick_sequence,
                self.movement_tick_sequence,
            )
    }
    pub(crate) fn take_client_frames(
        &mut self,
        id: PlayerConnectionId,
    ) -> Result<Vec<ClientFrame>, ReplicationError> {
        self.replication
            .as_mut()
            .ok_or(ReplicationError::UnknownConnection)?
            .take_frames(id)
    }
    pub(crate) fn request_baseline(
        &mut self,
        id: PlayerConnectionId,
    ) -> Result<(), ReplicationError> {
        self.replication
            .as_mut()
            .ok_or(ReplicationError::UnknownConnection)?
            .resync(id)
    }
    pub(crate) fn publish_replication(
        &mut self,
        project: &Project,
        server: &Server,
        current_map: &str,
        fresh: &FxHashMap<Uuid, RegionEvents>,
    ) -> Result<(), ReplicationError> {
        let started = self.diagnostics.as_ref().map(|_| instant::Instant::now());
        self.local_connection = self.replication.as_ref().map(|_| server.local_connection());
        if let Some(hub) = &mut self.replication {
            hub.publish(
                self.session_id,
                self.tick_sequence,
                self.movement_tick_sequence,
                project,
                server,
                current_map,
                fresh,
                self.interests,
            )?;
        }
        if let (Some(started), Some(d)) = (started, &mut self.diagnostics) {
            d.replication.record(started.elapsed());
        }
        Ok(())
    }
    pub(crate) fn submit_movement(
        &mut self,
        server: &Server,
        id: PlayerConnectionId,
        input: crate::MovementInput,
    ) -> Result<crate::MovementSubmission, crate::MovementInputError> {
        use crate::{MovementInputError as E, MovementSubmission as S};
        if self.frame_inputs.enabled(id) {
            return Err(E::NeedsBaseline);
        }
        let state = server.connection_state(id).ok_or(E::UnknownConnection)?;
        let control = state.control().ok_or(E::NotControlled)?;
        let replica = self.client_world(id).ok_or(E::NeedsBaseline)?;
        if replica.generation() != input.generation
            || replica.controlled_entity() != Some(control.entity_id)
        {
            return Err(E::NeedsBaseline);
        }
        if input.sequence == 0 {
            return Err(E::InvalidSequence);
        }
        let grid = replica
            .current_map()
            .and_then(|map| {
                map.entities
                    .iter()
                    .find(|actor| actor.id == control.entity_id)
            })
            .is_some_and(|actor| {
                matches!(
                    actor.attributes.get("player_camera"),
                    Some(rusterix::Value::PlayerCamera(
                        rusterix::PlayerCamera::D2Grid
                            | rusterix::PlayerCamera::D2IsoGrid
                            | rusterix::PlayerCamera::D3FirstPGrid
                    ))
                )
            });
        if self.movement_inputs.get(&id).is_some_and(|last| {
            last.input.generation == input.generation && last.input.sequence >= input.sequence
        }) {
            return Ok(S::IgnoredStale);
        }
        let last =
            self.movement_inputs
                .entry(id)
                .or_insert_with(|| crate::movement::PendingMovement {
                    input: crate::MovementInput {
                        sequence: 0,
                        ..input
                    },
                    pending: false,
                    grid,
                    edges: Default::default(),
                });
        let reset = last.input.generation != input.generation || last.grid != grid;
        let transition = reset || last.input.sequence == 0 || last.input.intent != input.intent;
        if grid
            && transition
            && !reset
            && last.edges.len() >= crate::movement::MAX_QUEUED_MOVEMENT_STATES
        {
            return Err(E::Backpressure);
        }
        if reset {
            last.edges.clear();
        }
        if grid && transition {
            last.edges.push_back(input);
        }
        last.grid = grid;
        last.input = input;
        last.pending = !grid;
        Ok(S::Accepted)
    }
    pub(crate) fn accepted_movement(&self, id: PlayerConnectionId) -> Option<crate::MovementInput> {
        let input = self.movement_inputs.get(&id)?.input;
        (self.client_world(id)?.generation() == input.generation).then_some(input)
    }
    pub(crate) fn enable_frame_input(
        &mut self,
        id: PlayerConnectionId,
        control: crate::PlayerControl,
    ) {
        self.movement_inputs.remove(&id);
        self.frame_inputs.enable(id, control);
    }
    pub(crate) fn frame_input_enabled(&self, id: PlayerConnectionId) -> bool {
        self.frame_inputs.enabled(id)
    }
    pub(crate) fn disable_frame_input(&mut self, id: PlayerConnectionId) {
        self.frame_inputs.remove(id);
    }
    pub(crate) fn submit_input_frame(
        &mut self,
        server: &Server,
        id: PlayerConnectionId,
        command: crate::MovementCommand,
    ) -> Result<crate::MovementSubmission, crate::FrameInputError> {
        let replica = self
            .replication
            .as_ref()
            .and_then(|hub| hub.client(id))
            .ok_or(crate::FrameInputError::UnknownConnection)?;
        self.frame_inputs
            .submit(server, replica, id, self.movement_tick_sequence, command)
    }
    fn flush_movement(&mut self, server: &Server, allow_grid_edge: bool) {
        let replication = &self.replication;
        self.movement_inputs.retain(|id, pending| {
            let current = server.player_control(*id);
            let replica = replication.as_ref().and_then(|hub| hub.client(*id));
            if current.is_none()
                || !replica.is_some_and(|client| {
                    client.generation() == pending.input.generation
                        && client.controlled_entity() == current.map(|control| control.entity_id)
                })
            {
                if current.is_some() {
                    let _ = server.player_action(*id, rusterix::EntityAction::Off);
                }
                return false;
            }
            if pending.grid && allow_grid_edge {
                if let Some(edge) = pending.edges.pop_front() {
                    let _ = server.player_action(*id, edge.intent.action());
                }
            } else if !pending.grid && pending.pending {
                let _ = server.player_action(*id, pending.input.intent.action());
                pending.pending = false;
            }
            true
        });
    }
    pub fn advance(
        &mut self,
        clock: &mut SessionClock,
        elapsed: Duration,
        project: &mut Project,
        server: &mut Server,
        assets: &mut Assets,
        current_map: &mut String,
    ) -> SessionUpdate {
        let started = self.diagnostics.as_ref().map(|_| instant::Instant::now());
        server.prepare_region_schedule(if server.state == ServerState::Running { elapsed } else { Duration::ZERO });
        self.flush_movement(server, false);
        server.redraw_tick();
        let steps = if server.state == ServerState::Running {
            clock.schedule(elapsed)
        } else {
            clock.reset_elapsed();
            vec![]
        };
        let (ticks, movement) = self.run_steps(server, steps);
        if let (Some(started), Some(d)) = (started, &mut self.diagnostics) {
            d.simulation.record(started.elapsed());
        }
        let mut update = self.update(0, true, project, server, assets, current_map);
        update.system_ticks = ticks;
        update.movement_steps = movement;
        update
    }
    fn run_steps(
        &mut self,
        server: &Server,
        steps: Vec<rusterix::simulation::ScheduledStep>,
    ) -> (u32, u32) {
        let (mut ticks, mut movement) = (0, 0);
        for step in steps {
            match step {
                rusterix::simulation::ScheduledStep::Game => {
                    server.system_tick();
                    ticks += 1;
                }
                rusterix::simulation::ScheduledStep::Movement => {
                    self.flush_movement(server, true);
                    server.redraw_tick();
                    self.movement_tick_sequence = self.movement_tick_sequence.saturating_add(1);
                    let resync = self.frame_inputs.step(
                        server,
                        self.replication.as_ref(),
                        self.movement_tick_sequence,
                    );
                    if let Some(hub) = &mut self.replication {
                        for id in resync {
                            let _ = hub.resync(id);
                        }
                    }
                    server.movement_tick();
                    movement += 1;
                }
            }
        }
        self.tick_sequence = self.tick_sequence.saturating_add(u64::from(ticks));
        (ticks, movement)
    }
    /// Explicit stepping/polling and legacy redraw cadence share the same sole
    /// collector. Time is returned to the owning document/runtime policy.
    pub fn update(
        &mut self,
        ticks: u32,
        redraw: bool,
        project: &mut Project,
        server: &mut Server,
        assets: &mut Assets,
        current_map: &mut String,
    ) -> SessionUpdate {
        if redraw {
            server.prepare_region_schedule(Duration::ZERO);
            self.flush_movement(server, false);
            server.redraw_tick();
        }
        let mut issued = (0, 0);
        if ticks > 0 && server.state == ServerState::Running {
            let milliseconds = project
                .config
                .parse::<toml::Table>()
                .ok()
                .and_then(|t| t.get("game")?.get("game_tick_ms")?.as_integer())
                .unwrap_or(250)
                .max(1) as u64;
            let step = Duration::from_millis(milliseconds);
            if self.explicit_clock.step() != step {
                self.explicit_clock = SessionClock::new(step);
            }
            let steps = self.explicit_clock.schedule(step.saturating_mul(ticks));
            issued = self.run_steps(server, steps);
        }
        let started = self.diagnostics.as_ref().map(|_| instant::Instant::now());
        let changed = server.update(assets);
        if let Some(name) = &changed {
            *current_map = name.clone();
        }
        let mut time = None;
        let mut fresh = FxHashMap::default();
        for region in &mut project.regions {
            server.apply_entities_items(&mut region.map);
            if region.map.name == *current_map {
                time = server.get_time(&region.map.id);
            }
            fresh
                .entry(region.map.id)
                .or_insert_with(RegionEvents::default)
                .collect(
                    server,
                    &region.map.id,
                    if self.replication.is_some() {
                        EventInterests::default()
                    } else {
                        self.interests
                    },
                );
        }
        if let (Some(started), Some(d)) = (started, &mut self.diagnostics) {
            d.collection.record(started.elapsed());
        }
        let replication_error = self
            .publish_replication(project, server, current_map, &fresh)
            .err();
        if self.replication.is_none() {
            for (id, events) in fresh {
                self.events.entry(id).or_default().append(events);
            }
        }
        SessionUpdate {
            system_ticks: issued.0,
            movement_steps: issued.1,
            region_changed: changed,
            time,
            replication_error,
        }
    }
}
