use crate::{Camera3D, Chunk, Light, Line3D, Poly2D, Poly3D, dynamic::DynamicObject};
use bytemuck::{Pod, Zeroable};
use std::hash::{Hash, Hasher};
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
    GeometryObject(Uuid),
    Hole(u32, u32),
    Gizmo(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct PaintSurfacePixel {
    pub valid: bool,
    pub geo_id: GeoId,
    pub paint_geo: [u32; 4],
    pub face_id: u32,
    pub depth: f32,
    pub world: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Default for PaintSurfacePixel {
    fn default() -> Self {
        Self {
            valid: false,
            geo_id: GeoId::Unknown(0),
            paint_geo: [0; 4],
            face_id: u32::MAX,
            depth: f32::INFINITY,
            world: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaintSurfaceBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<PaintSurfacePixel>,
}

impl PaintSurfaceBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![PaintSurfacePixel::default(); width as usize * height as usize],
        }
    }

    pub fn pixel(&self, x: i32, y: i32) -> Option<&PaintSurfacePixel> {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return None;
        }
        self.pixels
            .get(y as usize * self.width as usize + x as usize)
    }

    pub fn content_key(&self) -> u64 {
        let mut hasher = rustc_hash::FxHasher::default();
        self.width.hash(&mut hasher);
        self.height.hash(&mut hasher);
        let stride = (self.pixels.len() / 4096).max(1);
        let mut valid_count = 0usize;
        for (index, pixel) in self.pixels.iter().enumerate() {
            if !pixel.valid {
                continue;
            }
            valid_count += 1;
            if index % stride != 0 {
                continue;
            }
            index.hash(&mut hasher);
            pixel.geo_id.hash(&mut hasher);
            pixel.face_id.hash(&mut hasher);
            pixel.depth.to_bits().hash(&mut hasher);
        }
        valid_count.hash(&mut hasher);
        hasher.finish()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct Raster3DSurfacePaintEntry {
    pub geo: [u32; 4],
    pub uv_origin: [i32; 2],
    pub uv_size: [u32; 2],
    pub atlas_rect: [u32; 4],
}

pub fn pack_raster3d_paint_geo_id(geo_id: GeoId) -> [u32; 4] {
    match geo_id {
        GeoId::Unknown(id) => [1, id, 0, 0],
        GeoId::Vertex(id) => [2, id, 0, 0],
        GeoId::Linedef(id) => [3, id, 0, 0],
        GeoId::Sector(id) => [4, id, 0, 0],
        GeoId::Character(id) => [5, id, 0, 0],
        GeoId::Item(id) => [6, id, 0, 0],
        GeoId::Light(id) => [7, id, 0, 0],
        GeoId::ItemLight(id) => [8, id, 0, 0],
        GeoId::Triangle(id) => [9, id, 0, 0],
        GeoId::Terrain(x, z) => [10, x as u32, z as u32, 0],
        GeoId::GeometryObject(id) => {
            let value = id.as_u128();
            [
                11,
                (value & 0xffff_ffff) as u32,
                ((value >> 32) & 0xffff_ffff) as u32,
                ((value >> 64) & 0xffff_ffff) as u32,
            ]
        }
        GeoId::Hole(sector_id, hole_id) => [12, sector_id, hole_id, 0],
        GeoId::Gizmo(id) => [13, id, 0, 0],
    }
}

/// Reconstruct the pre-persistent-surface paint identity for one planar 3D
/// surface. This remains public so editors can migrate authored paint without
/// changing its visible placement.
pub fn legacy_raster3d_paint_surface_id(
    geo_id: GeoId,
    layer: i32,
    normal: [f32; 3],
    point: [f32; 3],
) -> [u32; 4] {
    let mut hasher = rustc_hash::FxHasher::default();
    geo_id.hash(&mut hasher);
    layer.hash(&mut hasher);
    let mut normal = normal;
    let dominant = if normal[0].abs() >= normal[1].abs() && normal[0].abs() >= normal[2].abs() {
        normal[0]
    } else if normal[1].abs() >= normal[2].abs() {
        normal[1]
    } else {
        normal[2]
    };
    if dominant < 0.0 {
        normal = [-normal[0], -normal[1], -normal[2]];
    }
    for value in normal {
        ((value * 4096.0).round() as i32).hash(&mut hasher);
    }
    let plane_offset = normal[0] * point[0] + normal[1] * point[1] + normal[2] * point[2];
    ((plane_offset * 1024.0).round() as i64).hash(&mut hasher);
    let hash = hasher.finish();
    let group = (((hash as u32) ^ ((hash >> 32) as u32)) & 0x3fff_ffff) << 2;
    let axis = {
        let x = normal[0].abs();
        let y = normal[1].abs();
        let z = normal[2].abs();
        if y >= x && y >= z {
            0
        } else if x >= z {
            1
        } else {
            2
        }
    };
    let mut paint_geo = pack_raster3d_paint_geo_id(geo_id);
    paint_geo[3] = group | axis;
    paint_geo
}

/// World-projected coordinates used by legacy planar 3D paint.
pub fn legacy_raster3d_paint_surface_uv(world: [f32; 3], paint_geo: [u32; 4]) -> [f32; 2] {
    match paint_geo[3] & 0x3 {
        0 => [world[0], world[2]],
        1 => [world[2], world[1]],
        _ => [world[0], world[1]],
    }
}

#[derive(Debug, Clone)]
pub struct Raster3DPaintGpuStroke {
    pub brush_width: u32,
    pub brush_height: u32,
    pub brush_rgba: Vec<u8>,
    pub draw_x: i32,
    pub draw_y: i32,
    pub draw_width: u32,
    pub draw_height: u32,
    pub scale: f32,
    pub clip_mode: u32,
    pub start_screen: Option<[i32; 2]>,
    pub clip_geo_id: Option<GeoId>,
    pub color_coverage_scale: f32,
    pub replace_material: bool,
    pub replace_opacity: u8,
    pub writes_material: bool,
    pub material_id: u8,
    pub erase: bool,
}

#[derive(Debug, Clone)]
pub struct Raster3DPaintGpuSurface {
    pub width: u32,
    pub height: u32,
    pub geo_rgba: Vec<[u32; 4]>,
}

impl Raster3DPaintGpuSurface {
    pub fn from_paint_surface(surface: &PaintSurfaceBuffer) -> Self {
        Self {
            width: surface.width,
            height: surface.height,
            geo_rgba: surface
                .pixels
                .iter()
                .map(|pixel| pack_raster3d_paint_geo_id(pixel.geo_id))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrganicBillboardSprite {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct OrganicBillboardInstance {
    pub center: [f32; 3],
    pub width: f32,
    pub height: f32,
    pub sprite_index: u32,
    pub flags: u32,
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
    SetMaterialTable(Vec<[f32; 4]>),
    SetRaster3DPaintOverlay {
        width: u32,
        height: u32,
        color_rgba: Vec<u8>,
        material_rgba: Vec<u8>,
        paint_alpha_geo_ids: Vec<GeoId>,
    },
    SetRaster3DSurfacePaint {
        width: u32,
        height: u32,
        color_rgba: Vec<u8>,
        material_rgba: Vec<u8>,
        entries: Vec<Raster3DSurfacePaintEntry>,
        paint_alpha_geo_ids: Vec<GeoId>,
    },
    ClearRaster3DPaintOverlay,
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
    AddLine3D {
        line: Line3D,
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
    SetOrganicVisible {
        visible: bool,
    },
    SetOrganicBillboards {
        sprites: Vec<OrganicBillboardSprite>,
        instances: Vec<OrganicBillboardInstance>,
    },
    ClearOrganicBillboards,
    SetPaintBillboards {
        sprites: Vec<OrganicBillboardSprite>,
        instances: Vec<OrganicBillboardInstance>,
    },
    ClearPaintBillboards,
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
    pub particle_dynamics: usize,
    pub dropped_particle_dynamics: usize,
    pub particle_dynamic_budget: usize,
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
