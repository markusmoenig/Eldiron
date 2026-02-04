// Default SDF body that runs without any scene geometry.
// Each entry in SDF_DATA is a vec4 encoding an atlas rect: (ofs.x, ofs.y, scale.x, scale.y).
// You can pack rects with SceneVM::atlas_sdf_uv4 (host side) and fetch them here with sdf_data_at().

fn circle_sdf(p: vec2<f32>, c: vec2<f32>, r: f32) -> f32 {
  return length(p - c) - r;
}

fn render_scene(uv: vec2<f32>) -> vec4<f32> {
  let count = U.data_len;

  // If no data is provided, just return the background color.
  if (count == 0u) {
    return vec4<f32>(U.background.rgb, 1.0);
  }

  // Simple SDF: soft circle centered via gp3.xy, radius gp3.z, thickness gp3.w
  let c = vec2<f32>(U.gp3.x, U.gp3.y);
  let radius = max(U.gp3.z, 0.25);
  let thickness = max(U.gp3.w, 0.01);
  let d = circle_sdf(uv, c, radius);
  let edge = abs(d) - thickness;

  // Sample atlas using the first rect if available; otherwise mix background/gp1.
  var sample_col = mix(U.background.rgb, U.gp1.rgb, clamp(U.gp1.a, 0.0, 1.0));
  if (count > 0u) {
    let rect = sdf_data_at(0u);
    sample_col = sdf_sample_atlas(rect, fract(uv)).rgb;
  }

  let feather = max(1.5 / f32(max(U.fb_size.x, U.fb_size.y)), 1e-4);
  let alpha = 1.0 - smoothstep(0.0, feather, edge);
  let glow = exp(-4.0 * abs(edge)) * 0.25;
  let rgb = sample_col + glow * U.gp2.rgb;
  return vec4<f32>(rgb, alpha);
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let use_viewport = (U.viewport_rect.z > 0.0) && (U.viewport_rect.w > 0.0);
  let viewport_origin = vec2<u32>(
    u32(max(U.viewport_rect.x, 0.0)),
    u32(max(U.viewport_rect.y, 0.0))
  );
  let viewport_size = vec2<u32>(
    u32(max(U.viewport_rect.z, 0.0)),
    u32(max(U.viewport_rect.w, 0.0))
  );

  var px = gid.xy;
  if (use_viewport) {
    if (gid.x >= viewport_size.x || gid.y >= viewport_size.y) {
      return;
    }
    let clamped_origin = min(viewport_origin, U.fb_size);
    px = clamped_origin + gid.xy;
  }

  if (px.x >= U.fb_size.x || px.y >= U.fb_size.y) {
    return;
  }

  let uv = vec2<f32>(
    f32(px.x) / max(f32(U.fb_size.x), 1.0),
    f32(px.y) / max(f32(U.fb_size.y), 1.0)
  );

  if ((U.vm_flags & 1u) == 0u) {
    // Clear background when not preserving the surface.
    clear_if_needed(px);
  }

  let color = render_scene(uv);
  textureStore(color_out, vec2<i32>(i32(px.x), i32(px.y)), color);
}
