//! Bounded fixed input frames. Host clock steps, not packet count, consume them.
use crate::{ClientWorld, PlayerConnectionId, PlayerControl};
use rustc_hash::FxHashMap;
use rusterix::{Server, movement::MovementCommand};
use std::collections::VecDeque;

pub const MAX_INPUT_FRAMES: usize = 32;
pub const INPUT_EXPIRY_TICKS: u64 = 25;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameInputError {
    UnknownConnection,
    NotEnabled,
    NeedsBaseline,
    InvalidInput,
    MissingSequence { expected: u64 },
    Backpressure,
    Paused,
}
impl std::fmt::Display for FrameInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for FrameInputError {}
#[derive(Default)]
pub(crate) struct FrameInputs {
    streams: FxHashMap<PlayerConnectionId, Stream>,
}
struct Stream {
    generation: u64,
    received: u64,
    revision: u32,
    control: PlayerControl,
    queue: VecDeque<(u64, MovementCommand)>,
    last_tick: u64,
    moving: bool,
}
impl FrameInputs {
    pub fn enable(&mut self, id: PlayerConnectionId, control: PlayerControl) {
        self.streams.insert(
            id,
            Stream {
                generation: 0,
                received: 0,
                revision: 0,
                control,
                queue: Default::default(),
                last_tick: 0,
                moving: false,
            },
        );
    }
    pub fn enabled(&self, id: PlayerConnectionId) -> bool {
        self.streams.contains_key(&id)
    }
    pub fn clear(&mut self) {
        self.streams.clear();
    }
    pub fn remove(&mut self, id: PlayerConnectionId) {
        self.streams.remove(&id);
    }
    pub fn submit(
        &mut self,
        server: &Server,
        replica: &ClientWorld,
        id: PlayerConnectionId,
        tick: u64,
        command: MovementCommand,
    ) -> Result<crate::MovementSubmission, FrameInputError> {
        use FrameInputError as E;
        let control = server.player_control(id).ok_or(E::UnknownConnection)?;
        if server.state != rusterix::ServerState::Running {
            return Err(E::Paused);
        }
        let stream = self.streams.get_mut(&id).ok_or(E::NotEnabled)?;
        if !command.valid() {
            return Err(E::InvalidInput);
        }
        let motion = replica.motion().ok_or(E::NeedsBaseline)?;
        if command.generation != replica.generation()
            || replica.controlled_entity() != Some(control.entity_id)
            || server.player_motion_snapshot(id).is_none_or(|actual| {
                actual.revision != motion.revision || actual.parameters != motion.parameters
            })
        {
            return Err(E::NeedsBaseline);
        }
        if stream.generation != command.generation {
            stream.generation = command.generation;
            stream.received = 0;
            stream.queue.clear();
            stream.revision = motion.revision;
            stream.control = control;
            stream.last_tick = tick;
            stream.moving = false;
            server.set_frame_motion(id, true);
        }
        if command.sequence <= stream.received {
            return Ok(crate::MovementSubmission::IgnoredStale);
        }
        let expected = stream.received.checked_add(1).ok_or(E::NeedsBaseline)?;
        if command.sequence != expected {
            return Err(E::MissingSequence { expected });
        }
        if stream.queue.len() == MAX_INPUT_FRAMES {
            return Err(E::Backpressure);
        }
        stream.queue.push_back((tick, command));
        stream.received = command.sequence;
        Ok(crate::MovementSubmission::Accepted)
    }
    /// Returns connections that require a fresh baseline. No input frame is
    /// repeated when a stream stalls; a release/expiry cannot leave walking on.
    pub fn step(
        &mut self,
        server: &Server,
        hub: Option<&crate::replication::ReplicationHub>,
        tick: u64,
    ) -> Vec<PlayerConnectionId> {
        let mut resync = vec![];
        self.streams.retain(|id, stream| {
            if server.connection_state(*id).is_none() {
                return false;
            }
            let Some(control) = server.player_control(*id) else {
                return true;
            };
            let replica = hub.and_then(|hub| hub.client(*id));
            let Some(actual) = server.player_motion_snapshot(*id) else {
                if server.set_frame_motion(*id, true) {
                    resync.push(*id);
                }
                stream.queue.clear();
                stream.generation = 0;
                return true;
            };
            let Some(replica) = replica else {
                return true;
            };
            if stream.generation == 0 {
                return true;
            }
            let baseline_already = stream.generation != replica.generation();
            let expired =
                stream.queue.front().is_some_and(|(received, _)| {
                    tick.saturating_sub(*received) > INPUT_EXPIRY_TICKS
                }) || (stream.queue.is_empty()
                    && stream.moving
                    && tick.saturating_sub(stream.last_tick) > INPUT_EXPIRY_TICKS);
            if stream.generation != replica.generation()
                || control != stream.control
                || stream.revision != actual.revision
                || replica
                    .motion()
                    .is_none_or(|old| old.parameters != actual.parameters)
                || expired
            {
                stream.queue.clear();
                stream.generation = 0;
                stream.moving = false;
                server.set_frame_motion(*id, true);
                if !baseline_already {
                    resync.push(*id);
                }
                return true;
            }
            if let Some((_, command)) = stream.queue.pop_front() {
                if server.apply_movement_frame(*id, command) {
                    stream.last_tick = tick;
                    stream.moving = command.intent != rusterix::movement::MovementIntent::Stop
                        || server
                            .player_motion_snapshot(*id)
                            .is_some_and(|m| m.controller.grid.is_some());
                } else {
                    stream.generation = 0;
                    resync.push(*id);
                }
            }
            true
        });
        resync
    }
}
