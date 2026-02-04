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

    // ===== choose tracing mode =====
    var hit_any = false;
    var best_t  = 1e30;
    var best_tri: u32 = 0u;
    var best_u = 0.0;
    var best_v = 0.0;

    // Try grid first; if it misses, fall back to brute-force triangle loop.
    var grid_used = true;
    if (grid_used) {
        let th = sv_trace_grid(ro, rd, 0.001, 1e6);
        if (th.hit) {
            hit_any = true;
            best_t = th.t;
            best_tri = th.tri;
            best_u = th.u;
            best_v = th.v;
            grid_used = true;
        }
    } else {
    // Brute-force fallback
        let tri_len: u32 = arrayLength(&indices3d.data);
        let tri_count: u32 = tri_len / 3u;
        for (var tri: u32 = 0u; tri < tri_count; tri = tri + 1u) {
            let base = 3u * tri;
            let i0 = indices3d.data[base + 0u];
            let i1 = indices3d.data[base + 1u];
            let i2 = indices3d.data[base + 2u];
            let a = verts3d.data[i0].pos;
            let b = verts3d.data[i1].pos;
            let c = verts3d.data[i2].pos;
            let h = sv_ray_tri_full(ro, rd, a, b, c);
            if (h.hit && h.t < best_t) {
                hit_any = true;
                best_t = h.t;
                best_tri = tri;
                best_u = h.u;
                best_v = h.v;
            }
        }
    }

    if (!hit_any) {
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

    let P = ro + rd * best_t;
    let uv_atlas = sv_tri_atlas_uv_obj(i0, i1, i2, best_u, best_v);

    var base_col = sv_tri_sample_albedo(i0, i1, i2, best_u, best_v);
    sv_write(px, py, base_col);
}
