struct Params {
    size:vec4<u32>,tile:vec4<i32>,dispatch:vec4<u32>,plane:vec4<f32>,bounds:vec4<f32>,
    source:vec4<f32>,forward:vec4<f32>,sun:vec4<f32>,sun_color:vec4<f32>,bits:vec4<u32>,
};
struct Pixel { color:u32,depth:f32,albedo:u32,normal:u32,material:u32,surface_channels:u32 };
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var<storage,read> pixels:array<Pixel>;
@group(0) @binding(2) var<storage,read_write> depth:array<f32>;
@group(0) @binding(3) var image:texture_storage_2d<rgba8unorm,write>;
fn fp(x:f32)->f32 {return bitcast<f32>(bitcast<u32>(x)^p.bits.y);}
fn divided(a:f32,b:f32)->f32 {
    let q=fp(a/b);let remainder=fma(-q,b,a);
    return fp(q+fp(remainder/b));
}
fn bytes(word:u32)->vec4<f32> {return vec4<f32>(f32(word&255u),f32((word>>8u)&255u),f32((word>>16u)&255u),f32(word>>24u))/255.;}
fn shade(pixel:Pixel)->vec4<f32> {
    let fallback=bytes(pixel.color);let alpha=fallback.a;
    if alpha<=0. {return vec4<f32>(0.);}
    if p.tile.w==0 || pixel.surface_channels==0u || (pixel.albedo>>24u)==0u {return fallback;}
    let encoded=fallback.rgb/alpha;
    let mapped=pow(encoded,vec3<f32>(2.2));
    let indirect=mapped/max(vec3<f32>(1.)-mapped,vec3<f32>(1e-4));
    let albedo=bytes(pixel.albedo).rgb;
    let raw_normal=bytes(pixel.normal).rgb*2.-vec3<f32>(1.);
    var normal=vec3<f32>(0.,1.,0.);
    if dot(raw_normal,raw_normal)>1e-6 {normal=normalize(raw_normal);}
    let material=bytes(pixel.material);
    let roughness=clamp(material.x,0.04,1.);
    let metallic=clamp(material.y,0.,1.);
    let light=-normalize(p.sun.xyz);let view=-normalize(p.forward.xyz);
    let ndotl=max(dot(normal,light),0.);let ndotv=max(dot(normal,view),0.);
    var direct=vec3<f32>(0.);
    if ndotl>0. && ndotv>0. {
        let half_raw=view+light;var half=normal;
        if dot(half_raw,half_raw)>1e-6 {half=normalize(half_raw);}
        let ndoth=max(dot(normal,half),0.);let vdoth=max(dot(view,half),0.);
        let a=roughness*roughness;let a2=a*a;
        let denom=ndoth*ndoth*(a2-1.)+1.;
        let distribution=a2/(3.141592653589793*denom*denom+1e-6);
        let k=(roughness+1.)*(roughness+1.)/8.;
        let geometry_v=ndotv/(ndotv*(1.-k)+k+1e-6);
        let geometry_l=ndotl/(ndotl*(1.-k)+k+1e-6);
        let f0=mix(vec3<f32>(0.04),albedo,vec3<f32>(metallic));
        let fresnel=f0+(vec3<f32>(1.)-f0)*pow(1.-vdoth,5.);
        let specular=fresnel*(distribution*geometry_v*geometry_l/(4.*ndotv*ndotl+1e-6));
        let diffuse=albedo*((1.-metallic)/3.141592653589793);
        direct=(diffuse+specular)*p.sun_color.xyz*(max(p.sun.w,0.)*ndotl);
    }
    let linear=indirect+direct;
    let display=pow(linear/(linear+vec3<f32>(1.)),vec3<f32>(1./2.2))*alpha*255.;
    return vec4<f32>(floor(clamp(display,vec3<f32>(0.),vec3<f32>(255.))+vec3<f32>(0.5))/255.,alpha);
}
@compute @workgroup_size(8,8)
fn clear(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=p.size.xy) {return;}
    textureStore(image,id.xy,vec4<f32>(0.));depth[id.y*p.size.x+id.x]=bitcast<f32>(p.bits.x);
}
@compute @workgroup_size(8,8)
fn draw(@builtin(global_invocation_id) id:vec3<u32>) {
    let at=id.xy+p.dispatch.xy;
    if any(at>=p.dispatch.zw) {return;}
    let nx=divided(fp(fp(f32(at.x)+0.5)*2.),f32(p.size.x));
    let ny=divided(fp(fp(f32(at.y)+0.5)*2.),f32(p.size.y));
    let u=fp(p.plane.x+fp(fp(nx-1.)*p.plane.z));
    let v=fp(p.plane.y+fp(fp(1.-ny)*p.plane.w));
    if p.tile.z!=0 && (u<p.bounds.x || u>p.bounds.y || v<p.bounds.z || v>p.bounds.w) {return;}
    var origin=vec2<f32>(0.);if p.tile.z!=0 {origin=vec2<f32>(p.bounds.x,p.bounds.z);}
    let tile=vec2<i32>(floor(vec2<f32>(divided(fp(u-origin.x),p.source.y),divided(fp(v-origin.y),p.source.y))));
    if any(tile!=p.tile.xy) {return;}
    let local_x=u32(clamp(floor(fp(fp(u-fp(origin.x+fp(f32(tile.x)*p.source.y)))*p.source.x)),0.,f32(p.size.z-1u)));
    let local_y=u32(clamp(floor(fp(fp(fp(origin.y+fp(f32(tile.y+1)*p.source.y))-v)*p.source.x)),0.,f32(p.size.z-1u)));
    let pixel=pixels[p.bits.z+local_y*p.size.z+local_x];
    textureStore(image,at,shade(pixel));
    // Legacy versions intentionally lack depth occlusion. Invalid/no-hit source
    // depths remain infinity, preserving compatibility without a CPU plane.
    if p.size.w!=0u && abs(pixel.depth)<bitcast<f32>(p.bits.x) {
        depth[at.y*p.size.x+at.x]=pixel.depth+p.source.z;
    }
}
