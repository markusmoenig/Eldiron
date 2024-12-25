pub mod area;
pub mod asset;
// pub mod bsdf;
// pub mod camera;
pub mod character;
pub mod client;
pub mod fx;
// pub mod gamecanvas;
// pub mod geofxnode;
// pub mod geofxobject;
// pub mod heightmap;
pub mod interaction;
pub mod item;
pub mod level;
// pub mod map_geometry;
pub mod maprender;
pub mod materialfxnode;
pub mod materialfxobject;
// pub mod patterns;
// pub mod prerendered;
// pub mod prerenderthread;
pub mod project;
pub mod region;
pub mod regionfxnode;
pub mod regionfxobject;
// pub mod renderer;
pub mod renderer_utils;
pub mod screen;
// pub mod sdf;
pub mod server;
pub mod tiledrawer;
pub mod tilefx;
pub mod tilefxnode;
pub mod tilefxobject;
pub mod tilemap;
pub mod tileselection;
pub mod update;
pub mod widget;

pub const PATTERN2D_DISTANCE_BORDER: f32 = 0.01;

pub mod prelude {
    pub use crate::PATTERN2D_DISTANCE_BORDER;
    pub use ::serde::{Deserialize, Serialize};

    pub use crate::area::Area;
    pub use crate::asset::*;
    //pub use crate::bsdf::*;
    //pub use crate::camera::Camera;
    pub use crate::character::Character;
    pub use crate::client::*;
    pub use crate::fx::*;
    // pub use crate::gamecanvas::GameCanvas;
    // pub use crate::geofxobject::*;
    // pub use crate::heightmap::*;
    pub use crate::interaction::*;
    pub use crate::item::Item;
    pub use crate::level::*;
    // pub use crate::map_geometry::*;
    pub use crate::maprender::*;
    pub use crate::materialfxnode::*;
    pub use crate::materialfxobject::*;
    // pub use crate::patterns::*;
    pub use rusterix::map::*;
    // pub use crate::prerendered::*;
    // pub use crate::prerenderthread::*;
    pub use crate::project::{MapMode, Project};
    pub use crate::region::{CameraMode, Layer2DRole, Region, RegionTile};
    pub use crate::regionfxnode::*;
    pub use crate::regionfxobject::*;
    // pub use crate::renderer::Renderer;
    pub use crate::renderer_utils::ray_sphere;
    pub use crate::screen::*;
    // pub use crate::sdf::*;
    pub use crate::server::context::*;
    pub use crate::server::{Server, ServerState};
    pub use crate::tiledrawer::{RegionDrawSettings, TileDrawer};
    pub use crate::tilefx::TileFX;
    pub use crate::tilefxnode::*;
    pub use crate::tilefxobject::TileFXObject;
    pub use crate::tilemap::{Tile, TileRole, Tilemap};
    pub use crate::tileselection::{TileSelection, TileSelectionMode};
    pub use crate::update::*;
    pub use crate::widget::*;
    pub use crate::ServerMessage;
    pub use crate::{do_intersect, Hit, HitFace, HitMode, Ray, RenderTile, AABB2D};
    pub use indexmap::IndexMap;
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
    pub o: Vec3<f32>,
    pub d: Vec3<f32>,
}

impl Ray {
    pub fn new(o: Vec3<f32>, d: Vec3<f32>) -> Self {
        Self { o, d }
    }

    /// Returns the position on the ray at the given distance
    pub fn at(&self, d: f32) -> Vec3<f32> {
        self.o + self.d * d
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum HitMode {
    Albedo,
    Bump,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum HitFace {
    XFace,
    YFace,
    ZFace,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Hit {
    pub is_valid: bool,

    pub mode: HitMode,

    pub node: usize,

    pub eps: f32,

    pub key: Vec3<f32>,
    pub hash: f32,

    pub bump: f32,

    pub distance: f32,
    pub interior_distance: f32,

    pub hit_point: Vec3<f32>,
    pub normal: Vec3<f32>,
    pub uv: Vec2<f32>,
    pub global_uv: Vec2<f32>,
    pub face: HitFace,

    pub pattern_pos: Vec2<f32>,

    pub color: Vec4<f32>,

    //pub mat: BSDFMaterial,
    pub noise: Option<f32>,
    pub noise_scale: f32,

    pub value: f32,

    pub two_d: bool,
}

impl Default for Hit {
    fn default() -> Self {
        Self::new()
    }
}

impl Hit {
    pub fn new() -> Self {
        Self {
            is_valid: true,

            mode: HitMode::Albedo,

            node: 0,

            eps: 0.001, //0.0003,

            key: Vec3::zero(),
            hash: 0.0,

            bump: 0.0,

            distance: f32::MAX,
            interior_distance: f32::MAX,

            hit_point: Vec3::zero(),
            normal: Vec3::zero(),
            uv: Vec2::zero(),
            global_uv: Vec2::zero(),
            face: HitFace::XFace,

            pattern_pos: Vec2::zero(),

            color: Vec4::zero(),

            // mat: BSDFMaterial::default(),
            noise: None,
            noise_scale: 1.0,

            value: 1.0,

            two_d: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AABB2D {
    min: Vec2<f32>,
    max: Vec2<f32>,
}

impl Default for AABB2D {
    fn default() -> Self {
        Self::zero()
    }
}

impl AABB2D {
    pub fn new(min: Vec2<f32>, max: Vec2<f32>) -> Self {
        AABB2D { min, max }
    }

    pub fn zero() -> Self {
        AABB2D {
            min: Vec2::new(f32::MAX, f32::MAX),
            max: Vec2::new(f32::MIN, f32::MIN),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y
    }

    pub fn grow(&mut self, other: AABB2D) {
        self.min.x = self.min.x.min(other.min.x);
        self.min.y = self.min.y.min(other.min.y);
        self.max.x = self.max.x.max(other.max.x);
        self.max.y = self.max.y.max(other.max.y);
    }

    pub fn to_int(&self) -> (Vec2<i32>, Vec2<i32>) {
        let min_int = Vec2::new(self.min.x.floor() as i32, self.min.y.floor() as i32);
        let max_int = Vec2::new(self.max.x.ceil() as i32, self.max.y.ceil() as i32);
        (min_int, max_int)
    }

    pub fn to_tiles(&self) -> Vec<Vec2<i32>> {
        let (min_int, max_int) = self.to_int();
        let mut tiles = Vec::new();

        for x in min_int.x..=max_int.x {
            for y in min_int.y..=max_int.y {
                tiles.push(Vec2::new(x, y));
            }
        }

        tiles
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RenderTile {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl RenderTile {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn create_tiles(
        image_width: usize,
        image_height: usize,
        tile_width: usize,
        tile_height: usize,
    ) -> Vec<Self> {
        // TODO: Generate the tiles in a nice spiral pattern

        let mut tiles = Vec::new();
        let mut x = 0;
        let mut y = 0;
        while x < image_width && y < image_height {
            let tile = Self {
                x,
                y,
                width: tile_width,
                height: tile_height,
            };
            tiles.push(tile);
            x += tile_width;
            if x >= image_width {
                x = 0;
                y += tile_height;
            }
        }

        tiles
    }
}
