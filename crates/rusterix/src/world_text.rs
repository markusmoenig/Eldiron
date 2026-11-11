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
