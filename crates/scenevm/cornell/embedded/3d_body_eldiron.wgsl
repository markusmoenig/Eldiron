// --- Test Lambert shading (kept in BODY so headers stay generic) ---
fn lambert_pointlights(P: vec3<f32>, N: vec3<f32>, base_col: vec3<f32>) -> vec3<f32> {
    var diffuse = vec3<f32>(0.0);
    // Use background as ambient; clamp to a small minimum so unlit scenes aren't black
    let ambient = max(U.background.xyz, vec3<f32>(0.05, 0.05, 0.05));

    for (var li: u32 = 0u; li < U.lights_count; li = li + 1u) {
        let light = sd_light(li);

        if (light.header.y == 0u) { continue; } // emitting flag

        let Lp = light.position;
        let Lc = light.color.xyz;
        let Li = light.params0.x + light.params1.x;   // intensity + flicker

        let start_d = light.params0.z;
        let end_d   = max(light.params0.w, start_d + 1e-3);
        let L = Lp.xyz - P;
        let dist2 = max(dot(L, L), 1e-6);
        let dist = sqrt(dist2);
        let Ldir = normalize(L);

        // Always two-sided: use |N·L|
        let ndotl = abs(dot(N, Ldir));

        let fall = clamp((end_d - dist) / max(end_d - start_d, 1e-3), 0.0, 1.0);
        let atten = Li * ndotl * fall / dist2;
        diffuse += Lc * atten;
    }
    return base_col * (ambient + diffuse);
}

@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x; let py = gid.y;
    if (px >= U.fb_size.x || py >= U.fb_size.y) { return; }

    // Build pixel uv and get ray from the header-provided camera function
    let cam_uv = vec2<f32>( (f32(px) + 0.5) / f32(U.fb_size.x),
                            (f32(py) + 0.5) / f32(U.fb_size.y) );
    let ray = cam_ray(cam_uv);
    let ro = ray.ro;
    let rd = normalize(ray.rd);

    var hit_any = false;
    var best_t  = 1e30;
    var best_tri: u32 = 0u;
    var best_u = 0.0;
    var best_v = 0.0;

    let th = sv_trace_grid(ro, rd, 0.001, 1e6);
    if (th.hit) {
        hit_any = true;
        best_t = th.t;
        best_tri = th.tri;
        best_u = th.u;
        best_v = th.v;
    }

    if (!hit_any) {
      sv_write(px, py, U.background);
      return;
    }

    // Clamp the winning triangle id against current buffers (defensive)
    let tri_len_elems = arrayLength(&indices3d.data);
    let tri_len = tri_len_elems / 3u;
    let tri_safe = clamp_index_u(best_tri, tri_len);

    let i0 = indices3d.data[3u*tri_safe + 0u];
    let i1 = indices3d.data[3u*tri_safe + 1u];
    let i2 = indices3d.data[3u*tri_safe + 2u];

    let uv0 = verts3d.data[i0].uv; let n0 = verts3d.data[i0].normal;
    let uv1 = verts3d.data[i1].uv; let n1 = verts3d.data[i1].normal;
    let uv2 = verts3d.data[i2].uv; let n2 = verts3d.data[i2].normal;

    let w0 = 1.0 - best_u - best_v;
    // Interpolate smooth normal
    var N = normalize(n0*w0 + n1*best_u + n2*best_v);

    let P = ro + rd * best_t;
    let uv_atlas = sv_tri_atlas_uv_obj(i0, i1, i2, best_u, best_v);

    // Sample packed material+normal data (u32 format)
    // Lower 16 bits: materials (4 bits each for roughness, metallic, opacity, emissive)
    // Upper 16 bits: normal X,Y (8 bits each)
    let mats = sv_tri_sample_rmoe(i0, i1, i2, best_u, best_v);

    // Decompress materials from lower 16 bits (stored in x,y components as floats)
    // The packed u32 is returned as vec4, we need to convert back
    let packed = u32(mats.x * 255.0) | (u32(mats.y * 255.0) << 8u) |
                 (u32(mats.z * 255.0) << 16u) | (u32(mats.w * 255.0) << 24u);

    let mat_bits = packed & 0xFFFFu;
    let roughness = f32(mat_bits & 0xFu) / 15.0;
    let metallic = f32((mat_bits >> 4u) & 0xFu) / 15.0;
    let opacity = f32((mat_bits >> 8u) & 0xFu) / 15.0;
    let emission = f32((mat_bits >> 12u) & 0xFu) / 15.0;

    // Decompress normal from upper 16 bits
    let norm_bits = (packed >> 16u) & 0xFFFFu;
    let nx = (f32(norm_bits & 0xFFu) / 255.0) * 2.0 - 1.0;
    let ny = (f32((norm_bits >> 8u) & 0xFFu) / 255.0) * 2.0 - 1.0;
    let nz = sqrt(max(0.0, 1.0 - nx * nx - ny * ny));

    let BUMP_STRENGTH = 0.0;

    // Apply normal mapping using the packed normal data
    // Reconstruct triangle positions for TBN
    if (BUMP_STRENGTH) > 0.0 {
        let a = verts3d.data[i0].pos;
        let b = verts3d.data[i1].pos;
        let c = verts3d.data[i2].pos;

        let TBN = sv_tri_tbn(a, b, c, uv0, uv1, uv2);
        let n_ts = vec3<f32>(nx, ny, nz);
        let n_ws = normalize(TBN * n_ts);
        N = normalize(mix(N, n_ws, BUMP_STRENGTH));
    }
    //

    var base_col = sv_tri_sample_albedo(i0, i1, i2, best_u, best_v);
    if (dot(N, rd) > 0.0) { N = -N; } // two-sided

    let base_rgb = base_col.xyz;

    let lit = lambert_pointlights(P, N, base_rgb);

    // Add emission and apply opacity using decompressed material values
    let final_rgb = lit + emission * base_rgb;
    let final_a = base_col.a * opacity;

    sv_write(px, py, vec4<f32>(final_rgb, final_a));
}
