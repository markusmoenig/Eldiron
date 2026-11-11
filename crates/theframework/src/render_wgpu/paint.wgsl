struct Params { viewport:vec2<f32>,mode:u32,source:u32,color0:vec4<f32>,color1:vec4<f32>,gradient:vec4<f32> };
@group(0) @binding(0) var<uniform> params:Params;
@group(0) @binding(1) var image:texture_2d<f32>;
@group(0) @binding(2) var image_sampler:sampler;
struct Out{@builtin(position)position:vec4<f32>,@location(0)local:vec2<f32>,@location(1)uv:vec2<f32>};
@vertex fn vs(@location(0)p:vec2<f32>,@location(1)local:vec2<f32>,@location(2)uv:vec2<f32>)->Out{
    var o:Out;o.position=vec4<f32>(p.x/params.viewport.x*2.-1.,1.-p.y/params.viewport.y*2.,0.,1.);o.local=local;o.uv=uv;return o;
}
fn linear(c:vec3<f32>)->vec3<f32>{return select(c/12.92,pow((c+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),c>vec3<f32>(0.04045));}
@fragment fn fs(i:Out)->@location(0)vec4<f32>{
    var c=params.color0;
    if(params.mode==1u){let d=params.gradient.zw-params.gradient.xy;let t=clamp(dot(i.local-params.gradient.xy,d)/max(dot(d,d),0.000001),0.,1.);c=mix(params.color0,params.color1,t);}
    if(params.mode==2u){
        var tex=textureSample(image,image_sampler,i.uv);
        if((params.source&1u)!=0u){tex=vec4<f32>(linear(tex.rgb),tex.a);}
        if((params.source&2u)!=0u){return vec4<f32>(tex.rgb*c.rgb*c.a,tex.a*c.a);}
        c*=tex;
    }
    return vec4<f32>(c.rgb*c.a,c.a);
}
