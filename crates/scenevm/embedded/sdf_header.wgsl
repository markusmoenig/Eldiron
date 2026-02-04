struct USdf {
  background: vec4<f32>,
  fb_size: vec2<u32>,
  _pad0: vec2<u32>,
  gp0: vec4<f32>,
  gp1: vec4<f32>,
  gp2: vec4<f32>,
  gp3: vec4<f32>,
  gp4: vec4<f32>,
  gp5: vec4<f32>,
  gp6: vec4<f32>,
  gp7: vec4<f32>,
  gp8: vec4<f32>,
  gp9: vec4<f32>,
  cam_pos: vec4<f32>,
  cam_fwd: vec4<f32>,
  cam_right: vec4<f32>,
  cam_up: vec4<f32>,
  cam_vfov_deg: f32,
  cam_ortho_half_h: f32,
  cam_near: f32,
  cam_far: f32,
  cam_kind: u32, // 0=OrthoIso, 1=OrbitPersp, 2=FirstPersonPersp
  _pad1: u32,
  _pad2: u32,
  _pad3: u32,
  data_len: u32,
  vm_flags: u32,
  anim_counter: u32,
  _pad4: u32,
  viewport_rect: vec4<f32>, // [x, y, width, height] in screen pixels. width=0 means full screen.
  palette: array<vec4<f32>, 256>,
};

@group(0) @binding(0) var<uniform> U: USdf;
@group(0) @binding(1) var color_out: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(5) var prev_layer: texture_2d<f32>;
struct SdfDataBuffer {
  data: array<vec4<f32>>,
};

@group(0) @binding(2) var<storage, read_write> SDF_DATA: SdfDataBuffer;
@group(0) @binding(3) var atlas_tex: texture_2d<f32>;
@group(0) @binding(4) var atlas_smp: sampler;

fn sdf_data_len() -> u32 {
  // arrayLength returns the number of vec4<f32> elements, not bytes
  return arrayLength(&SDF_DATA.data);
}

fn sdf_data_at(i: u32) -> vec4<f32> {
  let len = max(sdf_data_len(), 1u);
  return SDF_DATA.data[i % len];
}

fn set_sdf_data_at(i: u32, v: vec4<f32>) {
  let len = max(sdf_data_len(), 1u);
  SDF_DATA.data[i % len] = v;
}

fn clear_if_needed(px: vec2<u32>) {
  // Layer texture is already cleared by render pass - no action needed
}

fn sv_write(px: u32, py: u32, c: vec4<f32>) {
  textureStore(color_out, vec2<i32>(i32(px), i32(py)), c);
}

fn sdf_sample_atlas(rect: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
  let uv_atlas = rect.xy + rect.zw * uv;
  return textureSampleLevel(atlas_tex, atlas_smp, uv_atlas, 0.0);
}

struct Ray {
  origin: vec3<f32>,
  dir: vec3<f32>,
};

// Build a world-space ray from screen uv (0..1) using the camera in USdf.
fn sdf_create_ray(screen_uv: vec2<f32>) -> Ray {
  let u = clamp(screen_uv.x, 0.0, 1.0);
  let v = clamp(screen_uv.y, 0.0, 1.0);
  let ndc_x = u * 2.0 - 1.0;
  let ndc_y = (v * 2.0 - 1.0) * -1.0;
  let fb_w = max(f32(U.fb_size.x), 1.0);
  let fb_h = max(f32(U.fb_size.y), 1.0);

  let kind = U.cam_kind;
  if (kind == 0u) { // OrthoIso
    let aspect = fb_w / fb_h;
    let half_w = U.cam_ortho_half_h * aspect;
    let origin = U.cam_pos.xyz
      + U.cam_right.xyz * (ndc_x * half_w)
      + U.cam_up.xyz * (ndc_y * U.cam_ortho_half_h);
    return Ray(origin, normalize(U.cam_fwd.xyz));
  } else {
    let tan_half = tan(radians(U.cam_vfov_deg) * 0.5);
    let aspect = fb_w / fb_h;
    let dx = ndc_x * aspect * tan_half;
    let dy = ndc_y * tan_half;
    let dir = normalize(U.cam_fwd.xyz + U.cam_right.xyz * dx + U.cam_up.xyz * dy);
    return Ray(U.cam_pos.xyz, dir);
  }
}
