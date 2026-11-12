struct Params { viewport:vec2<f32>,mode:u32,source:u32,color0:vec4<f32>,color1:vec4<f32>,gradient:vec4<f32> };
@group(0) @binding(0) var<uniform> params:Params;
@group(0) @binding(1) var image:texture_2d<f32>;
@group(0) @binding(2) var image_sampler:sampler;
struct DistancePrimitive { bounds: vec4<f32>, radii: vec4<f32>, fill: vec4<f32>, border: vec4<f32>, properties: vec4<f32> };
@group(0) @binding(3) var<storage,read> field: array<DistancePrimitive>;
fn distance_to(shape: DistancePrimitive, point: vec2<f32>) -> f32 {
    let p = point-shape.bounds.xy;
    let b = shape.bounds.zw;
    switch u32(shape.properties.x) {
        case 0u: { return length(p)-b.x; }
        case 1u: {
            var q=abs(p); let r=b.x; let k=vec3<f32>(-0.8660254,0.5,0.57735026);
            q-=2.*min(dot(k.xy,q),0.)*k.xy; q-=vec2<f32>(clamp(q.x,-k.z*r,k.z*r),r);
            return length(q)*sign(q.y);
        }
        case 2u: {
            let q=abs(p); let delta=b-2.*q; let h=clamp((delta.x*b.x-delta.y*b.y)/max(dot(b,b),0.000001),-1.,1.);
            return length(q-0.5*b*vec2<f32>(1.-h,1.+h))*sign(q.x*b.y+q.y*b.x-b.x*b.y);
        }
        default: {
            var radii=select(shape.radii.zw,shape.radii.xy,p.x>0.);
            let radius=select(radii.y,radii.x,p.y>0.);
            let q=abs(p)-b+vec2<f32>(radius);
            return min(max(q.x,q.y),0.)+length(max(q,vec2<f32>(0.)))-radius;
        }
    }
}
fn field_color(point: vec2<f32>, background: vec4<f32>) -> vec4<f32> {
    var d=3.402823e38; var nearest=0u;
    for(var index=0u;index<params.source;index++){let next=distance_to(field[index],point);if(next<d){d=next;nearest=index;}}
    let base=vec4<f32>(background.rgb*background.a,background.a);
    if(params.source==0u){return base;}
    let shape=field[nearest];
    let aa=max(fwidth(d),0.0001);
    let coverage=clamp(0.5-d/aa,0.,1.);
    let inner=clamp(0.5-(d+shape.properties.y)/aa,0.,1.);
    let border=vec4<f32>(shape.border.rgb*shape.border.a,shape.border.a);
    let fill=vec4<f32>(shape.fill.rgb*shape.fill.a,shape.fill.a);
    let edge=mix(border,fill,select(1.,inner,shape.properties.y>0.))*coverage;
    return edge+base*(1.-edge.a);
}
struct Out{@builtin(position)position:vec4<f32>,@location(0)local:vec2<f32>,@location(1)uv:vec2<f32>,@location(2)color:vec4<f32>};
@vertex fn vs(@location(0)p:vec2<f32>,@location(1)local:vec2<f32>,@location(2)uv:vec2<f32>,@location(3)color:vec4<f32>)->Out{
    var o:Out;o.position=vec4<f32>(p.x/params.viewport.x*2.-1.,1.-p.y/params.viewport.y*2.,0.,1.);o.local=local;o.uv=uv;o.color=color;return o;
}
fn linear(c:vec3<f32>)->vec3<f32>{return select(c/12.92,pow((c+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),c>vec3<f32>(0.04045));}
fn srgb(c:vec3<f32>)->vec3<f32>{let v=max(c,vec3<f32>(0.));return select(v*12.92,1.055*pow(v,vec3<f32>(1./2.4))-vec3<f32>(0.055),v>vec3<f32>(0.0031308));}
// Explicit LOD is required inside the non-uniform dilation loop. The UV crop
// is an image boundary, so adjacent atlas/cropped body pixels cannot leak in.
fn cropped_alpha(uv: vec2<f32>) -> f32 {
    if(any(uv < params.color1.xy) || any(uv >= params.color1.zw)){return 0.;}
    return textureSampleLevel(image,image_sampler,uv,0.).a;
}
fn outline(uv: vec2<f32>) -> vec4<f32> {
    let center=cropped_alpha(uv);
    if(center>=1.){return vec4<f32>(0.);}
    let radius=i32(ceil(params.gradient.z));
    var coverage=0.;
    for(var y=-radius;y<=radius;y++){
        for(var x=-radius;x<=radius;x++){
            let delta=vec2<f32>(f32(x),f32(y))*(params.gradient.z/f32(radius));
            if(dot(delta,delta)<=params.gradient.z*params.gradient.z){
                coverage=max(coverage,cropped_alpha(uv+delta*params.gradient.xy));
                if(coverage>=1.){break;}
            }
        }
        if(coverage>=1.){break;}
    }
    coverage=clamp((coverage-center)/max(1.-center,0.000001),0.,1.);
    let alpha=coverage*params.color0.a;
    return vec4<f32>(params.color0.rgb*alpha,alpha);
}
@fragment fn fs(i:Out)->@location(0)vec4<f32>{
    var c=params.color0;
    if(params.mode==6u){return outline(i.uv);}
    if(params.mode==4u){return field_color(i.local-params.gradient.xy,c);}
    if(params.mode==3u){c=vec4<f32>(linear(i.color.rgb),i.color.a);}
    if(params.mode==1u){let d=params.gradient.zw-params.gradient.xy;let t=clamp(dot(i.local-params.gradient.xy,d)/max(dot(d,d),0.000001),0.,1.);c=mix(params.color0,params.color1,t);}
    if(params.mode==2u || params.mode==5u || params.mode==7u){
        var tex=textureSample(image,image_sampler,i.uv);
        if((params.source&1u)!=0u){tex=vec4<f32>(linear(tex.rgb),tex.a);}
        if(params.mode==7u){
            var straight=tex.rgb;
            if((params.source&2u)!=0u){straight=tex.rgb/max(tex.a,0.000001);}
            let bytes=round(clamp(srgb(straight),vec3<f32>(0.),vec3<f32>(1.))*255.);
            let tone=select(params.color0,params.color1,dot(bytes,params.gradient.rgb)>=params.gradient.a);
            let alpha=tex.a*tone.a;
            return vec4<f32>(tone.rgb*alpha,alpha);
        }
        if(params.mode==5u){
            var straight=tex.rgb;
            if((params.source&2u)!=0u){straight=tex.rgb/max(tex.a,0.000001);}
            let encoded=srgb(straight);
            let gray=dot(encoded,vec3<f32>(0.299,0.587,0.114));
            let adjusted=clamp(max(mix(vec3<f32>(gray),encoded,params.color1.a)*params.color1.rgb+params.gradient.rgb,vec3<f32>(params.gradient.a)),vec3<f32>(0.),vec3<f32>(1.));
            return vec4<f32>(linear(adjusted)*tex.a*c.rgb*c.a,tex.a*c.a);
        }
        if((params.source&2u)!=0u){return vec4<f32>(tex.rgb*c.rgb*c.a,tex.a*c.a);}
        c*=tex;
    }
    return vec4<f32>(c.rgb*c.a,c.a);
}
