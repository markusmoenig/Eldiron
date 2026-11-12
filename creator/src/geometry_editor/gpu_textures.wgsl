// Geometry inspection atlas keeps the reference's encoded 8-bit map contract.
// The material's float outputs stay on the GPU; no displayed source is mapped.
@group(0) @binding(0) var base: texture_2d<f32>;
@group(0) @binding(1) var opacity: texture_2d<f32>;
@group(0) @binding(2) var roughness: texture_2d<f32>;
@group(0) @binding(3) var metallic: texture_2d<f32>;
@group(0) @binding(4) var emission: texture_2d<f32>;
@group(0) @binding(5) var colors: texture_storage_2d_array<rgba8unorm, write>;
@group(0) @binding(6) var data: texture_storage_2d_array<rgba8unorm, write>;
@group(0) @binding(7) var glow: texture_storage_2d_array<rgba8unorm, write>;
struct Parameters { size: vec2u, layer: u32, source_size: u32 }
@group(0) @binding(8) var<uniform> p: Parameters;
fn unit(value: f32) -> f32 { return floor(clamp(value,0.0,1.0)*255.0+0.5)/255.0; }
fn srgb(value: f32) -> f32 {
    let v=max(value,0.0);
    return unit(select(1.055*pow(v,1.0/2.4)-0.055,12.92*v,v<=0.0031308));
}
fn encoded(value: vec3f) -> vec3f { return vec3f(srgb(value.r),srgb(value.g),srgb(value.b)); }
@compute @workgroup_size(8,8)
fn initialize(@builtin(global_invocation_id) id: vec3u) {
    if any(id.xy>=p.size) { return; }
    textureStore(colors,vec2i(id.xy),i32(p.layer),vec4f(1.0));
    textureStore(data,vec2i(id.xy),i32(p.layer),vec4f(1.0));
    textureStore(glow,vec2i(id.xy),i32(p.layer),vec4f(0.0));
}
@compute @workgroup_size(8,8)
fn convert(@builtin(global_invocation_id) id: vec3u) {
    if any(id.xy>=p.size) { return; }
    // Match the reference atlas's integer nearest-source resampling.
    let xy=vec2i(id.xy*p.source_size/p.size);
    textureStore(colors,vec2i(id.xy),i32(p.layer),vec4f(encoded(textureLoad(base,xy,0).rgb),unit(textureLoad(opacity,xy,0).r)));
    textureStore(data,vec2i(id.xy),i32(p.layer),vec4f(unit(textureLoad(roughness,xy,0).r),unit(textureLoad(metallic,xy,0).r),0.0,1.0));
    textureStore(glow,vec2i(id.xy),i32(p.layer),vec4f(encoded(textureLoad(emission,xy,0).rgb),1.0));
}
