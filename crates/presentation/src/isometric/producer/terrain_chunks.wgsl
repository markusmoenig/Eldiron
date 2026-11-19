// A pixel reads the ordered triangle list for its world-space projected bin.
// Bins do not overlap in screen space, even when their triangles cross a seam.
struct TerrainFrame {
    projection: vec4<f32>,
    canvas: vec4<u32>,
    grid_bounds: vec4<i32>,
};
@group(0) @binding(0) var<uniform> frame: TerrainFrame;
@group(0) @binding(1) var<storage, read_write> albedo: array<u32>;
@group(0) @binding(2) var<storage, read_write> surfaces: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read_write> owners: array<u32>;
@group(0) @binding(4) var<storage, read_write> terrain: array<vec2<f32>>;
@group(0) @binding(5) var<storage, read> chunks: array<vec4<u32>>;
@group(0) @binding(6) var<storage, read> materials_a: array<vec4<u32>>;
@group(0) @binding(7) var<storage, read> materials_b: array<vec4<u32>>;

fn surface_projection() -> vec4<f32> { return frame.projection; }
fn surface_rounding_zero() -> u32 { return frame.canvas.w; }

const BANK_BIT: u32 = 0x80000000u;
var<private> material_offsets: vec2<u32>;
fn atlas_word(address: u32) -> vec4<u32> {
    if (address & BANK_BIT) != 0u { return materials_b[address & ~BANK_BIT]; }
    return materials_a[address];
}
fn material_word(which: u32, index: u32) -> vec4<u32> {
    return atlas_word(material_offsets[which] + index);
}
fn material_phase(address: u32) -> u32 {
    let directory = atlas_word(address);
    let phase = frame.canvas.z % directory.x;
    return (address & BANK_BIT) | atlas_word(address + 1u + phase / 4u)[phase % 4u];
}
fn triangle_index(position: u32) -> u32 {
    return chunks[chunks[1].x + position / 4u][position % 4u];
}

@compute @workgroup_size(8, 8)
fn raster(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= frame.canvas.xy) { return; }
    let q = vec2<f32>(id.xy) + 0.5;
    let directory = chunks[0];
    let bin = vec2<i32>(floor((q - frame.projection.xy) / frame.projection.z * 4.0));
    let local = bin - bitcast<vec2<i32>>(directory.xy);
    if any(local < vec2<i32>(0)) || any(vec2<u32>(local) >= directory.zw) { return; }
    let bin_index = u32(local.y) * directory.z + u32(local.x);
    let range = chunks[2u + bin_index / 2u];
    let start = range[(bin_index % 2u) * 2u];
    let count = range[(bin_index % 2u) * 2u + 1u];
    if count == 0u { return; }

    let dst = id.y * frame.canvas.x + id.x;
    var depth = terrain[dst].x;
    var height: f32;
    var packed_surface: vec2<u32>;
    var under = albedo[dst];
    var wrote = false;
    for (var i = start; i < start + count; i += 1u) {
        let at = chunks[1].y + triangle_index(i) * 5u;
        let info = chunks[at + 4u];
        let cell = bitcast<vec2<i32>>(info.xy);
        if any(cell < frame.grid_bounds.xy) || any(cell >= frame.grid_bounds.zw) { continue; }
        let va = bitcast<vec3<f32>>(chunks[at].xyz);
        let vb = bitcast<vec3<f32>>(chunks[at + 1u].xyz);
        let vc = bitcast<vec3<f32>>(chunks[at + 2u].xyz);
        let a = project(va); let b = project(vb); let c = project(vc);
        let area = cross2(b - a, c - a);
        if abs(area) < 0.0000001 || !raster_contains(array<vec2<f32>, 3>(a, b, c), q, area) {
            continue;
        }
        let wa = surface_divide(cross2(b - q, c - q), area);
        let wb = surface_divide(cross2(c - q, a - q), area);
        let wc = surface_fp(surface_fp(1.0 - wa) - wb);
        if min(min(wa, wb), wc) < -0.00001 { continue; }
        let world = surface_fp3(surface_fp3(surface_fp3(va * wa) + surface_fp3(vb * wb)) + surface_fp3(vc * wc));
        let candidate = world.z - world.x + world.y * 0.8164966;
        if candidate <= depth { continue; }

        let sampled = sampling_world(world.xz);
        material_offsets.x = material_phase(info.z);
        let base = sample_material(0u, sampled, true);
        var color = base.color;
        var rme = base.rme;
        if info.w != 0xffffffffu {
            material_offsets.y = material_phase(info.w);
            let blend = bitcast<vec4<f32>>(chunks[at + 3u]);
            let t = fract(sampled);
            let l = blend.x * (1.0 - t.y) + blend.w * t.y;
            let r = blend.y * (1.0 - t.y) + blend.z * t.y;
            let amount = clamp(l * (1.0 - t.x) + r * t.x, 0.0, 1.0);
            let other = sample_material(1u, sampled, true);
            color = mix_color(color, other.color, amount);
            rme = rme * (1.0 - amount) + other.rme * amount;
        }
        let value = bytes(color);
        let alpha = value.a;
        if alpha == 0u { continue; }
        under = word(vec4<u32>((value.rgb * alpha + bytes(under).rgb * (255u - alpha) + 127u) / 255u, 255u));
        let normal = bitcast<vec3<f32>>(vec3<u32>(chunks[at].w, chunks[at + 1u].w, chunks[at + 2u].w));
        packed_surface = packed(normal, world.y, rme);
        depth = candidate;
        height = world.y;
        wrote = true;
    }
    if wrote {
        terrain[dst] = vec2<f32>(depth, height);
        surfaces[dst] = packed_surface;
        owners[dst] = 0u;
        albedo[dst] = under;
    }
}
