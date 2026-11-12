struct SurfaceParams {
    projection:vec4<f32>, canvas:vec4<u32>, region:vec4<i32>,
    vertices:array<vec4<f32>,3>, normal:vec4<f32>, uv_bounds:vec4<f32>,
    blend:vec4<f32>, contact:vec4<u32>, cell:vec4<i32>,
};
@group(0) @binding(0) var<uniform> p:SurfaceParams;
@group(0) @binding(1) var<storage,read_write> albedo:array<u32>;
@group(0) @binding(2) var<storage,read_write> surfaces:array<vec2<u32>>;
@group(0) @binding(3) var<storage,read_write> owners:array<u32>;
@group(0) @binding(4) var<storage,read_write> terrain:array<vec2<f32>>;
@group(0) @binding(5) var<storage,read> protection:array<u32>;
@group(1) @binding(0) var<storage,read> material_a:array<vec4<u32>>;
@group(1) @binding(1) var<storage,read> material_b:array<vec4<u32>>;
fn project(v:vec3<f32>)->vec2<f32> {
    return p.projection.xy+vec2<f32>((v.x+v.z)*p.projection.z,(v.z-v.x)*p.projection.z*0.5-v.y*p.projection.z*1.2247449);
}
fn cross2(a:vec2<f32>,b:vec2<f32>)->f32 {return a.x*b.y-a.y*b.x;}
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
@compute @workgroup_size(8,8)
fn raster(@builtin(global_invocation_id) id:vec3<u32>) {
    let at=vec2<i32>(id.xy)+p.region.xy;if any(at>=p.region.zw){return;}
    let q=vec2<f32>(at)+0.5;
    let a=project(p.vertices[0].xyz);let b=project(p.vertices[1].xyz);let c=project(p.vertices[2].xyz);
    let area=cross2(b-a,c-a);
    var world=vec3<f32>(0.0);
    if p.canvas.z!=0u {
        if abs(area)<0.0000001 {return;}
        if !raster_contains(array<vec2<f32>,3>(a,b,c),q,area) {return;}
        let wa=cross2(b-q,c-q)/area;let wb=cross2(c-q,a-q)/area;let wc=1.0-wa-wb;
        if min(min(wa,wb),wc)< -0.00001{return;}
        world=p.vertices[0].xyz*wa+p.vertices[1].xyz*wb+p.vertices[2].xyz*wc;
    } else {
        if abs(area)<0.0001{return;}
        if !raster_contains(array<vec2<f32>,3>(a,b,c),q,area) {return;}
        let h=p.vertices[0].y;
        let point=(q+vec2<f32>(0.0,h*p.projection.z*1.2247449)-p.projection.xy)/max(p.projection.z,0.001);
        world=vec3<f32>(point.x*0.5-point.y,h,point.x*0.5+point.y);
    }
    if p.cell.z!=0 && any(vec2<i32>(floor(world.xz))!=p.cell.xy) {return;}
    let dst=u32(at.y)*p.canvas.x+u32(at.x);let previous=owners[dst];
    if previous<p.contact.z&&protection[p.contact.y+previous]!=0u{return;}
    let depth=world.z-world.x+world.y*0.8164966;
    if p.canvas.z!=0u&&depth<=terrain[dst].x{return;}
    var uv=world.xz;let repeat=p.canvas.w!=0u;
    if !repeat {uv=(uv-p.uv_bounds.xy)/max(p.uv_bounds.zw-p.uv_bounds.xy,vec2<f32>(0.001));}
    let base=sample_material(0u,uv,repeat);var color=base.color;var rme=base.rme;
    if p.contact.w!=0u {
        let t=fract(world.xz);let l=p.blend.x*(1.0-t.y)+p.blend.w*t.y;let r=p.blend.y*(1.0-t.y)+p.blend.z*t.y;
        let amount=clamp(l*(1.0-t.x)+r*t.x,0.0,1.0);let other=sample_material(1u,uv,repeat);
        color=mix_color(color,other.color,amount);rme=rme*(1.0-amount)+other.rme*amount;
    }
    let value=bytes(color);let alpha=value.a;if alpha==0u{return;}
    let under=bytes(albedo[dst]);albedo[dst]=word(vec4<u32>((value.rgb*alpha+under.rgb*(255u-alpha)+127u)/255u,255u));
    owners[dst]=p.contact.x;
    if p.canvas.z!=0u {terrain[dst]=vec2<f32>(depth,world.y);surfaces[dst]=packed(p.normal.xyz,world.y,rme);}
    else {surfaces[dst]=packed(vec3<f32>(0.0,1.0,0.0),world.y,vec3<f32>(0.95,0.0,0.0));}
}
