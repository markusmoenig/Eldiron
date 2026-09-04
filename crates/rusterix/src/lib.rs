//! Rusterix is a fast software renderer for 2D and 3D triangles and lines.
//! Its goals are to provide an easy and portable alternative to hardware rasterization for retro and low-poly games.

pub mod audio;
pub mod avatar;
#[cfg(feature = "graphics")]
pub mod avatar_builder;
#[cfg(feature = "graphics")]
pub mod avatar_recipe;
pub mod batch;
pub mod builderpreview;
pub mod camera;
pub mod chunk;
pub mod chunkbuilder;
#[cfg(feature = "graphics")]
pub mod client;
#[cfg(feature = "graphics")]
pub use client::text_command;
pub mod collision_world;
pub mod command;
pub mod edge;
pub mod hitinfo;
pub mod intodata;
pub mod map;
pub mod material_library;
pub mod material_profile;
#[cfg(feature = "graphics")]
pub mod orthographic_bake;
pub mod particleharness;
pub mod procedural;
pub mod rasterizer;
pub mod ray;
pub mod rect;
#[cfg(feature = "graphics")]
pub mod render_settings;
pub mod rendermode;
#[cfg(feature = "graphics")]
pub mod rusterix;
pub mod scene;
pub mod scene_build_index;
#[cfg(feature = "graphics")]
pub mod scene_handler;
pub mod scenebuilder;
pub mod scenemanager;
pub mod server;
#[cfg(not(feature = "graphics"))]
#[path = "client/text_command.rs"]
pub mod text_command;
pub mod texture;
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
#[cfg(feature = "graphics")]
pub use crate::client::{
    Client,
    daylight::Daylight,
    parser::{MsgParser, Tok},
};
pub use crate::command::Command;
#[cfg(feature = "graphics")]
pub use crate::orthographic_bake::{
    OrthographicBakeController, OrthographicBakeLighting, OrthographicBakeStatus,
    OrthographicBakeWork,
};
#[cfg(feature = "graphics")]
pub use crate::render_settings::RenderSettings;
#[cfg(feature = "graphics")]
pub use crate::rusterix::Rusterix;
#[cfg(feature = "graphics")]
pub use crate::scene_handler::{ParticleDebugStats, ParticlePipelineStats, SceneHandler};
#[cfg(feature = "graphics")]
pub use crate::scenebuilder::{d2preview::D2PreviewBuilder, d3builder::D3Builder};
pub use crate::{
    audio::{AudioConfig, AudioEngine, AudioError, OutputInfo, SineVoiceId},
    avatar::{
        Avatar, AvatarAnimation, AvatarAnimationFrame, AvatarBuildOutput, AvatarBuildRequest,
        AvatarBuilder, AvatarDirection, AvatarMarkerChannel, AvatarMarkerColors, AvatarPerspective,
        AvatarPerspectiveCount, AvatarShadingOptions,
    },
    batch::{CullMode, GeometrySource, PrimitiveMode, batch2d::Batch2D, batch3d::Batch3D},
    camera::{D3Camera, d3firstp::D3FirstPCamera, d3iso::D3IsoCamera, d3orbit::D3OrbitCamera},
    chunk::{BillboardMetadata, Chunk},
    chunkbuilder::{
        ChunkBuilder,
        d2chunkbuilder::D2ChunkBuilder,
        d3chunkbuilder::D3ChunkBuilder,
        geometry_object_builder::GeometryObjectBuilder,
        topology_builder::{TopologyBuilder, TopologyScene},
    },
    collision_world::{
        CollisionProbeBlocker, CollisionProbeFloorSample, CollisionProbeResult, CollisionProbeStep,
        CollisionProbeStepKind, CollisionWorld,
    },
    edge::Edges,
    hitinfo::HitInfo,
    intodata::IntoDataInput,
    map::{
        Map, MapCamera, MapToolType,
        bbox::BBox,
        block_prop::{
            BlockPropAsset, BlockPropAssetLayer, BlockPropAttachment, BlockPropComponent,
            BlockPropEffectResolution, BlockPropFaceRef, BlockPropGeometryDiagnostic,
            BlockPropGeometryDiagnosticKind, BlockPropGeometryResolution, BlockPropGeometrySource,
            BlockPropHostAttachment, BlockPropInstance, BlockPropInteractionHit,
            BlockPropInteractionTarget, BlockPropLightEffect, BlockPropOccupancyPolicy,
            BlockPropOccupant, BlockPropPart, BlockPropParticleEffect, BlockPropPlacementMode,
            BlockPropPlacementProfile, BlockPropSemanticShape, BlockPropSupportSurface,
            BlockPropSupportSurfaceHit, BlockPropSurfacePlacement, BlockPropTransform,
            ResolvedBlockPropLightEffect, ResolvedBlockPropParticleEffect,
            block_prop_asset_material_slots, block_prop_door_controls_part,
            block_prop_door_is_open, block_prop_instance_object_id, block_prop_interaction_verb,
            block_prop_interaction_world_anchor, block_prop_material_override_key,
            block_prop_part_world_anchor, block_prop_support_surface_local_point,
            block_prop_support_surface_world_point, block_prop_support_surface_world_transform,
            block_prop_surface_placement_world_position, identity_block_prop_transform,
            multiply_block_prop_transforms, resolve_block_prop_asset, resolve_block_prop_effects,
            resolve_block_prop_geometry, resolve_block_prop_interaction_hit,
            resolve_block_prop_preview_geometry, resolve_block_prop_support_surface_hit,
            resolve_block_prop_support_surface_hit_at_point,
            resolve_block_prop_support_surface_hit_at_world_point, set_block_prop_door_open,
            sync_block_prop_surface_item_positions,
        },
        geometry_object::{
            FaceEmission, FaceParticleEmission, GeometryFace, GeometryObject, GeometryObjectKind,
            GeometrySurfacePoint, GeometrySurfacePointMode, GeometrySurfaceSegment,
            GeometrySurfaceSegmentMode, geometry_face_effective_paint_surface_id,
            geometry_face_paint_uvs, remap_geometry_face_paint_uvs, triangulate_geometry_polygon,
        },
        light::CompiledLight,
        light::Light,
        light::LightType,
        linedef::CompiledLinedef,
        linedef::Linedef,
        meta::MapMeta,
        mini::MapMini,
        organic::{
            OrganicBushCluster, OrganicGrowthShape, OrganicVineStroke,
            default_organic_bush_clusters, default_organic_vine_strokes,
        },
        particle::{Particle, ParticleEmissionShape, ParticleEmitter, ParticleEmitterDef},
        pixelsource::NoiseTarget,
        pixelsource::PixelSource,
        sector::Sector,
        surface::{BillboardAnimation, LoopOp, ProfileLoop, Surface},
        tile::{
            Tile, TileAttachment, TileBoxGeometry, TileGeometryFeature, TileGeometryOperation,
            TileLightEffect, TileMaterialMeta, TileNicheGeometry, TileParticleEffect,
            TileRecipePlacement, TileRole,
        },
        tilesource::{TileGroup, TileGroupMemberRef, TileSource},
        topology::MapTopology,
        vertex::Vertex,
        wall::{
            WallAreaSurface, WallAreaSurfacePreview, WallAssembly, WallBrickKey, WallBrickPreview,
            WallGeometryLayer, WallJunctionKind, WallMasonryPattern, WallNode, WallOpening,
            WallOpeningFrame, WallOpeningPreview, WallOpeningShape, WallOpeningSurround, WallSpan,
            WallStyle, WallSurfaceEdge,
        },
    },
    material_profile::MaterialProfile,
    rasterizer::{BrushPreview, Rasterizer},
    ray::Ray,
    rect::Rect,
    rendermode::RenderMode,
    scene::Scene,
    scene_build_index::SceneBuildIndex,
    scenebuilder::{d2builder::D2Builder, d2material::D2MaterialBuilder},
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
        message::{Choice, MultipleChoice, PaletteRemap2DState, PlayerCamera, RegionMessage},
        region::RegionInstance,
        regionctx::RegionCtx,
    },
    texture::{RepeatMode, SampleMode, Texture},
    value::{HeightControlPoint, Value, ValueContainer},
    value_toml::{ValueGroups, ValueTomlLoader},
    vertexblend::VertexBlendPreset,
};

// Prelude
pub mod prelude {
    pub use crate::Chunk;
    #[cfg(feature = "graphics")]
    pub use crate::Client;
    pub use crate::IntoDataInput;
    pub use crate::audio::{AudioConfig, AudioEngine, AudioError, OutputInfo, SineVoiceId};
    pub use crate::{
        Avatar, AvatarAnimation, AvatarAnimationFrame, AvatarBuildOutput, AvatarBuildRequest,
        AvatarBuilder, AvatarDirection, AvatarMarkerColors, AvatarPerspective,
        AvatarPerspectiveCount, AvatarShadingOptions,
    };
    // pub use crate::MapScript;
    pub use crate::Rasterizer;
    pub use crate::RenderMode;
    pub use crate::scenebuilder::{d2builder::D2Builder, d2material::D2MaterialBuilder};
    #[cfg(feature = "graphics")]
    pub use crate::scenebuilder::{d2preview::D2PreviewBuilder, d3builder::D3Builder};
    pub use crate::vm::{EldrinDebugEntry, EldrinDebugFrame, EldrinDebugModule, EldrinDebugTarget};
    pub use crate::{
        Assets, Choice, Currencies, Currency, Entity, EntityUpdate, Item, ItemUpdate,
        MultipleChoice, PaletteRemap2DState, RegionInstance, RegionMessage, Server, Wallet,
    };
    pub use crate::{BLACK, Pixel, TRANSPARENT, WHITE};
    pub use crate::{Batch2D, Batch3D, CullMode, GeometrySource, PrimitiveMode};
    pub use crate::{
        BlockPropAsset, BlockPropAssetLayer, BlockPropAttachment, BlockPropComponent,
        BlockPropEffectResolution, BlockPropFaceRef, BlockPropGeometryDiagnostic,
        BlockPropGeometryDiagnosticKind, BlockPropGeometryResolution, BlockPropGeometrySource,
        BlockPropHostAttachment, BlockPropInstance, BlockPropInteractionHit,
        BlockPropInteractionTarget, BlockPropLightEffect, BlockPropOccupancyPolicy,
        BlockPropOccupant, BlockPropPart, BlockPropParticleEffect, BlockPropPlacementMode,
        BlockPropPlacementProfile, BlockPropSemanticShape, BlockPropSupportSurface,
        BlockPropSupportSurfaceHit, BlockPropSurfacePlacement, BlockPropTransform, FaceEmission,
        FaceParticleEmission, Light, LightType, Map, MapMeta, MapToolType, NoiseTarget,
        OrganicBushCluster, OrganicGrowthShape, OrganicVineStroke, Particle, ParticleEmitter,
        ParticleEmitterDef, PixelSource, ResolvedBlockPropLightEffect,
        ResolvedBlockPropParticleEffect, Sector, Tile, TileAttachment, TileBoxGeometry,
        TileGeometryFeature, TileGeometryOperation, TileGroup, TileGroupMemberRef, TileLightEffect,
        TileNicheGeometry, TileParticleEffect, TileRole, TileSource, Vertex, WallAreaSurface,
        WallAreaSurfacePreview, WallAssembly, WallBrickKey, WallBrickPreview, WallGeometryLayer,
        WallJunctionKind, WallMasonryPattern, WallNode, WallOpening, WallOpeningFrame,
        WallOpeningPreview, WallOpeningShape, WallOpeningSurround, WallSpan, WallStyle,
        WallSurfaceEdge, block_prop_asset_material_slots, block_prop_door_controls_part,
        block_prop_interaction_verb, block_prop_material_override_key,
        block_prop_part_world_anchor, block_prop_support_surface_local_point,
        block_prop_support_surface_world_point, block_prop_support_surface_world_transform,
        block_prop_surface_placement_world_position, default_organic_bush_clusters,
        default_organic_vine_strokes, resolve_block_prop_effects,
        resolve_block_prop_interaction_hit, resolve_block_prop_support_surface_hit,
        resolve_block_prop_support_surface_hit_at_point,
        resolve_block_prop_support_surface_hit_at_world_point,
        sync_block_prop_surface_item_positions,
    };
    #[cfg(feature = "graphics")]
    pub use crate::{Command, Daylight, MsgParser, Tok};
    pub use crate::{D3Camera, D3FirstPCamera, D3IsoCamera, D3OrbitCamera};
    #[cfg(feature = "graphics")]
    pub use crate::{
        ParticleDebugStats, ParticlePipelineStats, RenderSettings, Rusterix, SceneHandler,
    };
    pub use crate::{
        Rect, Scene, SceneManager, SceneManagerCmd, SceneManagerResult, Value, ValueContainer,
    };
    pub use crate::{RepeatMode, SampleMode, Texture};
    pub use crate::{pixel_to_vec4, vec4_to_pixel};
}
