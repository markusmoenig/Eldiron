struct Params {size:vec4<u32>,source:vec4<u32>,page:vec4<f32>,period:vec4<f32>,world:vec4<f32>,background:vec4<u32>};
struct Triangle {p:array<vec4<f32>,3>,edges:array<vec4<f32>,3>};
struct Occlusion {bounds:vec4<f32>,value:vec4<f32>};
@group(0) @binding(0) var<uniform> u:Params;
@group(0) @binding(1) var<storage,read_write> pixels:array<u32>;
@group(0) @binding(2) var<storage,read> triangles:array<Triangle>;
@group(0) @binding(3) var<storage,read> bins:array<u32>;
@group(0) @binding(4) var<storage,read> occlusions:array<Occlusion>;
@group(0) @binding(5) var sources:texture_2d_array<u32>;
@group(0) @binding(6) var<storage,read> source_sizes:array<vec4<u32>>;
@group(0) @binding(7) var output:texture_storage_2d<rgba8unorm,write>;
// Byte-compatible CPU sampling requires observable f32 rounding between
// operations. The uniform opaque-alpha identity is zero at runtime, but keeps
// the backend compiler from contracting arithmetic across these bit boundaries.
fn fp(x:f32)->f32 {return bitcast<f32>(bitcast<u32>(x)^(u.background.w^255u));}
fn divided(a:f32,b:f32)->f32 {
    let q=fp(a/b);
    let remainder=fma(-q,b,a);
    return fp(q+fp(remainder/b));
}
fn fraction(x:f32)->f32 {return fp(x-floor(x));}
fn nearest(x:f32)->u32 {
    let base=floor(x);
    return u32(base+select(0.,1.,fp(x-base)>=0.5));
}
fn packed(c:vec4<u32>)->u32 {return c.x|(c.y<<8u)|(c.z<<16u)|(c.w<<24u);}
fn unpack(c:u32)->vec4<u32> {return vec4<u32>(c&255u,(c>>8u)&255u,(c>>16u)&255u,c>>24u);}
@compute @workgroup_size(8,8)
fn clear(@builtin(global_invocation_id) id:vec3<u32>) {
    if(any(id.xy>=u.size.xy)){return;}
    pixels[id.y*u.size.x+id.x]=packed(u.background);
}
fn texel(input:vec2<f32>)->vec4<u32> {
    var uv=input;var layer=u.source.y;
    if(u.period.z>0.) {
        if(all(u.period.xy>vec2<f32>(0.))){uv=vec2<f32>(fraction(divided(uv.x,u.period.x)),fraction(divided(uv.y,u.period.y)));}
        else {
            let grid=vec2<f32>(divided(uv.x,u.page.z),divided(uv.y,u.page.z));
            let page=vec2<u32>(clamp(floor(grid)-u.page.xy,vec2<f32>(0.),vec2<f32>(f32(u.source.w)-1.,u.page.w-1.)));
            layer+=page.y*u.source.w+page.x;uv=vec2<f32>(fraction(grid.x),fraction(grid.y));
        }
    }
    if(layer>=u.source.z){return vec4<u32>(0u);}
    uv=vec2<f32>(select(clamp(uv.x,0.,1.),fraction(uv.x),u.source.x==1u||u.source.x==2u),select(clamp(uv.y,0.,1.),fraction(uv.y),u.source.x==1u||u.source.x==3u));
    let size=source_sizes[layer].xy;
    let at=vec2<i32>(min(vec2<u32>(nearest(fp(uv.x*f32(size.x-1u))),nearest(fp(uv.y*f32(size.y-1u)))),size-vec2<u32>(1u)));
    return textureLoad(sources,at,i32(layer),0);
}
@compute @workgroup_size(8,8)
fn draw(@builtin(global_invocation_id) id:vec3<u32>) {
    if(any(id.xy>=u.size.xy)){return;}
    let at=id.y*u.size.x+id.x;var color=unpack(pixels[at]);
    let point=vec2<f32>(id.xy)+0.5;
    let world=(vec2<f32>(id.xy)-u.world.xy)/u.world.zw;
    var occlusion=1.;
    for(var i=0u;i<u32(u.period.w);i++) {
        let r=occlusions[i];
        if(all(world>=r.bounds.xy)&&all(world<=r.bounds.zw)){occlusion=clamp(r.value.x,0.,1.);break;}
    }
    let cell=(id.y/8u)*u.size.z+id.x/8u;
    for(var i=bins[cell];i<bins[cell+1u];i++) {
        let t=triangles[bins[i]];var inside=true;
        for(var edge=0u;edge<3u;edge++) {
            let e=t.edges[edge];let result=fp(fp(fp(e.x*point.x)+fp(e.y*point.y))+e.z);
            let inclusive=e.x>0.||(e.x==0.&&e.y>0.);
            if(result<0.||(result==0.&&!inclusive)){inside=false;break;}
        }
        if(!inside){continue;}
        let a=t.p[0].xy;let b=t.p[1].xy;let c=t.p[2].xy;
        let ac=vec2<f32>(fp(c.x-a.x),fp(c.y-a.y));let ab=vec2<f32>(fp(b.x-a.x),fp(b.y-a.y));
        let ap=vec2<f32>(fp(point.x-a.x),fp(point.y-a.y));
        let pc=vec2<f32>(fp(c.x-point.x),fp(c.y-point.y));let pb=vec2<f32>(fp(b.x-point.x),fp(b.y-point.y));
        let area=fp(fp(ac.x*ab.y)-fp(ac.y*ab.x));if(area==0.){continue;}
        let alpha=divided(fp(fp(pc.x*pb.y)-fp(pc.y*pb.x)),area);
        let beta=divided(fp(fp(ac.x*ap.y)-fp(ac.y*ap.x)),area);
        let gamma=fp(fp(1.-alpha)-beta);
        var uv=vec2<f32>(0.);
        for(var component=0u;component<2u;component++) {
            uv[component]=fp(fp(fp(t.p[0][component+2u]*alpha)+fp(t.p[1][component+2u]*beta))+fp(t.p[2][component+2u]*gamma));
        }
        var src=texel(uv);
        src=vec4<u32>(vec3<u32>(clamp(vec3<f32>(src.rgb)/255.*occlusion*255.,vec3<f32>(0.),vec3<f32>(255.))),src.a);
        if(src.a==0u){continue;}
        if(src.a==255u){color=src;continue;}
        let opacity=f32(src.a)/255.;
        color=vec4<u32>(vec3<u32>(vec3<f32>(src.rgb)*opacity+vec3<f32>(color.rgb)*(1.-opacity)),255u);
    }
    pixels[at]=packed(color);
}
@compute @workgroup_size(8,8)
fn resolve(@builtin(global_invocation_id) id:vec3<u32>) {
    if(any(id.xy>=u.size.xy)){return;}
    textureStore(output,vec2<i32>(id.xy),vec4<f32>(unpack(pixels[id.y*u.size.x+id.x]))/255.);
}
