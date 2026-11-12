struct Params { viewport:vec4<f32>, origin_scale:vec4<f32>, dark:vec4<u32>, light:vec4<u32> };
@group(0) @binding(0) var<uniform> params:Params;
@group(0) @binding(1) var source:texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) index:u32)->@builtin(position) vec4<f32> {
    let positions=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));
    return vec4<f32>(positions[index],0.,1.);
}
@fragment fn fs(@builtin(position) position:vec4<f32>)->@location(0) vec4<f32> {
    // Match sampling the logical reference viewport at any physical density.
    let p=floor(position.xy*params.viewport.xy/params.viewport.zw);
    let checker=(u32(p.x)/12u+u32(p.y)/12u)%2u;
    var color=select(params.dark,params.light,checker==1u);
    let s=(p-params.origin_scale.xy)/params.origin_scale.z;
    let size=textureDimensions(source);
    if(all(s>=vec2<f32>(0.)) && all(s<vec2<f32>(size))) {
        let tex=vec4<u32>(round(textureLoad(source,vec2<i32>(s),0)*255.));
        color=vec4<u32>((tex.rgb*tex.a+color.rgb*(255u-tex.a)+vec3<u32>(127u))/255u,color.a);
    }
    // This unorm attachment intentionally contains encoded source bytes.
    // Its image descriptor tags sRGB for the common compositor to decode once.
    return vec4<f32>(color)/255.;
}
