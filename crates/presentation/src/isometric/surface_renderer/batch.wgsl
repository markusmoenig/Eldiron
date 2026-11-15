struct SurfaceBatch { first:u32, masks:u32, count:u32, padding:u32 };
@group(0) @binding(0) var<uniform> batch:SurfaceBatch;
var<private> p:SurfaceParams;
fn surface_words(at:u32)->vec4<u32> {
    return vec4<u32>(protection[at],protection[at+1u],protection[at+2u],protection[at+3u]);
}
fn surface_params(index:u32)->SurfaceParams {
    let at=index*44u;
    return SurfaceParams(
        bitcast<vec4<f32>>(surface_words(at)),surface_words(at+4u),bitcast<vec4<i32>>(surface_words(at+8u)),
        array<vec4<f32>,3>(bitcast<vec4<f32>>(surface_words(at+12u)),bitcast<vec4<f32>>(surface_words(at+16u)),bitcast<vec4<f32>>(surface_words(at+20u))),
        bitcast<vec4<f32>>(surface_words(at+24u)),bitcast<vec4<f32>>(surface_words(at+28u)),bitcast<vec4<f32>>(surface_words(at+32u)),
        surface_words(at+36u),bitcast<vec4<i32>>(surface_words(at+40u))
    );
}
