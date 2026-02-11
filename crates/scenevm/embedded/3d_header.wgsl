// 3D Header with utility functions. Cannot be replaced from the API

struct U3D {
  background: vec4<f32>,
  fb_size: vec2<u32>, _pad0: vec2<u32>,
  gp0: vec4<f32>, gp1: vec4<f32>, gp2: vec4<f32>,
  gp3: vec4<f32>, gp4: vec4<f32>, gp5: vec4<f32>,
  gp6: vec4<f32>, gp7: vec4<f32>, gp8: vec4<f32>, gp9: vec4<f32>,
  mat3d_c0: vec4<f32>,
  mat3d_c1: vec4<f32>,
  mat3d_c2: vec4<f32>,
  mat3d_c3: vec4<f32>,
  lights_count: u32,
  vm_flags: u32,
  anim_counter: u32,
  _pad_lights: u32,

  // --- Camera (matches Compute3DUniforms on the CPU) ---
  cam_pos:   vec4<f32>,  // xyz, pad
  cam_fwd:   vec4<f32>,  // xyz, pad
  cam_right: vec4<f32>,  // xyz, pad
  cam_up:    vec4<f32>,  // xyz, pad
  cam_vfov_deg:     f32, // perspective vertical FOV in degrees
  cam_ortho_half_h: f32, // ortho half-height
  cam_near:         f32,
  cam_far:          f32,
  cam_kind: u32,         // 0=OrthoIso, 1=OrbitPersp, 2=FirstPersonPersp
  _pad_cam: vec3<u32>,
  palette: array<vec4<f32>, 256>,
};
@group(0) @binding(0) var<uniform> U: U3D;
@group(0) @binding(1) var color_out: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var atlas_tex: texture_2d<f32>;
@group(0) @binding(3) var atlas_smp: sampler;
@group(0) @binding(14) var prev_layer: texture_2d<f32>;

struct LightWGSL {
  header:   vec4<u32>,  // [light_type, emitting, _, _]
  position: vec4<f32>,  // xyz, _
  color:    vec4<f32>,  // rgb, _
  params0:  vec4<f32>,  // [intensity, radius, startD, endD]
  params1:  vec4<f32>,  // [flicker, _, _, _]
};
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
@group(0) @binding(4) var<storage, read> scene_data: SceneData;

const SCENE_LIGHT_WORDS: u32 = 20u;
const SCENE_BILLBOARD_CMD_WORDS: u32 = 16u;
const DYNAMIC_KIND_BILLBOARD_TILE: u32 = 0u;
const DYNAMIC_KIND_BILLBOARD_AVATAR: u32 = 1u;

fn sd_data_word(idx: u32) -> u32 {
  if (idx >= scene_data.header.data_word_count) {
    return 0u;
  }
  return scene_data.data[idx];
}

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
  let base = scene_data.header.lights_offset_words + li * SCENE_LIGHT_WORDS;
  var light: LightWGSL;
  light.header = sd_vec4u(base + 0u);
  light.position = sd_vec4f(base + 4u);
  light.color = sd_vec4f(base + 8u);
  light.params0 = sd_vec4f(base + 12u);
  light.params1 = sd_vec4f(base + 16u);
  return light;
}

struct DynBillboardCmd {
  center: vec4<f32>,       // xyz + width
  axis_right: vec4<f32>,   // xyz + height
  axis_up: vec4<f32>,      // xyz + repeat_mode
  params: vec4<u32>,       // tile_index, kind, opacity_bits, unused
};

struct DynBillboardHit {
  hit: bool,           // offset 0, size 4 (stored as u32 in SPIR-V)
  t: f32,              // offset 4, size 4
  uv: vec2<f32>,       // offset 8, size 8 (needs 8-byte alignment)
  tile_index: u32,     // offset 16, size 4
  _pad0: u32,          // offset 20, size 4 (AMD alignment fix)
  _pad1: u32,          // offset 24, size 4
  _pad2: u32,          // offset 28, size 4 (pad to 32 bytes for 16-byte boundary alignment)
};

fn sd_billboard_cmd(idx: u32) -> DynBillboardCmd {
  var cmd = DynBillboardCmd(
    vec4<f32>(0.0, 0.0, 0.0, 0.0),
    vec4<f32>(0.0, 0.0, 0.0, 0.0),
    vec4<f32>(0.0, 0.0, 0.0, 0.0),
    vec4<u32>(0u, 0u, 0u, 0u)
  );
  if (scene_data.header.billboard_cmd_count == 0u) {
    return cmd;
  }
  let base = scene_data.header.billboard_cmd_offset_words + idx * SCENE_BILLBOARD_CMD_WORDS;
  if (base + SCENE_BILLBOARD_CMD_WORDS > scene_data.header.data_word_count) {
    return cmd;
  }
  cmd.center = sd_vec4f(base + 0u);
  cmd.axis_right = sd_vec4f(base + 4u);
  cmd.axis_up = sd_vec4f(base + 8u);
  cmd.params = sd_vec4u(base + 12u);
  return cmd;
}

fn sd_ray_billboard(ro: vec3<f32>, rd: vec3<f32>, cmd: DynBillboardCmd) -> DynBillboardHit {
  var hit = DynBillboardHit(false, 0.0, vec2<f32>(0.0, 0.0), 0u, 0u, 0u, 0u);
  if (cmd.params.y != DYNAMIC_KIND_BILLBOARD_TILE && cmd.params.y != DYNAMIC_KIND_BILLBOARD_AVATAR) {
    return hit;
  }
  let axis_u = cmd.axis_right.xyz;
  let axis_v = cmd.axis_up.xyz;
  if (all(axis_u == vec3<f32>(0.0)) || all(axis_v == vec3<f32>(0.0))) {
    return hit;
  }
  let normal = normalize(cross(axis_u, axis_v));
  let denom = dot(normal, rd);
  if (abs(denom) < 1e-5) {
    return hit;
  }
  let center = cmd.center.xyz;
  let t = dot(center - ro, normal) / denom;
  if (t <= 0.0) {
    return hit;
  }
  let pos = ro + rd * t;
  let rel = pos - center;
  let len_u2 = max(dot(axis_u, axis_u), 1e-6);
  let len_v2 = max(dot(axis_v, axis_v), 1e-6);
  let u = dot(rel, axis_u) / len_u2;
  let v = dot(rel, axis_v) / len_v2;
  if (abs(u) > 1.0 || abs(v) > 1.0) {
    return hit;
  }
  hit.hit = true;
  hit.t = t;
  hit.uv = vec2<f32>(0.5 * (u + 1.0), 0.5 * (1.0 - v));
  hit.tile_index = cmd.params.x;
  return hit;
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

// Vert3D layout (std430): Each member aligned to its natural alignment
// - uv       : OBJECT UVs (not atlas-mapped). These can be any scale; we wrap in shader.
struct Vert3D {
  pos: vec3<f32>, _pad0: f32,               // offset 0, size 16
  uv: vec2<f32>, _pad_uv: vec2<f32>,        // offset 16, size 16
  tile_index: u32,                          // offset 32, size 4 (primary texture)
  tile_index2: u32,                         // offset 36, size 4 (secondary texture for blending)
  blend_factor: f32,                        // offset 40, size 4 (0.0=primary, 1.0=secondary)
  _pad_blend: f32,                          // offset 44, size 4
  normal: vec3<f32>, _pad2: f32             // offset 48, size 16
};
struct Verts3D { data: array<Vert3D> };
struct Indices { data: array<u32> };

@group(0) @binding(5) var<storage, read> verts3d: Verts3D;
@group(0) @binding(6) var<storage, read> indices3d: Indices;

// --- Scene-wide BVH accel (reusing existing grid bindings; toggle via gp9.w) ---
struct Grid3DHeader {
  origin: vec4<f32>,     // xyz, pad
  cell_size: vec4<f32>,  // xyz, pad
  dims: vec4<u32>,       // nx, ny, nz, pad
  ranges: vec4<u32>,     // nodes_start, tris_start, node_count, tri_count
  visibility: vec4<u32>, // vis_start, vis_word_count, pad, pad
};
@group(0) @binding(7) var<uniform> gridH: Grid3DHeader;
struct GridDataBuffer {
  data: array<u32>,
};
@group(0) @binding(8)  var<storage, read> grid_data: GridDataBuffer;
@group(0) @binding(11) var atlas_mat_tex: texture_2d<f32>;
struct TileAnimMeta {
  first_frame: u32,
  frame_count: u32,
  _pad: vec2<u32>,
};
struct TileAnims {
  data: array<TileAnimMeta>,
};
struct TileFrame {
  ofs: vec2<f32>,
  scale: vec2<f32>,
};
struct TileFrames {
  data: array<TileFrame>,
};
@group(0) @binding(12) var<storage, read> tile_anims: TileAnims;
@group(0) @binding(13) var<storage, read> tile_frames: TileFrames;

fn sv_grid_active() -> bool { return U.gp9.w > 0.5; }

fn sv_write(px: u32, py: u32, c: vec4<f32>) {
  textureStore(color_out, vec2<i32>(i32(px), i32(py)), c);
}
fn sv_sample(uv: vec2<f32>) -> vec4<f32> {
  return textureSampleLevel(atlas_tex, atlas_smp, uv, 0.0);
}
// ---- 3D utilities ----
// Full hit record including **geometric** normal (for debug/fallback). Shaders may
// still compute/interpolate their own shading normal using vertex data.
// Note: vec3 requires 16-byte alignment in structs for Vulkan SPIR-V
struct Hit3DFull {
  hit: bool,
  t: f32,
  u: f32,
  v: f32,           // offset 12, next vec3 needs offset 16
  Ng: vec3<f32>,    // offset 16 (naturally aligned)
  _pad0: f32        // offset 28, pad to 32 bytes (16-byte aligned)
};

fn sv_ray_tri_full(ro: vec3<f32>, rd: vec3<f32>, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>) -> Hit3DFull {
  let e1 = b - a;
  let e2 = c - a;
  let p = cross(rd, e2);
  let det = dot(e1, p);
  if (abs(det) < 1e-8) { return Hit3DFull(false, 0.0, 0.0, 0.0, vec3<f32>(0.0), 0.0); }
  let inv_det = 1.0 / det;
  let tv = ro - a;
  let u = dot(tv, p) * inv_det;
  if (u < 0.0 || u > 1.0) { return Hit3DFull(false, 0.0, 0.0, 0.0, vec3<f32>(0.0), 0.0); }
  let q = cross(tv, e1);
  let v = dot(rd, q) * inv_det;
  if (v < 0.0 || u + v > 1.0) { return Hit3DFull(false, 0.0, 0.0, 0.0, vec3<f32>(0.0), 0.0); }
  let t = dot(e2, q) * inv_det;
  if (t <= 0.0) { return Hit3DFull(false, 0.0, 0.0, 0.0, vec3<f32>(0.0), 0.0); }
  // Geometric normal; flip to face the ray if needed for stability
  var Ng = normalize(cross(e1, e2));
  if (det > 0.0) { Ng = -Ng; }
  return Hit3DFull(true, t, u, v, Ng, 0.0);
}

// --- Helpers: wrap UVs and clamp indices to valid SSBO ranges ---
fn clamp_index_u(i: u32, len: u32) -> u32 {
    return select(0u, min(i, max(len, 1u) - 1u), len > 0u);
}

// ---- UV wrapping helpers (GPU-side repeat inside atlas rect) ----
// OBJECT-UV bary mapping into atlas
fn sv_tile_frame(tile_index: u32) -> TileFrame {
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
  let frame_offset = anim.first_frame + (U.anim_counter % count);
  let frame_idx = min(frame_offset, frames_len - 1u);
  return tile_frames.data[frame_idx];
}

fn sv_tri_atlas_uv_obj(i0: u32, i1: u32, i2: u32, bu: f32, bv: f32) -> vec2<f32> {
  // Barycentric blend of per-vertex OBJECT uv
  let uv0 = verts3d.data[i0].uv;
  let uv1 = verts3d.data[i1].uv;
  let uv2 = verts3d.data[i2].uv;
  let w = 1.0 - bu - bv;
  let uv_obj = uv0 * w + uv1 * bu + uv2 * bv;

  let frame = sv_tile_frame(verts3d.data[i0].tile_index);

  // Repeat OBJECT uv, then scale into sub-rect and add offset
  var uv_wrapped = fract(uv_obj);          // [0,1) repeat in object space
  uv_wrapped.y = fract(1.0 - uv_wrapped.y); // flip Y so tiles aren't upside down
  let uv_atlas   = frame.ofs + uv_wrapped * frame.scale; // map into atlas sub-rect

  // Clamp into the interior of the frame to avoid bleeding from neighboring tiles.
  // Use a half-texel pad in atlas UV space so sampling stays within the tile.
  let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0));
  let pad_uv = vec2<f32>(0.5) / atlas_dims;
  let uv_min = frame.ofs + pad_uv;
  let uv_max = frame.ofs + frame.scale - pad_uv;
  return clamp(uv_atlas, uv_min, uv_max);
}

// Sample atlas texture using barycentrics on a triangle with GPU-side repeat (object-UV based)
fn sv_tri_sample_albedo(i0: u32, i1: u32, i2: u32, bu: f32, bv: f32) -> vec4<f32> {
  let uv = sv_tri_atlas_uv_obj(i0, i1, i2, bu, bv);
  return sv_sample(uv);
}

fn sv_tri_sample_rmoe(i0: u32, i1: u32, i2: u32, bu: f32, bv: f32) -> vec4<f32> {
  let uv = sv_tri_atlas_uv_obj(i0, i1, i2, bu, bv);
  return textureSampleLevel(atlas_mat_tex, atlas_smp, uv, 0.0);
}

// ===== Vertex Blending Functions =====

// Sample albedo for a specific tile (not triangle-based)
fn sv_sample_tile_albedo(tile_index: u32, uv: vec2<f32>) -> vec4<f32> {
  let frame = sv_tile_frame(tile_index);
  var uv_wrapped = fract(uv);
  uv_wrapped.y = fract(1.0 - uv_wrapped.y);
  let uv_atlas = frame.ofs + uv_wrapped * frame.scale;

  let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0));
  let pad_uv = vec2<f32>(0.5) / atlas_dims;
  let uv_min = frame.ofs + pad_uv;
  let uv_max = frame.ofs + frame.scale - pad_uv;
  let uv_clamped = clamp(uv_atlas, uv_min, uv_max);

  return textureSampleLevel(atlas_tex, atlas_smp, uv_clamped, 0.0);
}

// Sample material (RMOE) for a specific tile
fn sv_sample_tile_rmoe(tile_index: u32, uv: vec2<f32>) -> vec4<f32> {
  let frame = sv_tile_frame(tile_index);
  var uv_wrapped = fract(uv);
  uv_wrapped.y = fract(1.0 - uv_wrapped.y);
  let uv_atlas = frame.ofs + uv_wrapped * frame.scale;

  let atlas_dims = vec2<f32>(textureDimensions(atlas_mat_tex, 0));
  let pad_uv = vec2<f32>(0.5) / atlas_dims;
  let uv_min = frame.ofs + pad_uv;
  let uv_max = frame.ofs + frame.scale - pad_uv;
  let uv_clamped = clamp(uv_atlas, uv_min, uv_max);

  return textureSampleLevel(atlas_mat_tex, atlas_smp, uv_clamped, 0.0);
}

// Blend albedo between two textures based on vertex blend factors
fn sv_tri_sample_albedo_blended(i0: u32, i1: u32, i2: u32, bu: f32, bv: f32) -> vec4<f32> {
  // Get interpolated blend factor from vertices
  let blend0 = verts3d.data[i0].blend_factor;
  let blend1 = verts3d.data[i1].blend_factor;
  let blend2 = verts3d.data[i2].blend_factor;
  let w = 1.0 - bu - bv;
  let blend = blend0 * w + blend1 * bu + blend2 * bv;

  // Get tile indices (assume all vertices share same tiles for now)
  let tile1 = verts3d.data[i0].tile_index;
  let tile2 = verts3d.data[i0].tile_index2;

  // Get interpolated UV
  let uv0 = verts3d.data[i0].uv;
  let uv1 = verts3d.data[i1].uv;
  let uv2 = verts3d.data[i2].uv;
  let uv = uv0 * w + uv1 * bu + uv2 * bv;

  // Sample both textures
  let albedo1 = sv_sample_tile_albedo(tile1, uv);
  let albedo2 = sv_sample_tile_albedo(tile2, uv);

  // Blend between them
  return mix(albedo1, albedo2, blend);
}

// Blend material (RMOE) between two textures based on vertex blend factors
fn sv_tri_sample_rmoe_blended(i0: u32, i1: u32, i2: u32, bu: f32, bv: f32) -> vec4<f32> {
  // Get interpolated blend factor from vertices
  let blend0 = verts3d.data[i0].blend_factor;
  let blend1 = verts3d.data[i1].blend_factor;
  let blend2 = verts3d.data[i2].blend_factor;
  let w = 1.0 - bu - bv;
  let blend = blend0 * w + blend1 * bu + blend2 * bv;

  // Get tile indices
  let tile1 = verts3d.data[i0].tile_index;
  let tile2 = verts3d.data[i0].tile_index2;

  // Get interpolated UV
  let uv0 = verts3d.data[i0].uv;
  let uv1 = verts3d.data[i1].uv;
  let uv2 = verts3d.data[i2].uv;
  let uv = uv0 * w + uv1 * bu + uv2 * bv;

  // Sample both material textures
  let rmoe1 = sv_sample_tile_rmoe(tile1, uv);
  let rmoe2 = sv_sample_tile_rmoe(tile2, uv);

  // Blend between them
  return mix(rmoe1, rmoe2, blend);
}

fn sv_interp3(a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, u: f32, v: f32) -> vec3<f32> {
  return a*(1.0-u-v) + b*u + c*v;
}

// ===== BVH traversal over triangles (bindings unchanged) =====

// Packed hit record returned by DDA tracing.
// Note: vec3 requires 16-byte alignment in structs for Vulkan SPIR-V
struct TraceHit {
  hit: bool,        // offset 0
  t: f32,           // offset 4  (distance along ray)
  tri: u32,         // offset 8  (winning triangle index in indices3d, tri = tix)
  u: f32,           // offset 12 (barycentric u)
  v: f32,           // offset 16 (barycentric v)
  _pad0: u32,       // offset 20 (padding)
  _pad1: u32,       // offset 24 (padding)
  _pad2: u32,       // offset 28 (padding to align Ng to 16-byte boundary)
  Ng: vec3<f32>,    // offset 32 (geometric normal, 16-byte aligned)
  _pad3: f32,       // offset 44, pad to 48 bytes (16-byte aligned)
};

// Grid helpers (preserve API surface; dims defaults to 1^3 for BVH backing)
fn grid_bounds_min() -> vec3<f32> { return gridH.origin.xyz; }
fn grid_cell_size() -> vec3<f32> { return gridH.cell_size.xyz; }
fn grid_dims() -> vec3<u32> { return max(gridH.dims.xyz, vec3<u32>(1u)); }
fn grid_bounds_max() -> vec3<f32> {
  return grid_bounds_min() + grid_cell_size() * vec3<f32>(grid_dims());
}

// Legacy grid helpers kept for API compatibility (no-op with BVH backing)
fn grid_cell_index(ix: u32, iy: u32, iz: u32) -> u32 {
  let nx = max(gridH.dims.x, 1u); let ny = max(gridH.dims.y, 1u);
  return (iz * ny + iy) * nx + ix;
}
fn grid_world_to_cell(p: vec3<f32>) -> vec3<i32> {
  let minb = grid_bounds_min();
  let cs = grid_cell_size();
  let rel = (p - minb) / cs;
  return vec3<i32>(floor(rel));
}
fn clamp_cell(c: vec3<i32>) -> vec3<i32> {
  let d = grid_dims();
  return vec3<i32>(
    clamp(c.x, 0, i32(d.x) - 1),
    clamp(c.y, 0, i32(d.y) - 1),
    clamp(c.z, 0, i32(d.z) - 1)
  );
}
fn grid_offset(idx: u32) -> u32 { return grid_data.data[gridH.ranges.x + idx]; }
fn grid_count(_idx: u32) -> u32 { return 0u; }
fn grid_tri_index(idx: u32) -> u32 { return bvh_tri_id(idx); }

// BVH buffer layout helpers (node = 2x vec3 bounds + left_first + tri_count)
struct BvhNode {
  bmin: vec3<f32>,
  bmax: vec3<f32>,
  left_first: u32,
  tri_count: u32,
};

fn bvh_nodes_start() -> u32 { return gridH.ranges.x; }
fn bvh_tris_start() -> u32 { return gridH.ranges.y; }
fn bvh_node_count() -> u32 { return gridH.ranges.z; }
fn bvh_tri_count() -> u32 { return gridH.ranges.w; }

fn bvh_node_base(idx: u32) -> u32 { return bvh_nodes_start() + idx * 8u; }

fn bvh_load_node(idx: u32) -> BvhNode {
  let base = bvh_node_base(idx);
  let bmin = vec3<f32>(
    bitcast<f32>(grid_data.data[base + 0u]),
    bitcast<f32>(grid_data.data[base + 1u]),
    bitcast<f32>(grid_data.data[base + 2u])
  );
  let bmax = vec3<f32>(
    bitcast<f32>(grid_data.data[base + 3u]),
    bitcast<f32>(grid_data.data[base + 4u]),
    bitcast<f32>(grid_data.data[base + 5u])
  );
  let left_first = grid_data.data[base + 6u];
  let tri_count = grid_data.data[base + 7u];
  return BvhNode(bmin, bmax, left_first, tri_count);
}

fn bvh_tri_id(offset: u32) -> u32 {
  return grid_data.data[bvh_tris_start() + offset];
}

// Check if a triangle is visible using the packed bitmask
fn is_tri_visible(tri_idx: u32) -> bool {
  let vis_start = gridH.visibility.x;
  let vis_word_count = gridH.visibility.y;

  // No visibility data means all triangles are visible
  if (vis_word_count == 0u) {
    return true;
  }

  // Calculate which word and bit this triangle's visibility is stored in
  let word_idx = tri_idx / 32u;
  let bit_idx = tri_idx % 32u;

  // Out of bounds check
  if (word_idx >= vis_word_count) {
    return false;
  }

  // Read the visibility word and check the bit
  let vis_word = grid_data.data[vis_start + word_idx];
  return (vis_word & (1u << bit_idx)) != 0u;
}

// Legacy DDA API surface (kept for compatibility; no longer used internally)
struct DDAState {
  tMax:   vec3<f32>,
  tDelta: vec3<f32>,
  step:   vec3<i32>,
};
fn dda_setup(_p: vec3<f32>, _rd: vec3<f32>, _cell: vec3<i32>, _tEnter: f32) -> DDAState {
  return DDAState(vec3<f32>(0.0), vec3<f32>(0.0), vec3<i32>(0, 0, 0));
}

// Ray/AABB for the whole grid, returns (hit, tEnter, tExit)
fn ray_box(ro: vec3<f32>, rd: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> vec3<f32> {
  let eps = 1e-6;

  // For each axis: if |rd| >= eps use rd, else use sign(rd)*eps (preserve sign)
  let rx = select(sign(rd.x) * eps, rd.x, abs(rd.x) >= eps);
  let ry = select(sign(rd.y) * eps, rd.y, abs(rd.y) >= eps);
  let rz = select(sign(rd.z) * eps, rd.z, abs(rd.z) >= eps);

  let inv = vec3<f32>(1.0 / rx, 1.0 / ry, 1.0 / rz);

  let t0 = (bmin - ro) * inv;
  let t1 = (bmax - ro) * inv;

  let tmin = max(max(min(t0.x, t1.x), min(t0.y, t1.y)), min(t0.z, t1.z));
  let tmax = min(min(max(t0.x, t1.x), max(t0.y, t1.y)), max(t0.z, t1.z));

  let hit = select(0.0, 1.0, tmax >= max(tmin, 0.0));
  return vec3<f32>(hit, tmin, tmax);
}

// BVH traversal. tmin/tmax clip the segment (e.g., near/far planes).
fn sv_trace_grid(ro: vec3<f32>, rd: vec3<f32>, tmin: f32, tmax: f32) -> TraceHit {
  let node_count = bvh_node_count();
  if (node_count == 0u) { return TraceHit(false, 0.0, 0u, 0.0, 0.0, 0u, 0u, 0u, vec3<f32>(0.0), 0.0); }

  // Early reject with the root bounds stored in the header
  let rb_root = ray_box(ro, rd, grid_bounds_min(), grid_bounds_max());
  if (rb_root.x < 0.5) { return TraceHit(false, 0.0, 0u, 0.0, 0.0, 0u, 0u, 0u, vec3<f32>(0.0), 0.0); }
  let seg_min = max(rb_root.y, tmin);
  let seg_max = min(rb_root.z, tmax);
  if (seg_min > seg_max) { return TraceHit(false, 0.0, 0u, 0.0, 0.0, 0u, 0u, 0u, vec3<f32>(0.0), 0.0); }

  var best_t = seg_max;
  var best_tri: u32 = 0u;
  var best_u = 0.0;
  var best_v = 0.0;
  var best_Ng = vec3<f32>(0.0);

  // Small explicit stack
  var stack: array<u32, 64u>;
  var stack_size: u32 = 1u;
  stack[0u] = 0u;

  loop {
    if (stack_size == 0u) { break; }
    stack_size = stack_size - 1u;
    let node_idx = stack[stack_size];
    if (node_idx >= node_count) { continue; }

    let node = bvh_load_node(node_idx);
    let rb = ray_box(ro, rd, node.bmin, node.bmax);
    if (rb.x < 0.5) { continue; }
    let entry = max(rb.y, seg_min);
    let exit = min(rb.z, best_t);
    if (entry > exit) { continue; }

    if (node.tri_count > 0u) {
      let base = node.left_first;
      for (var i: u32 = 0u; i < node.tri_count; i = i + 1u) {
        let tri = bvh_tri_id(base + i);
        if (tri >= bvh_tri_count()) { continue; }
        if (tri * 3u + 2u >= arrayLength(&indices3d.data)) { continue; }

        // Skip invisible triangles
        if (!is_tri_visible(tri)) { continue; }

        let i0 = indices3d.data[3u*tri + 0u];
        let i1 = indices3d.data[3u*tri + 1u];
        let i2 = indices3d.data[3u*tri + 2u];

        // Triangle vertices
        let a = verts3d.data[i0].pos;
        let b = verts3d.data[i1].pos;
        let c = verts3d.data[i2].pos;

        let hit = sv_ray_tri_full(ro, rd, a, b, c);
        if (!hit.hit) { continue; }

        if (hit.t > seg_min && hit.t < best_t) {
          best_t  = hit.t;
          best_tri = tri;
          best_u  = hit.u;
          best_v  = hit.v;
          best_Ng = hit.Ng;
        }
      }
      continue;
    }

    // Internal node: visit near child first
    let left_idx = node.left_first;
    let right_idx = left_idx + 1u;
    let left = bvh_load_node(left_idx);
    let right = bvh_load_node(right_idx);
    let rb_left = ray_box(ro, rd, left.bmin, left.bmax);
    let rb_right = ray_box(ro, rd, right.bmin, right.bmax);

    let left_hit = rb_left.x > 0.5 && max(rb_left.y, seg_min) <= min(rb_left.z, best_t);
    let right_hit = rb_right.x > 0.5 && max(rb_right.y, seg_min) <= min(rb_right.z, best_t);

    if (left_hit && right_hit) {
      let l_entry = max(rb_left.y, seg_min);
      let r_entry = max(rb_right.y, seg_min);
      if (stack_size + 2u < 64u) {
        if (l_entry < r_entry) {
          stack[stack_size] = right_idx;
          stack[stack_size + 1u] = left_idx;
        } else {
          stack[stack_size] = left_idx;
          stack[stack_size + 1u] = right_idx;
        }
        stack_size = stack_size + 2u;
      }
    } else if (left_hit && stack_size < 64u) {
      stack[stack_size] = left_idx;
      stack_size = stack_size + 1u;
    } else if (right_hit && stack_size < 64u) {
      stack[stack_size] = right_idx;
      stack_size = stack_size + 1u;
    }
  }

  if (best_t < seg_max && best_t < 1e29) {
    return TraceHit(true, best_t, best_tri, best_u, best_v, 0u, 0u, 0u, best_Ng, 0.0);
  }

  return TraceHit(false, 0.0, 0u, 0.0, 0.0, 0u, 0u, 0u, vec3<f32>(0.0), 0.0);
}
// ===== end BVH =====

// TBN from triangle positions and OBJECT-space UVs (no atlas mapping here),
// using geometric normal for stability.
fn sv_tri_tbn(a: vec3<f32>, b: vec3<f32>, c: vec3<f32>,
              uv0: vec2<f32>, uv1: vec2<f32>, uv2: vec2<f32>) -> mat3x3<f32> {
  let e1 = b - a;
  let e2 = c - a;
  let d1 = uv1 - uv0;
  let d2 = uv2 - uv0;
  let r = 1.0 / max(d1.x * d2.y - d1.y * d2.x, 1e-8);

  var T = normalize((e1 * d2.y - e2 * d1.y) * r);
  var Ng = normalize(cross(e1, e2));
  T = normalize(T - Ng * dot(Ng, T));
  let B = normalize(cross(Ng, T));
  return mat3x3<f32>(T, B, Ng);
}

// Luma from color (height proxy)
fn sv_luma(rgb: vec3<f32>) -> f32 {
  return dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
}

// Camera
struct Ray { ro: vec3<f32>, rd: vec3<f32> };

fn cam_ray(uv: vec2<f32>) -> Ray {
  // uv in [0,1]^2 pixel center; build NDC in [-1,1]
  let res = vec2<f32>(f32(U.fb_size.x), f32(U.fb_size.y));
  let ndc = (uv * 2.0 - vec2<f32>(1.0,1.0)) * vec2<f32>(1.0, -1.0); // y up

  if (U.cam_kind == 0u) {
    // Ortho: origin scans a rectangle on a plane; direction = forward
    let aspect = res.x / max(res.y, 1.0);
    let half_w = U.cam_ortho_half_h * aspect;
    let p = U.cam_pos.xyz
          + U.cam_right.xyz * (ndc.x * half_w)
          + U.cam_up.xyz    * (ndc.y * U.cam_ortho_half_h);
    return Ray(p, normalize(U.cam_fwd.xyz));
  } else {
    // Perspective: pinhole at cam_pos
    // screen basis on near plane using vfov
    let tan_half = tan(radians(U.cam_vfov_deg) * 0.5);
    let aspect = res.x / max(res.y, 1.0);
    let dx = ndc.x * aspect * tan_half;
    let dy = ndc.y * tan_half;
    let dir = normalize(U.cam_fwd.xyz + U.cam_right.xyz * dx + U.cam_up.xyz * dy);
    return Ray(U.cam_pos.xyz, dir);
  }
}
