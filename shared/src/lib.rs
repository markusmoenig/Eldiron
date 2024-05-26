pub mod area;
pub mod asset;
pub mod camera;
pub mod character;
pub mod client;
pub mod fx;
pub mod geofxnode;
pub mod geofxobject;
pub mod interaction;
pub mod item;
pub mod level;
pub mod modelfx;
pub mod modelfxnode;
pub mod modelfxstore;
pub mod modelfxterminal;
pub mod patterns;
pub mod project;
pub mod region;
pub mod regionfx;
pub mod renderer;
pub mod renderer_utils;
pub mod screen;
pub mod sdf3d;
pub mod server;
pub mod tilearea;
pub mod tiledrawer;
pub mod tilefx;
pub mod tilemap;
pub mod update;
pub mod voxelthread;
pub mod widget;

pub mod prelude {
    pub use ::serde::{Deserialize, Serialize};

    pub use crate::area::Area;
    pub use crate::asset::*;
    pub use crate::camera::Camera;
    pub use crate::character::Character;
    pub use crate::client::*;
    pub use crate::fx::*;
    pub use crate::geofxnode::*;
    pub use crate::geofxobject::*;
    pub use crate::interaction::*;
    pub use crate::item::Item;
    pub use crate::level::*;
    pub use crate::modelfx::ModelFX;
    pub use crate::modelfxnode::*;
    pub use crate::modelfxstore::ModelFXStore;
    pub use crate::modelfxterminal::*;
    pub use crate::patterns::*;
    pub use crate::project::{MapMode, Project};
    pub use crate::region::{CameraMode, CameraType, Layer2DRole, Region, RegionTile};
    pub use crate::regionfx::*;
    pub use crate::renderer::Renderer;
    pub use crate::renderer_utils::*;
    pub use crate::screen::*;
    pub use crate::sdf3d::*;
    pub use crate::server::context::ServerContext;
    pub use crate::server::{Server, ServerState};
    pub use crate::tilearea::TileArea;
    pub use crate::tiledrawer::{RegionDrawSettings, TileDrawer};
    pub use crate::tilefx::TileFX;
    pub use crate::tilemap::{Tile, TileRole, Tilemap};
    pub use crate::update::*;
    pub use crate::voxelthread::*;
    pub use crate::widget::*;
    pub use crate::ServerMessage;
    pub use crate::{do_intersect, Hit, HitFace, Ray, Voxel};
    pub use rand::prelude::*;
}

use theframework::prelude::*;

/// Messages to the clients. The first argument is always the client id.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ServerMessage {
    /// The given player instance has joined the server in the given region.
    /// The client will use this to know what player / region to draw / focus on.
    PlayerJoined(Uuid, Uuid, Uuid),
}

/// Ray
#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub struct Ray {
    pub o: Vec3f,
    pub d: Vec3f,
}

impl Ray {
    pub fn new(o: Vec3f, d: Vec3f) -> Self {
        Self { o, d }
    }

    /// Returns the position on the ray at the given distance
    pub fn at(&self, d: f32) -> Vec3f {
        self.o + self.d * d
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum HitFace {
    XFace,
    YFace,
    ZFace,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Hit {
    pub node: usize,

    pub key: Vec3f,
    pub hash: f32,

    pub distance: f32,
    pub hit_point: Vec3f,
    pub normal: Vec3f,
    pub uv: Vec2f,
    pub face: HitFace,

    pub color: Vec4f,
    pub roughness: f32,
    pub metallic: f32,
    pub reflectance: f32,
}

impl Default for Hit {
    fn default() -> Self {
        Self::new()
    }
}

impl Hit {
    pub fn new() -> Self {
        Self {
            node: 0,

            key: Vec3f::zero(),
            hash: 0.0,

            distance: f32::MAX,
            hit_point: Vec3f::zero(),
            normal: Vec3f::zero(),
            uv: Vec2f::zero(),
            face: HitFace::XFace,

            color: Vec4f::zero(),
            roughness: 0.5,
            metallic: 0.0,
            reflectance: 0.5,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub struct Voxel {
    pub color: [u8; 3],
    pub roughness: u8,
    pub metallic: u8,
    pub reflectance: u8,
}

impl Default for Voxel {
    fn default() -> Self {
        Self::new()
    }
}

impl Voxel {
    pub fn new() -> Self {
        Self {
            color: [0, 0, 0],
            roughness: 128,
            metallic: 0,
            reflectance: 128,
        }
    }
}

// https://www.geeksforgeeks.org/check-if-two-given-line-segments-intersect/
pub fn do_intersect(p1: (i32, i32), q1: (i32, i32), p2: (i32, i32), q2: (i32, i32)) -> bool {
    // Given three collinear points p, q, r, the function checks if
    // point q lies on line segment 'pr'
    fn on_segment(p: (i32, i32), q: (i32, i32), r: (i32, i32)) -> bool {
        q.0 <= std::cmp::max(p.0, r.0)
            && q.0 >= std::cmp::min(p.0, r.0)
            && q.1 <= std::cmp::max(p.1, r.1)
            && q.1 >= std::cmp::min(p.1, r.1)
    }

    // To find orientation of ordered triplet (p, q, r).
    // The function returns following values
    // 0 --> p, q and r are collinear
    // 1 --> Clockwise
    // 2 --> Counterclockwise
    fn orientation(p: (i32, i32), q: (i32, i32), r: (i32, i32)) -> i32 {
        let val = (q.1 - p.1) * (r.0 - q.0) - (q.0 - p.0) * (r.1 - q.1);
        if val == 0 {
            return 0;
        } // collinear
        if val > 0 {
            1
        } else {
            2
        } // clock or counterclock wise
    }

    // Check if line segments 'p1q1' and 'p2q2' intersect.
    let o1 = orientation(p1, q1, p2);
    let o2 = orientation(p1, q1, q2);
    let o3 = orientation(p2, q2, p1);
    let o4 = orientation(p2, q2, q1);

    // General case
    if o1 != o2 && o3 != o4 {
        return true;
    }

    // Special Cases
    // p1, q1 and p2 are collinear and p2 lies on segment p1q1
    if o1 == 0 && on_segment(p1, p2, q1) {
        return true;
    }

    // p1, q1 and q2 are collinear and q2 lies on segment p1q1
    if o2 == 0 && on_segment(p1, q2, q1) {
        return true;
    }

    // p2, q2 and p1 are collinear and p1 lies on segment p2q2
    if o3 == 0 && on_segment(p2, p1, q2) {
        return true;
    }

    // p2, q2 and q1 are collinear and q1 lies on segment p2q2
    if o4 == 0 && on_segment(p2, q1, q2) {
        return true;
    }

    // Doesn't fall in any of the above cases
    false
}
