pub mod area;
pub mod asset;
pub mod camera;
pub mod character;
pub mod client;
pub mod fx;
pub mod interaction;
pub mod item;
pub mod level;
pub mod modelfx;
pub mod modelfxnode;
pub mod project;
pub mod region;
pub mod regionfx;
pub mod renderer;
pub mod renderer_utils;
pub mod screen;
pub mod server;
pub mod tilearea;
pub mod tiledrawer;
pub mod tilefx;
pub mod tilemap;
pub mod update;
pub mod widget;

pub mod prelude {
    pub use ::serde::{Deserialize, Serialize};

    pub use crate::area::Area;
    pub use crate::asset::*;
    pub use crate::camera::Camera;
    pub use crate::character::Character;
    pub use crate::client::*;
    pub use crate::fx::*;
    pub use crate::interaction::*;
    pub use crate::item::Item;
    pub use crate::level::Level;
    pub use crate::modelfx::ModelFX;
    pub use crate::modelfxnode::*;
    pub use crate::project::Project;
    pub use crate::region::{CameraMode, CameraType, Layer2DRole, Region, RegionTile};
    pub use crate::regionfx::*;
    pub use crate::renderer::Renderer;
    //pub use crate::renderer_utils::*;
    pub use crate::screen::*;
    pub use crate::server::context::ServerContext;
    pub use crate::server::{Server, ServerState};
    pub use crate::tilearea::TileArea;
    pub use crate::tiledrawer::{RegionDrawSettings, TileDrawer};
    pub use crate::tilefx::TileFX;
    pub use crate::tilemap::{Tile, TileRole, Tilemap};
    pub use crate::update::*;
    pub use crate::widget::*;
    pub use crate::{Hit, HitFace, Ray};
    pub use rand::prelude::*;
}

use theframework::prelude::*;

/// Ray
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Ray {
    pub o: Vec3f,
    pub d: Vec3f,

    pub inv_direction: Vec3f,

    pub sign_x: usize,
    pub sign_y: usize,
    pub sign_z: usize,
}

impl Ray {
    pub fn new(o: Vec3f, d: Vec3f) -> Self {
        Self {
            o,
            d,

            inv_direction: Vec3f::new(1.0 / d.x, 1.0 / d.y, 1.0 / d.z),
            sign_x: (d.x < 0.0) as usize,
            sign_y: (d.y < 0.0) as usize,
            sign_z: (d.z < 0.0) as usize,
        }
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
    pub distance: f32,
    pub hit_point: Vec3f,
    pub normal: Vec3f,
    pub uv: Vec2f,
    pub face: HitFace,
}
