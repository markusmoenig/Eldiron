//! Ephemeral player connections into one authoritative world. These handles
//! route trusted host requests; account authentication and wire validation live
//! outside the simulation. Restarting the server revokes every old handle.
use theframework_core::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerConnectionId(Uuid);

impl PlayerConnectionId {
    pub(super) fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn uuid(self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerControl {
    /// Runtime region instance, not an authored map UUID.
    pub region_id: u32,
    pub entity_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlayerConnectionState {
    #[default]
    AwaitingCharacter,
    Spawning {
        region_id: u32,
    },
    Active(PlayerControl),
}

impl PlayerConnectionState {
    pub fn control(self) -> Option<PlayerControl> {
        match self {
            Self::Active(control) => Some(control),
            _ => None,
        }
    }
}
