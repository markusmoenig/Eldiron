// Include with read-only material_a/material_b arrays of vec4<u32>. Their
// immutable format is owned by material_sources.rs; no viewport bytes enter it.
struct MaterialSample { color:u32, rme:vec3<f32> };
fn material_word(which:u32,index:u32)->vec4<u32> {
    if which==0u {return material_a[index];}
    return material_b[index];
}
fn sample_material(which:u32,coordinates:vec2<f32>,repeat:bool)->MaterialSample {
    let head=material_word(which,0u);let origin=material_word(which,1u);
    var uv=coordinates;var page=0u;
    if repeat && (head.w&1u)!=0u {
        if (head.w&2u)!=0u {
            uv=fract(uv/bitcast<vec2<f32>>(material_word(which,2u).xy));
        } else {
            let grid=uv/f32(origin.z);
            let cell=vec2<u32>(clamp(floor(grid)-vec2<f32>(bitcast<vec2<i32>>(origin.xy)),vec2<f32>(0.0),vec2<f32>(head.yz-1u)));
            page=cell.y*head.y+cell.x;uv=fract(grid);
        }
    }
    if repeat {uv=fract(uv);} else {uv=clamp(uv,vec2<f32>(0.0),vec2<f32>(1.0));}
    let image=material_word(which,3u+page);
    if image.y==0u || image.z==0u {return MaterialSample(0u,vec3<f32>(0.95,0.0,0.0));}
    let pixel=vec2<u32>(floor(uv*vec2<f32>(image.yz-1u)+0.5));
    let texel=material_word(which,image.x+pixel.y*image.y+pixel.x);
    return MaterialSample(texel.x,bitcast<vec3<f32>>(texel.yzw));
}
