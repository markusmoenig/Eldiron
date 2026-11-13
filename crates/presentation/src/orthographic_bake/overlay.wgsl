struct Params {size:vec4<u32>,camera:vec4<f32>,forward:vec4<f32>,background:vec4<f32>,bits:vec4<u32>};
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var bake:texture_2d<f32>;
@group(0) @binding(2) var<storage,read> depth:array<f32>;
@group(0) @binding(3) var overlay:texture_2d<f32>;
@group(0) @binding(4) var world:texture_2d<f32>;
@group(0) @binding(5) var metadata:texture_2d<f32>;
@group(0) @binding(6) var output:texture_storage_2d<rgba8unorm,write>;
fn fp(x:f32)->f32 {return bitcast<f32>(bitcast<u32>(x)^p.bits.y);}
fn rounded(bytes:vec4<f32>)->vec4<f32> {return floor(clamp(bytes,vec4<f32>(0.),vec4<f32>(255.))+vec4<f32>(0.5));}
@compute @workgroup_size(8,8)
fn compose(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=p.size.xy) {return;}
    let at=vec2<i32>(id.xy);
    var dst=rounded(textureLoad(bake,at,0)*255.);
    let src=rounded(textureLoad(overlay,at,0)*255.);
    var visible=true;
    if p.size.z!=0u && textureLoad(metadata,at,0).a>0.5 {
        let pos=textureLoad(world,at,0).xyz-p.camera.xyz;
        let linear=fp(fp(fp(pos.x*p.forward.x)+fp(pos.y*p.forward.y))+fp(pos.z*p.forward.z));
        let baked=depth[id.y*p.size.x+id.x];
        if abs(baked)<bitcast<f32>(p.bits.x) && linear>baked+0.02 {visible=false;}
    }
    if src.a>0. && visible {dst=rounded(src+dst*(1.-src.a/255.));}
    let rgb=rounded(vec4<f32>(dst.rgb+p.background.rgb*(1.-dst.a/255.),255.));
    textureStore(output,id.xy,rgb/255.);
}
