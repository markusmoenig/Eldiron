// 2D Header with utility functions. Cannot be replaced from the API

struct U2D {
  background: vec4<f32>,
  fb_size: vec2<u32>, _pad0: vec2<u32>,
  gp0: vec4<f32>, gp1: vec4<f32>, gp2: vec4<f32>,
  gp3: vec4<f32>, gp4: vec4<f32>, gp5: vec4<f32>,
  gp6: vec4<f32>, gp7: vec4<f32>, gp8: vec4<f32>, gp9: vec4<f32>,
  mat2d_c0: vec4<f32>,
  mat2d_c1: vec4<f32>,
  mat2d_c2: vec4<f32>,
  mat2d_inv_c0: vec4<f32>,
  mat2d_inv_c1: vec4<f32>,
  mat2d_inv_c2: vec4<f32>,
  lights_count: u32,
  vm_flags: u32,
  anim_counter: u32,
  _pad_lights: u32,
  viewport_rect: vec4<f32>,  // [x, y, width, height] in screen pixels. width=0 means full screen.
  palette: array<vec4<f32>, 256>,
};

@group(0) @binding(0) var<uniform> U: U2D;
@group(0) @binding(1) var color_out: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var atlas_tex: texture_2d<f32>;
@group(0) @binding(3) var atlas_smp: sampler;
@group(0) @binding(13) var prev_layer: texture_2d<f32>;
struct Vert {
  pos: vec2<f32>,
  uv: vec2<f32>,
  tile_index: u32,
  _pad_tile: u32,
};
struct Verts { data: array<Vert> };
struct Indices { data: array<u32> };
@group(0) @binding(4) var<storage, read> verts: Verts;
@group(0) @binding(5) var<storage, read> indices: Indices;
struct U32s { data: array<u32> };
struct TileBin {
  offset: u32,
  count: u32,
};
struct TileBins {
  data: array<TileBin>,
};
@group(0) @binding(6) var<storage, read> tile_bins: TileBins;
@group(0) @binding(7) var<storage, read> tile_tris: U32s;
@group(0) @binding(8) var atlas_mat_tex: texture_2d<f32>;
@group(0) @binding(12) var atlas_smp_linear: sampler;
struct LightWGSL {
  header:   vec4<u32>,
  position: vec4<f32>,
  color:    vec4<f32>,
  params0:  vec4<f32>,
  params1:  vec4<f32>,
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
@group(0) @binding(9) var<storage, read> scene_data: SceneData;

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
  params: vec4<u32>,       // tile_index, kind, unused, unused
};

struct DynBillboardHit2D {
  hit: bool,           // offset 0, size 4
  _pad0: u32,          // offset 4, size 4 (align vec2 to 8 bytes)
  uv: vec2<f32>,       // offset 8, size 8 (needs 8-byte alignment)
  tile_index: u32,     // offset 16, size 4
  repeat_mode: u32,    // offset 20, size 4
  _pad1: u32,          // offset 24, size 4 (AMD alignment fix)
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

fn sv_screen_from_world(world: vec3<f32>) -> vec2<f32> {
  let M = mat3x3<f32>(U.mat2d_c0.xyz, U.mat2d_c1.xyz, U.mat2d_c2.xyz);
  let v = M * vec3<f32>(world.xy, 1.0);
  return v.xy;
}

fn sd_billboard_hit_screen(pix: vec2<f32>, cmd: DynBillboardCmd) -> DynBillboardHit2D {
  var hit = DynBillboardHit2D(false, 0u, vec2<f32>(0.0), 0u, 0u, 0u, 0u);
  if (cmd.params.y != DYNAMIC_KIND_BILLBOARD_TILE && cmd.params.y != DYNAMIC_KIND_BILLBOARD_AVATAR) {
    return hit;
  }
  let axis_u = cmd.axis_right.xyz;
  let axis_v = cmd.axis_up.xyz;
  if (all(axis_u == vec3<f32>(0.0)) || all(axis_v == vec3<f32>(0.0))) {
    return hit;
  }
  let center = cmd.center.xyz;
  let width = cmd.center.w;
  let height = cmd.axis_right.w;
  let repeat_mode = u32(cmd.axis_up.w);

  let center_scr = sv_screen_from_world(center);
  let right_scr = sv_screen_from_world(center + axis_u);
  let up_scr    = sv_screen_from_world(center + axis_v);
  let axis_scr_u = right_scr - center_scr;
  let axis_scr_v = up_scr - center_scr;
  let det = axis_scr_u.x * axis_scr_v.y - axis_scr_u.y * axis_scr_v.x;
  if (abs(det) < 1e-5) {
    return hit;
  }
  let rel = pix - center_scr;
  let inv = 1.0 / det;
  let u = ( rel.x * axis_scr_v.y - rel.y * axis_scr_v.x) * inv;
  let v = (-rel.x * axis_scr_u.y + rel.y * axis_scr_u.x) * inv;
  if (abs(u) > 1.0 || abs(v) > 1.0) {
    return hit;
  }
  hit.hit = true;

  // For repeat mode, scale UVs by the billboard dimensions
  if (repeat_mode == 1u) {
    // Repeat mode: scale UVs by width/height so tiles repeat at their natural size
    hit.uv = vec2<f32>((u + 1.0) * 0.5 * width, (v + 1.0) * 0.5 * height);
  } else {
    // Scale mode: single UV [0,1] mapping
    hit.uv = vec2<f32>(0.5 * (u + 1.0), 0.5 * (v + 1.0));
  }

  hit.tile_index = cmd.params.x;
  hit.repeat_mode = repeat_mode;
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
@group(0) @binding(10) var<storage, read> tile_anims: TileAnims;
@group(0) @binding(11) var<storage, read> tile_frames: TileFrames;

fn tiles_x() -> u32 { return (U.fb_size.x + 7u) / 8u; }
fn tiles_y() -> u32 { return (U.fb_size.y + 7u) / 8u; }
fn tile_index(tx: u32, ty: u32) -> u32 { return ty * tiles_x() + tx; }
fn tile_of_px(px: u32, py: u32) -> u32 {
  let tx = px / 8u;
  let ty = py / 8u;
  return tile_index(tx, ty);
}

fn sv_write(px: u32, py: u32, c: vec4<f32>) {
  textureStore(color_out, vec2<i32>(i32(px), i32(py)), c);
}
fn sv_sample(uv: vec2<f32>) -> vec4<f32> {
  return textureSampleLevel(atlas_tex, atlas_smp, uv, 0.0);
}
// ----- SceneVM 2D helpers -----
// Note: vec3/vec4 require 16-byte alignment in structs for Vulkan SPIR-V
struct BaryHit {
  hit: bool,
  _pad0: u32, _pad1: u32, _pad2: u32,  // pad to 16 bytes before vec3
  w: vec3<f32>
};
struct ColorHit {
  hit: bool,
  _pad0: u32, _pad1: u32, _pad2: u32,  // pad to 16 bytes before vec4
  color: vec4<f32>,
  tri: u32,
  _pad3: u32,  // align vec2 to 8 bytes
  uv: vec2<f32>,
  _pad4: u32, _pad5: u32  // pad to 48 bytes (16-byte aligned)
};

fn sv_edge(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
  return (p.x - a.x)*(b.y - a.y) - (p.y - a.y)*(b.x - a.x);
}

fn sv_tri_bary(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> BaryHit {
  let e0 = sv_edge(p,a,b);
  let e1 = sv_edge(p,b,c);
  let e2 = sv_edge(p,c,a);
  // Allow a small negative tolerance to avoid cracks between adjacent tris sharing an edge.
  let tol = -1e-4;
  let ok = (e0 >= tol && e1 >= tol && e2 >= tol) || (e0 <= -tol && e1 <= -tol && e2 <= -tol);
  if (!ok) { return BaryHit(false, 0u, 0u, 0u, vec3<f32>(0.0)); }
  let area = abs((b.x - a.x)*(c.y - a.y) - (b.y - a.y)*(c.x - a.x));
  if (area <= 0.0) { return BaryHit(false, 0u, 0u, 0u, vec3<f32>(0.0)); }
  let w0 = abs((b.x - p.x)*(c.y - p.y) - (b.y - p.y)*(c.x - p.x)) / area;
  let w1 = abs((c.x - p.x)*(a.y - p.y) - (c.y - p.y)*(a.x - p.x)) / area;
  let w2 = 1.0 - w0 - w1;
  return BaryHit(true, 0u, 0u, 0u, vec3<f32>(w0, w1, w2));
}

fn sv_edge_signed(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
  // Same as sv_edge but signed consistently with winding
  return (p.x - a.x)*(b.y - a.y) - (p.y - a.y)*(b.x - a.x);
}

fn sv_edge_len(a: vec2<f32>, b: vec2<f32>) -> f32 {
  return max(length(b - a), 1e-6);
}

// Distance to the closest triangle edge in *pixels*
fn sv_min_edge_distance_px(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> f32 {
  let e0 = abs(sv_edge_signed(p, a, b)) / sv_edge_len(a, b);
  let e1 = abs(sv_edge_signed(p, b, c)) / sv_edge_len(b, c);
  let e2 = abs(sv_edge_signed(p, c, a)) / sv_edge_len(c, a);
  return min(e0, min(e1, e2));
}

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

fn sv_tri_atlas_uv(i0: u32, i1: u32, i2: u32, w: vec3<f32>) -> vec2<f32> {
  let uv0 = verts.data[i0].uv;
  let uv1 = verts.data[i1].uv;
  let uv2 = verts.data[i2].uv;
  let uv_obj = uv0 * w.x + uv1 * w.y + uv2 * w.z;
  let frame = sv_tile_frame(verts.data[i0].tile_index);
  let uv_wrapped = fract(uv_obj);
  return frame.ofs + uv_wrapped * frame.scale;
}

fn sv_tri_color(p: vec2<f32>, i0: u32, i1: u32, i2: u32) -> ColorHit {
  let a = verts.data[i0].pos;
  let b = verts.data[i1].pos;
  let c = verts.data[i2].pos;
  let bh = sv_tri_bary(p, a, b, c);
  if (!bh.hit) { return ColorHit(false, 0u, 0u, 0u, vec4<f32>(0.0), 0u, 0u, vec2<f32>(0.0), 0u, 0u); }

  let w = bh.w;
  let uv = sv_tri_atlas_uv(i0, i1, i2, w);
  var col = sv_sample(uv);
  if (col.a < 0.01) { return ColorHit(false, 0u, 0u, 0u, vec4<f32>(0.0), 0u, 0u, vec2<f32>(0.0), 0u, 0u); }

  // --- Analytic edge AA ---
  // Disable feathering to avoid visible cracks between adjacent triangles forming a solid.
  // Set to >0.0 if you want AA on standalone tris and accept potential seams on shared edges.
  // let feather = 0.0;
  // if (feather > 0.0) {
  //   let d = sv_min_edge_distance_px(p, a, b, c);  // pixels
  //   let cov = smoothstep(0.0, feather, d);
  //   col.a = col.a * cov;
  // }

  // tri id is not known here; sv_shade_tile_pixel wraps this and sets it
  return ColorHit(true, 0u, 0u, 0u, col, 0u, 0u, uv, 0u, 0u);
}

fn sv_world_from_screen(pix: vec2<f32>) -> vec2<f32> {
  let invM = mat3x3<f32>(U.mat2d_inv_c0.xyz, U.mat2d_inv_c1.xyz, U.mat2d_inv_c2.xyz);
  let v = invM * vec3<f32>(pix, 1.0);
  return v.xy;
}

fn sv_shade_tile_pixel(p: vec2<f32>, px: u32, py: u32, tid: u32) -> ColorHit {
  // AMD fix: Bounds check all SSBO accesses to prevent garbage data on Vulkan
  let bins_len = arrayLength(&tile_bins.data);
  if (bins_len == 0u || tid >= bins_len) {
    return ColorHit(false, 0u, 0u, 0u, vec4<f32>(0.0), 0u, 0u, vec2<f32>(0.0), 0u, 0u);
  }

  let bin = tile_bins.data[tid];
  let off = bin.offset;
  let cnt = bin.count;

  let tris_len = arrayLength(&tile_tris.data);
  let indices_len = arrayLength(&indices.data);

  for (var k: u32 = 0u; k < cnt; k = k + 1u) {
    let tri_idx = off + k;
    if (tri_idx >= tris_len) { break; }

    let t = tile_tris.data[tri_idx];
    let base_idx = 3u * t;
    if (base_idx + 2u >= indices_len) { continue; }

    let i0 = indices.data[base_idx + 0u];
    let i1 = indices.data[base_idx + 1u];
    let i2 = indices.data[base_idx + 2u];

    let verts_len = arrayLength(&verts.data);
    if (i0 >= verts_len || i1 >= verts_len || i2 >= verts_len) { continue; }

    let ch = sv_tri_color(p, i0, i1, i2);
    if (ch.hit) {
      return ColorHit(true, 0u, 0u, 0u, ch.color, t, 0u, ch.uv, 0u, 0u);
    }
  }
  return ColorHit(false, 0u, 0u, 0u, vec4<f32>(0.0), 0u, 0u, vec2<f32>(0.0), 0u, 0u);
}

// RNG Helper
fn wang_hash(x0: u32) -> u32 {
  var x = x0;
  x = (x ^ 61u) ^ (x >> 16u);
  x = x + (x << 3u);
  x = x ^ (x >> 4u);
  x = x * 0x27d4eb2du;
  x = x ^ (x >> 15u);
  return x;
}

// Combine pixel, frame, and any salt to make a good seed
fn sv_seed(px: u32, py: u32, salt: u32) -> u32 {
  return wang_hash(px ^ (py << 11u) ^ salt);
}

// Convert seed to [0,1)
fn sv_rand01(seed: u32) -> f32 {
  let h = wang_hash(seed);
  // 1/2^32 as f32
  return f32(h) * (1.0 / 4294967296.0);
}
