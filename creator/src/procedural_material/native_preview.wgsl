struct Params {viewport:vec4<f32>,preview:vec4<u32>,domain:vec4<f32>,grid:vec4<f32>,dark:vec4<u32>,light:vec4<u32>};
@group(0) @binding(0) var<uniform> params:Params;
@group(0) @binding(1) var base:texture_2d<f32>;
@group(0) @binding(2) var opacity:texture_2d<f32>;
@group(0) @binding(3) var roughness:texture_2d<f32>;
@group(0) @binding(4) var metallic:texture_2d<f32>;
@group(0) @binding(5) var emission:texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) index:u32)->@builtin(position) vec4<f32> {
    let positions=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));
    return vec4<f32>(positions[index],0.,1.);
}
fn unit(v:f32)->u32 {return u32(floor(clamp(v,0.,1.)*255.+0.5));}
fn srgb(v:vec3<f32>)->vec3<u32> {
    let p=max(v,vec3<f32>(0.));
    let encoded=select(p*12.92,1.055*pow(p,vec3<f32>(1./2.4))-vec3<f32>(0.055),p>vec3<f32>(0.0031308));
    return vec3<u32>(floor(clamp(encoded,vec3<f32>(0.),vec3<f32>(1.))*255.+vec3<f32>(0.5)));
}
@fragment fn fs(@builtin(position) position:vec4<f32>)->@location(0) vec4<f32> {
    let p=floor(position.xy*params.viewport.xy/params.viewport.zw);
    var color=select(params.dark,params.light,(u32(p.x)/12u+u32(p.y)/12u)%2u==1u);
    var uv=p/params.viewport.xy;
    if(params.preview.y==0u){uv=fract(p/(params.viewport.y*0.5));}
    let cell=min(vec2<u32>(uv*f32(params.preview.z)),vec2<u32>(params.preview.z-1u));
    var sample=vec2<i32>(cell);
    if(params.grid.x>0.){
        sample=vec2<i32>(max(floor((params.domain.xy+(vec2<f32>(cell)+vec2<f32>(0.5))/f32(params.preview.z)*params.domain.zw)*params.grid.x)-params.grid.yz,vec2<f32>(0.)));
    }
    sample=min(sample,vec2<i32>(textureDimensions(base))-vec2<i32>(1));
    var rgb=srgb(textureLoad(base,sample,0).rgb);
    var alpha=unit(textureLoad(opacity,sample,0).x);
    switch params.preview.x {
        case 1u:{rgb=vec3<u32>(unit(textureLoad(opacity,sample,0).x));alpha=255u;}
        case 2u:{rgb=vec3<u32>(unit(textureLoad(roughness,sample,0).x));alpha=255u;}
        case 3u:{rgb=vec3<u32>(unit(textureLoad(metallic,sample,0).x));alpha=255u;}
        case 4u:{rgb=srgb(textureLoad(emission,sample,0).rgb);alpha=255u;}
        default:{}
    }
    if(params.preview.w==1u){return vec4<f32>(vec4<u32>(rgb,alpha))/255.;}
    color=vec4<u32>((rgb*alpha+color.rgb*(255u-alpha)+vec3<u32>(127u))/255u,color.a);
    return vec4<f32>(color)/255.;
}
