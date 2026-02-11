// Non-empty dummy buffers for wgpu STORAGE bindings when a scene grid is empty.
const DUMMY_U32_1: [u32; 1] = [0];

use crate::{
    Camera3D, CameraKind, Chunk, Light, LightType, Poly2D, Poly3D, Texture,
    atlas::{AtlasEntry, AtlasGpuTables, SharedAtlas, default_material_frame},
    dynamic::{DynamicKind, DynamicObject},
};
use bytemuck::{Pod, Zeroable};
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use uuid::Uuid;
use vek::{Mat3, Mat4, Vec2, Vec3, Vec4};

// --- Scene-wide acceleration structure (BVH over all 3D geometry) ---
#[derive(Debug, Clone, Default)]
pub struct SceneBvhAccel {
    /// World-space minimum of the scene AABB (root bounds min).
    pub origin: vek::Vec3<f32>,
    /// Extent of the scene AABB (root bounds max = origin + extent).
    pub extent: vek::Vec3<f32>,
    /// Flattened BVH node data, packed as u32 words for the GPU buffer.
    pub nodes: Vec<u32>,
    /// Triangle indices referenced by BVH leaves.
    pub tri_indices: Vec<u32>,
    pub node_count: u32,
    pub tri_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SceneAccel {
    pub bvh: SceneBvhAccel,
}

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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vert2DPod {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub tile_index: u32,
    pub _pad_tile: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TileBinPod {
    pub offset: u32,
    pub count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vert3DPod {
    pub pos: [f32; 3],
    pub _pad_pos: f32,     // 16
    pub uv: [f32; 2],      // +8  = 24
    pub _pad_uv: [f32; 2], // +8  = 32
    pub tile_index: u32,   // Primary texture
    pub tile_index2: u32,  // Secondary texture (for blending)
    pub blend_factor: f32, // 0.0=all primary, 1.0=all secondary
    pub _pad_blend: f32,   // Padding
    pub normal: [f32; 3],
    pub _pad_n: f32, // +16 = 64 total (SAME SIZE!)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TileAnimMetaPod {
    pub first_frame: u32,
    pub frame_count: u32,
    _pad: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TileFramePod {
    pub ofs: [f32; 2],
    pub scale: [f32; 2],
}

const DUMMY_TILE_META: TileAnimMetaPod = TileAnimMetaPod {
    first_frame: 0,
    frame_count: 0,
    _pad: [0, 0],
};
const DUMMY_TILE_FRAME: TileFramePod = TileFramePod {
    ofs: [0.0, 0.0],
    scale: [0.0, 0.0],
};
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightPod {
    // header: [light_type, emitting, pad, pad]
    pub header: [u32; 4],
    pub position: [f32; 4], // xyz, pad
    pub color: [f32; 4],    // rgb, pad
    // params0: [intensity, radius, start_distance, end_distance]
    pub params0: [f32; 4],
    // params1: [flicker, pad, pad, pad]
    pub params1: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct SceneDataHeaderPod {
    pub lights_offset_words: u32,
    pub lights_count: u32,
    pub billboard_cmd_offset_words: u32,
    pub billboard_cmd_count: u32,
    pub avatar_meta_offset_words: u32,
    pub avatar_meta_count: u32,
    pub avatar_pixel_offset_words: u32,
    pub data_word_count: u32,
}

#[allow(dead_code)]
const SCENE_LIGHT_WORDS: u32 =
    (std::mem::size_of::<LightPod>() / std::mem::size_of::<u32>()) as u32;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DynamicBillboardPod {
    pub center: [f32; 4],     // xyz + width
    pub axis_right: [f32; 4], // xyz + height
    pub axis_up: [f32; 4],    // xyz + repeat_mode (as f32)
    pub params: [u32; 4],     // tile_index, kind, opacity_bits, unused
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct DynamicAvatarMetaPod {
    pub offset_pixels: u32,
    pub size: u32,
    pub _pad: [u32; 2],
}

#[derive(Clone, Debug, Default)]
struct DynamicAvatarData {
    size: u32,
    rgba: Vec<u8>,
}

#[allow(dead_code)]
const SCENE_BILLBOARD_CMD_WORDS: u32 =
    (std::mem::size_of::<DynamicBillboardPod>() / std::mem::size_of::<u32>()) as u32;

/// VM instruction set
#[derive(Debug)]
pub enum Atom {
    /// Add a tile with `id`, dimensions, and animation frames (RGBA8). Each frame is tightly packed width*height*4 bytes.
    AddTile {
        id: Uuid,
        width: u32,
        height: u32,
        frames: Vec<Vec<u8>>, // frames[f][row*width*4 .. (row+1)*width*4]
        material_frames: Option<Vec<Vec<u8>>>,
    },
    /// Provide or replace per-frame material maps (RGBA = roughness/metallic/opacity/emission) for an existing tile.
    SetTileMaterialFrames {
        id: Uuid,
        frames: Vec<Vec<u8>>,
    },
    /// Add a solid-color 1x1 tile with `id` and RGBA color.
    AddSolid {
        id: Uuid,
        color: [u8; 4],
    },
    /// Add a solid-color 1x1 tile with `id`, RGBA color, and material properties.
    /// Material properties: RGBA = roughness/metallic/opacity/emission
    AddSolidWithMaterial {
        id: Uuid,
        color: [u8; 4],
        material: [u8; 4],
    },
    /// Build the atlas for all frames
    BuildAtlas,
    /// Override atlas dimensions (default is 4096x4096). Rebuilds atlas on next build/upload.
    SetAtlasSize {
        width: u32,
        height: u32,
    },
    /// Add a polygon (world coords) that references a tile by UUID into the CURRENT chunk; indices are local to the chunk.
    AddPoly {
        poly: Poly2D,
    },
    /// Add a 3D polygon (world coords) that references a tile by UUID; indices are local to the chunk.
    AddPoly3D {
        poly: Poly3D,
    },
    /// Add a simple 2D line strip as thick segments tessellated into quads (no caps/joins)
    /// Points are in world coordinates; width is in the same units.
    AddLineStrip2D {
        id: GeoId,
        tile_id: Uuid,
        points: Vec<[f32; 2]>,
        width: f32,
    },
    /// Add a 2D line strip rendered in screen space with a constant pixel width.
    /// Points are in world coordinates; width is in pixels.
    AddLineStrip2Dpx {
        id: GeoId,
        tile_id: Uuid,
        points: Vec<[f32; 2]>,
        width_px: f32,
    },
    /// Create an empty chunk (no switch)
    NewChunk {
        id: Uuid,
    },
    /// Insert or replace an entire chunk in one go (prepared externally, e.g., in Rusterix)
    AddChunk {
        id: Uuid,
        chunk: Chunk,
    },
    /// Remove an existing chunk; if it is the current chunk, unset it
    RemoveChunk {
        id: Uuid,
    },
    /// Remove a chunk at a given origin (in chunk grid coordinates)
    RemoveChunkAt {
        origin: vek::Vec2<i32>,
    },
    /// Switch the current chunk (created if missing)
    SetCurrentChunk {
        id: Uuid,
    },
    /// Set the current animation counter (frame index modulo each tile's frame count)
    SetAnimationCounter(usize),
    /// Set background color/params shared by 2D and 3D
    SetBackground(Vec4<f32>),
    /// General-purpose vec4 slots shared by 2D and 3D
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
    /// Switch between 2D/3D/SDF compute drawing
    SetRenderMode(RenderMode),
    /// Set a 256-entry color palette available in shaders (vec4<f32> entries).
    SetPalette(Vec<Vec4<f32>>),
    /// Set a 2D transform (Mat3) applied on CPU to polygon vertices before 2D compute draw
    SetTransform2D(Mat3<f32>),
    /// Set a 3D transform (Mat4) applied on CPU to polygon vertices before 3D compute draw
    SetTransform3D(Mat4<f32>),
    /// Set current 2D/3D layer for subsequently added geometry
    SetLayer(i32),
    /// Toggle visibility for a specific geometry id across all chunks
    SetGeoVisible {
        id: GeoId,
        visible: bool,
    },
    /// Provide a custom WGSL body for the 2D compute shader. The VM will prepend a header and compile at runtime.
    SetSource2D(String),
    /// Set the viewport rect for the 2D compute shader (x, y, width, height in screen pixels).
    /// If None, uses full screen. The rect is passed to shader via uniforms.
    SetViewportRect2D(Option<[f32; 4]>),
    /// Provide a custom WGSL body for the 3D compute shader. The VM will prepend a header and compile at runtime.
    SetSource3D(String),
    /// Provide a custom WGSL body for the SDF compute shader. The VM will prepend a header and compile at runtime.
    SetSourceSdf(String),
    /// Replace the SDF data buffer (read-only storage) exposed to the shader.
    SetSdfData(Vec<[f32; 4]>),
    /// Clear EVERYTHING: tiles, atlas, scene (chunks), counters and modes
    Clear,
    /// Clear only the tiles and atlas (keep scene/chunks intact)
    ClearTiles,
    /// Clear only the scene geometry (chunks & current selection), keep tiles/atlas intact
    ClearGeometry,
    /// Add a light to the scene
    AddLight {
        id: GeoId,
        light: Light,
    },
    /// Remove a light by its id
    RemoveLight {
        id: GeoId,
    },
    /// Remove all lights from the scene
    ClearLights,
    /// Remove all dynamic billboards for this VM layer.
    ClearDynamics,
    /// Add a dynamic object (billboard, particles, etc.) that is evaluated this frame.
    AddDynamic {
        object: DynamicObject,
    },
    /// Set or replace avatar billboard RGBA data for a GeoId (square size x size).
    SetAvatarBillboardData {
        id: GeoId,
        size: u32,
        rgba: Vec<u8>,
    },
    /// Remove avatar billboard RGBA data for a GeoId.
    RemoveAvatarBillboardData {
        id: GeoId,
    },
    /// Clear all avatar billboard RGBA data.
    ClearAvatarBillboardData,
    /// Set BVH leaf size (max triangles per leaf)
    SetBvhLeafSize {
        max_tris: u32,
    },
    /// Set the camera
    SetCamera3D {
        camera: Camera3D,
    },
}

/// Screen-space line strip description (width in pixels; rendered as quads built in screen space).
#[derive(Debug, Clone)]
pub struct LineStrip2D {
    pub id: GeoId,
    pub tile_id: uuid::Uuid,
    pub points: Vec<[f32; 2]>, // world-space points (will be transformed, then rasterized in screen space)
    pub width_px: f32,         // line width in pixels (constant regardless of world scale)
    pub layer: i32,
    pub visible: bool,
}

// GPU rendering resources managed directly by VM
pub struct VMGpu {
    pub pipeline_2d: wgpu::RenderPipeline,
    pub globals_buf: wgpu::Buffer,
    pub globals_bgl: wgpu::BindGroupLayout,
    pub atlas_bgl: wgpu::BindGroupLayout,
    pub globals_bg: Option<wgpu::BindGroup>,
    pub atlas_bg: Option<wgpu::BindGroup>,
    pub vbuf: Option<wgpu::Buffer>,
    pub ibuf: Option<wgpu::Buffer>,
    pub index_count: u32,
    pub sampler: wgpu::Sampler,
    pub sampler_linear: wgpu::Sampler,
    // --- Compute pipelines and uniforms (lazily created)
    pub compute2d_pipeline: Option<wgpu::ComputePipeline>,
    pub compute3d_pipeline: Option<wgpu::ComputePipeline>,
    pub compute_sdf_pipeline: Option<wgpu::ComputePipeline>,
    pub u2d_buf: Option<wgpu::Buffer>,
    pub u3d_buf: Option<wgpu::Buffer>,
    pub u_sdf_buf: Option<wgpu::Buffer>,
    pub u2d_bgl: Option<wgpu::BindGroupLayout>,
    pub u3d_bgl: Option<wgpu::BindGroupLayout>,
    pub u_sdf_bgl: Option<wgpu::BindGroupLayout>,
    pub u2d_bg: Option<wgpu::BindGroup>,
    pub u3d_bg: Option<wgpu::BindGroup>,
    pub u_sdf_bg: Option<wgpu::BindGroup>,
    pub v2d_ssbo: Option<wgpu::Buffer>,
    pub i2d_ssbo: Option<wgpu::Buffer>,
    pub v3d_ssbo: Option<wgpu::Buffer>,
    pub i3d_ssbo: Option<wgpu::Buffer>,
    // --- Tiling
    pub tile_bins: Option<wgpu::Buffer>,
    pub tile_tris: Option<wgpu::Buffer>,
    pub tile_meta_ssbo: Option<wgpu::Buffer>,
    pub tile_frames_ssbo: Option<wgpu::Buffer>,
    // Scene-wide data (lights, billboards, ...)
    pub scene_data_ssbo: Option<wgpu::Buffer>,
    // --- Scene-wide uniform grid buffers (3D)
    pub grid_hdr: Option<wgpu::Buffer>,
    pub grid_data: Option<wgpu::Buffer>,
    // --- SDF data
    pub sdf_data_ssbo: Option<wgpu::Buffer>,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Globals {
    pub tx: f32,
    pub ty: f32,
    pub scale: f32,
    _pad0: f32,
    pub atlas_w: f32,
    pub atlas_h: f32,
    _pad1: f32,
    _pad2: f32,
}

pub const SCENEVM_2D_WGSL: &str = r#"
struct Globals {
  tx: f32, ty: f32, scale: f32, _pad0: f32,
  atlas_w: f32, atlas_h: f32, _pad1: f32, _pad2: f32,
};
@group(0) @binding(0) var<uniform> G: Globals;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_smp: sampler;

struct VsIn { @location(0) pos: vec2<f32>, @location(1) uv: vec2<f32> };
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex
fn vs_main(in: VsIn) -> VsOut {
  var out: VsOut;
  // Temporary mapping: interpret pos as pixels in an atlas-sized viewport
  let x = (in.pos.x / G.atlas_w) * 2.0 - 1.0;
  let y = (in.pos.y / G.atlas_h) * -2.0 + 1.0;
  out.pos = vec4<f32>(x, y, 0.0, 1.0);
  out.uv = in.uv;
  return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  return textureSample(atlas_tex, atlas_smp, in.uv);
}
"#;

// --- Compute pipeline uniforms and WGSL shaders ---
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Compute2DUniforms {
    pub background: [f32; 4], // was param
    pub fb_size: [u32; 2],
    _pad0: [u32; 2],
    pub gp0: [f32; 4], // general-purpose vec4s
    pub gp1: [f32; 4],
    pub gp2: [f32; 4],
    pub gp3: [f32; 4],
    pub gp4: [f32; 4],
    pub gp5: [f32; 4],
    pub gp6: [f32; 4],
    pub gp7: [f32; 4],
    pub gp8: [f32; 4],
    pub gp9: [f32; 4],
    // Mat3<f32> as 3 padded vec4 columns (col-major), .w is padding
    pub mat2d_c0: [f32; 4],
    pub mat2d_c1: [f32; 4],
    pub mat2d_c2: [f32; 4],
    // Inverse 2D matrix columns
    pub mat2d_inv_c0: [f32; 4],
    pub mat2d_inv_c1: [f32; 4],
    pub mat2d_inv_c2: [f32; 4],

    pub lights_count: u32,
    pub vm_flags: u32,
    pub anim_counter: u32,
    pub _pad_lights: u32,
    // Viewport rect: [x, y, width, height] in screen pixels. If width=0, use full screen.
    pub viewport_rect: [f32; 4],
    pub palette: [[f32; 4]; 256],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Compute3DUniforms {
    pub background: [f32; 4],
    pub fb_size: [u32; 2],
    _pad0: [u32; 2],
    pub gp0: [f32; 4],
    pub gp1: [f32; 4],
    pub gp2: [f32; 4],
    pub gp3: [f32; 4],
    pub gp4: [f32; 4],
    pub gp5: [f32; 4],
    pub gp6: [f32; 4],
    pub gp7: [f32; 4],
    pub gp8: [f32; 4],
    pub gp9: [f32; 4],
    // Mat4<f32> as 4 vec4 columns (col-major)
    pub mat3d_c0: [f32; 4],
    pub mat3d_c1: [f32; 4],
    pub mat3d_c2: [f32; 4],
    pub mat3d_c3: [f32; 4],

    // Lights
    pub lights_count: u32,
    pub vm_flags: u32,
    pub anim_counter: u32,
    pub _pad_lights: u32,

    // Camera3D
    pub cam_pos: [f32; 4], // xyz, pad
    pub cam_fwd: [f32; 4], // xyz, pad
    pub cam_right: [f32; 4],
    pub cam_up: [f32; 4],
    pub cam_vfov_deg: f32,
    pub cam_ortho_half_h: f32,
    pub cam_near: f32,
    pub cam_far: f32,
    pub cam_kind: u32, // 0=OrthoIso, 1=OrbitPersp, 2=FirstPersonPersp
    _pad_cam: [u32; 3],

    pub _pad_tail: [u32; 4],
    pub palette: [[f32; 4]; 256],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ComputeSdfUniforms {
    pub background: [f32; 4],
    pub fb_size: [u32; 2],
    _pad0: [u32; 2],
    pub gp0: [f32; 4],
    pub gp1: [f32; 4],
    pub gp2: [f32; 4],
    pub gp3: [f32; 4],
    pub gp4: [f32; 4],
    pub gp5: [f32; 4],
    pub gp6: [f32; 4],
    pub gp7: [f32; 4],
    pub gp8: [f32; 4],
    pub gp9: [f32; 4],
    // Camera (matches Compute3DUniforms layout to keep alignment safe)
    pub cam_pos: [f32; 4],
    pub cam_fwd: [f32; 4],
    pub cam_right: [f32; 4],
    pub cam_up: [f32; 4],
    pub cam_vfov_deg: f32,
    pub cam_ortho_half_h: f32,
    pub cam_near: f32,
    pub cam_far: f32,
    pub cam_kind: u32, // 0=OrthoIso, 1=OrbitPersp, 2=FirstPersonPersp
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
    pub data_len: u32,
    pub vm_flags: u32,
    pub anim_counter: u32,
    pub _pad4: u32,
    pub viewport_rect: [f32; 4], // [x, y, width, height] in screen pixels. width=0 disables scissor.
    pub palette: [[f32; 4]; 256],
    pub _pad_end: [[u32; 4]; 4], // 64 bytes of padding to match WGSL std140 layout (4512 bytes total)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Grid3DHeader {
    pub origin: [f32; 4],     // xyz, pad
    pub cell_size: [f32; 4],  // xyz, pad
    pub dims: [u32; 4],       // nx, ny, nz, pad
    pub ranges: [u32; 4],     // nodes_start, tris_start, node_count, tri_count
    pub visibility: [u32; 4], // vis_start, vis_word_count, pad, pad
}

pub const SCENEVM_2D_CS_WGSL: &str = r#"
struct U2D { background: vec4<f32>, fb_size: vec2<u32>, _pad: vec2<u32> };
@group(0) @binding(0) var<uniform> U: U2D;
@group(0) @binding(1) var color_out: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= U.fb_size.x || gid.y >= U.fb_size.y) { return; }
  let uv = vec2<f32>(f32(gid.x)/f32(U.fb_size.x), f32(gid.y)/f32(U.fb_size.y));
  // For now: solid color with simple uv tint; later: raster & lighting
  let col = /*vec4<f32>(U.background.xyz, 1.0); */ vec4<f32>(uv.x, uv.y, 0.0, 1.0);
  textureStore(color_out, vec2<i32>(i32(gid.x), i32(gid.y)), col);
}
"#;

pub const SCENEVM_3D_CS_WGSL: &str = r#"
struct U3D { background: vec4<f32>, fb_size: vec2<u32>, _pad: vec2<u32>, };
@group(0) @binding(0) var<uniform> U: U3D;
@group(0) @binding(1) var color_out: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= U.fb_size.x || gid.y >= U.fb_size.y) { return; }
  // Placeholder: gradient with background.x as brightness; later we pathtrace here
  let uv = vec2<f32>(f32(gid.x)/f32(U.fb_size.x), f32(gid.y)/f32(U.fb_size.y));
  let b = U.background.x;
  let col = vec4<f32>(uv.x*b, uv.y*b, b, 1.0);
  textureStore(color_out, vec2<i32>(i32(gid.x), i32(gid.y)), col);
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Compute2D,
    Compute3D,
    Sdf,
}

/// How a VM layer should be composited over the previous result.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LayerBlendMode {
    /// Default alpha blending in sRGB.
    Alpha,
    /// Decode destination to linear, blend in linear, encode back to sRGB. Useful for accum/displays.
    AlphaLinear,
}

pub struct VM {
    shared_atlas: SharedAtlas,
    pub chunks_map: FxHashMap<Uuid, Chunk>,
    pub current_chunk: Option<Uuid>,

    pub animation_counter: usize,
    pub render_mode: RenderMode,
    pub blend_mode: LayerBlendMode,

    pub gpu: Option<VMGpu>,
    // Intermediate texture for this VM layer (for compositing)
    pub layer_texture: Option<crate::Texture>,
    // Optional ping-pong textures when enabled (front index selects current composited view)
    ping_pong_textures: Option<[crate::Texture; 2]>,
    ping_pong_front: usize,
    ping_pong_enabled: bool,
    prev_dummy: Option<crate::Texture>,
    // --- Compute pipeline params (shared by 2D/3D)
    pub background: Vec4<f32>,
    pub gp0: Vec4<f32>,
    pub gp1: Vec4<f32>,
    pub gp2: Vec4<f32>,
    pub gp3: Vec4<f32>,
    pub gp4: Vec4<f32>,
    pub gp5: Vec4<f32>,
    pub gp6: Vec4<f32>,
    pub gp7: Vec4<f32>,
    pub gp8: Vec4<f32>,
    pub gp9: Vec4<f32>,
    // --- Programmable compute shader sources
    pub source2d: String,
    pub viewport_rect2d: Option<[f32; 4]>, // Optional viewport rect for 2D shader (x, y, w, h)
    pub source3d: String,
    pub source_sdf: String,
    pub sdf_data: Vec<[f32; 4]>,
    pub sdf_data_dirty: bool,
    pub palette: [[f32; 4]; 256],
    pub palette_dirty: bool,

    pub transform2d: Mat3<f32>,
    pub transform3d: Mat4<f32>,

    pub lights: FxHashMap<GeoId, Light>,
    dynamic_objects: Vec<DynamicObject>,
    dynamic_avatar_objects: FxHashMap<GeoId, DynamicObject>,
    dynamic_avatar_data: FxHashMap<GeoId, DynamicAvatarData>,

    pub current_layer: i32,

    // Scene-wide 3D acceleration via BVH
    pub bvh_leaf_size: u32,
    pub scene_accel: SceneAccel,
    pub accel_dirty: bool,
    cached_v3: Vec<Vert3DPod>,
    cached_i3: Vec<u32>,
    cached_tri_visibility: Vec<u32>, // Per-triangle visibility bitmask (1 bit per triangle)
    visibility_dirty: bool,          // True when only visibility changed (no BVH rebuild needed)
    geometry2d_dirty: bool,
    cached_v2: Vec<Vert2DPod>,
    cached_i2: Vec<u32>,
    cached_tile_bins: Vec<TileBinPod>,
    cached_tile_tris: Vec<u32>,
    cached_fb_size_2d: (u32, u32),
    cached_tile_anim_meta: Vec<TileAnimMetaPod>,
    cached_tile_frame_data: Vec<TileFramePod>,
    cached_atlas_layout_version: u64,
    tile_gpu_dirty: bool,

    // Camera
    pub camera3d: Camera3D,

    pub enabled: bool,
    layer_index: usize,
    activity_logging: bool,
}

impl VM {
    #[inline]
    fn mark_2d_dirty(&mut self) {
        self.geometry2d_dirty = true;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable/disable ping-pong rendering for this VM. Disabling drops the extra textures.
    pub fn set_ping_pong_enabled(&mut self, enabled: bool) {
        if self.ping_pong_enabled != enabled {
            self.ping_pong_enabled = enabled;
            self.ping_pong_front = 0;
            if !enabled {
                self.ping_pong_textures = None;
            }
        }
    }

    pub fn ping_pong_enabled(&self) -> bool {
        self.ping_pong_enabled
    }

    fn ensure_prev_dummy(&mut self, device: &wgpu::Device) -> wgpu::TextureView {
        if self.prev_dummy.is_none() {
            let mut tex = crate::Texture::new(1, 1);
            tex.data = vec![0, 0, 0, 0];
            tex.ensure_gpu_with(device);
            self.prev_dummy = Some(tex);
        }
        self.prev_dummy
            .as_ref()
            .unwrap()
            .gpu
            .as_ref()
            .unwrap()
            .view
            .clone()
    }

    /// Ensure the layer texture(s) exist and match the given size
    pub(crate) fn ensure_layer_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.ping_pong_enabled {
            let needs_recreate = self
                .ping_pong_textures
                .as_ref()
                .map(|pair| {
                    pair[0].width != width
                        || pair[0].height != height
                        || pair[1].width != width
                        || pair[1].height != height
                })
                .unwrap_or(true);

            if needs_recreate {
                let mut a = crate::Texture::new(width, height);
                let mut b = crate::Texture::new(width, height);
                a.ensure_gpu_with(device);
                b.ensure_gpu_with(device);
                self.ping_pong_textures = Some([a, b]);
                self.ping_pong_front = 0;
            }
        } else {
            let needs_recreate = match &self.layer_texture {
                None => true,
                Some(tex) => tex.width != width || tex.height != height,
            };

            if needs_recreate {
                let mut tex = crate::Texture::new(width, height);
                tex.ensure_gpu_with(device);
                self.layer_texture = Some(tex);
            }
        }
    }

    /// View for compositing (current front buffer)
    pub(crate) fn composite_texture(&self) -> Option<&crate::Texture> {
        if self.ping_pong_enabled {
            if self.activity_logging {
                println!(
                    "[VM Layer {}] composite_texture: returning buffer[{}]",
                    self.layer_index, self.ping_pong_front
                );
            }
            self.ping_pong_textures
                .as_ref()
                .map(|pair| &pair[self.ping_pong_front])
        } else {
            self.layer_texture.as_ref()
        }
    }

    /// Returns write/view pair plus the index that will become the new front after this frame.
    fn prepare_layer_views(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> (wgpu::TextureView, wgpu::TextureView, usize) {
        self.ensure_layer_texture(device, width, height);
        let bg = self.background;

        if self.ping_pong_enabled {
            let pair = self
                .ping_pong_textures
                .as_ref()
                .expect("ping-pong textures should exist when enabled");
            let read_idx = self.ping_pong_front;
            let write_idx = 1 - read_idx;

            if self.activity_logging {
                println!(
                    "[VM Layer {}] prepare_layer_views: front={}, read_idx={}, write_idx={}, anim_counter={}",
                    self.layer_index,
                    self.ping_pong_front,
                    read_idx,
                    write_idx,
                    self.animation_counter
                );
            }

            let read_view = pair[read_idx].gpu.as_ref().unwrap().view.clone();
            let write_view = pair[write_idx].gpu.as_ref().unwrap().view.clone();

            // Clear both buffers on the very first frame so the sampled prev layer is not garbage.
            if self.animation_counter == 0 {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("clear-pingpong-layers"),
                });

                // Clear the write buffer
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear-pingpong-write"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &write_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: bg.x as f64,
                                g: bg.y as f64,
                                b: bg.z as f64,
                                a: bg.w as f64,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // Clear the read buffer (prev frame) to avoid garbage on first frame
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear-pingpong-read"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &read_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: bg.x as f64,
                                g: bg.y as f64,
                                b: bg.z as f64,
                                a: bg.w as f64,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                queue.submit(Some(encoder.finish()));
            }

            (write_view, read_view, write_idx)
        } else {
            let view = self
                .layer_texture
                .as_ref()
                .expect("layer texture should exist")
                .gpu
                .as_ref()
                .unwrap()
                .view
                .clone();
            let prev_view = self.ensure_prev_dummy(device);

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clear-layer"),
            });
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-layer-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg.x as f64,
                            g: bg.y as f64,
                            b: bg.z as f64,
                            a: bg.w as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            queue.submit(Some(encoder.finish()));

            (view.clone(), prev_view, self.ping_pong_front)
        }
    }

    pub fn set_layer_index(&mut self, index: usize) {
        self.layer_index = index;
    }

    pub fn set_activity_logging(&mut self, enabled: bool) {
        self.activity_logging = enabled;
    }

    /// Get tile data for creating tinted copies
    pub fn get_tile_data(&self, id: uuid::Uuid) -> Option<(u32, u32, Vec<u8>)> {
        self.shared_atlas.get_tile_data(id)
    }

    fn log_layer<S: AsRef<str>>(&self, msg: S) {
        if self.activity_logging {
            println!("[SceneVM][Layer {}] {}", self.layer_index, msg.as_ref());
        }
    }

    pub fn set_blend_mode(&mut self, mode: LayerBlendMode) {
        self.blend_mode = mode;
    }

    pub fn blend_mode(&self) -> LayerBlendMode {
        self.blend_mode
    }

    fn sanitize_billboard_axes(
        view_right: Vec3<f32>,
        view_up: Vec3<f32>,
    ) -> (Vec3<f32>, Vec3<f32>) {
        let right = if view_right.magnitude() < 1e-5 || !view_right.magnitude().is_finite() {
            Vec3::unit_x()
        } else {
            view_right / view_right.magnitude()
        };

        let mut up = if view_up.magnitude() < 1e-5 || !view_up.magnitude().is_finite() {
            Vec3::unit_y()
        } else {
            view_up / view_up.magnitude()
        };

        // Remove any component along right to keep basis orthogonal
        up = up - right * right.dot(up);
        let up_len = up.magnitude();
        if up_len < 1e-5 || !up_len.is_finite() {
            let mut fallback = if right.y.abs() < 0.9 {
                Vec3::unit_y()
            } else {
                Vec3::unit_z()
            };
            fallback = fallback - right * right.dot(fallback);
            let fb_len = fallback.magnitude();
            up = if fb_len < 1e-5 || !fb_len.is_finite() {
                Vec3::unit_z()
            } else {
                fallback / fb_len
            };
        } else {
            up /= up_len;
        }

        (right, up)
    }

    fn push_dynamic_object(&mut self, mut object: DynamicObject) {
        if object.width <= 0.0
            || !object.width.is_finite()
            || object.height <= 0.0
            || !object.height.is_finite()
        {
            return;
        }

        let (axis_r, axis_u) = VM::sanitize_billboard_axes(object.view_right, object.view_up);
        object.view_right = axis_r;
        object.view_up = axis_u;
        if object.kind == DynamicKind::BillboardAvatar {
            self.dynamic_avatar_objects.insert(object.id, object);
        } else {
            self.dynamic_objects.push(object);
        }
    }

    fn build_2d_batches(
        &self,
        fb_w: u32,
        fb_h: u32,
    ) -> (Vec<Vert2DPod>, Vec<u32>, Vec<TileBinPod>, Vec<u32>) {
        use vek::Vec3;

        // Estimate capacities for better performance
        let total_polys: usize = self.chunks_map.values().map(|ch| ch.polys_map.len()).sum();
        let total_lines: usize = self.chunks_map.values().map(|ch| ch.lines2d_px.len()).sum();
        let estimated_verts = total_polys * 4 + total_lines * 8; // Conservative estimate
        let estimated_indices = total_polys * 6 + total_lines * 12; // Conservative estimate

        let mut verts_flat: Vec<Vert2DPod> = Vec::with_capacity(estimated_verts);
        let mut indices_flat: Vec<u32> = Vec::with_capacity(estimated_indices);

        #[derive(Clone, Copy)]
        struct TriMeta {
            layer: i32,
            prio: i32,
            ord: u32,
        }
        let mut tri_meta: Vec<TriMeta> = Vec::with_capacity(estimated_indices / 3);
        let mut tri_ord: u32 = 0;

        for (_cid, ch) in &self.chunks_map {
            let prio = ch.priority;
            for poly in ch.polys_map.values() {
                if !poly.visible {
                    continue;
                }
                let tile_index = match self.shared_atlas.tile_index(&poly.tile_id) {
                    Some(idx) => idx,
                    None => continue,
                };

                let base = verts_flat.len() as u32;

                for (i, v) in poly.vertices.iter().enumerate() {
                    let local_p = poly.transform * Vec3::new(v[0], v[1], 1.0);
                    let world_p = self.transform2d * local_p;

                    let base_uv = poly.uvs[i];

                    verts_flat.push(Vert2DPod {
                        pos: [world_p.x, world_p.y],
                        uv: [base_uv[0], base_uv[1]],
                        tile_index,
                        _pad_tile: 0,
                    });
                }

                for &(a, b, c) in &poly.indices {
                    indices_flat.extend_from_slice(&[
                        base + a as u32,
                        base + b as u32,
                        base + c as u32,
                    ]);
                    tri_meta.push(TriMeta {
                        layer: poly.layer,
                        prio,
                        ord: tri_ord,
                    });
                    tri_ord = tri_ord.wrapping_add(1);
                }
            }
        }

        // Screen-space line strips rendered as quads
        {
            for (_cid, ch) in &self.chunks_map {
                for ls in ch.lines2d_px.values() {
                    if !ls.visible || ls.points.len() < 2 {
                        continue;
                    }
                    let tile_index = match self.shared_atlas.tile_index(&ls.tile_id) {
                        Some(idx) => idx,
                        None => continue,
                    };
                    let mut pts_scr: Vec<[f32; 2]> = Vec::with_capacity(ls.points.len());
                    for p in &ls.points {
                        let local = Vec3::new(p[0], p[1], 1.0);
                        let world = self.transform2d * local;
                        pts_scr.push([world.x, world.y]);
                    }

                    let half = 0.5 * ls.width_px.max(0.0);
                    for seg in 0..(pts_scr.len().saturating_sub(1)) {
                        let p0 = pts_scr[seg];
                        let p1 = pts_scr[seg + 1];
                        let dx = p1[0] - p0[0];
                        let dy = p1[1] - p0[1];
                        let len = (dx * dx + dy * dy).sqrt();
                        if len < 1e-6 {
                            continue;
                        }
                        let nx = -dy / len;
                        let ny = dx / len;
                        let ox = nx * half;
                        let oy = ny * half;

                        let q0 = [p0[0] - ox, p0[1] - oy];
                        let q1 = [p0[0] + ox, p0[1] + oy];
                        let q2 = [p1[0] + ox, p1[1] + oy];
                        let q3 = [p1[0] - ox, p1[1] - oy];

                        let base = verts_flat.len() as u32;
                        let v0v1v2v3 = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
                        for uv01 in v0v1v2v3 {
                            verts_flat.push(Vert2DPod {
                                pos: [0.0, 0.0],
                                uv: [uv01[0], uv01[1]],
                                tile_index,
                                _pad_tile: 0,
                            });
                        }
                        let n = verts_flat.len();
                        verts_flat[n - 4].pos = q0;
                        verts_flat[n - 3].pos = q1;
                        verts_flat[n - 2].pos = q2;
                        verts_flat[n - 1].pos = q3;

                        indices_flat.extend_from_slice(&[
                            base + 0,
                            base + 1,
                            base + 2,
                            base + 0,
                            base + 2,
                            base + 3,
                        ]);

                        tri_meta.push(TriMeta {
                            layer: ls.layer,
                            prio: 0,
                            ord: tri_ord,
                        });
                        tri_ord = tri_ord.wrapping_add(1);
                        tri_meta.push(TriMeta {
                            layer: ls.layer,
                            prio: 0,
                            ord: tri_ord,
                        });
                        tri_ord = tri_ord.wrapping_add(1);
                    }
                }
            }
        }

        let tiles_x = ((fb_w + 7) / 8).max(1);
        let tiles_y = ((fb_h + 7) / 8).max(1);
        let tiles_n = (tiles_x * tiles_y) as usize;

        #[derive(Clone, Copy)]
        struct TriRef {
            tri: u32,
            layer: i32,
            prio: i32,
            ord: u32,
        }
        let mut bins: Vec<Vec<TriRef>> = vec![Vec::new(); tiles_n];

        let tri_count = (indices_flat.len() / 3) as u32;
        // Pre-allocate bins with estimated capacity
        let estimated_tris_per_tile = (tri_count as usize) / tiles_n.max(1) + 1;
        for bin in &mut bins {
            bin.reserve(estimated_tris_per_tile);
        }
        for t in 0..tri_count {
            let i0 = indices_flat[(3 * t as usize) + 0] as usize;
            let i1 = indices_flat[(3 * t as usize) + 1] as usize;
            let i2 = indices_flat[(3 * t as usize) + 2] as usize;
            let a = verts_flat[i0].pos;
            let b = verts_flat[i1].pos;
            let c = verts_flat[i2].pos;

            let minx = f32::min(a[0], f32::min(b[0], c[0])).floor().max(0.0) as i32;
            let maxx = f32::max(a[0], f32::max(b[0], c[0])).ceil().min(fb_w as f32) as i32;
            let miny = f32::min(a[1], f32::min(b[1], c[1])).floor().max(0.0) as i32;
            let maxy = f32::max(a[1], f32::max(b[1], c[1])).ceil().min(fb_h as f32) as i32;
            if minx >= maxx || miny >= maxy {
                continue;
            }

            let tx0 = (minx.max(0) as u32) / 8;
            let ty0 = (miny.max(0) as u32) / 8;
            let tx1 = ((maxx.max(0) as u32).saturating_sub(1)) / 8;
            let ty1 = ((maxy.max(0) as u32).saturating_sub(1)) / 8;

            for ty in ty0..=ty1 {
                for tx in tx0..=tx1 {
                    let idx = (ty * tiles_x + tx) as usize;
                    let meta = tri_meta[t as usize];
                    bins[idx].push(TriRef {
                        tri: t as u32,
                        layer: meta.layer,
                        prio: meta.prio,
                        ord: meta.ord,
                    });
                }
            }
        }

        let mut tile_offsets: Vec<u32> = Vec::with_capacity(tiles_n);
        let mut tile_counts: Vec<u32> = Vec::with_capacity(tiles_n);
        let mut tile_tris: Vec<u32> = Vec::with_capacity(tri_count as usize);
        let mut running: u32 = 0;
        for v in &mut bins {
            tile_offsets.push(running);
            if !v.is_empty() {
                v.sort_by(|a, b| {
                    b.layer
                        .cmp(&a.layer)
                        .then_with(|| b.prio.cmp(&a.prio))
                        .then_with(|| b.ord.cmp(&a.ord))
                });
                tile_tris.extend(v.iter().map(|r| r.tri));
            }
            let c = v.len() as u32;
            tile_counts.push(c);
            running += c;
        }

        if tile_offsets.is_empty() {
            // AMD fix: Ensure minimum 16-byte buffer size
            tile_offsets.extend_from_slice(&[0u32; 4]);
        }
        if tile_counts.is_empty() {
            // AMD fix: Ensure minimum 16-byte buffer size
            tile_counts.extend_from_slice(&[0u32; 4]);
        }
        if tile_tris.is_empty() {
            // AMD fix: Ensure minimum 16-byte buffer size
            tile_tris.extend_from_slice(&[0u32; 4]);
        }

        let mut tile_bins: Vec<TileBinPod> = Vec::with_capacity(tile_offsets.len());
        for (offset, count) in tile_offsets.iter().zip(tile_counts.iter()) {
            tile_bins.push(TileBinPod {
                offset: *offset,
                count: *count,
            });
        }

        if tile_bins.is_empty() {
            tile_bins.push(TileBinPod {
                offset: 0,
                count: 0,
            });
        }

        (verts_flat, indices_flat, tile_bins, tile_tris)
    }

    fn build_scene_data_blob(&self) -> Vec<u8> {
        let mut lights_flat: Vec<LightPod> = Vec::with_capacity(self.lights.len());
        for (_id, l) in &self.lights {
            let flicker: f32 = if l.flicker > 0.0 {
                let hash = hash_u32(self.animation_counter as u32);
                let combined_hash = hash.wrapping_add(
                    (l.position.x as u32 + l.position.y as u32 + l.position.z as u32) * 100,
                );
                let flicker_value = (combined_hash as f32 / u32::MAX as f32).clamp(0.0, 1.0);
                1.0 - flicker_value * l.flicker
            } else {
                1.0
            };

            lights_flat.push(LightPod {
                header: [
                    match l.light_type {
                        LightType::Point => 0,
                    },
                    if l.emitting { 1 } else { 0 },
                    0,
                    0,
                ],
                position: [l.position.x, l.position.y, l.position.z, 0.0],
                color: [l.color.x, l.color.y, l.color.z, 0.0],
                params0: [l.intensity, l.radius, l.start_distance, l.end_distance],
                params1: [flicker, 0.0, 0.0, 0.0],
            });
        }

        let mut data_words: Vec<u32> = Vec::new();
        let lights_offset_words = data_words.len() as u32;
        if !lights_flat.is_empty() {
            data_words.extend_from_slice(bytemuck::cast_slice(&lights_flat));
        }

        let mut billboard_cmds: Vec<DynamicBillboardPod> = Vec::new();
        let mut avatar_metas: Vec<DynamicAvatarMetaPod> = Vec::new();
        let mut avatar_pixels_rgba8: Vec<u32> = Vec::new();
        let mut avatar_indices: FxHashMap<GeoId, u32> = FxHashMap::default();
        for obj in self
            .dynamic_objects
            .iter()
            .chain(self.dynamic_avatar_objects.values())
        {
            match obj.kind {
                DynamicKind::BillboardTile => {
                    let tile_id = match obj.tile_id {
                        Some(id) => id,
                        None => continue,
                    };
                    let tile_index = match self.shared_atlas.tile_index(&tile_id) {
                        Some(idx) => idx,
                        None => continue,
                    };
                    let half_width = (obj.width * 0.5).max(0.0);
                    let half_height = (obj.height * 0.5).max(0.0);
                    if !half_width.is_finite()
                        || half_width <= 0.0
                        || !half_height.is_finite()
                        || half_height <= 0.0
                    {
                        continue;
                    }
                    let axis_right = obj.view_right * half_width;
                    let axis_up = obj.view_up * half_height;
                    billboard_cmds.push(DynamicBillboardPod {
                        center: [obj.center.x, obj.center.y, obj.center.z, obj.width],
                        axis_right: [axis_right.x, axis_right.y, axis_right.z, obj.height],
                        axis_up: [
                            axis_up.x,
                            axis_up.y,
                            axis_up.z,
                            obj.repeat_mode as u32 as f32,
                        ],
                        params: [tile_index, obj.kind as u32, obj.opacity.to_bits(), 0],
                    });
                }
                DynamicKind::BillboardAvatar => {
                    let half_width = (obj.width * 0.5).max(0.0);
                    let half_height = (obj.height * 0.5).max(0.0);
                    if !half_width.is_finite()
                        || half_width <= 0.0
                        || !half_height.is_finite()
                        || half_height <= 0.0
                    {
                        continue;
                    }

                    let avatar_index = if let Some(existing) = avatar_indices.get(&obj.id).copied()
                    {
                        existing
                    } else {
                        let Some(avatar) = self.dynamic_avatar_data.get(&obj.id) else {
                            continue;
                        };
                        if avatar.size == 0 {
                            continue;
                        }
                        let expected_len = avatar.size as usize * avatar.size as usize * 4;
                        if avatar.rgba.len() != expected_len {
                            continue;
                        }
                        let offset_pixels = avatar_pixels_rgba8.len() as u32;
                        for px in avatar.rgba.chunks_exact(4) {
                            avatar_pixels_rgba8
                                .push(u32::from_le_bytes([px[0], px[1], px[2], px[3]]));
                        }
                        let index = avatar_metas.len() as u32;
                        avatar_metas.push(DynamicAvatarMetaPod {
                            offset_pixels,
                            size: avatar.size,
                            _pad: [0, 0],
                        });
                        avatar_indices.insert(obj.id, index);
                        index
                    };

                    let axis_right = obj.view_right * half_width;
                    let axis_up = obj.view_up * half_height;
                    billboard_cmds.push(DynamicBillboardPod {
                        center: [obj.center.x, obj.center.y, obj.center.z, obj.width],
                        axis_right: [axis_right.x, axis_right.y, axis_right.z, obj.height],
                        axis_up: [axis_up.x, axis_up.y, axis_up.z, 0.0],
                        params: [avatar_index, obj.kind as u32, obj.opacity.to_bits(), 0],
                    });
                }
            }
        }

        let billboard_cmd_offset_words = if billboard_cmds.is_empty() {
            0
        } else {
            data_words.len() as u32
        };
        if !billboard_cmds.is_empty() {
            data_words.extend_from_slice(bytemuck::cast_slice(&billboard_cmds));
            if self.activity_logging {
                self.log_layer(format!(
                    "Uploaded {} dynamic billboards",
                    billboard_cmds.len()
                ));
            }
        }

        let avatar_meta_offset_words = if avatar_metas.is_empty() {
            0
        } else {
            data_words.len() as u32
        };
        if !avatar_metas.is_empty() {
            data_words.extend_from_slice(bytemuck::cast_slice(&avatar_metas));
        }
        let avatar_pixel_offset_words = if avatar_pixels_rgba8.is_empty() {
            0
        } else {
            data_words.len() as u32
        };
        if !avatar_pixels_rgba8.is_empty() {
            data_words.extend_from_slice(&avatar_pixels_rgba8);
        }

        let logical_word_count = data_words.len() as u32;
        if data_words.is_empty() {
            // AMD fix: Ensure minimum 16-byte buffer size (wgpu validation + AMD compatibility)
            data_words.extend_from_slice(&[0u32; 4]);
        }

        let header = SceneDataHeaderPod {
            lights_offset_words,
            lights_count: lights_flat.len() as u32,
            billboard_cmd_offset_words,
            billboard_cmd_count: billboard_cmds.len() as u32,
            avatar_meta_offset_words,
            avatar_meta_count: avatar_metas.len() as u32,
            avatar_pixel_offset_words,
            data_word_count: logical_word_count,
        };

        let header_bytes = bytemuck::bytes_of(&header);
        let data_bytes: &[u8] = bytemuck::cast_slice(&data_words);
        let mut blob = Vec::with_capacity(header_bytes.len() + data_bytes.len());
        blob.extend_from_slice(header_bytes);
        blob.extend_from_slice(data_bytes);
        blob
    }

    #[inline]
    pub fn mark_all_geometry_dirty(&mut self) {
        self.geometry2d_dirty = true;
        self.accel_dirty = true;
    }

    fn vm_flags(&self) -> u32 {
        // No flags needed - layer clearing handled by render pass
        0
    }

    fn atlas_dims(&self) -> (u32, u32) {
        self.shared_atlas.dims()
    }

    fn frame_rect_owned(&self, id: &Uuid, anim_frame: u32) -> Option<AtlasEntry> {
        self.shared_atlas.frame_rect(id, anim_frame)
    }

    fn ensure_tile_metadata(&mut self) {
        let layout_version = self.shared_atlas.layout_version();
        if layout_version != self.cached_atlas_layout_version {
            self.rebuild_tile_metadata(layout_version);
        }
    }

    fn rebuild_tile_metadata(&mut self, new_version: u64) {
        let (atlas_w_px, atlas_h_px) = self.atlas_dims();
        let atlas_w = atlas_w_px.max(1) as f32;
        let atlas_h = atlas_h_px.max(1) as f32;
        let tables: AtlasGpuTables = self.shared_atlas.gpu_tile_tables();

        self.cached_tile_anim_meta.clear();
        if tables.metas.is_empty() {
            self.cached_tile_anim_meta.push(TileAnimMetaPod {
                first_frame: 0,
                frame_count: 0,
                _pad: [0, 0],
            });
        } else {
            for meta in tables.metas {
                self.cached_tile_anim_meta.push(TileAnimMetaPod {
                    first_frame: meta.first_frame,
                    frame_count: meta.frame_count,
                    _pad: [0, 0],
                });
            }
        }

        self.cached_tile_frame_data.clear();
        if tables.frames.is_empty() {
            self.cached_tile_frame_data.push(TileFramePod {
                ofs: [0.0, 0.0],
                scale: [0.0, 0.0],
            });
        } else {
            for rect in tables.frames {
                self.cached_tile_frame_data.push(TileFramePod {
                    ofs: [rect.x as f32 / atlas_w, rect.y as f32 / atlas_h],
                    scale: [rect.w as f32 / atlas_w, rect.h as f32 / atlas_h],
                });
            }
        }

        self.cached_atlas_layout_version = new_version;
        self.tile_gpu_dirty = true;
        self.log_layer(format!(
            "Updated tile metadata (tiles: {}, frames: {})",
            self.cached_tile_anim_meta.len(),
            self.cached_tile_frame_data.len()
        ));
    }

    fn upload_tile_metadata_to_gpu(&mut self, device: &wgpu::Device) {
        if self.gpu.is_none() {
            return;
        }
        self.ensure_tile_metadata();
        use wgpu::util::DeviceExt;
        let g = self.gpu.as_mut().unwrap();

        let meta_slice: &[TileAnimMetaPod] = if self.cached_tile_anim_meta.is_empty() {
            std::slice::from_ref(&DUMMY_TILE_META)
        } else {
            &self.cached_tile_anim_meta
        };
        if self.tile_gpu_dirty || g.tile_meta_ssbo.is_none() {
            g.tile_meta_ssbo = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("vm-tile-meta-ssbo"),
                    contents: bytemuck::cast_slice(meta_slice),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        let frame_slice: &[TileFramePod] = if self.cached_tile_frame_data.is_empty() {
            std::slice::from_ref(&DUMMY_TILE_FRAME)
        } else {
            &self.cached_tile_frame_data
        };
        if self.tile_gpu_dirty || g.tile_frames_ssbo.is_none() {
            g.tile_frames_ssbo = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("vm-tile-frame-ssbo"),
                    contents: bytemuck::cast_slice(frame_slice),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                },
            ));
        }

        if self.tile_gpu_dirty {
            self.log_layer("Uploaded tile metadata buffers");
        }
        self.tile_gpu_dirty = false;
    }

    fn upload_sdf_data_to_gpu(&mut self, device: &wgpu::Device) {
        if self.gpu.is_none() {
            return;
        }
        use wgpu::util::DeviceExt;
        let g = self.gpu.as_mut().unwrap();
        let data_slice: &[[f32; 4]] = if self.sdf_data.is_empty() {
            &[[0.0; 4]]
        } else {
            &self.sdf_data
        };

        if self.sdf_data_dirty || g.sdf_data_ssbo.is_none() {
            g.sdf_data_ssbo = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("vm-sdf-data-ssbo"),
                    contents: bytemuck::cast_slice(data_slice),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                }),
            );
            self.sdf_data_dirty = false;
        }
    }

    /// Create a VM with a fixed-size atlas (atlas_w x atlas_h).
    pub fn new(atlas_w: u32, atlas_h: u32) -> Self {
        Self::new_with_shared_atlas(SharedAtlas::new(atlas_w, atlas_h))
    }

    pub fn new_with_shared_atlas(shared_atlas: SharedAtlas) -> Self {
        let mut source2d = String::new();
        if let Some(bytes) = crate::Embedded::get("2d_body.wgsl") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                source2d = source.to_string();
            }
        }

        let mut source3d = String::new();
        if let Some(bytes) = crate::Embedded::get("3d_body.wgsl") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                source3d = source.to_string();
            }
        }
        let mut source_sdf = String::new();
        if let Some(bytes) = crate::Embedded::get("sdf_body.wgsl") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                source_sdf = source.to_string();
            }
        }
        Self {
            shared_atlas,
            chunks_map: FxHashMap::default(),
            current_chunk: None,
            animation_counter: 0,
            render_mode: RenderMode::Compute2D,
            blend_mode: LayerBlendMode::Alpha,
            gpu: None,
            layer_texture: None,
            ping_pong_textures: None,
            ping_pong_front: 0,
            ping_pong_enabled: false,
            prev_dummy: None,
            background: Vec4::new(1.0, 0.8, 0.2, 1.0),
            palette: [[0.0; 4]; 256],
            palette_dirty: true,
            gp0: Vec4::new(0.0, 0.0, 0.0, 0.0),
            gp1: Vec4::new(0.0, 0.0, 0.0, 0.0),
            gp2: Vec4::new(0.0, 0.0, 0.0, 0.0),
            gp3: Vec4::new(0.0, 0.0, 0.0, 0.0),
            gp4: Vec4::new(0.0, 0.0, 0.0, 0.0),
            gp5: Vec4::new(0.0, 0.0, 0.0, 0.0),
            gp6: Vec4::new(0.0, 0.0, 0.0, 0.0),
            gp7: Vec4::new(0.0, 0.0, 0.0, 0.0),
            gp8: Vec4::new(0.0, 0.0, 0.0, 0.0),
            gp9: Vec4::new(0.0, 0.0, 0.0, 0.0),
            source2d,
            viewport_rect2d: None,
            source3d,
            source_sdf,
            sdf_data: Vec::new(),
            sdf_data_dirty: true,
            transform2d: Mat3::identity(),
            transform3d: Mat4::identity(),
            lights: FxHashMap::default(),
            dynamic_objects: Vec::new(),
            dynamic_avatar_objects: FxHashMap::default(),
            dynamic_avatar_data: FxHashMap::default(),
            current_layer: 0,
            scene_accel: SceneAccel::default(),
            accel_dirty: true,
            bvh_leaf_size: 8,
            cached_v3: Vec::new(),
            cached_i3: Vec::new(),
            cached_tri_visibility: Vec::new(),
            visibility_dirty: false,
            geometry2d_dirty: true,
            cached_v2: Vec::new(),
            cached_i2: Vec::new(),
            cached_tile_bins: Vec::new(),
            cached_tile_tris: Vec::new(),
            cached_fb_size_2d: (0, 0),
            cached_tile_anim_meta: Vec::new(),
            cached_tile_frame_data: Vec::new(),
            cached_atlas_layout_version: 0,
            tile_gpu_dirty: true,
            camera3d: Camera3D::default(),
            enabled: true,
            layer_index: 0,
            activity_logging: false,
        }
    }

    /// Interpret one instruction.
    pub fn execute(&mut self, atom: Atom) {
        match atom {
            Atom::SetGeoVisible { id, visible } => {
                let mut dirty_2d = false;
                let mut dirty_3d = false;
                for ch in self.chunks_map.values_mut() {
                    if let Some(p) = ch.polys_map.get_mut(&id) {
                        p.visible = visible;
                        dirty_2d = true;
                    }
                    if let Some(p3_vec) = ch.polys3d_map.get_mut(&id) {
                        for p3 in p3_vec.iter_mut() {
                            p3.visible = visible;
                        }
                        dirty_3d = true;
                    }
                }
                if dirty_2d {
                    self.mark_2d_dirty();
                }
                if dirty_3d {
                    // Only mark visibility dirty, NOT accel_dirty
                    // This avoids rebuilding the BVH structure
                    self.visibility_dirty = true;
                }
            }
            Atom::AddTile {
                id,
                width,
                height,
                frames,
                material_frames,
            } => {
                // Basic validation: ensure each frame has enough bytes; pad/trim as needed
                let need = (width as usize) * (height as usize) * 4;
                let frames: Vec<Vec<u8>> = frames
                    .into_iter()
                    .map(|mut f| {
                        if f.len() < need {
                            f.resize(need, 0);
                        }
                        if f.len() > need {
                            f.truncate(need);
                        }
                        f
                    })
                    .collect();
                let mut mat_frames = material_frames.unwrap_or_default();
                if mat_frames.is_empty() {
                    mat_frames = (0..frames.len())
                        .map(|_| default_material_frame(need))
                        .collect();
                }
                if mat_frames.len() < frames.len() {
                    let missing = frames.len() - mat_frames.len();
                    mat_frames.extend((0..missing).map(|_| default_material_frame(need)));
                }
                if mat_frames.len() > frames.len() {
                    mat_frames.truncate(frames.len());
                }
                for mf in mat_frames.iter_mut() {
                    if mf.len() < need {
                        mf.resize(need, 0);
                    }
                    if mf.len() > need {
                        mf.truncate(need);
                    }
                }
                if mat_frames.is_empty() {
                    mat_frames.push(default_material_frame(need));
                }
                self.shared_atlas
                    .add_tile(id, width, height, frames, mat_frames);
                self.mark_all_geometry_dirty();
            }
            Atom::AddSolid { id, color } => {
                // Create a 1x1 tile with a single frame of the given color
                let frame = color.to_vec();
                let mat_frame = default_material_frame(4);
                self.shared_atlas
                    .add_tile(id, 1, 1, vec![frame], vec![mat_frame]);
                self.mark_all_geometry_dirty();
            }
            Atom::AddSolidWithMaterial {
                id,
                color,
                material,
            } => {
                // Create a 1x1 tile with a single frame of the given color and material properties
                let frame = color.to_vec();
                let mat_frame = material.to_vec();
                self.shared_atlas
                    .add_tile(id, 1, 1, vec![frame], vec![mat_frame]);
                self.mark_all_geometry_dirty();
            }
            Atom::SetTileMaterialFrames { id, frames } => {
                self.shared_atlas.with_tile_mut(&id, move |tile| {
                    let need = (tile.w as usize) * (tile.h as usize) * 4;
                    let mut mats: Vec<Vec<u8>> = frames
                        .into_iter()
                        .map(|mut f| {
                            if f.len() < need {
                                f.resize(need, 0);
                            }
                            if f.len() > need {
                                f.truncate(need);
                            }
                            f
                        })
                        .collect();
                    if mats.len() < tile.frames.len() {
                        let missing = tile.frames.len() - mats.len();
                        mats.extend((0..missing).map(|_| default_material_frame(need)));
                    }
                    if mats.len() > tile.frames.len() {
                        mats.truncate(tile.frames.len());
                    }
                    tile.material_frames = mats;
                });
                self.mark_all_geometry_dirty();
            }
            Atom::BuildAtlas => {
                self.build_atlas();
                self.mark_all_geometry_dirty();
            }
            Atom::SetPalette(p) => {
                let mut out = [[0.0f32; 4]; 256];
                for (dst, src) in out.iter_mut().zip(p.iter().take(256)) {
                    *dst = src.into_array();
                }
                self.palette = out;
                self.palette_dirty = true;
            }
            Atom::SetAtlasSize { width, height } => {
                let w = width.max(1);
                let h = height.max(1);
                self.shared_atlas.resize(w, h);
                self.cached_atlas_layout_version = 0; // force metadata rebuild on next upload
                self.mark_all_geometry_dirty();
            }
            Atom::AddPoly { poly } => {
                let chunk_id = match self.current_chunk {
                    Some(cid) => cid,
                    None => {
                        let cid = uuid::Uuid::new_v4();
                        self.chunks_map.insert(cid, Chunk::default());
                        self.current_chunk = Some(cid);
                        cid
                    }
                };

                self.chunks_map.entry(chunk_id).or_default().add(poly);
                self.mark_2d_dirty();
            }
            Atom::AddPoly3D { poly } => {
                let chunk_id = match self.current_chunk {
                    Some(cid) => cid,
                    None => {
                        let cid = Uuid::new_v4();
                        self.chunks_map.insert(cid, Chunk::default());
                        self.current_chunk = Some(cid);
                        cid
                    }
                };

                self.chunks_map.entry(chunk_id).or_default().add_3d(poly);
                self.accel_dirty = true;
            }
            Atom::AddLineStrip2D {
                id,
                tile_id,
                points,
                width,
            } => {
                if points.len() < 2 {
                    return;
                }
                let chunk_id = match self.current_chunk {
                    Some(cid) => cid,
                    None => {
                        let cid = Uuid::new_v4();
                        self.chunks_map.insert(cid, Chunk::default());
                        self.current_chunk = Some(cid);
                        cid
                    }
                };
                self.chunks_map
                    .entry(chunk_id)
                    .or_default()
                    .add_line_strip_2d(id, tile_id, points, width, self.current_layer);
                self.accel_dirty = true;
                self.mark_2d_dirty();
            }
            Atom::AddLineStrip2Dpx {
                id,
                tile_id,
                points,
                width_px,
            } => {
                if points.len() < 2 || width_px <= 0.0 {
                    return;
                }
                let chunk_id = match self.current_chunk {
                    Some(cid) => cid,
                    None => {
                        let cid = Uuid::new_v4();
                        self.chunks_map.insert(cid, Chunk::default());
                        self.current_chunk = Some(cid);
                        cid
                    }
                };
                self.chunks_map
                    .entry(chunk_id)
                    .or_default()
                    .add_line_strip_2d_px(id, tile_id, points, width_px, self.current_layer);
                self.mark_2d_dirty();
            }
            Atom::NewChunk { id } => {
                self.chunks_map.entry(id).or_insert_with(Chunk::default);
                self.accel_dirty = true;
                self.mark_2d_dirty();
            }
            Atom::AddChunk { id, chunk } => {
                // Insert or replace the chunk as-is; caller controls current_chunk separately
                self.chunks_map.insert(id, chunk);
                self.accel_dirty = true;
                self.mark_2d_dirty();
            }
            Atom::RemoveChunk { id } => {
                let was_current = self.current_chunk == Some(id);
                self.chunks_map.remove(&id);
                if was_current {
                    self.current_chunk = None;
                }
                self.accel_dirty = true;
                self.mark_2d_dirty();
            }
            Atom::RemoveChunkAt { origin } => {
                if let Some((id, _)) = self.chunks_map.iter().find(|(_, ch)| ch.origin == origin) {
                    let id = *id;
                    let was_current = self.current_chunk == Some(id);
                    self.chunks_map.remove(&id);
                    if was_current {
                        self.current_chunk = None;
                    }
                }
                self.accel_dirty = true;
                self.mark_2d_dirty();
            }
            Atom::SetCurrentChunk { id } => {
                if !self.chunks_map.contains_key(&id) {
                    self.chunks_map.insert(id, Chunk::default());
                }
                self.current_chunk = Some(id);
            }
            Atom::SetAnimationCounter(n) => {
                self.animation_counter = n;
            }
            Atom::SetSource2D(src) => {
                self.source2d = src;
                if let Some(g) = self.gpu.as_mut() {
                    g.compute2d_pipeline = None;
                }
            }
            Atom::SetViewportRect2D(rect) => {
                self.viewport_rect2d = rect;
            }
            Atom::SetSource3D(src) => {
                self.source3d = src;
                if let Some(g) = self.gpu.as_mut() {
                    g.compute3d_pipeline = None;
                }
            }
            Atom::SetSourceSdf(src) => {
                self.source_sdf = src;
                if let Some(g) = self.gpu.as_mut() {
                    g.compute_sdf_pipeline = None;
                }
            }
            Atom::SetSdfData(data) => {
                self.sdf_data = data;
                self.sdf_data_dirty = true;
            }
            Atom::SetTransform2D(m) => {
                if self.transform2d != m {
                    self.transform2d = m;
                    self.mark_2d_dirty();
                }
            }
            Atom::SetTransform3D(m) => {
                self.transform3d = m;
                self.accel_dirty = true;
            }
            Atom::SetLayer(l) => {
                self.current_layer = l;
            }
            Atom::Clear => {
                self.shared_atlas.clear();
                self.chunks_map.clear();
                self.current_chunk = None;
                self.animation_counter = 0;
                self.background = Vec4::new(1.0, 0.8, 0.2, 1.0);
                self.gp0 = Vec4::new(0.0, 0.0, 0.0, 0.0);
                self.gp1 = Vec4::new(0.0, 0.0, 0.0, 0.0);
                self.gp2 = Vec4::new(0.0, 0.0, 0.0, 0.0);
                self.render_mode = RenderMode::Compute2D;
                self.sdf_data.clear();
                self.sdf_data_dirty = true;
                self.mark_all_geometry_dirty();
                self.dynamic_objects.clear();
                self.dynamic_avatar_objects.clear();
                self.dynamic_avatar_data.clear();
            }
            Atom::ClearTiles => {
                // Clear tile-related state and atlas pixels; keep scene/chunks
                self.shared_atlas.clear();
                self.mark_all_geometry_dirty();
                self.dynamic_objects.clear();
                self.dynamic_avatar_objects.clear();
            }
            Atom::ClearGeometry => {
                // Remove all chunks and unset current chunk; keep tiles/atlas/state
                self.chunks_map.clear();
                self.current_chunk = None;
                self.accel_dirty = true;
                self.mark_2d_dirty();
                self.dynamic_objects.clear();
                self.dynamic_avatar_objects.clear();
            }
            Atom::SetBackground(v) => {
                self.background = v;
            }
            Atom::SetGP0(v) => {
                self.gp0 = v;
            }
            Atom::SetGP1(v) => {
                self.gp1 = v;
            }
            Atom::SetGP2(v) => {
                self.gp2 = v;
            }
            Atom::SetGP3(v) => {
                self.gp3 = v;
            }
            Atom::SetGP4(v) => {
                self.gp4 = v;
            }
            Atom::SetGP5(v) => {
                self.gp5 = v;
            }
            Atom::SetGP6(v) => {
                self.gp6 = v;
            }
            Atom::SetGP7(v) => {
                self.gp7 = v;
            }
            Atom::SetGP8(v) => {
                self.gp8 = v;
            }
            Atom::SetGP9(v) => {
                self.gp9 = v;
            }
            Atom::SetRenderMode(m) => {
                self.render_mode = m;
            }
            Atom::AddLight { id, light } => {
                self.lights.insert(id, light);
            }
            Atom::RemoveLight { id } => {
                self.lights.remove(&id);
            }
            Atom::ClearLights => {
                self.lights.clear();
            }
            Atom::ClearDynamics => {
                self.dynamic_objects.clear();
                self.dynamic_avatar_objects.clear();
            }
            Atom::AddDynamic { object } => {
                self.push_dynamic_object(object);
            }
            Atom::SetAvatarBillboardData { id, size, rgba } => {
                let expected_len = size as usize * size as usize * 4;
                if size == 0 || rgba.len() != expected_len {
                    return;
                }
                self.dynamic_avatar_data
                    .insert(id, DynamicAvatarData { size, rgba });
            }
            Atom::RemoveAvatarBillboardData { id } => {
                self.dynamic_avatar_data.remove(&id);
            }
            Atom::ClearAvatarBillboardData => {
                self.dynamic_avatar_data.clear();
            }
            Atom::SetCamera3D { camera } => {
                self.camera3d = camera;
            }
            Atom::SetBvhLeafSize { max_tris } => {
                self.bvh_leaf_size = max_tris.max(1);
                self.accel_dirty = true;
            }
        }
    }

    pub fn init_gpu(&mut self, device: &wgpu::Device) -> crate::SceneVMResult<()> {
        use wgpu::ShaderSource;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scenevm-2d-shader"),
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SCENEVM_2D_WGSL)),
        });

        let globals_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vm-globals-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Globals>() as _),
                },
                count: None,
            }],
        });

        let atlas_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vm-atlas-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vm-2d-pipeline-layout"),
            bind_group_layouts: &[&globals_bgl, &atlas_bgl],
            push_constant_ranges: &[],
        });

        let vbuf_layout = wgpu::VertexBufferLayout {
            array_stride: (4 * std::mem::size_of::<f32>()) as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
        };

        let pipeline_2d = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vm-2d-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vbuf_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Base sampler (nearest) plus optional linear variant for UI text smoothing.
        let sampler: wgpu::Sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vm-atlas-sampler-repeat-nearest"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let sampler_linear: wgpu::Sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vm-atlas-sampler-repeat-linear"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vm-globals-buffer"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.gpu = Some(VMGpu {
            pipeline_2d,
            globals_buf,
            globals_bgl,
            atlas_bgl,
            globals_bg: None,
            atlas_bg: None,
            vbuf: None,
            ibuf: None,
            index_count: 0,
            sampler,
            sampler_linear,
            compute2d_pipeline: None,
            compute3d_pipeline: None,
            compute_sdf_pipeline: None,
            u2d_buf: None,
            u3d_buf: None,
            u_sdf_buf: None,
            u2d_bgl: None,
            u3d_bgl: None,
            u_sdf_bgl: None,
            u2d_bg: None,
            u3d_bg: None,
            u_sdf_bg: None,
            v2d_ssbo: None,
            i2d_ssbo: None,
            v3d_ssbo: None,
            i3d_ssbo: None,
            tile_bins: None,
            tile_tris: None,
            tile_meta_ssbo: None,
            tile_frames_ssbo: None,
            scene_data_ssbo: None,
            grid_hdr: None,
            grid_data: None,
            sdf_data_ssbo: None,
        });

        Ok(())
    }

    /// Returns a copy of the current color atlas pixels (RGBA8).
    pub fn atlas_pixels(&self) -> Vec<u8> {
        self.shared_atlas.atlas_pixels()
    }

    /// Returns a copy of the material atlas pixels (RGBA8 storing R/M/O/E).
    pub fn material_atlas_pixels(&self) -> Vec<u8> {
        self.shared_atlas.material_atlas_pixels()
    }

    /// Copies the atlas into a destination pixel slice of size (dst_w x dst_h) RGBA8.
    pub fn copy_atlas_to_slice(&self, dst: &mut [u8], dst_w: u32, dst_h: u32) {
        self.shared_atlas.copy_atlas_to_slice(dst, dst_w, dst_h);
    }

    /// Copies the material atlas into a destination pixel slice (RGBA8 R/M/O/E).
    pub fn copy_material_atlas_to_slice(&self, dst: &mut [u8], dst_w: u32, dst_h: u32) {
        self.shared_atlas
            .copy_material_atlas_to_slice(dst, dst_w, dst_h);
    }

    /// Upload the CPU atlas to GPU (creates GPU resources if needed).
    pub fn upload_atlas_to_gpu_with(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.build_atlas();
        self.shared_atlas.upload_to_gpu_with(device, queue);
    }

    /// Download the atlas from GPU into CPU memory; blocks on native, schedules on wasm.
    pub fn download_atlas_from_gpu_with(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.shared_atlas.download_from_gpu_with(device, queue);
    }

    /// Get the atlas rect for a tile's animation frame. Returns None if the tile wasn't packed.
    pub fn frame_rect(&self, id: &Uuid, anim_frame: u32) -> Option<AtlasEntry> {
        self.frame_rect_owned(id, anim_frame)
    }

    /// Ensure the shared atlas is packed.
    fn build_atlas(&self) {
        if self.shared_atlas.ensure_built() {
            self.log_layer("Built shared atlas layout");
        }
    }

    /// Iterate polygons ready for drawing: always yields all polygons in all chunks (ignores current_chunk).
    pub fn polys_2d(&self) -> impl Iterator<Item = (&Poly2D, Option<AtlasEntry>)> {
        let anim = self.animation_counter as u32;
        self.chunks_map
            .values()
            .flat_map(|ch| ch.polys_map.values())
            .map(move |p| {
                let rect = self.frame_rect(&p.tile_id, anim);
                (p, rect)
            })
    }
}

impl VM {
    /// Initialize compute pipelines and uniform buffers if not yet present.
    pub fn init_compute(&mut self, device: &wgpu::Device) -> crate::SceneVMResult<()> {
        if self.gpu.is_none() {
            // If render pipeline not initialized yet, do it now to allocate gpu struct

            self.init_gpu(device)?;
        }
        let g = self.gpu.as_mut().unwrap();

        // Uniform BGLs
        let u2d_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vm-u2d-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    // UBO
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // storage image (color)
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // atlas texture (sampled)
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // atlas sampler
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // verts SSBO
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // indices SSBO
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // tile bins (offset/count)
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // tile tris
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // material atlas texture
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // tile animation metadata
                    binding: 10,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // tile frame rects
                    binding: 11,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // scene data (lights, billboards, ...)
                    binding: 9,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // atlas sampler (linear) for text smoothing
                    binding: 12,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // previous layer texture (sampled) for ping-pong accumulation
                    binding: 13,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let u3d_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vm-u3d-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    // UBO
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // storage image (color)
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // atlas texture (sampled)
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // atlas sampler
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // material atlas texture (sampled)
                    binding: 11,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // scene data (lights, billboards, ...)
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 5: verts3d
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 6: indices3d
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 7: grid header (uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 8: combined grid data (storage read)
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 12,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 13,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // previous layer texture (sampled) for ping-pong accumulation
                    binding: 14,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let sdf_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vm-sdf-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    // UBO
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // storage image (color)
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // SDF data buffer (array<vec4<f32>>)
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // atlas texture (sampled)
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // atlas sampler
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // previous layer texture (sampled) for ping-pong accumulation
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        // Pipelines (compile only if missing)
        if g.u2d_bgl.is_none() {
            g.u2d_bgl = Some(u2d_bgl);
        }
        if g.u3d_bgl.is_none() {
            g.u3d_bgl = Some(u3d_bgl);
        }
        if g.u_sdf_bgl.is_none() {
            g.u_sdf_bgl = Some(sdf_bgl);
        }

        if g.compute2d_pipeline.is_none() {
            let mut header_2d = String::new();
            if let Some(bytes) = crate::Embedded::get("2d_header.wgsl") {
                if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                    header_2d = source.to_string();
                }
            }
            let src2d = [header_2d.as_str(), &self.source2d].concat();
            let cs2d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vm-2d-cs"),
                source: wgpu::ShaderSource::Wgsl(src2d.into()),
            });
            let pl2d = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("vm-2d-cs-pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("vm-2d-cs-layout"),
                        bind_group_layouts: &[g.u2d_bgl.as_ref().unwrap()],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &cs2d,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            });
            g.compute2d_pipeline = Some(pl2d);
        }

        if g.compute3d_pipeline.is_none() {
            let mut header_3d = String::new();
            if let Some(bytes) = crate::Embedded::get("3d_header.wgsl") {
                if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                    header_3d = source.to_string();
                }
            }

            let src3d = [header_3d.as_str(), &self.source3d].concat();
            let cs3d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vm-3d-cs"),
                source: wgpu::ShaderSource::Wgsl(src3d.into()),
            });
            let pl3d = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("vm-3d-cs-pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("vm-3d-cs-layout"),
                        bind_group_layouts: &[g.u3d_bgl.as_ref().unwrap()],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &cs3d,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            });
            g.compute3d_pipeline = Some(pl3d);
        }

        if g.compute_sdf_pipeline.is_none() {
            let mut header_sdf = String::new();
            if let Some(bytes) = crate::Embedded::get("sdf_header.wgsl") {
                if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                    header_sdf = source.to_string();
                }
            }

            let src_sdf = [header_sdf.as_str(), &self.source_sdf].concat();
            let cs_sdf = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vm-sdf-cs"),
                source: wgpu::ShaderSource::Wgsl(src_sdf.into()),
            });
            let pl_sdf = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("vm-sdf-cs-pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("vm-sdf-cs-layout"),
                        bind_group_layouts: &[g.u_sdf_bgl.as_ref().unwrap()],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &cs_sdf,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            });
            g.compute_sdf_pipeline = Some(pl_sdf);
        }

        // UBOs
        if g.u2d_buf.is_none() {
            let u2d_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vm-u2d"),
                size: std::mem::size_of::<Compute2DUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            g.u2d_buf = Some(u2d_buf);
        }
        if g.u3d_buf.is_none() {
            let u3d_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vm-u3d"),
                size: std::mem::size_of::<Compute3DUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            g.u3d_buf = Some(u3d_buf);
        }
        if g.u_sdf_buf.is_none() {
            let u_sdf_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vm-u-sdf"),
                size: std::mem::size_of::<ComputeSdfUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            g.u_sdf_buf = Some(u_sdf_buf);
        }
        g.u2d_bg = None;
        g.u3d_bg = None;
        g.u_sdf_bg = None;

        Ok(())
    }

    /// Dispatches 2D compute pipeline into a storage-capable surface.
    pub fn compute_draw_2d_into(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _surface: &mut Texture,
        fb_w: u32,
        fb_h: u32,
    ) -> crate::SceneVMResult<()> {
        if self.gpu.is_none() {
            self.init_gpu(device)?;
        }
        self.init_compute(device)?;
        self.upload_tile_metadata_to_gpu(device);
        // Ensure layer texture exists and matches size
        let (write_view, prev_view, next_front) =
            self.prepare_layer_views(device, queue, fb_w, fb_h);
        // Update uniforms
        let m = self.transform2d;
        let m_inv = mat3_inverse_f32(&m).unwrap_or(Mat3::<f32>::identity());
        let u = Compute2DUniforms {
            background: self.background.into_array(),
            fb_size: [fb_w, fb_h],
            _pad0: [0, 0],
            gp0: self.gp0.into_array(),
            gp1: self.gp1.into_array(),
            gp2: self.gp2.into_array(),
            gp3: self.gp3.into_array(),
            gp4: self.gp4.into_array(),
            gp5: self.gp5.into_array(),
            gp6: self.gp6.into_array(),
            gp7: self.gp7.into_array(),
            gp8: self.gp8.into_array(),
            gp9: self.gp9.into_array(),
            // Mat3 columns (col-major), pad .w = 0.0
            mat2d_c0: [m[(0, 0)], m[(1, 0)], m[(2, 0)], 0.0],
            mat2d_c1: [m[(0, 1)], m[(1, 1)], m[(2, 1)], 0.0],
            mat2d_c2: [m[(0, 2)], m[(1, 2)], m[(2, 2)], 0.0],
            mat2d_inv_c0: [m_inv[(0, 0)], m_inv[(1, 0)], m_inv[(2, 0)], 0.0],
            mat2d_inv_c1: [m_inv[(0, 1)], m_inv[(1, 1)], m_inv[(2, 1)], 0.0],
            mat2d_inv_c2: [m_inv[(0, 2)], m_inv[(1, 2)], m_inv[(2, 2)], 0.0],
            lights_count: self.lights.len() as u32,
            vm_flags: self.vm_flags(),
            anim_counter: self.animation_counter as u32,
            _pad_lights: 0,
            viewport_rect: self
                .viewport_rect2d
                .unwrap_or([0.0, 0.0, fb_w as f32, fb_h as f32]),
            palette: self.palette,
        };
        if let Some(g) = self.gpu.as_ref() {
            queue.write_buffer(g.u2d_buf.as_ref().unwrap(), 0, bytemuck::bytes_of(&u));
        }
        self.upload_atlas_to_gpu_with(device, queue);
        let (atlas_tex_view, atlas_mat_tex_view) = self
            .shared_atlas
            .texture_views()
            .expect("atlas GPU resources missing");

        let fb_dims = (fb_w, fb_h);
        let mut geometry_changed = false;
        if self.geometry2d_dirty || self.cached_v2.is_empty() || self.cached_fb_size_2d != fb_dims {
            let (verts_flat, indices_flat, tile_bins, tile_tris) =
                self.build_2d_batches(fb_w, fb_h);
            self.cached_v2 = verts_flat;
            self.cached_i2 = indices_flat;
            self.cached_tile_bins = tile_bins;
            self.cached_tile_tris = tile_tris;
            self.cached_fb_size_2d = fb_dims;
            self.geometry2d_dirty = false;
            geometry_changed = true;
            if self.activity_logging {
                self.log_layer(format!(
                    "2D geometry built: {} vertices, {} indices, {} tile bins",
                    self.cached_v2.len(),
                    self.cached_i2.len(),
                    self.cached_tile_bins.len()
                ));
            }
        }

        use wgpu::util::DeviceExt;

        let scene_data = self.build_scene_data_blob();
        {
            let g = self.gpu.as_mut().unwrap();
            g.scene_data_ssbo = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("vm-scene-data-ssbo"),
                    contents: &scene_data,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                },
            ));
        }

        let mut uploaded_geometry = false;
        {
            let g = self.gpu.as_mut().unwrap();

            if geometry_changed || g.v2d_ssbo.is_none() || g.i2d_ssbo.is_none() {
                let mut _v_dummy: Option<Vec<u8>> = None;
                let verts_bytes: &[u8] = if self.cached_v2.is_empty() {
                    _v_dummy = Some(
                        bytemuck::bytes_of(&Vert2DPod {
                            pos: [0.0, 0.0],
                            uv: [0.0, 0.0],
                            tile_index: 0,
                            _pad_tile: 0,
                        })
                        .to_vec(),
                    );
                    _v_dummy.as_ref().unwrap()
                } else {
                    bytemuck::cast_slice(&self.cached_v2)
                };
                let mut _i_dummy: Option<Vec<u8>> = None;
                let indices_bytes: &[u8] = if self.cached_i2.is_empty() {
                    _i_dummy = Some(0u32.to_ne_bytes().to_vec());
                    _i_dummy.as_ref().unwrap()
                } else {
                    bytemuck::cast_slice(&self.cached_i2)
                };

                g.v2d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-2d-verts-ssbo"),
                        contents: verts_bytes,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                g.i2d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-2d-indices-ssbo"),
                        contents: indices_bytes,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                if geometry_changed {
                    uploaded_geometry = true;
                }
            }

            if geometry_changed || g.tile_bins.is_none() || g.tile_tris.is_none() {
                let bins_slice: &[TileBinPod] = if self.cached_tile_bins.is_empty() {
                    &[]
                } else {
                    &self.cached_tile_bins
                };
                let tris_slice: &[u32] = if self.cached_tile_tris.is_empty() {
                    &DUMMY_U32_1
                } else {
                    &self.cached_tile_tris
                };

                g.tile_bins = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-2d-tile-bins"),
                        contents: if bins_slice.is_empty() {
                            bytemuck::bytes_of(&TileBinPod {
                                offset: 0,
                                count: 0,
                            })
                        } else {
                            bytemuck::cast_slice(bins_slice)
                        },
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                g.tile_tris = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-2d-tile-tris"),
                        contents: bytemuck::cast_slice(tris_slice),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    }),
                );
            }
        }
        if uploaded_geometry {
            self.log_layer(format!(
                "Uploaded {} 2D verts, {} indices (tiles: {})",
                self.cached_v2.len(),
                self.cached_i2.len(),
                self.cached_tile_bins.len()
            ));
        }

        // Build bind group with layer texture view and atlas, plus 2D geometry SSBOs
        let g = self.gpu.as_mut().unwrap();
        g.u2d_bg = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vm-u2d-bg"),
            layout: g.u2d_bgl.as_ref().unwrap(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: g.u2d_buf.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&write_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&atlas_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&g.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: g.v2d_ssbo.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: g.i2d_ssbo.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: g.tile_bins.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: g.tile_tris.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&atlas_mat_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: g.scene_data_ssbo.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: g.tile_meta_ssbo.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: g.tile_frames_ssbo.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: wgpu::BindingResource::Sampler(&g.sampler_linear),
                },
                wgpu::BindGroupEntry {
                    binding: 13,
                    resource: wgpu::BindingResource::TextureView(&prev_view),
                },
            ],
        }));
        // Dispatch
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vm-2d-cs-enc"),
        });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vm-2d-cs-pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(g.compute2d_pipeline.as_ref().unwrap());
            cpass.set_bind_group(0, g.u2d_bg.as_ref().unwrap(), &[]);

            // Use viewport rect if set, otherwise use full framebuffer
            let (dispatch_w, dispatch_h) = if let Some([_x, _y, w, h]) = self.viewport_rect2d {
                if w > 0.0 && h > 0.0 {
                    (w.ceil() as u32, h.ceil() as u32)
                } else {
                    (fb_w, fb_h)
                }
            } else {
                (fb_w, fb_h)
            };

            let gx = (dispatch_w + 7) / 8;
            let gy = (dispatch_h + 7) / 8;
            cpass.dispatch_workgroups(gx, gy, 1);
        }
        queue.submit(Some(encoder.finish()));
        if self.ping_pong_enabled {
            self.ping_pong_front = next_front;
        }
        Ok(())
    }

    /// Dispatches 3D compute pipeline into a storage-capable surface.
    pub fn compute_draw_3d_into(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _surface: &mut Texture,
        fb_w: u32,
        fb_h: u32,
    ) -> crate::SceneVMResult<()> {
        if self.gpu.is_none() {
            self.init_gpu(device)?;
        }
        self.init_compute(device)?;
        self.upload_tile_metadata_to_gpu(device);
        let (write_view, prev_view, next_front) =
            self.prepare_layer_views(device, queue, fb_w, fb_h);

        // --- Uniforms ---
        let m = self.transform3d;
        let c = self.camera3d;
        let u = Compute3DUniforms {
            background: self.background.into_array(),
            fb_size: [fb_w, fb_h],
            _pad0: [0, 0],
            gp0: self.gp0.into_array(),
            gp1: self.gp1.into_array(),
            gp2: self.gp2.into_array(),
            gp3: self.gp3.into_array(),
            gp4: self.gp4.into_array(),
            gp5: self.gp5.into_array(),
            gp6: self.gp6.into_array(),
            gp7: self.gp7.into_array(),
            gp8: self.gp8.into_array(),
            gp9: self.gp9.into_array(),
            mat3d_c0: [m[(0, 0)], m[(1, 0)], m[(2, 0)], m[(3, 0)]],
            mat3d_c1: [m[(0, 1)], m[(1, 1)], m[(2, 1)], m[(3, 1)]],
            mat3d_c2: [m[(0, 2)], m[(1, 2)], m[(2, 2)], m[(3, 2)]],
            mat3d_c3: [m[(0, 3)], m[(1, 3)], m[(2, 3)], m[(3, 3)]],
            lights_count: self.lights.len() as u32,
            vm_flags: self.vm_flags(),
            anim_counter: self.animation_counter as u32,
            _pad_lights: 0,
            cam_pos: [c.pos.x, c.pos.y, c.pos.z, 0.0],
            cam_fwd: [c.forward.x, c.forward.y, c.forward.z, 0.0],
            cam_right: [c.right.x, c.right.y, c.right.z, 0.0],
            cam_up: [c.up.x, c.up.y, c.up.z, 0.0],
            cam_vfov_deg: c.vfov_deg,
            cam_ortho_half_h: c.ortho_half_h,
            cam_near: c.near,
            cam_far: c.far,
            cam_kind: match c.kind {
                CameraKind::OrthoIso => 0,
                CameraKind::OrbitPersp => 1,
                CameraKind::FirstPersonPersp => 2,
            },
            _pad_cam: [0, 0, 0],
            _pad_tail: [0, 0, 0, 0],
            palette: self.palette,
        };
        if let Some(g) = self.gpu.as_ref() {
            queue.write_buffer(g.u3d_buf.as_ref().unwrap(), 0, bytemuck::bytes_of(&u));
        }

        self.upload_atlas_to_gpu_with(device, queue);
        let (_atlas_tex_view, _atlas_mat_tex_view) = self
            .shared_atlas
            .texture_views()
            .expect("atlas GPU resources missing");

        use wgpu::util::DeviceExt;
        let scene_data = self.build_scene_data_blob();
        {
            let g = self.gpu.as_mut().unwrap();
            g.scene_data_ssbo = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("vm-scene-data-ssbo"),
                    contents: &scene_data,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                },
            ));
        }

        // --- Build 3D geometry only when accel_dirty says so ---
        let mut geometry_changed = false;
        if self.accel_dirty || self.cached_v3.is_empty() {
            let mut v3: Vec<Vert3DPod> = Vec::new();
            let mut i3: Vec<u32> = Vec::new();
            let mut tri_visibility: Vec<bool> = Vec::new();

            for (_cid, ch) in &self.chunks_map {
                for poly_list in ch.polys3d_map.values() {
                    for poly in poly_list {
                        // IMPORTANT: Include ALL geometry in BVH, not just visible
                        // We'll track visibility separately
                        let tile_index = match self.shared_atlas.tile_index(&poly.tile_id) {
                            Some(idx) => idx,
                            None => continue,
                        };

                        let vcount = poly.vertices.len();
                        let mut poly_pos: Vec<[f32; 3]> = Vec::with_capacity(vcount);
                        let mut poly_nrm: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; vcount];

                        for v in &poly.vertices {
                            let p = m * Vec4::new(v[0], v[1], v[2], v[3]);
                            let w = if p.w != 0.0 { p.w } else { 1.0 };
                            poly_pos.push([p.x / w, p.y / w, p.z / w]);
                        }

                        for &(a, b, c) in &poly.indices {
                            let pa = poly_pos[a];
                            let pb = poly_pos[b];
                            let pc = poly_pos[c];
                            let e1 = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
                            let e2 = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
                            let nx = e1[1] * e2[2] - e1[2] * e2[1];
                            let ny = e1[2] * e2[0] - e1[0] * e2[2];
                            let nz = e1[0] * e2[1] - e1[1] * e2[0];
                            poly_nrm[a][0] += nx;
                            poly_nrm[a][1] += ny;
                            poly_nrm[a][2] += nz;
                            poly_nrm[b][0] += nx;
                            poly_nrm[b][1] += ny;
                            poly_nrm[b][2] += nz;
                            poly_nrm[c][0] += nx;
                            poly_nrm[c][1] += ny;
                            poly_nrm[c][2] += nz;
                        }
                        for n in &mut poly_nrm {
                            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                            if len > 1e-12 {
                                n[0] /= len;
                                n[1] /= len;
                                n[2] /= len;
                            }
                        }

                        let base = v3.len() as u32;

                        // Get blend texture index if available
                        let tile_index2 = if let Some(tid2) = poly.tile_id2 {
                            self.shared_atlas.tile_index(&tid2).unwrap_or(tile_index)
                        } else {
                            tile_index
                        };

                        // Validate blend_weights length matches vertices
                        let has_valid_blend = poly.tile_id2.is_some()
                            && poly.blend_weights.len() == poly.vertices.len();

                        for (i, p) in poly_pos.iter().enumerate() {
                            let uv0 = poly.uvs[i];
                            let n = poly_nrm[i];
                            let blend_factor = if has_valid_blend {
                                poly.blend_weights[i].clamp(0.0, 1.0)
                            } else {
                                0.0
                            };

                            v3.push(Vert3DPod {
                                pos: [p[0], p[1], p[2]],
                                _pad_pos: 0.0,
                                uv: [uv0[0], uv0[1]],
                                _pad_uv: [0.0, 0.0],
                                tile_index,
                                tile_index2,
                                blend_factor,
                                _pad_blend: 0.0,
                                normal: [n[0], n[1], n[2]],
                                _pad_n: 0.0,
                            });
                        }

                        for &(a, b, c) in &poly.indices {
                            i3.extend_from_slice(&[
                                base + a as u32,
                                base + b as u32,
                                base + c as u32,
                            ]);
                            // Track visibility per triangle
                            tri_visibility.push(poly.visible);
                        }
                    }
                }
            }

            if v3.is_empty() {
                v3.push(Vert3DPod {
                    pos: [0.0; 3],
                    _pad_pos: 0.0,
                    uv: [0.0; 2],
                    _pad_uv: [0.0, 0.0],
                    tile_index: 0,
                    tile_index2: 0,
                    blend_factor: 0.0,
                    _pad_blend: 0.0,
                    normal: [0.0, 0.0, 1.0],
                    _pad_n: 0.0,
                });
            }
            if i3.is_empty() {
                // AMD fix: Ensure minimum 16-byte buffer size
                i3.extend_from_slice(&[0u32; 4]);
                tri_visibility.push(false);
            }

            self.cached_v3 = v3;
            self.cached_i3 = i3;

            // Convert bool visibility to packed u32 bitmask
            let tri_count = tri_visibility.len();
            let word_count = (tri_count + 31) / 32;
            let mut visibility_bits = vec![0u32; word_count.max(1)];
            for (tri_idx, &visible) in tri_visibility.iter().enumerate() {
                if visible {
                    let word_idx = tri_idx / 32;
                    let bit_idx = tri_idx % 32;
                    visibility_bits[word_idx] |= 1u32 << bit_idx;
                }
            }
            self.cached_tri_visibility = visibility_bits;

            geometry_changed = true;
            self.visibility_dirty = false; // Reset since we just rebuilt everything
        }

        // --- Update visibility buffer if only visibility changed (no geometry rebuild) ---
        if self.visibility_dirty && !geometry_changed {
            // Rebuild visibility bitmask from current chunk data
            let mut tri_visibility: Vec<bool> = Vec::new();

            for (_cid, ch) in &self.chunks_map {
                for poly_list in ch.polys3d_map.values() {
                    for poly in poly_list {
                        // Count triangles for this poly
                        for _ in &poly.indices {
                            tri_visibility.push(poly.visible);
                        }
                    }
                }
            }

            if tri_visibility.is_empty() {
                tri_visibility.push(false);
            }

            // Convert to packed bitmask
            let tri_count = tri_visibility.len();
            let word_count = (tri_count + 31) / 32;
            let mut visibility_bits = vec![0u32; word_count.max(1)];
            for (tri_idx, &visible) in tri_visibility.iter().enumerate() {
                if visible {
                    let word_idx = tri_idx / 32;
                    let bit_idx = tri_idx % 32;
                    visibility_bits[word_idx] |= 1u32 << bit_idx;
                }
            }
            self.cached_tri_visibility = visibility_bits;
            self.visibility_dirty = false;
        }

        let mut grid_changed = false;
        if self.accel_dirty {
            self.scene_accel.bvh =
                Self::build_scene_bvh_from(&self.cached_v3, &self.cached_i3, self.bvh_leaf_size);
            grid_changed = true;
            self.accel_dirty = false;
        }
        let gr = &self.scene_accel.bvh;

        let mut uploaded_grid = false;
        let mut uploaded_geom = false;
        {
            let g = self.gpu.as_mut().unwrap();
            let need_grid_upload = grid_changed || g.grid_hdr.is_none() || g.grid_data.is_none();
            if need_grid_upload {
                let node_data: Vec<u32> = if gr.nodes.is_empty() {
                    vec![0]
                } else {
                    gr.nodes.clone()
                };
                let tris_data: Vec<u32> = if gr.tri_indices.is_empty() {
                    vec![0]
                } else {
                    gr.tri_indices.clone()
                };

                let nodes_start = 0u32;
                let tris_start = nodes_start + node_data.len() as u32;

                // Append visibility bitmask to grid_data to avoid extra storage buffer
                let visibility_data = if self.cached_tri_visibility.is_empty() {
                    vec![0u32]
                } else {
                    self.cached_tri_visibility.clone()
                };
                let vis_start = tris_start + tris_data.len() as u32;
                let vis_word_count = visibility_data.len() as u32;

                let mut combined: Vec<u32> =
                    Vec::with_capacity(node_data.len() + tris_data.len() + visibility_data.len());
                combined.extend_from_slice(&node_data);
                combined.extend_from_slice(&tris_data);
                combined.extend_from_slice(&visibility_data);

                let grid_hdr_data = Grid3DHeader {
                    origin: [gr.origin.x, gr.origin.y, gr.origin.z, 0.0],
                    cell_size: [gr.extent.x, gr.extent.y, gr.extent.z, 0.0],
                    dims: [1, 1, 1, 0],
                    ranges: [nodes_start, tris_start, gr.node_count, gr.tri_count],
                    visibility: [vis_start, vis_word_count, 0, 0],
                };

                g.grid_hdr = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-grid3d-hdr"),
                        contents: bytemuck::bytes_of(&grid_hdr_data),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                g.grid_data = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-grid3d-data"),
                        contents: bytemuck::cast_slice(&combined),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                uploaded_grid = grid_changed;
            }

            let need_geom_upload = geometry_changed || g.v3d_ssbo.is_none() || g.i3d_ssbo.is_none();
            if need_geom_upload {
                g.v3d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-3d-verts-ssbo"),
                        contents: bytemuck::cast_slice(&self.cached_v3),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                g.i3d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-3d-indices-ssbo"),
                        contents: bytemuck::cast_slice(&self.cached_i3),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                if geometry_changed {
                    uploaded_geom = true;
                }
            }
        }
        if uploaded_geom {
            self.log_layer(format!(
                "Uploaded {} 3D verts, {} indices",
                self.cached_v3.len(),
                self.cached_i3.len()
            ));
        }
        if uploaded_grid {
            let gr = &self.scene_accel.bvh;
            self.log_layer(format!(
                "Rebuilt 3D BVH accel nodes {}, tris {}",
                gr.node_count, gr.tri_count
            ));
        }

        let (atlas_view, atlas_mat_view) = self
            .shared_atlas
            .texture_views()
            .expect("atlas GPU resources missing");

        // Build the bind group
        {
            let g = self.gpu.as_mut().unwrap();
            g.u3d_bg = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vm-u3d-bg"),
                layout: g.u3d_bgl.as_ref().unwrap(),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: g.u3d_buf.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&write_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&atlas_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&g.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: wgpu::BindingResource::TextureView(&atlas_mat_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: g.scene_data_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: g.v3d_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: g.i3d_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: g.grid_hdr.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: g.grid_data.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 12,
                        resource: g.tile_meta_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 13,
                        resource: g.tile_frames_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 14,
                        resource: wgpu::BindingResource::TextureView(&prev_view),
                    },
                ],
            }));
        }

        // Dispatch
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vm-3d-cs-enc"),
        });
        {
            let g = self.gpu.as_ref().unwrap();
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vm-3d-cs-pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(g.compute3d_pipeline.as_ref().unwrap());
            cpass.set_bind_group(0, g.u3d_bg.as_ref().unwrap(), &[]);
            let gx = (fb_w + 7) / 8;
            let gy = (fb_h + 7) / 8;
            cpass.dispatch_workgroups(gx, gy, 1);
        }
        queue.submit(Some(encoder.finish()));
        if self.ping_pong_enabled {
            if self.activity_logging {
                println!(
                    "[VM Layer {}] Ping-pong swap: {} -> {}, anim_counter: {}",
                    self.layer_index, self.ping_pong_front, next_front, self.animation_counter
                );
            }
            self.ping_pong_front = next_front;
        }

        Ok(())
    }

    /// Dispatches the SDF compute pipeline into a storage-capable surface.
    pub fn compute_draw_sdf_into(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _surface: &mut Texture,
        fb_w: u32,
        fb_h: u32,
    ) -> crate::SceneVMResult<()> {
        if self.gpu.is_none() {
            self.init_gpu(device)?;
        }
        self.init_compute(device)?;
        let (write_view, prev_view, next_front) =
            self.prepare_layer_views(device, queue, fb_w, fb_h);
        self.upload_atlas_to_gpu_with(device, queue);
        let c = self.camera3d;

        let u = ComputeSdfUniforms {
            background: self.background.into_array(),
            fb_size: [fb_w, fb_h],
            _pad0: [0, 0],
            gp0: self.gp0.into_array(),
            gp1: self.gp1.into_array(),
            gp2: self.gp2.into_array(),
            gp3: self.gp3.into_array(),
            gp4: self.gp4.into_array(),
            gp5: self.gp5.into_array(),
            gp6: self.gp6.into_array(),
            gp7: self.gp7.into_array(),
            gp8: self.gp8.into_array(),
            gp9: self.gp9.into_array(),
            cam_pos: [c.pos.x, c.pos.y, c.pos.z, 0.0],
            cam_fwd: [c.forward.x, c.forward.y, c.forward.z, 0.0],
            cam_right: [c.right.x, c.right.y, c.right.z, 0.0],
            cam_up: [c.up.x, c.up.y, c.up.z, 0.0],
            cam_vfov_deg: c.vfov_deg,
            cam_ortho_half_h: c.ortho_half_h,
            cam_near: c.near,
            cam_far: c.far,
            cam_kind: match c.kind {
                CameraKind::OrthoIso => 0,
                CameraKind::OrbitPersp => 1,
                CameraKind::FirstPersonPersp => 2,
            },
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
            data_len: (self.sdf_data.len().min(u32::MAX as usize)) as u32,
            vm_flags: self.vm_flags(),
            anim_counter: self.animation_counter as u32,
            _pad4: 0,
            viewport_rect: self
                .viewport_rect2d
                .unwrap_or([0.0, 0.0, fb_w as f32, fb_h as f32]),
            palette: self.palette,
            _pad_end: [[0; 4]; 4],
        };

        if let Some(g) = self.gpu.as_ref() {
            queue.write_buffer(g.u_sdf_buf.as_ref().unwrap(), 0, bytemuck::bytes_of(&u));
        }

        self.upload_sdf_data_to_gpu(device);

        let g = self.gpu.as_mut().unwrap();
        let (atlas_tex_view, _atlas_mat_tex_view) = self
            .shared_atlas
            .texture_views()
            .expect("atlas GPU resources missing");
        g.u_sdf_bg = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vm-sdf-bg"),
            layout: g.u_sdf_bgl.as_ref().unwrap(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: g.u_sdf_buf.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&write_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: g.sdf_data_ssbo.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&atlas_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&g.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&prev_view),
                },
            ],
        }));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vm-sdf-cs-enc"),
        });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vm-sdf-cs-pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(g.compute_sdf_pipeline.as_ref().unwrap());
            cpass.set_bind_group(0, g.u_sdf_bg.as_ref().unwrap(), &[]);
            let (dispatch_w, dispatch_h) = if let Some([_x, _y, w, h]) = self.viewport_rect2d {
                if w > 0.0 && h > 0.0 {
                    (w.ceil() as u32, h.ceil() as u32)
                } else {
                    (fb_w, fb_h)
                }
            } else {
                (fb_w, fb_h)
            };

            let gx = (dispatch_w + 7) / 8;
            let gy = (dispatch_h + 7) / 8;
            cpass.dispatch_workgroups(gx, gy, 1);
        }
        queue.submit(Some(encoder.finish()));
        if self.ping_pong_enabled {
            self.ping_pong_front = next_front;
        }

        Ok(())
    }

    /// Cast a CPU-side ray through a normalized screen UV and return the hit GeoId (if any).
    /// Uses the same camera model and 3D transforms as the GPU compute path.
    /// Returns the GeoId, world-space hit position, and the distance along the ray.
    pub fn pick_geo_id_at_uv(
        &self,
        fb_w: u32,
        fb_h: u32,
        screen_uv: [f32; 2],
        include_hidden: bool,
        include_billboards: bool,
    ) -> Option<(GeoId, Vec3<f32>, f32)> {
        if fb_w == 0 || fb_h == 0 {
            return None;
        }

        let (ray_origin, ray_dir) = camera_ray_from_uv(&self.camera3d, fb_w, fb_h, screen_uv);
        let mut best_t = f32::INFINITY;
        let mut best_geo: Option<GeoId> = None;
        let mut best_pos = Vec3::new(0.0, 0.0, 0.0);

        let m = self.transform3d;

        for chunk in self.chunks_map.values() {
            for poly_list in chunk.polys3d_map.values() {
                for poly in poly_list {
                    if poly.indices.is_empty() || poly.vertices.is_empty() {
                        continue;
                    }

                    if !poly.visible && !include_hidden {
                        continue;
                    }

                    let mut poly_pos: Vec<[f32; 3]> = Vec::with_capacity(poly.vertices.len());
                    for v in &poly.vertices {
                        let p = m * Vec4::new(v[0], v[1], v[2], v[3]);
                        let w = if p.w != 0.0 { p.w } else { 1.0 };
                        poly_pos.push([p.x / w, p.y / w, p.z / w]);
                    }

                    for &(ia, ib, ic) in &poly.indices {
                        let a = poly_pos.get(ia).copied();
                        let b = poly_pos.get(ib).copied();
                        let c = poly_pos.get(ic).copied();
                        let (a, b, c) = match (a, b, c) {
                            (Some(a), Some(b), Some(c)) => (a, b, c),
                            _ => continue,
                        };
                        if let Some((t, _, _)) =
                            ray_triangle_intersect(ray_origin, ray_dir, a, b, c)
                        {
                            if t > 1e-5 && t < best_t {
                                best_t = t;
                                best_geo = Some(poly.id);
                                best_pos = ray_origin + ray_dir * t;
                            }
                        }
                    }
                }
            }
        }

        if include_billboards {
            // Include dynamic billboards in hit testing (camera-facing quads).
            for obj in self
                .dynamic_objects
                .iter()
                .chain(self.dynamic_avatar_objects.values())
            {
                if obj.kind != DynamicKind::BillboardTile
                    && obj.kind != DynamicKind::BillboardAvatar
                {
                    continue;
                }

                let half_w = (obj.width * 0.5).max(0.0);
                let half_h = (obj.height * 0.5).max(0.0);
                if !half_w.is_finite() || !half_h.is_finite() || half_w <= 0.0 || half_h <= 0.0 {
                    continue;
                }

                // Transform center (with translation) and axes (direction only).
                let center_v4 = m * Vec4::new(obj.center.x, obj.center.y, obj.center.z, 1.0);
                let center_w = if center_v4.w != 0.0 { center_v4.w } else { 1.0 };
                let center = Vec3::new(
                    center_v4.x / center_w,
                    center_v4.y / center_w,
                    center_v4.z / center_w,
                );

                let axis_r_v4 = m * Vec4::new(
                    obj.view_right.x * half_w,
                    obj.view_right.y * half_w,
                    obj.view_right.z * half_w,
                    0.0,
                );
                let axis_u_v4 = m * Vec4::new(
                    obj.view_up.x * half_h,
                    obj.view_up.y * half_h,
                    obj.view_up.z * half_h,
                    0.0,
                );

                let axis_r = Vec3::new(axis_r_v4.x, axis_r_v4.y, axis_r_v4.z);
                let axis_u = Vec3::new(axis_u_v4.x, axis_u_v4.y, axis_u_v4.z);

                let normal = axis_r.cross(axis_u);
                let normal_len = normal.magnitude();
                if normal_len < 1e-6 || !normal_len.is_finite() {
                    continue;
                }

                let denom = normal.dot(ray_dir);
                if denom.abs() < 1e-6 {
                    continue; // Ray parallel to billboard plane
                }

                let t = normal.dot(center - ray_origin) / denom;
                if t <= 1e-5 || t >= best_t {
                    continue;
                }

                let hit = ray_origin + ray_dir * t;
                let rel = hit - center;

                // Solve rel = u*axis_r + v*axis_u (works even if axes are not orthonormal).
                let aa = axis_r.dot(axis_r);
                let bb = axis_u.dot(axis_u);
                let ab = axis_r.dot(axis_u);
                let denom_uv = aa * bb - ab * ab;
                if denom_uv.abs() < 1e-8 {
                    continue;
                }
                let ar = rel.dot(axis_r);
                let au = rel.dot(axis_u);
                let u = (ar * bb - au * ab) / denom_uv;
                let v = (au * aa - ar * ab) / denom_uv;

                if u.abs() <= 1.0 + 1e-4 && v.abs() <= 1.0 + 1e-4 {
                    best_t = t;
                    best_geo = Some(obj.id);
                    best_pos = hit;
                }
            }
        }

        best_geo.map(|id| (id, best_pos, best_t))
    }

    /// Collect all visible GeoIds of the requested variant whose screen-space projection
    /// intersects the provided rectangle.
    /// `rect_min` and `rect_max` are in screen pixels (top-left and bottom-right corners).
    pub fn pick_geo_ids_in_rect(
        &self,
        fb_w: u32,
        fb_h: u32,
        rect_min: Vec2<f32>,
        rect_max: Vec2<f32>,
        target_kind: GeoId,
        include_hidden: bool,
        include_billboards: bool,
    ) -> Vec<GeoId> {
        if fb_w == 0 || fb_h == 0 {
            return Vec::new();
        }

        let fb_w_f = fb_w as f32;
        let fb_h_f = fb_h as f32;

        let min_x = rect_min.x.min(rect_max.x);
        let min_y = rect_min.y.min(rect_max.y);
        let max_x = rect_min.x.max(rect_max.x);
        let max_y = rect_min.y.max(rect_max.y);

        let rect_min = Vec2::new(min_x.clamp(0.0, fb_w_f), min_y.clamp(0.0, fb_h_f));
        let rect_max = Vec2::new(max_x.clamp(0.0, fb_w_f), max_y.clamp(0.0, fb_h_f));

        if rect_min.x >= rect_max.x || rect_min.y >= rect_max.y {
            return Vec::new();
        }

        // Sample every pixel in the rectangle
        let min_x_i = rect_min.x.floor() as u32;
        let min_y_i = rect_min.y.floor() as u32;
        let max_x_i = rect_max.x.ceil() as u32;
        let max_y_i = rect_max.y.ceil() as u32;

        #[cfg(target_arch = "wasm32")]
        {
            let mut seen = FxHashSet::default();
            for y in min_y_i..max_y_i {
                for x in min_x_i..max_x_i {
                    let screen_uv = [x as f32 / fb_w_f, y as f32 / fb_h_f];
                    if let Some((geo_id, _, _)) = self.pick_geo_id_at_uv(
                        fb_w,
                        fb_h,
                        screen_uv,
                        include_hidden,
                        include_billboards,
                    ) {
                        if std::mem::discriminant(&geo_id) == std::mem::discriminant(&target_kind) {
                            seen.insert(geo_id);
                        }
                    }
                }
            }
            return seen.into_iter().collect();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::Mutex;
            let seen = Mutex::new(FxHashSet::default());
            (min_y_i..max_y_i).into_par_iter().for_each(|y| {
                for x in min_x_i..max_x_i {
                    let screen_uv = [x as f32 / fb_w_f, y as f32 / fb_h_f];

                    if let Some((geo_id, _, _)) = self.pick_geo_id_at_uv(
                        fb_w,
                        fb_h,
                        screen_uv,
                        include_hidden,
                        include_billboards,
                    ) {
                        if std::mem::discriminant(&geo_id) == std::mem::discriminant(&target_kind) {
                            seen.lock().unwrap().insert(geo_id);
                        }
                    }
                }
            });

            return seen.into_inner().unwrap().into_iter().collect();
        }
    }

    /// Build a world-space ray from screen uv (0..1) using the current camera.
    pub fn ray_from_uv(
        &self,
        fb_w: u32,
        fb_h: u32,
        screen_uv: [f32; 2],
    ) -> Option<(Vec3<f32>, Vec3<f32>)> {
        if fb_w == 0 || fb_h == 0 {
            return None;
        }
        Some(camera_ray_from_uv(&self.camera3d, fb_w, fb_h, screen_uv))
    }

    fn build_scene_bvh_from(verts: &[Vert3DPod], indices: &[u32], leaf_size: u32) -> SceneBvhAccel {
        use vek::Vec3;

        #[derive(Clone, Copy, Debug, Default)]
        struct BvhNode {
            bmin: Vec3<f32>,
            bmax: Vec3<f32>,
            left_first: u32,
            tri_count: u32,
        }

        #[inline(always)]
        fn vmin(a: Vec3<f32>, b: Vec3<f32>) -> Vec3<f32> {
            Vec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))
        }
        #[inline(always)]
        fn vmax(a: Vec3<f32>, b: Vec3<f32>) -> Vec3<f32> {
            Vec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))
        }

        // --- Scene bounds over all vertices ---
        let mut scene_min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut scene_max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for v in verts {
            let p = Vec3::new(v.pos[0], v.pos[1], v.pos[2]);
            scene_min = vmin(scene_min, p);
            scene_max = vmax(scene_max, p);
        }

        if !scene_min.x.is_finite() {
            // Empty scene guard: keep bindings valid with a small dummy box
            return SceneBvhAccel {
                origin: Vec3::zero(),
                extent: Vec3::broadcast(1.0),
                nodes: vec![0],
                tri_indices: vec![0],
                node_count: 0,
                tri_count: 0,
            };
        }

        // Pad bounds slightly for numerical robustness
        let diag = (scene_max - scene_min).magnitude().max(1e-6);
        let pad = 0.1 * diag;
        scene_min -= Vec3::broadcast(pad);
        scene_max += Vec3::broadcast(pad);

        let mut extent = scene_max - scene_min;
        extent.x = extent.x.max(1e-4);
        extent.y = extent.y.max(1e-4);
        extent.z = extent.z.max(1e-4);

        let tri_count = indices.len() / 3;
        if tri_count == 0 {
            return SceneBvhAccel {
                origin: scene_min,
                extent,
                nodes: vec![0],
                tri_indices: vec![0],
                node_count: 0,
                tri_count: 0,
            };
        }

        // Leaf size is a direct knob; clamp to keep traversal stack small.
        let mut leaf_size = leaf_size.max(1);
        leaf_size = leaf_size.min(16);

        // Precompute tri bounds and centroids
        let mut tri_bounds: Vec<(Vec3<f32>, Vec3<f32>)> = Vec::with_capacity(tri_count);
        let mut tri_centroids: Vec<Vec3<f32>> = Vec::with_capacity(tri_count);
        for tri in 0..tri_count {
            let i0 = indices[3 * tri + 0] as usize;
            let i1 = indices[3 * tri + 1] as usize;
            let i2 = indices[3 * tri + 2] as usize;

            let p0 = Vec3::new(verts[i0].pos[0], verts[i0].pos[1], verts[i0].pos[2]);
            let p1 = Vec3::new(verts[i1].pos[0], verts[i1].pos[1], verts[i1].pos[2]);
            let p2 = Vec3::new(verts[i2].pos[0], verts[i2].pos[1], verts[i2].pos[2]);

            let tmin = vmin(vmin(p0, p1), p2);
            let tmax = vmax(vmax(p0, p1), p2);
            tri_bounds.push((tmin, tmax));
            tri_centroids.push((p0 + p1 + p2) / 3.0);
        }

        // BVH nodes + triangle ordering array (re-ordered in place)
        let mut nodes: Vec<BvhNode> = Vec::new();
        nodes.push(BvhNode::default()); // root placeholder
        let mut tri_indices: Vec<u32> = (0..tri_count as u32).collect();

        // Recursively build a binary BVH using median split on the widest centroid axis.
        fn surface_area(e: Vec3<f32>) -> f32 {
            let ex = e.x.max(0.0);
            let ey = e.y.max(0.0);
            let ez = e.z.max(0.0);
            2.0 * (ex * ey + ey * ez + ez * ex).max(1e-12)
        }

        // Recursively build a binary BVH using binned SAH on centroid axis.
        fn build_node(
            node_idx: usize,
            start: u32,
            count: u32,
            leaf_size: u32,
            nodes: &mut Vec<BvhNode>,
            tri_indices: &mut [u32],
            tri_bounds: &[(Vec3<f32>, Vec3<f32>)],
            tri_centroids: &[Vec3<f32>],
        ) {
            let start_usize = start as usize;
            let count_usize = count as usize;
            let end = start_usize + count_usize;

            let mut bmin = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
            let mut bmax = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
            for &t in &tri_indices[start_usize..end] {
                let (tmin, tmax) = tri_bounds[t as usize];
                bmin = vmin(bmin, tmin);
                bmax = vmax(bmax, tmax);
            }
            nodes[node_idx].bmin = bmin;
            nodes[node_idx].bmax = bmax;

            if count <= leaf_size {
                nodes[node_idx].left_first = start;
                nodes[node_idx].tri_count = count;
                return;
            }

            // Centroid bounds for split axis selection
            let mut cmin = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
            let mut cmax = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
            for &t in &tri_indices[start_usize..end] {
                let c = tri_centroids[t as usize];
                cmin = vmin(cmin, c);
                cmax = vmax(cmax, c);
            }
            let cextent = cmax - cmin;
            const BINS: usize = 16;
            let mut best_axis = 3usize;
            let mut best_cost = f32::INFINITY;
            let mut best_split_bin = 0usize;

            for axis in 0..3 {
                let extent_axis = match axis {
                    0 => cextent.x,
                    1 => cextent.y,
                    _ => cextent.z,
                };
                if extent_axis < 1e-6 {
                    continue;
                }

                let mut bin_count = [0u32; BINS];
                let mut bin_bmin = [Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY); BINS];
                let mut bin_bmax =
                    [Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY); BINS];

                for &t in &tri_indices[start_usize..end] {
                    let c = tri_centroids[t as usize];
                    let c_axis = match axis {
                        0 => c.x,
                        1 => c.y,
                        _ => c.z,
                    };
                    let mut bin = (((c_axis
                        - match axis {
                            0 => cmin.x,
                            1 => cmin.y,
                            _ => cmin.z,
                        })
                        / extent_axis)
                        * ((BINS - 1) as f32)) as i32;
                    bin = bin.clamp(0, (BINS - 1) as i32);
                    let bin = bin as usize;
                    bin_count[bin] += 1;
                    let (tmin, tmax) = tri_bounds[t as usize];
                    bin_bmin[bin] = vmin(bin_bmin[bin], tmin);
                    bin_bmax[bin] = vmax(bin_bmax[bin], tmax);
                }

                let mut prefix_count = [0u32; BINS];
                let mut prefix_bmin =
                    [Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY); BINS];
                let mut prefix_bmax =
                    [Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY); BINS];

                let mut running_count = 0u32;
                let mut running_bmin = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
                let mut running_bmax =
                    Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
                for i in 0..BINS {
                    running_count += bin_count[i];
                    running_bmin = vmin(running_bmin, bin_bmin[i]);
                    running_bmax = vmax(running_bmax, bin_bmax[i]);
                    prefix_count[i] = running_count;
                    prefix_bmin[i] = running_bmin;
                    prefix_bmax[i] = running_bmax;
                }

                let mut suffix_count = [0u32; BINS];
                let mut suffix_bmin =
                    [Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY); BINS];
                let mut suffix_bmax =
                    [Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY); BINS];

                let mut running_count_r = 0u32;
                let mut running_bmin_r = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
                let mut running_bmax_r =
                    Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
                for i in (0..BINS).rev() {
                    running_count_r += bin_count[i];
                    running_bmin_r = vmin(running_bmin_r, bin_bmin[i]);
                    running_bmax_r = vmax(running_bmax_r, bin_bmax[i]);
                    suffix_count[i] = running_count_r;
                    suffix_bmin[i] = running_bmin_r;
                    suffix_bmax[i] = running_bmax_r;
                }

                for split_bin in 0..(BINS - 1) {
                    let left_count = prefix_count[split_bin];
                    let right_count = suffix_count[split_bin + 1];
                    if left_count == 0 || right_count == 0 {
                        continue;
                    }
                    let left_sa = surface_area(prefix_bmax[split_bin] - prefix_bmin[split_bin]);
                    let right_sa =
                        surface_area(suffix_bmax[split_bin + 1] - suffix_bmin[split_bin + 1]);
                    let cost = left_sa * left_count as f32 + right_sa * right_count as f32;
                    if cost < best_cost {
                        best_cost = cost;
                        best_axis = axis;
                        best_split_bin = split_bin;
                    }
                }
            }

            let node_sa = surface_area(bmax - bmin);
            let leaf_cost = node_sa * count as f32;

            if best_axis == 3 || best_cost >= leaf_cost {
                nodes[node_idx].left_first = start;
                nodes[node_idx].tri_count = count;
                return;
            }

            // Partition by best axis/split (using bin membership to match SAH evaluation)
            let cmin_axis = match best_axis {
                0 => cmin.x,
                1 => cmin.y,
                _ => cmin.z,
            };
            let cextent_axis = match best_axis {
                0 => cextent.x,
                1 => cextent.y,
                _ => cextent.z,
            };

            let mut i = start_usize;
            let mut j = end - 1;
            while i <= j {
                let ci_val = tri_centroids[tri_indices[i] as usize][best_axis];
                let mut bin_i =
                    (((ci_val - cmin_axis) / cextent_axis) * ((BINS - 1) as f32)) as i32;
                bin_i = bin_i.clamp(0, (BINS - 1) as i32);

                if bin_i as usize <= best_split_bin {
                    i += 1;
                    continue;
                }

                let cj_val = tri_centroids[tri_indices[j] as usize][best_axis];
                let mut bin_j =
                    (((cj_val - cmin_axis) / cextent_axis) * ((BINS - 1) as f32)) as i32;
                bin_j = bin_j.clamp(0, (BINS - 1) as i32);

                if bin_j as usize > best_split_bin {
                    if j == 0 {
                        break;
                    }
                    j -= 1;
                    continue;
                }

                tri_indices.swap(i, j);
                i += 1;
                if j == 0 {
                    break;
                }
                j -= 1;
            }

            let mid = i.max(start_usize + 1).min(end - 1);
            let left_count = (mid - start_usize) as u32;
            let right_count = count - left_count;
            if left_count == 0 || right_count == 0 {
                nodes[node_idx].left_first = start;
                nodes[node_idx].tri_count = count;
                return;
            }

            let left_idx = nodes.len();
            nodes[node_idx].left_first = left_idx as u32;
            nodes[node_idx].tri_count = 0;
            nodes.push(BvhNode::default());
            nodes.push(BvhNode::default());

            build_node(
                left_idx,
                start,
                left_count,
                leaf_size,
                nodes,
                tri_indices,
                tri_bounds,
                tri_centroids,
            );
            build_node(
                left_idx + 1,
                mid as u32,
                right_count,
                leaf_size,
                nodes,
                tri_indices,
                tri_bounds,
                tri_centroids,
            );
        }

        build_node(
            0,
            0,
            tri_count as u32,
            leaf_size,
            &mut nodes,
            &mut tri_indices,
            &tri_bounds,
            &tri_centroids,
        );

        // Flatten nodes into u32 words for the GPU buffer
        let mut node_data: Vec<u32> = Vec::with_capacity(nodes.len() * 8);
        for n in &nodes {
            node_data.push(f32::to_bits(n.bmin.x));
            node_data.push(f32::to_bits(n.bmin.y));
            node_data.push(f32::to_bits(n.bmin.z));
            node_data.push(f32::to_bits(n.bmax.x));
            node_data.push(f32::to_bits(n.bmax.y));
            node_data.push(f32::to_bits(n.bmax.z));
            node_data.push(n.left_first);
            node_data.push(n.tri_count);
        }

        SceneBvhAccel {
            origin: scene_min,
            extent,
            nodes: if node_data.is_empty() {
                vec![0]
            } else {
                node_data
            },
            tri_indices: if tri_indices.is_empty() {
                vec![0]
            } else {
                tri_indices
            },
            node_count: nodes.len() as u32,
            tri_count: tri_count as u32,
        }
    }

    /// Unified draw entry: chooses 2D or 3D compute path based on `self.render_mode`.
    pub fn draw_into(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: &mut Texture,
        fb_w: u32,
        fb_h: u32,
    ) -> crate::SceneVMResult<()> {
        // Skip rendering if this VM layer is disabled
        if !self.enabled {
            return Ok(());
        }

        if self.gpu.is_none() {
            self.init_gpu(device)?;
        }

        match self.render_mode {
            RenderMode::Compute2D => {
                self.compute_draw_2d_into(device, queue, surface, fb_w, fb_h)?;

                if self.activity_logging {
                    self.log_layer("2D compute draw completed".to_string());
                }
            }
            RenderMode::Compute3D => {
                self.compute_draw_3d_into(device, queue, surface, fb_w, fb_h)?;

                if self.activity_logging {
                    self.log_layer("3D compute draw completed".to_string());
                }
            }
            RenderMode::Sdf => {
                self.compute_draw_sdf_into(device, queue, surface, fb_w, fb_h)?;

                if self.activity_logging {
                    self.log_layer("SDF compute draw completed".to_string());
                }
            }
        }

        Ok(())
    }
} // end impl VM

// Helper for inverting a 3x3 matrix (vek::Mat3<f32>)
fn mat3_inverse_f32(m: &Mat3<f32>) -> Option<Mat3<f32>> {
    // Treat elements as a standard 3x3 laid out by rows using vek indexing (col, row)
    let a = m[(0, 0)];
    let b = m[(1, 0)];
    let c = m[(2, 0)];
    let d = m[(0, 1)];
    let e = m[(1, 1)];
    let f = m[(2, 1)];
    let g = m[(0, 2)];
    let h = m[(1, 2)];
    let i = m[(2, 2)];

    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-8 {
        return None;
    }
    let inv_det = 1.0 / det;

    let m00 = (e * i - f * h) * inv_det;
    let m01 = (c * h - b * i) * inv_det;
    let m02 = (b * f - c * e) * inv_det;

    let m10 = (f * g - d * i) * inv_det;
    let m11 = (a * i - c * g) * inv_det;
    let m12 = (c * d - a * f) * inv_det;

    let m20 = (d * h - e * g) * inv_det;
    let m21 = (b * g - a * h) * inv_det;
    let m22 = (a * e - b * d) * inv_det;

    let mut out = Mat3::<f32>::zero();
    // Write back using vek's (col,row) indexing
    out[(0, 0)] = m00;
    out[(1, 0)] = m01;
    out[(2, 0)] = m02;
    out[(0, 1)] = m10;
    out[(1, 1)] = m11;
    out[(2, 1)] = m12;
    out[(0, 2)] = m20;
    out[(1, 2)] = m21;
    out[(2, 2)] = m22;
    Some(out)
}

/// Build a world-space ray from screen uv (0..1) using the current camera parameters.
pub fn cpu_ray_from_uv(
    cam: &Camera3D,
    fb_w: u32,
    fb_h: u32,
    screen_uv: [f32; 2],
) -> (Vec3<f32>, Vec3<f32>) {
    camera_ray_from_uv(cam, fb_w, fb_h, screen_uv)
}

fn camera_ray_from_uv(
    camera: &Camera3D,
    fb_w: u32,
    fb_h: u32,
    screen_uv: [f32; 2],
) -> (Vec3<f32>, Vec3<f32>) {
    let u = screen_uv[0].clamp(0.0, 1.0);
    let v = screen_uv[1].clamp(0.0, 1.0);
    let ndc_x = u * 2.0 - 1.0;
    let ndc_y = (v * 2.0 - 1.0) * -1.0;

    let fb_w_f = fb_w.max(1) as f32;
    let fb_h_f = fb_h.max(1) as f32;

    match camera.kind {
        CameraKind::OrthoIso => {
            let aspect = fb_w_f / fb_h_f;
            let half_w = camera.ortho_half_h * aspect;
            let origin = camera.pos
                + camera.right * (ndc_x * half_w)
                + camera.up * (ndc_y * camera.ortho_half_h);
            (origin, camera.forward.normalized())
        }
        CameraKind::OrbitPersp | CameraKind::FirstPersonPersp => {
            let tan_half = (camera.vfov_deg.to_radians() * 0.5).tan();
            let aspect = fb_w_f / fb_h_f;
            let dx = ndc_x * aspect * tan_half;
            let dy = ndc_y * tan_half;
            let dir = (camera.forward + camera.right * dx + camera.up * dy).normalized();
            (camera.pos, dir)
        }
    }
}

fn ray_triangle_intersect(
    ray_origin: Vec3<f32>,
    ray_dir: Vec3<f32>,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
) -> Option<(f32, f32, f32)> {
    let a = Vec3::new(a[0], a[1], a[2]);
    let b = Vec3::new(b[0], b[1], b[2]);
    let c = Vec3::new(c[0], c[1], c[2]);
    let e1 = b - a;
    let e2 = c - a;
    let p = ray_dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-8 {
        return None;
    }
    let inv_det = 1.0 / det;
    let t_vec = ray_origin - a;
    let u = t_vec.dot(p) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = t_vec.cross(e1);
    let v = ray_dir.dot(q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv_det;
    if t <= 0.0 {
        return None;
    }
    Some((t, u, v))
}

/// Hash for light flickering
fn hash_u32(mut state: u32) -> u32 {
    state = (state ^ 61) ^ (state >> 16);
    state = state.wrapping_add(state << 3);
    state ^= state >> 4;
    state = state.wrapping_mul(0x27d4eb2d);
    state ^= state >> 15;
    state
}
