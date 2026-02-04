//! Rusterix is a fast software renderer for 2D and 3D triangles and lines.
//! Its goals are to provide an easy and portable alternative to hardware rasterization for retro and low-poly games.

pub mod batch;
pub mod camera;
pub mod chunk;
pub mod chunkbuilder;
pub mod client;
pub mod collision_world;
pub mod edge;
pub mod intodata;
pub mod map;
pub mod material_profile;
pub mod rasterizer;
pub mod rect;
pub mod render_settings;
pub mod rendermode;
pub mod rusterix;
pub mod scene;
pub mod scene_handler;
pub mod scenebuilder;
pub mod scenemanager;
// pub mod script;
pub mod server;
pub mod shader;
pub mod shapestack;
pub mod terrain;
pub mod texture;
pub mod tracer;
pub mod utils;
pub mod value;
pub mod value_toml;
pub mod vertexblend;
pub mod vm;
pub mod wavefront;

#[cfg(feature = "single_thread")]
pub const IS_THREADED: bool = false;

#[cfg(not(feature = "single_thread"))]
pub const IS_THREADED: bool = true;

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.txt"]
#[exclude = "*.DS_Store"]
pub struct Embedded;

pub type Pixel = [u8; 4];
const INV_255: f32 = 1.0 / 255.0;

/// Convert from Pixel to Vec4<f32>
#[inline(always)]
pub fn pixel_to_vec4(pixel: &Pixel) -> vek::Vec4<f32> {
    vek::Vec4::new(
        pixel[0] as f32 * INV_255,
        pixel[1] as f32 * INV_255,
        pixel[2] as f32 * INV_255,
        pixel[3] as f32 * INV_255,
    )
}

#[inline(always)]
fn f32_to_u8_saturated(x: f32) -> u8 {
    let y = x.max(0.0).min(1.0).mul_add(255.0, 0.5);
    y as i32 as u8
}

/// Convert from Vec4<f32> to Pixel
#[inline(always)]
pub fn vec4_to_pixel(vec: &vek::Vec4<f32>) -> Pixel {
    [
        f32_to_u8_saturated(vec.x),
        f32_to_u8_saturated(vec.y),
        f32_to_u8_saturated(vec.z),
        f32_to_u8_saturated(vec.w),
    ]
}

/// Get time in ms
pub fn get_time() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().unwrap().performance().unwrap().now() as u128
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let stop = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards");
        stop.as_millis()
    }
}

pub const TRANSPARENT: Pixel = [0, 0, 0, 0];
pub const BLACK: Pixel = [0, 0, 0, 255];
pub const WHITE: Pixel = [255, 255, 255, 255];

// Re-exports
pub use crate::{
    batch::{CullMode, GeometrySource, PrimitiveMode, batch2d::Batch2D, batch3d::Batch3D},
    camera::{D3Camera, d3firstp::D3FirstPCamera, d3iso::D3IsoCamera, d3orbit::D3OrbitCamera},
    chunk::{BillboardMetadata, Chunk},
    chunkbuilder::{ChunkBuilder, d2chunkbuilder::D2ChunkBuilder, d3chunkbuilder::D3ChunkBuilder},
    client::{
        Client,
        command::Command,
        daylight::Daylight,
        parser::{MsgParser, Tok},
    },
    collision_world::CollisionWorld,
    edge::Edges,
    intodata::IntoDataInput,
    map::{
        Map, MapCamera, MapToolType,
        bbox::BBox,
        light::CompiledLight,
        light::Light,
        light::LightType,
        linedef::CompiledLinedef,
        linedef::Linedef,
        meta::MapMeta,
        mini::MapMini,
        particle::{Particle, ParticleEmitter},
        pixelsource::NoiseTarget,
        pixelsource::PixelSource,
        sector::Sector,
        softrig::{Keyform, SoftRig, SoftRigAnimator},
        surface::{BillboardAnimation, LoopOp, ProfileLoop, Surface},
        tile::{Tile, TileRole},
        vertex::Vertex,
    },
    material_profile::MaterialProfile,
    rasterizer::{BrushPreview, Rasterizer},
    rect::Rect,
    render_settings::RenderSettings,
    rendermode::RenderMode,
    rusterix::Rusterix,
    scene::Scene,
    scene_handler::SceneHandler,
    scenebuilder::{
        d2builder::D2Builder, d2material::D2MaterialBuilder, d2preview::D2PreviewBuilder,
    },
    scenemanager::*,
    // script::mapscript::MapScript,
    server::{
        Server, ServerState,
        assets::Assets,
        currency::{Currencies, Currency, Wallet},
        entity::Entity,
        entity::EntityUpdate,
        item::{Item, ItemUpdate},
        message::EntityAction,
        message::{Choice, MultipleChoice, PlayerCamera, RegionMessage},
        region::RegionInstance,
        regionctx::RegionCtx,
    },
    shader::{Shader, grid::GridShader, vgradient::VGrayGradientShader},
    shapestack::{
        ShapeStack,
        material::{Material, MaterialModifier, MaterialRole},
        shape::{Shape, ShapeType},
        shapecontext::ShapeContext,
        shapefx::{ShapeFX, ShapeFXModifierPass, ShapeFXParam, ShapeFXRole},
        shapefxgraph::ShapeFXGraph,
        tilebuilder::tile_builder,
    },
    terrain::{
        Terrain, TerrainHit,
        chunk::{TerrainBlendMode, TerrainChunk},
    },
    texture::{RepeatMode, SampleMode, Texture},
    tracer::{HitInfo, Ray, buffer::AccumBuffer, trace::Tracer},
    value::{HeightControlPoint, Value, ValueContainer},
    value_toml::{ValueGroups, ValueTomlLoader},
    vertexblend::VertexBlendPreset,
};

// Prelude
pub mod prelude {
    pub use crate::Chunk;
    pub use crate::Client;
    pub use crate::IntoDataInput;
    // pub use crate::MapScript;
    pub use crate::Rasterizer;
    pub use crate::RenderMode;
    pub use crate::scenebuilder::{
        d2builder::D2Builder, d2material::D2MaterialBuilder, d2preview::D2PreviewBuilder,
        d3builder::D3Builder,
    };
    pub use crate::{
        Assets, Choice, Currencies, Currency, Entity, EntityUpdate, Item, ItemUpdate,
        MultipleChoice, RegionInstance, RegionMessage, Server, Wallet,
    };
    pub use crate::{BLACK, Pixel, TRANSPARENT, WHITE};
    pub use crate::{Batch2D, Batch3D, CullMode, GeometrySource, PrimitiveMode};
    pub use crate::{D3Camera, D3FirstPCamera, D3IsoCamera, D3OrbitCamera};
    pub use crate::{GridShader, Shader, VGrayGradientShader};
    pub use crate::{
        Keyform, Light, LightType, Map, MapMeta, MapToolType, NoiseTarget, Particle,
        ParticleEmitter, PixelSource, Sector, SoftRig, SoftRigAnimator, Tile, TileRole, Vertex,
    };
    pub use crate::{Material, MaterialModifier, MaterialRole};
    pub use crate::{
        Rect, Scene, SceneManager, SceneManagerCmd, SceneManagerResult, Value, ValueContainer,
    };
    pub use crate::{RepeatMode, SampleMode, Texture};
    pub use crate::{pixel_to_vec4, vec4_to_pixel};
    pub use codegridfx::{DebugGrid, DebugModule};
}
