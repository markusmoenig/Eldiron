pub mod asset;
pub mod character;
pub mod context;
pub mod effectwrapper;
pub mod fx;
pub mod interaction;
pub mod item;
pub mod project;
pub mod region;
pub mod renderer_utils;
pub mod rusterix_utils;
pub mod screen;
pub mod settingscontainer;
pub mod tilemap;
pub mod tileselection;

pub mod prelude {
    pub use ::serde::{Deserialize, Serialize};

    pub use crate::asset::*;
    pub use crate::character::Character;
    pub use crate::context::*;
    pub use crate::effectwrapper::*;
    pub use crate::fx::*;
    pub use crate::interaction::*;
    pub use crate::item::Item;
    pub use crate::project::{MapMode, Project};
    pub use crate::region::Region;
    pub use crate::renderer_utils::ray_sphere;
    pub use crate::screen::*;
    pub use crate::tilemap::{Tile, Tilemap};
    pub use indexmap::IndexMap;
    pub use rusterix::{
        Avatar, AvatarAnimation, AvatarAnimationFrame, AvatarDirection, AvatarPerspective,
        AvatarPerspectiveCount,
    };
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
pub enum MaterialType {
    Off,
    PBR,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Material {
    pub base_color: Vec3<f32>,

    pub roughness: f32,
    pub metallic: f32,
    pub ior: f32,

    pub mat_type: MaterialType,
}

impl Default for Material {
    fn default() -> Self {
        Self::new()
    }
}

impl Material {
    pub fn new() -> Self {
        Self {
            base_color: Vec3::new(0.5, 0.5, 0.5),
            roughness: 0.5,
            metallic: 0.0,
            ior: 1.45,

            mat_type: MaterialType::Off,
        }
    }

    /// Mixes two materials.
    pub fn mix(&mut self, mat1: &Material, mat2: &Material, t: f32) {
        self.base_color = mat1
            .base_color
            .map2(mat2.base_color, |a, b| a + t * (b - a));

        self.metallic = f32::lerp(mat1.metallic, mat2.metallic, t);
        self.roughness = f32::lerp(mat1.roughness, mat2.roughness, t);
        self.ior = f32::lerp(mat1.ior, mat2.ior, t);
    }
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

    pub mat: Material,
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

            mat: Material::default(),
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
        if val > 0 { 1 } else { 2 } // clock or counterclock wise
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
