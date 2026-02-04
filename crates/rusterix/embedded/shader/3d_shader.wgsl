// PBR Ray-Traced Shader for Eldiron
// Supports: Cook-Torrance BRDF, ray-traced shadows, AO, full material system, and bump mapping
//
// Uniforms (gp0-gp9):
// - gp0.xyz: Sky color (RGB)
// - gp0.w:   Unused
// - gp1.xyz: Sun color (RGB)
// - gp1.w:   Sun intensity
// - gp2.xyz: Sun direction (normalized)
// - gp2.w:   Sun enabled (0.0 = disabled, 1.0 = enabled)
// - gp3.xyz: Ambient color (RGB, independent from sky)
// - gp3.w:   Ambient strength
// - gp4.xyz: Fog color (RGB)
// - gp4.w:   Fog density (0.0 = no fog, higher values = denser fog)
// - gp5.x:   AO samples (number of rays, default 8)
// - gp5.y:   AO radius (default 0.5)
// - gp5.z:   Bump strength (0.0-1.0, default 1.0)
// - gp5.w:   Max transparency bounces (default 8)
// - gp6.x:   Max shadow distance (default 10.0)
// - gp6.y:   Max sky distance (default 50.0)
// - gp6.z:   Max shadow steps (default 0, 0=binary/fast, >0=transparent shadows)
// - gp6.w:   Reflection samples (0 = disabled, >=1 = GGX PBR reflection rays)

// ===== Constants =====
const PI: f32 = 3.14159265359;
const MIN_ROUGHNESS: f32 = 0.04;

// ===== Hash functions for random sampling =====
fn hash13(p3: vec3<f32>) -> f32 {
    var p = fract(p3 * 0.1031);
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}

fn hash33(p3: vec3<f32>) -> vec3<f32> {
    var p = fract(p3 * vec3<f32>(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yxz + 33.33);
    return fract((p.xxy + p.yxx) * p.zyx);
}

// ===== Cosine-weighted hemisphere sampling =====
fn cosine_sample_hemisphere(u1: f32, u2: f32) -> vec3<f32> {
    let r = sqrt(u1);
    let theta = 2.0 * PI * u2;
    let x = r * cos(theta);
    let y = r * sin(theta);
    let z = sqrt(max(0.0, 1.0 - u1));
    return vec3<f32>(x, y, z);
}

// Build orthonormal basis from normal
fn build_onb(N: vec3<f32>) -> mat3x3<f32> {
    let up = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(N.y) > 0.999);
    let T = normalize(cross(up, N));
    let B = cross(N, T);
    return mat3x3<f32>(T, B, N);
}

// ===== GGX Importance Sampling =====
// Sample a microfacet normal based on GGX distribution
fn sample_ggx(u1: f32, u2: f32, roughness: f32) -> vec3<f32> {
    let a = roughness * roughness;
    let a2 = a * a;

    let phi = 2.0 * PI * u1;
    let cos_theta = sqrt((1.0 - u2) / (1.0 + (a2 - 1.0) * u2));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);

    return vec3<f32>(
        cos(phi) * sin_theta,
        sin(phi) * sin_theta,
        cos_theta
    );
}

// ===== PBR Helper Functions =====

// GGX/Trowbridge-Reitz normal distribution function
fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;

    let denom = (NdotH2 * (a2 - 1.0) + 1.0);
    return a2 / (PI * denom * denom + 1e-7);
}

// Schlick-GGX geometry function (single direction)
fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k + 1e-7);
}

// Smith's method for geometry obstruction
fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx2 = geometry_schlick_ggx(NdotV, roughness);
    let ggx1 = geometry_schlick_ggx(NdotL, roughness);
    return ggx1 * ggx2;
}

// Fresnel-Schlick approximation
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// ===== Material unpacking =====
struct Material {
    roughness: f32,
    metallic: f32,
    opacity: f32,
    emissive: f32,
    normal: vec3<f32>,
};

fn unpack_material(mats: vec4<f32>) -> Material {
    // Convert RGBA back to packed u32
    let packed = u32(mats.x * 255.0) | (u32(mats.y * 255.0) << 8u) |
                 (u32(mats.z * 255.0) << 16u) | (u32(mats.w * 255.0) << 24u);

    // Lower 16 bits: materials (4 bits each)
    let mat_bits = packed & 0xFFFFu;
    let roughness = clamp(
        max(f32(mat_bits & 0xFu) / 15.0, MIN_ROUGHNESS),
        MIN_ROUGHNESS,
        1.0
    );
    let metallic = f32((mat_bits >> 4u) & 0xFu) / 15.0;
    let opacity = f32((mat_bits >> 8u) & 0xFu) / 15.0;
    let emissive = f32((mat_bits >> 12u) & 0xFu) / 15.0;

    // Upper 16 bits: normal X,Y (8 bits each)
    let norm_bits = (packed >> 16u) & 0xFFFFu;
    let nx = (f32(norm_bits & 0xFFu) / 255.0) * 2.0 - 1.0;
    let ny = (f32((norm_bits >> 8u) & 0xFFu) / 255.0) * 2.0 - 1.0;
    let nz = sqrt(max(0.0, 1.0 - nx * nx - ny * ny));

    return Material(roughness, metallic, opacity, emissive, vec3<f32>(nx, ny, nz));
}

// ===== Unified Hit System =====
// Unified hit record for both geometry and billboards with material data

const HIT_TYPE_NONE: u32 = 0u;
const HIT_TYPE_GEOMETRY: u32 = 1u;
const HIT_TYPE_BILLBOARD: u32 = 2u;

struct UnifiedHit {
    hit_type: u32,          // 0=none, 1=geometry, 2=billboard
    t: f32,                 // distance along ray
    position: vec3<f32>,    // world space hit position
    normal: vec3<f32>,      // surface normal (interpolated for geo, computed for billboard)
    albedo: vec4<f32>,      // RGBA albedo (linear space)
    material: Material,     // unpacked material data
    // Internal data for re-querying if needed
    tri: u32,               // triangle index (geometry only)
    billboard_index: u32,   // billboard index (billboard only)
};

// ===== Billboard Support =====
// Modular billboard system for dynamic objects (particles, sprites, effects, etc.)

struct BillboardHit {
    hit: bool,              // offset 0, size 4 (stored as u32 in SPIR-V)
    t: f32,                 // offset 4, size 4
    uv: vec2<f32>,          // offset 8, size 8 (needs 8-byte alignment)
    tile_index: u32,        // offset 16, size 4
    billboard_index: u32,   // offset 20, size 4
    repeat_mode: u32,       // offset 24, size 4
    _pad1: u32,             // offset 28, size 4 (pad to 16-byte boundary)
};

/// Ray-billboard intersection test
/// Returns hit information if ray intersects the billboard quad
fn intersect_billboard(ro: vec3<f32>, rd: vec3<f32>, center: vec3<f32>,
                       axis_right: vec3<f32>, axis_up: vec3<f32>,
                       tile_index: u32, billboard_idx: u32, repeat_mode: u32,
                       width: f32, height: f32) -> BillboardHit {
    var result = BillboardHit(false, 0.0, vec2<f32>(0.0), 0u, 0u, 0u, 0u);

    // Compute billboard normal
    let normal = normalize(cross(axis_right, axis_up));

    // Ray-plane intersection
    let denom = dot(normal, rd);
    if (abs(denom) < 1e-5) {
        return result; // Ray parallel to billboard
    }

    let t = dot(center - ro, normal) / denom;
    if (t <= 0.0) {
        return result; // Behind ray origin
    }

    // Compute hit point and local coordinates
    let hit_pos = ro + rd * t;
    let rel = hit_pos - center;

    let len_right2 = max(dot(axis_right, axis_right), 1e-6);
    let len_up2 = max(dot(axis_up, axis_up), 1e-6);

    let u = dot(rel, axis_right) / len_right2;
    let v = dot(rel, axis_up) / len_up2;

    // Check if within quad bounds
    if (abs(u) > 1.0 || abs(v) > 1.0) {
        return result;
    }

    // Convert to texture coordinates [0,1]
    // For repeat mode, scale UVs by the billboard dimensions
    var uv: vec2<f32>;
    if (repeat_mode == 1u) {
        // Repeat mode: scale UVs by width/height so tiles repeat at their natural size
        uv = vec2<f32>((u + 1.0) * 0.5 * width, (1.0 - v) * 0.5 * height);
    } else {
        // Scale mode: single UV [0,1] mapping
        uv = vec2<f32>(0.5 * (u + 1.0), 0.5 * (1.0 - v));
    }

    result.hit = true;
    result.t = t;
    result.uv = uv;
    result.tile_index = tile_index;
    result.billboard_index = billboard_idx;
    result.repeat_mode = repeat_mode;

    return result;
}

/// Sample billboard color from atlas
/// This is separated to allow future procedural effects (fire, smoke, etc.)
fn sample_billboard(hit: BillboardHit) -> vec4<f32> {
    // Get tile frame information
    let frame = sv_tile_frame(hit.tile_index);

    // Map UV to atlas coordinates based on repeat mode
    var atlas_uv: vec2<f32>;

    if (hit.repeat_mode == 1u) {
        // Repeat mode: wrap UVs and map into atlas sub-rect
        let uv_wrapped = fract(hit.uv);
        atlas_uv = frame.ofs + uv_wrapped * frame.scale;

        // Clamp to avoid bleeding from neighboring tiles
        let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0));
        let pad_uv = vec2<f32>(0.5) / atlas_dims;
        let uv_min = frame.ofs + pad_uv;
        let uv_max = frame.ofs + frame.scale - pad_uv;
        atlas_uv = clamp(atlas_uv, uv_min, uv_max);
    } else {
        // Scale mode (default): scale the tile to fit billboard size
        atlas_uv = frame.ofs + hit.uv * frame.scale;
    }

    // Sample albedo from atlas
    let color = textureSampleLevel(atlas_tex, atlas_smp, atlas_uv, 0.0);

    return color;
}

/// Sample billboard material data (RMOE)
fn sample_billboard_material(hit: BillboardHit) -> vec4<f32> {
    // Get tile frame information
    let frame = sv_tile_frame(hit.tile_index);

    // Map UV to atlas coordinates based on repeat mode
    var atlas_uv: vec2<f32>;

    if (hit.repeat_mode == 1u) {
        // Repeat mode: wrap UVs and map into atlas sub-rect
        let uv_wrapped = fract(hit.uv);
        atlas_uv = frame.ofs + uv_wrapped * frame.scale;

        // Clamp to avoid bleeding from neighboring tiles
        let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0));
        let pad_uv = vec2<f32>(0.5) / atlas_dims;
        let uv_min = frame.ofs + pad_uv;
        let uv_max = frame.ofs + frame.scale - pad_uv;
        atlas_uv = clamp(atlas_uv, uv_min, uv_max);
    } else {
        // Scale mode (default): scale the tile to fit billboard size
        atlas_uv = frame.ofs + hit.uv * frame.scale;
    }

    // Sample material from material atlas
    let material = textureSampleLevel(atlas_mat_tex, atlas_smp, atlas_uv, 0.0);

    return material;
}

/// Trace all billboards and return the closest hit
/// This is modular to allow future expansion for different billboard types
fn trace_billboards(ro: vec3<f32>, rd: vec3<f32>, max_t: f32) -> BillboardHit {
    var closest = BillboardHit(false, max_t, vec2<f32>(0.0), 0u, 0u, 0u, 0u);

    let billboard_count = scene_data.header.billboard_cmd_count;
    if (billboard_count == 0u) {
        return closest;
    }

    for (var i: u32 = 0u; i < billboard_count; i = i + 1u) {
        let cmd = sd_billboard_cmd(i);

        // Extract billboard parameters
        let center = cmd.center.xyz;
        let width = cmd.center.w;
        let axis_right = cmd.axis_right.xyz;
        let height = cmd.axis_right.w;
        let axis_up = cmd.axis_up.xyz;
        let repeat_mode = u32(cmd.axis_up.w);
        let tile_index = cmd.params.x;

        // Test intersection
        let hit = intersect_billboard(ro, rd, center, axis_right, axis_up, tile_index, i, repeat_mode, width, height);

        if (hit.hit && hit.t < closest.t) {
            closest = hit;
        }
    }

    return closest;
}

// ===== Unified Trace Function =====
// Traces both geometry and billboards, returns complete hit with materials
fn trace_unified(ro: vec3<f32>, rd: vec3<f32>, tmin: f32, tmax: f32) -> UnifiedHit {
    // Trace geometry
    let geo_hit = sv_trace_grid(ro, rd, tmin, tmax);

    // Trace billboards
    let billboard_hit = trace_billboards(ro, rd, tmax);

    // Determine which is closer
    let use_billboard = billboard_hit.hit && (!geo_hit.hit || billboard_hit.t < geo_hit.t);

    if (use_billboard) {
        // Build unified hit from billboard
        let P = ro + rd * billboard_hit.t;

        // Sample billboard data
        var albedo = sample_billboard(billboard_hit);
        let mat_data = sample_billboard_material(billboard_hit);
        let mat = unpack_material(mat_data);

        // Convert albedo to linear
        albedo = vec4<f32>(pow(albedo.rgb, vec3<f32>(2.2)), albedo.a);

        // Compute billboard normal
        let cmd = sd_billboard_cmd(billboard_hit.billboard_index);
        let axis_right = cmd.axis_right.xyz;
        let axis_up = cmd.axis_up.xyz;
        var N = normalize(cross(axis_right, axis_up));

        // Two-sided: flip normal to face ray
        if (dot(N, rd) > 0.0) { N = -N; }

        return UnifiedHit(
            HIT_TYPE_BILLBOARD,
            billboard_hit.t,
            P,
            N,
            albedo,
            mat,
            0u,
            billboard_hit.billboard_index
        );
    } else if (geo_hit.hit) {
        // Build unified hit from geometry
        let P = ro + rd * geo_hit.t;

        // Get triangle indices
        let tri = geo_hit.tri;
        let i0 = indices3d.data[3u * tri + 0u];
        let i1 = indices3d.data[3u * tri + 1u];
        let i2 = indices3d.data[3u * tri + 2u];

        let v0 = verts3d.data[i0];
        let v1 = verts3d.data[i1];
        let v2 = verts3d.data[i2];

        // Barycentric interpolation
        let w = 1.0 - geo_hit.u - geo_hit.v;

        // Interpolate smooth normal
        var N = normalize(v0.normal * w + v1.normal * geo_hit.u + v2.normal * geo_hit.v);

        // Sample albedo and material
        var albedo = sv_tri_sample_albedo(i0, i1, i2, geo_hit.u, geo_hit.v);
        albedo = vec4<f32>(pow(albedo.rgb, vec3<f32>(2.2)), albedo.a);

        let mat_data = sv_tri_sample_rmoe(i0, i1, i2, geo_hit.u, geo_hit.v);
        let mat = unpack_material(mat_data);

        // Apply bump mapping
        let bump_strength = select(1.0, U.gp5.z, U.gp5.z >= 0.0);
        if (bump_strength > 0.0 && length(mat.normal) > 0.1) {
            let TBN = sv_tri_tbn(v0.pos, v1.pos, v2.pos, v0.uv, v1.uv, v2.uv);
            let N_ts = mat.normal;
            let N_ws = normalize(TBN * N_ts);
            N = normalize(mix(N, N_ws, bump_strength));
        }

        // Two-sided lighting
        if (dot(N, rd) > 0.0) { N = -N; }

        return UnifiedHit(
            HIT_TYPE_GEOMETRY,
            geo_hit.t,
            P,
            N,
            albedo,
            mat,
            tri,
            0u
        );
    }

    // No hit
    return UnifiedHit(
        HIT_TYPE_NONE,
        tmax,
        vec3<f32>(0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec4<f32>(0.0),
        Material(0.0, 0.0, 0.0, 0.0, vec3<f32>(0.0)),
        0u,
        0u
    );
}

// ===== Unified Shadow Trace (lightweight, no full materials for performance) =====
fn trace_shadow_unified(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> f32 {
    let geo_hit = sv_trace_grid(ro, rd, 0.0, tmax);
    let billboard_hit = trace_billboards(ro, rd, tmax);

    let use_billboard = billboard_hit.hit && (!geo_hit.hit || billboard_hit.t < geo_hit.t);

    if (use_billboard) {
        // Sample billboard alpha and material opacity
        let albedo = sample_billboard(billboard_hit);
        let mat_data = sample_billboard_material(billboard_hit);
        let mat = unpack_material(mat_data);
        return mat.opacity * albedo.a;
    } else if (geo_hit.hit) {
        // Sample geometry material opacity
        let tri = geo_hit.tri;
        let i0 = indices3d.data[3u * tri + 0u];
        let i1 = indices3d.data[3u * tri + 1u];
        let i2 = indices3d.data[3u * tri + 2u];
        let mat_data = sv_tri_sample_rmoe(i0, i1, i2, geo_hit.u, geo_hit.v);
        let mat = unpack_material(mat_data);
        return mat.opacity;
    }

    return 0.0; // No hit = no shadow
}

// ===== Ray-traced shadows with opacity support =====
fn trace_shadow(P: vec3<f32>, L: vec3<f32>, max_dist: f32) -> f32 {
    let shadow_bias = 0.01; // Increased from 0.001 to avoid edge artifacts
    let max_shadow_steps = u32(select(0.0, U.gp6.z, U.gp6.z >= 0.0));

    // Minimum distance to light to avoid self-shadowing when light is near/in geometry
    let min_light_dist = 0.1;

    // Fast path: Binary shadow test
    if (max_shadow_steps == 0u) {
        let opacity = trace_shadow_unified(P + L * shadow_bias, L, max_dist - min_light_dist);
        if (opacity > 0.99) {
            return 0.0; // Fully opaque shadow
        }
        return 1.0; // lit
    }

    // Slow path: Multi-step transparency-aware shadows
    var current_pos = P + L * shadow_bias;
    var remaining_dist = max_dist;
    var transparency = 1.0;

    for (var step: u32 = 0u; step < max_shadow_steps; step = step + 1u) {
        let opacity = trace_shadow_unified(current_pos, L, remaining_dist);

        if (opacity < 0.01) {
            break; // No more occlusion
        }

        // Get the actual hit distance to advance the ray
        let geo_hit = sv_trace_grid(current_pos, L, 0.0, remaining_dist);
        let billboard_hit = trace_billboards(current_pos, L, remaining_dist);
        let use_billboard = billboard_hit.hit && (!geo_hit.hit || billboard_hit.t < geo_hit.t);
        let hit_t = select(geo_hit.t, billboard_hit.t, use_billboard);

        if (remaining_dist - hit_t < min_light_dist) {
            break;
        }

        transparency *= (1.0 - opacity);

        if (transparency < 0.01) {
            return 0.0;
        }

        current_pos = current_pos + L * (hit_t + shadow_bias);
        remaining_dist = remaining_dist - hit_t - shadow_bias;

        if (remaining_dist <= 0.0) {
            break;
        }
    }

    return transparency;
}

// ===== Ambient Occlusion with opacity support =====
fn compute_ao(P: vec3<f32>, N: vec3<f32>, seed: vec3<f32>) -> f32 {
    // Read AO parameters from GP5 (negative = use default, 0.0 = disable/off)
    let ao_samples = u32(select(8.0, U.gp5.x, U.gp5.x >= 0.0));
    let ao_radius = select(0.5, U.gp5.y, U.gp5.y >= 0.0);

    // Early return if AO is disabled
    if (ao_samples == 0u || ao_radius <= 0.0) {
        return 1.0;
    }

    let onb = build_onb(N);
    var occlusion = 0.0;

    for (var i: u32 = 0u; i < ao_samples; i = i + 1u) {
        let hash_seed = seed + vec3<f32>(f32(i) * 0.1);
        let u1 = hash13(hash_seed);
        let u2 = hash13(hash_seed + vec3<f32>(7.3, 11.7, 13.1));

        // Cosine-weighted hemisphere sample
        let local_dir = cosine_sample_hemisphere(u1, u2);
        let world_dir = onb * local_dir;

        let opacity = trace_shadow_unified(P + N * 0.001, world_dir, ao_radius);

        if (opacity > 0.01) {
            // Get hit distance for distance-based falloff
            let geo_hit = sv_trace_grid(P + N * 0.001, world_dir, 0.0, ao_radius);
            let billboard_hit = trace_billboards(P + N * 0.001, world_dir, ao_radius);
            let use_billboard = billboard_hit.hit && (!geo_hit.hit || billboard_hit.t < geo_hit.t);
            let hit_t = select(geo_hit.t, billboard_hit.t, use_billboard);

            let dist_factor = 1.0 - (hit_t / ao_radius);
            occlusion += dist_factor * opacity;
        }
    }

    return 1.0 - (occlusion / f32(ao_samples));
}

// ===== PBR Direct Lighting =====
fn pbr_lighting(P: vec3<f32>, N: vec3<f32>, V: vec3<f32>, albedo: vec3<f32>, mat: Material) -> vec3<f32> {
    var Lo = vec3<f32>(0.0);

    // Base reflectance at zero incidence (for dielectrics use 0.04, metals use albedo)
    let F0 = mix(vec3<f32>(0.04), albedo, mat.metallic);

    // ===== Directional Sun Light =====
    if (U.gp2.w > 0.5) { // Sun enabled
        let sun_dir = normalize(U.gp2.xyz);
        let sun_color = U.gp1.xyz; // Already in linear space
        let sun_intensity = U.gp1.w;

        let L = -sun_dir; // Light direction points FROM surface TO light
        let H = normalize(V + L);

        let NdotL = max(dot(N, L), 0.0);

        if (NdotL > 0.0) {
            // Ray-traced shadow
            let max_shadow_dist = select(10.0, U.gp6.x, U.gp6.x >= 0.0);
            let shadow = trace_shadow(P, L, max_shadow_dist);

            if (shadow > 0.01) {
                let radiance = sun_color * sun_intensity * shadow;

                // Cook-Torrance BRDF
                let NdotV = max(dot(N, V), 0.0);
                let NDF = distribution_ggx(N, H, mat.roughness);
                let G = geometry_smith(N, V, L, mat.roughness);
                let F = fresnel_schlick(max(dot(H, V), 0.0), F0);

                let numerator = NDF * G * F;
                let denominator = 4.0 * NdotV * NdotL + 1e-7;
                let specular = numerator / denominator;

                // Energy conservation
                let kS = F;
                let kD = (vec3<f32>(1.0) - kS) * (1.0 - mat.metallic);

                Lo += (kD * albedo / PI + specular) * radiance * NdotL;
            }
        }
    }

    // ===== Point Lights =====
    for (var li: u32 = 0u; li < U.lights_count; li = li + 1u) {
        let light = sd_light(li);

        if (light.header.y == 0u) { continue; } // skip non-emitting lights

        let Lp = light.position.xyz;
        let Lc = light.color.xyz;
        let Li = light.params0.x * light.params1.x; // intensity * flicker_multiplier

        let start_d = light.params0.z;
        let end_d = max(light.params0.w, start_d + 1e-3);

        let L_vec = Lp - P;
        let dist = length(L_vec);

        // Early exit if beyond range
        if (dist > end_d) { continue; }

        let L = normalize(L_vec);
        let H = normalize(V + L);

        // Distance-based attenuation with smooth falloff
        let dist2 = max(dot(L_vec, L_vec), 1e-6);

        // Smooth distance falloff between start and end
        let range_factor = smoothstep(end_d, start_d, dist);
        let attenuation = (Li * range_factor) / dist2;

        // Ray-traced shadow
        let shadow = trace_shadow(P, L, dist);
        if (shadow < 0.01) { continue; }

        let radiance = Lc * attenuation * shadow;

        // Cook-Torrance BRDF
        let NdotL = max(dot(N, L), 0.0);
        let NdotV = max(dot(N, V), 0.0);

        if (NdotL > 0.0) {
            let NDF = distribution_ggx(N, H, mat.roughness);
            let G = geometry_smith(N, V, L, mat.roughness);
            let F = fresnel_schlick(max(dot(H, V), 0.0), F0);

            let numerator = NDF * G * F;
            let denominator = 4.0 * NdotV * NdotL + 1e-7;
            var specular = numerator / denominator;

            // Clamp specular to prevent explosion at grazing angles
            specular = min(specular, vec3<f32>(1.0));

            // Energy conservation
            let kS = F;
            let kD = (vec3<f32>(1.0) - kS) * (1.0 - mat.metallic);

            Lo += (kD * albedo / PI + specular) * radiance * NdotL;
        }
    }

    return Lo;
}

// ===== Main Compute Shader =====
@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x;
    let py = gid.y;
    if (px >= U.fb_size.x || py >= U.fb_size.y) { return; }

    // Build camera ray
    let cam_uv = vec2<f32>(
        (f32(px) + 0.5) / f32(U.fb_size.x),
        (f32(py) + 0.5) / f32(U.fb_size.y)
    );
    let ray = cam_ray(cam_uv);
    var ro = ray.ro;
    let rd = normalize(ray.rd);

    // Accumulated color (front to back compositing)
    var accum_color = vec3<f32>(0.0);
    var accum_alpha = 0.0;
    var fog_distance = 0.0; // Track distance from camera for fog (set on first hit)
    var first_hit = true;

    // Sky color for background (already in linear space from CPU)
    let sky_rgb = select(U.background.rgb, U.gp0.xyz, length(U.gp0.xyz) > 0.01);
    let ambient_strength = U.gp3.w; // User-defined, no default

    // Trace through transparent layers
    let max_bounces = u32(max(1.0, select(8.0, U.gp5.w, U.gp5.w >= 0.0)));
    for (var bounce: u32 = 0u; bounce < max_bounces; bounce = bounce + 1u) {
        // First ray uses epsilon, continuation uses 0 to avoid self-intersection vs gaps
        let tmin = select(0.0, 0.001, bounce == 0u);

        // Unified trace for both geometry and billboards
        let unified_hit = trace_unified(ro, rd, tmin, 1e6);

        // No hit - sky
        if (unified_hit.hit_type == HIT_TYPE_NONE) {
            let sky_color = sky_rgb * (1.0 - accum_alpha);
            accum_color += sky_color;
            accum_alpha = 1.0;
            break;
        }

        // Alpha test - skip fully transparent pixels
        if (unified_hit.albedo.a < 0.01) {
            ro = ro + rd * (unified_hit.t + 0.001);
            continue;
        }

        // Track fog distance on first hit
        if (first_hit) {
            fog_distance = unified_hit.t;
            first_hit = false;
        }

        // Extract common hit data (already computed in trace_unified)
        let P = unified_hit.position;
        let N = unified_hit.normal;
        let albedo = unified_hit.albedo;
        let mat = unified_hit.material;
        let V = -rd;

        // Calculate final color based on emissive value
        var layer_color: vec3<f32>;

        if (mat.emissive > 0.99) {
            // Fully emissive (e.g., light billboards) - skip all lighting, just glow
            layer_color = albedo.rgb * 2.0;
        } else {
            // Regular surface with lighting

            // Compute ambient occlusion
            let ao = compute_ao(P, N, P + vec3<f32>(f32(px), f32(py), f32(bounce)));

            // PBR direct lighting
            let direct = pbr_lighting(P, N, V, albedo.rgb, mat);

            // Ambient contribution (already in linear space from CPU)
            let ambient_color = U.gp3.xyz;

            // Sky contribution: combine orientation and occlusion
            let sky_factor = max(dot(N, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
            let max_sky_dist = select(50.0, U.gp6.y, U.gp6.y >= 0.0);

            var sky_contribution = vec3<f32>(0.0);
            if (max_sky_dist > 0.0 && sky_factor > 0.0) {
                let sky_dir = reflect(rd, N);
                let sky_dir_up = max(dot(sky_dir, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
                let sky_visibility = select(0.0, trace_shadow(P, sky_dir, max_sky_dist), sky_dir_up > 0.0);
                sky_contribution = sky_rgb * sky_factor * sky_visibility;
            }

            // Combine ambient (uniform) and sky (directional based on upward facing)
            let ambient = (ambient_color * ambient_strength + sky_contribution) * albedo.rgb * ao;

            // Emissive contribution (self-illumination)
            let emissive = albedo.rgb * mat.emissive * 2.0;

            // PBR Reflections (GGX importance sampled, gp6.w = sample count)
            var reflections = vec3<f32>(0.0);
            let reflection_samples = u32(max(U.gp6.w, 0.0));

            if (reflection_samples > 0u) {
                let F0 = mix(vec3<f32>(0.04), albedo.rgb, mat.metallic);
                let roughness = clamp(mat.roughness, MIN_ROUGHNESS, 1.0);
                let onb = build_onb(N);
                let sample_count = max(reflection_samples, 1u);
                let max_refl_dist = select(50.0, U.gp6.y, U.gp6.y >= 0.0);

                var refl_accum = vec3<f32>(0.0);
                var weight_sum = 0.0;

                for (var s: u32 = 0u; s < sample_count; s = s + 1u) {
                    let rand = hash33(P + vec3<f32>(
                        f32(px) * 0.5 + f32(s),
                        f32(py) * 0.5 + f32(bounce) * 0.73,
                        f32(s) * 7.31
                    ));

                    // Sample microfacet normal and build reflection direction
                    let H_sample = sample_ggx(rand.x, rand.y, roughness);
                    let H = normalize(onb * H_sample);
                    let L = reflect(-V, H);
                    let NdotL = max(dot(N, L), 0.0);
                    if (NdotL <= 0.0) { continue; }

                    let refl_hit = trace_unified(P + N * 0.01, L, 0.0, max_refl_dist);

                    var sample_color = vec3<f32>(0.0);
                    if (refl_hit.hit_type == HIT_TYPE_NONE) {
                        // Reflect sky
                        sample_color = select(U.background.rgb, U.gp0.xyz, length(U.gp0.xyz) > 0.01);
                    } else {
                        // Get reflected surface properties
                        let refl_P = refl_hit.position;
                        let refl_N = refl_hit.normal;
                        let refl_V = -L;
                        let refl_albedo = refl_hit.albedo.rgb;
                        let refl_mat = refl_hit.material;

                        if (refl_mat.emissive > 0.99) {
                            sample_color = refl_albedo * 2.0;
                        } else {
                            let refl_direct = pbr_lighting(refl_P, refl_N, refl_V, refl_albedo, refl_mat);
                            let refl_ambient = (ambient_color * ambient_strength) * refl_albedo;
                            let refl_emissive = refl_albedo * refl_mat.emissive * 2.0;
                            sample_color = refl_direct + refl_ambient + refl_emissive;
                        }
                    }

                    let F = fresnel_schlick(max(dot(H, V), 0.0), F0);
                    refl_accum += sample_color * F * NdotL;
                    weight_sum += NdotL;
                }

                if (weight_sum > 0.0) {
                    reflections = refl_accum / weight_sum;
                }
            }

            // Combine lighting for this layer
            layer_color = direct + ambient + emissive + reflections;
        }

        // Calculate layer opacity (from material and texture alpha)
        let layer_opacity = albedo.a * mat.opacity;

        // Handle opaque vs transparent surfaces differently
        if (layer_opacity >= 0.99) {
            // Fully opaque - blend any previous transparent layers and use this color
            accum_color += layer_color * (1.0 - accum_alpha);
            accum_alpha = 1.0;
            break;
        } else {
            // Transparent - front-to-back alpha compositing
            accum_color += layer_color * layer_opacity * (1.0 - accum_alpha);
            accum_alpha += layer_opacity * (1.0 - accum_alpha);

            // Check if we've accumulated enough opacity to stop
            if (accum_alpha >= 0.99) {
                accum_alpha = 1.0;
                break;
            }

            // Continue ray from just past this surface
            ro = P + rd * 0.001;
        }
    }

    // Apply fog based on distance traveled
    var final_color = accum_color;

    let fog_density = U.gp4.w;
    if (fog_density > 0.0) {
        // Exponential squared fog: fog_amount = density * distance²
        let fog_amount = fog_density * fog_distance * fog_distance;
        let fog_factor = clamp(exp(-fog_amount), 0.0, 1.0);
        let fog_color = U.gp4.xyz; // Already in linear space from CPU

        // Mix between scene color and fog color based on fog factor
        // fog_factor = 1.0 means no fog (close), 0.0 means full fog (far)
        final_color = mix(fog_color, final_color, fog_factor);
    }

    // Apply tone mapping and gamma to accumulated color
    final_color = final_color / (final_color + vec3<f32>(1.0));
    final_color = pow(final_color, vec3<f32>(1.0 / 2.2));

    sv_write(px, py, vec4<f32>(final_color, accum_alpha));
}
