struct View {right:vec4<f32>,up:vec4<f32>,forward:vec4<f32>,origin:vec4<f32>,projection:vec4<f32>,settings:vec4<u32>};
struct Vertex {position:vec4<f32>,uv_alpha:vec4<f32>};
@group(0) @binding(0) var<uniform> camera:View;
@group(0) @binding(1) var<storage,read> vertices:array<Vertex>;
@group(0) @binding(2) var<storage,read> alpha:array<u32>;
struct Output {
    @builtin(position) position:vec4<f32>,
    @location(0) uv:vec2<f32>,
    @location(1) @interpolate(flat) mask:vec2<u32>,
    @location(2) @interpolate(flat) emissive:f32,
};
@vertex fn vertex(@builtin(vertex_index) index:u32)->Output {
    let v=vertices[index]; let p=v.position.xyz-camera.origin.xyz;
    let z=dot(p,camera.forward.xyz); let near=camera.projection.z; let far=camera.projection.w;
    var out:Output;
    let xy=vec2<f32>(dot(p,camera.right.xyz),dot(p,camera.up.xyz))/camera.projection.xy;
    if camera.settings.x==1u {
        out.position=vec4<f32>(xy,(far*z-near*far)/(far-near),z);
    } else { out.position=vec4<f32>(xy,(z-near)/(far-near),1.0); }
    out.uv=v.uv_alpha.xy; out.mask=bitcast<vec2<u32>>(v.uv_alpha.zw); out.emissive=v.position.w;
    return out;
}
@fragment fn fragment(in:Output) {
    if camera.settings.x==1u && in.emissive>0.5 {discard;}
    let width=in.mask.y&65535u; let height=in.mask.y>>16u;
    if width>0u && height>0u {
        let uv=fract(in.uv); let xy=min(vec2<u32>(uv*vec2<f32>(f32(width),f32(height))),vec2<u32>(width-1u,height-1u));
        let pixel=xy.y*width+xy.x; let bits=alpha[in.mask.x+pixel/32u];
        if (bits&(1u<<(pixel%32u)))==0u {discard;}
    }
}
