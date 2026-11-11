// Preserve the original alpha-weighted byte filter, including integer truncation
// and clamped edges for one-dimensional and odd-sized levels.
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var destination: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(destination);
    if (any(id.xy >= size)) { return; }
    let last = textureDimensions(source) - vec2<u32>(1);
    let p = min(id.xy * 2u, last);
    let q = min(p + vec2<u32>(1), last);
    let a = vec4<u32>(round(textureLoad(source, vec2<i32>(p), 0) * 255.0));
    let b = vec4<u32>(round(textureLoad(source, vec2<i32>(vec2<u32>(q.x, p.y)), 0) * 255.0));
    let c = vec4<u32>(round(textureLoad(source, vec2<i32>(vec2<u32>(p.x, q.y)), 0) * 255.0));
    let d = vec4<u32>(round(textureLoad(source, vec2<i32>(q), 0) * 255.0));
    let alpha = a.w + b.w + c.w + d.w;
    var rgb = vec3<u32>(0);
    if (alpha > 0u) {
        rgb = (a.xyz * a.w + b.xyz * b.w + c.xyz * c.w + d.xyz * d.w) / alpha;
    }
    textureStore(destination, vec2<i32>(id.xy), vec4<f32>(vec4<u32>(rgb, alpha / 4u)) / 255.0);
}
