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
// - gp6.w:   Unused

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
    let min_roughness = select(0.04, U.gp6.x, U.gp6.x > 0.0);
    let roughness = max(f32(mat_bits & 0xFu) / 15.0, min_roughness);
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

// ===== Billboard Support =====
// Modular billboard system for dynamic objects (particles, sprites, effects, etc.)

struct BillboardHit {
    hit: bool,              // offset 0, size 4 (stored as u32 in SPIR-V)
    t: f32,                 // offset 4, size 4
    uv: vec2<f32>,          // offset 8, size 8 (needs 8-byte alignment)
    tile_index: u32,        // offset 16, size 4
    billboard_index: u32,   // offset 20, size 4
    _pad0: u32,             // offset 24, size 4 (AMD alignment fix)
    _pad1: u32,             // offset 28, size 4 (pad to 16-byte boundary)
};

/// Ray-billboard intersection test
/// Returns hit information if ray intersects the billboard quad
fn intersect_billboard(ro: vec3<f32>, rd: vec3<f32>, center: vec3<f32>,
                       axis_right: vec3<f32>, axis_up: vec3<f32>,
                       tile_index: u32, billboard_idx: u32) -> BillboardHit {
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
    let uv = vec2<f32>(0.5 * (u + 1.0), 0.5 * (1.0 - v));

    result.hit = true;
    result.t = t;
    result.uv = uv;
    result.tile_index = tile_index;
    result.billboard_index = billboard_idx;

    return result;
}

/// Sample billboard color from atlas
/// This is separated to allow future procedural effects (fire, smoke, etc.)
fn sample_billboard(hit: BillboardHit) -> vec4<f32> {
    // Get tile frame information
    let frame = sv_tile_frame(hit.tile_index);

    // Map UV to atlas coordinates
    let atlas_uv = frame.ofs + hit.uv * frame.scale;

    // Sample from atlas
    let color = textureSampleLevel(atlas_tex, atlas_smp, atlas_uv, 0.0);

    // TODO: Future expansion point for procedural effects
    // if (billboard_type == FIRE) { return apply_fire_effect(color, hit); }
    // if (billboard_type == SMOKE) { return apply_smoke_effect(color, hit); }

    return color;
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
        let axis_right = cmd.axis_right.xyz;
        let axis_up = cmd.axis_up.xyz;
        let tile_index = cmd.params.x;

        // Test intersection
        let hit = intersect_billboard(ro, rd, center, axis_right, axis_up, tile_index, i);

        if (hit.hit && hit.t < closest.t) {
            closest = hit;
        }
    }

    return closest;
}

// ===== Ray-traced shadows with opacity support =====
fn trace_shadow(P: vec3<f32>, L: vec3<f32>, max_dist: f32) -> f32 {
    let shadow_bias = 0.01; // Increased from 0.001 to avoid edge artifacts
    let max_shadow_steps = u32(select(0.0, U.gp6.z, U.gp6.z >= 0.0));

    // Minimum distance to light to avoid self-shadowing when light is near/in geometry
    let min_light_dist = 0.1;

    // Fast path: Binary shadow test (no transparency support)
    // This is much faster for scenes without transparent objects
    if (max_shadow_steps == 0u) {
        let hit = sv_trace_grid(P + L * shadow_bias, L, 0.0, max_dist);
        // Only shadow if hit is not too close to the light (avoid geometry at light position)
        if (hit.hit && hit.t < (max_dist - min_light_dist)) {
            return 0.0; // shadowed
        }
        return 1.0; // lit
    }

    // Slow path: Multi-step transparency-aware shadows
    var current_pos = P + L * shadow_bias;
    var remaining_dist = max_dist;
    var transparency = 1.0; // Starts fully lit

    for (var step: u32 = 0u; step < max_shadow_steps; step = step + 1u) {
        let hit = sv_trace_grid(current_pos, L, 0.0, remaining_dist);

        if (!hit.hit) {
            break; // No more occlusion, light reaches
        }

        // Skip geometry very close to light position (light might be embedded in wall)
        if (remaining_dist - hit.t < min_light_dist) {
            break; // Close enough to light, don't shadow
        }

        // Get material at hit point
        let tri = hit.tri;
        let i0 = indices3d.data[3u * tri + 0u];
        let i1 = indices3d.data[3u * tri + 1u];
        let i2 = indices3d.data[3u * tri + 2u];

        // Sample material to get opacity
        let mat_data = sv_tri_sample_rmoe(i0, i1, i2, hit.u, hit.v);
        let mat = unpack_material(mat_data);

        // Accumulate transparency (opacity reduces light transmission)
        transparency *= (1.0 - mat.opacity);

        // Early exit if fully occluded
        if (transparency < 0.01) {
            return 0.0;
        }

        // Continue ray from just past this hit
        current_pos = current_pos + L * (hit.t + shadow_bias);
        remaining_dist = remaining_dist - hit.t - shadow_bias;

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

        let ao_hit = sv_trace_grid(P + N * 0.001, world_dir, 0.0, ao_radius);
        if (ao_hit.hit) {
            // Get material at hit point to check opacity
            let tri = ao_hit.tri;
            let i0 = indices3d.data[3u * tri + 0u];
            let i1 = indices3d.data[3u * tri + 1u];
            let i2 = indices3d.data[3u * tri + 2u];

            let mat_data = sv_tri_sample_rmoe(i0, i1, i2, ao_hit.u, ao_hit.v);
            let mat = unpack_material(mat_data);

            // Weight by distance - closer occluders contribute more
            let dist_factor = 1.0 - (ao_hit.t / ao_radius);

            // Modulate occlusion by opacity (transparent objects occlude less)
            occlusion += dist_factor * mat.opacity;
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

// ===== Light Sampling Helper =====
// Sample a random point light and return (position, color * intensity, pdf)
fn sample_point_light(rand: f32) -> vec3<f32> {
    if (U.lights_count == 0u) {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    // Uniform light selection
    let light_idx = min(u32(rand * f32(U.lights_count)), U.lights_count - 1u);
    let light = sd_light(light_idx);

    return light.position.xyz;
}

// ===== Main Compute Shader with Next Event Estimation =====
@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x;
    let py = gid.y;
    if (px >= U.fb_size.x || py >= U.fb_size.y) { return; }

    // Build camera ray with per-frame subpixel jitter for AA
    let res = vec2<f32>(f32(U.fb_size.x), f32(U.fb_size.y));
    // Jitter in range [-0.5, +0.5] pixels for proper anti-aliasing
    let jitter = (hash33(vec3<f32>(f32(px), f32(py), f32(U.anim_counter))).xy - vec2<f32>(0.5));
    let cam_uv = (vec2<f32>(f32(px), f32(py)) + vec2<f32>(0.5) + jitter) / res;
    let ray = cam_ray(cam_uv);
    var ro = ray.ro;
    let rd = normalize(ray.rd);

    // Path tracer with Next Event Estimation (direct light sampling)
    let sky_rgb = select(U.background.rgb, U.gp0.xyz, length(U.gp0.xyz) > 0.01);
    let max_bounces: u32 = 4u;
    var radiance = vec3<f32>(0.0);
    var throughput = vec3<f32>(1.0);
    var dir = rd;

    for (var bounce: u32 = 0u; bounce < max_bounces; bounce = bounce + 1u) {
        let tmin = select(0.0, 0.001, bounce == 0u);
        let hit = sv_trace_grid(ro, dir, tmin, 1e6);

        if (!hit.hit) {
            radiance += throughput * sky_rgb;
            break;
        }

        let tri = hit.tri;
        let i0 = indices3d.data[3u * tri + 0u];
        let i1 = indices3d.data[3u * tri + 1u];
        let i2 = indices3d.data[3u * tri + 2u];

        let v0 = verts3d.data[i0];
        let v1 = verts3d.data[i1];
        let v2 = verts3d.data[i2];

        let w = 1.0 - hit.u - hit.v;
        let n0 = vec3<f32>(v0.normal[0], v0.normal[1], v0.normal[2]);
        let n1 = vec3<f32>(v1.normal[0], v1.normal[1], v1.normal[2]);
        let n2 = vec3<f32>(v2.normal[0], v2.normal[1], v2.normal[2]);
        var N = normalize(w * n0 + hit.u * n1 + hit.v * n2);

        let p0 = vec3<f32>(v0.pos[0], v0.pos[1], v0.pos[2]);
        let p1 = vec3<f32>(v1.pos[0], v1.pos[1], v1.pos[2]);
        let p2 = vec3<f32>(v2.pos[0], v2.pos[1], v2.pos[2]);

        // Sample textures using header helpers
        let albedo_rgba = sv_tri_sample_albedo(i0, i1, i2, hit.u, hit.v);
        let mats_rgba = sv_tri_sample_rmoe(i0, i1, i2, hit.u, hit.v);
        let mat = unpack_material(mats_rgba);

        let albedo = pow(albedo_rgba.rgb, vec3<f32>(2.2));
        let opacity = albedo_rgba.a * mat.opacity;

        // Normal map perturbation using header TBN
        let nrm_map = vec3<f32>(mat.normal.xy, mat.normal.z);
        if (length(nrm_map.xy) > 1e-5) {
            let TBN = sv_tri_tbn(
                vec3<f32>(v0.pos[0], v0.pos[1], v0.pos[2]),
                vec3<f32>(v1.pos[0], v1.pos[1], v1.pos[2]),
                vec3<f32>(v2.pos[0], v2.pos[1], v2.pos[2]),
                vec2<f32>(v0.uv[0], v0.uv[1]),
                vec2<f32>(v1.uv[0], v1.uv[1]),
                vec2<f32>(v2.uv[0], v2.uv[1])
            );
            let mapped = normalize(TBN * nrm_map);
            N = normalize(mix(N, mapped, clamp(U.gp5.z, 0.0, 1.0)));
        }

        let hit_pos = ro + dir * hit.t;
        let V = -dir; // View direction

        // Emissive contribution (direct hit on light)
        if (mat.emissive > 0.0) {
            radiance += throughput * albedo * mat.emissive;
        }

        // Handle transparency: skip shading
        if (opacity < 0.999) {
            ro = hit_pos + dir * 0.001;
            continue;
        }

        // ===== Next Event Estimation: Direct light sampling =====
        // Sample ALL lights (simple but correct for small light counts)
        for (var li: u32 = 0u; li < U.lights_count; li = li + 1u) {
            let light = sd_light(li);
            let light_pos = light.position.xyz;
            let light_color = light.color.xyz;
            let light_intensity = light.params0.x * light.params1.x;

            let to_light = light_pos - hit_pos;
            let light_dist = length(to_light);
            let L = normalize(to_light);
            // Two-sided lighting: take absolute value for Cornell box with inverted normals
            let NdotL = abs(dot(N, L));

            if (NdotL > 0.0) {
                // Check visibility to light
                let shadow = trace_shadow(hit_pos, L, light_dist);

                if (shadow > 0.01) {
                    // Distance attenuation
                    let dist2 = max(light_dist * light_dist, 1e-6);
                    let attenuation = light_intensity / dist2;

                    // Cook-Torrance BRDF evaluation
                    let F0 = mix(vec3<f32>(0.04), albedo, mat.metallic);
                    let H = normalize(V + L);

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

                    let brdf = kD * albedo / PI + specular;
                    let incoming_light = light_color * attenuation * shadow;

                    // Add direct lighting contribution
                    radiance += throughput * brdf * incoming_light * NdotL;
                }
            }
        }

        // ===== Indirect lighting: Diffuse bounce =====
        // Use cosine-weighted hemisphere sampling for diffuse surfaces
        let xi = hash33(vec3<f32>(f32(px + bounce * 3u), f32(py), f32(U.anim_counter)));
        let onb = build_onb(N);
        let hemi = cosine_sample_hemisphere(xi.x, xi.y);
        let new_dir = normalize(onb * hemi);

        // Update throughput with diffuse BRDF (cosine term cancels with PDF)
        let F0 = mix(vec3<f32>(0.04), albedo, mat.metallic);
        let kD = (vec3<f32>(1.0) - F0) * (1.0 - mat.metallic);
        throughput *= kD * albedo;

        ro = hit_pos + new_dir * 0.001;
        dir = new_dir;

        // Russian roulette after a few bounces
        if (bounce >= 2u) {
            let p = clamp(max(throughput.x, max(throughput.y, throughput.z)), 0.05, 0.95);
            let rr = hash13(vec3<f32>(f32(px * 7u), f32(py * 11u), f32(bounce)));
            if (rr > p) { break; }
            throughput /= p;
        }
    }

    let final_color = radiance;

    // Accumulate over frames using ping-pong targets; fall back to direct write on first frame.
    // Read previous frame as linear (compute target stores raw linear values).
    let prev = textureSampleLevel(prev_layer, atlas_smp, cam_uv, 0.0).rgb;
    // Treat anim_counter as sample count (starting at 0 on first frame).
    let sample_idx = max(f32(U.anim_counter) + 1.0, 1.0);
    var accum_lin = final_color;
    if (sample_idx > 1.0) {
        accum_lin = prev + (final_color - prev) / sample_idx;
    }

    // Store linear; compositing with AlphaLinear will handle sRGB encoding for display.
    sv_write(px, py, vec4<f32>(accum_lin, 1.0));
}
