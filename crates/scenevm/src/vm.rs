// Non-empty dummy buffers for wgpu STORAGE bindings when a scene grid is empty.
const DUMMY_U32_1: [u32; 1] = [0];
const MATERIAL_TABLE_ID_COUNT: usize = 256;
const MATERIAL_TABLE_ROWS_PER_ID: usize = 3;
const DEFAULT_MATERIAL_TABLE_ROW_COUNT: usize =
    MATERIAL_TABLE_ID_COUNT * MATERIAL_TABLE_ROWS_PER_ID;
const DEFAULT_PAINT_COLOR_PIXEL: [u8; 4] = [0, 0, 0, 0];
const DEFAULT_PAINT_MATERIAL_PIXEL: [u8; 4] = [254, 0, 0, 0];

mod raster3d;

use crate::{
    Camera3D, CameraKind, Chunk, Light, LightType, Poly2D, Texture,
    atlas::{
        AtlasEntry, AtlasGpuTables, SharedAtlas, TileEmissiveSummary, default_material_frame,
        normalize_material_frame,
    },
    core::{
        Atom, GeoId, LayerBlendMode, OrganicBillboardInstance, OrganicBillboardSprite,
        PaintSurfaceBuffer, PaletteRemap2DMode, RenderMode, VMDebugStats,
    },
    dynamic::{DynamicKind, DynamicObject},
};
use bytemuck::{Pod, Zeroable};
use raster3d::record_raster3d_debug_timing;
pub use raster3d::{Line3DPod, Raster3DUniforms, Vert3DPod};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vert2DPod {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub tile_index: u32,
    pub tile_index2: u32,
    pub blend_factor: f32,
    pub _pad0: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TileBinPod {
    pub offset: u32,
    pub count: u32,
}

const RASTER3D_MAX_POINT_LIGHTS: usize = 8;
const EMISSIVE_SURFACE_LIGHT_BUDGET: usize = 6;
const EMISSIVE_SURFACE_POINT_LIGHTS_ENABLED: bool = false;
const EMISSIVE_SURFACE_IRRADIANCE_ENABLED: bool = true;
const EMISSIVE_SURFACE_CLUSTER_SIZE: f32 = 2.0;
const IRRADIANCE_GRID_MAX_SOURCES: usize = 64;
const IRRADIANCE_GRID_TARGET_CELL_SIZE: f32 = 3.0;
const IRRADIANCE_GRID_MAX_XZ: u32 = 18;
const IRRADIANCE_GRID_MAX_Y: u32 = 8;
const IRRADIANCE_OCCLUSION_MAX_TRIANGLES: usize = 1024;
const IRRADIANCE_OCCLUSION_BLOCKED_VISIBILITY: f32 = 0.18;

#[derive(Debug, Clone, Copy)]
struct RasterPointLight {
    position: Vec3<f32>,
    color: Vec3<f32>,
    intensity: f32,
    range: f32,
    score: f32,
}

#[derive(Debug, Clone, Default)]
struct EmissiveSurfaceLighting {
    point_lights: Vec<RasterPointLight>,
    broad_color: Vec3<f32>,
}

#[derive(Debug, Clone, Copy)]
struct IrradianceOccluder {
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    min: Vec3<f32>,
    max: Vec3<f32>,
}

fn palette_index_tile_uuid(index: u16) -> Uuid {
    Uuid::from_u128(0x50414C455454455F0000000000000000u128 | index as u128)
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

fn poly_uses_clamped_uv(poly: &crate::Poly3D) -> bool {
    poly.uvs
        .iter()
        .all(|uv| uv[0] >= -0.001 && uv[0] <= 1.001 && uv[1] >= -0.001 && uv[1] <= 1.001)
}

const TILE_INDEX_CLAMP_UV_FLAG_RUST: u32 = 0x4000_0000u32;
const TILE_INDEX_PARTICLE_FLAG_RUST: u32 = 0x0800_0000u32;

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

#[derive(Clone, Debug, Default)]
struct OrganicBillboardData {
    sprites: Vec<OrganicBillboardSprite>,
    instances: Vec<OrganicBillboardInstance>,
    dirty: bool,
}

#[allow(dead_code)]
const SCENE_BILLBOARD_CMD_WORDS: u32 =
    (std::mem::size_of::<DynamicBillboardPod>() / std::mem::size_of::<u32>()) as u32;

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
    pub sampler_raster: wgpu::Sampler,
    // --- Compute pipelines and uniforms (lazily created)
    pub compute2d_pipeline: Option<wgpu::ComputePipeline>,
    pub compute3d_pipeline: Option<wgpu::ComputePipeline>,
    pub compute_sdf_pipeline: Option<wgpu::ComputePipeline>,
    pub raster2d_pipeline: Option<wgpu::RenderPipeline>,
    pub raster3d_pipeline: Option<wgpu::RenderPipeline>,
    pub raster3d_alpha_pipeline: Option<wgpu::RenderPipeline>,
    pub raster3d_particle_pipeline: Option<wgpu::RenderPipeline>,
    pub raster3d_line_pipeline: Option<wgpu::RenderPipeline>,
    pub raster3d_organic_billboard_pipeline: Option<wgpu::RenderPipeline>,
    pub raster3d_shadow_pipeline: Option<wgpu::RenderPipeline>,
    pub raster3d_bloom_extract_pipeline: Option<wgpu::RenderPipeline>,
    pub raster3d_bloom_composite_pipeline: Option<wgpu::RenderPipeline>,
    pub u2d_buf: Option<wgpu::Buffer>,
    pub u3d_buf: Option<wgpu::Buffer>,
    pub u_sdf_buf: Option<wgpu::Buffer>,
    pub u_raster2d_buf: Option<wgpu::Buffer>,
    pub u_raster3d_buf: Option<wgpu::Buffer>,
    pub u2d_bgl: Option<wgpu::BindGroupLayout>,
    pub u3d_bgl: Option<wgpu::BindGroupLayout>,
    pub u_sdf_bgl: Option<wgpu::BindGroupLayout>,
    pub u_raster2d_bgl: Option<wgpu::BindGroupLayout>,
    pub u_raster3d_bgl: Option<wgpu::BindGroupLayout>,
    pub u_raster3d_shadow_bgl: Option<wgpu::BindGroupLayout>,
    pub u_raster3d_post_bgl: Option<wgpu::BindGroupLayout>,
    pub u2d_bg: Option<wgpu::BindGroup>,
    pub u3d_bg: Option<wgpu::BindGroup>,
    pub u_sdf_bg: Option<wgpu::BindGroup>,
    pub u_raster2d_bg: Option<wgpu::BindGroup>,
    pub u_raster3d_bg: Option<wgpu::BindGroup>,
    pub u_raster3d_shadow_bg: Option<wgpu::BindGroup>,
    pub v2d_ssbo: Option<wgpu::Buffer>,
    pub i2d_ssbo: Option<wgpu::Buffer>,
    pub v3d_ssbo: Option<wgpu::Buffer>,
    pub i3d_ssbo: Option<wgpu::Buffer>,
    pub v3d_ssbo_capacity: u64,
    pub i3d_ssbo_capacity: u64,
    pub i3d_raster: Option<wgpu::Buffer>,
    pub i3d_raster_count: u32,
    pub i3d_raster_capacity: u64,
    pub i3d_raster_opaque: Option<wgpu::Buffer>,
    pub i3d_raster_opaque_count: u32,
    pub i3d_raster_opaque_capacity: u64,
    pub i3d_raster_transparent: Option<wgpu::Buffer>,
    pub i3d_raster_transparent_count: u32,
    pub i3d_raster_transparent_capacity: u64,
    pub i3d_raster_particles: Option<wgpu::Buffer>,
    pub i3d_raster_particles_count: u32,
    pub i3d_raster_particles_capacity: u64,
    pub line3d_raster: Option<wgpu::Buffer>,
    pub line3d_raster_count: u32,
    pub line3d_raster_capacity: u64,
    pub shadow_sampler_compare: Option<wgpu::Sampler>,
    pub raster3d_shadow_tex: Option<wgpu::Texture>,
    pub raster3d_shadow_view: Option<wgpu::TextureView>,
    pub raster3d_shadow_res: u32,
    pub raster3d_scene_tex: Option<wgpu::Texture>,
    pub raster3d_scene_view: Option<wgpu::TextureView>,
    pub raster3d_bloom_tex: Option<wgpu::Texture>,
    pub raster3d_bloom_view: Option<wgpu::TextureView>,
    pub raster3d_bloom_size: (u32, u32),
    pub raster3d_msaa_color_tex: Option<wgpu::Texture>,
    pub raster3d_msaa_color_view: Option<wgpu::TextureView>,
    pub raster3d_depth_tex: Option<wgpu::Texture>,
    pub raster3d_depth_view: Option<wgpu::TextureView>,
    pub raster3d_fb_size: (u32, u32),
    pub raster3d_sample_count: u32,
    // --- Tiling
    pub tile_bins: Option<wgpu::Buffer>,
    pub tile_tris: Option<wgpu::Buffer>,
    pub tile_meta_ssbo: Option<wgpu::Buffer>,
    pub tile_frames_ssbo: Option<wgpu::Buffer>,
    // Scene-wide data (lights, billboards, ...)
    pub scene_data_ssbo: Option<wgpu::Buffer>,
    pub scene_data_ssbo_size: usize,
    pub organic_billboard_ssbo: Option<wgpu::Buffer>,
    pub organic_billboard_ssbo_size: usize,
    pub organic_billboard_count: u32,
    pub irradiance_grid_ssbo: Option<wgpu::Buffer>,
    pub irradiance_grid_ssbo_size: usize,
    pub material_table_ssbo: Option<wgpu::Buffer>,
    pub material_table_ssbo_size: usize,
    pub raster3d_paint_color_tex: Option<wgpu::Texture>,
    pub raster3d_paint_color_view: Option<wgpu::TextureView>,
    pub raster3d_paint_material_tex: Option<wgpu::Texture>,
    pub raster3d_paint_material_view: Option<wgpu::TextureView>,
    pub raster3d_paint_tex_size: (u32, u32),
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

#[derive(Clone, Debug)]
struct Raster3DPaintOverlayData {
    width: u32,
    height: u32,
    color_rgba: Vec<u8>,
    material_rgba: Vec<u8>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Raster2DUniforms {
    pub misc0: [f32; 4],                  // x=fb_w, y=fb_h, z=anim_counter, w=unused
    pub post_params: [f32; 4],            // x=enabled, y=tone mapper, z=exposure, w=gamma
    pub post_color_adjust: [f32; 4],      // x=saturation, y=luminance, z/w unused
    pub post_style0: [f32; 4],            // x=grit, y=posterize, z=palette_bias, w=shadow_lift
    pub post_style1: [f32; 4],            // x=edge_soften, yzw=reserved
    pub ambient_color_strength: [f32; 4], // rgb + strength
    pub sun_color_intensity: [f32; 4],    // rgb + intensity
    pub sun_dir_enabled: [f32; 4],        // xyz + enabled
    pub remap_params: [f32; 4],           // x=start, y=end, z=blend, w=mode
    pub mat2d_inv_c0: [f32; 4],
    pub mat2d_inv_c1: [f32; 4],
    pub mat2d_inv_c2: [f32; 4],
    pub palette: [[f32; 4]; 256],
}

const RASTER2D_UNIFORM_BYTES: usize = (12 * 16) + (256 * 16);
const _: [(); RASTER2D_UNIFORM_BYTES] = [(); std::mem::size_of::<Raster2DUniforms>()];
const _: [(); 0] = [(); std::mem::offset_of!(Raster2DUniforms, misc0)];
const _: [(); 16] = [(); std::mem::offset_of!(Raster2DUniforms, post_params)];
const _: [(); 32] = [(); std::mem::offset_of!(Raster2DUniforms, post_color_adjust)];
const _: [(); 48] = [(); std::mem::offset_of!(Raster2DUniforms, post_style0)];
const _: [(); 64] = [(); std::mem::offset_of!(Raster2DUniforms, post_style1)];
const _: [(); 80] = [(); std::mem::offset_of!(Raster2DUniforms, ambient_color_strength)];
const _: [(); 192] = [(); std::mem::offset_of!(Raster2DUniforms, palette)];

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

pub const SCENEVM_2D_RASTER_WGSL: &str = r#"
struct U2D {
  misc0: vec4<f32>,
  post_params: vec4<f32>,
  post_color_adjust: vec4<f32>,
  post_style0: vec4<f32>,
  post_style1: vec4<f32>,
  ambient_color_strength: vec4<f32>,
  sun_color_intensity: vec4<f32>,
  sun_dir_enabled: vec4<f32>,
  remap_params: vec4<f32>,
  mat2d_inv_c0: vec4<f32>,
  mat2d_inv_c1: vec4<f32>,
  mat2d_inv_c2: vec4<f32>,
  palette: array<vec4<f32>, 256>,
};
@group(0) @binding(0) var<uniform> UBO: U2D;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_smp: sampler;
@group(0) @binding(3) var atlas_mat_tex: texture_2d<f32>;

struct TileAnimMeta {
  first_frame: u32,
  frame_count: u32,
  _pad: vec2<u32>,
};
struct TileAnims { data: array<TileAnimMeta> };
struct TileFrame {
  ofs: vec2<f32>,
  scale: vec2<f32>,
};
struct TileFrames { data: array<TileFrame> };
@group(0) @binding(4) var<storage, read> tile_anims: TileAnims;
@group(0) @binding(5) var<storage, read> tile_frames: TileFrames;
struct SceneDataHeader {
  lights_offset_words: u32,
  lights_count: u32,
  billboard_cmd_offset_words: u32,
  billboard_cmd_count: u32,
  avatar_meta_offset_words: u32,
  avatar_meta_count: u32,
  avatar_pixel_offset_words: u32,
  data_word_count: u32,
};
struct SceneData { header: SceneDataHeader, data: array<u32> };
@group(0) @binding(6) var<storage, read> scene_data: SceneData;

struct VsIn {
  @location(0) pos: vec2<f32>,
  @location(1) uv: vec2<f32>,
  @location(2) tile_index: u32,
  @location(3) tile_index2: u32,
  @location(4) blend_factor: f32,
  @location(5) _pad0: u32,
};

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) @interpolate(flat) tile_index: u32,
  @location(2) @interpolate(flat) tile_index2: u32,
  @location(3) blend_factor: f32,
  @location(4) @interpolate(flat) kind: u32,
};

fn tile_frame(tile_index: u32, phase_start_counter: u32) -> TileFrame {
  let meta_len = arrayLength(&tile_anims.data);
  if (meta_len == 0u) {
    return TileFrame(vec2<f32>(0.0), vec2<f32>(0.0));
  }
  let idx = min(tile_index, meta_len - 1u);
  let anim = tile_anims.data[idx];
  let count = max(anim.frame_count, 1u);
  let frames_len = arrayLength(&tile_frames.data);
  if (frames_len == 0u) {
    return TileFrame(vec2<f32>(0.0), vec2<f32>(0.0));
  }
  let anim_counter = max(u32(max(UBO.misc0.z, 0.0)), phase_start_counter);
  let frame_offset = anim.first_frame + ((anim_counter - phase_start_counter) % count);
  let frame_idx = min(frame_offset, frames_len - 1u);
  return tile_frames.data[frame_idx];
}

fn atlas_uv(tile_index: u32, uv_obj: vec2<f32>, phase_start_counter: u32) -> vec2<f32> {
  let frame = tile_frame(tile_index, phase_start_counter);
  let uv_wrapped = fract(uv_obj);
  return frame.ofs + uv_wrapped * frame.scale;
}

fn sd_data_word(idx: u32) -> u32 {
  if (idx >= scene_data.header.data_word_count) {
    return 0u;
  }
  return scene_data.data[idx];
}

struct LightWGSL {
  header: vec4<u32>,
  position: vec4<f32>,
  color: vec4<f32>,
  params0: vec4<f32>,
  params1: vec4<f32>,
};

fn sd_vec4u(base_word: u32) -> vec4<u32> {
  return vec4<u32>(
    sd_data_word(base_word + 0u),
    sd_data_word(base_word + 1u),
    sd_data_word(base_word + 2u),
    sd_data_word(base_word + 3u)
  );
}

fn sd_vec4f(base_word: u32) -> vec4<f32> {
  return bitcast<vec4<f32>>(sd_vec4u(base_word));
}

fn sd_light(li: u32) -> LightWGSL {
  let base = scene_data.header.lights_offset_words + li * 20u;
  var light: LightWGSL;
  light.header = sd_vec4u(base + 0u);
  light.position = sd_vec4f(base + 4u);
  light.color = sd_vec4f(base + 8u);
  light.params0 = sd_vec4f(base + 12u);
  light.params1 = sd_vec4f(base + 16u);
  return light;
}

fn sd_unpack_rgba8(word: u32) -> vec4<f32> {
  let r = f32((word >> 0u) & 0xffu) * (1.0 / 255.0);
  let g = f32((word >> 8u) & 0xffu) * (1.0 / 255.0);
  let b = f32((word >> 16u) & 0xffu) * (1.0 / 255.0);
  let a = f32((word >> 24u) & 0xffu) * (1.0 / 255.0);
  return vec4<f32>(r, g, b, a);
}

fn sd_sample_avatar(avatar_index: u32, uv: vec2<f32>) -> vec4<f32> {
  if (avatar_index >= scene_data.header.avatar_meta_count) {
    return vec4<f32>(0.0);
  }
  let meta_base = scene_data.header.avatar_meta_offset_words + avatar_index * 4u;
  if (meta_base + 1u >= scene_data.header.data_word_count) {
    return vec4<f32>(0.0);
  }
  let offset_pixels = sd_data_word(meta_base + 0u);
  let size = sd_data_word(meta_base + 1u);
  if (size == 0u) {
    return vec4<f32>(0.0);
  }
  let u = clamp(uv.x, 0.0, 0.999999);
  let v = clamp(uv.y, 0.0, 0.999999);
  let x = u32(floor(u * f32(size)));
  let y = u32(floor(v * f32(size)));
  let idx = scene_data.header.avatar_pixel_offset_words + offset_pixels + y * size + x;
  if (idx >= scene_data.header.data_word_count) {
    return vec4<f32>(0.0);
  }
  return sd_unpack_rgba8(sd_data_word(idx));
}

fn hash12(p: vec2<f32>) -> f32 {
  return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453123);
}

fn apply_post(color: vec3<f32>, frag_pos: vec4<f32>) -> vec3<f32> {
  var c = max(color, vec3<f32>(0.0));
  let enabled = UBO.post_params.x > 0.5;
  let tone = u32(max(UBO.post_params.y, 0.0));
  let exposure = max(UBO.post_params.z, 0.0);
  let gamma = max(UBO.post_params.w, 0.001);
  let saturation = max(UBO.post_color_adjust.x, 0.0);
  let luminance = max(UBO.post_color_adjust.y, 0.0);
  let grit = clamp(UBO.post_style0.x, 0.0, 1.0);
  let posterize = clamp(UBO.post_style0.y, 0.0, 1.0);
  let palette_bias = clamp(UBO.post_style0.z, 0.0, 1.0);
  let shadow_lift = clamp(UBO.post_style0.w, 0.0, 1.0);
  let edge_soften = clamp(UBO.post_style1.x, 0.0, 1.0);
  if (enabled) {
    c = max(c * exposure, vec3<f32>(0.0));
    if (tone == 1u) {
      c = c / (c + vec3<f32>(1.0));
    } else if (tone == 2u) {
      let a = 2.51;
      let b = 0.03;
      let c2 = 2.43;
      let d = 0.59;
      let e = 0.14;
      c = clamp((c * (a * c + vec3<f32>(b))) / (c * (c2 * c + vec3<f32>(d)) + vec3<f32>(e)), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    c *= luminance;
    let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    c = mix(c, c + vec3<f32>(pow(max(1.0 - luma, 0.0), 2.0)) * 0.12, shadow_lift);
    let earth = vec3<f32>(luma) * vec3<f32>(1.07, 0.98, 0.82);
    c = mix(c, mix(c, earth, 0.45), palette_bias);
    let levels = mix(32.0, 7.0, posterize);
    c = mix(c, floor(c * levels + vec3<f32>(0.5)) / levels, posterize);
    let grain = hash12(floor(frag_pos.xy)) * 2.0 - 1.0;
    c = c + vec3<f32>(grain) * grit * 0.035;
    c = mix(c, vec3<f32>(dot(c, vec3<f32>(0.2126, 0.7152, 0.0722))), edge_soften * 0.10);
    let sat_luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    c = vec3<f32>(sat_luma) + (c - vec3<f32>(sat_luma)) * saturation;
  }
  c = pow(c, vec3<f32>(1.0 / gamma));
  return c;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
  return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}

fn semantic_material_rmoe(material_id: u32) -> vec4<f32> {
  let family = material_id / 4u;
  let finish = material_id & 3u;
  var roughness = 0.5;
  var metallic = 0.0;
  var opacity = 1.0;
  var emissive = 0.0;

  if (family == 1u) { roughness = 0.78; }
  else if (family == 2u) { roughness = 0.92; }
  else if (family == 3u) { roughness = 0.64; }
  else if (family == 4u) { roughness = 0.34; metallic = 0.9; }
  else if (family == 5u) { roughness = 0.06; opacity = 0.35; }
  else if (family == 6u) { roughness = 0.03; opacity = 0.55; }
  else if (family == 7u) { roughness = 0.02; metallic = 1.0; }
  else if (family == 8u) { roughness = 0.45; emissive = 1.0; }
  else if (family == 9u) { roughness = 0.86; }
  else if (family == 10u) { roughness = 0.45; }
  else if (family == 11u) { roughness = 0.78; }
  else if (family == 12u) { roughness = 0.56; }
  else if (family == 13u) { roughness = 0.66; }
  else if (family == 14u) { roughness = 0.42; }

  if (finish == 1u) { roughness = roughness + 0.15; }
  else if (finish == 2u) { roughness = roughness - 0.25; }
  else if (finish == 3u) { roughness = roughness - 0.35; }

  return vec4<f32>(clamp(roughness, 0.02, 1.0), metallic, opacity, emissive);
}

fn unpack_material_nibbles(m: vec4<f32>) -> vec4<f32> {
  let packed = u32(round(clamp(m.x, 0.0, 1.0) * 255.0)) |
    (u32(round(clamp(m.y, 0.0, 1.0) * 255.0)) << 8u);
  if ((packed & 0xFFu) == 254u) {
    return semantic_material_rmoe((packed >> 8u) & 0xFFu);
  }
  let roughness = f32(packed & 0xFu) / 15.0;
  let metallic = f32((packed >> 4u) & 0xFu) / 15.0;
  let opacity = f32((packed >> 8u) & 0xFu) / 15.0;
  let emissive = f32((packed >> 12u) & 0xFu) / 15.0;
  return vec4<f32>(roughness, metallic, opacity, emissive);
}

fn apply_scene_lighting(albedo: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
  let ambient = UBO.ambient_color_strength.xyz * UBO.ambient_color_strength.w;
  let sun_enabled = UBO.sun_dir_enabled.w > 0.5;
  let sun = select(
    vec3<f32>(0.0),
    UBO.sun_color_intensity.xyz * (UBO.sun_color_intensity.w * 0.35),
    sun_enabled
  );
  var lighting = max(ambient + sun, vec3<f32>(0.0));

  // Point lights from SceneVM scene data (same backing as Compute2D).
  for (var li: u32 = 0u; li < scene_data.header.lights_count; li = li + 1u) {
    let light = sd_light(li);
    if (light.header.y == 0u) { continue; }

    let to_light = light.position.xyz - world;
    let dist2 = max(dot(to_light, to_light), 1e-6);
    let dist = sqrt(dist2);

    let start_d = light.params0.z;
    let end_d = max(light.params0.w, start_d + 1e-3);
    let fall = clamp((end_d - dist) / max(end_d - start_d, 1e-3), 0.0, 1.0);

    let intensity = light.params0.x * light.params1.x;
    let atten = intensity * fall / dist2;
    lighting += light.color.xyz * atten;
  }

  lighting = clamp(lighting, vec3<f32>(0.0), vec3<f32>(1.0));
  return albedo * lighting;
}

fn palette_remap_range() -> vec2<u32> {
  let a = u32(max(UBO.remap_params.x, 0.0));
  let b = u32(max(UBO.remap_params.y, 0.0));
  return vec2<u32>(min(a, b), max(a, b));
}

fn palette_remap_blend() -> f32 {
  return clamp(UBO.remap_params.z, 0.0, 1.0);
}

fn palette_remap_mode() -> u32 {
  return u32(max(UBO.remap_params.w, 0.0));
}

fn palette_color(idx: u32) -> vec3<f32> {
  return UBO.palette[min(idx, 255u)].rgb;
}

fn bayer4_threshold(pix: vec2<f32>) -> f32 {
  let x = u32(abs(i32(floor(pix.x))) & 3);
  let y = u32(abs(i32(floor(pix.y))) & 3);
  let idx = y * 4u + x;
  let table = array<f32, 16>(
    0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0,
    12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0,
    3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0,
    15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0
  );
  return table[idx];
}

fn remap_color_luma_ramp(color: vec3<f32>, pix: vec2<f32>, dithered: bool) -> vec3<f32> {
  let range = palette_remap_range();
  let count = range.y - range.x + 1u;
  if (count <= 1u) {
    return palette_color(range.x);
  }
  let luma = clamp(dot(color, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.0, 1.0);
  let pos = luma * f32(count - 1u);
  let base = min(u32(floor(pos)), count - 1u);
  let next = min(base + 1u, count - 1u);
  let frac = fract(pos);
  let c0 = palette_color(range.x + base);
  let c1 = palette_color(range.x + next);
  if (dithered) {
    let choose_hi = frac > bayer4_threshold(pix);
    return select(c0, c1, choose_hi);
  }
  return mix(c0, c1, frac);
}

fn remap_color_nearest(color: vec3<f32>) -> vec3<f32> {
  let range = palette_remap_range();
  var best = palette_color(range.x);
  var best_d = dot(color - best, color - best);
  for (var idx: u32 = range.x + 1u; idx <= range.y; idx = idx + 1u) {
    let candidate = palette_color(idx);
    let dist2 = dot(color - candidate, color - candidate);
    if (dist2 < best_d) {
      best_d = dist2;
      best = candidate;
    }
  }
  return best;
}

fn apply_palette_remap_2d(color: vec3<f32>, pix: vec2<f32>) -> vec3<f32> {
  let blend = palette_remap_blend();
  let mode = palette_remap_mode();
  let range = palette_remap_range();
  if (blend <= 0.0001 || mode == 0u || range.x > range.y) {
    return color;
  }

  var remapped = color;
  switch mode {
    case 1u: {
      remapped = remap_color_luma_ramp(color, pix, false);
    }
    case 2u: {
      remapped = remap_color_nearest(color);
    }
    case 3u: {
      remapped = remap_color_luma_ramp(color, pix, true);
    }
    default: {}
  }
  return mix(color, remapped, blend);
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
  var out: VsOut;
  let fb_w = max(UBO.misc0.x, 1.0);
  let fb_h = max(UBO.misc0.y, 1.0);
  let x = (in.pos.x / fb_w) * 2.0 - 1.0;
  let y = (in.pos.y / fb_h) * -2.0 + 1.0;
  out.pos = vec4<f32>(x, y, 0.0, 1.0);
  out.uv = in.uv;
  out.tile_index = in.tile_index;
  out.tile_index2 = in.tile_index2;
  out.blend_factor = in.blend_factor;
  return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let Minv = mat3x3<f32>(UBO.mat2d_inv_c0.xyz, UBO.mat2d_inv_c1.xyz, UBO.mat2d_inv_c2.xyz);
  let world2 = Minv * vec3<f32>(in.pos.xy, 1.0);
  let world = vec3<f32>(world2.x, 0.0, world2.y);

  let is_avatar = (in.tile_index2 & 0x80000000u) != 0u;
  if (is_avatar) {
    let col_srgb = sd_sample_avatar(in.tile_index, in.uv);
    if (col_srgb.a <= 0.0) {
      discard;
    }
    let col = vec4<f32>(srgb_to_linear(col_srgb.rgb), col_srgb.a);
    return vec4<f32>(apply_post(apply_scene_lighting(col.rgb, world), in.pos), col.a);
  }

  let phase_start = select(0u, u32(max(in.blend_factor, 0.0)), in.blend_factor > 1.0);
  let uv0 = atlas_uv(in.tile_index, in.uv, phase_start);
  let col0_srgb = textureSampleLevel(atlas_tex, atlas_smp, uv0, 0.0);
  let col0 = vec4<f32>(srgb_to_linear(col0_srgb.rgb), col0_srgb.a);
  var col = col0;

  let blend = clamp(in.blend_factor, 0.0, 1.0);
  if (in.tile_index2 != in.tile_index && blend > 0.0) {
    let uv1 = atlas_uv(in.tile_index2, in.uv, phase_start);
    let col1_srgb = textureSampleLevel(atlas_tex, atlas_smp, uv1, 0.0);
    let col1 = vec4<f32>(srgb_to_linear(col1_srgb.rgb), col1_srgb.a);
    let overlay_a = clamp(blend * col1.a, 0.0, 1.0);
    let out_rgb = mix(col0.rgb, col1.rgb, overlay_a);
    let out_a = max(col0.a, overlay_a);
    col = vec4<f32>(out_rgb, out_a);
  }

  let mat_dims = vec2<f32>(textureDimensions(atlas_mat_tex, 0));
  let mat_px = vec2<i32>(clamp(floor(uv0 * mat_dims), vec2<f32>(0.0), mat_dims - vec2<f32>(1.0)));
  let mats_raw = textureLoad(atlas_mat_tex, mat_px, 0);
  let mats = unpack_material_nibbles(mats_raw);
  let opacity = mats.z;
  let emission = mats.w;
  let remapped_rgb = apply_palette_remap_2d(col.rgb, in.pos.xy);
  let rgb = apply_scene_lighting(remapped_rgb, world) * (1.0 + emission);
  let a = col.a * opacity;
  if (a <= 0.0) {
    discard;
  }
  return vec4<f32>(apply_post(rgb, in.pos), a);
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
const _: [(); 0] = [(); std::mem::size_of::<Compute2DUniforms>() % 16];

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
const _: [(); 0] = [(); std::mem::size_of::<Compute3DUniforms>() % 16];

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
const _: [(); 0] = [(); std::mem::size_of::<ComputeSdfUniforms>() % 16];

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

pub const SCENEVM_3D_RASTER_WGSL: &str = r#"
struct U {
    cam_pos: vec4<f32>,
    cam_fwd: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    sun_color_intensity: vec4<f32>,
    sun_dir_enabled: vec4<f32>,
    ambient_color_strength: vec4<f32>,
    sky_color: vec4<f32>,
    fog_color_density: vec4<f32>,
    shadow_light_right: vec4<f32>,
    shadow_light_up: vec4<f32>,
    shadow_light_fwd: vec4<f32>,
    shadow_light_center: vec4<f32>,
    shadow_light_extents: vec4<f32>,
    shadow_params: vec4<f32>,
    render_params: vec4<f32>,
    point_light_pos_intensity: array<vec4<f32>, 8>,
    point_light_color_range: array<vec4<f32>, 8>,
    point_light_count_pad: vec4<u32>,
    _pad_lights: vec4<u32>,
    fb_size: vec2<f32>,
    cam_vfov_deg: f32,
    cam_ortho_half_h: f32,
    cam_near: f32,
    cam_far: f32,
    cam_kind: u32,
    anim_counter: u32,
    _pad0: vec2<u32>,
    _pad_post_pre: vec2<u32>,
    post_params: vec4<f32>,
    post_color_adjust: vec4<f32>,
    post_style0: vec4<f32>,
    post_style1: vec4<f32>,
    avatar_highlight_params: vec4<f32>,
    _pad_tail: vec4<u32>,
    palette: array<vec4<f32>, 256>,
    palette_tile_indices: array<vec4<u32>, 64>,
    organic_params: vec4<u32>,
};
@group(0) @binding(0) var<uniform> UBO: U;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_smp: sampler;
@group(0) @binding(5) var shadow_tex: texture_depth_2d;
@group(0) @binding(6) var shadow_smp: sampler_comparison;
@group(0) @binding(7) var atlas_mat_tex: texture_2d<f32>;
struct SceneDataBuf { data: array<u32> };
@group(0) @binding(8) var<storage, read> scene_data: SceneDataBuf;
struct IrradianceGridBuf { data: array<vec4<f32>> };
@group(0) @binding(11) var<storage, read> irradiance_grid: IrradianceGridBuf;
struct MaterialTableBuf { data: array<vec4<f32>> };
@group(0) @binding(12) var<storage, read> material_table: MaterialTableBuf;
@group(0) @binding(13) var paint_color_tex: texture_2d<f32>;
@group(0) @binding(14) var paint_material_tex: texture_2d<f32>;

struct TileAnimMeta { first_frame: u32, frame_count: u32, _pad0: u32, _pad1: u32 };
struct TileFrame { ofs: vec2<f32>, scale: vec2<f32> };
struct TileAnimMetaBuf { data: array<TileAnimMeta> };
struct TileFrameBuf { data: array<TileFrame> };
@group(0) @binding(3) var<storage, read> tile_meta: TileAnimMetaBuf;
@group(0) @binding(4) var<storage, read> tile_frames: TileFrameBuf;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) _pad0: f32,
    @location(2) uv: vec2<f32>,
    @location(3) _pad1: vec2<f32>,
    @location(4) tile_index: u32,
    @location(5) tile_index2: u32,
    @location(6) blend_factor: f32,
    @location(7) opacity: f32,
    @location(8) normal: vec3<f32>,
    @location(9) _pad2: f32,
    @location(13) surface_noise: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(3) @interpolate(flat) tile_index: u32,
    @location(4) @interpolate(flat) tile_index2: u32,
    @location(5) blend_factor: f32,
    @location(6) opacity: f32,
    @location(7) normal: vec3<f32>,
    @location(12) world_pos: vec3<f32>,
    @location(13) surface_noise: vec4<f32>,
};

struct VsShadowOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) tile_index: u32,
    @location(2) @interpolate(flat) tile_index2: u32,
    @location(3) blend_factor: f32,
    @location(4) opacity: f32,
};

const TILE_INDEX_AVATAR_FLAG: u32 = 0x80000000u;
const TILE_INDEX_CLAMP_UV_FLAG: u32 = 0x40000000u;
const TILE_INDEX_BILLBOARD_FLAG: u32 = 0x20000000u;
const TILE_INDEX_BLOCK_SUN_FLAG: u32 = 0x10000000u;
const TILE_INDEX_PARTICLE_FLAG: u32 = 0x08000000u;
const TILE_INDEX_FLAGS_MASK: u32 =
    TILE_INDEX_AVATAR_FLAG
    | TILE_INDEX_CLAMP_UV_FLAG
    | TILE_INDEX_BILLBOARD_FLAG
    | TILE_INDEX_BLOCK_SUN_FLAG
    | TILE_INDEX_PARTICLE_FLAG;

fn camera_to_clip(world_pos: vec3<f32>) -> vec4<f32> {
    let rel = world_pos - UBO.cam_pos.xyz;
    let cx = dot(rel, UBO.cam_right.xyz);
    let cy = dot(rel, UBO.cam_up.xyz);
    let cz = dot(rel, UBO.cam_fwd.xyz);

    let near_z = max(UBO.cam_near, 0.0001);
    let far_z = max(UBO.cam_far, near_z + 0.0001);
    let aspect = max(UBO.fb_size.x / max(UBO.fb_size.y, 1.0), 0.0001);

    if (UBO.cam_kind == 0u) {
        let depth = clamp((cz - near_z) / (far_z - near_z), 0.0, 1.0);
        let half_h = max(UBO.cam_ortho_half_h, 0.0001);
        let half_w = max(half_h * aspect, 0.0001);
        return vec4<f32>(cx / half_w, cy / half_h, depth, 1.0);
    }

    // Keep camera-space sign so behind-camera vertices clip correctly.
    var z = cz;
    if (abs(z) < 0.0001) {
        z = select(-0.0001, 0.0001, z >= 0.0);
    }
    let f = 1.0 / tan(radians(max(UBO.cam_vfov_deg, 1.0)) * 0.5);
    // Perspective clip-space (wgpu): ndc = clip / w, z in [0,1].
    // Build clip.z/clip.w so depth behaves like a standard projection matrix.
    let a = far_z / (far_z - near_z);
    let b = (-near_z * far_z) / (far_z - near_z);
    return vec4<f32>(cx * (f / aspect), cy * f, a * z + b, z);
}

fn tile_frame(tile_idx: u32, phase_start_counter: u32) -> TileFrame {
    if (arrayLength(&tile_meta.data) == 0u || arrayLength(&tile_frames.data) == 0u) {
        return TileFrame(vec2<f32>(0.0), vec2<f32>(0.0));
    }
    let meta_idx = min(tile_idx, arrayLength(&tile_meta.data) - 1u);
    let tile_anim = tile_meta.data[meta_idx];
    if (tile_anim.frame_count == 0u) {
        return TileFrame(vec2<f32>(0.0), vec2<f32>(0.0));
    }
    var frame_offset: u32 = 0u;
    if (tile_anim.frame_count > 1u) {
        let anim_counter = max(UBO.anim_counter, phase_start_counter);
        frame_offset = (anim_counter - phase_start_counter) % tile_anim.frame_count;
    }
    let frame_idx = min(tile_anim.first_frame + frame_offset, arrayLength(&tile_frames.data) - 1u);
    return tile_frames.data[frame_idx];
}

fn sample_tile(tile_idx: u32, uv: vec2<f32>, clamp_uv: bool, phase_start_counter: u32) -> vec4<f32> {
    let frame = tile_frame(tile_idx, phase_start_counter);
    var uv_wrapped = fract(uv);
    if (clamp_uv) {
        uv_wrapped = clamp(uv, vec2<f32>(0.0), vec2<f32>(0.9999));
    }
    var atlas_uv = frame.ofs + uv_wrapped * frame.scale;
    // Use gradients from the non-wrapped UVs to avoid mip shimmer on repeating tiles.
    // Slightly bias iso camera toward coarser mips to reduce shimmering on dense tile patterns.
    let lod_bias = select(1.0, 1.8, UBO.cam_kind == 0u);
    let atlas_ddx = dpdx(uv) * frame.scale * lod_bias;
    let atlas_ddy = dpdy(uv) * frame.scale * lod_bias;
    let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0));
    let ddx_tex = atlas_ddx * atlas_dims;
    let ddy_tex = atlas_ddy * atlas_dims;
    let rho2 = max(dot(ddx_tex, ddx_tex), dot(ddy_tex, ddy_tex));
    let lod = max(0.0, 0.5 * log2(max(rho2, 1e-8)));
    // Atlas mipmaps are globally generated; clamp max lod to avoid distant bleed/grid artifacts.
    let lod_clamped = min(lod, 2.5);
    let pad_uv = vec2<f32>(0.5) / max(atlas_dims, vec2<f32>(1.0));
    let uv_min = frame.ofs + pad_uv;
    let uv_max = frame.ofs + frame.scale - pad_uv;
    atlas_uv = clamp(atlas_uv, uv_min, uv_max);
    return textureSampleLevel(atlas_tex, atlas_smp, atlas_uv, lod_clamped);
}

fn sample_tile_lod0(tile_idx: u32, uv: vec2<f32>, clamp_uv: bool, phase_start_counter: u32) -> vec4<f32> {
    let frame = tile_frame(tile_idx, phase_start_counter);
    var uv_wrapped = fract(uv);
    if (clamp_uv) {
        uv_wrapped = clamp(uv, vec2<f32>(0.0), vec2<f32>(0.9999));
    }
    var atlas_uv = frame.ofs + uv_wrapped * frame.scale;
    let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0));
    let pad_uv = vec2<f32>(0.5) / max(atlas_dims, vec2<f32>(1.0));
    let uv_min = frame.ofs + pad_uv;
    let uv_max = frame.ofs + frame.scale - pad_uv;
    atlas_uv = clamp(atlas_uv, uv_min, uv_max);
    return textureSampleLevel(atlas_tex, atlas_smp, atlas_uv, 0.0);
}

fn sample_tile_material(tile_idx: u32, uv: vec2<f32>, clamp_uv: bool, phase_start_counter: u32) -> vec4<f32> {
    let frame = tile_frame(tile_idx, phase_start_counter);
    var uv_wrapped = fract(uv);
    if (clamp_uv) {
        uv_wrapped = clamp(uv, vec2<f32>(0.0), vec2<f32>(0.9999));
    }
    var atlas_uv = frame.ofs + uv_wrapped * frame.scale;
    let atlas_dims = vec2<f32>(textureDimensions(atlas_mat_tex, 0));
    let pad_uv = vec2<f32>(0.5) / max(atlas_dims, vec2<f32>(1.0));
    let uv_min = frame.ofs + pad_uv;
    let uv_max = frame.ofs + frame.scale - pad_uv;
    atlas_uv = clamp(atlas_uv, uv_min, uv_max);
    // Material data stores ids and normals; it must not be filtered like color.
    let texel = vec2<i32>(clamp(floor(atlas_uv * atlas_dims), vec2<f32>(0.0), atlas_dims - vec2<f32>(1.0)));
    return textureLoad(atlas_mat_tex, texel, 0);
}

fn palette_tile_index(idx: u32) -> u32 {
    let clamped = min(idx, 255u);
    let pack = UBO.palette_tile_indices[clamped / 4u];
    let lane = clamped % 4u;
    if (lane == 0u) { return pack.x; }
    if (lane == 1u) { return pack.y; }
    if (lane == 2u) { return pack.z; }
    return pack.w;
}

fn sample_avatar(meta_idx: u32, uv: vec2<f32>) -> vec4<f32> {
    if (arrayLength(&scene_data.data) < 8u) {
        return vec4<f32>(0.0);
    }
    let avatar_meta_offset_words = scene_data.data[4u];
    let avatar_meta_count = scene_data.data[5u];
    let avatar_pixel_offset_words = scene_data.data[6u];
    if (meta_idx >= avatar_meta_count) {
        return vec4<f32>(0.0);
    }
    let header_words = 8u;
    let meta_word_offset = header_words + avatar_meta_offset_words + meta_idx * 4u;
    if (meta_word_offset + 1u >= arrayLength(&scene_data.data)) {
        return vec4<f32>(0.0);
    }
    let offset_pixels = scene_data.data[meta_word_offset];
    let size = scene_data.data[meta_word_offset + 1u];
    if (size == 0u) {
        return vec4<f32>(0.0);
    }
    let sizef = f32(size);
    let suv = clamp(uv, vec2<f32>(0.0), vec2<f32>(0.9999));
    let px = min(u32(floor(suv.x * sizef)), size - 1u);
    let py = min(u32(floor(suv.y * sizef)), size - 1u);
    let pixel_word = header_words + avatar_pixel_offset_words + offset_pixels + py * size + px;
    if (pixel_word >= arrayLength(&scene_data.data)) {
        return vec4<f32>(0.0);
    }
    let packed = scene_data.data[pixel_word];
    let r = f32(packed & 0xFFu) / 255.0;
    let g = f32((packed >> 8u) & 0xFFu) / 255.0;
    let b = f32((packed >> 16u) & 0xFFu) / 255.0;
    let a = f32((packed >> 24u) & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

fn unpack_material_nibbles(m: vec4<f32>) -> vec4<f32> {
    let packed = u32(round(m.x * 255.0)) | (u32(round(m.y * 255.0)) << 8u) |
                 (u32(round(m.z * 255.0)) << 16u) | (u32(round(m.w * 255.0)) << 24u);
    if ((packed & 0xFFu) == 254u) {
        return semantic_material_rmoe((packed >> 8u) & 0xFFu);
    }
    let bits = packed & 0xFFFFu;
    let roughness = f32(bits & 0xFu) / 15.0;
    let metallic = f32((bits >> 4u) & 0xFu) / 15.0;
    let opacity = f32((bits >> 8u) & 0xFu) / 15.0;
    let emissive = f32((bits >> 12u) & 0xFu) / 15.0;
    return vec4<f32>(roughness, metallic, opacity, emissive);
}

fn unpack_material_id(m: vec4<f32>) -> u32 {
    let packed = u32(round(m.x * 255.0)) | (u32(round(m.y * 255.0)) << 8u) |
                 (u32(round(m.z * 255.0)) << 16u) | (u32(round(m.w * 255.0)) << 24u);
    if ((packed & 0xFFu) == 254u) {
        return (packed >> 8u) & 0xFFu;
    }
    return 0u;
}

fn semantic_material_row(material_id: u32, row: u32, fallback: vec4<f32>) -> vec4<f32> {
    let clamped_id = min(material_id, 255u);
    let index = clamped_id * 3u + row;
    if (index >= arrayLength(&material_table.data)) {
        return fallback;
    }
    return material_table.data[index];
}

fn semantic_material_rmoe(material_id: u32) -> vec4<f32> {
    return semantic_material_row(material_id, 0u, vec4<f32>(0.5, 0.0, 1.0, 0.0));
}

fn semantic_material_traits0(material_id: u32) -> vec4<f32> {
    return semantic_material_row(material_id, 1u, vec4<f32>(0.0, 0.0, 0.0, 0.12));
}

fn semantic_material_traits1(material_id: u32) -> vec4<f32> {
    return semantic_material_row(material_id, 2u, vec4<f32>(0.08, 0.0, 0.0, 0.0));
}

fn sample_paint_overlay(frag_pos: vec4<f32>) -> vec4<f32> {
    let dims = textureDimensions(paint_color_tex, 0);
    if (dims.x == 0u || dims.y == 0u) {
        return vec4<f32>(0.0);
    }
    let px = vec2<i32>(
        clamp(i32(floor(frag_pos.x)), 0, i32(dims.x) - 1),
        clamp(i32(floor(frag_pos.y)), 0, i32(dims.y) - 1)
    );
    return textureLoad(paint_color_tex, px, 0);
}

fn sample_paint_material_id(frag_pos: vec4<f32>) -> u32 {
    let dims = textureDimensions(paint_material_tex, 0);
    if (dims.x == 0u || dims.y == 0u) {
        return 0u;
    }
    let px = vec2<i32>(
        clamp(i32(floor(frag_pos.x)), 0, i32(dims.x) - 1),
        clamp(i32(floor(frag_pos.y)), 0, i32(dims.y) - 1)
    );
    return unpack_material_id(textureLoad(paint_material_tex, px, 0));
}

fn unpack_material_normal_ts(m: vec4<f32>) -> vec3<f32> {
    let packed = u32(round(m.x * 255.0)) | (u32(round(m.y * 255.0)) << 8u) |
                 (u32(round(m.z * 255.0)) << 16u) | (u32(round(m.w * 255.0)) << 24u);
    let norm_bits = (packed >> 16u) & 0xFFFFu;
    let nx = (f32(norm_bits & 0xFFu) / 255.0) * 2.0 - 1.0;
    let ny = (f32((norm_bits >> 8u) & 0xFFu) / 255.0) * 2.0 - 1.0;
    let nz = sqrt(max(0.0, 1.0 - nx * nx - ny * ny));
    return vec3<f32>(nx, ny, nz);
}

fn distribution_ggx(NdotH: f32, roughness: f32) -> f32 {
    let a = max(roughness * roughness, 0.04);
    let a2 = a * a;
    let nh2 = max(NdotH * NdotH, 0.0);
    let denom = nh2 * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom + 1e-6);
}

fn geometry_schlick_ggx(NdotX: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return NdotX / (NdotX * (1.0 - k) + k + 1e-6);
}

fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn fresnel_schlick_roughness(cos_theta: f32, F0: vec3<f32>, roughness: f32) -> vec3<f32> {
    let one_minus_cos5 = pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
    return F0 + (max(vec3<f32>(1.0 - roughness), F0) - F0) * one_minus_cos5;
}

fn hash13(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453123);
}

fn surface_noise_value(pos: vec3<f32>, seed: f32) -> f32 {
    let cell = floor(pos);
    let frac_pos = fract(pos);
    let f = frac_pos * frac_pos * (vec3<f32>(3.0) - 2.0 * frac_pos);
    let n000 = hash13(cell + vec3<f32>(0.0, 0.0, 0.0) + seed);
    let n100 = hash13(cell + vec3<f32>(1.0, 0.0, 0.0) + seed);
    let n010 = hash13(cell + vec3<f32>(0.0, 1.0, 0.0) + seed);
    let n110 = hash13(cell + vec3<f32>(1.0, 1.0, 0.0) + seed);
    let n001 = hash13(cell + vec3<f32>(0.0, 0.0, 1.0) + seed);
    let n101 = hash13(cell + vec3<f32>(1.0, 0.0, 1.0) + seed);
    let n011 = hash13(cell + vec3<f32>(0.0, 1.0, 1.0) + seed);
    let n111 = hash13(cell + vec3<f32>(1.0, 1.0, 1.0) + seed);
    let nx00 = mix(n000, n100, f.x);
    let nx10 = mix(n010, n110, f.x);
    let nx01 = mix(n001, n101, f.x);
    let nx11 = mix(n011, n111, f.x);
    let nxy0 = mix(nx00, nx10, f.y);
    let nxy1 = mix(nx01, nx11, f.y);
    return mix(nxy0, nxy1, f.z);
}

fn surface_noise_blend(params: vec4<f32>, world_pos: vec3<f32>) -> f32 {
    if (params.w <= 0.5 || params.y <= 0.0001) {
        return 0.0;
    }
    let scale = max(params.x, 0.0001);
    let amount = clamp(params.y, 0.0, 1.0);
    let seed = vec3<f32>(params.z * 0.173, params.z * 0.371, params.z * 0.619);
    return surface_noise_value(world_pos * scale + seed, params.z * 0.013) * amount;
}

fn bayer4_threshold(x: u32, y: u32) -> f32 {
    let xi = x & 3u;
    let yi = y & 3u;
    let idx = yi * 4u + xi;
    let v = array<u32, 16>(
        0u, 8u, 2u, 10u,
        12u, 4u, 14u, 6u,
        3u, 11u, 1u, 9u,
        15u, 7u, 13u, 5u
    );
    return (f32(v[idx]) + 0.5) / 16.0;
}

fn irradiance_grid_index(x: u32, y: u32, z: u32, dims: vec3<u32>) -> u32 {
    return 3u + x + y * dims.x + z * dims.x * dims.y;
}

fn irradiance_grid_sample_at(x: u32, y: u32, z: u32, dims: vec3<u32>) -> vec3<f32> {
    let idx = irradiance_grid_index(
        min(x, dims.x - 1u),
        min(y, dims.y - 1u),
        min(z, dims.z - 1u),
        dims
    );
    if (idx >= arrayLength(&irradiance_grid.data)) {
        return vec3<f32>(0.0);
    }
    return max(irradiance_grid.data[idx].xyz, vec3<f32>(0.0));
}

fn sample_irradiance_grid_raw(world_pos: vec3<f32>) -> vec3<f32> {
    if (arrayLength(&irradiance_grid.data) < 4u || irradiance_grid.data[0u].w < 0.5) {
        return vec3<f32>(0.0);
    }
    let dims = vec3<u32>(
        max(u32(round(irradiance_grid.data[2u].x)), 2u),
        max(u32(round(irradiance_grid.data[2u].y)), 2u),
        max(u32(round(irradiance_grid.data[2u].z)), 2u)
    );
    let cell = max(irradiance_grid.data[1u].xyz, vec3<f32>(0.001));
    let local = clamp(
        (world_pos - irradiance_grid.data[0u].xyz) / cell,
        vec3<f32>(0.0),
        vec3<f32>(f32(dims.x - 1u), f32(dims.y - 1u), f32(dims.z - 1u))
    );
    let base_f = floor(local);
    let f = local - base_f;
    let base = vec3<u32>(
        min(u32(base_f.x), dims.x - 2u),
        min(u32(base_f.y), dims.y - 2u),
        min(u32(base_f.z), dims.z - 2u)
    );

    let c000 = irradiance_grid_sample_at(base.x, base.y, base.z, dims);
    let c100 = irradiance_grid_sample_at(base.x + 1u, base.y, base.z, dims);
    let c010 = irradiance_grid_sample_at(base.x, base.y + 1u, base.z, dims);
    let c110 = irradiance_grid_sample_at(base.x + 1u, base.y + 1u, base.z, dims);
    let c001 = irradiance_grid_sample_at(base.x, base.y, base.z + 1u, dims);
    let c101 = irradiance_grid_sample_at(base.x + 1u, base.y, base.z + 1u, dims);
    let c011 = irradiance_grid_sample_at(base.x, base.y + 1u, base.z + 1u, dims);
    let c111 = irradiance_grid_sample_at(base.x + 1u, base.y + 1u, base.z + 1u, dims);

    let cx00 = mix(c000, c100, f.x);
    let cx10 = mix(c010, c110, f.x);
    let cx01 = mix(c001, c101, f.x);
    let cx11 = mix(c011, c111, f.x);
    let cxy0 = mix(cx00, cx10, f.y);
    let cxy1 = mix(cx01, cx11, f.y);
    return mix(cxy0, cxy1, f.z);
}

fn sample_irradiance_grid(world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    if (arrayLength(&irradiance_grid.data) < 4u || irradiance_grid.data[0u].w < 0.5) {
        return vec3<f32>(0.0);
    }
    let cell = max(irradiance_grid.data[1u].xyz, vec3<f32>(0.001));
    let probe_step = max(max(cell.x, cell.y), cell.z) * 0.42;
    let n = normalize(normal);
    let center = sample_irradiance_grid_raw(world_pos);
    let front = sample_irradiance_grid_raw(world_pos + n * probe_step);
    let up_bias = 0.82 + clamp(n.y, -1.0, 1.0) * 0.10;
    return max(mix(center, front, 0.62) * up_bias, vec3<f32>(0.0));
}

fn sample_shadow(world_pos: vec3<f32>, NdotL: f32) -> f32 {
    if (UBO.sun_dir_enabled.w <= 0.5 || UBO.shadow_params.x <= 0.5) {
        return 1.0;
    }
    let rel = world_pos - UBO.shadow_light_center.xyz;
    let lx = dot(rel, UBO.shadow_light_right.xyz);
    let ly = dot(rel, UBO.shadow_light_up.xyz);
    let lz = dot(rel, UBO.shadow_light_fwd.xyz);
    let half_w = max(UBO.shadow_light_extents.x, 0.0001);
    let half_h = max(UBO.shadow_light_extents.y, 0.0001);
    let near_z = UBO.shadow_light_extents.z;
    let far_z = max(UBO.shadow_light_extents.w, near_z + 0.0001);
    let nx = lx / half_w;
    let ny = ly / half_h;
    let raw_depth = (lz - near_z) / (far_z - near_z);
    let depth = clamp(raw_depth, 0.0, 1.0);
    // Render target space is Y-down for texture sampling; flip Y from NDC.
    let uv = vec2<f32>(nx * 0.5 + 0.5, 1.0 - (ny * 0.5 + 0.5));
    // Keep a border margin so PCF taps never sample outside the shadow map.
    let shadow_dims = vec2<f32>(textureDimensions(shadow_tex));
    let texel = 1.0 / max(shadow_dims, vec2<f32>(1.0));
    let margin = texel * 2.0;
    let max_uv = vec2<f32>(1.0, 1.0) - margin;
    let clamped_uv = clamp(uv, margin, max_uv);
    let valid_depth = raw_depth > 0.0001 && raw_depth < 0.9999;
    let valid_uv = uv.x > margin.x && uv.x < max_uv.x && uv.y > margin.y && uv.y < max_uv.y;
    let sample_valid = valid_depth && valid_uv;
    // Slope- and depth-scaled bias; deliberately soft to avoid hard acne seams in iso.
    let slope_bias = select(0.007, 0.012, UBO.cam_kind == 2u) * (1.0 - NdotL);
    let depth_bias = depth * 0.0020;
    let bias = max(max(UBO.shadow_params.w, 0.0010), slope_bias + depth_bias);
    let ref_depth = depth - bias;
    // Wider PCF keeps sun shadows readable but less PBR-crisp.
    var occ = 0.0;
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv, ref_depth) * 2.0;
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>(-1.5,  0.0) * texel, ref_depth);
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>( 1.5,  0.0) * texel, ref_depth);
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>( 0.0, -1.5) * texel, ref_depth);
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>( 0.0,  1.5) * texel, ref_depth);
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>(-1.5, -1.5) * texel, ref_depth);
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>( 1.5, -1.5) * texel, ref_depth);
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>(-1.5,  1.5) * texel, ref_depth);
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>( 1.5,  1.5) * texel, ref_depth);
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>(-2.75,  0.0) * texel, ref_depth) * 0.5;
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>( 2.75,  0.0) * texel, ref_depth) * 0.5;
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>( 0.0, -2.75) * texel, ref_depth) * 0.5;
    occ += textureSampleCompare(shadow_tex, shadow_smp, clamped_uv + vec2<f32>( 0.0,  2.75) * texel, ref_depth) * 0.5;
    let shadow = occ * (1.0 / 12.0);
    let edge_dist = min(min(uv.x - margin.x, max_uv.x - uv.x), min(uv.y - margin.y, max_uv.y - uv.y));
    let edge_fade = smoothstep(0.0, max(texel.x, texel.y) * 8.0, edge_dist);
    return select(1.0, mix(1.0, shadow, edge_fade), sample_valid);
}

fn hash12(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453123);
}

fn apply_post(color_linear: vec3<f32>, frag_pos: vec4<f32>) -> vec3<f32> {
    let post_enabled = UBO.post_params.x > 0.5;
    let tone_mapper = u32(max(UBO.post_params.y, 0.0));
    let first_person_post = UBO.cam_kind == 2u;
    let post_style_strength = select(1.0, 0.45, first_person_post);
    let exposure = max(UBO.post_params.z, 0.0);
    let gamma = max(UBO.post_params.w, 0.001);
    let grit = clamp(UBO.post_style0.x, 0.0, 1.0);
    let posterize = clamp(UBO.post_style0.y, 0.0, 1.0);
    let palette_bias = clamp(UBO.post_style0.z, 0.0, 1.0);
    let shadow_lift = clamp(UBO.post_style0.w, 0.0, 1.0);
    let edge_soften = clamp(UBO.post_style1.x, 0.0, 1.0);
    var c = max(color_linear, vec3<f32>(0.0));
    if (UBO.post_color_adjust.z > 0.5) {
        return c;
    }

    if (post_enabled) {
        c = c * exposure;
        if (tone_mapper == 1u) {
            // Reinhard
            c = c / (c + vec3<f32>(1.0));
        } else if (tone_mapper == 2u) {
            // ACES fit
            let a = 2.51;
            let b = 0.03;
            let d = 0.59;
            let e = 0.14;
            c = clamp((c * (a * c + b)) / (c * (2.43 * c + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
        }

        let luminance = max(UBO.post_color_adjust.y, 0.0);
        c = c * luminance;
        let highlight = max(c - vec3<f32>(0.62), vec3<f32>(0.0));
        let highlight_luma = dot(highlight, vec3<f32>(0.2126, 0.7152, 0.0722));
        let bloom_tint = mix(vec3<f32>(1.0), max(UBO.post_style1.yzw, vec3<f32>(0.0)), 0.28);
        c = c + highlight * bloom_tint * (0.055 + highlight_luma * 0.070) * post_style_strength;
        let saturation = max(UBO.post_color_adjust.x, 0.0);
        let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
        c = mix(c, c + vec3<f32>(pow(max(1.0 - luma, 0.0), 2.0)) * 0.12, shadow_lift);
        let earth = vec3<f32>(luma) * vec3<f32>(1.07, 0.98, 0.82);
        c = mix(c, mix(c, earth, 0.45), palette_bias);
        let levels = mix(32.0, 7.0, posterize);
        c = mix(c, floor(c * levels + vec3<f32>(0.5)) / levels, posterize);
        let grain = hash12(floor(frag_pos.xy)) * 2.0 - 1.0;
        let grain_amount = select(0.026, 0.014, first_person_post);
        c = c + vec3<f32>(grain) * grit * grain_amount;
        let paper = hash12(floor(frag_pos.xy * 0.5) + vec2<f32>(17.0, 3.0)) * 2.0 - 1.0;
        let nomad_luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
        let shadow_tone = 1.0 - smoothstep(0.14, 0.62, nomad_luma);
        c = mix(c, c * vec3<f32>(1.035, 0.995, 0.925), shadow_tone * 0.075 * post_style_strength);
        c = c + vec3<f32>(paper) * 0.0035 * post_style_strength;
        c = mix(c, vec3<f32>(dot(c, vec3<f32>(0.2126, 0.7152, 0.0722))), edge_soften * 0.10);
        let sat_luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
        c = mix(vec3<f32>(sat_luma), c, saturation);
    }

    c = max(c, vec3<f32>(0.0));

    return pow(c, vec3<f32>(1.0 / gamma));
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let is_particle = (in.tile_index2 & TILE_INDEX_PARTICLE_FLAG) != 0u;
    out.pos = camera_to_clip(in.pos);
    out.uv = in.uv;
    out.tile_index = in.tile_index;
    out.tile_index2 = in.tile_index2;
    out.blend_factor = in.blend_factor;
    out.opacity = clamp(in.opacity, 0.0, 1.0);
    out.normal = select(normalize(in.normal), max(in.normal, vec3<f32>(0.0)), is_particle);
    out.world_pos = in.pos;
    out.surface_noise = in.surface_noise;
    return out;
}

@vertex
fn vs_shadow(in: VsIn) -> VsShadowOut {
    var out: VsShadowOut;
    let rel = in.pos - UBO.shadow_light_center.xyz;
    let lx = dot(rel, UBO.shadow_light_right.xyz);
    let ly = dot(rel, UBO.shadow_light_up.xyz);
    let lz = dot(rel, UBO.shadow_light_fwd.xyz);
    let half_w = max(UBO.shadow_light_extents.x, 0.0001);
    let half_h = max(UBO.shadow_light_extents.y, 0.0001);
    let near_z = UBO.shadow_light_extents.z;
    let far_z = max(UBO.shadow_light_extents.w, near_z + 0.0001);
    let nx = lx / half_w;
    let ny = ly / half_h;
    let depth = clamp((lz - near_z) / (far_z - near_z), 0.0, 1.0);
    out.pos = vec4<f32>(nx, ny, depth, 1.0);
    out.uv = in.uv;
    out.tile_index = in.tile_index;
    out.tile_index2 = in.tile_index2;
    out.blend_factor = in.blend_factor;
    out.opacity = clamp(in.opacity, 0.0, 1.0);
    return out;
}

@fragment
fn fs_shadow(in: VsShadowOut) {
    let clamp_uv = (in.tile_index2 & TILE_INDEX_CLAMP_UV_FLAG) != 0u;
    let tile_index2 = in.tile_index2 & (~TILE_INDEX_FLAGS_MASK);
    let is_avatar = (in.tile_index2 & TILE_INDEX_AVATAR_FLAG) != 0u;
    let is_billboard = (in.tile_index2 & TILE_INDEX_BILLBOARD_FLAG) != 0u;
    let is_particle = (in.tile_index2 & TILE_INDEX_PARTICLE_FLAG) != 0u;
    if (is_particle) {
        discard;
    }
    let phase_start = select(0u, u32(max(in.blend_factor, 0.0)), is_billboard);
    let blend = clamp(in.blend_factor, 0.0, 1.0);
    let c0 = select(sample_tile_lod0(in.tile_index, in.uv, clamp_uv, phase_start), sample_avatar(in.tile_index, in.uv), is_avatar);
    let c1 = sample_tile_lod0(tile_index2, in.uv, clamp_uv, phase_start);
    let m0_raw = sample_tile_material(in.tile_index, in.uv, clamp_uv, phase_start);
    let m1_raw = sample_tile_material(tile_index2, in.uv, clamp_uv, phase_start);
    let m0 = select(
        unpack_material_nibbles(m0_raw),
        vec4<f32>(1.0, 0.0, 1.0, 0.0),
        is_avatar
    );
    let m1 = unpack_material_nibbles(m1_raw);
    var mat = select(mix(m0, m1, blend), m0, is_avatar);
    let color = select(mix(c0, c1, blend), c0, is_avatar);
    let intrinsic_alpha = clamp(color.a * mat.z, 0.0, 1.0);
    // Materials with opacity < 1.0 should not cast sun shadows in Raster3D.
    // This lets sunlight pass through semi-transparent tiles (e.g. window glass).
    if (mat.z < 0.999) {
        discard;
    }
    // Shadow occlusion should not follow per-geometry fade opacity; only intrinsic cutout alpha.
    if (intrinsic_alpha <= 0.5) {
        discard;
    }
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let clamp_uv = (in.tile_index2 & TILE_INDEX_CLAMP_UV_FLAG) != 0u;
    let tile_index2 = in.tile_index2 & (~TILE_INDEX_FLAGS_MASK);
    let is_avatar = (in.tile_index2 & TILE_INDEX_AVATAR_FLAG) != 0u;
    let is_billboard = (in.tile_index2 & TILE_INDEX_BILLBOARD_FLAG) != 0u;
    let is_particle = (in.tile_index2 & TILE_INDEX_PARTICLE_FLAG) != 0u;
    let dpdx_pos = dpdx(in.world_pos);
    let dpdy_pos = dpdy(in.world_pos);
    let dpdx_uv = dpdx(in.uv);
    let dpdy_uv = dpdy(in.uv);
    let sun_enabled = select(0.0, 1.0, UBO.sun_dir_enabled.w > 0.5);
    let L = normalize(-UBO.sun_dir_enabled.xyz);
    let N_shadow = normalize(in.normal);
    let Nf_shadow = select(-N_shadow, N_shadow, dot(N_shadow, L) >= 0.0);
    let NdotL_shadow = max(dot(Nf_shadow, L), 0.0);
    let shadow_receiver = in.world_pos + Nf_shadow * select(0.01, 0.03, UBO.cam_kind == 2u);
    let shadow = sample_shadow(shadow_receiver, NdotL_shadow);
    let phase_start = select(0u, u32(max(in.blend_factor, 0.0)), is_billboard);
    var blend = clamp(in.blend_factor, 0.0, 1.0);
    if (!is_avatar && !is_billboard && !is_particle && in.surface_noise.w > 0.5) {
        blend = surface_noise_blend(in.surface_noise, in.world_pos);
    }
    let c0 = select(sample_tile(in.tile_index, in.uv, clamp_uv, phase_start), sample_avatar(in.tile_index, in.uv), is_avatar);
    let c1 = sample_tile(tile_index2, in.uv, clamp_uv, phase_start);
    let c0_base = select(sample_tile_lod0(in.tile_index, in.uv, clamp_uv, phase_start), sample_avatar(in.tile_index, in.uv), is_avatar);
    let c1_base = sample_tile_lod0(tile_index2, in.uv, clamp_uv, phase_start);
    let m0_raw = sample_tile_material(in.tile_index, in.uv, clamp_uv, phase_start);
    let m1_raw = sample_tile_material(tile_index2, in.uv, clamp_uv, phase_start);
    if (is_particle) {
        let local = in.uv * 2.0 - vec2<f32>(1.0, 1.0);
        let dist = length(local);
        if (dist > 1.0) {
            discard;
        }
        let radial = pow(max(1.0 - dist, 0.0), 2.4);
        let vertical = clamp(1.0 - in.uv.y, 0.0, 1.0);
        let tint = max(in.normal, vec3<f32>(0.0));
        let color = tint * mix(0.75, 1.15, vertical);
        let alpha = radial * pow(clamp(in.opacity, 0.0, 1.0), 0.85);
        if (alpha <= 0.015) {
            discard;
        }
        return vec4<f32>(apply_post(color * 1.1, in.pos), alpha);
    }
    let m0 = select(
        unpack_material_nibbles(m0_raw),
        vec4<f32>(1.0, 0.0, 1.0, 0.0),
        is_avatar
    );
    let m1 = unpack_material_nibbles(m1_raw);
    var mat = select(mix(m0, m1, blend), m0, is_avatar);
    let material_id0 = unpack_material_id(m0_raw);
    let material_id1 = unpack_material_id(m1_raw);
    var material_id = select(select(material_id0, material_id1, blend > 0.5), 0u, is_avatar);
    var material_traits0 = semantic_material_traits0(material_id);
    var material_traits1 = semantic_material_traits1(material_id);
    let n0_ts = select(unpack_material_normal_ts(m0_raw), vec3<f32>(0.0, 0.0, 1.0), is_avatar);
    let n1_ts = unpack_material_normal_ts(m1_raw);
    let n_ts = normalize(select(mix(n0_ts, n1_ts, blend), n0_ts, is_avatar));
    var color = select(mix(c0, c1, blend), c0, is_avatar);
    let color_base = select(mix(c0_base, c1_base, blend), c0_base, is_avatar);
    let paint_overlay = sample_paint_overlay(in.pos);
    let paint_weight = clamp(paint_overlay.a, 0.0, 1.0) * select(1.0, 0.0, is_avatar);
    if (paint_weight > 0.001) {
        let paint_material_id = sample_paint_material_id(in.pos);
        let paint_mat = semantic_material_rmoe(paint_material_id);
        material_id = paint_material_id;
        material_traits0 = semantic_material_traits0(material_id);
        material_traits1 = semantic_material_traits1(material_id);
        color = vec4<f32>(mix(color.rgb, paint_overlay.rgb, paint_weight), color.a);
        mat = mix(mat, paint_mat, paint_weight);
    }
    // Keep first-person nearby surfaces crisp by blending from LOD0 near the camera.
    var alpha_sample = color.a;
    if (!is_avatar && UBO.cam_kind == 2u) {
        let dist = distance(in.world_pos, UBO.cam_pos.xyz);
        let near_end = max(UBO.render_params.z, 0.0);
        let far_start = max(UBO.render_params.w, near_end + 0.001);
        let t = smoothstep(near_end, far_start, dist);
        color = mix(color_base, color, t);
        alpha_sample = mix(color_base.a, color.a, t);
    }
    // Keep opacity/cutout tied to the same sample path as color to avoid distant dark speckles.
    let intrinsic_alpha = clamp(alpha_sample * mat.z, 0.0, 1.0);
    var coverage = clamp(intrinsic_alpha * in.opacity, 0.0, 1.0);
    if (is_particle) {
        if (coverage <= 0.04) {
            discard;
        }
        coverage = pow(coverage, 0.55);
    }
    if (coverage <= 0.001) {
        discard;
    }
    let fade_mode = UBO._pad0.x;
    let has_material_fade = mat.z < 0.999;
    let is_fading = in.opacity < 0.999 || has_material_fade;
    if (is_fading) {
        if (!is_particle && !has_material_fade && fade_mode == 0u) {
            // Ordered dither mode: stable pseudo-transparency without alpha sorting.
            let dither = bayer4_threshold(u32(in.pos.x), u32(in.pos.y));
            if (coverage <= dither) {
                discard;
            }
        }
    } else {
        // Non-fading surfaces use cutout thresholding for stable alpha-tested materials.
        if (intrinsic_alpha <= 0.5) {
            discard;
        }
    }
    let out_alpha = select(
        1.0,
        coverage,
        is_particle || has_material_fade || (in.opacity < 0.999 && fade_mode == 1u)
    );
    let color_linear = pow(color.rgb, vec3<f32>(2.2));
    if (mat.w > 0.001) {
        let emission_normal = normalize(in.normal);
        let emission_view = normalize(UBO.cam_pos.xyz - in.world_pos);
        let emission_edge = pow(clamp(1.0 - abs(dot(emission_normal, emission_view)), 0.0, 1.0), 1.65);
        let emission_luma = dot(color_linear, vec3<f32>(0.2126, 0.7152, 0.0722));
        let screen_sparkle = hash12(floor(in.pos.xy * 0.5) + vec2<f32>(31.0, 7.0)) * 2.0 - 1.0;
        let style_glow = max(UBO.post_style1.yzw, vec3<f32>(0.0));
        let emissive_boost = 1.05 + mat.w * 2.45;
        let surface_halo = color_linear * (0.22 + emission_edge * 0.62 + emission_luma * 0.18);
        let emissive_color = color_linear * emissive_boost
            + surface_halo * mat.w
            + style_glow * (0.035 + emission_luma * 0.10)
            + vec3<f32>(screen_sparkle) * (0.018 * mat.w);
        return vec4<f32>(apply_post(emissive_color, in.pos), out_alpha);
    }
    let ambient = UBO.ambient_color_strength.xyz * UBO.ambient_color_strength.w
        + max(UBO.post_style1.yzw, vec3<f32>(0.0));
    var N = normalize(in.normal);
    let V = normalize(UBO.cam_pos.xyz - in.world_pos);
    var bump_strength = clamp(UBO.shadow_params.z, 0.0, 1.0);
    // Bump mapping is unstable on cutout/partially transparent texels and at long distances.
    // Fade it out with distance in first-person and disable it on non-opaque coverage.
    if (!is_avatar && UBO.cam_kind == 2u) {
        let dist = distance(in.world_pos, UBO.cam_pos.xyz);
        let near_end = max(UBO.render_params.z, 0.0);
        let far_start = max(UBO.render_params.w, near_end + 0.001);
        let t = smoothstep(near_end, far_start, dist);
        bump_strength = bump_strength * (1.0 - t);
    }
    if (intrinsic_alpha < 0.999) {
        bump_strength = 0.0;
    }
    let det = dpdx_uv.x * dpdy_uv.y - dpdx_uv.y * dpdy_uv.x;
    let safe_det = select(1.0, det, abs(det) > 1e-6);
    let T = normalize((dpdx_pos * dpdy_uv.y - dpdy_pos * dpdx_uv.y) / safe_det);
    let B = normalize((-dpdx_pos * dpdy_uv.x + dpdy_pos * dpdx_uv.x) / safe_det);
    let N_ws = normalize(mat3x3<f32>(T, B, N) * n_ts);
    let bump_apply = (!is_avatar) && bump_strength > 0.001 && abs(det) > 1e-6;
    let bump_mix = select(0.0, bump_strength, bump_apply);
    N = normalize(mix(N, N_ws, bump_mix));
    let material_family = u32(round(clamp(material_traits1.y, 0.0, 255.0)));
    let material_finish = u32(round(clamp(material_traits1.z, 0.0, 3.0)));
    let painted_material_weight = select(1.0, paint_weight, paint_weight > 0.001);
    let water_surface_weight = select(0.0, 1.0, material_family == 6u)
        * select(1.0, 0.0, is_avatar)
        * painted_material_weight
        * select(1.0, 0.0, abs(det) <= 1e-6);
    let water_ripple_a = sin(dot(in.world_pos, vec3<f32>(7.7, 3.1, 9.2)));
    let water_ripple_b = sin(dot(in.world_pos, vec3<f32>(4.2, 5.7, 11.3)) + 1.7);
    let water_ripple = (T * water_ripple_a + B * water_ripple_b) * 0.035;
    N = normalize(mix(N, normalize(N + water_ripple), water_surface_weight * (0.45 + paint_weight * 0.35)));
    // Match probe lookup to the same two-sided surface convention as direct lighting.
    let Nf_view = select(-N, N, dot(N, V) >= 0.0);
    let Nf_sun = select(-N, N, dot(N, L) >= 0.0);
    let probe = sample_irradiance_grid(in.world_pos, Nf_view);
    let probe_luma = dot(probe, vec3<f32>(0.2126, 0.7152, 0.0722));
    let point_residual = 1.0 - smoothstep(0.025, 0.16, probe_luma) * 0.9;
    var ambient_probe = ambient + probe;
    // Sun lighting stays light-facing so iso camera motion does not change whether a surface is lit.
    let first_person_view = UBO.cam_kind == 2u;
    let material_style_strength = select(1.0, 0.55, first_person_view);
    var roughness = clamp(mat.x, 0.04, 1.0);
    var metallic = clamp(mat.y, 0.0, 1.0);
    let scene_surface = select(1.0, 0.0, is_avatar);
    let polished_finish_weight =
        select(0.0, 1.0, material_finish == 2u) * scene_surface * painted_material_weight;
    let wet_finish_weight =
        select(0.0, 1.0, material_finish == 3u) * scene_surface * painted_material_weight;
    let water_material_weight =
        select(0.0, 1.0, material_family == 6u) * scene_surface * painted_material_weight;
    roughness = mix(roughness, max(0.035, roughness * 0.72), polished_finish_weight * 0.35);
    roughness = mix(roughness, min(roughness, 0.075), wet_finish_weight * 0.72);
    roughness = mix(roughness, 0.025, water_material_weight);
    metallic = mix(metallic, 0.0, water_material_weight);
    let matte_weight = smoothstep(0.58, 0.92, roughness) * (1.0 - metallic) * scene_surface;
    let polished_weight = smoothstep(0.20, 0.70, metallic + (1.0 - roughness)) * scene_surface;
    let vertical_weight = (1.0 - abs(N.y)) * scene_surface;
    let ceiling_weight = smoothstep(0.18, 0.72, -N.y) * scene_surface;
    let floor_weight = smoothstep(0.18, 0.72, N.y) * scene_surface;
    // Outdoor iso/orbit readability: clear sunny scenes need stronger plane separation
    // without extra user-facing settings.
    let non_first_person = select(1.0, 0.0, first_person_view);
    let clear_air = 1.0 - smoothstep(0.04, 0.55, UBO.fog_color_density.w);
    let outdoor_space = scene_surface * non_first_person * sun_enabled * clear_air;
    let sunward = smoothstep(0.08, 0.72, dot(Nf_sun, L));
    let sky_luma = dot(UBO.sky_color.xyz, vec3<f32>(0.2126, 0.7152, 0.0722));
    let sky_chroma = mix(vec3<f32>(sky_luma), UBO.sky_color.xyz, 0.55);
    ambient_probe += sky_chroma * outdoor_space * (floor_weight * 0.030 + vertical_weight * (0.010 + (1.0 - sunward) * 0.016));
    ambient_probe *= 1.0 - outdoor_space * vertical_weight * (1.0 - sunward) * 0.075;
    ambient_probe *= clamp(1.0 - ceiling_weight * 0.08 - vertical_weight * 0.025 + floor_weight * 0.115, 0.66, 1.18);
    let n_dot_v = max(dot(Nf_view, V), 0.0);
    var albedo = color_linear;
    if (water_material_weight > 0.001) {
        let water_depth_noise = surface_noise_value(in.world_pos * 2.2 + vec3<f32>(6.0, 13.0, 2.0), 89.0);
        let water_absorb = vec3<f32>(0.46, 0.62, 0.70) * (0.86 + water_depth_noise * 0.16);
        albedo = mix(albedo, albedo * water_absorb + UBO.sky_color.xyz * 0.035, water_material_weight * (0.54 + paint_weight * 0.22));
    }
    if (wet_finish_weight > 0.001) {
        let wet_grain = surface_noise_value(in.world_pos * 3.8 + N * 0.37 + vec3<f32>(2.0, 19.0, 5.0), 97.0);
        let wet_absorb = vec3<f32>(0.72, 0.77, 0.80) * (0.92 + wet_grain * 0.13);
        albedo = mix(albedo, albedo * wet_absorb + UBO.sky_color.xyz * 0.018, wet_finish_weight * 0.42);
    }
    if (!is_avatar) {
        let material_noise = surface_noise_value(in.world_pos * 3.35 + N * 0.19, 23.0) * 2.0 - 1.0;
        albedo *= 1.0 + material_noise * matte_weight * 0.075;
        let albedo_luma = dot(albedo, vec3<f32>(0.2126, 0.7152, 0.0722));
        let matte_contrast = vec3<f32>(albedo_luma) + (albedo - vec3<f32>(albedo_luma)) * 1.08;
        let ceiling_soft_contrast = vec3<f32>(albedo_luma) + (albedo - vec3<f32>(albedo_luma)) * 0.78;
        albedo = mix(
            mix(albedo, matte_contrast, matte_weight * 0.35),
            ceiling_soft_contrast,
            ceiling_weight * 0.28
        );
    }
    if (!is_avatar) {
        let style_amount = 0.42 * material_style_strength;
        let target_roughness = 0.68;
        let target_metallic = 0.82;
        let large_grain_scale = 1.25;
        let fine_grain_scale = 4.25;
        let large_grain = hash13(floor(in.world_pos * large_grain_scale));
        let fine_grain = hash13(floor(in.world_pos * fine_grain_scale + vec3<f32>(19.0, 7.0, 31.0)));
        let grime = mix(large_grain, fine_grain, 0.35);
        let grime_value = 0.88 + grime * 0.20;
        let style_tint = vec3<f32>(1.015, 0.985, 0.925);
        let earth_tint = mix(vec3<f32>(1.0), style_tint * grime_value, style_amount);
        albedo = albedo * earth_tint;
        roughness = mix(roughness, clamp(roughness * 0.86 + 0.08, 0.06, target_roughness), style_amount);
        metallic = clamp(mix(metallic, metallic * target_metallic, polished_weight * style_amount), 0.0, 1.0);
    }
    let F0 = mix(vec3<f32>(0.04), albedo, metallic);
    let NdotL = max(dot(Nf_sun, L), 0.0);
    let shadow_strength = clamp(UBO.shadow_params.y, 0.0, 1.0);
    let shadow_term = mix(1.0, shadow, shadow_strength);
    let shadow_depth = 1.0 - shadow_term;
    let sun_radiance = UBO.sun_color_intensity.xyz * UBO.sun_color_intensity.w * sun_enabled * shadow_term;
    var sun = vec3<f32>(0.0);
    if (NdotL > 0.0) {
        let H = normalize(V + L);
        let NdotV = max(dot(Nf_view, V), 0.0);
        let NdotH = max(dot(Nf_sun, H), 0.0);
        let VdotH = max(dot(V, H), 0.0);
        let NDF = distribution_ggx(NdotH, roughness);
        let G = geometry_schlick_ggx(NdotV, roughness) * geometry_schlick_ggx(NdotL, roughness);
        let F = fresnel_schlick(VdotH, F0);
        let spec = (NDF * G * F) / max(4.0 * NdotV * NdotL, 1e-5);
        let kS = F;
        let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic);
        let diffuse = kD * albedo;
        sun = (diffuse + spec) * sun_radiance * NdotL;
    }
    var point = vec3<f32>(0.0);
    let point_count = min(UBO.point_light_count_pad.x, 8u);
    for (var li: u32 = 0u; li < point_count; li = li + 1u) {
        let lp = UBO.point_light_pos_intensity[li].xyz;
        let l_intensity = UBO.point_light_pos_intensity[li].w;
        let l_range = max(UBO.point_light_color_range[li].w, 0.001);
        let Lp_vec = lp - in.world_pos;
        let l_dist = length(Lp_vec);
        let l_dir = select(vec3<f32>(0.0, 1.0, 0.0), normalize(Lp_vec), l_dist > 1e-5);
        let Nf_point = select(-N, N, dot(N, l_dir) >= 0.0);
        let l_ndotl = max(dot(Nf_point, l_dir), 0.0);
        let l_range_factor = smoothstep(l_range, 0.0, l_dist);
        let l_atten = (l_intensity * l_range_factor) / max(l_dist * l_dist, 1e-4);
        let radiance = UBO.point_light_color_range[li].xyz * l_atten;
        if (l_ndotl > 0.0) {
            let H = normalize(V + l_dir);
            let NdotV = max(dot(Nf_view, V), 0.0);
            let NdotH = max(dot(Nf_point, H), 0.0);
            let VdotH = max(dot(V, H), 0.0);
            let NDF = distribution_ggx(NdotH, roughness);
            let G = geometry_schlick_ggx(NdotV, roughness) * geometry_schlick_ggx(l_ndotl, roughness);
            let F = fresnel_schlick(VdotH, F0);
            let spec = (NDF * G * F) / max(4.0 * NdotV * l_ndotl, 1e-5);
            let kS = F;
            let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic);
            let diffuse = kD * albedo;
            point += (diffuse + spec) * radiance * l_ndotl;
        }
    }
    point *= point_residual;
    var lit_color = max(ambient_probe * albedo + sun + point, vec3<f32>(0.0));

    if (!is_avatar) {
        let fog_luma = dot(UBO.fog_color_density.xyz, vec3<f32>(0.2126, 0.7152, 0.0722));
        let fog_chroma = mix(vec3<f32>(fog_luma), UBO.fog_color_density.xyz, 0.50);
        let reflected_view = reflect(-V, Nf_view);
        let env_up = clamp(reflected_view.y * 0.5 + 0.5, 0.0, 1.0);
        let env_color = mix(fog_chroma, sky_chroma, env_up);
        let shadow_color = mix(fog_chroma, sky_chroma, 0.62);
        let indirect_strength = material_style_strength * scene_surface;
        let sky_wrap = pow(clamp(Nf_view.y * 0.5 + 0.5, 0.0, 1.0), 1.25);
        let shadow_bounce = shadow_depth * (0.030 + matte_weight * 0.030 + floor_weight * 0.018) * indirect_strength;
        let cool_shadow = albedo * shadow_color * shadow_bounce;
        let matte_scatter = albedo * sky_chroma * matte_weight * sky_wrap * (0.012 + outdoor_space * 0.018) * indirect_strength;
        let polish_env = env_color * F0 * polished_weight * pow(max(1.0 - roughness, 0.0), 1.6) * (0.090 + shadow_depth * 0.035) * indirect_strength;
        let wet_weight = smoothstep(0.54, 0.92, 1.0 - roughness) * (1.0 - metallic) * scene_surface;
        let translucent_weight = (1.0 - smoothstep(0.62, 0.98, mat.z)) * scene_surface;
        let semantic_subsurface = material_traits0.x;
        let semantic_transmission = material_traits0.y;
        let semantic_fuzz = material_traits0.z;
        let semantic_porosity = material_traits0.w;
        let semantic_sheen = clamp(material_traits1.x, 0.0, 1.0);
        let grazing = pow(clamp(1.0 - n_dot_v, 0.0, 1.0), 2.6);
        let glint_noise = surface_noise_value(in.world_pos * 10.0 + N * 1.7, 71.0);
        let finish_wet_sheen = wet_finish_weight * (0.060 + grazing * 0.115);
        let finish_polish_sheen = polished_finish_weight * (0.028 + grazing * 0.055);
        let wet_sheen = env_color * (wet_weight * 0.050 + translucent_weight * 0.065 + finish_wet_sheen + finish_polish_sheen)
            * (0.35 + grazing * 1.40)
            * (0.78 + glint_noise * 0.44)
            * material_style_strength;
        let puddle_gloss = env_color
            * water_material_weight
            * (0.080 + semantic_transmission * 0.070)
            * (0.45 + grazing * 1.75)
            * (0.80 + glint_noise * 0.55)
            * material_style_strength;
        let back_wrap = pow(clamp(dot(-Nf_sun, L) * 0.5 + 0.5, 0.0, 1.0), 2.0);
        let sss_tint = mix(albedo, vec3<f32>(1.0, 0.58, 0.36), smoothstep(0.45, 0.75, semantic_subsurface));
        let sss_light = (sun_radiance + sky_chroma * (0.12 + outdoor_space * 0.10)) * back_wrap;
        let semantic_sss = sss_tint * sss_light * semantic_subsurface * (0.050 + semantic_transmission * 0.035) * material_style_strength;
        let semantic_fuzz_light = albedo * sky_chroma * semantic_fuzz * pow(clamp(1.0 - n_dot_v, 0.0, 1.0), 1.2) * 0.040 * material_style_strength;
        let semantic_porous_dark = semantic_porosity * (0.010 + shadow_depth * 0.026) * material_style_strength;
        let semantic_family_sheen = env_color * semantic_sheen * pow(max(1.0 - roughness, 0.0), 1.25) * (0.030 + grazing * 0.070) * material_style_strength;
        let matte_tooth = (surface_noise_value(in.world_pos * 6.5 + vec3<f32>(13.0, 3.0, 7.0), 41.0) * 2.0 - 1.0)
            * matte_weight
            * material_style_strength;
        lit_color += cool_shadow + matte_scatter + polish_env + wet_sheen + puddle_gloss + semantic_sss + semantic_fuzz_light + semantic_family_sheen;
        lit_color *= 1.0 + matte_tooth * 0.018 - semantic_porous_dark;

        let edge = pow(clamp(1.0 - n_dot_v, 0.0, 1.0), 2.4);
        let edge_amount = edge * (0.018 + matte_weight * 0.020 + polished_weight * 0.045 + outdoor_space * 0.016);
        lit_color += albedo * edge_amount;
        let lit_luma = dot(lit_color, vec3<f32>(0.2126, 0.7152, 0.0722));
        let view_dist = distance(in.world_pos, UBO.cam_pos.xyz);
        // Dark first-person dungeon fill: keeps unlit corridors readable while staying
        // inactive in bright/open scenes.
        let first_person_space = select(0.0, 1.0, UBO.cam_kind == 2u);
        let dark_fog_space = smoothstep(0.32, 1.25, UBO.fog_color_density.w)
            * (1.0 - smoothstep(0.12, 0.46, dot(UBO.fog_color_density.xyz, vec3<f32>(0.2126, 0.7152, 0.0722))));
        let corridor_band = smoothstep(1.6, 5.5, view_dist) * (1.0 - smoothstep(22.0, 34.0, view_dist));
        let corridor_dark = 1.0 - smoothstep(0.045, 0.18, lit_luma);
        let corridor_surface = vertical_weight * (0.50 + n_dot_v * 0.35) + floor_weight * 0.58 + ceiling_weight * 0.18;
        let corridor_fill = first_person_space * dark_fog_space * corridor_band * corridor_dark * corridor_surface * (0.035 + matte_weight * 0.014);
        let ambient_luma = dot(UBO.ambient_color_strength.xyz, vec3<f32>(0.2126, 0.7152, 0.0722));
        let corridor_tint = mix(vec3<f32>(ambient_luma), UBO.ambient_color_strength.xyz, 0.62);
        lit_color += albedo * corridor_tint * corridor_fill;
        let dark_floor = (1.0 - smoothstep(0.035, 0.16, lit_luma)) * floor_weight;
        let floor_bounce = dark_floor * (0.014 + probe_luma * 0.26 + matte_weight * 0.012);
        lit_color += albedo * floor_bounce;
        let near_wall = vertical_weight * (1.0 - smoothstep(1.1, 3.4, view_dist));
        let near_wall_luma = dot(lit_color, vec3<f32>(0.2126, 0.7152, 0.0722));
        let near_wall_compress = near_wall * smoothstep(0.18, 0.66, near_wall_luma) * 0.24;
        lit_color *= 1.0 - near_wall_compress;
        let upper_shadow = ceiling_weight * (0.022 + matte_weight * 0.010);
        lit_color *= 1.0 - upper_shadow;
        let outdoor_luma = dot(lit_color, vec3<f32>(0.2126, 0.7152, 0.0722));
        let outdoor_highlight = outdoor_space * smoothstep(0.48, 1.05, outdoor_luma);
        let outdoor_sat = vec3<f32>(outdoor_luma) + (lit_color - vec3<f32>(outdoor_luma)) * 1.06;
        let outdoor_capped = lit_color * (1.0 - outdoor_highlight * 0.13);
        lit_color = mix(lit_color, outdoor_sat, outdoor_space * 0.14);
        lit_color = mix(lit_color, outdoor_capped, outdoor_highlight);
        let normal_detail = clamp((length(dpdx(Nf_view)) + length(dpdy(Nf_view))) * 0.16, 0.0, 1.0);
        let dominant_x_wall = abs(N.x) > abs(N.z);
        let wall_uv = select(
            vec2<f32>(in.world_pos.x, in.world_pos.y),
            vec2<f32>(in.world_pos.z, in.world_pos.y),
            dominant_x_wall
        );
        let floor_uv = vec2<f32>(in.world_pos.x, in.world_pos.z);
        let paint_uv = mix(wall_uv, floor_uv, floor_weight);
        let grid_frac = abs(fract(paint_uv) - vec2<f32>(0.5));
        let grid_dist = 0.5 - max(grid_frac.x, grid_frac.y);
        let contact_noise = 0.72 + surface_noise_value(in.world_pos * 5.1 + vec3<f32>(5.0, 11.0, 17.0), 53.0) * 0.46;
        let world_crease = (1.0 - smoothstep(0.010, 0.075, grid_dist))
            * contact_noise
            * (vertical_weight * 0.030 + floor_weight * 0.012)
            * material_style_strength;
        let lower_wall_band = (1.0 - smoothstep(0.030, 0.180, fract(in.world_pos.y)))
            * vertical_weight
            * (0.020 + matte_weight * 0.018)
            * contact_noise
            * material_style_strength;
        let screen_crease = normal_detail * (0.026 + matte_weight * 0.020 + polished_weight * 0.010) * material_style_strength;
        let contact_shadow = clamp(world_crease + lower_wall_band + screen_crease, 0.0, 0.11);
        let contact_tint = mix(vec3<f32>(0.70, 0.74, 0.80), vec3<f32>(0.62, 0.58, 0.50), matte_weight);
        lit_color *= 1.0 - contact_shadow;
        lit_color += albedo * contact_tint * world_crease * 0.18;
        let soft_edge = pow(clamp(1.0 - n_dot_v, 0.0, 1.0), 1.7);
        let nomad_rim = soft_edge * (0.030 + matte_weight * 0.018 + polished_weight * 0.070) * material_style_strength;
        let plane_lift = (floor_weight * 0.012 + vertical_weight * 0.008) * material_style_strength;
        lit_color += albedo * (nomad_rim + plane_lift);
        let nomad_luma = dot(lit_color, vec3<f32>(0.2126, 0.7152, 0.0722));
        let chroma = vec3<f32>(nomad_luma) + (lit_color - vec3<f32>(nomad_luma)) * 1.045;
        let highlight_guard = 1.0 - smoothstep(0.62, 1.25, nomad_luma) * 0.10;
        lit_color = mix(lit_color, chroma * highlight_guard, 0.24 * material_style_strength);
    }

    if (!is_avatar) {
        let levels = select(10.0, 18.0, first_person_view);
        let min_luma = select(0.026, 0.018, first_person_view);
        let quant_mix = select(0.46, 0.18, first_person_view);
        let luma = max(dot(lit_color, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.001);
        let quantized = max(floor(luma * levels + 0.5) / levels, min_luma);
        let quantized_color = lit_color * (quantized / luma);
        let floor_color = albedo * min_luma;
        let floor_weight = 1.0 - smoothstep(min_luma, min_luma * 3.0, luma);
        let stylized_color = mix(quantized_color, max(quantized_color, floor_color), floor_weight);
        lit_color = mix(lit_color, stylized_color, quant_mix);
    }

    if (is_avatar && UBO.avatar_highlight_params.w > 0.5) {
        // Avatar readability boost: keep sprites/avatars from visually collapsing into scene tones.
        // This is intentionally subtle and only applied to avatar draw path.
        let avatar_lift = max(UBO.avatar_highlight_params.x, 0.0);
        let avatar_fill = max(UBO.avatar_highlight_params.y, 0.0);
        let avatar_rim = max(UBO.avatar_highlight_params.z, 0.0);
        let rim = pow(clamp(1.0 - n_dot_v, 0.0, 1.0), 2.0);
        let key = UBO.sun_color_intensity.xyz * UBO.sun_color_intensity.w;
        let fill = ambient_probe * albedo;
        let boosted = lit_color * avatar_lift + fill * avatar_fill + key * (avatar_rim * rim);

        // Keep pale atlas colors from clipping to white after post exposure.
        let post_enabled = UBO.post_params.x > 0.5;
        let tone_mapper = u32(max(UBO.post_params.y, 0.0));
        let exposure = select(1.0, max(UBO.post_params.z, 0.001), post_enabled && tone_mapper == 0u);
        let avatar_cap = clamp(0.88 / exposure, 0.45, 0.90);
        let avatar_knee = avatar_cap * 0.72;
        let avatar_headroom = max(avatar_cap - avatar_knee, 0.001);
        let avatar_luma = max(dot(boosted, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.0001);
        let avatar_over = max(avatar_luma - avatar_knee, 0.0);
        let compressed_luma = avatar_knee + avatar_headroom * (1.0 - exp(-avatar_over / avatar_headroom));
        let avatar_scale = select(1.0, min(1.0, compressed_luma / avatar_luma), avatar_luma > avatar_knee);
        lit_color = boosted * avatar_scale;
    }

    let fog_density = max(UBO.fog_color_density.w, 0.0);
    if (fog_density <= 0.0) {
        return vec4<f32>(apply_post(lit_color, in.pos), out_alpha);
    }
    let fog_dist = distance(in.world_pos, UBO.cam_pos.xyz);
    let fog_amount = fog_density * fog_dist * fog_dist;
    let fog_factor = clamp(exp(-fog_amount), 0.0, 1.0);
    let fogged = mix(UBO.fog_color_density.xyz, lit_color, fog_factor);
    return vec4<f32>(apply_post(fogged, in.pos), out_alpha);
}
"#;

pub const SCENEVM_3D_LINE_WGSL: &str = r#"
struct U {
    cam_pos: vec4<f32>,
    cam_fwd: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    sun_color_intensity: vec4<f32>,
    sun_dir_enabled: vec4<f32>,
    ambient_color_strength: vec4<f32>,
    sky_color: vec4<f32>,
    fog_color_density: vec4<f32>,
    shadow_light_right: vec4<f32>,
    shadow_light_up: vec4<f32>,
    shadow_light_fwd: vec4<f32>,
    shadow_light_center: vec4<f32>,
    shadow_light_extents: vec4<f32>,
    shadow_params: vec4<f32>,
    render_params: vec4<f32>,
    point_light_pos_intensity: array<vec4<f32>, 8>,
    point_light_color_range: array<vec4<f32>, 8>,
    point_light_count_pad: vec4<u32>,
    _pad_lights: vec4<u32>,
    fb_size: vec2<f32>,
    cam_vfov_deg: f32,
    cam_ortho_half_h: f32,
    cam_near: f32,
    cam_far: f32,
    cam_kind: u32,
    anim_counter: u32,
    _pad0: vec2<u32>,
    _pad_post_pre: vec2<u32>,
    post_params: vec4<f32>,
    post_color_adjust: vec4<f32>,
    post_style0: vec4<f32>,
    post_style1: vec4<f32>,
    avatar_highlight_params: vec4<f32>,
    _pad_tail: vec4<u32>,
    palette: array<vec4<f32>, 256>,
    palette_tile_indices: array<vec4<u32>, 64>,
    organic_params: vec4<u32>,
};
@group(0) @binding(0) var<uniform> UBO: U;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn camera_to_clip(world_pos: vec3<f32>) -> vec4<f32> {
    let rel = world_pos - UBO.cam_pos.xyz;
    let cx = dot(rel, UBO.cam_right.xyz);
    let cy = dot(rel, UBO.cam_up.xyz);
    let cz = dot(rel, UBO.cam_fwd.xyz);

    let near_z = max(UBO.cam_near, 0.0001);
    let far_z = max(UBO.cam_far, near_z + 0.0001);
    let aspect = max(UBO.fb_size.x / max(UBO.fb_size.y, 1.0), 0.0001);

    if (UBO.cam_kind == 0u) {
        let depth = clamp((cz - near_z) / (far_z - near_z), 0.0, 1.0);
        let half_h = max(UBO.cam_ortho_half_h, 0.0001);
        let half_w = max(half_h * aspect, 0.0001);
        return vec4<f32>(cx / half_w, cy / half_h, depth, 1.0);
    }

    var z = cz;
    if (abs(z) < 0.0001) {
        z = select(-0.0001, 0.0001, z >= 0.0);
    }
    let f = 1.0 / tan(radians(max(UBO.cam_vfov_deg, 1.0)) * 0.5);
    let a = far_z / (far_z - near_z);
    let b = (-near_z * far_z) / (far_z - near_z);
    return vec4<f32>(cx * (f / aspect), cy * f, a * z + b, z);
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.pos = camera_to_clip(in.pos);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

pub const SCENEVM_3D_POST_WGSL: &str = r#"
struct U {
    cam_pos: vec4<f32>,
    cam_fwd: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    sun_color_intensity: vec4<f32>,
    sun_dir_enabled: vec4<f32>,
    ambient_color_strength: vec4<f32>,
    sky_color: vec4<f32>,
    fog_color_density: vec4<f32>,
    shadow_light_right: vec4<f32>,
    shadow_light_up: vec4<f32>,
    shadow_light_fwd: vec4<f32>,
    shadow_light_center: vec4<f32>,
    shadow_light_extents: vec4<f32>,
    shadow_params: vec4<f32>,
    render_params: vec4<f32>,
    point_light_pos_intensity: array<vec4<f32>, 8>,
    point_light_color_range: array<vec4<f32>, 8>,
    point_light_count_pad: vec4<u32>,
    _pad_lights: vec4<u32>,
    fb_size: vec2<f32>,
    cam_vfov_deg: f32,
    cam_ortho_half_h: f32,
    cam_near: f32,
    cam_far: f32,
    cam_kind: u32,
    anim_counter: u32,
    _pad0: vec2<u32>,
    _pad_post_pre: vec2<u32>,
    post_params: vec4<f32>,
    post_color_adjust: vec4<f32>,
    post_style0: vec4<f32>,
    post_style1: vec4<f32>,
    avatar_highlight_params: vec4<f32>,
    _pad_tail: vec4<u32>,
    palette: array<vec4<f32>, 256>,
    palette_tile_indices: array<vec4<u32>, 64>,
    organic_params: vec4<u32>,
};

@group(0) @binding(0) var<uniform> UBO: U;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var bloom_tex: texture_2d<f32>;
@group(0) @binding(3) var post_smp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn hash12(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453123);
}

fn apply_post(color_linear: vec3<f32>, frag_pos: vec4<f32>) -> vec3<f32> {
    let post_enabled = UBO.post_params.x > 0.5;
    let tone_mapper = u32(max(UBO.post_params.y, 0.0));
    let first_person_post = UBO.cam_kind == 2u;
    let post_style_strength = select(1.0, 0.45, first_person_post);
    let exposure = max(UBO.post_params.z, 0.0);
    let gamma = max(UBO.post_params.w, 0.001);
    let grit = clamp(UBO.post_style0.x, 0.0, 1.0);
    let posterize = clamp(UBO.post_style0.y, 0.0, 1.0);
    let palette_bias = clamp(UBO.post_style0.z, 0.0, 1.0);
    let shadow_lift = clamp(UBO.post_style0.w, 0.0, 1.0);
    let edge_soften = clamp(UBO.post_style1.x, 0.0, 1.0);
    var c = max(color_linear, vec3<f32>(0.0));

    if (post_enabled) {
        c = c * exposure;
        if (tone_mapper == 1u) {
            c = c / (c + vec3<f32>(1.0));
        } else if (tone_mapper == 2u) {
            let a = 2.51;
            let b = 0.03;
            let d = 0.59;
            let e = 0.14;
            c = clamp((c * (a * c + b)) / (c * (2.43 * c + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
        }

        let luminance = max(UBO.post_color_adjust.y, 0.0);
        c = c * luminance;
        let highlight = max(c - vec3<f32>(0.62), vec3<f32>(0.0));
        let highlight_luma = dot(highlight, vec3<f32>(0.2126, 0.7152, 0.0722));
        let bloom_tint = mix(vec3<f32>(1.0), max(UBO.post_style1.yzw, vec3<f32>(0.0)), 0.28);
        c = c + highlight * bloom_tint * (0.035 + highlight_luma * 0.045) * post_style_strength;
        let saturation = max(UBO.post_color_adjust.x, 0.0);
        let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
        c = mix(c, c + vec3<f32>(pow(max(1.0 - luma, 0.0), 2.0)) * 0.12, shadow_lift);
        let earth = vec3<f32>(luma) * vec3<f32>(1.07, 0.98, 0.82);
        c = mix(c, mix(c, earth, 0.45), palette_bias);
        let levels = mix(32.0, 7.0, posterize);
        c = mix(c, floor(c * levels + vec3<f32>(0.5)) / levels, posterize);
        let grain = hash12(floor(frag_pos.xy)) * 2.0 - 1.0;
        let grain_amount = select(0.026, 0.014, first_person_post);
        c = c + vec3<f32>(grain) * grit * grain_amount;
        let paper = hash12(floor(frag_pos.xy * 0.5) + vec2<f32>(17.0, 3.0)) * 2.0 - 1.0;
        let nomad_luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
        let shadow_tone = 1.0 - smoothstep(0.14, 0.62, nomad_luma);
        c = mix(c, c * vec3<f32>(1.035, 0.995, 0.925), shadow_tone * 0.075 * post_style_strength);
        c = c + vec3<f32>(paper) * 0.0035 * post_style_strength;
        c = mix(c, vec3<f32>(dot(c, vec3<f32>(0.2126, 0.7152, 0.0722))), edge_soften * 0.10);
        let sat_luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
        c = mix(vec3<f32>(sat_luma), c, saturation);
    }

    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / gamma));
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let xy = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0)
    );
    out.pos = vec4<f32>(xy[vertex_index], 0.0, 1.0);
    out.uv = out.pos.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    return out;
}

fn bright_part(c: vec3<f32>) -> vec3<f32> {
    let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let threshold = select(0.82, 1.05, UBO.cam_kind == 2u);
    let knee = 0.48;
    let soft = clamp((luma - threshold + knee) / max(knee, 0.001), 0.0, 1.0);
    let weight = soft * soft * (3.0 - 2.0 * soft);
    return c * weight;
}

@fragment
fn fs_extract(in: VsOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(scene_tex, 0));
    var c = textureSampleLevel(scene_tex, post_smp, in.uv, 0.0).rgb * 0.28;
    c += textureSampleLevel(scene_tex, post_smp, in.uv + texel * vec2<f32>( 1.5,  0.0), 0.0).rgb * 0.12;
    c += textureSampleLevel(scene_tex, post_smp, in.uv + texel * vec2<f32>(-1.5,  0.0), 0.0).rgb * 0.12;
    c += textureSampleLevel(scene_tex, post_smp, in.uv + texel * vec2<f32>( 0.0,  1.5), 0.0).rgb * 0.12;
    c += textureSampleLevel(scene_tex, post_smp, in.uv + texel * vec2<f32>( 0.0, -1.5), 0.0).rgb * 0.12;
    c += textureSampleLevel(scene_tex, post_smp, in.uv + texel * vec2<f32>( 2.5,  2.5), 0.0).rgb * 0.06;
    c += textureSampleLevel(scene_tex, post_smp, in.uv + texel * vec2<f32>(-2.5,  2.5), 0.0).rgb * 0.06;
    c += textureSampleLevel(scene_tex, post_smp, in.uv + texel * vec2<f32>( 2.5, -2.5), 0.0).rgb * 0.06;
    c += textureSampleLevel(scene_tex, post_smp, in.uv + texel * vec2<f32>(-2.5, -2.5), 0.0).rgb * 0.06;
    return vec4<f32>(bright_part(c), 1.0);
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let scene = textureSampleLevel(scene_tex, post_smp, in.uv, 0.0);
    let bloom_texel = 1.0 / vec2<f32>(textureDimensions(bloom_tex, 0));
    var bloom = textureSampleLevel(bloom_tex, post_smp, in.uv, 0.0).rgb * 0.34;
    bloom += textureSampleLevel(bloom_tex, post_smp, in.uv + bloom_texel * vec2<f32>( 2.0,  0.0), 0.0).rgb * 0.12;
    bloom += textureSampleLevel(bloom_tex, post_smp, in.uv + bloom_texel * vec2<f32>(-2.0,  0.0), 0.0).rgb * 0.12;
    bloom += textureSampleLevel(bloom_tex, post_smp, in.uv + bloom_texel * vec2<f32>( 0.0,  2.0), 0.0).rgb * 0.12;
    bloom += textureSampleLevel(bloom_tex, post_smp, in.uv + bloom_texel * vec2<f32>( 0.0, -2.0), 0.0).rgb * 0.12;
    bloom += textureSampleLevel(bloom_tex, post_smp, in.uv + bloom_texel * vec2<f32>( 5.0,  5.0), 0.0).rgb * 0.045;
    bloom += textureSampleLevel(bloom_tex, post_smp, in.uv + bloom_texel * vec2<f32>(-5.0,  5.0), 0.0).rgb * 0.045;
    bloom += textureSampleLevel(bloom_tex, post_smp, in.uv + bloom_texel * vec2<f32>( 5.0, -5.0), 0.0).rgb * 0.045;
    bloom += textureSampleLevel(bloom_tex, post_smp, in.uv + bloom_texel * vec2<f32>(-5.0, -5.0), 0.0).rgb * 0.045;

    let bloom_strength = select(0.34, 0.22, UBO.cam_kind == 2u);
    let color = scene.rgb + bloom * bloom_strength;
    return vec4<f32>(apply_post(color, in.pos), scene.a);
}
"#;

pub const SCENEVM_3D_ORGANIC_BILLBOARD_WGSL: &str = r#"
struct U {
    cam_pos: vec4<f32>,
    cam_fwd: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    sun_color_intensity: vec4<f32>,
    sun_dir_enabled: vec4<f32>,
    ambient_color_strength: vec4<f32>,
    sky_color: vec4<f32>,
    fog_color_density: vec4<f32>,
    shadow_light_right: vec4<f32>,
    shadow_light_up: vec4<f32>,
    shadow_light_fwd: vec4<f32>,
    shadow_light_center: vec4<f32>,
    shadow_light_extents: vec4<f32>,
    shadow_params: vec4<f32>,
    render_params: vec4<f32>,
    point_light_pos_intensity: array<vec4<f32>, 8>,
    point_light_color_range: array<vec4<f32>, 8>,
    point_light_count_pad: vec4<u32>,
    _pad_lights: vec4<u32>,
    fb_size: vec2<f32>,
    cam_vfov_deg: f32,
    cam_ortho_half_h: f32,
    cam_near: f32,
    cam_far: f32,
    cam_kind: u32,
    anim_counter: u32,
    _pad0: vec2<u32>,
    _pad_post_pre: vec2<u32>,
    post_params: vec4<f32>,
    post_color_adjust: vec4<f32>,
    post_style0: vec4<f32>,
    post_style1: vec4<f32>,
    avatar_highlight_params: vec4<f32>,
    _pad_tail: vec4<u32>,
    palette: array<vec4<f32>, 256>,
    palette_tile_indices: array<vec4<u32>, 64>,
    organic_params: vec4<u32>,
};

@group(0) @binding(0) var<uniform> UBO: U;

struct OrganicBillboardData {
    data: array<u32>,
};
@group(0) @binding(10) var<storage, read> organic_billboards: OrganicBillboardData;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) sprite_index: u32,
    @location(2) world_pos: vec3<f32>,
    @location(3) normal: vec3<f32>,
};

fn organic_word(index: u32) -> u32 {
    if (index >= arrayLength(&organic_billboards.data)) {
        return 0u;
    }
    return organic_billboards.data[index];
}

fn organic_f32(index: u32) -> f32 {
    return bitcast<f32>(organic_word(index));
}

fn camera_to_clip(world_pos: vec3<f32>) -> vec4<f32> {
    let rel = world_pos - UBO.cam_pos.xyz;
    let cx = dot(rel, UBO.cam_right.xyz);
    let cy = dot(rel, UBO.cam_up.xyz);
    let cz = dot(rel, UBO.cam_fwd.xyz);

    let near_z = max(UBO.cam_near, 0.0001);
    let far_z = max(UBO.cam_far, near_z + 0.0001);
    let aspect = max(UBO.fb_size.x / max(UBO.fb_size.y, 1.0), 0.0001);

    if (UBO.cam_kind == 0u) {
        let depth = clamp((cz - near_z) / (far_z - near_z), 0.0, 1.0);
        let half_h = max(UBO.cam_ortho_half_h, 0.0001);
        let half_w = max(half_h * aspect, 0.0001);
        return vec4<f32>(cx / half_w, cy / half_h, depth, 1.0);
    }

    var z = cz;
    if (abs(z) < 0.0001) {
        z = select(-0.0001, 0.0001, z >= 0.0);
    }
    let f = 1.0 / tan(radians(max(UBO.cam_vfov_deg, 1.0)) * 0.5);
    let a = far_z / (far_z - near_z);
    let b = (-near_z * far_z) / (far_z - near_z);
    return vec4<f32>(cx * (f / aspect), cy * f, a * z + b, z);
}

fn apply_post(color_in: vec3<f32>, pos: vec4<f32>) -> vec3<f32> {
    var c = max(color_in, vec3<f32>(0.0));
    if (UBO.post_color_adjust.z > 0.5) {
        return c;
    }
    if (UBO.post_params.x > 0.5) {
        c *= max(UBO.post_params.z, 0.0);
        let tone = u32(max(UBO.post_params.y, 0.0));
        if (tone == 1u) {
            c = c / (c + vec3<f32>(1.0));
        } else if (tone == 2u) {
            c = clamp((c * (2.51 * c + vec3<f32>(0.03))) /
                      (c * (2.43 * c + vec3<f32>(0.59)) + vec3<f32>(0.14)),
                      vec3<f32>(0.0), vec3<f32>(1.0));
        }
        c *= max(UBO.post_color_adjust.y, 0.0);
        let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
        c = mix(vec3<f32>(luma), c, max(UBO.post_color_adjust.x, 0.0));
    }
    return pow(c, vec3<f32>(1.0 / max(UBO.post_params.w, 0.001)));
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let instance_count = organic_word(2u);
    if (instance_count == 0u) {
        out.pos = vec4<f32>(0.0);
        out.uv = vec2<f32>(0.0);
        out.sprite_index = 0u;
        out.world_pos = vec3<f32>(0.0);
        out.normal = vec3<f32>(0.0, 1.0, 0.0);
        return out;
    }

    let instance_index = min(vertex_index / 6u, instance_count - 1u);
    let corner_index = vertex_index % 6u;
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0, -1.0)
    );
    let corner = corners[corner_index];
    let instance_base = organic_word(3u) + instance_index * 8u;
    let center = vec3<f32>(
        organic_f32(instance_base + 0u),
        organic_f32(instance_base + 1u),
        organic_f32(instance_base + 2u)
    );
    let width = max(organic_f32(instance_base + 3u), 0.001);
    let height = max(organic_f32(instance_base + 4u), 0.001);
    let sprite_index = organic_word(instance_base + 5u);
    let right = normalize(UBO.cam_right.xyz) * (width * 0.5);
    let up = normalize(UBO.cam_up.xyz) * (height * 0.5);
    let world_pos = center + right * corner.x + up * corner.y;

    out.pos = camera_to_clip(world_pos);
    out.uv = vec2<f32>(corner.x * 0.5 + 0.5, 0.5 - corner.y * 0.5);
    out.sprite_index = sprite_index;
    out.world_pos = world_pos;
    out.normal = normalize(cross(right, up));
    return out;
}

fn sample_sprite(sprite_index: u32, uv: vec2<f32>) -> vec4<f32> {
    let sprite_count = organic_word(1u);
    if (sprite_index >= sprite_count) {
        return vec4<f32>(0.0);
    }
    let meta_base = organic_word(4u) + sprite_index * 4u;
    let pixel_offset = organic_word(meta_base + 0u);
    let width = max(organic_word(meta_base + 1u), 1u);
    let height = max(organic_word(meta_base + 2u), 1u);
    let x = min(u32(floor(clamp(uv.x, 0.0, 0.9999) * f32(width))), width - 1u);
    let y = min(u32(floor(clamp(uv.y, 0.0, 0.9999) * f32(height))), height - 1u);
    let packed = organic_word(organic_word(5u) + pixel_offset + y * width + x);
    let r = f32((packed >> 0u) & 0xffu) * (1.0 / 255.0);
    let g = f32((packed >> 8u) & 0xffu) * (1.0 / 255.0);
    let b = f32((packed >> 16u) & 0xffu) * (1.0 / 255.0);
    let a = f32((packed >> 24u) & 0xffu) * (1.0 / 255.0);
    return vec4<f32>(r, g, b, a);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let texel = sample_sprite(in.sprite_index, in.uv);
    if (texel.a <= 0.5) {
        discard;
    }
    // These sprites are already baked as tiny pixel-art colors. Keep them readable
    // instead of pushing them through the full scene lighting path.
    var lit = texel.rgb;

    let fog_density = max(UBO.fog_color_density.w, 0.0);
    if (fog_density > 0.0) {
        let fog_dist = distance(in.world_pos, UBO.cam_pos.xyz);
        let fog_factor = clamp(exp(-(fog_density * fog_dist * fog_dist)), 0.0, 1.0);
        lit = mix(UBO.fog_color_density.xyz, lit, fog_factor);
    }

    return vec4<f32>(lit, 1.0);
}
"#;

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
    pub palette_remap_2d_start: u32,
    pub palette_remap_2d_end: u32,
    pub palette_remap_2d_blend: f32,
    pub palette_remap_2d_mode: PaletteRemap2DMode,
    pub raster3d_msaa_samples: u32,
    pub raster3d_avatar_highlight_params: Vec4<f32>,
    pub raster3d_post_style0: Vec4<f32>,
    pub raster3d_post_style1: Vec4<f32>,
    // --- Programmable compute shader sources
    pub source2d: String,
    pub viewport_rect2d: Option<[f32; 4]>, // Optional viewport rect for 2D shader (x, y, w, h)
    pub source3d: String,
    pub source_sdf: String,
    pub sdf_data: Vec<[f32; 4]>,
    pub sdf_data_dirty: bool,
    pub material_table: Vec<[f32; 4]>,
    pub material_table_dirty: bool,
    raster3d_paint_overlay: Option<Raster3DPaintOverlayData>,
    raster3d_paint_overlay_dirty: bool,
    pub palette: [[f32; 4]; 256],
    pub palette_dirty: bool,

    pub transform2d: Mat3<f32>,
    pub transform3d: Mat4<f32>,

    pub lights: FxHashMap<GeoId, Light>,
    dynamic_objects: Vec<DynamicObject>,
    dynamic_avatar_objects: FxHashMap<GeoId, DynamicObject>,
    dynamic_avatar_data: FxHashMap<GeoId, DynamicAvatarData>,
    organic_billboards: OrganicBillboardData,

    pub current_layer: i32,

    // Scene-wide 3D acceleration via BVH
    pub bvh_leaf_size: u32,
    pub scene_accel: SceneAccel,
    pub accel_dirty: bool,
    cached_v3: Vec<Vert3DPod>,
    cached_i3: Vec<u32>,
    cached_tri_visibility: Vec<u32>, // Per-triangle visibility bitmask (1 bit per triangle)
    cached_tri_geo_ids: Vec<GeoId>,
    cached_static_v3: Vec<Vert3DPod>,
    cached_static_i3: Vec<u32>,
    cached_static_tri_visibility: Vec<bool>,
    cached_static_tri_geo_ids: Vec<GeoId>,
    cached_static_raster_visible_indices: Vec<u32>,
    cached_static_raster_opaque_indices: Vec<u32>,
    cached_static_raster_transparent_indices: Vec<u32>,
    cached_static_raster_particle_indices: Vec<u32>,
    cached_static_raster_camera_key: [f32; 6],
    cached_static_raster_indices_valid: bool,
    cached_line3d: Vec<Line3DPod>,
    line3d_dirty: bool,
    visibility_dirty: bool, // True when only visibility changed (no BVH rebuild needed)
    geometry3d_dirty: bool, // True when 3D vertex attributes changed (e.g. opacity)
    geometry2d_dirty: bool,
    cached_v2: Vec<Vert2DPod>,
    cached_i2: Vec<u32>,
    cached_static_v2: Vec<Vert2DPod>,
    cached_static_i2: Vec<u32>,
    cached_static_tile_bins: Vec<TileBinPod>,
    cached_static_tile_tris: Vec<u32>,
    cached_static_fb_size_2d: (u32, u32),
    cached_tile_bins: Vec<TileBinPod>,
    cached_tile_tris: Vec<u32>,
    cached_fb_size_2d: (u32, u32),
    cached_tile_anim_meta: Vec<TileAnimMetaPod>,
    cached_tile_frame_data: Vec<TileFramePod>,
    cached_atlas_layout_version: u64,
    cached_tile_emissive_summaries: Vec<TileEmissiveSummary>,
    cached_tile_emissive_content_version: u64,
    tile_gpu_dirty: bool,
    cached_scene_data_hash: u64,
    irradiance_grid_dirty: bool,
    cached_irradiance_grid_data: Vec<[f32; 4]>,
    raster_had_dynamics_last_frame: bool,
    organic_visible: bool,

    // Camera
    pub camera3d: Camera3D,

    pub enabled: bool,
    layer_index: usize,
    activity_logging: bool,
}

impl VM {
    fn palette_tile_indices_uniform(&self) -> [[u32; 4]; 64] {
        let mut out = [[0u32; 4]; 64];
        for index in 0..256u16 {
            let tile_uuid = palette_index_tile_uuid(index);
            let tile_index = self.shared_atlas.tile_index(&tile_uuid).unwrap_or(0);
            out[(index as usize) / 4][(index as usize) % 4] = tile_index;
        }
        out
    }

    #[inline]
    fn raster3d_effective_samples(&self) -> u32 {
        if self.raster3d_msaa_samples == 0 {
            1
        } else {
            self.raster3d_msaa_samples
        }
    }

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

    pub fn debug_stats(&self) -> VMDebugStats {
        let mut stats = VMDebugStats {
            chunks: self.chunks_map.len(),
            dynamics: self.dynamic_objects.len(),
            lights: self.lights.len(),
            cached_v3: self.cached_v3.len(),
            cached_i3: self.cached_i3.len(),
            accel_dirty: self.accel_dirty,
            visibility_dirty: self.visibility_dirty,
            geometry3d_dirty: self.geometry3d_dirty,
            geometry2d_dirty: self.geometry2d_dirty,
            ..Default::default()
        };

        for ch in self.chunks_map.values() {
            stats.polys2d += ch.polys_map.len();
            stats.lines2d += ch.lines2d_px.len();
            for polys in ch.polys3d_map.values() {
                stats.polys3d += polys.len();
                for poly in polys {
                    stats.tris3d += poly.indices.len();
                }
            }
        }
        stats
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

    /// Configure Raster3D avatar readability boost parameters.
    /// x=lift, y=fill, z=rim, w=enabled (0/1).
    pub fn set_raster3d_avatar_highlight_params(&mut self, params: Vec4<f32>) {
        self.raster3d_avatar_highlight_params = params;
    }

    /// Configure Raster3D stylized post controls.
    /// style0: x=grit, y=posterize, z=palette_bias, w=shadow_lift.
    /// style1: x=edge_soften, yzw=reserved.
    pub fn set_raster3d_post_style_params(&mut self, style0: Vec4<f32>, style1: Vec4<f32>) {
        self.raster3d_post_style0 = style0;
        self.raster3d_post_style1 = style1;
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
                    multiview_mask: None,
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
                    multiview_mask: None,
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
                multiview_mask: None,
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
        if object.kind == DynamicKind::Mesh {
            if object.mesh_vertices.is_empty() || object.mesh_indices.len() < 3 {
                return;
            }
            self.dynamic_objects.push(object);
            return;
        }

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

    fn geo_id_sort_key(id: GeoId) -> (u8, u64, u64) {
        match id {
            GeoId::Unknown(v) => (0, v as u64, 0),
            GeoId::Vertex(v) => (1, v as u64, 0),
            GeoId::Linedef(v) => (2, v as u64, 0),
            GeoId::Sector(v) => (3, v as u64, 0),
            GeoId::Character(v) => (4, v as u64, 0),
            GeoId::Item(v) => (5, v as u64, 0),
            GeoId::Light(v) => (6, v as u64, 0),
            GeoId::ItemLight(v) => (7, v as u64, 0),
            GeoId::Triangle(v) => (8, v as u64, 0),
            GeoId::Terrain(x, y) => (9, x as i64 as u64, y as i64 as u64),
            GeoId::GeometryObject(id) => {
                let raw = id.as_u128();
                (10, (raw >> 64) as u64, raw as u64)
            }
            GeoId::Hole(host, profile) => (11, host as u64, profile as u64),
            GeoId::Gizmo(v) => (12, v as u64, 0),
        }
    }

    fn dynamic_object_order(a: &DynamicObject, b: &DynamicObject) -> Ordering {
        b.layer
            .cmp(&a.layer)
            .then_with(|| (a.kind as u32).cmp(&(b.kind as u32)))
            .then_with(|| Self::geo_id_sort_key(a.id).cmp(&Self::geo_id_sort_key(b.id)))
    }

    fn sorted_dynamic_objects(&self) -> Vec<&DynamicObject> {
        let mut dynamic_objs: Vec<&DynamicObject> = self
            .dynamic_objects
            .iter()
            .chain(self.dynamic_avatar_objects.values())
            .collect();
        dynamic_objs.sort_by(|a, b| Self::dynamic_object_order(a, b));
        dynamic_objs
    }

    fn avatar_meta_indices_for_objects(
        &self,
        dynamic_objs: &[&DynamicObject],
    ) -> FxHashMap<GeoId, u32> {
        let mut avatar_meta_indices: FxHashMap<GeoId, u32> = FxHashMap::default();
        let mut avatar_meta_count: u32 = 0;
        for obj in dynamic_objs.iter().copied() {
            if obj.kind != DynamicKind::BillboardAvatar || avatar_meta_indices.contains_key(&obj.id)
            {
                continue;
            }
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
            avatar_meta_indices.insert(obj.id, avatar_meta_count);
            avatar_meta_count += 1;
        }
        avatar_meta_indices
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
                let tile_index2 = if let Some(tid2) = poly.tile_id2 {
                    self.shared_atlas.tile_index(&tid2).unwrap_or(tile_index)
                } else {
                    tile_index
                };
                let has_valid_blend =
                    poly.tile_id2.is_some() && poly.blend_weights.len() == poly.vertices.len();

                let base = verts_flat.len() as u32;

                for (i, v) in poly.vertices.iter().enumerate() {
                    let local_p = poly.transform * Vec3::new(v[0], v[1], 1.0);
                    let world_p = self.transform2d * local_p;

                    let base_uv = poly.uvs[i];

                    verts_flat.push(Vert2DPod {
                        pos: [world_p.x, world_p.y],
                        uv: [base_uv[0], base_uv[1]],
                        tile_index,
                        tile_index2,
                        blend_factor: if has_valid_blend {
                            poly.blend_weights[i].clamp(0.0, 1.0)
                        } else {
                            0.0
                        },
                        _pad0: 0,
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
                                tile_index2: tile_index,
                                blend_factor: 0.0,
                                _pad0: 0,
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
        let dynamic_objs = self.sorted_dynamic_objects();
        for obj in dynamic_objs {
            match obj.kind {
                DynamicKind::BillboardTile | DynamicKind::ParticleBillboard => {
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
                        params: [
                            tile_index,
                            obj.kind as u32,
                            obj.opacity.to_bits(),
                            obj.alpha_mode as u32,
                        ],
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
                        params: [
                            avatar_index,
                            obj.kind as u32,
                            obj.opacity.to_bits(),
                            obj.alpha_mode as u32,
                        ],
                    });
                }
                DynamicKind::Mesh => {}
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

    fn upload_scene_data_ssbo(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        use wgpu::util::DeviceExt;

        let scene_data = self.build_scene_data_blob();
        let mut hasher = rustc_hash::FxHasher::default();
        hasher.write(&scene_data);
        let scene_data_hash = hasher.finish();
        let needs_recreate = if let Some(g) = self.gpu.as_ref() {
            g.scene_data_ssbo.is_none() || g.scene_data_ssbo_size != scene_data.len()
        } else {
            true
        };

        if !needs_recreate && self.cached_scene_data_hash == scene_data_hash {
            return;
        }

        let g = self.gpu.as_mut().unwrap();
        if needs_recreate {
            g.scene_data_ssbo = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("vm-scene-data-ssbo"),
                    contents: &scene_data,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                },
            ));
            g.scene_data_ssbo_size = scene_data.len();
        } else if let Some(buf) = g.scene_data_ssbo.as_ref() {
            queue.write_buffer(buf, 0, &scene_data);
        }
        self.cached_scene_data_hash = scene_data_hash;
    }

    fn build_organic_billboard_words(&self) -> Vec<u32> {
        let chunk_sprite_count: usize = self
            .chunks_map
            .values()
            .map(|chunk| chunk.organic_billboard_sprites.len())
            .sum();
        let chunk_instance_count: usize = self
            .chunks_map
            .values()
            .map(|chunk| chunk.organic_billboard_instances.len())
            .sum();
        let sprite_count = (self.organic_billboards.sprites.len() + chunk_sprite_count)
            .min(u32::MAX as usize) as u32;
        let instance_count = (self.organic_billboards.instances.len() + chunk_instance_count)
            .min(u32::MAX as usize) as u32;
        let instance_offset_words = 8u32;
        let sprite_meta_offset_words = instance_offset_words + instance_count * 8;
        let pixel_offset_words = sprite_meta_offset_words + sprite_count * 4;
        let mut words = vec![
            0x4F42_494Cu32,
            sprite_count,
            instance_count,
            instance_offset_words,
            sprite_meta_offset_words,
            pixel_offset_words,
            0,
            0,
        ];

        let mut sprite_base = self.organic_billboards.sprites.len() as u32;
        for instance in self.organic_billboards.instances.iter() {
            words.extend_from_slice(&[
                instance.center[0].to_bits(),
                instance.center[1].to_bits(),
                instance.center[2].to_bits(),
                instance.width.to_bits(),
                instance.height.to_bits(),
                instance.sprite_index,
                instance.flags,
                0,
            ]);
        }
        for chunk in self.chunks_map.values() {
            for instance in &chunk.organic_billboard_instances {
                words.extend_from_slice(&[
                    instance.center[0].to_bits(),
                    instance.center[1].to_bits(),
                    instance.center[2].to_bits(),
                    instance.width.to_bits(),
                    instance.height.to_bits(),
                    sprite_base.saturating_add(instance.sprite_index),
                    instance.flags,
                    0,
                ]);
            }
            sprite_base = sprite_base.saturating_add(chunk.organic_billboard_sprites.len() as u32);
        }

        let mut pixel_words = Vec::new();
        for sprite in &self.organic_billboards.sprites {
            let offset_pixels = pixel_words.len() as u32;
            words.extend_from_slice(&[offset_pixels, sprite.width, sprite.height, 0]);
            for px in sprite.rgba.chunks_exact(4) {
                pixel_words.push(u32::from_le_bytes([px[0], px[1], px[2], px[3]]));
            }
        }
        for chunk in self.chunks_map.values() {
            for sprite in &chunk.organic_billboard_sprites {
                let offset_pixels = pixel_words.len() as u32;
                words.extend_from_slice(&[offset_pixels, sprite.width, sprite.height, 0]);
                for px in sprite.rgba.chunks_exact(4) {
                    pixel_words.push(u32::from_le_bytes([px[0], px[1], px[2], px[3]]));
                }
            }
        }
        words.extend_from_slice(&pixel_words);

        if words.is_empty() {
            words.push(0);
        }
        words
    }

    fn upload_organic_billboard_ssbo(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue) {
        use wgpu::util::DeviceExt;

        let organic_words = self.build_organic_billboard_words();
        let organic_bytes: &[u8] = bytemuck::cast_slice(&organic_words);
        let organic_byte_len = organic_bytes.len().max(std::mem::size_of::<u32>());
        let instance_count = self.organic_billboards.instances.len().saturating_add(
            self.chunks_map
                .values()
                .map(|chunk| chunk.organic_billboard_instances.len())
                .sum::<usize>(),
        ) as u32;
        let needs_recreate = if let Some(g) = self.gpu.as_ref() {
            self.organic_billboards.dirty
                || g.organic_billboard_ssbo.is_none()
                || g.organic_billboard_ssbo_size != organic_byte_len
        } else {
            true
        };

        if !needs_recreate {
            if let Some(g) = self.gpu.as_mut() {
                g.organic_billboard_count = instance_count;
            }
            return;
        }

        let g = self.gpu.as_mut().unwrap();
        g.organic_billboard_ssbo = Some(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("vm-organic-billboards"),
                contents: organic_bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            },
        ));
        g.organic_billboard_ssbo_size = organic_byte_len;
        g.organic_billboard_count = instance_count;
        self.organic_billboards.dirty = false;
    }

    #[inline]
    fn mark_irradiance_grid_dirty(&mut self) {
        self.irradiance_grid_dirty = true;
    }

    #[inline]
    pub fn mark_all_geometry_dirty(&mut self) {
        self.geometry2d_dirty = true;
        self.accel_dirty = true;
        self.line3d_dirty = true;
        self.mark_irradiance_grid_dirty();
        self.cached_static_v3.clear();
        self.cached_static_i3.clear();
        self.cached_static_tri_visibility.clear();
        self.cached_static_tri_geo_ids.clear();
        self.cached_static_raster_visible_indices.clear();
        self.cached_static_raster_opaque_indices.clear();
        self.cached_static_raster_transparent_indices.clear();
        self.cached_static_raster_particle_indices.clear();
        self.cached_static_raster_indices_valid = false;
        self.cached_static_v2.clear();
        self.cached_static_i2.clear();
        self.cached_static_tile_bins.clear();
        self.cached_static_tile_tris.clear();
        self.cached_static_fb_size_2d = (0, 0);
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

    fn default_material_table() -> Vec<[f32; 4]> {
        let mut rows = vec![[0.0; 4]; DEFAULT_MATERIAL_TABLE_ROW_COUNT];
        for id in 0..MATERIAL_TABLE_ID_COUNT {
            let base = id * MATERIAL_TABLE_ROWS_PER_ID;
            rows[base] = [0.5, 0.0, 1.0, 0.0];
            rows[base + 1] = [0.0, 0.0, 0.0, 0.12];
            rows[base + 2] = [0.08, 0.0, 0.0, 0.0];
        }
        rows
    }

    fn normalize_material_table(mut rows: Vec<[f32; 4]>) -> Vec<[f32; 4]> {
        let mut defaults = Self::default_material_table();
        let copy_len = rows.len().min(defaults.len());
        defaults[..copy_len].copy_from_slice(&rows[..copy_len]);
        rows.clear();
        defaults
    }

    fn upload_material_table_ssbo(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.gpu.is_none() {
            return;
        }
        if self.material_table.is_empty() {
            self.material_table = Self::default_material_table();
            self.material_table_dirty = true;
        }
        let byte_len = (self.material_table.len() * std::mem::size_of::<[f32; 4]>())
            .max(std::mem::size_of::<[f32; 4]>());
        let needs_recreate = self
            .gpu
            .as_ref()
            .map(|g| g.material_table_ssbo.is_none() || g.material_table_ssbo_size != byte_len)
            .unwrap_or(true);
        if !self.material_table_dirty && !needs_recreate {
            return;
        }
        let bytes: &[u8] = bytemuck::cast_slice(&self.material_table);
        let g = self.gpu.as_mut().unwrap();
        if needs_recreate {
            use wgpu::util::DeviceExt;
            g.material_table_ssbo = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("vm-raster3d-material-table"),
                    contents: bytes,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                },
            ));
            g.material_table_ssbo_size = byte_len;
        } else if let Some(buf) = g.material_table_ssbo.as_ref() {
            queue.write_buffer(buf, 0, bytes);
        }
        self.material_table_dirty = false;
    }

    fn upload_raster3d_paint_overlay(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.gpu.is_none() {
            return;
        }

        let fallback_color = DEFAULT_PAINT_COLOR_PIXEL;
        let fallback_material = DEFAULT_PAINT_MATERIAL_PIXEL;
        let (width, height, color_rgba, material_rgba) =
            if let Some(overlay) = self.raster3d_paint_overlay.as_ref() {
                (
                    overlay.width.max(1),
                    overlay.height.max(1),
                    overlay.color_rgba.as_slice(),
                    overlay.material_rgba.as_slice(),
                )
            } else {
                (
                    1,
                    1,
                    fallback_color.as_slice(),
                    fallback_material.as_slice(),
                )
            };

        let recreate = self
            .gpu
            .as_ref()
            .map(|g| {
                g.raster3d_paint_color_tex.is_none()
                    || g.raster3d_paint_material_tex.is_none()
                    || g.raster3d_paint_tex_size != (width, height)
            })
            .unwrap_or(true);

        if !recreate && !self.raster3d_paint_overlay_dirty {
            return;
        }

        let g = self.gpu.as_mut().unwrap();
        if recreate {
            let desc = |label: &'static str| wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            };
            let color_tex = device.create_texture(&desc("vm-raster3d-paint-color"));
            let material_tex = device.create_texture(&desc("vm-raster3d-paint-material"));
            g.raster3d_paint_color_view =
                Some(color_tex.create_view(&wgpu::TextureViewDescriptor::default()));
            g.raster3d_paint_material_view =
                Some(material_tex.create_view(&wgpu::TextureViewDescriptor::default()));
            g.raster3d_paint_color_tex = Some(color_tex);
            g.raster3d_paint_material_tex = Some(material_tex);
            g.raster3d_paint_tex_size = (width, height);
        }

        let layout = wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        };
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        if let Some(tex) = g.raster3d_paint_color_tex.as_ref() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                color_rgba,
                layout,
                extent,
            );
        }
        if let Some(tex) = g.raster3d_paint_material_tex.as_ref() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                material_rgba,
                layout,
                extent,
            );
        }
        self.raster3d_paint_overlay_dirty = false;
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
            palette_remap_2d_start: 0,
            palette_remap_2d_end: 0,
            palette_remap_2d_blend: 0.0,
            palette_remap_2d_mode: PaletteRemap2DMode::Disabled,
            raster3d_msaa_samples: 4,
            raster3d_avatar_highlight_params: Vec4::new(1.12, 0.20, 0.18, 1.0),
            raster3d_post_style0: Vec4::new(0.0, 0.0, 0.0, 0.0),
            raster3d_post_style1: Vec4::new(0.0, 0.0, 0.0, 0.0),
            source2d,
            viewport_rect2d: None,
            source3d,
            source_sdf,
            sdf_data: Vec::new(),
            sdf_data_dirty: true,
            material_table: Self::default_material_table(),
            material_table_dirty: true,
            raster3d_paint_overlay: None,
            raster3d_paint_overlay_dirty: true,
            transform2d: Mat3::identity(),
            transform3d: Mat4::identity(),
            lights: FxHashMap::default(),
            dynamic_objects: Vec::new(),
            dynamic_avatar_objects: FxHashMap::default(),
            dynamic_avatar_data: FxHashMap::default(),
            organic_billboards: OrganicBillboardData::default(),
            current_layer: 0,
            scene_accel: SceneAccel::default(),
            accel_dirty: true,
            bvh_leaf_size: 8,
            cached_v3: Vec::new(),
            cached_i3: Vec::new(),
            cached_tri_visibility: Vec::new(),
            cached_tri_geo_ids: Vec::new(),
            cached_static_v3: Vec::new(),
            cached_static_i3: Vec::new(),
            cached_static_tri_visibility: Vec::new(),
            cached_static_tri_geo_ids: Vec::new(),
            cached_static_raster_visible_indices: Vec::new(),
            cached_static_raster_opaque_indices: Vec::new(),
            cached_static_raster_transparent_indices: Vec::new(),
            cached_static_raster_particle_indices: Vec::new(),
            cached_static_raster_camera_key: [0.0; 6],
            cached_static_raster_indices_valid: false,
            cached_line3d: Vec::new(),
            line3d_dirty: true,
            visibility_dirty: false,
            geometry3d_dirty: false,
            geometry2d_dirty: true,
            cached_v2: Vec::new(),
            cached_i2: Vec::new(),
            cached_static_v2: Vec::new(),
            cached_static_i2: Vec::new(),
            cached_static_tile_bins: Vec::new(),
            cached_static_tile_tris: Vec::new(),
            cached_static_fb_size_2d: (0, 0),
            cached_tile_bins: Vec::new(),
            cached_tile_tris: Vec::new(),
            cached_fb_size_2d: (0, 0),
            cached_tile_anim_meta: Vec::new(),
            cached_tile_frame_data: Vec::new(),
            cached_atlas_layout_version: 0,
            cached_tile_emissive_summaries: Vec::new(),
            cached_tile_emissive_content_version: u64::MAX,
            tile_gpu_dirty: true,
            cached_scene_data_hash: 0,
            irradiance_grid_dirty: true,
            cached_irradiance_grid_data: Self::disabled_irradiance_grid_data(),
            raster_had_dynamics_last_frame: false,
            organic_visible: true,
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
                    self.mark_irradiance_grid_dirty();
                }
            }
            Atom::SetGeoOpacity { id, opacity } => {
                let clamped = opacity.clamp(0.0, 1.0);
                let mut dirty_3d = false;
                for ch in self.chunks_map.values_mut() {
                    if let Some(p3_vec) = ch.polys3d_map.get_mut(&id) {
                        for p3 in p3_vec.iter_mut() {
                            if (p3.opacity - clamped).abs() > 1e-6 {
                                p3.opacity = clamped;
                                dirty_3d = true;
                            }
                        }
                    }
                }
                if dirty_3d {
                    self.geometry3d_dirty = true;
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
                    *mf = normalize_material_frame(std::mem::take(mf), need);
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
            Atom::SetMaterialTable(rows) => {
                self.material_table = Self::normalize_material_table(rows);
                self.material_table_dirty = true;
            }
            Atom::SetRaster3DPaintOverlay {
                width,
                height,
                color_rgba,
                material_rgba,
            } => {
                let expected = width as usize * height as usize * 4;
                if width == 0
                    || height == 0
                    || color_rgba.len() != expected
                    || material_rgba.len() != expected
                {
                    if self.raster3d_paint_overlay.is_some() {
                        self.raster3d_paint_overlay = None;
                        self.raster3d_paint_overlay_dirty = true;
                    }
                } else {
                    let next = Raster3DPaintOverlayData {
                        width,
                        height,
                        color_rgba,
                        material_rgba,
                    };
                    let changed = self
                        .raster3d_paint_overlay
                        .as_ref()
                        .map(|current| {
                            current.width != next.width
                                || current.height != next.height
                                || current.color_rgba != next.color_rgba
                                || current.material_rgba != next.material_rgba
                        })
                        .unwrap_or(true);
                    if changed {
                        self.raster3d_paint_overlay = Some(next);
                        self.raster3d_paint_overlay_dirty = true;
                    }
                }
            }
            Atom::ClearRaster3DPaintOverlay => {
                if self.raster3d_paint_overlay.is_some() {
                    self.raster3d_paint_overlay = None;
                    self.raster3d_paint_overlay_dirty = true;
                }
            }
            Atom::SetTileMaterialFrames { id, frames } => {
                self.shared_atlas.with_tile_mut(&id, move |tile| {
                    let need = (tile.w as usize) * (tile.h as usize) * 4;
                    let mut mats: Vec<Vec<u8>> = frames
                        .into_iter()
                        .map(|f| normalize_material_frame(f, need))
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
            Atom::AddLine3D { line } => {
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
                    .add_line3d(line);
                self.line3d_dirty = true;
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
                self.line3d_dirty = true;
                self.organic_billboards.dirty = true;
                self.mark_2d_dirty();
            }
            Atom::AddChunk { id, chunk } => {
                // Insert or replace the chunk as-is; caller controls current_chunk separately
                self.chunks_map.insert(id, chunk);
                self.accel_dirty = true;
                self.line3d_dirty = true;
                self.organic_billboards.dirty = true;
                self.mark_2d_dirty();
            }
            Atom::RemoveChunk { id } => {
                let was_current = self.current_chunk == Some(id);
                self.chunks_map.remove(&id);
                if was_current {
                    self.current_chunk = None;
                }
                self.accel_dirty = true;
                self.line3d_dirty = true;
                self.organic_billboards.dirty = true;
                self.mark_2d_dirty();
            }
            Atom::RemoveChunkAt { origin } => {
                let ids = self
                    .chunks_map
                    .iter()
                    .filter_map(|(id, ch)| (ch.origin == origin).then_some(*id))
                    .collect::<Vec<_>>();
                for id in ids {
                    let was_current = self.current_chunk == Some(id);
                    self.chunks_map.remove(&id);
                    if was_current {
                        self.current_chunk = None;
                    }
                }
                self.accel_dirty = true;
                self.line3d_dirty = true;
                self.organic_billboards.dirty = true;
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
                self.line3d_dirty = true;
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
                self.organic_billboards = OrganicBillboardData {
                    dirty: true,
                    ..OrganicBillboardData::default()
                };
            }
            Atom::ClearTiles => {
                // Clear tile-related state and atlas pixels; keep scene/chunks
                self.shared_atlas.clear();
                self.mark_all_geometry_dirty();
                self.dynamic_objects.clear();
                self.dynamic_avatar_objects.clear();
                self.organic_billboards = OrganicBillboardData {
                    dirty: true,
                    ..OrganicBillboardData::default()
                };
            }
            Atom::ClearGeometry => {
                // Remove all chunks and unset current chunk; keep tiles/atlas/state
                self.chunks_map.clear();
                self.current_chunk = None;
                self.accel_dirty = true;
                self.mark_2d_dirty();
                self.dynamic_objects.clear();
                self.dynamic_avatar_objects.clear();
                self.organic_billboards = OrganicBillboardData {
                    dirty: true,
                    ..OrganicBillboardData::default()
                };
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
            Atom::SetPaletteRemap2D {
                start_index,
                end_index,
                mode,
            } => {
                self.palette_remap_2d_start = start_index.min(255);
                self.palette_remap_2d_end = end_index.min(255);
                self.palette_remap_2d_mode = mode;
            }
            Atom::SetPaletteRemap2DBlend(blend) => {
                self.palette_remap_2d_blend = blend.clamp(0.0, 1.0);
            }
            Atom::SetRaster3DMsaaSamples(samples) => {
                // Use only WebGPU-guaranteed sample counts for RGBA8 color targets.
                let s = if samples == 0 { 0 } else { 4 };
                if self.raster3d_msaa_samples != s {
                    self.raster3d_msaa_samples = s;
                    if let Some(g) = self.gpu.as_mut() {
                        g.raster3d_pipeline = None;
                        g.raster3d_alpha_pipeline = None;
                        g.raster3d_particle_pipeline = None;
                        g.raster3d_organic_billboard_pipeline = None;
                        g.raster3d_shadow_pipeline = None;
                        g.u_raster3d_bgl = None;
                        g.u_raster3d_shadow_bgl = None;
                        g.u_raster3d_bg = None;
                        g.u_raster3d_shadow_bg = None;
                    }
                }
            }
            Atom::SetRenderMode(m) => {
                self.render_mode = m;
            }
            Atom::AddLight { id, light } => {
                self.lights.insert(id, light);
                self.mark_irradiance_grid_dirty();
            }
            Atom::RemoveLight { id } => {
                self.lights.remove(&id);
                self.mark_irradiance_grid_dirty();
            }
            Atom::ClearLights => {
                self.lights.clear();
                self.mark_irradiance_grid_dirty();
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
            Atom::SetOrganicVisible { visible } => {
                self.organic_visible = visible;
            }
            Atom::SetOrganicBillboards { sprites, instances } => {
                let sanitized_sprites: Vec<OrganicBillboardSprite> = sprites
                    .into_iter()
                    .filter(|sprite| {
                        sprite.width > 0
                            && sprite.height > 0
                            && sprite.width <= 256
                            && sprite.height <= 256
                            && sprite.rgba.len()
                                == sprite.width as usize * sprite.height as usize * 4
                    })
                    .collect();
                let sprite_count = sanitized_sprites.len() as u32;
                let sanitized_instances: Vec<OrganicBillboardInstance> = instances
                    .into_iter()
                    .filter(|instance| {
                        instance.sprite_index < sprite_count
                            && instance.width.is_finite()
                            && instance.width > 0.0
                            && instance.height.is_finite()
                            && instance.height > 0.0
                            && instance.center.iter().all(|v| v.is_finite())
                    })
                    .collect();
                self.organic_billboards = OrganicBillboardData {
                    sprites: sanitized_sprites,
                    instances: sanitized_instances,
                    dirty: true,
                };
            }
            Atom::ClearOrganicBillboards => {
                self.organic_billboards = OrganicBillboardData {
                    dirty: true,
                    ..OrganicBillboardData::default()
                };
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
            bind_group_layouts: &[Some(&globals_bgl), Some(&atlas_bgl)],
            immediate_size: 0,
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
            multiview_mask: None,
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
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let sampler_linear: wgpu::Sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vm-atlas-sampler-repeat-linear"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        // Raster 3D sampler: keep pixel-art look up close (nearest mag), smooth only minification.
        let sampler_raster: wgpu::Sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vm-atlas-sampler-repeat-raster"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 1,
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
            sampler_raster,
            compute2d_pipeline: None,
            compute3d_pipeline: None,
            compute_sdf_pipeline: None,
            raster2d_pipeline: None,
            raster3d_pipeline: None,
            raster3d_alpha_pipeline: None,
            raster3d_particle_pipeline: None,
            raster3d_line_pipeline: None,
            raster3d_organic_billboard_pipeline: None,
            raster3d_shadow_pipeline: None,
            raster3d_bloom_extract_pipeline: None,
            raster3d_bloom_composite_pipeline: None,
            u2d_buf: None,
            u3d_buf: None,
            u_sdf_buf: None,
            u_raster2d_buf: None,
            u_raster3d_buf: None,
            u2d_bgl: None,
            u3d_bgl: None,
            u_sdf_bgl: None,
            u_raster2d_bgl: None,
            u_raster3d_bgl: None,
            u_raster3d_shadow_bgl: None,
            u_raster3d_post_bgl: None,
            u2d_bg: None,
            u3d_bg: None,
            u_sdf_bg: None,
            u_raster2d_bg: None,
            u_raster3d_bg: None,
            u_raster3d_shadow_bg: None,
            v2d_ssbo: None,
            i2d_ssbo: None,
            v3d_ssbo: None,
            i3d_ssbo: None,
            v3d_ssbo_capacity: 0,
            i3d_ssbo_capacity: 0,
            i3d_raster: None,
            i3d_raster_count: 0,
            i3d_raster_capacity: 0,
            i3d_raster_opaque: None,
            i3d_raster_opaque_count: 0,
            i3d_raster_opaque_capacity: 0,
            i3d_raster_transparent: None,
            i3d_raster_transparent_count: 0,
            i3d_raster_transparent_capacity: 0,
            i3d_raster_particles: None,
            i3d_raster_particles_count: 0,
            i3d_raster_particles_capacity: 0,
            line3d_raster: None,
            line3d_raster_count: 0,
            line3d_raster_capacity: 0,
            shadow_sampler_compare: None,
            raster3d_shadow_tex: None,
            raster3d_shadow_view: None,
            raster3d_shadow_res: 0,
            raster3d_scene_tex: None,
            raster3d_scene_view: None,
            raster3d_bloom_tex: None,
            raster3d_bloom_view: None,
            raster3d_bloom_size: (0, 0),
            raster3d_msaa_color_tex: None,
            raster3d_msaa_color_view: None,
            raster3d_depth_tex: None,
            raster3d_depth_view: None,
            raster3d_fb_size: (0, 0),
            raster3d_sample_count: 0,
            tile_bins: None,
            tile_tris: None,
            tile_meta_ssbo: None,
            tile_frames_ssbo: None,
            scene_data_ssbo: None,
            scene_data_ssbo_size: 0,
            organic_billboard_ssbo: None,
            organic_billboard_ssbo_size: 0,
            organic_billboard_count: 0,
            irradiance_grid_ssbo: None,
            irradiance_grid_ssbo_size: 0,
            material_table_ssbo: None,
            material_table_ssbo_size: 0,
            raster3d_paint_color_tex: None,
            raster3d_paint_color_view: None,
            raster3d_paint_material_tex: None,
            raster3d_paint_material_view: None,
            raster3d_paint_tex_size: (0, 0),
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
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Compute2DUniforms>() as u64,
                        ),
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
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Compute3DUniforms>() as u64,
                        ),
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
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Grid3DHeader>() as u64,
                        ),
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
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<
                            ComputeSdfUniforms,
                        >() as u64),
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
                        bind_group_layouts: &[Some(g.u2d_bgl.as_ref().unwrap())],
                        immediate_size: 0,
                    }),
                ),
                module: &cs2d,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            });
            g.compute2d_pipeline = Some(pl2d);
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
                        bind_group_layouts: &[Some(g.u_sdf_bgl.as_ref().unwrap())],
                        immediate_size: 0,
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

    /// Lazily compiles the Compute3D pipeline only when a Compute3D draw is requested.
    fn ensure_compute3d_pipeline(&mut self, device: &wgpu::Device) -> crate::SceneVMResult<()> {
        if self.gpu.is_none() {
            self.init_gpu(device)?;
        }
        if self.gpu.as_ref().and_then(|g| g.u3d_bgl.as_ref()).is_none() {
            self.init_compute(device)?;
        }

        let g = self.gpu.as_mut().unwrap();
        if g.compute3d_pipeline.is_some() {
            return Ok(());
        }

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
                    bind_group_layouts: &[Some(g.u3d_bgl.as_ref().unwrap())],
                    immediate_size: 0,
                }),
            ),
            module: &cs3d,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        g.compute3d_pipeline = Some(pl3d);

        Ok(())
    }

    fn init_raster2d(&mut self, device: &wgpu::Device) -> crate::SceneVMResult<()> {
        if self.gpu.is_none() {
            self.init_gpu(device)?;
        }
        self.upload_tile_metadata_to_gpu(device);
        let g = self.gpu.as_mut().unwrap();
        if g.raster2d_pipeline.is_some() && g.u_raster2d_bgl.is_some() && g.u_raster2d_buf.is_some()
        {
            return Ok(());
        }

        let u_raster2d_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vm-raster2d-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Raster2DUniforms>() as _,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vm-2d-raster-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SCENEVM_2D_RASTER_WGSL)),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vm-2d-raster-pipeline-layout"),
            bind_group_layouts: &[Some(&u_raster2d_bgl)],
            immediate_size: 0,
        });
        let raster2d_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vm-2d-raster-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vert2DPod>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Uint32,
                        },
                        wgpu::VertexAttribute {
                            offset: 20,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Uint32,
                        },
                        wgpu::VertexAttribute {
                            offset: 24,
                            shader_location: 4,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 28,
                            shader_location: 5,
                            format: wgpu::VertexFormat::Uint32,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let u_raster2d_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vm-raster2d-uniforms"),
            size: std::mem::size_of::<Raster2DUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        g.raster2d_pipeline = Some(raster2d_pipeline);
        g.u_raster2d_bgl = Some(u_raster2d_bgl);
        g.u_raster2d_buf = Some(u_raster2d_buf);

        Ok(())
    }

    fn init_raster3d(&mut self, device: &wgpu::Device) -> crate::SceneVMResult<()> {
        if self.gpu.is_none() {
            self.init_gpu(device)?;
        }
        self.upload_tile_metadata_to_gpu(device);
        let raster_samples = self.raster3d_effective_samples();
        let g = self.gpu.as_mut().unwrap();
        if g.raster3d_pipeline.is_some()
            && g.raster3d_alpha_pipeline.is_some()
            && g.raster3d_line_pipeline.is_some()
            && g.u_raster3d_bgl.is_some()
            && g.u_raster3d_post_bgl.is_some()
            && g.u_raster3d_buf.is_some()
            && g.raster3d_organic_billboard_pipeline.is_some()
            && g.raster3d_bloom_extract_pipeline.is_some()
            && g.raster3d_bloom_composite_pipeline.is_some()
        {
            return Ok(());
        }

        let u_raster3d_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vm-raster3d-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Raster3DUniforms>() as _,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 10,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 11,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 12,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 13,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 14,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let u_raster3d_shadow_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("vm-raster3d-shadow-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<
                                Raster3DUniforms,
                            >()
                                as _),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 11,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 12,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let u_raster3d_post_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("vm-raster3d-post-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<
                                Raster3DUniforms,
                            >()
                                as _),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vm-3d-raster-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SCENEVM_3D_RASTER_WGSL)),
        });
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vm-3d-post-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SCENEVM_3D_POST_WGSL)),
        });
        let organic_billboard_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vm-3d-organic-billboard-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                SCENEVM_3D_ORGANIC_BILLBOARD_WGSL,
            )),
        });
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vm-3d-line-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SCENEVM_3D_LINE_WGSL)),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vm-3d-raster-pipeline-layout"),
            bind_group_layouts: &[Some(&u_raster3d_bgl)],
            immediate_size: 0,
        });
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("vm-3d-raster-shadow-pipeline-layout"),
                bind_group_layouts: &[Some(&u_raster3d_shadow_bgl)],
                immediate_size: 0,
            });
        let post_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vm-3d-post-pipeline-layout"),
            bind_group_layouts: &[Some(&u_raster3d_post_bgl)],
            immediate_size: 0,
        });

        let raster3d_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vm-3d-raster-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vert3DPod>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 24,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 32,
                            shader_location: 4,
                            format: wgpu::VertexFormat::Uint32,
                        },
                        wgpu::VertexAttribute {
                            offset: 36,
                            shader_location: 5,
                            format: wgpu::VertexFormat::Uint32,
                        },
                        wgpu::VertexAttribute {
                            offset: 40,
                            shader_location: 6,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 44,
                            shader_location: 7,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 48,
                            shader_location: 8,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 60,
                            shader_location: 9,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 64,
                            shader_location: 13,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: raster_samples,
                alpha_to_coverage_enabled: true,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        let raster3d_alpha_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("vm-3d-raster-alpha-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vert3DPod>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 12,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 24,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 4,
                                format: wgpu::VertexFormat::Uint32,
                            },
                            wgpu::VertexAttribute {
                                offset: 36,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Uint32,
                            },
                            wgpu::VertexAttribute {
                                offset: 40,
                                shader_location: 6,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 44,
                                shader_location: 7,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 8,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 60,
                                shader_location: 9,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 64,
                                shader_location: 13,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(false),
                    // Transparent surfaces still blend without writing depth, but they
                    // must respect opaque depth so water/glass volumes do not draw over
                    // bridge posts or other opaque geometry inside/behind the volume.
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: raster_samples,
                    alpha_to_coverage_enabled: false,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });
        let raster3d_particle_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("vm-3d-raster-particle-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vert3DPod>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 12,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 24,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 4,
                                format: wgpu::VertexFormat::Uint32,
                            },
                            wgpu::VertexAttribute {
                                offset: 36,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Uint32,
                            },
                            wgpu::VertexAttribute {
                                offset: 40,
                                shader_location: 6,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 44,
                                shader_location: 7,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 8,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 60,
                                shader_location: 9,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 64,
                                shader_location: 13,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: raster_samples,
                    alpha_to_coverage_enabled: false,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });
        let raster3d_line_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("vm-3d-line-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &line_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Line3DPod>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &line_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: raster_samples,
                    alpha_to_coverage_enabled: true,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });
        let raster3d_organic_billboard_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("vm-3d-organic-billboard-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &organic_billboard_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &organic_billboard_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: raster_samples,
                    alpha_to_coverage_enabled: true,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });
        let raster3d_bloom_extract_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("vm-3d-bloom-extract-pipeline"),
                layout: Some(&post_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &post_shader,
                    entry_point: Some("fs_extract"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        let raster3d_bloom_composite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("vm-3d-bloom-composite-pipeline"),
                layout: Some(&post_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &post_shader,
                    entry_point: Some("fs_composite"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        let raster3d_shadow_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("vm-3d-raster-shadow-pipeline"),
                layout: Some(&shadow_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_shadow"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vert3DPod>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 12,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 24,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 4,
                                format: wgpu::VertexFormat::Uint32,
                            },
                            wgpu::VertexAttribute {
                                offset: 36,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Uint32,
                            },
                            wgpu::VertexAttribute {
                                offset: 40,
                                shader_location: 6,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 44,
                                shader_location: 7,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 8,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 60,
                                shader_location: 9,
                                format: wgpu::VertexFormat::Float32,
                            },
                            wgpu::VertexAttribute {
                                offset: 64,
                                shader_location: 13,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_shadow"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 1,
                        slope_scale: 1.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        let u_raster3d_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vm-raster3d-uniforms"),
            size: std::mem::size_of::<Raster3DUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_sampler_compare = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vm-raster3d-shadow-compare-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        g.u_raster3d_bgl = Some(u_raster3d_bgl);
        g.u_raster3d_shadow_bgl = Some(u_raster3d_shadow_bgl);
        g.u_raster3d_post_bgl = Some(u_raster3d_post_bgl);
        g.u_raster3d_buf = Some(u_raster3d_buf);
        g.raster3d_pipeline = Some(raster3d_pipeline);
        g.raster3d_alpha_pipeline = Some(raster3d_alpha_pipeline);
        g.raster3d_particle_pipeline = Some(raster3d_particle_pipeline);
        g.raster3d_line_pipeline = Some(raster3d_line_pipeline);
        g.raster3d_organic_billboard_pipeline = Some(raster3d_organic_billboard_pipeline);
        g.raster3d_bloom_extract_pipeline = Some(raster3d_bloom_extract_pipeline);
        g.raster3d_bloom_composite_pipeline = Some(raster3d_bloom_composite_pipeline);
        g.raster3d_shadow_pipeline = Some(raster3d_shadow_pipeline);
        g.shadow_sampler_compare = Some(shadow_sampler_compare);

        Ok(())
    }

    fn raster_camera_key(camera: &Camera3D) -> [f32; 6] {
        [
            camera.pos.x,
            camera.pos.y,
            camera.pos.z,
            camera.forward.x,
            camera.forward.y,
            camera.forward.z,
        ]
    }

    fn raster_camera_key_matches(a: [f32; 6], b: [f32; 6]) -> bool {
        a.iter().zip(b.iter()).all(|(a, b)| (*a - *b).abs() <= 1e-4)
    }

    fn split_raster_visible_indices_range(
        &self,
        camera: &Camera3D,
        tri_start: usize,
        tri_end: usize,
    ) -> (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>) {
        if self.cached_i3.is_empty() || self.cached_tri_visibility.is_empty() {
            return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        }
        const TILE_INDEX_FLAGS_MASK_CPU: u32 = 0xF000_0000;
        const TILE_INDEX_PARTICLE_FLAG_CPU: u32 = 0x0800_0000u32;
        let base_tile_index = |idx: u32| idx & !TILE_INDEX_FLAGS_MASK_CPU;
        let mut translucent_tile_cache: FxHashMap<u32, bool> = FxHashMap::default();
        let mut tile_is_translucent = |idx: u32| -> bool {
            let base = base_tile_index(idx);
            if let Some(v) = translucent_tile_cache.get(&base) {
                *v
            } else {
                let v = self.shared_atlas.tile_index_has_translucency(base);
                translucent_tile_cache.insert(base, v);
                v
            }
        };

        let tri_capacity = self.cached_i3.len() / 3;
        let tri_start = tri_start.min(tri_capacity);
        let tri_end = tri_end.min(tri_capacity).max(tri_start);
        let index_capacity = (tri_end - tri_start) * 3;
        let mut all_visible: Vec<u32> = Vec::with_capacity(index_capacity);
        let mut opaque: Vec<u32> = Vec::with_capacity(index_capacity);
        let mut transparent_tris: Vec<(f32, [u32; 3])> = Vec::new();
        let mut particle_tris: Vec<(f32, [u32; 3])> = Vec::new();
        for tri in tri_start..tri_end {
            let word = tri / 32;
            let bit = tri % 32;
            let visible = self
                .cached_tri_visibility
                .get(word)
                .map(|w| ((w >> bit) & 1) != 0)
                .unwrap_or(false);
            if visible {
                let base = tri * 3;
                if base + 2 < self.cached_i3.len() {
                    let i0 = self.cached_i3[base];
                    let i1 = self.cached_i3[base + 1];
                    let i2 = self.cached_i3[base + 2];
                    all_visible.extend_from_slice(&[i0, i1, i2]);
                    let v0 = self.cached_v3.get(i0 as usize);
                    let v1 = self.cached_v3.get(i1 as usize);
                    let v2 = self.cached_v3.get(i2 as usize);
                    let is_particle = if let (Some(a), Some(b), Some(c)) = (v0, v1, v2) {
                        (a.tile_index2 & TILE_INDEX_PARTICLE_FLAG_CPU) != 0
                            || (b.tile_index2 & TILE_INDEX_PARTICLE_FLAG_CPU) != 0
                            || (c.tile_index2 & TILE_INDEX_PARTICLE_FLAG_CPU) != 0
                    } else {
                        false
                    };
                    let is_transparent = if let (Some(a), Some(b), Some(c)) = (v0, v1, v2) {
                        a.opacity < 0.999
                            || b.opacity < 0.999
                            || c.opacity < 0.999
                            || tile_is_translucent(a.tile_index)
                            || tile_is_translucent(a.tile_index2)
                            || tile_is_translucent(b.tile_index)
                            || tile_is_translucent(b.tile_index2)
                            || tile_is_translucent(c.tile_index)
                            || tile_is_translucent(c.tile_index2)
                    } else {
                        false
                    };
                    if is_transparent {
                        if let (Some(a), Some(b), Some(c)) = (v0, v1, v2) {
                            let centroid = Vec3::new(
                                (a.pos[0] + b.pos[0] + c.pos[0]) / 3.0,
                                (a.pos[1] + b.pos[1] + c.pos[1]) / 3.0,
                                (a.pos[2] + b.pos[2] + c.pos[2]) / 3.0,
                            );
                            let depth = (centroid - camera.pos).dot(camera.forward);
                            if is_particle {
                                particle_tris.push((depth, [i0, i1, i2]));
                            } else {
                                transparent_tris.push((depth, [i0, i1, i2]));
                            }
                        }
                    } else {
                        opaque.extend_from_slice(&[i0, i1, i2]);
                    }
                }
            }
        }
        transparent_tris.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut transparent: Vec<u32> = Vec::with_capacity(transparent_tris.len() * 3);
        for (_, tri) in transparent_tris {
            transparent.extend_from_slice(&tri);
        }
        particle_tris.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut particles: Vec<u32> = Vec::with_capacity(particle_tris.len() * 3);
        for (_, tri) in particle_tris {
            particles.extend_from_slice(&tri);
        }
        (all_visible, opaque, transparent, particles)
    }

    fn rebuild_raster_visible_indices(
        &mut self,
        camera: &Camera3D,
    ) -> (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>) {
        let tri_capacity = self.cached_i3.len() / 3;
        if tri_capacity == 0 {
            return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        }

        let static_tri_count = (self.cached_static_i3.len() / 3).min(tri_capacity);
        let camera_key = Self::raster_camera_key(camera);
        let static_cache_valid = self.cached_static_raster_indices_valid
            && Self::raster_camera_key_matches(self.cached_static_raster_camera_key, camera_key);
        if !static_cache_valid {
            let (visible, opaque, transparent, particles) =
                self.split_raster_visible_indices_range(camera, 0, static_tri_count);
            self.cached_static_raster_visible_indices = visible;
            self.cached_static_raster_opaque_indices = opaque;
            self.cached_static_raster_transparent_indices = transparent;
            self.cached_static_raster_particle_indices = particles;
            self.cached_static_raster_camera_key = camera_key;
            self.cached_static_raster_indices_valid = true;
        }

        if static_tri_count >= tri_capacity {
            return (
                self.cached_static_raster_visible_indices.clone(),
                self.cached_static_raster_opaque_indices.clone(),
                self.cached_static_raster_transparent_indices.clone(),
                self.cached_static_raster_particle_indices.clone(),
            );
        }

        let (dynamic_visible, dynamic_opaque, dynamic_transparent, dynamic_particles) =
            self.split_raster_visible_indices_range(camera, static_tri_count, tri_capacity);

        let mut visible = self.cached_static_raster_visible_indices.clone();
        visible.extend_from_slice(&dynamic_visible);
        let mut opaque = self.cached_static_raster_opaque_indices.clone();
        opaque.extend_from_slice(&dynamic_opaque);
        let mut transparent = self.cached_static_raster_transparent_indices.clone();
        transparent.extend_from_slice(&dynamic_transparent);
        let mut particles = self.cached_static_raster_particle_indices.clone();
        particles.extend_from_slice(&dynamic_particles);
        (visible, opaque, transparent, particles)
    }

    fn ensure_tile_emissive_summaries(&mut self) {
        let content_version = self.shared_atlas.content_version();
        if self.cached_tile_emissive_content_version != content_version {
            self.cached_tile_emissive_summaries = self.shared_atlas.tile_emissive_summaries();
            self.cached_tile_emissive_content_version = content_version;
        }
    }

    fn collect_emissive_surface_lighting(
        &mut self,
        visible_indices: &[u32],
        _camera: &Camera3D,
        budget: usize,
    ) -> EmissiveSurfaceLighting {
        if budget == 0 || visible_indices.is_empty() {
            return EmissiveSurfaceLighting::default();
        }

        self.ensure_tile_emissive_summaries();
        if self.cached_tile_emissive_summaries.is_empty() {
            return EmissiveSurfaceLighting::default();
        }

        const TILE_INDEX_ALL_FLAGS_MASK_CPU: u32 = 0xF800_0000;
        const TILE_INDEX_PARTICLE_FLAG_CPU: u32 = 0x0800_0000;
        const TILE_INDEX_CLAMP_UV_FLAG_CPU: u32 = 0x4000_0000;
        let base_tile_index = |idx: u32| idx & !TILE_INDEX_ALL_FLAGS_MASK_CPU;

        #[derive(Clone, Copy)]
        struct Cluster {
            position_weighted: Vec3<f32>,
            color_weighted: Vec3<f32>,
            weight: f32,
            intensity: f32,
            range: f32,
            score: f32,
        }

        let mut clusters: FxHashMap<(i32, i32, i32), Cluster> = FxHashMap::default();
        let mut broad_energy = Vec3::zero();
        let mut broad_area = 0.0f32;

        for tri in visible_indices.chunks_exact(3) {
            let Some(v0) = self.cached_v3.get(tri[0] as usize) else {
                continue;
            };
            let Some(v1) = self.cached_v3.get(tri[1] as usize) else {
                continue;
            };
            let Some(v2) = self.cached_v3.get(tri[2] as usize) else {
                continue;
            };
            if (v0.tile_index2 | v1.tile_index2 | v2.tile_index2) & TILE_INDEX_PARTICLE_FLAG_CPU
                != 0
            {
                continue;
            }

            let tile_index = base_tile_index(v0.tile_index) as usize;
            let Some(summary) = self.cached_tile_emissive_summaries.get(tile_index) else {
                continue;
            };
            if summary.strength <= 0.002 || summary.hotspot_strength <= 0.0 {
                continue;
            }

            let p0 = Vec3::new(v0.pos[0], v0.pos[1], v0.pos[2]);
            let p1 = Vec3::new(v1.pos[0], v1.pos[1], v1.pos[2]);
            let p2 = Vec3::new(v2.pos[0], v2.pos[1], v2.pos[2]);
            let edge0 = p1 - p0;
            let edge1 = p2 - p0;
            let cross = edge0.cross(edge1);
            let cross_len = cross.magnitude();
            if cross_len <= 1e-5 {
                continue;
            }
            let area = (cross_len * 0.5).clamp(0.001, 64.0);
            let normal = cross / cross_len;

            let uv0 = Vec2::new(v0.uv[0], v0.uv[1]);
            let uv1 = Vec2::new(v1.uv[0], v1.uv[1]);
            let uv2 = Vec2::new(v2.uv[0], v2.uv[1]);
            let hotspot_uv = Vec2::new(summary.hotspot_uv[0], summary.hotspot_uv[1]);
            let repeats_uv = (v0.tile_index2 & TILE_INDEX_CLAMP_UV_FLAG_CPU) == 0;
            let hotspot_anchor = Self::world_position_from_triangle_hotspot_uv(
                hotspot_uv, repeats_uv, uv0, uv1, uv2, p0, p1, p2,
            );

            let position = if summary.coverage < 0.45 {
                let Some(anchor) = hotspot_anchor else {
                    continue;
                };
                anchor
            } else {
                hotspot_anchor.unwrap_or((p0 + p1 + p2) / 3.0)
            } + normal * 0.04;

            let color = Vec3::new(
                summary.color_linear[0],
                summary.color_linear[1],
                summary.color_linear[2],
            )
            .map(|v: f32| v.clamp(0.0, 8.0));

            let emissive_area = area * summary.coverage.clamp(0.0, 1.0);
            if emissive_area > 0.0 {
                broad_energy += color * (summary.strength * emissive_area);
                broad_area += emissive_area;
            }

            if summary.coverage > 0.65 && area > 0.45 {
                continue;
            }

            let area_gain = area.sqrt().clamp(0.35, 4.0);
            let coverage_gain = summary.coverage.sqrt().clamp(0.25, 1.0);
            let intensity = (summary.strength * area_gain * coverage_gain * 5.5).clamp(0.03, 8.0);
            let range = (1.25 + area_gain * 1.6 + summary.coverage * 2.0).clamp(1.5, 8.0);
            let score = intensity * range * (0.5 + summary.coverage);
            let key = (
                (position.x / EMISSIVE_SURFACE_CLUSTER_SIZE).floor() as i32,
                (position.y / EMISSIVE_SURFACE_CLUSTER_SIZE).floor() as i32,
                (position.z / EMISSIVE_SURFACE_CLUSTER_SIZE).floor() as i32,
            );
            let weight = intensity.max(0.001);

            clusters
                .entry(key)
                .and_modify(|cluster| {
                    cluster.position_weighted += position * weight;
                    cluster.color_weighted += color * weight;
                    cluster.weight += weight;
                    cluster.intensity = (cluster.intensity + intensity).clamp(0.03, 8.0);
                    cluster.range = cluster.range.max(range);
                    cluster.score = cluster.score.max(score);
                })
                .or_insert(Cluster {
                    position_weighted: position * weight,
                    color_weighted: color * weight,
                    weight,
                    intensity,
                    range,
                    score,
                });
        }

        let mut lights: Vec<RasterPointLight> = clusters
            .values()
            .filter_map(|cluster| {
                if cluster.weight <= 0.0 || cluster.intensity <= 0.0 {
                    return None;
                }
                let inv_weight = 1.0 / cluster.weight;
                Some(RasterPointLight {
                    position: cluster.position_weighted * inv_weight,
                    color: cluster.color_weighted * inv_weight,
                    intensity: cluster.intensity,
                    range: cluster.range,
                    score: cluster.score,
                })
            })
            .collect();
        lights.sort_by(|a, b| b.score.total_cmp(&a.score));
        lights.truncate(budget);

        let broad_color = if broad_area > 0.0 {
            let area_factor = (broad_area / 18.0).sqrt().clamp(0.0, 1.25);
            (broad_energy / broad_area * (0.34 * area_factor)).map(|v: f32| v.clamp(0.0, 0.65))
        } else {
            Vec3::zero()
        };

        EmissiveSurfaceLighting {
            point_lights: lights,
            broad_color,
        }
    }

    fn world_position_from_triangle_hotspot_uv(
        hotspot_uv: Vec2<f32>,
        repeats_uv: bool,
        uv0: Vec2<f32>,
        uv1: Vec2<f32>,
        uv2: Vec2<f32>,
        p0: Vec3<f32>,
        p1: Vec3<f32>,
        p2: Vec3<f32>,
    ) -> Option<Vec3<f32>> {
        if !repeats_uv {
            return Self::world_position_from_triangle_uv(hotspot_uv, uv0, uv1, uv2, p0, p1, p2);
        }

        let uv_centroid = (uv0 + uv1 + uv2) / 3.0;
        let base_offset = Vec2::new(
            (uv_centroid.x - hotspot_uv.x).round(),
            (uv_centroid.y - hotspot_uv.y).round(),
        );
        let mut best: Option<(f32, Vec3<f32>)> = None;
        for oy in -1..=1 {
            for ox in -1..=1 {
                let repeated_uv = hotspot_uv + base_offset + Vec2::new(ox as f32, oy as f32);
                let Some(position) =
                    Self::world_position_from_triangle_uv(repeated_uv, uv0, uv1, uv2, p0, p1, p2)
                else {
                    continue;
                };
                let dist2 = (repeated_uv - uv_centroid).magnitude_squared();
                match best {
                    Some((best_dist2, _)) if dist2 >= best_dist2 => {}
                    _ => best = Some((dist2, position)),
                }
            }
        }
        best.map(|(_, position)| position)
    }

    fn world_position_from_triangle_uv(
        uv: Vec2<f32>,
        uv0: Vec2<f32>,
        uv1: Vec2<f32>,
        uv2: Vec2<f32>,
        p0: Vec3<f32>,
        p1: Vec3<f32>,
        p2: Vec3<f32>,
    ) -> Option<Vec3<f32>> {
        let v0 = uv1 - uv0;
        let v1 = uv2 - uv0;
        let v2 = uv - uv0;
        let denom = v0.x * v1.y - v1.x * v0.y;
        if denom.abs() <= 1e-6 {
            return None;
        }
        let b1 = (v2.x * v1.y - v1.x * v2.y) / denom;
        let b2 = (v0.x * v2.y - v2.x * v0.y) / denom;
        let b0 = 1.0 - b1 - b2;
        let tolerance = -0.03;
        if b0 < tolerance || b1 < tolerance || b2 < tolerance {
            return None;
        }
        Some(p0 * b0 + p1 * b1 + p2 * b2)
    }

    fn collect_emissive_irradiance_sources(&mut self, budget: usize) -> Vec<RasterPointLight> {
        if budget == 0 || self.cached_i3.is_empty() {
            return Vec::new();
        }

        self.ensure_tile_emissive_summaries();
        if self.cached_tile_emissive_summaries.is_empty() {
            return Vec::new();
        }

        const TILE_INDEX_ALL_FLAGS_MASK_CPU: u32 = 0xF800_0000;
        const TILE_INDEX_PARTICLE_FLAG_CPU: u32 = 0x0800_0000;
        const TILE_INDEX_CLAMP_UV_FLAG_CPU: u32 = 0x4000_0000;
        let base_tile_index = |idx: u32| idx & !TILE_INDEX_ALL_FLAGS_MASK_CPU;

        #[derive(Clone, Copy)]
        struct Cluster {
            position_weighted: Vec3<f32>,
            color_weighted: Vec3<f32>,
            weight: f32,
            intensity: f32,
            range: f32,
            score: f32,
        }

        let mut clusters: FxHashMap<(i32, i32, i32), Cluster> = FxHashMap::default();
        let cluster_size = EMISSIVE_SURFACE_CLUSTER_SIZE * 1.75;

        for (tri_idx, tri) in self.cached_i3.chunks_exact(3).enumerate() {
            let word = tri_idx / 32;
            let bit = tri_idx % 32;
            let visible = self
                .cached_tri_visibility
                .get(word)
                .map(|w| ((w >> bit) & 1) != 0)
                .unwrap_or(true);
            if !visible {
                continue;
            }
            let Some(v0) = self.cached_v3.get(tri[0] as usize) else {
                continue;
            };
            let Some(v1) = self.cached_v3.get(tri[1] as usize) else {
                continue;
            };
            let Some(v2) = self.cached_v3.get(tri[2] as usize) else {
                continue;
            };
            if (v0.tile_index2 | v1.tile_index2 | v2.tile_index2) & TILE_INDEX_PARTICLE_FLAG_CPU
                != 0
            {
                continue;
            }

            let tile_index = base_tile_index(v0.tile_index) as usize;
            let Some(summary) = self.cached_tile_emissive_summaries.get(tile_index) else {
                continue;
            };
            if summary.strength <= 0.002 || summary.hotspot_strength <= 0.0 {
                continue;
            }

            let p0 = Vec3::new(v0.pos[0], v0.pos[1], v0.pos[2]);
            let p1 = Vec3::new(v1.pos[0], v1.pos[1], v1.pos[2]);
            let p2 = Vec3::new(v2.pos[0], v2.pos[1], v2.pos[2]);
            let edge0 = p1 - p0;
            let edge1 = p2 - p0;
            let cross = edge0.cross(edge1);
            let cross_len = cross.magnitude();
            if cross_len <= 1e-5 {
                continue;
            }
            let area = (cross_len * 0.5).clamp(0.001, 96.0);
            let normal = cross / cross_len;

            let uv0 = Vec2::new(v0.uv[0], v0.uv[1]);
            let uv1 = Vec2::new(v1.uv[0], v1.uv[1]);
            let uv2 = Vec2::new(v2.uv[0], v2.uv[1]);
            let hotspot_uv = Vec2::new(summary.hotspot_uv[0], summary.hotspot_uv[1]);
            let repeats_uv = (v0.tile_index2 & TILE_INDEX_CLAMP_UV_FLAG_CPU) == 0;
            let hotspot_anchor = Self::world_position_from_triangle_hotspot_uv(
                hotspot_uv, repeats_uv, uv0, uv1, uv2, p0, p1, p2,
            );
            let position = hotspot_anchor.unwrap_or((p0 + p1 + p2) / 3.0) + normal * 0.08;
            let color = Vec3::new(
                summary.color_linear[0],
                summary.color_linear[1],
                summary.color_linear[2],
            )
            .map(|v: f32| v.clamp(0.0, 8.0));

            let area_gain = area.sqrt().clamp(0.35, 5.0);
            let coverage_gain = summary.coverage.sqrt().clamp(0.25, 1.0);
            let intensity = (summary.strength * area_gain * coverage_gain * 4.8).clamp(0.025, 7.0);
            let range = (2.5 + area_gain * 1.9 + summary.coverage * 3.8).clamp(2.5, 12.0);
            let score = intensity * range * (0.5 + summary.coverage);
            let key = (
                (position.x / cluster_size).floor() as i32,
                (position.y / cluster_size).floor() as i32,
                (position.z / cluster_size).floor() as i32,
            );
            let weight = intensity.max(0.001);

            clusters
                .entry(key)
                .and_modify(|cluster| {
                    cluster.position_weighted += position * weight;
                    cluster.color_weighted += color * weight;
                    cluster.weight += weight;
                    cluster.intensity = cluster.intensity.max(intensity);
                    cluster.range = cluster.range.max(range);
                    cluster.score += score;
                })
                .or_insert(Cluster {
                    position_weighted: position * weight,
                    color_weighted: color * weight,
                    weight,
                    intensity,
                    range,
                    score,
                });
        }

        let mut sources: Vec<RasterPointLight> = clusters
            .values()
            .filter_map(|cluster| {
                if cluster.weight <= 0.0 || cluster.intensity <= 0.0 {
                    return None;
                }
                let inv_weight = 1.0 / cluster.weight;
                Some(RasterPointLight {
                    position: cluster.position_weighted * inv_weight,
                    color: cluster.color_weighted * inv_weight,
                    intensity: cluster.intensity,
                    range: cluster.range,
                    score: cluster.score,
                })
            })
            .collect();
        sources.sort_by(|a, b| b.score.total_cmp(&a.score));
        sources.truncate(budget);
        sources
    }

    fn disabled_irradiance_grid_data() -> Vec<[f32; 4]> {
        vec![
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0, 0.0],
            [2.0, 2.0, 2.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
        ]
    }

    fn collect_irradiance_occluders(&self) -> Vec<IrradianceOccluder> {
        if self.cached_i3.is_empty() || self.cached_v3.is_empty() {
            return Vec::new();
        }

        const TILE_INDEX_ALL_FLAGS_MASK_CPU: u32 = 0xF800_0000;
        const TILE_INDEX_PARTICLE_FLAG_CPU: u32 = 0x0800_0000;
        let base_tile_index = |idx: u32| idx & !TILE_INDEX_ALL_FLAGS_MASK_CPU;
        let mut translucent_tile_cache: FxHashMap<u32, bool> = FxHashMap::default();
        let mut tile_is_translucent = |idx: u32| -> bool {
            let base = base_tile_index(idx);
            if let Some(v) = translucent_tile_cache.get(&base) {
                *v
            } else {
                let v = self.shared_atlas.tile_index_has_translucency(base);
                translucent_tile_cache.insert(base, v);
                v
            }
        };

        let mut occluders = Vec::new();
        for (tri_idx, tri) in self.cached_i3.chunks_exact(3).enumerate() {
            let visible = if self.cached_tri_visibility.is_empty() {
                true
            } else {
                let word = tri_idx / 32;
                let bit = tri_idx % 32;
                self.cached_tri_visibility
                    .get(word)
                    .map(|w| ((w >> bit) & 1) != 0)
                    .unwrap_or(false)
            };
            if !visible {
                continue;
            }

            let Some(v0) = self.cached_v3.get(tri[0] as usize) else {
                continue;
            };
            let Some(v1) = self.cached_v3.get(tri[1] as usize) else {
                continue;
            };
            let Some(v2) = self.cached_v3.get(tri[2] as usize) else {
                continue;
            };
            if (v0.tile_index2 | v1.tile_index2 | v2.tile_index2) & TILE_INDEX_PARTICLE_FLAG_CPU
                != 0
            {
                continue;
            }
            if v0.opacity < 0.999
                || v1.opacity < 0.999
                || v2.opacity < 0.999
                || tile_is_translucent(v0.tile_index)
                || tile_is_translucent(v0.tile_index2)
                || tile_is_translucent(v1.tile_index)
                || tile_is_translucent(v1.tile_index2)
                || tile_is_translucent(v2.tile_index)
                || tile_is_translucent(v2.tile_index2)
            {
                continue;
            }

            let p0 = Vec3::new(v0.pos[0], v0.pos[1], v0.pos[2]);
            let p1 = Vec3::new(v1.pos[0], v1.pos[1], v1.pos[2]);
            let p2 = Vec3::new(v2.pos[0], v2.pos[1], v2.pos[2]);
            let normal_len = (p1 - p0).cross(p2 - p0).magnitude();
            if normal_len <= 1e-5 {
                continue;
            }
            let min = Vec3::new(
                p0.x.min(p1.x).min(p2.x),
                p0.y.min(p1.y).min(p2.y),
                p0.z.min(p1.z).min(p2.z),
            );
            let max = Vec3::new(
                p0.x.max(p1.x).max(p2.x),
                p0.y.max(p1.y).max(p2.y),
                p0.z.max(p1.z).max(p2.z),
            );
            occluders.push(IrradianceOccluder {
                a: v0.pos,
                b: v1.pos,
                c: v2.pos,
                min,
                max,
            });
        }

        if occluders.len() > IRRADIANCE_OCCLUSION_MAX_TRIANGLES {
            let stride = occluders
                .len()
                .div_ceil(IRRADIANCE_OCCLUSION_MAX_TRIANGLES)
                .max(1);
            occluders = occluders
                .into_iter()
                .step_by(stride)
                .take(IRRADIANCE_OCCLUSION_MAX_TRIANGLES)
                .collect();
        }
        occluders
    }

    fn segment_intersects_aabb(
        origin: Vec3<f32>,
        dir: Vec3<f32>,
        max_t: f32,
        min: Vec3<f32>,
        max: Vec3<f32>,
    ) -> bool {
        let mut t_min = 0.0f32;
        let mut t_max = max_t;
        for axis in 0..3 {
            let o = origin[axis];
            let d = dir[axis];
            let mn = min[axis] - 0.015;
            let mx = max[axis] + 0.015;
            if d.abs() <= 1e-6 {
                if o < mn || o > mx {
                    return false;
                }
            } else {
                let inv_d = 1.0 / d;
                let mut t0 = (mn - o) * inv_d;
                let mut t1 = (mx - o) * inv_d;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                t_min = t_min.max(t0);
                t_max = t_max.min(t1);
                if t_max < t_min {
                    return false;
                }
            }
        }
        t_max >= 0.0 && t_min <= max_t
    }

    fn irradiance_segment_visibility(
        probe: Vec3<f32>,
        source: Vec3<f32>,
        occluders: &[IrradianceOccluder],
    ) -> f32 {
        if occluders.is_empty() {
            return 1.0;
        }
        let to_source = source - probe;
        let dist = to_source.magnitude();
        if dist <= 0.2 {
            return 1.0;
        }

        let dir = to_source / dist;
        let origin = probe + dir * 0.08;
        let max_t = (dist - 0.16).max(0.0);
        if max_t <= 0.0 {
            return 1.0;
        }

        for occ in occluders {
            if !Self::segment_intersects_aabb(origin, dir, max_t, occ.min, occ.max) {
                continue;
            }
            let Some((t, _, _)) = ray_triangle_intersect(origin, dir, occ.a, occ.b, occ.c) else {
                continue;
            };
            if t > 0.04 && t < max_t - 0.04 {
                return IRRADIANCE_OCCLUSION_BLOCKED_VISIBILITY;
            }
        }
        1.0
    }

    fn build_irradiance_grid_data(&mut self) -> Vec<[f32; 4]> {
        if self.cached_v3.is_empty() {
            return Self::disabled_irradiance_grid_data();
        }

        let mut sources: Vec<RasterPointLight> = self
            .lights
            .values()
            .filter(|l| l.emitting && matches!(l.light_type, LightType::Point))
            .map(|light| RasterPointLight {
                position: light.position,
                color: light.color,
                intensity: light.intensity,
                range: light.end_distance.max(light.radius).max(0.1),
                score: light.intensity * light.end_distance.max(0.1),
            })
            .collect();
        if EMISSIVE_SURFACE_IRRADIANCE_ENABLED {
            let remaining = IRRADIANCE_GRID_MAX_SOURCES.saturating_sub(sources.len());
            sources.extend(self.collect_emissive_irradiance_sources(remaining));
        }
        sources.sort_by(|a, b| b.score.total_cmp(&a.score));
        sources.truncate(IRRADIANCE_GRID_MAX_SOURCES);
        if sources.is_empty() {
            return Self::disabled_irradiance_grid_data();
        }
        let occluders = self.collect_irradiance_occluders();

        let mut bmin = Vec3::broadcast(f32::INFINITY);
        let mut bmax = Vec3::broadcast(f32::NEG_INFINITY);
        for v in &self.cached_v3 {
            let p = Vec3::new(v.pos[0], v.pos[1], v.pos[2]);
            bmin.x = bmin.x.min(p.x);
            bmin.y = bmin.y.min(p.y);
            bmin.z = bmin.z.min(p.z);
            bmax.x = bmax.x.max(p.x);
            bmax.y = bmax.y.max(p.y);
            bmax.z = bmax.z.max(p.z);
        }
        if !bmin.x.is_finite() || !bmax.x.is_finite() {
            return Self::disabled_irradiance_grid_data();
        }

        let margin = IRRADIANCE_GRID_TARGET_CELL_SIZE * 1.25;
        bmin -= Vec3::broadcast(margin);
        bmax += Vec3::broadcast(margin);
        let extent = Vec3::new(
            (bmax.x - bmin.x).max(1.0),
            (bmax.y - bmin.y).max(1.0),
            (bmax.z - bmin.z).max(1.0),
        );
        let dim_for = |extent: f32, max_dim: u32| -> u32 {
            ((extent / IRRADIANCE_GRID_TARGET_CELL_SIZE).ceil() as u32 + 1).clamp(2, max_dim)
        };
        let dims = [
            dim_for(extent.x, IRRADIANCE_GRID_MAX_XZ),
            dim_for(extent.y, IRRADIANCE_GRID_MAX_Y),
            dim_for(extent.z, IRRADIANCE_GRID_MAX_XZ),
        ];
        let cell = Vec3::new(
            extent.x / (dims[0] - 1) as f32,
            extent.y / (dims[1] - 1) as f32,
            extent.z / (dims[2] - 1) as f32,
        );
        let probe_count = (dims[0] * dims[1] * dims[2]) as usize;
        let mut direct = vec![Vec3::zero(); probe_count];

        let idx = |x: u32, y: u32, z: u32| -> usize {
            (x + y * dims[0] + z * dims[0] * dims[1]) as usize
        };
        let smooth_range = |range: f32, dist: f32| -> f32 {
            let t = ((range - dist) / range.max(0.001)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };

        for z in 0..dims[2] {
            for y in 0..dims[1] {
                for x in 0..dims[0] {
                    let p = Vec3::new(
                        bmin.x + cell.x * x as f32,
                        bmin.y + cell.y * y as f32,
                        bmin.z + cell.z * z as f32,
                    );
                    let mut irradiance = Vec3::zero();
                    for source in &sources {
                        let to_probe = p - source.position;
                        let dist2 = to_probe.magnitude_squared().max(0.45);
                        let dist = dist2.sqrt();
                        if dist > source.range {
                            continue;
                        }
                        let visibility =
                            Self::irradiance_segment_visibility(p, source.position, &occluders);
                        let range_factor = smooth_range(source.range, dist);
                        let atten = source.intensity * range_factor / dist2;
                        irradiance += source.color * (atten * 0.24 * visibility);
                    }
                    direct[idx(x, y, z)] = irradiance.map(|v: f32| v.clamp(0.0, 1.25));
                }
            }
        }

        let mut propagated = direct.clone();
        for _ in 0..2 {
            let previous = propagated.clone();
            for z in 0..dims[2] {
                for y in 0..dims[1] {
                    for x in 0..dims[0] {
                        let mut sum = Vec3::zero();
                        let mut count = 0.0f32;
                        for (ox, oy, oz) in [
                            (-1i32, 0i32, 0i32),
                            (1, 0, 0),
                            (0, -1, 0),
                            (0, 1, 0),
                            (0, 0, -1),
                            (0, 0, 1),
                        ] {
                            let nx = x as i32 + ox;
                            let ny = y as i32 + oy;
                            let nz = z as i32 + oz;
                            if nx < 0
                                || ny < 0
                                || nz < 0
                                || nx >= dims[0] as i32
                                || ny >= dims[1] as i32
                                || nz >= dims[2] as i32
                            {
                                continue;
                            }
                            sum += previous[idx(nx as u32, ny as u32, nz as u32)];
                            count += 1.0;
                        }
                        let neighbor = if count > 0.0 {
                            sum / count
                        } else {
                            Vec3::zero()
                        };
                        let i = idx(x, y, z);
                        propagated[i] = (direct[i] + neighbor * 0.42 + previous[i] * 0.18)
                            .map(|v: f32| v.clamp(0.0, 0.85));
                    }
                }
            }
        }

        let mut out = Vec::with_capacity(3 + probe_count);
        out.push([bmin.x, bmin.y, bmin.z, 1.0]);
        out.push([cell.x, cell.y, cell.z, 0.0]);
        out.push([dims[0] as f32, dims[1] as f32, dims[2] as f32, 0.0]);
        for c in propagated {
            out.push([c.x, c.y, c.z, 0.0]);
        }
        out
    }

    fn upload_irradiance_grid_ssbo(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.gpu.is_none() {
            return;
        }
        if EMISSIVE_SURFACE_IRRADIANCE_ENABLED
            && self.cached_tile_emissive_content_version != self.shared_atlas.content_version()
        {
            self.mark_irradiance_grid_dirty();
        }
        let rebuilt = self.irradiance_grid_dirty || self.cached_irradiance_grid_data.is_empty();
        if rebuilt {
            self.cached_irradiance_grid_data = self.build_irradiance_grid_data();
            self.irradiance_grid_dirty = false;
        }
        let byte_len = (self.cached_irradiance_grid_data.len() * std::mem::size_of::<[f32; 4]>())
            .max(std::mem::size_of::<[f32; 4]>());
        let needs_recreate = self
            .gpu
            .as_ref()
            .map(|g| g.irradiance_grid_ssbo.is_none() || g.irradiance_grid_ssbo_size != byte_len)
            .unwrap_or(true);
        if !rebuilt && !needs_recreate {
            return;
        }
        let bytes: &[u8] = bytemuck::cast_slice(&self.cached_irradiance_grid_data);
        let g = self.gpu.as_mut().unwrap();
        if needs_recreate {
            use wgpu::util::DeviceExt;
            g.irradiance_grid_ssbo = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("vm-raster3d-irradiance-grid"),
                    contents: bytes,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                },
            ));
            g.irradiance_grid_ssbo_size = byte_len;
        } else if rebuilt {
            if let Some(buf) = g.irradiance_grid_ssbo.as_ref() {
                queue.write_buffer(buf, 0, bytes);
            }
        }
    }

    /// Dispatches 2D compute pipeline into a storage-capable surface.
    pub fn raster_draw_2d_into(
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
        self.init_raster2d(device)?;
        self.upload_tile_metadata_to_gpu(device);
        let (write_view, _prev_view, next_front) =
            self.prepare_layer_views(device, queue, fb_w, fb_h);

        let fb_dims = (fb_w, fb_h);
        let has_dynamic_billboards = self
            .dynamic_objects
            .iter()
            .any(|obj| obj.kind != DynamicKind::Mesh)
            || !self.dynamic_avatar_objects.is_empty();
        let mut geometry_changed = false;
        if self.geometry2d_dirty
            || self.cached_v2.is_empty()
            || self.cached_fb_size_2d != fb_dims
            || has_dynamic_billboards
        {
            let rebuild_static_2d = self.geometry2d_dirty
                || self.cached_static_v2.is_empty()
                || self.cached_static_i2.is_empty()
                || self.cached_static_fb_size_2d != fb_dims;
            if rebuild_static_2d {
                let (verts_flat, indices_flat, tile_bins, tile_tris) =
                    self.build_2d_batches(fb_w, fb_h);
                self.cached_static_v2 = verts_flat;
                self.cached_static_i2 = indices_flat;
                self.cached_static_tile_bins = tile_bins;
                self.cached_static_tile_tris = tile_tris;
                self.cached_static_fb_size_2d = fb_dims;
            }

            let mut verts_flat = self.cached_static_v2.clone();
            let mut indices_flat = self.cached_static_i2.clone();

            let m = self.transform2d;
            let dynamic_objs = self.sorted_dynamic_objects();
            let avatar_meta_indices = self.avatar_meta_indices_for_objects(&dynamic_objs);

            for obj in dynamic_objs {
                let (tile_index, tile_index2) = match obj.kind {
                    DynamicKind::BillboardTile | DynamicKind::ParticleBillboard => {
                        let Some(tile_id) = obj.tile_id else { continue };
                        let Some(tile_index) = self.shared_atlas.tile_index(&tile_id) else {
                            continue;
                        };
                        let mut tile_index2 = tile_index;
                        if obj.kind == DynamicKind::ParticleBillboard {
                            tile_index2 |= TILE_INDEX_PARTICLE_FLAG_RUST;
                        }
                        (tile_index, tile_index2)
                    }
                    DynamicKind::BillboardAvatar => {
                        let Some(avatar_index) = avatar_meta_indices.get(&obj.id).copied() else {
                            continue;
                        };
                        (avatar_index, 0x8000_0000u32)
                    }
                    DynamicKind::Mesh => continue,
                };

                let center_scr = m * Vec3::new(obj.center.x, obj.center.y, 1.0);
                let half_width = (obj.width * 0.5).max(0.0);
                let half_height = (obj.height * 0.5).max(0.0);
                if half_width <= 0.0 || half_height <= 0.0 {
                    continue;
                }
                let right_world = obj.view_right * half_width;
                let up_world = obj.view_up * half_height;
                let right_scr = (m * Vec3::new(
                    obj.center.x + right_world.x,
                    obj.center.y + right_world.y,
                    1.0,
                ))
                .xy()
                    - center_scr.xy();
                let up_scr =
                    (m * Vec3::new(obj.center.x + up_world.x, obj.center.y + up_world.y, 1.0)).xy()
                        - center_scr.xy();

                let c = center_scr.xy();
                let p0 = c - right_scr - up_scr;
                let p1 = c - right_scr + up_scr;
                let p2 = c + right_scr + up_scr;
                let p3 = c + right_scr - up_scr;

                let uvs = if matches!(obj.repeat_mode, crate::dynamic::RepeatMode::Repeat) {
                    [
                        [0.0f32, obj.height],
                        [0.0, 0.0],
                        [obj.width, 0.0],
                        [obj.width, obj.height],
                    ]
                } else {
                    [[0.0f32, 1.0f32], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]
                };

                let base = verts_flat.len() as u32;
                let pts = [p0, p1, p2, p3];
                for i in 0..4 {
                    verts_flat.push(Vert2DPod {
                        pos: [pts[i].x, pts[i].y],
                        uv: uvs[i],
                        tile_index,
                        tile_index2,
                        blend_factor: obj.anim_start_counter.map(|v| v as f32).unwrap_or(0.0),
                        _pad0: 0,
                    });
                }
                indices_flat.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base,
                    base + 2,
                    base + 3,
                ]);
            }

            self.cached_v2 = verts_flat;
            self.cached_i2 = indices_flat;
            self.cached_tile_bins = self.cached_static_tile_bins.clone();
            self.cached_tile_tris = self.cached_static_tile_tris.clone();
            self.cached_fb_size_2d = fb_dims;
            self.geometry2d_dirty = false;
            geometry_changed = true;
        }

        use wgpu::util::DeviceExt;
        {
            let g = self.gpu.as_mut().unwrap();
            if geometry_changed || g.v2d_ssbo.is_none() || g.i2d_ssbo.is_none() {
                let mut v_data = bytemuck::cast_slice(&self.cached_v2).to_vec();
                if v_data.is_empty() {
                    v_data = bytemuck::bytes_of(&Vert2DPod {
                        pos: [0.0, 0.0],
                        uv: [0.0, 0.0],
                        tile_index: 0,
                        tile_index2: 0,
                        blend_factor: 0.0,
                        _pad0: 0,
                    })
                    .to_vec();
                }
                let mut i_data = bytemuck::cast_slice(&self.cached_i2).to_vec();
                if i_data.is_empty() {
                    i_data = 0u32.to_ne_bytes().to_vec();
                }
                g.v2d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-2d-verts-raster"),
                        contents: &v_data,
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::VERTEX
                            | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                g.i2d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-2d-indices-raster"),
                        contents: &i_data,
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::INDEX
                            | wgpu::BufferUsages::COPY_DST,
                    }),
                );
            }
        }

        self.upload_scene_data_ssbo(device, queue);
        self.upload_atlas_to_gpu_with(device, queue);
        let (atlas_view, atlas_mat_view) = self
            .shared_atlas
            .texture_views()
            .expect("atlas GPU resources missing");

        let m = self.transform2d;
        let m_inv = mat3_inverse_f32(&m).unwrap_or(Mat3::<f32>::identity());
        let u = Raster2DUniforms {
            misc0: [fb_w as f32, fb_h as f32, self.animation_counter as f32, 0.0],
            post_params: [
                self.gp9.x,
                self.gp9.y,
                self.gp9.z.max(0.0),
                self.gp9.w.max(0.001),
            ],
            post_color_adjust: [self.gp8.z.max(0.0), self.gp8.w.max(0.0), 0.0, 0.0],
            post_style0: self.raster3d_post_style0.into_array(),
            post_style1: self.raster3d_post_style1.into_array(),
            ambient_color_strength: self.gp3.into_array(),
            sun_color_intensity: self.gp1.into_array(),
            sun_dir_enabled: self.gp2.into_array(),
            remap_params: [
                self.palette_remap_2d_start as f32,
                self.palette_remap_2d_end as f32,
                self.palette_remap_2d_blend.clamp(0.0, 1.0),
                self.palette_remap_2d_mode as u32 as f32,
            ],
            mat2d_inv_c0: [m_inv[(0, 0)], m_inv[(1, 0)], m_inv[(2, 0)], 0.0],
            mat2d_inv_c1: [m_inv[(0, 1)], m_inv[(1, 1)], m_inv[(2, 1)], 0.0],
            mat2d_inv_c2: [m_inv[(0, 2)], m_inv[(1, 2)], m_inv[(2, 2)], 0.0],
            palette: self.palette,
        };
        let tone_mapper = self.gp9.y.max(0.0) as u32;
        let post_enabled = self.gp9.x > 0.5;
        let exposure = self.gp9.z.max(0.0);
        let gamma = self.gp9.w.max(0.001);
        let saturation = self.gp8.z.max(0.0);
        let luminance = self.gp8.w.max(0.0);
        let post_style0 = self.raster3d_post_style0.into_array();
        let posterize = post_style0[1].clamp(0.0, 1.0);
        let palette_bias = post_style0[2].clamp(0.0, 1.0);
        let shadow_lift = post_style0[3].clamp(0.0, 1.0);
        let apply_post_cpu = |mut c: [f32; 3]| -> [f32; 3] {
            c[0] = c[0].max(0.0);
            c[1] = c[1].max(0.0);
            c[2] = c[2].max(0.0);
            if post_enabled {
                c[0] = (c[0] * exposure).max(0.0);
                c[1] = (c[1] * exposure).max(0.0);
                c[2] = (c[2] * exposure).max(0.0);
                match tone_mapper {
                    1 => {
                        c[0] = c[0] / (c[0] + 1.0);
                        c[1] = c[1] / (c[1] + 1.0);
                        c[2] = c[2] / (c[2] + 1.0);
                    }
                    2 => {
                        let aces = |x: f32| -> f32 {
                            let a = 2.51;
                            let b = 0.03;
                            let c2 = 2.43;
                            let d = 0.59;
                            let e = 0.14;
                            ((x * (a * x + b)) / (x * (c2 * x + d) + e)).clamp(0.0, 1.0)
                        };
                        c[0] = aces(c[0]);
                        c[1] = aces(c[1]);
                        c[2] = aces(c[2]);
                    }
                    _ => {}
                }
                c[0] *= luminance;
                c[1] *= luminance;
                c[2] *= luminance;
                let luma = c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
                let lift = (1.0 - luma).max(0.0).powf(2.0) * 0.12 * shadow_lift;
                c[0] += lift;
                c[1] += lift;
                c[2] += lift;
                let luma = c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
                let earth = [luma * 1.07, luma * 0.98, luma * 0.82];
                c[0] = c[0] + ((c[0] + (earth[0] - c[0]) * 0.45) - c[0]) * palette_bias;
                c[1] = c[1] + ((c[1] + (earth[1] - c[1]) * 0.45) - c[1]) * palette_bias;
                c[2] = c[2] + ((c[2] + (earth[2] - c[2]) * 0.45) - c[2]) * palette_bias;
                if posterize > 0.0 {
                    let levels = 32.0 + (7.0 - 32.0) * posterize;
                    let q0 = ((c[0] * levels + 0.5).floor() / levels).max(0.0);
                    let q1 = ((c[1] * levels + 0.5).floor() / levels).max(0.0);
                    let q2 = ((c[2] * levels + 0.5).floor() / levels).max(0.0);
                    c[0] = c[0] + (q0 - c[0]) * posterize;
                    c[1] = c[1] + (q1 - c[1]) * posterize;
                    c[2] = c[2] + (q2 - c[2]) * posterize;
                }
                let luma = c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
                c[0] = luma + (c[0] - luma) * saturation;
                c[1] = luma + (c[1] - luma) * saturation;
                c[2] = luma + (c[2] - luma) * saturation;
            }
            c[0] = c[0].powf(1.0 / gamma);
            c[1] = c[1].powf(1.0 / gamma);
            c[2] = c[2].powf(1.0 / gamma);
            c
        };
        let clear_linear = if self.gp0.x.abs() + self.gp0.y.abs() + self.gp0.z.abs() > 0.001 {
            [self.gp0.x, self.gp0.y, self.gp0.z]
        } else {
            [self.background.x, self.background.y, self.background.z]
        };
        let clear = {
            let p = apply_post_cpu(clear_linear);
            [
                p[0].clamp(0.0, 1.0),
                p[1].clamp(0.0, 1.0),
                p[2].clamp(0.0, 1.0),
            ]
        };

        {
            let g = self.gpu.as_mut().unwrap();
            queue.write_buffer(
                g.u_raster2d_buf.as_ref().unwrap(),
                0,
                bytemuck::bytes_of(&u),
            );
            g.u_raster2d_bg = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vm-raster2d-bg"),
                layout: g.u_raster2d_bgl.as_ref().unwrap(),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: g.u_raster2d_buf.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&atlas_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&g.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&atlas_mat_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: g.tile_meta_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: g.tile_frames_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: g.scene_data_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                ],
            }));
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vm-2d-raster-enc"),
        });
        {
            let g = self.gpu.as_ref().unwrap();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vm-2d-raster-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &write_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0] as f64,
                            g: clear[1] as f64,
                            b: clear[2] as f64,
                            a: if self.layer_index == 0 {
                                1.0
                            } else {
                                self.background.w.clamp(0.0, 1.0) as f64
                            },
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if let Some([x, y, w, h]) = self.viewport_rect2d
                && w > 0.0
                && h > 0.0
            {
                let sx = x.max(0.0).min(fb_w as f32) as u32;
                let sy = y.max(0.0).min(fb_h as f32) as u32;
                let sw = w.max(0.0).min((fb_w as f32) - sx as f32) as u32;
                let sh = h.max(0.0).min((fb_h as f32) - sy as f32) as u32;
                pass.set_scissor_rect(sx, sy, sw.max(1), sh.max(1));
            }

            pass.set_pipeline(g.raster2d_pipeline.as_ref().unwrap());
            pass.set_bind_group(0, g.u_raster2d_bg.as_ref().unwrap(), &[]);
            pass.set_vertex_buffer(0, g.v2d_ssbo.as_ref().unwrap().slice(..));
            pass.set_index_buffer(
                g.i2d_ssbo.as_ref().unwrap().slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..self.cached_i2.len() as u32, 0, 0..1);
        }
        queue.submit(Some(encoder.finish()));
        if self.ping_pong_enabled {
            self.ping_pong_front = next_front;
        }
        Ok(())
    }

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
        self.ensure_compute3d_pipeline(device)?;
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
        self.upload_scene_data_ssbo(device, queue);

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
                            tile_index2: 0,
                            blend_factor: 0.0,
                            _pad0: 0,
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
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::VERTEX
                            | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                g.i2d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-2d-indices-ssbo"),
                        contents: indices_bytes,
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::INDEX
                            | wgpu::BufferUsages::COPY_DST,
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
        self.upload_scene_data_ssbo(device, queue);

        // --- Build 3D geometry only when accel_dirty says so ---
        let mut geometry_changed = false;
        let mut visibility_changed = false;
        if self.accel_dirty || self.geometry3d_dirty || self.cached_v3.is_empty() {
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
                        let mut tile_index2 = if let Some(tid2) = poly.tile_id2 {
                            self.shared_atlas.tile_index(&tid2).unwrap_or(tile_index)
                        } else {
                            tile_index
                        };
                        if poly_uses_clamped_uv(poly) {
                            tile_index2 |= TILE_INDEX_CLAMP_UV_FLAG_RUST;
                        }

                        // Validate blend_weights length matches vertices
                        let has_valid_blend = poly.tile_id2.is_some()
                            && poly.blend_weights.len() == poly.vertices.len();
                        let surface_noise = poly
                            .surface_noise
                            .map(|noise| [noise.scale, noise.amount, noise.seed, 1.0])
                            .unwrap_or([0.0, 0.0, 0.0, 0.0]);

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
                                _pad0: 0.0,
                                uv: [uv0[0], uv0[1]],
                                _pad1: [0.0, 0.0],
                                tile_index,
                                tile_index2,
                                blend_factor,
                                opacity: poly.opacity,
                                normal: [n[0], n[1], n[2]],
                                _pad2: 0.0,
                                surface_noise,
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
                    _pad0: 0.0,
                    uv: [0.0; 2],
                    _pad1: [0.0, 0.0],
                    tile_index: 0,
                    tile_index2: 0,
                    blend_factor: 0.0,
                    opacity: 1.0,
                    normal: [0.0, 0.0, 1.0],
                    _pad2: 0.0,
                    surface_noise: [0.0, 0.0, 0.0, 0.0],
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
            visibility_changed = true;
            self.visibility_dirty = false; // Reset since we just rebuilt everything
            self.geometry3d_dirty = false;
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
            let need_grid_upload =
                grid_changed || visibility_changed || g.grid_hdr.is_none() || g.grid_data.is_none();
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
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::VERTEX
                            | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                g.i3d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-3d-indices-ssbo"),
                        contents: bytemuck::cast_slice(&self.cached_i3),
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::INDEX
                            | wgpu::BufferUsages::COPY_DST,
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

    pub fn raster_draw_3d_into(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _surface: &mut Texture,
        fb_w: u32,
        fb_h: u32,
    ) -> crate::SceneVMResult<()> {
        let debug_total_start = instant::Instant::now();
        let mut debug_geometry_ms = 0.0;
        let mut debug_visibility_ms = 0.0;

        let debug_init_start = instant::Instant::now();
        if self.gpu.is_none() {
            self.init_gpu(device)?;
        }
        self.init_raster3d(device)?;
        let debug_init_ms = debug_init_start.elapsed().as_secs_f64() * 1000.0;

        let debug_prepare_start = instant::Instant::now();
        self.upload_tile_metadata_to_gpu(device);
        self.upload_scene_data_ssbo(device, queue);
        let (write_view, _prev_view, next_front) =
            self.prepare_layer_views(device, queue, fb_w, fb_h);

        let c = self.camera3d;

        use wgpu::util::DeviceExt;

        // Ensure tile indices are current before we resolve dynamic billboard tile ids.
        self.build_atlas();
        let debug_prepare_ms = debug_prepare_start.elapsed().as_secs_f64() * 1000.0;

        let m = self.transform3d;
        let mut geometry_changed = false;
        let has_dynamic_objects =
            !self.dynamic_objects.is_empty() || !self.dynamic_avatar_objects.is_empty();
        let need_dynamic_refresh = has_dynamic_objects || self.raster_had_dynamics_last_frame;
        let mut static_geometry_rebuilt = false;
        if self.accel_dirty
            || self.geometry3d_dirty
            || self.cached_v3.is_empty()
            || need_dynamic_refresh
        {
            let debug_geometry_start = instant::Instant::now();
            let rebuild_static_geometry = self.accel_dirty
                || self.geometry3d_dirty
                || self.cached_static_v3.is_empty()
                || self.cached_static_i3.is_empty();
            let mut v3: Vec<Vert3DPod>;
            let mut i3: Vec<u32>;
            let mut tri_visibility: Vec<bool>;
            let mut tri_geo_ids: Vec<GeoId>;

            if rebuild_static_geometry {
                static_geometry_rebuilt = true;
                v3 = Vec::new();
                i3 = Vec::new();
                tri_visibility = Vec::new();
                tri_geo_ids = Vec::new();

                for ch in self.chunks_map.values() {
                    for poly_list in ch.polys3d_map.values() {
                        for poly in poly_list {
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
                            let mut tile_index2 = if let Some(tid2) = poly.tile_id2 {
                                self.shared_atlas.tile_index(&tid2).unwrap_or(tile_index)
                            } else {
                                tile_index
                            };
                            if poly_uses_clamped_uv(poly) {
                                tile_index2 |= TILE_INDEX_CLAMP_UV_FLAG_RUST;
                            }
                            let has_valid_blend = poly.tile_id2.is_some()
                                && poly.blend_weights.len() == poly.vertices.len();
                            let surface_noise = poly
                                .surface_noise
                                .map(|noise| [noise.scale, noise.amount, noise.seed, 1.0])
                                .unwrap_or([0.0, 0.0, 0.0, 0.0]);

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
                                    _pad0: 0.0,
                                    uv: [uv0[0], uv0[1]],
                                    _pad1: [0.0, 0.0],
                                    tile_index,
                                    tile_index2,
                                    blend_factor,
                                    opacity: poly.opacity,
                                    normal: [n[0], n[1], n[2]],
                                    _pad2: 0.0,
                                    surface_noise,
                                });
                            }

                            for &(a, b, c) in &poly.indices {
                                i3.extend_from_slice(&[
                                    base + a as u32,
                                    base + b as u32,
                                    base + c as u32,
                                ]);
                                tri_visibility.push(poly.visible);
                                tri_geo_ids.push(poly.id);
                            }
                        }
                    }
                }

                self.cached_static_v3 = v3;
                self.cached_static_i3 = i3;
                self.cached_static_tri_visibility = tri_visibility;
                self.cached_static_tri_geo_ids = tri_geo_ids;
                self.cached_static_raster_indices_valid = false;
            }

            let static_vertex_count = self.cached_static_v3.len();
            let static_index_count = self.cached_static_i3.len();
            let static_tri_count = self.cached_static_tri_geo_ids.len();
            if rebuild_static_geometry
                || self.cached_v3.len() < static_vertex_count
                || self.cached_i3.len() < static_index_count
                || self.cached_tri_geo_ids.len() < static_tri_count
            {
                v3 = self.cached_static_v3.clone();
                i3 = self.cached_static_i3.clone();
                tri_geo_ids = self.cached_static_tri_geo_ids.clone();
            } else {
                v3 = std::mem::take(&mut self.cached_v3);
                v3.truncate(static_vertex_count);
                i3 = std::mem::take(&mut self.cached_i3);
                i3.truncate(static_index_count);
                tri_geo_ids = std::mem::take(&mut self.cached_tri_geo_ids);
                tri_geo_ids.truncate(static_tri_count);
            }
            tri_visibility = self.cached_static_tri_visibility.clone();

            // Dynamic billboards (tile + avatar) as camera-facing quads in world space.
            let dynamic_objs = self.sorted_dynamic_objects();
            let avatar_meta_indices = self.avatar_meta_indices_for_objects(&dynamic_objs);
            for obj in dynamic_objs {
                let (tile_index, mut tile_index2) = match obj.kind {
                    DynamicKind::BillboardTile | DynamicKind::ParticleBillboard => {
                        let Some(tile_id) = obj.tile_id else { continue };
                        let Some(tile_index) = self.shared_atlas.tile_index(&tile_id) else {
                            continue;
                        };
                        let mut tile_index2 = tile_index;
                        if obj.kind == DynamicKind::ParticleBillboard {
                            tile_index2 |= TILE_INDEX_PARTICLE_FLAG_RUST;
                        }
                        (tile_index, tile_index2)
                    }
                    DynamicKind::BillboardAvatar => {
                        // tile_index stores avatar meta index for raster path
                        // (resolved in WGSL via scene_data SSBO).
                        let Some(avatar_index) = avatar_meta_indices.get(&obj.id).copied() else {
                            continue;
                        };
                        (avatar_index, 0x8000_0000u32)
                    }
                    DynamicKind::Mesh => continue,
                };
                // For non-repeating billboards, clamp UVs in shader to avoid MSAA edge wrap seams.
                if !matches!(obj.repeat_mode, crate::dynamic::RepeatMode::Repeat) {
                    tile_index2 |= 0x4000_0000u32;
                }
                tile_index2 |= 0x2000_0000u32;
                if matches!(obj.id, GeoId::Hole(_, _)) {
                    tile_index2 |= 0x1000_0000u32;
                }
                let right = obj.view_right * (obj.width * 0.5);
                let up = obj.view_up * (obj.height * 0.5);
                let p0 = obj.center - right - up;
                let p1 = obj.center - right + up;
                let p2 = obj.center + right + up;
                let p3 = obj.center + right - up;
                let mut n = right.cross(up);
                if n.magnitude_squared() <= 1e-8 {
                    n = Vec3::new(0.0, 1.0, 0.0);
                } else {
                    n = n.normalized();
                }
                let normal_or_tint = if obj.kind == DynamicKind::ParticleBillboard {
                    obj.tint
                } else {
                    n
                };
                let base = v3.len() as u32;
                let opacity = obj.opacity.clamp(0.0, 1.0);
                let pts = [p0, p1, p2, p3];
                let uvs = if matches!(obj.repeat_mode, crate::dynamic::RepeatMode::Repeat) {
                    [
                        [0.0f32, obj.height],
                        [0.0, 0.0],
                        [obj.width, 0.0],
                        [obj.width, obj.height],
                    ]
                } else {
                    [[0.0f32, 1.0f32], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]
                };
                for i in 0..4 {
                    let p = pts[i];
                    v3.push(Vert3DPod {
                        pos: [p.x, p.y, p.z],
                        _pad0: 0.0,
                        uv: uvs[i],
                        _pad1: [0.0, 0.0],
                        tile_index,
                        tile_index2,
                        blend_factor: 0.0,
                        opacity,
                        normal: [normal_or_tint.x, normal_or_tint.y, normal_or_tint.z],
                        _pad2: 0.0,
                        surface_noise: [0.0, 0.0, 0.0, 0.0],
                    });
                }
                i3.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                tri_visibility.push(true);
                tri_visibility.push(true);
                tri_geo_ids.push(obj.id);
                tri_geo_ids.push(obj.id);
            }

            for obj in &self.dynamic_objects {
                if obj.kind != DynamicKind::Mesh {
                    continue;
                }
                let Some(tile_id) = obj.tile_id else { continue };
                let Some(tile_index) = self.shared_atlas.tile_index(&tile_id) else {
                    continue;
                };
                let tile_index2 = tile_index | 0x2000_0000u32;
                let opacity = obj.opacity.clamp(0.0, 1.0);
                let base = v3.len() as u32;

                for vert in &obj.mesh_vertices {
                    let p = m * Vec4::new(vert.position.x, vert.position.y, vert.position.z, 1.0);
                    let w = if p.w != 0.0 { p.w } else { 1.0 };
                    let n = m * Vec4::new(vert.normal.x, vert.normal.y, vert.normal.z, 0.0);
                    let mut nn = Vec3::new(n.x, n.y, n.z);
                    if nn.magnitude_squared() <= 1e-8 {
                        nn = Vec3::new(0.0, 1.0, 0.0);
                    } else {
                        nn = nn.normalized();
                    }
                    v3.push(Vert3DPod {
                        pos: [p.x / w, p.y / w, p.z / w],
                        _pad0: 0.0,
                        uv: [vert.uv.x, vert.uv.y],
                        _pad1: [0.0, 0.0],
                        tile_index,
                        tile_index2,
                        blend_factor: 0.0,
                        opacity,
                        normal: [nn.x, nn.y, nn.z],
                        _pad2: 0.0,
                        surface_noise: [0.0, 0.0, 0.0, 0.0],
                    });
                }
                for tri in obj.mesh_indices.chunks_exact(3) {
                    i3.extend_from_slice(&[base + tri[0], base + tri[1], base + tri[2]]);
                    tri_visibility.push(true);
                    tri_geo_ids.push(obj.id);
                }
            }

            if v3.is_empty() {
                v3.push(Vert3DPod {
                    pos: [0.0; 3],
                    _pad0: 0.0,
                    uv: [0.0; 2],
                    _pad1: [0.0, 0.0],
                    tile_index: 0,
                    tile_index2: 0,
                    blend_factor: 0.0,
                    opacity: 1.0,
                    normal: [0.0, 0.0, 1.0],
                    _pad2: 0.0,
                    surface_noise: [0.0, 0.0, 0.0, 0.0],
                });
            }
            if i3.is_empty() {
                i3.extend_from_slice(&[0u32; 4]);
                tri_visibility.push(false);
                tri_geo_ids.push(GeoId::Unknown(0));
            }

            self.cached_v3 = v3;
            self.cached_i3 = i3;
            self.cached_tri_geo_ids = tri_geo_ids;

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
            if static_geometry_rebuilt {
                self.cached_static_raster_indices_valid = false;
            }

            self.mark_irradiance_grid_dirty();
            geometry_changed = true;
            self.visibility_dirty = false;
            self.geometry3d_dirty = false;
            self.raster_had_dynamics_last_frame = has_dynamic_objects;
            debug_geometry_ms = debug_geometry_start.elapsed().as_secs_f64() * 1000.0;
        }

        if self.visibility_dirty && !geometry_changed {
            let debug_visibility_start = instant::Instant::now();
            let mut tri_visibility: Vec<bool> = Vec::new();
            for ch in self.chunks_map.values() {
                for poly_list in ch.polys3d_map.values() {
                    for poly in poly_list {
                        for _ in &poly.indices {
                            tri_visibility.push(poly.visible);
                        }
                    }
                }
            }
            if tri_visibility.is_empty() {
                tri_visibility.push(false);
            }
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
            self.cached_static_raster_indices_valid = false;
            self.mark_irradiance_grid_dirty();
            self.visibility_dirty = false;
            debug_visibility_ms = debug_visibility_start.elapsed().as_secs_f64() * 1000.0;
        }

        if self.accel_dirty {
            self.scene_accel.bvh =
                Self::build_scene_bvh_from(&self.cached_v3, &self.cached_i3, self.bvh_leaf_size);
            self.accel_dirty = false;
        }

        let mut line3d_changed = false;
        if self.line3d_dirty {
            let mut layered_lines = Vec::new();
            for ch in self.chunks_map.values() {
                for line_list in ch.lines3d.values() {
                    for line in line_list {
                        if !line.visible {
                            continue;
                        }

                        let a = m * Vec4::new(line.a.x, line.a.y, line.a.z, 1.0);
                        let b = m * Vec4::new(line.b.x, line.b.y, line.b.z, 1.0);
                        let aw = if a.w != 0.0 { a.w } else { 1.0 };
                        let bw = if b.w != 0.0 { b.w } else { 1.0 };
                        layered_lines.push((
                            line.layer,
                            Line3DPod {
                                pos: [a.x / aw, a.y / aw, a.z / aw],
                                _pad0: 0.0,
                                color: line.color,
                            },
                            Line3DPod {
                                pos: [b.x / bw, b.y / bw, b.z / bw],
                                _pad0: 0.0,
                                color: line.color,
                            },
                        ));
                    }
                }
            }
            layered_lines.sort_by_key(|(layer, _, _)| *layer);
            let mut lines = Vec::with_capacity(layered_lines.len() * 2);
            for (_, a, b) in layered_lines {
                lines.push(a);
                lines.push(b);
            }
            self.cached_line3d = lines;
            self.line3d_dirty = false;
            line3d_changed = true;
        }

        // Keep static atlas/tile metadata in sync.
        self.upload_atlas_to_gpu_with(device, queue);
        self.upload_tile_metadata_to_gpu(device);

        let (atlas_view, atlas_mat_view) = self
            .shared_atlas
            .texture_views()
            .expect("atlas GPU resources missing");
        let debug_visibility_start = instant::Instant::now();
        let (visible_indices, opaque_indices, transparent_indices, particle_indices) =
            self.rebuild_raster_visible_indices(&c);
        debug_visibility_ms += debug_visibility_start.elapsed().as_secs_f64() * 1000.0;

        let debug_visible_count = visible_indices.len();
        let debug_opaque_count = opaque_indices.len();
        let debug_transparent_count = transparent_indices.len();
        let debug_particle_count = particle_indices.len();

        let shadow_res = self.gp7.z.round().clamp(256.0, 4096.0) as u32;
        let raster_samples = self.raster3d_effective_samples();
        let use_msaa = raster_samples > 1;
        let shadow_enabled = !self.cached_i3.is_empty() && self.gp2.w > 0.5 && self.gp7.x > 0.5;

        let shadow_bounds_verts = if !self.cached_static_v3.is_empty() {
            &self.cached_static_v3
        } else {
            &self.cached_v3
        };
        let mut shadow_center = Vec3::zero();
        let mut shadow_half_w = 32.0f32;
        let mut shadow_half_h = 32.0f32;
        let mut shadow_near = -32.0f32;
        let mut shadow_far = 32.0f32;
        if !shadow_bounds_verts.is_empty() {
            let mut bmin = Vec3::broadcast(f32::INFINITY);
            let mut bmax = Vec3::broadcast(f32::NEG_INFINITY);
            for v in shadow_bounds_verts {
                let p = Vec3::new(v.pos[0], v.pos[1], v.pos[2]);
                bmin.x = bmin.x.min(p.x);
                bmin.y = bmin.y.min(p.y);
                bmin.z = bmin.z.min(p.z);
                bmax.x = bmax.x.max(p.x);
                bmax.y = bmax.y.max(p.y);
                bmax.z = bmax.z.max(p.z);
            }
            shadow_center = (bmin + bmax) * 0.5;
        }
        let sun_l_raw = Vec3::new(-self.gp2.x, -self.gp2.y, -self.gp2.z);
        let sun_l = if sun_l_raw.magnitude_squared() > 1e-6 {
            sun_l_raw.normalized()
        } else {
            Vec3::new(0.0, -1.0, 0.0)
        };
        let shadow_fwd = -sun_l;
        let up_hint = if shadow_fwd.y.abs() > 0.99 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let shadow_right = up_hint.cross(shadow_fwd).normalized();
        let shadow_up = shadow_fwd.cross(shadow_right).normalized();
        if !shadow_bounds_verts.is_empty() {
            let mut minx = f32::INFINITY;
            let mut maxx = f32::NEG_INFINITY;
            let mut miny = f32::INFINITY;
            let mut maxy = f32::NEG_INFINITY;
            let mut minz = f32::INFINITY;
            let mut maxz = f32::NEG_INFINITY;
            for v in shadow_bounds_verts {
                let p = Vec3::new(v.pos[0], v.pos[1], v.pos[2]);
                let rel = p - shadow_center;
                let x = rel.dot(shadow_right);
                let y = rel.dot(shadow_up);
                let z = rel.dot(shadow_fwd);
                minx = minx.min(x);
                maxx = maxx.max(x);
                miny = miny.min(y);
                maxy = maxy.max(y);
                minz = minz.min(z);
                maxz = maxz.max(z);
            }
            shadow_half_w = ((maxx - minx) * 0.5 + 2.0).max(1.0);
            shadow_half_h = ((maxy - miny) * 0.5 + 2.0).max(1.0);
            shadow_near = minz - 4.0;
            shadow_far = (maxz + 4.0).max(shadow_near + 1.0);

            let texel_w = (shadow_half_w * 2.0) / (shadow_res as f32).max(1.0);
            let texel_h = (shadow_half_h * 2.0) / (shadow_res as f32).max(1.0);
            if texel_w.is_finite() && texel_w > 0.0 && texel_h.is_finite() && texel_h > 0.0 {
                let center_x = shadow_center.dot(shadow_right);
                let center_y = shadow_center.dot(shadow_up);
                let snapped_x = (center_x / texel_w).round() * texel_w;
                let snapped_y = (center_y / texel_h).round() * texel_h;
                shadow_center +=
                    shadow_right * (snapped_x - center_x) + shadow_up * (snapped_y - center_y);
            }
        }

        let mut ranked_lights: Vec<RasterPointLight> = self
            .lights
            .values()
            .filter(|l| l.emitting && matches!(l.light_type, LightType::Point))
            .map(|light| {
                let score = light.intensity * light.end_distance.max(light.radius).max(0.1);
                let flicker_multiplier: f32 = if light.flicker > 0.0 {
                    let hash = hash_u32(self.animation_counter as u32);
                    let combined_hash = hash.wrapping_add(
                        (light.position.x as u32
                            + light.position.y as u32
                            + light.position.z as u32)
                            * 100,
                    );
                    let flicker_value = (combined_hash as f32 / u32::MAX as f32).clamp(0.0, 1.0);
                    1.0 - flicker_value * light.flicker
                } else {
                    1.0
                };
                RasterPointLight {
                    position: light.position,
                    color: light.color,
                    intensity: light.intensity * flicker_multiplier,
                    range: light.end_distance,
                    score: score * 1.15,
                }
            })
            .collect();
        let emissive_lighting = self.collect_emissive_surface_lighting(
            &visible_indices,
            &c,
            if EMISSIVE_SURFACE_POINT_LIGHTS_ENABLED {
                EMISSIVE_SURFACE_LIGHT_BUDGET.min(RASTER3D_MAX_POINT_LIGHTS)
            } else {
                0
            },
        );
        ranked_lights.extend(emissive_lighting.point_lights.iter().copied());
        ranked_lights.sort_by(|a, b| b.score.total_cmp(&a.score));
        let mut point_light_pos_intensity = [[0.0; 4]; RASTER3D_MAX_POINT_LIGHTS];
        let mut point_light_color_range = [[0.0; 4]; RASTER3D_MAX_POINT_LIGHTS];
        let point_count = ranked_lights.len().min(RASTER3D_MAX_POINT_LIGHTS);
        for i in 0..point_count {
            let light = ranked_lights[i];
            point_light_pos_intensity[i] = [
                light.position.x,
                light.position.y,
                light.position.z,
                light.intensity,
            ];
            point_light_color_range[i] = [light.color.x, light.color.y, light.color.z, light.range];
        }
        let mut post_style1 = self.raster3d_post_style1.into_array();
        post_style1[1] = emissive_lighting.broad_color.x;
        post_style1[2] = emissive_lighting.broad_color.y;
        post_style1[3] = emissive_lighting.broad_color.z;

        let u = Raster3DUniforms {
            cam_pos: [c.pos.x, c.pos.y, c.pos.z, 0.0],
            cam_fwd: [c.forward.x, c.forward.y, c.forward.z, 0.0],
            cam_right: [c.right.x, c.right.y, c.right.z, 0.0],
            cam_up: [c.up.x, c.up.y, c.up.z, 0.0],
            sun_color_intensity: self.gp1.into_array(),
            sun_dir_enabled: self.gp2.into_array(),
            ambient_color_strength: self.gp3.into_array(),
            sky_color: self.gp0.into_array(),
            fog_color_density: self.gp4.into_array(),
            shadow_light_right: [shadow_right.x, shadow_right.y, shadow_right.z, 0.0],
            shadow_light_up: [shadow_up.x, shadow_up.y, shadow_up.z, 0.0],
            shadow_light_fwd: [shadow_fwd.x, shadow_fwd.y, shadow_fwd.z, 0.0],
            shadow_light_center: [shadow_center.x, shadow_center.y, shadow_center.z, 0.0],
            shadow_light_extents: [shadow_half_w, shadow_half_h, shadow_near, shadow_far],
            shadow_params: [self.gp7.x, self.gp7.y, self.gp5.z, self.gp7.w],
            render_params: self.gp6.into_array(),
            point_light_pos_intensity,
            point_light_color_range,
            point_light_count: point_count as u32,
            _pad_light_count: [0, 0, 0],
            _pad_lights: [0, 0, 0, 0],
            fb_size: [fb_w as f32, fb_h as f32],
            cam_vfov_deg: c.vfov_deg,
            cam_ortho_half_h: c.ortho_half_h,
            cam_near: c.near,
            cam_far: c.far,
            cam_kind: match c.kind {
                CameraKind::OrthoIso => 0,
                CameraKind::OrbitPersp => 1,
                CameraKind::FirstPersonPersp => 2,
            },
            anim_counter: self.animation_counter as u32,
            _pad: [self.gp8.x.max(0.0) as u32, self.gp8.y.max(0.0) as u32],
            _pad_post_pre: [0, 0],
            post_params: [
                self.gp9.x,
                self.gp9.y,
                self.gp9.z.max(0.0),
                self.gp9.w.max(0.001),
            ],
            post_color_adjust: [self.gp8.z.max(0.0), self.gp8.w.max(0.0), 1.0, 0.0],
            post_style0: self.raster3d_post_style0.into_array(),
            post_style1,
            avatar_highlight_params: self.raster3d_avatar_highlight_params.into_array(),
            _pad_tail: [0, 0, 0, 0],
            palette: self.palette,
            palette_tile_indices: self.palette_tile_indices_uniform(),
            organic_params: [self.organic_visible as u32, 0, 0, 0],
        };

        let debug_upload_start = instant::Instant::now();
        {
            self.upload_organic_billboard_ssbo(device, queue);
            self.upload_irradiance_grid_ssbo(device, queue);
            self.upload_material_table_ssbo(device, queue);
            self.upload_raster3d_paint_overlay(device, queue);
            let g = self.gpu.as_mut().unwrap();
            g.ensure_raster3d_targets(device, fb_w, fb_h, shadow_res, raster_samples);
            queue.write_buffer(
                g.u_raster3d_buf.as_ref().unwrap(),
                0,
                bytemuck::bytes_of(&u),
            );
            let vertex_bytes = (self.cached_v3.len() * std::mem::size_of::<Vert3DPod>()) as u64;
            let index_bytes = (self.cached_i3.len() * std::mem::size_of::<u32>()) as u64;
            let need_full_geom_upload = static_geometry_rebuilt
                || g.v3d_ssbo.is_none()
                || g.i3d_ssbo.is_none()
                || vertex_bytes > g.v3d_ssbo_capacity
                || index_bytes > g.i3d_ssbo_capacity;
            if need_full_geom_upload {
                g.v3d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-3d-verts-raster"),
                        contents: bytemuck::cast_slice(&self.cached_v3),
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::VERTEX
                            | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                g.v3d_ssbo_capacity = vertex_bytes;
                g.i3d_ssbo = Some(
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vm-3d-indices-raster-all"),
                        contents: bytemuck::cast_slice(&self.cached_i3),
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::INDEX
                            | wgpu::BufferUsages::COPY_DST,
                    }),
                );
                g.i3d_ssbo_capacity = index_bytes;
            } else if geometry_changed {
                let static_vertex_count = self.cached_static_v3.len().min(self.cached_v3.len());
                let static_index_count = self.cached_static_i3.len().min(self.cached_i3.len());
                if static_vertex_count < self.cached_v3.len() {
                    let offset = (static_vertex_count * std::mem::size_of::<Vert3DPod>()) as u64;
                    queue.write_buffer(
                        g.v3d_ssbo.as_ref().unwrap(),
                        offset,
                        bytemuck::cast_slice(&self.cached_v3[static_vertex_count..]),
                    );
                }
                if static_index_count < self.cached_i3.len() {
                    let offset = (static_index_count * std::mem::size_of::<u32>()) as u64;
                    queue.write_buffer(
                        g.i3d_ssbo.as_ref().unwrap(),
                        offset,
                        bytemuck::cast_slice(&self.cached_i3[static_index_count..]),
                    );
                }
            }

            if line3d_changed || g.line3d_raster.is_none() {
                let line_upload = if self.cached_line3d.is_empty() {
                    vec![Line3DPod {
                        pos: [0.0, 0.0, 0.0],
                        _pad0: 0.0,
                        color: [0.0, 0.0, 0.0, 0.0],
                    }]
                } else {
                    self.cached_line3d.clone()
                };
                g.line3d_raster_count = self.cached_line3d.len() as u32;
                g.line3d_raster = Some(device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("vm-3d-lines-raster"),
                        contents: bytemuck::cast_slice(&line_upload),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    },
                ));
                g.line3d_raster_capacity =
                    (line_upload.len() * std::mem::size_of::<Line3DPod>()) as u64;
            }

            let visible_upload = if visible_indices.is_empty() {
                vec![0u32]
            } else {
                visible_indices
            };
            g.i3d_raster_count = if visible_upload.len() == 1 {
                0
            } else {
                visible_upload.len() as u32
            };
            VMGpu::update_or_create_index_buffer(
                device,
                queue,
                &mut g.i3d_raster,
                &mut g.i3d_raster_capacity,
                "vm-3d-indices-raster-visible",
                &visible_upload,
            );

            let opaque_upload = if opaque_indices.is_empty() {
                vec![0u32]
            } else {
                opaque_indices
            };
            g.i3d_raster_opaque_count = if opaque_upload.len() == 1 {
                0
            } else {
                opaque_upload.len() as u32
            };
            VMGpu::update_or_create_index_buffer(
                device,
                queue,
                &mut g.i3d_raster_opaque,
                &mut g.i3d_raster_opaque_capacity,
                "vm-3d-indices-raster-opaque",
                &opaque_upload,
            );

            let transparent_upload = if transparent_indices.is_empty() {
                vec![0u32]
            } else {
                transparent_indices
            };
            g.i3d_raster_transparent_count = if transparent_upload.len() == 1 {
                0
            } else {
                transparent_upload.len() as u32
            };
            VMGpu::update_or_create_index_buffer(
                device,
                queue,
                &mut g.i3d_raster_transparent,
                &mut g.i3d_raster_transparent_capacity,
                "vm-3d-indices-raster-transparent",
                &transparent_upload,
            );

            let particle_upload = if particle_indices.is_empty() {
                vec![0u32]
            } else {
                particle_indices
            };
            g.i3d_raster_particles_count = if particle_upload.len() == 1 {
                0
            } else {
                particle_upload.len() as u32
            };
            VMGpu::update_or_create_index_buffer(
                device,
                queue,
                &mut g.i3d_raster_particles,
                &mut g.i3d_raster_particles_capacity,
                "vm-3d-indices-raster-particles",
                &particle_upload,
            );

            g.u_raster3d_shadow_bg = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vm-raster3d-shadow-bg"),
                layout: g.u_raster3d_shadow_bgl.as_ref().unwrap(),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: g.u_raster3d_buf.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&atlas_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&g.sampler_raster),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: g.tile_meta_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: g.tile_frames_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(&atlas_mat_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: g.scene_data_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: g.irradiance_grid_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 12,
                        resource: g.material_table_ssbo.as_ref().unwrap().as_entire_binding(),
                    },
                ],
            }));
            g.u_raster3d_bg = Some(
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("vm-raster3d-bg"),
                    layout: g.u_raster3d_bgl.as_ref().unwrap(),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: g.u_raster3d_buf.as_ref().unwrap().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&atlas_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&g.sampler_raster),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: g.tile_meta_ssbo.as_ref().unwrap().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: g.tile_frames_ssbo.as_ref().unwrap().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::TextureView(
                                g.raster3d_shadow_view.as_ref().unwrap(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: wgpu::BindingResource::Sampler(
                                g.shadow_sampler_compare.as_ref().unwrap(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 7,
                            resource: wgpu::BindingResource::TextureView(&atlas_mat_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 8,
                            resource: g.scene_data_ssbo.as_ref().unwrap().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 10,
                            resource: g
                                .organic_billboard_ssbo
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 11,
                            resource: g.irradiance_grid_ssbo.as_ref().unwrap().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 12,
                            resource: g.material_table_ssbo.as_ref().unwrap().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 13,
                            resource: wgpu::BindingResource::TextureView(
                                g.raster3d_paint_color_view.as_ref().unwrap(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 14,
                            resource: wgpu::BindingResource::TextureView(
                                g.raster3d_paint_material_view.as_ref().unwrap(),
                            ),
                        },
                    ],
                }),
            );
        }
        let debug_upload_ms = debug_upload_start.elapsed().as_secs_f64() * 1000.0;

        let debug_encode_start = instant::Instant::now();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vm-3d-raster-enc"),
        });
        let sky = if self.gp0.x.abs() + self.gp0.y.abs() + self.gp0.z.abs() > 0.01 {
            self.gp0
        } else {
            self.background
        };
        let sky_srgb = [sky.x.max(0.0), sky.y.max(0.0), sky.z.max(0.0)];
        // Overlay VMs must preserve transparency when they clear.
        // Base layer remains opaque by default.
        let clear_alpha = if self.layer_index == 0 {
            1.0
        } else {
            self.background.w.clamp(0.0, 1.0)
        };
        {
            let g = self.gpu.as_ref().unwrap();
            if g.i3d_raster_count > 0 && shadow_enabled {
                let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("vm-3d-raster-shadow-pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: g.raster3d_shadow_view.as_ref().unwrap(),
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                shadow_pass.set_pipeline(g.raster3d_shadow_pipeline.as_ref().unwrap());
                shadow_pass.set_bind_group(0, g.u_raster3d_shadow_bg.as_ref().unwrap(), &[]);
                shadow_pass.set_vertex_buffer(0, g.v3d_ssbo.as_ref().unwrap().slice(..));
                shadow_pass.set_index_buffer(
                    g.i3d_raster.as_ref().unwrap().slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                shadow_pass.draw_indexed(0..g.i3d_raster_count, 0, 0..1);
            }

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vm-3d-raster-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: if use_msaa {
                        g.raster3d_msaa_color_view.as_ref().unwrap()
                    } else {
                        g.raster3d_scene_view.as_ref().unwrap()
                    },
                    resolve_target: if use_msaa {
                        Some(g.raster3d_scene_view.as_ref().unwrap())
                    } else {
                        None
                    },
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: sky_srgb[0] as f64,
                            g: sky_srgb[1] as f64,
                            b: sky_srgb[2] as f64,
                            a: clear_alpha as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: g.raster3d_depth_view.as_ref().unwrap(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some([x, y, w, h]) = self.viewport_rect2d
                && w > 0.0
                && h > 0.0
            {
                let sx = x.max(0.0).min(fb_w as f32) as u32;
                let sy = y.max(0.0).min(fb_h as f32) as u32;
                let sw = w.max(0.0).min((fb_w as f32) - sx as f32) as u32;
                let sh = h.max(0.0).min((fb_h as f32) - sy as f32) as u32;
                pass.set_scissor_rect(sx, sy, sw.max(1), sh.max(1));
                pass.set_viewport(
                    sx as f32,
                    sy as f32,
                    sw.max(1) as f32,
                    sh.max(1) as f32,
                    0.0,
                    1.0,
                );
            }
            if g.i3d_raster_opaque_count > 0 {
                pass.set_pipeline(g.raster3d_pipeline.as_ref().unwrap());
                pass.set_bind_group(0, g.u_raster3d_bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, g.v3d_ssbo.as_ref().unwrap().slice(..));
                pass.set_index_buffer(
                    g.i3d_raster_opaque.as_ref().unwrap().slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..g.i3d_raster_opaque_count, 0, 0..1);
            }
            if g.organic_billboard_count > 0 && self.organic_visible {
                pass.set_pipeline(g.raster3d_organic_billboard_pipeline.as_ref().unwrap());
                pass.set_bind_group(0, g.u_raster3d_bg.as_ref().unwrap(), &[]);
                pass.draw(0..g.organic_billboard_count.saturating_mul(6), 0..1);
            }
            if g.i3d_raster_transparent_count > 0 {
                pass.set_pipeline(g.raster3d_alpha_pipeline.as_ref().unwrap());
                pass.set_bind_group(0, g.u_raster3d_bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, g.v3d_ssbo.as_ref().unwrap().slice(..));
                pass.set_index_buffer(
                    g.i3d_raster_transparent.as_ref().unwrap().slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..g.i3d_raster_transparent_count, 0, 0..1);
            }
            if g.i3d_raster_particles_count > 0 {
                pass.set_pipeline(g.raster3d_particle_pipeline.as_ref().unwrap());
                pass.set_bind_group(0, g.u_raster3d_bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, g.v3d_ssbo.as_ref().unwrap().slice(..));
                pass.set_index_buffer(
                    g.i3d_raster_particles.as_ref().unwrap().slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..g.i3d_raster_particles_count, 0, 0..1);
            }
            if g.line3d_raster_count > 0 {
                pass.set_pipeline(g.raster3d_line_pipeline.as_ref().unwrap());
                pass.set_bind_group(0, g.u_raster3d_bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, g.line3d_raster.as_ref().unwrap().slice(..));
                pass.draw(0..g.line3d_raster_count, 0..1);
            }
            drop(pass);

            let post_extract_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vm-raster3d-bloom-extract-bg"),
                layout: g.u_raster3d_post_bgl.as_ref().unwrap(),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: g.u_raster3d_buf.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            g.raster3d_scene_view.as_ref().unwrap(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            g.raster3d_scene_view.as_ref().unwrap(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&g.sampler_linear),
                    },
                ],
            });
            {
                let mut bloom_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("vm-3d-bloom-extract-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: g.raster3d_bloom_view.as_ref().unwrap(),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                bloom_pass.set_pipeline(g.raster3d_bloom_extract_pipeline.as_ref().unwrap());
                bloom_pass.set_bind_group(0, &post_extract_bg, &[]);
                bloom_pass.draw(0..3, 0..1);
            }

            let post_composite_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vm-raster3d-bloom-composite-bg"),
                layout: g.u_raster3d_post_bgl.as_ref().unwrap(),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: g.u_raster3d_buf.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            g.raster3d_scene_view.as_ref().unwrap(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            g.raster3d_bloom_view.as_ref().unwrap(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&g.sampler_linear),
                    },
                ],
            });
            let mut composite_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vm-3d-bloom-composite-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &write_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            composite_pass.set_pipeline(g.raster3d_bloom_composite_pipeline.as_ref().unwrap());
            composite_pass.set_bind_group(0, &post_composite_bg, &[]);
            composite_pass.draw(0..3, 0..1);
        }
        let debug_encode_ms = debug_encode_start.elapsed().as_secs_f64() * 1000.0;
        let debug_submit_start = instant::Instant::now();
        queue.submit(Some(encoder.finish()));
        let debug_submit_ms = debug_submit_start.elapsed().as_secs_f64() * 1000.0;
        if self.ping_pong_enabled {
            self.ping_pong_front = next_front;
        }
        record_raster3d_debug_timing(
            (fb_w, fb_h),
            debug_init_ms,
            debug_prepare_ms,
            debug_geometry_ms,
            debug_visibility_ms,
            debug_upload_ms,
            debug_encode_ms,
            debug_submit_ms,
            debug_total_start.elapsed().as_secs_f64() * 1000.0,
            self.cached_v3.len(),
            self.cached_i3.len(),
            debug_visible_count,
            debug_opaque_count,
            debug_transparent_count,
            debug_particle_count,
            geometry_changed,
            shadow_enabled,
            shadow_res,
            raster_samples,
            self.gp9.x > 0.5,
            self.gp5.z,
            self.gp7.x,
            self.gp7.y,
        );
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

    pub fn paint_surface_key(&self, fb_w: u32, fb_h: u32) -> u64 {
        let mut hasher = rustc_hash::FxHasher::default();
        fb_w.hash(&mut hasher);
        fb_h.hash(&mut hasher);
        (self.cached_static_v3.len() as u64).hash(&mut hasher);
        (self.cached_static_i3.len() as u64).hash(&mut hasher);
        (self.cached_static_tri_geo_ids.len() as u64).hash(&mut hasher);
        for value in [
            self.camera3d.pos.x,
            self.camera3d.pos.y,
            self.camera3d.pos.z,
            self.camera3d.forward.x,
            self.camera3d.forward.y,
            self.camera3d.forward.z,
            self.camera3d.right.x,
            self.camera3d.right.y,
            self.camera3d.right.z,
            self.camera3d.up.x,
            self.camera3d.up.y,
            self.camera3d.up.z,
            self.camera3d.vfov_deg,
            self.camera3d.ortho_half_h,
            self.camera3d.near,
            self.camera3d.far,
        ] {
            value.to_bits().hash(&mut hasher);
        }
        (self.camera3d.kind as u8).hash(&mut hasher);
        self.cached_static_tri_visibility.hash(&mut hasher);
        if let Some(first) = self.cached_static_tri_geo_ids.first() {
            first.hash(&mut hasher);
        }
        if let Some(mid) = self
            .cached_static_tri_geo_ids
            .get(self.cached_static_tri_geo_ids.len() / 2)
        {
            mid.hash(&mut hasher);
        }
        if let Some(last) = self.cached_static_tri_geo_ids.last() {
            last.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn paint_surface_buffer(&self, fb_w: u32, fb_h: u32) -> PaintSurfaceBuffer {
        let mut buffer = PaintSurfaceBuffer::new(fb_w, fb_h);
        if fb_w == 0
            || fb_h == 0
            || self.cached_static_i3.len() < 3
            || self.cached_static_v3.is_empty()
        {
            return buffer;
        }

        let width = fb_w as i32;
        let height = fb_h as i32;
        let mut depth = vec![f32::INFINITY; fb_w as usize * fb_h as usize];

        for (tri_idx, tri) in self.cached_static_i3.chunks_exact(3).enumerate() {
            if !self.static_triangle_visible(tri_idx) {
                continue;
            }
            let Some(a) = self.cached_static_v3.get(tri[0] as usize).copied() else {
                continue;
            };
            let Some(b) = self.cached_static_v3.get(tri[1] as usize).copied() else {
                continue;
            };
            let Some(c) = self.cached_static_v3.get(tri[2] as usize).copied() else {
                continue;
            };

            let Some((pa, da)) = paint_project_point(&self.camera3d, fb_w, fb_h, a.pos) else {
                continue;
            };
            let Some((pb, db)) = paint_project_point(&self.camera3d, fb_w, fb_h, b.pos) else {
                continue;
            };
            let Some((pc, dc)) = paint_project_point(&self.camera3d, fb_w, fb_h, c.pos) else {
                continue;
            };

            let area = paint_edge(pa, pb, pc);
            if area.abs() <= 1e-5 {
                continue;
            }

            let min_x = pa.x.min(pb.x).min(pc.x).floor().max(0.0) as i32;
            let max_x = pa.x.max(pb.x).max(pc.x).ceil().min((width - 1) as f32) as i32;
            let min_y = pa.y.min(pb.y).min(pc.y).floor().max(0.0) as i32;
            let max_y = pa.y.max(pb.y).max(pc.y).ceil().min((height - 1) as f32) as i32;
            if min_x > max_x || min_y > max_y {
                continue;
            }

            let geo_id = self
                .cached_static_tri_geo_ids
                .get(tri_idx)
                .copied()
                .unwrap_or(GeoId::Unknown(0));
            let face_id = tri_idx as u32;

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = paint_edge(pb, pc, p) / area;
                    let w1 = paint_edge(pc, pa, p) / area;
                    let w2 = paint_edge(pa, pb, p) / area;
                    if w0 < -0.0001 || w1 < -0.0001 || w2 < -0.0001 {
                        continue;
                    }

                    let d = w0 * da + w1 * db + w2 * dc;
                    let index = y as usize * fb_w as usize + x as usize;
                    if d >= depth[index] {
                        continue;
                    }

                    depth[index] = d;
                    let world = [
                        a.pos[0] * w0 + b.pos[0] * w1 + c.pos[0] * w2,
                        a.pos[1] * w0 + b.pos[1] * w1 + c.pos[1] * w2,
                        a.pos[2] * w0 + b.pos[2] * w1 + c.pos[2] * w2,
                    ];
                    let normal_vec = Vec3::new(
                        a.normal[0] * w0 + b.normal[0] * w1 + c.normal[0] * w2,
                        a.normal[1] * w0 + b.normal[1] * w1 + c.normal[1] * w2,
                        a.normal[2] * w0 + b.normal[2] * w1 + c.normal[2] * w2,
                    )
                    .try_normalized()
                    .unwrap_or_else(Vec3::unit_y);
                    let uv = [
                        a.uv[0] * w0 + b.uv[0] * w1 + c.uv[0] * w2,
                        a.uv[1] * w0 + b.uv[1] * w1 + c.uv[1] * w2,
                    ];

                    buffer.pixels[index] = crate::core::PaintSurfacePixel {
                        valid: true,
                        geo_id,
                        face_id,
                        depth: d,
                        world,
                        normal: [normal_vec.x, normal_vec.y, normal_vec.z],
                        uv,
                    };
                }
            }
        }

        buffer
    }

    fn static_triangle_visible(&self, tri_idx: usize) -> bool {
        if self.cached_static_tri_visibility.is_empty() {
            return true;
        }
        self.cached_static_tri_visibility
            .get(tri_idx)
            .copied()
            .unwrap_or(false)
    }

    /// Cast a CPU-side ray through a normalized screen UV and return the hit GeoId (if any).
    /// Uses the same camera model and 3D transforms as the GPU compute path.
    /// Returns the GeoId, world-space hit position, and the distance along the ray.
    pub fn pick_geo_id_normal_at_uv(
        &self,
        fb_w: u32,
        fb_h: u32,
        screen_uv: [f32; 2],
        include_hidden: bool,
        include_billboards: bool,
    ) -> Option<(GeoId, Vec3<f32>, f32, Vec3<f32>)> {
        if fb_w == 0 || fb_h == 0 {
            return None;
        }

        let (ray_origin, ray_dir) = camera_ray_from_uv(&self.camera3d, fb_w, fb_h, screen_uv);
        let mut best_t = f32::INFINITY;
        let mut best_geo: Option<GeoId> = None;
        let mut best_pos = Vec3::new(0.0, 0.0, 0.0);
        let mut best_normal = Vec3::unit_y();

        let m = self.transform3d;

        let cached_static_tri_count = self.cached_static_i3.len() / 3;
        if !self.accel_dirty
            && !self.geometry3d_dirty
            && cached_static_tri_count > 0
            && self.cached_static_tri_geo_ids.len() >= cached_static_tri_count
        {
            let cached_static_i3 = &self.cached_static_i3;
            let cached_static_v3 = &self.cached_static_v3;
            let cached_static_tri_geo_ids = &self.cached_static_tri_geo_ids;
            let cached_static_tri_visibility = &self.cached_static_tri_visibility;

            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some((t, geo, pos, normal)) = cached_static_i3
                    .par_chunks_exact(3)
                    .enumerate()
                    .filter_map(|(tri_idx, tri)| {
                        if !include_hidden
                            && !cached_static_tri_visibility
                                .get(tri_idx)
                                .copied()
                                .unwrap_or(false)
                        {
                            return None;
                        }
                        let a = cached_static_v3.get(tri[0] as usize)?;
                        let b = cached_static_v3.get(tri[1] as usize)?;
                        let c = cached_static_v3.get(tri[2] as usize)?;
                        let a = a.pos;
                        let b = b.pos;
                        let c = c.pos;
                        let normal = triangle_normal(a, b, c)?;
                        let (t, _, _) = ray_triangle_intersect(ray_origin, ray_dir, a, b, c)?;
                        if t <= 1e-5 {
                            return None;
                        }
                        let geo = cached_static_tri_geo_ids.get(tri_idx).copied()?;
                        Some((t, geo, ray_origin + ray_dir * t, normal))
                    })
                    .reduce_with(|a, b| if a.0 <= b.0 { a } else { b })
                {
                    best_t = t;
                    best_geo = Some(geo);
                    best_pos = pos;
                    best_normal = normal;
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                for (tri_idx, tri) in cached_static_i3.chunks_exact(3).enumerate() {
                    if !include_hidden
                        && !cached_static_tri_visibility
                            .get(tri_idx)
                            .copied()
                            .unwrap_or(false)
                    {
                        continue;
                    }
                    let a = cached_static_v3.get(tri[0] as usize);
                    let b = cached_static_v3.get(tri[1] as usize);
                    let c = cached_static_v3.get(tri[2] as usize);
                    let (a, b, c) = match (a, b, c) {
                        (Some(a), Some(b), Some(c)) => (a.pos, b.pos, c.pos),
                        _ => continue,
                    };
                    let Some(normal) = triangle_normal(a, b, c) else {
                        continue;
                    };
                    let Some((t, _, _)) = ray_triangle_intersect(ray_origin, ray_dir, a, b, c)
                    else {
                        continue;
                    };
                    if t <= 1e-5 || t >= best_t {
                        continue;
                    }
                    let Some(geo) = cached_static_tri_geo_ids.get(tri_idx).copied() else {
                        continue;
                    };
                    best_t = t;
                    best_geo = Some(geo);
                    best_pos = ray_origin + ray_dir * t;
                    best_normal = normal;
                }
            }
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let chunks: Vec<&Chunk> = self.chunks_map.values().collect();
                if let Some((t, geo, pos, normal)) = chunks
                    .par_iter()
                    .filter_map(|chunk| {
                        let mut best_t = f32::INFINITY;
                        let mut best_geo: Option<GeoId> = None;
                        let mut best_pos = Vec3::new(0.0, 0.0, 0.0);
                        let mut best_normal = Vec3::unit_y();

                        for poly_list in chunk.polys3d_map.values() {
                            for poly in poly_list {
                                if poly.indices.is_empty() || poly.vertices.is_empty() {
                                    continue;
                                }

                                if !poly.visible && !include_hidden {
                                    continue;
                                }

                                let mut poly_pos: Vec<[f32; 3]> =
                                    Vec::with_capacity(poly.vertices.len());
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
                                    let Some(normal) = triangle_normal(a, b, c) else {
                                        continue;
                                    };
                                    if let Some((t, _, _)) =
                                        ray_triangle_intersect(ray_origin, ray_dir, a, b, c)
                                        && t > 1e-5
                                        && t < best_t
                                    {
                                        best_t = t;
                                        best_geo = Some(poly.id);
                                        best_pos = ray_origin + ray_dir * t;
                                        best_normal = normal;
                                    }
                                }
                            }
                        }

                        best_geo.map(|geo| (best_t, geo, best_pos, best_normal))
                    })
                    .reduce_with(|a, b| if a.0 <= b.0 { a } else { b })
                {
                    best_t = t;
                    best_geo = Some(geo);
                    best_pos = pos;
                    best_normal = normal;
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                for chunk in self.chunks_map.values() {
                    for poly_list in chunk.polys3d_map.values() {
                        for poly in poly_list {
                            if poly.indices.is_empty() || poly.vertices.is_empty() {
                                continue;
                            }

                            if !poly.visible && !include_hidden {
                                continue;
                            }

                            let mut poly_pos: Vec<[f32; 3]> =
                                Vec::with_capacity(poly.vertices.len());
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
                                let Some(normal) = triangle_normal(a, b, c) else {
                                    continue;
                                };
                                if let Some((t, _, _)) =
                                    ray_triangle_intersect(ray_origin, ray_dir, a, b, c)
                                    && t > 1e-5
                                    && t < best_t
                                {
                                    best_t = t;
                                    best_geo = Some(poly.id);
                                    best_pos = ray_origin + ray_dir * t;
                                    best_normal = normal;
                                }
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
                if obj.kind == DynamicKind::Mesh {
                    let mut mesh_pos: Vec<[f32; 3]> = Vec::with_capacity(obj.mesh_vertices.len());
                    for v in &obj.mesh_vertices {
                        let p = m * Vec4::new(v.position.x, v.position.y, v.position.z, 1.0);
                        let w = if p.w != 0.0 { p.w } else { 1.0 };
                        mesh_pos.push([p.x / w, p.y / w, p.z / w]);
                    }

                    for tri in obj.mesh_indices.chunks_exact(3) {
                        let a = mesh_pos.get(tri[0] as usize).copied();
                        let b = mesh_pos.get(tri[1] as usize).copied();
                        let c = mesh_pos.get(tri[2] as usize).copied();
                        let (a, b, c) = match (a, b, c) {
                            (Some(a), Some(b), Some(c)) => (a, b, c),
                            _ => continue,
                        };
                        let Some(normal) = triangle_normal(a, b, c) else {
                            continue;
                        };
                        if let Some((t, _, _)) =
                            ray_triangle_intersect(ray_origin, ray_dir, a, b, c)
                        {
                            if t > 1e-5 && t < best_t {
                                best_t = t;
                                best_geo = Some(obj.id);
                                best_pos = ray_origin + ray_dir * t;
                                best_normal = normal;
                            }
                        }
                    }
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
                    // Reject transparent texels so billboard holes don't capture hover/click.
                    let mut tex_u = (u * 0.5 + 0.5).clamp(0.0, 0.9999);
                    let mut tex_v = (1.0 - (v * 0.5 + 0.5)).clamp(0.0, 0.9999);
                    if matches!(obj.repeat_mode, crate::dynamic::RepeatMode::Repeat) {
                        tex_u = (tex_u * obj.width).fract();
                        tex_v = (tex_v * obj.height).fract();
                        if tex_u < 0.0 {
                            tex_u += 1.0;
                        }
                        if tex_v < 0.0 {
                            tex_v += 1.0;
                        }
                    }

                    let mut alpha_ok = true;
                    match obj.kind {
                        DynamicKind::BillboardAvatar => {
                            if let Some(avatar) = self.dynamic_avatar_data.get(&obj.id) {
                                if avatar.size > 0 {
                                    let size = avatar.size as usize;
                                    let x = (tex_u * avatar.size as f32).floor() as usize;
                                    let y = (tex_v * avatar.size as f32).floor() as usize;
                                    let x = x.min(size.saturating_sub(1));
                                    let y = y.min(size.saturating_sub(1));
                                    let idx = (y * size + x) * 4 + 3;
                                    alpha_ok = avatar.rgba.get(idx).copied().unwrap_or(0) > 0;
                                }
                            }
                        }
                        DynamicKind::BillboardTile | DynamicKind::ParticleBillboard => {
                            if let Some(tile_id) = obj.tile_id {
                                let mut alpha = self
                                    .shared_atlas
                                    .sample_tile_alpha(
                                        &tile_id,
                                        self.animation_counter as u32,
                                        [tex_u, tex_v],
                                    )
                                    .unwrap_or(255);
                                if matches!(obj.alpha_mode, crate::dynamic::AlphaMode::ChromaKey)
                                    && self
                                        .shared_atlas
                                        .tile_pixel_matches_topleft_rgb(
                                            &tile_id,
                                            self.animation_counter as u32,
                                            [tex_u, tex_v],
                                        )
                                        .unwrap_or(false)
                                {
                                    alpha = 0;
                                }
                                alpha_ok = alpha > 0;
                            }
                        }
                        DynamicKind::Mesh => {}
                    }

                    if !alpha_ok {
                        continue;
                    }

                    best_t = t;
                    best_geo = Some(obj.id);
                    best_pos = hit;
                    best_normal = (normal / normal_len).normalized();
                }
            }
        }

        best_geo.map(|id| (id, best_pos, best_t, best_normal))
    }

    pub fn pick_geo_id_at_uv(
        &self,
        fb_w: u32,
        fb_h: u32,
        screen_uv: [f32; 2],
        include_hidden: bool,
        include_billboards: bool,
    ) -> Option<(GeoId, Vec3<f32>, f32)> {
        self.pick_geo_id_normal_at_uv(fb_w, fb_h, screen_uv, include_hidden, include_billboards)
            .map(|(geo_id, pos, distance, _normal)| (geo_id, pos, distance))
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
            RenderMode::Raster2D => {
                self.raster_draw_2d_into(device, queue, surface, fb_w, fb_h)?;

                if self.activity_logging {
                    self.log_layer("2D raster draw completed".to_string());
                }
            }
            RenderMode::Compute3D => {
                self.compute_draw_3d_into(device, queue, surface, fb_w, fb_h)?;

                if self.activity_logging {
                    self.log_layer("3D compute draw completed".to_string());
                }
            }
            RenderMode::Raster3D => {
                self.raster_draw_3d_into(device, queue, surface, fb_w, fb_h)?;

                if self.activity_logging {
                    self.log_layer("3D raster draw completed".to_string());
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

fn paint_project_point(
    camera: &Camera3D,
    fb_w: u32,
    fb_h: u32,
    point: [f32; 3],
) -> Option<(Vec2<f32>, f32)> {
    let p = Vec3::new(point[0], point[1], point[2]);
    let rel = p - camera.pos;
    let depth = rel.dot(camera.forward);
    if !depth.is_finite() || depth <= camera.near || depth >= camera.far {
        return None;
    }

    let fb_w_f = fb_w.max(1) as f32;
    let fb_h_f = fb_h.max(1) as f32;
    let aspect = fb_w_f / fb_h_f;
    let x_cam = rel.dot(camera.right);
    let y_cam = rel.dot(camera.up);

    let (ndc_x, ndc_y) = match camera.kind {
        CameraKind::OrthoIso => {
            let half_h = camera.ortho_half_h.max(1e-6);
            let half_w = half_h * aspect;
            (x_cam / half_w, y_cam / half_h)
        }
        CameraKind::OrbitPersp | CameraKind::FirstPersonPersp => {
            let tan_half = (camera.vfov_deg.to_radians() * 0.5).tan().max(1e-6);
            (
                x_cam / (depth * tan_half * aspect),
                y_cam / (depth * tan_half),
            )
        }
    };

    if !ndc_x.is_finite() || !ndc_y.is_finite() {
        return None;
    }

    Some((
        Vec2::new((ndc_x * 0.5 + 0.5) * fb_w_f, (0.5 - ndc_y * 0.5) * fb_h_f),
        depth,
    ))
}

fn paint_edge(a: Vec2<f32>, b: Vec2<f32>, c: Vec2<f32>) -> f32 {
    (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
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

fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Option<Vec3<f32>> {
    let a = Vec3::new(a[0], a[1], a[2]);
    let b = Vec3::new(b[0], b[1], b[2]);
    let c = Vec3::new(c[0], c[1], c[2]);
    let normal = (b - a).cross(c - a);
    if normal.magnitude() <= 1e-6 || !normal.magnitude().is_finite() {
        return None;
    }
    Some(normal.normalized())
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
