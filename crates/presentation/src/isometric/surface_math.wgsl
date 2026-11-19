// cell.w is a runtime zero in every CPU parameter record. As in the DDA
// painter, the opaque identity preserves observable f32 rounding: changing
// uniform versus storage parameter fetch must not change fused operations at
// nearest-texel or triangle ownership boundaries.
fn surface_fp(value:f32)->f32 {return bitcast<f32>(bitcast<u32>(value)^surface_rounding_zero());}
fn surface_fp2(value:vec2<f32>)->vec2<f32> {return bitcast<vec2<f32>>(bitcast<vec2<u32>>(value)^vec2<u32>(surface_rounding_zero()));}
fn surface_fp3(value:vec3<f32>)->vec3<f32> {return bitcast<vec3<f32>>(bitcast<vec3<u32>>(value)^vec3<u32>(surface_rounding_zero()));}
fn surface_divide(a:f32,b:f32)->f32 {
    let q=surface_fp(a/b);
    return surface_fp(q+surface_fp(fma(-q,b,a)/b));
}
fn project(v:vec3<f32>)->vec2<f32> {
    let x=surface_fp(surface_fp(v.x+v.z)*surface_projection().z);
    let y=surface_fp(surface_fp(surface_fp(v.z-v.x)*surface_projection().z)*0.5);
    let height=surface_fp(surface_fp(v.y*surface_projection().z)*1.2247449);
    return surface_fp2(surface_projection().xy+vec2<f32>(x,surface_fp(y-height)));
}
fn cross2(a:vec2<f32>,b:vec2<f32>)->f32 {
    let av=surface_fp2(a);let bv=surface_fp2(b);
    return surface_fp(surface_fp(av.x*bv.y)-surface_fp(av.y*bv.x));
}
fn edge(a:vec2<f32>,b:vec2<f32>,q:vec2<f32>)->f32 {return cross2(b-a,q-a);}
fn bytes(v:u32)->vec4<u32> {return vec4<u32>(v&255u,(v>>8u)&255u,(v>>16u)&255u,v>>24u);}
fn word(v:vec4<u32>)->u32 {return v.x|(v.y<<8u)|(v.z<<16u)|(v.w<<24u);}
fn linear(v:vec3<f32>)->vec3<f32> {return select(pow((v+0.055)/1.055,vec3<f32>(2.4)),v/12.92,v<=vec3<f32>(0.04045));}
fn srgb(v:vec3<f32>)->vec3<f32> {return select(1.055*pow(v,vec3<f32>(1.0/2.4))-0.055,v*12.92,v<=vec3<f32>(0.0031308));}
fn mix_color(a:u32,b:u32,t:f32)->u32 {
    let av=unpack4x8unorm(a);let bv=unpack4x8unorm(b);
    let rgb=srgb(linear(av.rgb)*(1.0-t)+linear(bv.rgb)*t);
    return word(vec4<u32>(floor(clamp(vec4<f32>(rgb,av.a*(1.0-t)+bv.a*t),vec4<f32>(0.0),vec4<f32>(1.0))*255.0+0.5)));
}
fn packed(normal:vec3<f32>,height:f32,rme:vec3<f32>)->vec2<u32> {
    let n=normal/max(abs(normal.x)+abs(normal.y)+abs(normal.z),0.00000001);
    var xy=n.xy;
    if n.z<0.0 {
        let signs=select(vec2<f32>(1.0),vec2<f32>(-1.0),(bitcast<vec2<u32>>(n.xy)&vec2<u32>(0x80000000u))!=vec2<u32>(0u));
        xy=(1.0-abs(n.yx))*signs;
    }
    let oct=vec2<u32>(floor(clamp((xy*0.5+0.5)*255.0,vec2<f32>(0.0),vec2<f32>(255.0))+0.5));
    let material=vec3<u32>(floor(clamp(rme,vec3<f32>(0.0),vec3<f32>(1.0))*vec3<f32>(15.0,15.0,255.0)+0.5));
    return vec2<u32>(oct.x|(oct.y<<8u)|(material.x<<16u)|(material.y<<20u)|(material.z<<24u),bitcast<u32>(height)&0xffffff00u);
}
fn raster_contains(corners:array<vec2<f32>,3>,q:vec2<f32>,area:f32)->bool {
    for(var i=0u;i<3u;i++) {
        var lo=corners[i];var hi=corners[(i+1u)%3u];
        if area<0.0 {let swap=lo;lo=hi;hi=swap;}
        let owns=hi.y<lo.y||(hi.y==lo.y&&hi.x>lo.x);let e=edge(lo,hi,q);
        if e< -0.00001||(abs(e)<=0.00001&&!owns){return false;}
    }
    return true;
}
// Same source-boundary ownership as rusterix::isometric::materials.
fn sampling_world(world:vec2<f32>)->vec2<f32> {
    let boundary=sign(world)*floor(abs(world*64.0)+0.5)/64.0;
    return select(world,boundary,abs(world-boundary)<=vec2<f32>(0.000001));
}
