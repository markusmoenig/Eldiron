struct Params {
    canvas:vec4<u32>, // width,height,unused,unused
    screen:vec4<i32>, // rounded x0,y0,x1,y1
    image:vec4<u32>, // width,height,bitmap flags,has source geometry
    scale:vec4<f32>, // screen scale,base height,world scale,opacity
    contact:vec4<u32>, // owner,protection offset,length,unused
    bitmap:vec4<f32>, // anchor y,pixels/unit,unused,unused
};
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var<storage,read_write> albedo:array<u32>;
@group(0) @binding(2) var<storage,read_write> surfaces:array<vec2<u32>>;
@group(0) @binding(3) var<storage,read_write> owners:array<u32>;
@group(0) @binding(4) var<storage,read> protection:array<u32>;
@group(1) @binding(0) var<storage,read> source:array<u32>;
@group(1) @binding(1) var<storage,read> geometry:array<vec2<u32>>;
@compute @workgroup_size(8,8)
fn clear(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=p.canvas.xy) {return;}
    let i=id.y*p.canvas.x+id.x;
    albedo[i]=p.canvas.z;surfaces[i]=vec2<u32>(0u,0x7f800000u);owners[i]=0u;
}
fn bytes(v:u32)->vec4<u32> { return vec4<u32>(v&255u,(v>>8u)&255u,(v>>16u)&255u,v>>24u); }
fn word(v:vec4<u32>)->u32 { return v.x|(v.y<<8u)|(v.z<<16u)|(v.w<<24u); }
@compute @workgroup_size(8,8)
fn blit(@builtin(global_invocation_id) id:vec3<u32>) {
    let at=vec2<i32>(id.xy)+max(p.screen.xy,vec2<i32>(0));
    if any(at>=min(p.screen.zw,vec2<i32>(p.canvas.xy))) {return;}
    let texel=min(vec2<u32>(vec2<f32>(at-p.screen.xy)/p.scale.x),p.image.xy-1u);
    let src=texel.y*p.image.x+texel.x;
    let color=bytes(source[src]);let alpha=u32(floor(f32(color.a)*p.scale.w+0.5));
    if alpha==0u {return;}
    let dst=u32(at.y)*p.canvas.x+u32(at.x);
    let previous_owner=owners[dst];
    if previous_owner<p.contact.z && protection[p.contact.y+previous_owner]!=0u {return;}
    owners[dst]=p.contact.x;
    if alpha==255u { albedo[dst]=source[src]; }
    else {
        let under=bytes(albedo[dst]);
        albedo[dst]=word(vec4<u32>((color.rgb*alpha+under.rgb*(255u-alpha)+127u)/255u,255u));
    }
    var height=p.scale.y;
    var packed=128u|(255u<<8u)|(14u<<16u); // same upright normal/RME fallback
    var mode=0u;
    if (p.image.z&1u)!=0u {
        if (p.image.z&4u)==0u { height+=(p.bitmap.x-f32(texel.y)-0.5)/p.bitmap.y*p.scale.z*0.61237246; }
        mode=select(2u,1u,(p.image.z&2u)!=0u);
    } else if p.image.w!=0u {
        let g=geometry[src];
        packed=(g.x&65535u)|(g.y&0xffff0000u);
        let source_height=bitcast<f32>((g.x>>16u)|((g.y&65535u)<<16u));
        height+=source_height*p.scale.z;
    }
    surfaces[dst]=vec2<u32>(packed,(bitcast<u32>(height)&0xffffff00u)|mode);
}
