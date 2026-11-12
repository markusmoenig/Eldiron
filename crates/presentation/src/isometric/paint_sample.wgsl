// paint_data stores an immutable source neighborhood and the CPU material
// library's full-precision R/M/O/E table. It is never a destination canvas.
struct PaintSample {color:u32,material:u32,found:bool};
fn sample_paint(world:vec2<f32>)->PaintSample {
    let n=paint_data[0];if n==0u{return PaintSample(0u,0u,false);}
    let unit=bitcast<f32>(paint_data[2]);let p=world*unit;
    let pixel=vec2<i32>(sign(p)*floor(abs(p)+0.5));let size=i32(paint_data[1]);
    let origin=vec2<i32>(floor(vec2<f32>(pixel)/f32(size)))*size;
    for(var i=0u;i<n;i++) {
        let at=4u+256u*4u+i*4u;let candidate=vec2<i32>(bitcast<i32>(paint_data[at]),bitcast<i32>(paint_data[at+1u]));
        if all(origin==candidate) {
            let local=pixel-origin;let index=paint_data[at+2u]+u32(local.y*size+local.x)*2u;
            return PaintSample(paint_data[index],paint_data[index+1u],true);
        }
    }
    return PaintSample(0u,0u,false);
}
fn library_material(id:u32)->vec3<f32> {
    let at=4u+id*4u;return vec3<f32>(bitcast<f32>(paint_data[at]),bitcast<f32>(paint_data[at+1u]),bitcast<f32>(paint_data[at+3u]));
}
fn apply_paint(color:u32,rme:vec3<f32>,world:vec2<f32>)->MaterialSample {
    let source=sample_paint(world);if !source.found{return MaterialSample(color,rme);}
    let base=bytes(color);let paint=bytes(source.color);let material=bytes(source.material);let marker=material.x;
    let replace=marker==252u||(marker>=128u&&marker<=187u)||(marker==254u&&material.z!=0u);
    let alpha=f32(paint.a)/255.0;let coverage=select(alpha,f32(material.a)/255.0,replace);
    let under=f32(base.a)/255.0*(1.0-coverage);let total=under+alpha;
    var result=0u;
    if total>0.0 {
        let rgb=(vec3<f32>(base.rgb)*under+vec3<f32>(paint.rgb)*alpha)/total;
        result=word(vec4<u32>(vec3<u32>(floor(rgb+0.5)),u32(min(floor(total*255.0+0.5),255.0))));
    }
    var changed=rme;
    let encoded=(marker>=64u&&marker<=123u)||(marker>=128u&&marker<=187u)||marker==252u||marker==253u;
    if marker==254u||encoded {
        var top=vec3<f32>(0.0);
        if marker==254u {top=library_material(material.y);}
        else if material.y==paint_data[3] {top=library_material(material.z);}
        else {let packed=material.y|(material.z<<8u);top=vec3<f32>(f32(packed&15u),f32((packed>>4u)&15u),f32((packed>>12u)&15u))/15.0;}
        let amount=select(f32(material.a)/255.0,1.0,replace&&material.a>0u);
        changed=rme*(1.0-amount)+top*amount;
    }
    return MaterialSample(result,changed);
}
