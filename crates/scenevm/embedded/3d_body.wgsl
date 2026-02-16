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
    _pad0: f32,  // pad to 32 bytes (16-byte aligned)
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

    return Material(roughness, metallic, opacity, emissive, vec3<f32>(nx, ny, nz), 0.0);
}

// ===== Unified Hit System =====
// Unified hit record for both geometry and billboards with material data

const HIT_TYPE_NONE: u32 = 0u;
const HIT_TYPE_GEOMETRY: u32 = 1u;
const HIT_TYPE_BILLBOARD: u32 = 2u;

struct UnifiedHit {
    hit_type: u32,          // offset 0
    t: f32,                 // offset 4
    _pad0: u32,             // offset 8
    _pad1: u32,             // offset 12 (align position to 16 bytes)
    position: vec3<f32>,    // offset 16 (16-byte aligned)
    _pad2: f32,             // offset 28
    normal: vec3<f32>,      // offset 32 (16-byte aligned)
    _pad3: f32,             // offset 44
    albedo: vec4<f32>,      // offset 48 (16-byte aligned)
    material: Material,     // offset 64 (Material is 32 bytes)
    tri: u32,               // offset 96
    billboard_index: u32,   // offset 100
    _pad4: u32,             // offset 104
    _pad5: u32,             // offset 108 (pad to 112 bytes, 16-byte aligned)
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
    _pad1: u32,             // offset 28, size 4 (pad to 32 bytes for 16-byte boundary alignment)
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
    let cmd = sd_billboard_cmd(hit.billboard_index);
    let opacity = bitcast<f32>(cmd.params.z);
    let uv = hit.uv;
    if (cmd.params.y == DYNAMIC_KIND_BILLBOARD_AVATAR) {
        var color = sd_sample_avatar(cmd.params.x, uv);
        color = color * opacity;
        return color;
    }

    // Get tile frame information
    let frame = sv_tile_frame(hit.tile_index);

    // Map UV to atlas coordinates based on repeat mode
    var atlas_uv: vec2<f32>;

    if (hit.repeat_mode == 1u) {
        // Repeat mode: wrap UVs and map into atlas sub-rect
        let uv_wrapped = fract(uv);
        atlas_uv = frame.ofs + uv_wrapped * frame.scale;

        // Clamp to avoid bleeding from neighboring tiles
        let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0));
        let pad_uv = vec2<f32>(0.5) / atlas_dims;
        let uv_min = frame.ofs + pad_uv;
        let uv_max = frame.ofs + frame.scale - pad_uv;
        atlas_uv = clamp(atlas_uv, uv_min, uv_max);
    } else {
        // Scale mode (default): scale the tile to fit billboard size
        atlas_uv = frame.ofs + uv * frame.scale;
    }

    // Sample albedo from atlas
    var color = textureSampleLevel(atlas_tex, atlas_smp, atlas_uv, 0.0);
    color = color * opacity;

    return color;
}

/// Sample billboard material data (RMOE)
fn sample_billboard_material(hit: BillboardHit) -> vec4<f32> {
    let cmd = sd_billboard_cmd(hit.billboard_index);
    if (cmd.params.y == DYNAMIC_KIND_BILLBOARD_AVATAR) {
        // Default material for dynamic avatar textures.
        return vec4<f32>(7.0 / 255.0, 15.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0);
    }

    let uv = hit.uv;

    // Get tile frame information
    let frame = sv_tile_frame(hit.tile_index);

    // Map UV to atlas coordinates based on repeat mode
    var atlas_uv: vec2<f32>;

    if (hit.repeat_mode == 1u) {
        // Repeat mode: wrap UVs and map into atlas sub-rect
        let uv_wrapped = fract(uv);
        atlas_uv = frame.ofs + uv_wrapped * frame.scale;

        // Clamp to avoid bleeding from neighboring tiles
        let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0));
        let pad_uv = vec2<f32>(0.5) / atlas_dims;
        let uv_min = frame.ofs + pad_uv;
        let uv_max = frame.ofs + frame.scale - pad_uv;
        atlas_uv = clamp(atlas_uv, uv_min, uv_max);
    } else {
        // Scale mode (default): scale the tile to fit billboard size
        atlas_uv = frame.ofs + uv * frame.scale;
    }

    // Sample material from material atlas
    let material = textureSampleLevel(atlas_mat_tex, atlas_smp, atlas_uv, 0.0);

    return material;
}

/// Trace all billboards and return the closest hit (with optional skip list)
/// skip_mask: bitmask of billboard indices to skip (max 32 billboards can be skipped)
fn trace_billboards_skip(ro: vec3<f32>, rd: vec3<f32>, max_t: f32, skip_mask: u32) -> BillboardHit {
    var closest = BillboardHit(false, max_t, vec2<f32>(0.0), 0u, 0u, 0u, 0u);

    let billboard_count = scene_data.header.billboard_cmd_count;
    if (billboard_count == 0u) {
        return closest;
    }

    for (var i: u32 = 0u; i < billboard_count; i = i + 1u) {
        // Skip if this billboard index is in the skip mask
        if (i < 32u && ((skip_mask >> i) & 1u) != 0u) {
            continue;
        }

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

/// Trace all billboards and return the closest hit
/// This is modular to allow future expansion for different billboard types
fn trace_billboards(ro: vec3<f32>, rd: vec3<f32>, max_t: f32) -> BillboardHit {
    return trace_billboards_skip(ro, rd, max_t, 0u);
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
            0u,
            0u,
            P,
            0.0,
            N,
            0.0,
            albedo,
            mat,
            0u,
            billboard_hit.billboard_index,
            0u,
            0u
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
        var albedo = sv_tri_sample_albedo_blended(i0, i1, i2, geo_hit.u, geo_hit.v);
        albedo = vec4<f32>(pow(albedo.rgb, vec3<f32>(2.2)), albedo.a);

        let mat_data = sv_tri_sample_rmoe_blended(i0, i1, i2, geo_hit.u, geo_hit.v);
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
            0u,
            0u,
            P,
            0.0,
            N,
            0.0,
            albedo,
            mat,
            tri,
            0u,
            0u,
            0u
        );
    }

    // No hit
    return UnifiedHit(
        HIT_TYPE_NONE,
        tmax,
        0u,
        0u,
        vec3<f32>(0.0),
        0.0,
        vec3<f32>(0.0, 1.0, 0.0),
        0.0,
        vec4<f32>(0.0),
        Material(0.0, 0.0, 0.0, 0.0, vec3<f32>(0.0), 0.0),
        0u,
        0u,
        0u,
        0u
    );
}

// ===== Unified Shadow Trace =====
// Single-hit shadow test: only mostly opaque surfaces cast shadows.
fn trace_shadow_unified(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> f32 {
    let hit = trace_unified(ro, rd, 0.0, tmax);
    if (hit.hit_type == HIT_TYPE_NONE) {
        return 0.0;
    }

    let layer_opacity = clamp(hit.albedo.a * hit.material.opacity, 0.0, 1.0);
    // Single-hit transmittance: shadow strength follows opacity continuously.
    return layer_opacity;
}

// ===== Ray-traced shadows =====
fn trace_shadow(P: vec3<f32>, L: vec3<f32>, max_dist: f32) -> f32 {
    let shadow_bias = 0.01;
    let min_light_dist = 0.1;
    if (max_dist <= min_light_dist) {
        return 1.0;
    }
    let opacity = trace_shadow_unified(P + L * shadow_bias, L, max_dist - min_light_dist);
    return 1.0 - opacity;
}

// ===== Ambient Occlusion =====
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

        let hit = trace_unified(P + N * 0.001, world_dir, 0.0, ao_radius);
        if (hit.hit_type != HIT_TYPE_NONE) {
            let opacity = hit.albedo.a * hit.material.opacity;
            if (opacity > 0.5) {
                let dist_factor = clamp(1.0 - (hit.t / ao_radius), 0.0, 1.0);
                occlusion += dist_factor * opacity;
            }
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

fn shade_surface(
    hit: UnifiedHit,
    rd: vec3<f32>,
    px: u32,
    py: u32,
    sky_rgb: vec3<f32>,
    ambient_strength: f32
) -> vec3<f32> {
    let P = hit.position;
    let N = hit.normal;
    let albedo = hit.albedo;
    let mat = hit.material;
    let V = -rd;

    if (mat.emissive > 0.99) {
        return albedo.rgb * 2.0;
    }

    let ao = compute_ao(P, N, P + vec3<f32>(f32(px), f32(py), 0.0));
    let direct = pbr_lighting(P, N, V, albedo.rgb, mat);

    let ambient_color = U.gp3.xyz;
    let sky_factor = max(dot(N, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
    let max_sky_dist = select(50.0, U.gp6.y, U.gp6.y >= 0.0);

    var sky_contribution = vec3<f32>(0.0);
    if (max_sky_dist > 0.0 && sky_factor > 0.0) {
        let sky_dir = reflect(rd, N);
        let sky_dir_up = max(dot(sky_dir, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
        let sky_visibility = select(0.0, trace_shadow(P, sky_dir, max_sky_dist), sky_dir_up > 0.0);
        sky_contribution = sky_rgb * sky_factor * sky_visibility;
    }

    let ambient = (ambient_color * ambient_strength + sky_contribution) * albedo.rgb * ao;
    let emissive = albedo.rgb * mat.emissive * 2.0;
    return direct + ambient + emissive;
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
    let ro = ray.ro;
    let rd = normalize(ray.rd);

    // Sky color for background (already in linear space from CPU)
    let sky_rgb = select(U.background.rgb, U.gp0.xyz, length(U.gp0.xyz) > 0.01);
    let ambient_strength = U.gp3.w; // User-defined, no default
    var final_color = sky_rgb;
    var fog_distance = 0.0;
    var has_surface = false;
    var surface_hit = trace_unified(ro, rd, 0.001, 1e6);
    var trace_origin = ro;
    var traveled = 0.0;

    // Keep rays passing through fully transparent texels (e.g. cutout billboards).
    for (var iter: u32 = 0u; iter < 6u; iter = iter + 1u) {
        let hit = trace_unified(trace_origin, rd, 0.001, 1e6);
        if (hit.hit_type == HIT_TYPE_NONE) {
            break;
        }

        let layer_opacity = hit.albedo.a * hit.material.opacity;
        if (layer_opacity > 0.01) {
            surface_hit = hit;
            fog_distance = traveled + hit.t;
            has_surface = true;
            break;
        }

        trace_origin = hit.position + rd * 0.001;
        traveled += hit.t + 0.001;
    }

    if (has_surface) {
        let front_color = shade_surface(surface_hit, rd, px, py, sky_rgb, ambient_strength);
        let front_opacity = clamp(surface_hit.albedo.a * surface_hit.material.opacity, 0.0, 1.0);

        var background_color = sky_rgb;
        // Only blend a second layer behind transparent billboards.
        // This prevents partially transparent geometry materials from leaking hidden billboards.
        if (surface_hit.hit_type == HIT_TYPE_BILLBOARD && front_opacity < 0.99) {
            var back_has_surface = false;
            var back_hit = surface_hit;
            var back_origin = surface_hit.position + rd * 0.001;

            // Find a single visible surface behind the front transparent layer.
            for (var iter: u32 = 0u; iter < 6u; iter = iter + 1u) {
                let hit = trace_unified(back_origin, rd, 0.001, 1e6);
                if (hit.hit_type == HIT_TYPE_NONE) {
                    break;
                }
                let opacity = hit.albedo.a * hit.material.opacity;
                if (opacity > 0.01) {
                    back_hit = hit;
                    back_has_surface = true;
                    break;
                }
                back_origin = hit.position + rd * 0.001;
            }

            if (back_has_surface) {
                let back_color = shade_surface(back_hit, rd, px, py, sky_rgb, ambient_strength);
                let back_opacity = clamp(back_hit.albedo.a * back_hit.material.opacity, 0.0, 1.0);
                background_color = mix(sky_rgb, back_color, back_opacity);
            }
        }

        final_color = mix(background_color, front_color, front_opacity);
    }

    // Apply fog based on distance traveled
    let fog_density = U.gp4.w;
    if (fog_density > 0.0 && has_surface) {
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

    sv_write(px, py, vec4<f32>(final_color, 1.0));
}
