use uuid::Uuid;
use vek::{Vec2, Vec3};

use crate::GeoId;

/// Types of dynamic objects that can be injected per-frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicKind {
    BillboardTile = 0,
    BillboardAvatar = 1,
    Mesh = 2,
    ParticleBillboard = 3,
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

/// Alpha handling for billboard tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaMode {
    /// Use texture alpha as-is.
    Texture = 0,
    /// Treat pixels matching the tile's top-left color as transparent.
    ChromaKey = 1,
}

impl Default for AlphaMode {
    fn default() -> Self {
        AlphaMode::Texture
    }
}

/// Per-frame dynamic object description (billboards, particles, etc.).
#[derive(Clone, Debug)]
pub struct DynamicMeshVertex {
    pub position: Vec3<f32>,
    pub uv: Vec2<f32>,
    pub normal: Vec3<f32>,
}

/// Per-frame dynamic object description (billboards, particles, meshes, etc.).
#[derive(Clone, Debug)]
pub struct DynamicObject {
    pub id: GeoId,
    pub kind: DynamicKind,
    pub tile_id: Option<Uuid>,
    pub layer: i32,
    pub center: Vec3<f32>,
    pub view_right: Vec3<f32>,
    pub view_up: Vec3<f32>,
    pub width: f32,
    pub height: f32,
    pub repeat_mode: RepeatMode,
    pub alpha_mode: AlphaMode,
    /// Per-billboard opacity (1.0 = fully opaque).
    pub opacity: f32,
    /// Per-billboard tint in linear space. Used by particle billboards.
    pub tint: Vec3<f32>,
    /// Optional animation start counter. When set, animated tiles can start at frame 0 for this instance.
    pub anim_start_counter: Option<u32>,
    /// Dynamic mesh vertices in local/object space. Used when `kind == Mesh`.
    pub mesh_vertices: Vec<DynamicMeshVertex>,
    /// Dynamic mesh triangle indices. Used when `kind == Mesh`.
    pub mesh_indices: Vec<u32>,
}

impl Default for DynamicObject {
    fn default() -> Self {
        Self {
            id: GeoId::Unknown(0),
            kind: DynamicKind::BillboardTile,
            tile_id: None,
            layer: 0,
            center: Vec3::zero(),
            view_right: Vec3::unit_x(),
            view_up: Vec3::unit_y(),
            width: 1.0,
            height: 1.0,
            repeat_mode: RepeatMode::Scale,
            alpha_mode: AlphaMode::Texture,
            opacity: 1.0,
            tint: Vec3::new(1.0, 1.0, 1.0),
            anim_start_counter: None,
            mesh_vertices: Vec::new(),
            mesh_indices: Vec::new(),
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
            layer: 0,
            center,
            view_right,
            view_up,
            width,
            height,
            repeat_mode: RepeatMode::Scale,
            alpha_mode: AlphaMode::Texture,
            opacity: 1.0,
            tint: Vec3::new(1.0, 1.0, 1.0),
            anim_start_counter: None,
            mesh_vertices: Vec::new(),
            mesh_indices: Vec::new(),
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
            Vec3::new(0.0, -1.0, 0.0),
            width,
            height,
        )
    }

    /// Convenience constructor for a particle billboard that references a tile.
    pub fn particle_tile(
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
            kind: DynamicKind::ParticleBillboard,
            tile_id: Some(tile_id),
            layer: 0,
            center,
            view_right,
            view_up,
            width,
            height,
            repeat_mode: RepeatMode::Scale,
            alpha_mode: AlphaMode::Texture,
            opacity: 1.0,
            tint: Vec3::new(1.0, 1.0, 1.0),
            anim_start_counter: None,
            mesh_vertices: Vec::new(),
            mesh_indices: Vec::new(),
        }
    }

    /// Convenience constructor for a 2D particle billboard.
    pub fn particle_tile_2d(
        id: GeoId,
        tile_id: Uuid,
        pos: Vec2<f32>,
        width: f32,
        height: f32,
    ) -> Self {
        Self::particle_tile(
            id,
            tile_id,
            Vec3::new(pos.x, pos.y, 0.0),
            Vec3::unit_x(),
            Vec3::new(0.0, -1.0, 0.0),
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
            layer: 0,
            center,
            view_right,
            view_up,
            width,
            height,
            repeat_mode: RepeatMode::Scale,
            alpha_mode: AlphaMode::Texture,
            opacity: 1.0,
            tint: Vec3::new(1.0, 1.0, 1.0),
            anim_start_counter: None,
            mesh_vertices: Vec::new(),
            mesh_indices: Vec::new(),
        }
    }

    /// Convenience constructor for a 2D avatar billboard, supplying only the XY position and size.
    pub fn billboard_avatar_2d(id: GeoId, pos: Vec2<f32>, width: f32, height: f32) -> Self {
        Self::billboard_avatar(
            id,
            Vec3::new(pos.x, pos.y, 0.0),
            Vec3::unit_x(),
            Vec3::new(0.0, -1.0, 0.0),
            width,
            height,
        )
    }

    /// Convenience constructor for a dynamic mesh that references a tile/material.
    pub fn mesh(
        id: GeoId,
        tile_id: Uuid,
        vertices: Vec<DynamicMeshVertex>,
        indices: Vec<u32>,
    ) -> Self {
        Self {
            id,
            kind: DynamicKind::Mesh,
            tile_id: Some(tile_id),
            layer: 0,
            center: Vec3::zero(),
            view_right: Vec3::unit_x(),
            view_up: Vec3::unit_y(),
            width: 1.0,
            height: 1.0,
            repeat_mode: RepeatMode::Scale,
            alpha_mode: AlphaMode::Texture,
            opacity: 1.0,
            tint: Vec3::new(1.0, 1.0, 1.0),
            anim_start_counter: None,
            mesh_vertices: vertices,
            mesh_indices: indices,
        }
    }

    /// Set the repeat mode for this billboard.
    pub fn with_repeat_mode(mut self, mode: RepeatMode) -> Self {
        self.repeat_mode = mode;
        self
    }

    /// Set alpha handling mode.
    pub fn with_alpha_mode(mut self, mode: AlphaMode) -> Self {
        self.alpha_mode = mode;
        self
    }

    /// Set per-billboard opacity (1.0 = opaque).
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Set per-billboard tint color in linear space.
    pub fn with_tint(mut self, tint: Vec3<f32>) -> Self {
        self.tint = tint;
        self
    }

    /// Set an optional animation start counter for this billboard.
    pub fn with_anim_start_counter(mut self, counter: Option<u32>) -> Self {
        self.anim_start_counter = counter;
        self
    }

    /// Set the 2D render layer for this billboard. Higher layers draw on top.
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }
}
