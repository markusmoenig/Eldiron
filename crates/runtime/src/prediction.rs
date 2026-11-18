//! Client-only motion and presentation. Authoritative state and addressed
//! events remain in ClientWorld; replay changes only a separate actor copy.
use crate::{
    ClientFrame, ClientWorld, MovementCommand, MovementIntent, Project, RegionEvents, RuntimeView,
    replication::{ApplyResult, RegionContentRef, ReplicationError, WorldUpdate},
};
use rustc_hash::FxHashMap;
use rusterix::{
    Assets, Entity, Map, ServerState,
    collision_world::CollisionWorld,
    map::mini::MapMini,
    movement::{ControllerState, MovementWorld, motion_revision, step_controller},
};
use std::{collections::VecDeque, time::Duration};
use theframework_core::{
    Uuid,
    prelude::{Vec2, Vec3},
};

pub const MAX_PREDICTION_FRAMES: usize = 128;
pub const MAX_REMOTE_SAMPLES: usize = 32;
#[derive(Clone, Copy, Debug)]
pub struct MotionSettings {
    pub interpolation_delay: Duration,
    pub correction_seconds: f32,
    pub snap_distance: f32,
}
impl Default for MotionSettings {
    fn default() -> Self {
        Self {
            interpolation_delay: Duration::from_millis(100),
            correction_seconds: 0.08,
            snap_distance: 2.,
        }
    }
}
impl MotionSettings {
    /// No artificial latency for a co-located single-player/Creator client.
    pub fn embedded() -> Self {
        Self {
            interpolation_delay: Duration::ZERO,
            ..Self::default()
        }
    }
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct MotionDiagnostics {
    pub predicted_steps: u64,
    pub replayed_steps: u64,
    pub corrections: u64,
    pub hard_resets: u64,
    pub pose_snaps: u64,
    pub last_correction_distance: f64,
    pub max_correction_distance: f64,
    pub total_correction_distance: f64,
    pub pending_frames: usize,
    pub queued_edges: usize,
}
#[derive(Clone, Copy)]
struct Pose {
    position: Vec3<f32>,
    orientation: Vec2<f32>,
    tilt: f32,
    revision: u32,
}
impl Pose {
    fn capture(actor: &Entity) -> Self {
        Self {
            position: actor.position,
            orientation: actor.orientation,
            tilt: actor.tilt,
            revision: motion_revision(actor),
        }
    }
    fn apply(self, actor: &mut Entity) {
        actor.position = self.position;
        actor.orientation = self.orientation;
        actor.tilt = self.tilt;
    }
    fn between(a: Self, b: Self, t: f32) -> Self {
        let start = a.orientation.y.atan2(a.orientation.x);
        let end = b.orientation.y.atan2(b.orientation.x);
        let angle = start
            + ((end - start + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI)
                * t;
        Self {
            position: a.position + (b.position - a.position) * t,
            orientation: Vec2::new(angle.cos(), angle.sin()),
            tilt: a.tilt + (b.tilt - a.tilt) * t,
            revision: b.revision,
        }
    }
}
struct Sample {
    tick: u64,
    pose: Pose,
}
struct CollisionRegion {
    mini: MapMini,
    world: CollisionWorld,
    props: Vec<rusterix::BlockPropInstance>,
    initialized: bool,
    authoritative: bool,
}
/// A remote-capable CPU client. It owns both the exact authority replica and a
/// presentation map used uniformly by camera, HUD, picking and scene drawing.
pub struct MotionClient {
    authority: ClientWorld,
    settings: MotionSettings,
    collisions: FxHashMap<RegionContentRef, CollisionRegion>,
    presented: FxHashMap<Uuid, Map>,
    shown_refs: FxHashMap<Uuid, RegionContentRef>,
    samples: FxHashMap<(Uuid, u32), VecDeque<Sample>>,
    actor: Option<Entity>,
    controller: ControllerState,
    history: VecDeque<MovementCommand>,
    next_sequence: u64,
    intent: MovementIntent,
    edges: VecDeque<MovementIntent>,
    yaw: f32,
    pending: Duration,
    server_clock: f64,
    offset: Vec3<f32>,
    angle_offset: f32,
    tilt_offset: f32,
    needs_baseline: bool,
    suspended: bool,
    view_id: Uuid,
    diagnostics: MotionDiagnostics,
}
impl MotionClient {
    pub fn new(world: Uuid, connection: Uuid, settings: MotionSettings) -> Self {
        let settings = MotionSettings {
            interpolation_delay: settings.interpolation_delay.min(Duration::from_millis(500)),
            correction_seconds: if settings.correction_seconds.is_finite() {
                settings.correction_seconds.clamp(0.001, 1.)
            } else {
                0.08
            },
            snap_distance: if settings.snap_distance.is_finite() {
                settings.snap_distance.clamp(0.1, 100.)
            } else {
                2.
            },
        };
        Self {
            authority: ClientWorld::new(world, connection),
            settings,
            collisions: Default::default(),
            presented: Default::default(),
            shown_refs: Default::default(),
            samples: Default::default(),
            actor: None,
            controller: Default::default(),
            history: Default::default(),
            next_sequence: 1,
            intent: MovementIntent::Stop,
            edges: Default::default(),
            yaw: 0.,
            pending: Duration::ZERO,
            server_clock: 0.,
            offset: Vec3::zero(),
            angle_offset: 0.,
            tilt_offset: 0.,
            needs_baseline: false,
            suspended: false,
            view_id: Uuid::new_v4(),
            diagnostics: Default::default(),
        }
    }
    pub fn authority(&self) -> &ClientWorld {
        &self.authority
    }
    pub fn diagnostics(&self) -> MotionDiagnostics {
        let mut diagnostics = self.diagnostics;
        diagnostics.pending_frames = self.history.len();
        diagnostics.queued_edges = self.edges.len();
        diagnostics
    }
    pub fn needs_baseline(&self) -> bool {
        self.needs_baseline
    }
    pub fn pending_inputs(&self) -> Vec<MovementCommand> {
        self.history.iter().copied().collect()
    }
    pub fn set_intent(&mut self, intent: MovementIntent) -> bool {
        if self.intent != intent && self.actor.as_ref().is_some_and(rusterix::movement::is_grid) {
            if self.edges.len() == crate::movement::MAX_QUEUED_MOVEMENT_STATES {
                self.reset(true);
                return false;
            }
            self.edges.push_back(intent);
        }
        self.intent = intent;
        true
    }
    pub fn look(&mut self, degrees: f32) {
        if degrees.is_finite() {
            self.yaw = (self.yaw + degrees).clamp(-180., 180.);
        }
    }
    pub fn take_events(&mut self, region: Uuid) -> RegionEvents {
        self.authority.take_events(region)
    }
    pub fn take_event_overflow(&mut self) -> bool {
        self.authority.take_event_overflow()
    }
    pub fn install_content(
        &mut self,
        reference: RegionContentRef,
        map: &Map,
    ) -> Result<(), ReplicationError> {
        self.authority.install_content(reference, map)
    }
    /// Collision data is independently provisioned CPU content. Matching content
    /// is required before prediction, including meshes/elevation for that revision.
    pub fn install_collision(
        &mut self,
        reference: RegionContentRef,
        mini: MapMini,
        world: CollisionWorld,
    ) {
        self.collisions
            .retain(|key, _| key.map != reference.map || *key == reference);
        self.collisions.insert(
            reference,
            CollisionRegion {
                mini,
                world,
                props: vec![],
                initialized: false,
                authoritative: false,
            },
        );
    }
    /// Server-compiled physics must not be rebuilt from sanitized visual props.
    /// A later content hint replaces this collision snapshot atomically.
    pub fn install_authoritative_collision(
        &mut self,
        reference: RegionContentRef,
        mini: MapMini,
        world: CollisionWorld,
    ) {
        self.install_collision(reference, mini, world);
        let cache = self.collisions.get_mut(&reference).unwrap();
        cache.authoritative = true;
        cache.initialized = true;
    }
    fn can_send(&self) -> bool {
        !self.needs_baseline
            && !self.suspended
            && self.authority.state() == ServerState::Running
            && self.authority.motion().is_some()
            && self
                .authority
                .current_region()
                .and_then(|id| self.authority.content_reference(id))
                .is_some_and(|r| self.collisions.contains_key(&r))
            && self.actor.is_some()
    }
    pub fn ready(&self) -> bool {
        self.can_send() && self.authority.motion().is_some_and(|m| m.predictable)
    }
    fn reset(&mut self, needs_baseline: bool) {
        self.history.clear();
        self.edges.clear();
        self.pending = Duration::ZERO;
        self.yaw = 0.;
        self.intent = MovementIntent::Stop;
        self.offset = Vec3::zero();
        self.angle_offset = 0.;
        self.tilt_offset = 0.;
        self.view_id = Uuid::new_v4();
        self.needs_baseline = needs_baseline;
        self.actor = self
            .authority
            .current_map()
            .and_then(|map| {
                map.entities
                    .iter()
                    .find(|e| Some(e.id) == self.authority.controlled_entity())
            })
            .map(motion_actor);
        self.controller = self
            .authority
            .motion()
            .map(|m| m.controller.clone())
            .unwrap_or_default();
        if let Some(map) = self
            .authority
            .current_region()
            .and_then(|id| self.presented.get_mut(&id))
            && let Some(actor) = &self.actor
            && let Some(shown) = map.entities.iter_mut().find(|e| e.id == actor.id)
        {
            Pose::capture(actor).apply(shown);
        }
        self.diagnostics.hard_resets = self.diagnostics.hard_resets.saturating_add(1);
    }
    /// Suspension releases local controls without pausing a shared server.
    /// The transport requests a fresh baseline before resuming this client.
    pub fn suspend(&mut self) {
        self.suspended = true;
        self.reset(true);
        self.samples.clear();
    }
    pub fn resume(&mut self) {
        self.suspended = false;
        self.reset(true);
    }

    pub fn receive(
        &mut self,
        frame: &ClientFrame,
        assets: &Assets,
    ) -> Result<ApplyResult, ReplicationError> {
        let fresh = self.authority.generation() == 0
            || frame.generation != self.authority.generation()
            || matches!(frame.update, WorldUpdate::Baseline(_));
        if !fresh
            && frame
                .motion
                .as_ref()
                .is_some_and(|m| m.processed_sequence >= self.next_sequence)
        {
            return Err(ReplicationError::InvalidFrame(
                "Acknowledgment exceeds submitted input".into(),
            ));
        }
        let old_actor = self.actor.as_ref().map(Pose::capture);
        let outcome = match self.authority.apply(frame) {
            Ok(outcome) => outcome,
            Err(error) => {
                if matches!(
                    error,
                    ReplicationError::NeedsBaseline | ReplicationError::MissingContent(_)
                ) {
                    self.reset(true);
                }
                return Err(error);
            }
        };
        if outcome == ApplyResult::IgnoredStale {
            return Ok(outcome);
        }
        self.server_clock = self
            .server_clock
            .max(frame.movement_tick as f64 * rusterix::simulation::MOVEMENT_STEP.as_secs_f64());
        if fresh {
            self.server_clock =
                frame.movement_tick as f64 * rusterix::simulation::MOVEMENT_STEP.as_secs_f64();
            self.reset(false);
            self.samples.clear();
            self.next_sequence = frame
                .motion
                .as_ref()
                .map(|m| m.processed_sequence.saturating_add(1))
                .unwrap_or(1);
            self.collisions
                .retain(|key, _| self.authority.content_reference(key.map) == Some(*key));
        }
        self.refresh_collision(assets);
        self.capture_samples();
        self.sync_presented();
        if !fresh {
            let ack = self
                .authority
                .motion()
                .map(|m| m.processed_sequence)
                .unwrap_or(0);
            while self
                .history
                .front()
                .is_some_and(|input| input.sequence <= ack)
            {
                self.history.pop_front();
            }
            self.actor = self
                .authority
                .current_map()
                .and_then(|map| {
                    map.entities
                        .iter()
                        .find(|e| Some(e.id) == self.authority.controlled_entity())
                })
                .map(motion_actor);
            self.controller = self
                .authority
                .motion()
                .map(|m| m.controller.clone())
                .unwrap_or_default();
            let replay: Vec<_> = self.history.iter().copied().collect();
            if self.ready() {
                for command in replay {
                    self.step(command, assets);
                    self.diagnostics.replayed_steps =
                        self.diagnostics.replayed_steps.saturating_add(1);
                }
            }
            if let (Some(old), Some(actor)) = (old_actor, self.actor.as_ref()) {
                let error = old.position - actor.position;
                if old.revision != motion_revision(actor)
                    || error.magnitude() > self.settings.snap_distance
                {
                    let distance = ((error.x as f64).powi(2)
                        + (error.y as f64).powi(2)
                        + (error.z as f64).powi(2))
                    .sqrt();
                    self.diagnostics.last_correction_distance = distance;
                    self.diagnostics.max_correction_distance =
                        self.diagnostics.max_correction_distance.max(distance);
                    self.diagnostics.total_correction_distance += distance;
                    self.diagnostics.pose_snaps = self.diagnostics.pose_snaps.saturating_add(1);
                    self.reset(true);
                } else {
                    let angle = wrap_angle(
                        old.orientation.y.atan2(old.orientation.x)
                            - actor.orientation.y.atan2(actor.orientation.x),
                    );
                    if error.magnitude_squared() > 1e-8
                        || angle.abs() > 1e-5
                        || (old.tilt - actor.tilt).abs() > 1e-5
                    {
                        let distance = ((error.x as f64).powi(2)
                            + (error.y as f64).powi(2)
                            + (error.z as f64).powi(2))
                        .sqrt();
                        self.diagnostics.last_correction_distance = distance;
                        self.diagnostics.max_correction_distance =
                            self.diagnostics.max_correction_distance.max(distance);
                        self.diagnostics.total_correction_distance += distance;
                        self.offset += error;
                        self.angle_offset = if angle.abs() < std::f32::consts::FRAC_PI_2 {
                            wrap_angle(self.angle_offset + angle)
                        } else {
                            0.
                        };
                        self.tilt_offset += old.tilt - actor.tilt;
                        self.diagnostics.corrections =
                            self.diagnostics.corrections.saturating_add(1);
                    }
                }
            }
        }
        self.compose(assets);
        Ok(outcome)
    }

    pub fn advance(&mut self, elapsed: Duration, assets: &Assets) {
        if self.suspended {
            return;
        }
        // A package can finish loading after its baseline was applied. Bring
        // its prop collision to the current authority before the first step.
        if self.collisions.values().any(|cache| !cache.initialized) {
            self.refresh_collision(assets);
        }
        self.server_clock += elapsed.as_secs_f64();
        let decay = (-elapsed.as_secs_f32() / self.settings.correction_seconds).exp();
        self.offset *= decay;
        self.angle_offset *= decay;
        self.tilt_offset *= decay;
        if self.can_send() {
            self.pending = self.pending.saturating_add(elapsed);
            let step = rusterix::simulation::MOVEMENT_STEP;
            if self.pending.as_nanos() / step.as_nanos() + self.history.len() as u128
                > MAX_PREDICTION_FRAMES as u128
            {
                self.reset(true);
            } else {
                while self.pending >= step {
                    self.pending -= step;
                    let command = MovementCommand {
                        generation: self.authority.generation(),
                        sequence: self.next_sequence,
                        intent: self.edges.pop_front().unwrap_or(self.intent),
                        yaw: self.yaw,
                    };
                    self.yaw = 0.;
                    let Some(next) = self.next_sequence.checked_add(1) else {
                        self.reset(true);
                        break;
                    };
                    self.next_sequence = next;
                    self.step(command, assets);
                    self.history.push_back(command);
                    self.diagnostics.predicted_steps =
                        self.diagnostics.predicted_steps.saturating_add(1);
                }
            }
        } else {
            self.pending = Duration::ZERO;
        }
        self.compose(assets);
    }
    fn step(&mut self, command: MovementCommand, assets: &Assets) {
        if !self.ready() {
            return;
        }
        let Some(map) = self.authority.current_map() else {
            return;
        };
        let Some(reference) = self.authority.content_reference(map.id) else {
            return;
        };
        let Some(collision) = self.collisions.get(&reference) else {
            return;
        };
        let Some(motion) = self.authority.motion() else {
            return;
        };
        if let Some(actor) = &mut self.actor {
            step_controller(
                &MovementWorld {
                    diagnostics: None,
                    map,
                    mapmini: &collision.mini,
                    collision_world: &collision.world,
                    assets,
                    mode: motion.parameters.collision_mode,
                },
                actor,
                &mut self.controller,
                motion.parameters,
                command,
            );
        }
    }
    fn capture_samples(&mut self) {
        let mut live = std::collections::HashSet::new();
        for map in self.authority.maps() {
            for actor in &map.entities {
                let key = (map.id, actor.id);
                live.insert(key);
                let pose = Pose::capture(actor);
                let samples = self.samples.entry(key).or_default();
                if samples.back().is_some_and(|old| {
                    old.pose.revision != pose.revision
                        || (old.pose.position - pose.position).magnitude()
                            > self.settings.snap_distance
                }) {
                    samples.clear();
                }
                if samples
                    .back()
                    .is_some_and(|old| old.tick == self.authority.movement_tick())
                {
                    samples.pop_back();
                }
                samples.push_back(Sample {
                    tick: self.authority.movement_tick(),
                    pose,
                });
                while samples.len() > MAX_REMOTE_SAMPLES {
                    samples.pop_front();
                }
            }
        }
        self.samples.retain(|key, _| live.contains(key));
    }
    fn compose(&mut self, assets: &Assets) {
        let target = self.server_clock - self.settings.interpolation_delay.as_secs_f64();
        let tick_seconds = rusterix::simulation::MOVEMENT_STEP.as_secs_f64();
        let mut preview = self.actor.clone();
        let animate = self.ready();
        let mut walking = false;
        if self.ready()
            && let Some(actor) = &mut preview
        {
            let start = Pose::capture(actor);
            let mut controller = self.controller.clone();
            if self.yaw != 0.
                && self.authority.motion().unwrap().parameters.camera
                    == rusterix::PlayerCamera::D3FirstPMouse
            {
                actor.turn_right(self.yaw);
            }
            let yaw_start = Pose::capture(actor);
            let map = self.authority.current_map().unwrap();
            let reference = self.authority.content_reference(map.id).unwrap();
            let collision = &self.collisions[&reference];
            let parameters = self.authority.motion().unwrap().parameters;
            let sample = step_controller(
                &MovementWorld {
                    diagnostics: None,
                    map,
                    mapmini: &collision.mini,
                    collision_world: &collision.world,
                    assets,
                    mode: parameters.collision_mode,
                },
                actor,
                &mut controller,
                parameters,
                MovementCommand {
                    generation: self.authority.generation(),
                    sequence: self.next_sequence,
                    intent: self.edges.front().copied().unwrap_or(self.intent),
                    yaw: 0.,
                },
            );
            walking = sample.moved;
            let mut pose = Pose::between(
                yaw_start,
                Pose::capture(actor),
                self.pending.as_secs_f32() / rusterix::simulation::MOVEMENT_STEP.as_secs_f32(),
            );
            if self.pending.is_zero() {
                pose.position = start.position;
            }
            pose.position += self.offset;
            let angle = pose.orientation.y.atan2(pose.orientation.x) + self.angle_offset;
            pose.orientation = Vec2::new(angle.cos(), angle.sin());
            pose.tilt += self.tilt_offset;
            pose.apply(actor);
        }
        for shown in self.presented.values_mut() {
            let id = shown.id;
            for actor in &mut shown.entities {
                if Some(actor.id) == self.authority.controlled_entity()
                    && Some(id) == self.authority.current_region()
                {
                    if let Some(predicted) = &preview {
                        Pose::capture(predicted).apply(actor);
                        if animate
                            && actor.attributes.get_float_default("avatar_attack_left", 0.) <= 0.
                            && !actor.attributes.get_bool_default("spell_casting", false)
                        {
                            actor.attributes.set(
                                "avatar_animation",
                                rusterix::Value::Str(if walking { "Walk" } else { "Idle" }.into()),
                            );
                        }
                    }
                    continue;
                }
                if let Some(samples) = self.samples.get(&(id, actor.id)) {
                    let mut pose = samples.front().map(|s| s.pose);
                    for (index, sample) in samples.iter().enumerate() {
                        if sample.tick as f64 * tick_seconds <= target {
                            pose = Some(sample.pose);
                            continue;
                        }
                        if index > 0 {
                            let before = &samples[index - 1];
                            let t = ((target - before.tick as f64 * tick_seconds)
                                / ((sample.tick - before.tick) as f64 * tick_seconds))
                                .clamp(0., 1.) as f32;
                            pose = Some(Pose::between(before.pose, sample.pose, t));
                        }
                        break;
                    }
                    if let Some(pose) = pose {
                        pose.apply(actor);
                    }
                }
            }
        }
    }
    fn sync_presented(&mut self) {
        self.presented
            .retain(|id, _| self.authority.map(*id).is_some());
        self.shown_refs
            .retain(|id, _| self.authority.map(*id).is_some());
        for map in self.authority.maps() {
            let reference = self.authority.content_reference(map.id).unwrap();
            if self.shown_refs.get(&map.id) != Some(&reference) {
                self.presented.insert(map.id, map.clone());
                self.shown_refs.insert(map.id, reference);
            } else if let Some(shown) = self.presented.get_mut(&map.id) {
                shown.entities.clone_from(&map.entities);
                shown.items.clone_from(&map.items);
                shown
                    .block_prop_instances
                    .clone_from(&map.block_prop_instances);
            }
        }
    }
    pub fn view<'a>(
        &'a self,
        project: &'a Project,
        assets: &'a Assets,
        region: Option<Uuid>,
    ) -> Option<RuntimeView<'a>> {
        let mut view = self.authority.view(project, assets, region)?;
        view.map = self.presented.get(&view.map?.id).or(view.map);
        view.session_id = self.view_id;
        Some(view)
    }

    fn refresh_collision(&mut self, assets: &Assets) {
        use rusterix::chunkbuilder::{ChunkBuilder, d3chunkbuilder::D3ChunkBuilder};
        for map in self.authority.maps() {
            let Some(reference) = self.authority.content_reference(map.id) else {
                continue;
            };
            let Some(cache) = self.collisions.get_mut(&reference) else {
                continue;
            };
            if cache.authoritative || cache.initialized && cache.props == map.block_prop_instances {
                continue;
            }
            let mut chunks = std::collections::BTreeSet::new();
            for props in [&cache.props, &map.block_prop_instances] {
                let resolved = rusterix::resolve_block_prop_geometry(props, &assets.block_props);
                for object in resolved.geometry_objects {
                    if let Some(bbox) = object.bbox() {
                        for x in
                            (bbox.min.x / 10.).floor() as i32..=(bbox.max.x / 10.).floor() as i32
                        {
                            for z in (bbox.min.y / 10.).floor() as i32
                                ..=(bbox.max.y / 10.).floor() as i32
                            {
                                chunks.insert((x, z));
                            }
                        }
                    }
                }
            }
            let mut builder = D3ChunkBuilder::new();
            for (x, z) in chunks {
                cache.world.update_chunk(
                    Vec2::new(x, z),
                    builder.build_collision(map, assets, Vec2::new(x, z), 10),
                );
            }
            cache.props = map.block_prop_instances.clone();
            cache.initialized = true;
        }
    }
}

fn motion_actor(source: &Entity) -> Entity {
    Entity {
        id: source.id,
        creator_id: source.creator_id,
        position: source.position,
        orientation: source.orientation,
        tilt: source.tilt,
        attributes: source.attributes.clone(),
        ..Entity::default()
    }
}

fn wrap_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}
