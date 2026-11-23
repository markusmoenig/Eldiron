struct Params {
    canvas:vec4<u32>, // width,height,background,empty terrain depth bits
    screen:vec4<i32>, // rounded x0,y0,x1,y1
    image:vec4<u32>, // width,height,bitmap flags,geometry & AO flags
    scale:vec4<f32>, // screen scale,base height,world scale,opacity
    contact:vec4<u32>, // owner,protection offset,length,unused
    bitmap:vec4<f32>, // anchor y,pixels/unit,unused,unused
    foot:vec4<f32>, // foot screen Y,world height,world depth,projection unit
    policy:vec4<u32>, // terrain clip/flat surface flags
};
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var<storage,read_write> albedo:array<u32>;
@group(0) @binding(2) var<storage,read_write> surfaces:array<vec2<u32>>;
@group(0) @binding(3) var<storage,read_write> owners:array<u32>;
@group(0) @binding(4) var<storage,read> protection:array<u32>;
@group(0) @binding(5) var<storage,read_write> terrain:array<vec2<f32>>;
@group(1) @binding(0) var<storage,read> source:array<u32>;
@group(1) @binding(1) var<storage,read> geometry:array<vec2<u32>>;
// The high bit marks an opaque mesh pixel. The lower bits retain the
// supporting-floor owner index (including for mesh pixels).
const MESH_OWNER_BIT:u32 = 0x80000000u;
@compute @workgroup_size(8,8)
fn clear(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=p.canvas.xy) {return;}
    let i=id.y*p.canvas.x+id.x;
    // Nonfinite values are permitted at runtime, but not as WGSL constants.
    albedo[i]=p.canvas.z;surfaces[i]=vec2<u32>(0u,0x7f800000u);owners[i]=0u;terrain[i]=vec2<f32>(bitcast<f32>(p.canvas.w),0.0);
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
    var height=p.scale.y;
    var packed=128u|(255u<<8u)|(14u<<16u); // same upright normal/RME fallback
    var mode=0u;
    var occlusion=0u;
    var has_surface_height=false;
    if (p.policy.x&2u)==0u && (p.image.z&1u)!=0u {
        if (p.image.z&4u)==0u { height+=(p.bitmap.x-f32(texel.y)-0.5)/p.bitmap.y*p.scale.z*0.61237246; }
        mode=select(2u,1u,(p.image.z&2u)!=0u);
    } else if (p.policy.x&2u)==0u && (p.image.w&1u)!=0u {
        let g=geometry[src];
        packed=(g.x&65535u)|(g.y&0xffff0000u);
        var source_height_bits=(g.x>>16u)|((g.y&65535u)<<16u);
        if (p.image.w&2u)!=0u {
            occlusion=source_height_bits&63u;
            source_height_bits=source_height_bits&0xffffffc0u;
        }
        let source_height=bitcast<f32>(source_height_bits);
        height+=source_height*p.scale.z;
        has_surface_height=(bitcast<u32>(source_height)&0x7f800000u)!=0x7f800000u;
    }
    if (p.policy.x&1u)!=0u {
        let ground=terrain[dst];
        if has_surface_height {
            // Along the same ortho ray, the higher world point is closer.
            if bitcast<u32>(ground.x)!=p.canvas.w && ground.y>height+0.06 {return;}
        } else if ground.y>p.foot.y+0.06 && ground.x>p.foot.z+(p.foot.x-f32(at.y)-0.5)/p.foot.w*0.6666667+0.02 {
            return;
        }
    }
    let previous=owners[dst];
    let previous_owner=previous&~MESH_OWNER_BIT;
    if previous_owner<p.contact.z && protection[p.contact.y+previous_owner]!=0u {return;}
    let height_bits=bitcast<u32>(height)&0xffffff00u;
    let opaque_mesh=has_surface_height && alpha==255u;
    if opaque_mesh && (previous&MESH_OWNER_BIT)!=0u {
        // Two opaque samples at the same screen pixel lie on the same
        // orthographic ray. The higher world point is closer to the camera.
        let previous_height=bitcast<f32>(surfaces[dst].y&0xffffff00u);
        if bitcast<f32>(height_bits)<previous_height {return;}
    }
    owners[dst]=p.contact.x|select(0u,MESH_OWNER_BIT,opaque_mesh);
    if alpha==255u { albedo[dst]=source[src]; }
    else {
        let under=bytes(albedo[dst]);
        albedo[dst]=word(vec4<u32>((color.rgb*alpha+under.rgb*(255u-alpha)+127u)/255u,255u));
    }
    surfaces[dst]=vec2<u32>(packed,height_bits|(occlusion<<2u)|mode);
}
