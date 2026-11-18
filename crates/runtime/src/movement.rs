//! Sequenced movement state. Continuous requests replace pending state; grid
//! edges are bounded and consumed on fixed steps. Only the host grants time.
//! One-shot gameplay requests use separate routing.
use serde::{Deserialize, Serialize};
pub const MAX_QUEUED_MOVEMENT_STATES: usize = 32;

pub use rusterix::movement::MovementIntent;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovementInput {
    pub generation: u64,
    pub sequence: u64,
    pub intent: MovementIntent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementSubmission {
    Accepted,
    IgnoredStale,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementInputError {
    UnknownConnection,
    NotControlled,
    NeedsBaseline,
    InvalidSequence,
    Backpressure,
}
impl std::fmt::Display for MovementInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for MovementInputError {}

pub(crate) struct PendingMovement {
    pub input: MovementInput,
    pub pending: bool,
    pub grid: bool,
    pub edges: std::collections::VecDeque<MovementInput>,
}
