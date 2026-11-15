@group(0) @binding(0) var source:texture_2d<f32>;
@group(0) @binding(1) var output:texture_storage_2d<rgba8unorm,write>;
@group(0) @binding(2) var<uniform> params:vec4<u32>;
fn srgb(c:vec3<f32>)->vec3<f32> {
    return select(1.055*pow(max(c,vec3<f32>(0.)),vec3<f32>(1./2.4))-0.055,c*12.92,c<=vec3<f32>(0.0031308));
}
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    if(any(id.xy>=textureDimensions(output))){return;}
    var color=vec4<f32>(30.,32.,35.,255.)/255.;
    let p=vec2<i32>(id.xy)-vec2<i32>(params.yz);
    if(all(p>=vec2<i32>(0)) && all(p<vec2<i32>(i32(params.x)))) {
        let dimensions=textureDimensions(source);
        let scale=vec2<f32>(f32(params.x))/vec2<f32>(dimensions);
        // CPU round() is away from zero at halves (all coordinates here >=0).
        let xy=vec2<u32>(floor(vec2<f32>(p)/scale+vec2<f32>(0.5)));
        let index=xy.y*dimensions.x+xy.x;
        color=vec4<f32>(0.);
        if(index<dimensions.x*dimensions.y) {
            color=textureLoad(source,vec2<i32>(i32(index%dimensions.x),i32(index/dimensions.x)),0);
            if(params.w!=0u){color=vec4<f32>(srgb(color.rgb),color.a);}
        }
    }
    textureStore(output,vec2<i32>(id.xy),color);
}
