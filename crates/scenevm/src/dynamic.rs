use uuid::Uuid;
use vek::{Vec2, Vec3};

use crate::vm::GeoId;

/// Types of dynamic objects that can be injected per-frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicKind {
    BillboardTile = 0,
    BillboardAvatar = 1,
}

impl Default for DynamicKind {
    fn default() -> Self {
        DynamicKind::BillboardTile
    }
}

/// Repeat mode for billboard tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepeatMode {
    /// Scale the tile to fit the billboard size (default)
    Scale = 0,
    /// Repeat the tile across the billboard
    Repeat = 1,
}

impl Default for RepeatMode {
    fn default() -> Self {
        RepeatMode::Scale
    }
}

/// Per-frame dynamic object description (billboards, particles, etc.).
#[derive(Clone, Debug)]
pub struct DynamicObject {
    pub id: GeoId,
    pub kind: DynamicKind,
    pub tile_id: Option<Uuid>,
    pub center: Vec3<f32>,
    pub view_right: Vec3<f32>,
    pub view_up: Vec3<f32>,
    pub width: f32,
    pub height: f32,
    pub repeat_mode: RepeatMode,
    /// Per-billboard opacity (1.0 = fully opaque).
    pub opacity: f32,
}

impl Default for DynamicObject {
    fn default() -> Self {
        Self {
            id: GeoId::Unknown(0),
            kind: DynamicKind::BillboardTile,
            tile_id: None,
            center: Vec3::zero(),
            view_right: Vec3::unit_x(),
            view_up: Vec3::unit_y(),
            width: 1.0,
            height: 1.0,
            repeat_mode: RepeatMode::Scale,
            opacity: 1.0,
        }
    }
}

impl DynamicObject {
    /// Convenience constructor for a billboard that references a tile.
    pub fn billboard_tile(
        id: GeoId,
        tile_id: Uuid,
        center: Vec3<f32>,
        view_right: Vec3<f32>,
        view_up: Vec3<f32>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            id,
            kind: DynamicKind::BillboardTile,
            tile_id: Some(tile_id),
            center,
            view_right,
            view_up,
            width,
            height,
            repeat_mode: RepeatMode::Scale,
            opacity: 1.0,
        }
    }

    /// Convenience constructor for a 2D billboard, supplying only the XY position and size.
    pub fn billboard_tile_2d(
        id: GeoId,
        tile_id: Uuid,
        pos: Vec2<f32>,
        width: f32,
        height: f32,
    ) -> Self {
        Self::billboard_tile(
            id,
            tile_id,
            Vec3::new(pos.x, pos.y, 0.0),
            Vec3::unit_x(),
            Vec3::unit_y(),
            width,
            height,
        )
    }

    /// Convenience constructor for an avatar billboard that references a tile.
    /// Avatar billboards are managed per GeoId and replace previous avatar data for the same id.
    pub fn billboard_avatar(
        id: GeoId,
        center: Vec3<f32>,
        view_right: Vec3<f32>,
        view_up: Vec3<f32>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            id,
            kind: DynamicKind::BillboardAvatar,
            tile_id: None,
            center,
            view_right,
            view_up,
            width,
            height,
            repeat_mode: RepeatMode::Scale,
            opacity: 1.0,
        }
    }

    /// Convenience constructor for a 2D avatar billboard, supplying only the XY position and size.
    pub fn billboard_avatar_2d(id: GeoId, pos: Vec2<f32>, width: f32, height: f32) -> Self {
        Self::billboard_avatar(
            id,
            Vec3::new(pos.x, pos.y, 0.0),
            Vec3::unit_x(),
            Vec3::unit_y(),
            width,
            height,
        )
    }

    /// Set the repeat mode for this billboard.
    pub fn with_repeat_mode(mut self, mode: RepeatMode) -> Self {
        self.repeat_mode = mode;
        self
    }

    /// Set per-billboard opacity (1.0 = opaque).
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
}
