//! Fixed-step player locomotion used by both authoritative frame input and
//! client replay. Contacts are data; this code never executes gameplay events.
use super::*;
use crate::{EntityAction, PlayerCamera, Value};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementIntent {
    #[default]
    Stop,
    Forward,
    Backward,
    Left,
    Right,
    StrafeLeft,
    StrafeRight,
    ForwardLeft,
    ForwardRight,
    BackwardLeft,
    BackwardRight,
}
impl MovementIntent {
    pub fn from_action(action: &EntityAction) -> Option<Self> {
        use EntityAction as A;
        Some(match action {
            A::Off => Self::Stop,
            A::Forward => Self::Forward,
            A::Backward => Self::Backward,
            A::Left => Self::Left,
            A::Right => Self::Right,
            A::StrafeLeft => Self::StrafeLeft,
            A::StrafeRight => Self::StrafeRight,
            A::ForwardLeft => Self::ForwardLeft,
            A::ForwardRight => Self::ForwardRight,
            A::BackwardLeft => Self::BackwardLeft,
            A::BackwardRight => Self::BackwardRight,
            _ => return None,
        })
    }
    pub fn action(self) -> EntityAction {
        use EntityAction as A;
        match self {
            Self::Stop => A::Off,
            Self::Forward => A::Forward,
            Self::Backward => A::Backward,
            Self::Left => A::Left,
            Self::Right => A::Right,
            Self::StrafeLeft => A::StrafeLeft,
            Self::StrafeRight => A::StrafeRight,
            Self::ForwardLeft => A::ForwardLeft,
            Self::ForwardRight => A::ForwardRight,
            Self::BackwardLeft => A::BackwardLeft,
            Self::BackwardRight => A::BackwardRight,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MovementCommand {
    pub generation: u64,
    pub sequence: u64,
    pub intent: MovementIntent,
    /// Mouse yaw in degrees, bounded to one half-turn per fixed input frame.
    pub yaw: f32,
}
impl MovementCommand {
    pub fn valid(&self) -> bool {
        self.generation > 0 && self.sequence > 0 && self.yaw.is_finite() && self.yaw.abs() <= 180.
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionParameters {
    pub camera: PlayerCamera,
    pub units_per_second: f32,
    pub turn_degrees_per_second: f32,
    pub entity_block_mode: i32,
    pub collision_mode: CollisionMode,
    /// Minimum time between grid turn starts; zero for realtime worlds.
    pub turn_period: f32,
}
impl MotionParameters {
    pub fn valid(&self) -> bool {
        [
            self.units_per_second,
            self.turn_degrees_per_second,
            self.turn_period,
        ]
        .iter()
        .all(|v| v.is_finite() && *v >= 0.)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GridMotion {
    Walk { target: [f32; 2], speed: f32 },
    Turn { target: [f32; 2] },
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControllerState {
    pub grid: Option<GridMotion>,
    pub held: MovementIntent,
    pub blocked: Option<MovementIntent>,
    pub turn_left: f32,
}
impl ControllerState {
    pub fn valid(&self) -> bool {
        self.turn_left.is_finite()
            && self.turn_left >= 0.
            && match &self.grid {
                None => true,
                Some(GridMotion::Walk { target, speed }) => {
                    target.iter().all(|v| v.is_finite()) && speed.is_finite() && *speed > 0.
                }
                Some(GridMotion::Turn { target }) => target.iter().all(|v| v.is_finite()),
            }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionSnapshot {
    pub generation: u64,
    /// Last frame actually simulated, not merely received by the host.
    pub processed_sequence: u64,
    pub revision: u32,
    pub controller: ControllerState,
    pub parameters: MotionParameters,
    pub predictable: bool,
}
pub struct ControllerStep {
    pub contacts: Vec<Contact>,
    pub moved: bool,
    pub turn_started: bool,
}

pub fn motion_revision(actor: &Entity) -> u32 {
    actor.attributes.get_uint_default("__motion_revision", 0)
}
pub fn mark_discontinuity(actor: &mut Entity) {
    let revision = motion_revision(actor).wrapping_add(1);
    actor.set_attribute("__motion_revision", Value::UInt(revision));
}
fn camera(actor: &Entity) -> PlayerCamera {
    match actor.attributes.get("player_camera") {
        Some(Value::PlayerCamera(camera)) => *camera,
        _ => PlayerCamera::D2,
    }
}
pub fn is_grid(actor: &Entity) -> bool {
    matches!(
        camera(actor),
        PlayerCamera::D2Grid | PlayerCamera::D2IsoGrid | PlayerCamera::D3FirstPGrid
    )
}
fn first_person(camera: PlayerCamera) -> bool {
    matches!(
        camera,
        PlayerCamera::D3FirstP | PlayerCamera::D3FirstPMouse | PlayerCamera::D3FirstPGrid
    )
}
fn cardinal(v: Vec2<f32>) -> Vec2<f32> {
    if v.x.abs() > v.y.abs() {
        Vec2::new(if v.x >= 0. { 1. } else { -1. }, 0.)
    } else {
        Vec2::new(0., if v.y >= 0. { 1. } else { -1. })
    }
}
fn direction(intent: MovementIntent, camera: PlayerCamera) -> Option<Vec2<f32>> {
    let action = if camera.is_isometric_2d() {
        crate::isometric::movement(&intent.action())
    } else {
        intent.action()
    };
    let v = match action {
        EntityAction::Forward => Vec2::new(0., -1.),
        EntityAction::Backward => Vec2::new(0., 1.),
        EntityAction::Left => Vec2::new(-1., 0.),
        EntityAction::Right => Vec2::new(1., 0.),
        // Preserve the engine's existing diagonal key/facing convention.
        EntityAction::ForwardLeft => Vec2::new(-1., 1.),
        EntityAction::ForwardRight => Vec2::new(1., 1.),
        EntityAction::BackwardLeft => Vec2::new(-1., -1.),
        EntityAction::BackwardRight => Vec2::new(1., -1.),
        _ => return None,
    };
    Some(v.normalized())
}
fn rotate_towards(actor: &mut Entity, target: Vec2<f32>, degrees: f32) -> bool {
    let start = actor.orientation.y.atan2(actor.orientation.x);
    let end = target.y.atan2(target.x);
    let delta = (end - start + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    if delta.abs() <= degrees.to_radians() {
        actor.set_orientation(target);
        true
    } else {
        let angle = start + degrees.to_radians() * delta.signum();
        actor.set_orientation(Vec2::new(angle.cos(), angle.sin()));
        false
    }
}

/// One constant-duration input frame. Replay never repeats spells, inventory
/// actions, turn callbacks, scripts, sector notifications or output delivery.
pub fn step_controller(
    world: &MovementWorld<'_>,
    actor: &mut Entity,
    state: &mut ControllerState,
    parameters: MotionParameters,
    command: MovementCommand,
) -> ControllerStep {
    let mut result = ControllerStep {
        contacts: vec![],
        moved: false,
        turn_started: false,
    };
    if !command.valid() || !parameters.valid() || actor.get_mode() == "dead" {
        return result;
    }
    let dt = crate::simulation::MOVEMENT_STEP.as_secs_f32();
    let original = actor.position;
    state.turn_left = (state.turn_left - dt).max(0.);
    if command.intent != state.held {
        state.blocked = None;
    }
    let pressed = state.held == MovementIntent::Stop;
    state.held = command.intent;
    let camera = camera(actor);
    // Directional intents remain authoritative gameplay commands, not replayable locomotion.
    if !first_person(camera)
        && actor
            .attributes
            .get_str("intent")
            .is_some_and(|intent| !intent.is_empty())
    {
        state.grid = None;
        return result;
    }
    if camera == PlayerCamera::D3FirstPMouse && command.yaw != 0. {
        actor.turn_right(command.yaw);
    }
    if is_grid(actor) {
        if state.grid.is_none()
            && state.turn_left <= 0.
            && command.intent != MovementIntent::Stop
            && state.blocked != Some(command.intent)
        {
            let facing = cardinal(actor.orientation);
            let (target, rotate, new_facing) = if first_person(camera) {
                match command.intent {
                    MovementIntent::Left => (Vec2::new(facing.y, -facing.x), true, facing),
                    MovementIntent::Right => (Vec2::new(-facing.y, facing.x), true, facing),
                    MovementIntent::Forward => (actor.get_pos_xz() + facing, false, facing),
                    MovementIntent::Backward => (actor.get_pos_xz() - facing, false, facing),
                    MovementIntent::StrafeLeft => (
                        actor.get_pos_xz() + Vec2::new(facing.y, -facing.x),
                        false,
                        facing,
                    ),
                    MovementIntent::StrafeRight => (
                        actor.get_pos_xz() + Vec2::new(-facing.y, facing.x),
                        false,
                        facing,
                    ),
                    _ => (actor.get_pos_xz(), false, facing),
                }
            } else {
                let step = if camera.is_isometric_2d() {
                    crate::isometric::grid_step(&command.intent.action())
                } else {
                    direction(command.intent, camera)
                };
                let step = step.unwrap_or(Vec2::zero());
                (
                    actor.get_pos_xz() + step,
                    false,
                    if step.magnitude_squared() > 0. {
                        step.normalized()
                    } else {
                        facing
                    },
                )
            };
            if rotate {
                state.grid = Some(GridMotion::Turn {
                    target: target.into_array(),
                });
            } else if target.distance(actor.get_pos_xz()) > 0.001 {
                actor.set_orientation(new_facing);
                let speed = if pressed {
                    actor.attributes.get_float_default("speed", 1.)
                } else {
                    actor.attributes.get_float_default(
                        "hold_speed",
                        actor.attributes.get_float_default("speed", 1.),
                    )
                };
                state.grid = Some(GridMotion::Walk {
                    target: (target.map(|v| v.floor()) + Vec2::broadcast(0.5)).into_array(),
                    speed: speed.max(0.01),
                });
            }
            if state.grid.is_some() {
                result.turn_started = true;
                state.turn_left = parameters.turn_period;
            }
        }
        if let Some(motion) = state.grid.clone() {
            match motion {
                GridMotion::Turn { target } => {
                    if rotate_towards(
                        actor,
                        target.into(),
                        parameters.turn_degrees_per_second * dt,
                    ) {
                        state.grid = None;
                    }
                }
                GridMotion::Walk { target, speed } => {
                    let target: Vec2<f32> = target.into();
                    let delta = target - actor.get_pos_xz();
                    let distance = delta.magnitude();
                    if distance < 0.001 {
                        state.grid = None;
                    } else {
                        let before = actor.position;
                        let collision = move_by(
                            world,
                            actor,
                            delta / distance
                                * distance.min(parameters.units_per_second * speed * dt),
                            parameters.entity_block_mode,
                        );
                        result.contacts.extend(collision.contacts);
                        if actor.get_pos_xz().distance(target) <= 0.001 {
                            state.grid = None;
                        } else if (actor.position - before).magnitude_squared() < 1e-8
                            || collision.geometry_blocked
                        {
                            state.grid = None;
                            state.blocked = Some(command.intent);
                        }
                    }
                }
            }
        }
    } else {
        state.grid = None;
        let mut vector = Vec2::zero();
        if first_person(camera) {
            let turn = match command.intent {
                MovementIntent::Left
                | MovementIntent::ForwardLeft
                | MovementIntent::BackwardLeft => -1.,
                MovementIntent::Right
                | MovementIntent::ForwardRight
                | MovementIntent::BackwardRight => 1.,
                _ => 0.,
            };
            actor.turn_right(parameters.turn_degrees_per_second * dt * turn);
            let forward = actor.orientation.try_normalized().unwrap_or(Vec2::unit_x());
            vector = match command.intent {
                MovementIntent::Forward
                | MovementIntent::ForwardLeft
                | MovementIntent::ForwardRight => forward,
                MovementIntent::Backward
                | MovementIntent::BackwardLeft
                | MovementIntent::BackwardRight => -forward,
                MovementIntent::StrafeLeft => Vec2::new(forward.y, -forward.x),
                MovementIntent::StrafeRight => Vec2::new(-forward.y, forward.x),
                _ => vector,
            };
        } else if let Some(v) = direction(command.intent, camera) {
            actor.set_orientation(v);
            vector = v;
        }
        if vector.magnitude_squared() > 0. {
            let collision = move_by(
                world,
                actor,
                vector * parameters.units_per_second * dt,
                parameters.entity_block_mode,
            );
            result.contacts = collision.contacts;
        }
        result.turn_started = pressed && command.intent != MovementIntent::Stop;
    }
    result.moved = (actor.position - original).magnitude_squared() > 1e-8;
    result
}
