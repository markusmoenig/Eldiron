use crate::{Camera3D, Chunk, Light, Poly2D, Poly3D, dynamic::DynamicObject};
use uuid::Uuid;
use vek::{Mat3, Vec2, Vec4};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The Geometry Identifier for polygons and triangles.
pub enum GeoId {
    Unknown(u32),
    Vertex(u32),
    Linedef(u32),
    Sector(u32),
    Character(u32),
    Item(u32),
    Light(u32),
    ItemLight(u32),
    Triangle(u32),
    Terrain(i32, i32),
    Hole(u32, u32),
    Gizmo(u32),
}

/// VM instruction set
#[derive(Debug)]
pub enum Atom {
    AddTile {
        id: Uuid,
        width: u32,
        height: u32,
        frames: Vec<Vec<u8>>,
        material_frames: Option<Vec<Vec<u8>>>,
    },
    SetTileMaterialFrames {
        id: Uuid,
        frames: Vec<Vec<u8>>,
    },
    AddSolid {
        id: Uuid,
        color: [u8; 4],
    },
    AddSolidWithMaterial {
        id: Uuid,
        color: [u8; 4],
        material: [u8; 4],
    },
    BuildAtlas,
    SetAtlasSize {
        width: u32,
        height: u32,
    },
    AddPoly {
        poly: Poly2D,
    },
    AddPoly3D {
        poly: Poly3D,
    },
    AddLineStrip2D {
        id: GeoId,
        tile_id: Uuid,
        points: Vec<[f32; 2]>,
        width: f32,
    },
    AddLineStrip2Dpx {
        id: GeoId,
        tile_id: Uuid,
        points: Vec<[f32; 2]>,
        width_px: f32,
    },
    NewChunk {
        id: Uuid,
    },
    AddChunk {
        id: Uuid,
        chunk: Chunk,
    },
    RemoveChunk {
        id: Uuid,
    },
    RemoveChunkAt {
        origin: Vec2<i32>,
    },
    SetCurrentChunk {
        id: Uuid,
    },
    SetAnimationCounter(usize),
    SetBackground(Vec4<f32>),
    SetGP0(Vec4<f32>),
    SetGP1(Vec4<f32>),
    SetGP2(Vec4<f32>),
    SetGP3(Vec4<f32>),
    SetGP4(Vec4<f32>),
    SetGP5(Vec4<f32>),
    SetGP6(Vec4<f32>),
    SetGP7(Vec4<f32>),
    SetGP8(Vec4<f32>),
    SetGP9(Vec4<f32>),
    SetPaletteRemap2D {
        start_index: u32,
        end_index: u32,
        mode: PaletteRemap2DMode,
    },
    SetPaletteRemap2DBlend(f32),
    SetRaster3DMsaaSamples(u32),
    SetRenderMode(RenderMode),
    SetPalette(Vec<Vec4<f32>>),
    SetTransform2D(Mat3<f32>),
    SetTransform3D(vek::Mat4<f32>),
    SetLayer(i32),
    SetGeoVisible {
        id: GeoId,
        visible: bool,
    },
    SetGeoOpacity {
        id: GeoId,
        opacity: f32,
    },
    SetSource2D(String),
    SetViewportRect2D(Option<[f32; 4]>),
    SetSource3D(String),
    SetSourceSdf(String),
    SetSdfData(Vec<[f32; 4]>),
    Clear,
    ClearTiles,
    ClearGeometry,
    AddLight {
        id: GeoId,
        light: Light,
    },
    RemoveLight {
        id: GeoId,
    },
    ClearLights,
    ClearDynamics,
    AddDynamic {
        object: DynamicObject,
    },
    SetAvatarBillboardData {
        id: GeoId,
        size: u32,
        rgba: Vec<u8>,
    },
    SetOrganicSurfaceDetail {
        surface_id: Uuid,
        size: u32,
        rgba: Vec<u8>,
    },
    SetOrganicSurfaceDetailRect {
        surface_id: Uuid,
        size: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    RemoveAvatarBillboardData {
        id: GeoId,
    },
    ClearAvatarBillboardData,
    SetBvhLeafSize {
        max_tris: u32,
    },
    SetCamera3D {
        camera: Camera3D,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum PaletteRemap2DMode {
    #[default]
    Disabled = 0,
    LumaRamp = 1,
    Nearest = 2,
    DitheredRamp = 3,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VMDebugStats {
    pub chunks: usize,
    pub polys2d: usize,
    pub polys3d: usize,
    pub tris3d: usize,
    pub lines2d: usize,
    pub dynamics: usize,
    pub lights: usize,
    pub cached_v3: usize,
    pub cached_i3: usize,
    pub accel_dirty: bool,
    pub visibility_dirty: bool,
    pub geometry3d_dirty: bool,
    pub geometry2d_dirty: bool,
}

/// Screen-space line strip description (width in pixels; rendered as quads built in screen space).
#[derive(Debug, Clone)]
pub struct LineStrip2D {
    pub id: GeoId,
    pub tile_id: uuid::Uuid,
    pub points: Vec<[f32; 2]>,
    pub width_px: f32,
    pub layer: i32,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Compute2D,
    Raster2D,
    Compute3D,
    Raster3D,
    Sdf,
}

/// How a VM layer should be composited over the previous result.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LayerBlendMode {
    Alpha,
    AlphaLinear,
}
