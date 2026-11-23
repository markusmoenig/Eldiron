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
@group(2) @binding(0) var<storage,read> paint_data:array<u32>;
fn surface_projection()->vec4<f32> { return p.projection; }
fn surface_rounding_zero()->u32 { return bitcast<u32>(p.cell.w); }
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
        let wa=surface_divide(cross2(b-q,c-q),area);let wb=surface_divide(cross2(c-q,a-q),area);let wc=surface_fp(surface_fp(1.0-wa)-wb);
        if min(min(wa,wb),wc)< -0.00001{return;}
        world=surface_fp3(surface_fp3(surface_fp3(p.vertices[0].xyz*wa)+surface_fp3(p.vertices[1].xyz*wb))+surface_fp3(p.vertices[2].xyz*wc));
    } else {
        if abs(area)<0.0001{return;}
        if !raster_contains(array<vec2<f32>,3>(a,b,c),q,area) {return;}
        let h=p.vertices[0].y;
        let lift=surface_fp(surface_fp(h*p.projection.z)*1.2247449);
        let translated=surface_fp2(surface_fp2(q+vec2<f32>(0.0,lift))-p.projection.xy);
        let unit=max(p.projection.z,0.001);
        let point=vec2<f32>(surface_divide(translated.x,unit),surface_divide(translated.y,unit));
        let half_x=surface_fp(point.x*0.5);
        world=vec3<f32>(surface_fp(half_x-point.y),h,surface_fp(half_x+point.y));
    }
    let sampled=sampling_world(world.xz);
    if p.cell.z!=0 && any(vec2<i32>(floor(sampled))!=p.cell.xy) {return;}
    // Sprite blits reserve the high owner bit for opaque mesh depth.
    let dst=u32(at.y)*p.canvas.x+u32(at.x);let previous=owners[dst]&0x7fffffffu;
    if previous<p.contact.z&&protection[p.contact.y+previous]!=0u{return;}
    let depth=world.z-world.x+world.y*0.8164966;
    if p.canvas.z!=0u&&depth<=terrain[dst].x{return;}
    var uv=sampled;let repeat=p.canvas.w!=0u;
    if !repeat {
        let delta=surface_fp2(uv-p.uv_bounds.xy);
        let extent=max(surface_fp2(p.uv_bounds.zw-p.uv_bounds.xy),vec2<f32>(0.001));
        uv=vec2<f32>(surface_divide(delta.x,extent.x),surface_divide(delta.y,extent.y));
    }
    let base=sample_material(0u,uv,repeat);var color=base.color;var rme=base.rme;
    if (p.contact.w&1u)!=0u {
        let t=fract(sampled);let l=p.blend.x*(1.0-t.y)+p.blend.w*t.y;let r=p.blend.y*(1.0-t.y)+p.blend.z*t.y;
        let amount=clamp(l*(1.0-t.x)+r*t.x,0.0,1.0);let other=sample_material(1u,uv,repeat);
        color=mix_color(color,other.color,amount);rme=rme*(1.0-amount)+other.rme*amount;
    }
    // Reference terrain rejects a transparent base before paint; floor paint
    // may create color on one. Erasure retains terrain geometry/depth.
    if p.canvas.z!=0u && bytes(color).a==0u {return;}
    if (p.contact.w&2u)!=0u {let painted=apply_paint(color,rme,sampled);color=painted.color;rme=painted.rme;}
    let value=bytes(color);let alpha=value.a;
    if alpha==0u && p.canvas.z==0u {return;}
    if alpha>0u {
        let under=bytes(albedo[dst]);albedo[dst]=word(vec4<u32>((value.rgb*alpha+under.rgb*(255u-alpha)+127u)/255u,255u));
        owners[dst]=p.contact.x;
    }
    if p.canvas.z!=0u {terrain[dst]=vec2<f32>(depth,world.y);surfaces[dst]=packed(p.normal.xyz,world.y,rme);}
    else {surfaces[dst]=packed(vec3<f32>(0.0,1.0,0.0),world.y,rme);}
}
