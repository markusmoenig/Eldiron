@group(0) @binding(0) var original:texture_2d<f32>;
@group(0) @binding(1) var painted:texture_2d<f32>;
@group(0) @binding(2) var coverage:texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) index:u32)->@builtin(position) vec4<f32> {
    let points=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));
    return vec4<f32>(points[index],0.,1.);
}
fn linear(c:vec3<f32>)->vec3<f32>{return select(c/12.92,pow((c+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),c>vec3<f32>(0.04045));}
fn srgb(c:vec3<f32>)->vec3<f32>{return select(c*12.92,1.055*pow(c,vec3<f32>(1./2.4))-0.055,c>vec3<f32>(0.0031308));}
@fragment fn fs(@builtin(position) position:vec4<f32>)->@location(0) vec4<f32> {
    let at=vec2<i32>(position.xy);
    let alpha=select(textureLoad(original,at,0).a,1.,textureLoad(coverage,at,0).a>0.);
    let rgb=textureLoad(painted,at,0).rgb;
    if(alpha>=1.){return vec4<f32>(rgb,1.);}
    if(alpha<=0.){return vec4<f32>(0.,0.,0.,1.);}
    return vec4<f32>(srgb(linear(rgb)*alpha),1.);
}
