// UI 2D body: shades rounded rects with optional border using atlas/material texels.
// Expects:
// - atlas texel 0: fill color (rgba8)
// - atlas texel 1: border color (rgba8)
// - material texel 0: radius_px, border_px in pixels

// Uses bindings/types from 2d_header.wgsl (U2D, verts, tile_bins, tile_tris, atlas_tex, atlas_mat_tex, etc.).
// Note: relies on the tiled 2D pipeline bindings already defined in the header; no bindings are re-declared here.

struct Style {
  fill: vec4<f32>,
  border: vec4<f32>,
  radius_px: f32,
  border_px: f32,
};

const STYLE_FLAG: f32 = 0.5; // params.b > 0.5 => style tile
const WIDGET_TYPE_BUTTON: f32 = 1.0;
const WIDGET_TYPE_COLOR_WHEEL: f32 = 2.0;

fn load_style(tile_index: u32) -> Style {
  let frame = sv_tile_frame(tile_index);
  // Sample centers of texel 0 and 1 in a 2x1 tile.
  let uv_fill = frame.ofs + frame.scale * vec2<f32>(0.25, 0.5);
  let uv_border = frame.ofs + frame.scale * vec2<f32>(0.75, 0.5);
  let uv_params = uv_fill;

  let fill = textureSampleLevel(atlas_tex, atlas_smp, uv_fill, 0.0);
  let border = textureSampleLevel(atlas_tex, atlas_smp, uv_border, 0.0);
  let params = textureSampleLevel(atlas_mat_tex, atlas_smp, uv_params, 0.0);
  // params.r = widget_type (1=button), params.g = radius_px (0-255), params.b = 255 (style flag), params.a = border_px (0-255)
  let radius_px = params.g * 255.0; // Convert back from 0-1 to pixels
  let border_px = params.a * 255.0; // Convert back from 0-1 to pixels
  return Style(fill, border, radius_px, border_px);
}

// HSV to RGB conversion where hue is in turns (0..1) for better uniformity and avoiding 0/360 wrap bias.
fn hsv_turns_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
  let h6 = h * 6.0;            // [0,6)
  let i = i32(floor(h6)) % 6;  // sector 0..5
  let f = h6 - floor(h6);      // fractional part

  let p = v * (1.0 - s);
  let q = v * (1.0 - s * f);
  let t = v * (1.0 - s * (1.0 - f));

  if (i == 0) {
    return vec3<f32>(v, t, p);
  } else if (i == 1) {
    return vec3<f32>(q, v, p);
  } else if (i == 2) {
    return vec3<f32>(p, v, t);
  } else if (i == 3) {
    return vec3<f32>(p, q, v);
  } else if (i == 4) {
    return vec3<f32>(t, p, v);
  }
  return vec3<f32>(v, p, q); // i == 5
}

fn shade_color_wheel_px(tile_index: u32, local_uv: vec2<f32>, rect_size: vec2<f32>, value: f32, background: vec4<f32>) -> vec4<f32> {
  // Rectangular HSV picker:
  // - Hue across X (mirrored so hue=0 on the right).
  // - Y: top half ramps saturation 0->1 at full value (white to vivid),
  //       bottom half keeps saturation=1 and ramps value 1->0 (vivid to black).
  let u = clamp(local_uv.x, 0.0, 1.0);
  let v = clamp(local_uv.y, 0.0, 1.0);

  // Mirror X so visual matches expected left/right hue layout.
  // Use hue in turns to avoid seam bias (0 and 1 are the same hue).
  let hue_turns = 1.0 - u; // 0..1

  var sat: f32 = 0.0;
  var val: f32 = 1.0;
  if (v < 0.5) {
    let t = v * 2.0;
    sat = t;
    val = 1.0;
  } else {
    let t = (v - 0.5) * 2.0;
    sat = 1.0;
    val = 1.0 - t;
  }

  let rgb = hsv_turns_to_rgb(hue_turns, sat, val);

  // Full coverage for the quad; alpha 1 so the tri coverage controls blending.
  return vec4<f32>(rgb, 1.0);
}

fn shade_style_px(tile_index: u32, local_uv: vec2<f32>, rect_size: vec2<f32>, fb_size: vec2<u32>, background: vec4<f32>) -> vec4<f32> {
  let style = load_style(tile_index);

  let radius_px = style.radius_px;
  let border_px = style.border_px;

  // Work in pixel space - convert UV to pixel position within rect
  let pixel_pos = local_uv * rect_size;
  let half_size = rect_size * 0.5;

  // Rounded box SDF in pixel space
  let p = abs(pixel_pos - half_size);
  let shrink = half_size - vec2<f32>(radius_px, radius_px);
  let d = p - shrink;
  let dist = length(max(d, vec2<f32>(0.0, 0.0))) + min(max(d.x, d.y), 0.0) - radius_px;

  // Antialiasing in pixels
  let fw = 1.0;
  let body = 1.0 - smoothstep(0.0, fw, dist);

  // Border calculation in pixel space - only apply if border_px > 0
  var surf = style.fill;
  if (border_px > 0.01) {
    // border_band = 1.0 at the edge, 0.0 in the center (inside border region)
    let border_band = smoothstep(-border_px - fw, -border_px + fw, dist);
    surf = mix(style.fill, style.border, border_band);
  }

  let cov = clamp(body, 0.0, 1.0);
  let rgb = mix(background.rgb, surf.rgb, cov);
  let a = mix(background.a, surf.a, cov);

  return vec4<f32>(rgb, a);
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let px = gid.x;
  let py = gid.y;
  if (px >= U.fb_size.x || py >= U.fb_size.y) { return; }

  // Layer texture is already cleared by render pass
  let tid = tile_of_px(px, py);
  let bins_len = arrayLength(&tile_bins.data);
  if (bins_len == 0u || tid >= bins_len) { return; }
  let bin = tile_bins.data[tid];

  let tris_len = arrayLength(&tile_tris.data);
  let indices_len = arrayLength(&indices.data);
  let verts_len = arrayLength(&verts.data);

  var out_col = U.background;
  var covered = false;

  let p = vec2<f32>(f32(px) + 0.5, f32(py) + 0.5);

  // Process all triangles in this tile bin in REVERSE order (front to back)
  // Triangles are sorted by layer, highest first, so we process backwards
  for (var k: u32 = 0u; k < bin.count; k = k + 1u) {
    let tri_idx = bin.offset + (bin.count - 1u - k);
    if (tri_idx >= tris_len) { break; }

    let t = tile_tris.data[tri_idx];
    let base = 3u * t;
    if (base + 2u >= indices_len) { continue; }

    let i0 = indices.data[base + 0u];
    let i1 = indices.data[base + 1u];
    let i2 = indices.data[base + 2u];
    if (i0 >= verts_len || i1 >= verts_len || i2 >= verts_len) { continue; }

    let p0 = verts.data[i0].pos;
    let p1 = verts.data[i1].pos;
    let p2 = verts.data[i2].pos;
    let bh = sv_tri_bary(p, p0, p1, p2);
    if (!bh.hit) { continue; }

    let w = bh.w;
    let uv0 = verts.data[i0].uv;
    let uv1 = verts.data[i1].uv;
    let uv2 = verts.data[i2].uv;
    let uv = w.x * uv0 + w.y * uv1 + w.z * uv2;

    let tile_idx = verts.data[i0].tile_index;
    let frame = sv_tile_frame(tile_idx);
    let atlas_uv = frame.ofs + uv * frame.scale;
    let params = textureSampleLevel(atlas_mat_tex, atlas_smp, atlas_uv, 0.0);
    let is_style = params.b > STYLE_FLAG;

    if (is_style) {
      // Calculate rect size from vertices (assumes axis-aligned quad)
      let min_pos = min(min(p0, p1), p2);
      let max_pos = max(max(p0, p1), p2);
      let rect_size = max_pos - min_pos;

      // Check widget type from material texture
      let widget_type = params.r * 255.0;

      var col: vec4<f32>;
      if (abs(widget_type - WIDGET_TYPE_COLOR_WHEEL) < 0.5) {
        // Color wheel: material contains widget type, U.gp0.z contains HSV value (0-1)
        let value = U.gp0.z;
        col = shade_color_wheel_px(tile_idx, uv, rect_size, value, out_col);
      } else {
        // Default to button/rounded rect shading
        col = shade_style_px(tile_idx, uv, rect_size, U.fb_size, out_col);
      }

      out_col = col;
      covered = true;
    } else {
      // Use linear filtering for text glyph tiles (material texel ~0), otherwise nearest.
      let use_linear = all(params < vec4<f32>(0.0001));
      var col: vec4<f32>;
      if (use_linear) {
        col = textureSampleLevel(atlas_tex, atlas_smp_linear, atlas_uv, 0.0);
      } else {
        col = textureSampleLevel(atlas_tex, atlas_smp, atlas_uv, 0.0);
      }
      if (col.a < 0.01) { continue; }
      // Alpha blend this texture layer over the accumulated color
      out_col = vec4<f32>(
        mix(out_col.rgb, col.rgb, col.a),
        1.0
      );
      covered = true;
    }
  }

  if (covered) {
    sv_write(px, py, out_col);
  }
  // Uncovered pixels keep the cleared background from render pass
}
