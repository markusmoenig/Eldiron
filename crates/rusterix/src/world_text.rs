//! Private, explicitly requested text attached to a world target.
use uuid::Uuid;
use vek::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub enum TextAnchor {
    Entity(u32),
    Item(u32),
    Prefab(Uuid),
    Position(Vec3<f32>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldText {
    pub anchor: TextAnchor,
    pub text: String,
}

mod passages;
mod policy;
pub use passages::{WorldPassage, WorldTexts, lifetime};
pub use policy::{anchor_position, carrier, resolve_for_player};
#[cfg(feature = "graphics")]
mod projection;
#[cfg(feature = "graphics")]
pub use projection::project;
