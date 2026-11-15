struct Params { viewport:vec2<f32>,mode:u32,source:u32,color0:vec4<f32>,color1:vec4<f32>,gradient:vec4<f32>,inverse:vec4<f32> };
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
fn radial_color(point: vec2<f32>, rect: vec4<f32>) -> vec4<f32> {
    let source_size=field[0].bounds.xy;
    let source_scale=vec2<f32>(params.color0.a,params.color1.a);
    let pixel=min(floor(floor(point-rect.xy)/source_scale),source_size-vec2<f32>(1.));
    let row=u32(pixel.y);let entry=field[1u+row/20u];let component=(row%20u)/4u;
    var words=entry.bounds;
    switch component {
        case 1u: { words=entry.radii; }
        case 2u: { words=entry.fill; }
        case 3u: { words=entry.border; }
        case 4u: { words=entry.properties; }
        default: {}
    }
    let packed=u32(words[row%4u]);
    var color=vec4<f32>(f32(packed&255u),f32((packed>>8u)&255u),f32((packed>>16u)&255u),255.);
    for(var index=1u+u32(field[0].bounds.z);index<params.source;index++){
        let shape=field[index];
        if(shape.properties.x==0.){
            let delta=(pixel+vec2<f32>(0.5)-shape.bounds.xy)/max(shape.bounds.z,0.001);
            let distance=length(delta);
            if(distance<=1.){
                let falloff=(1.-distance)*(1.-distance);
                let alpha=(shape.fill.a/255.)*falloff;
                color=vec4<f32>(floor(color.rgb*(1.-alpha)+shape.fill.rgb*alpha),255.);
            }
        }else if(all(pixel>=shape.bounds.xy) && all(pixel<shape.bounds.xy+shape.bounds.zw)){
            let distance=length(pixel-shape.radii.xy)-shape.radii.z;
            if(distance<1.){
                let alpha=clamp(-distance,0.,1.)*(shape.fill.a/255.);
                color=floor((color/255.*(1.-alpha)+shape.fill/255.*alpha)*255.);
                // The reference disc's border is a color mask, independent of
                // its stored alpha. This preserves its black edge over glows.
                let border=clamp(distance+shape.properties.y,0.,1.)-clamp(distance,0.,1.);
                color=floor((color/255.*(1.-border)+shape.border/255.*border)*255.);
            }
        }
    }
    let alpha=select(color.a/255.,1.,(params.mode&512u)!=0u);
    return vec4<f32>(target_color(color.rgb/255.)*alpha,alpha);
}
struct Out{@builtin(position)position:vec4<f32>,@location(0)local:vec2<f32>,@location(1)uv:vec2<f32>,@location(2)color:vec4<f32>,@location(3)@interpolate(flat)origin:vec2<f32>,@location(4)@interpolate(flat)pixel:vec2<f32>};
// SampledLine has no image and source is zero. The uniform identity keeps
// backend contraction from changing the CPU DDA's observable f32 rounding.
fn dda_fp(value:f32)->f32 {return bitcast<f32>(bitcast<u32>(value)^params.source);}
fn dda_divide(a:f32,b:f32)->f32 {
    let q=dda_fp(a/b);
    return dda_fp(q+dda_fp(fma(-q,b,a)/b));
}
@vertex fn vs(@location(0)p:vec2<f32>,@location(1)local:vec2<f32>,@location(2)uv:vec2<f32>,@location(3)color:vec4<f32>,@builtin(instance_index)instance:u32)->Out{
    var position=p;var output_local=local;var origin=vec2<f32>(0.);var sample_pixel=vec2<f32>(0.);
    if((params.mode&255u)==14u || (params.mode&255u)==19u){
        let endpoints=select(params.color1,color,(params.mode&255u)==19u);
        let a=endpoints.xy;
        let delta=vec2<f32>(dda_fp(endpoints.z-a.x),dda_fp(endpoints.w-a.y));
        let count=min(ceil(max(abs(delta.x),abs(delta.y))),8192.);
        let t=dda_divide(f32(instance),count);
        let q=vec2<f32>(dda_fp(a.x+dda_fp(delta.x*t)),dda_fp(a.y+dda_fp(delta.y*t)));
        // WGSL round uses ties-to-even; the reference uses ties-away-from-zero.
        // Split the integer part first: adding .5 can itself round a value just
        // below a half to the next integer before floor observes it.
        let base=floor(abs(q));
        let pixel=sign(q)*(base+select(vec2<f32>(0.),vec2<f32>(1.),abs(q)-base>=vec2<f32>(0.5)));
        let bx=params.gradient.xy;let by=params.gradient.zw;
        origin=p-bx*local.x-by*local.y;sample_pixel=pixel;
        let determinant=abs(bx.x*by.y-bx.y*by.x);
        // Expand support by a physical pixel. Coverage below evaluates the
        // logical pixel at the physical center, independently of MSAA.
        let pad=vec2<f32>(abs(by.y)+abs(by.x),abs(bx.y)+abs(bx.x))/determinant;
        output_local=local*(vec2<f32>(1.)+2.*pad)-pad;
        position+=bx*(pixel.x+output_local.x-local.x)+by*(pixel.y+output_local.y-local.y);
        if(f32(instance)>count){position=vec2<f32>(-100.,-100.);}
    }
    if((params.mode&255u)==15u || (params.mode&255u)==16u || (params.mode&255u)==17u || (params.mode&255u)==18u){
        let bx=params.gradient.xy;let by=params.gradient.zw;
        origin=p-bx*local.x-by*local.y;
        let determinant=abs(bx.x*by.y-bx.y*by.x);
        let pad=vec2<f32>(abs(by.y)+abs(by.x),abs(bx.y)+abs(bx.x))/determinant;
        let delta=((local-color.xy)/color.zw*2.-vec2<f32>(1.))*pad;
        position+=bx*delta.x+by*delta.y;
    }
    var o:Out;o.position=vec4<f32>(position.x/params.viewport.x*2.-1.,1.-position.y/params.viewport.y*2.,0.,1.);o.local=output_local;o.uv=uv;o.color=color;o.origin=origin;o.pixel=sample_pixel;return o;
}

fn linear(c:vec3<f32>)->vec3<f32>{return select(c/12.92,pow((c+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),c>vec3<f32>(0.04045));}
fn srgb(c:vec3<f32>)->vec3<f32>{let v=max(c,vec3<f32>(0.));return select(v*12.92,1.055*pow(v,vec3<f32>(1./2.4))-vec3<f32>(0.055),v>vec3<f32>(0.0031308));}
fn target_color(color:vec3<f32>)->vec3<f32> {
    if((params.mode&256u)!=0u){return color;}
    return linear(color);
}
fn target_image(texel:vec4<f32>)->vec4<f32> {
    if((params.mode&256u)==0u || (params.source&32u)!=0u){return texel;}
    if(texel.a>0.){return vec4<f32>(srgb(texel.rgb/texel.a)*texel.a,texel.a);}
    return vec4<f32>(0.);
}
// All sampled image colors leave these helpers linear and premultiplied.
// Normalized sources keep hardware filtering; raw GPU sources normalize each
// contributing texel before bilinear interpolation to preserve alpha edges.
fn normalized_texel(texel:vec4<f32>)->vec4<f32> {
    var c=texel;
    if((params.source&64u)!=0u){c.a=1.;}
    if((params.source&1u)!=0u){c=vec4<f32>(linear(c.rgb),c.a);}
    if((params.source&2u)==0u){c=vec4<f32>(c.rgb*c.a,c.a);}
    return c;
}
fn source_texel(position:vec2<i32>)->vec4<f32> {
    let size=vec2<i32>(textureDimensions(image));
    var p=position;
    if((params.source&8u)!=0u){p.x=((p.x%size.x)+size.x)%size.x;}
    else {p.x=clamp(p.x,0,size.x-1);}
    if((params.source&16u)!=0u){p.y=((p.y%size.y)+size.y)%size.y;}
    else {p.y=clamp(p.y,0,size.y-1);}
    return normalized_texel(textureLoad(image,p,0));
}
fn straight_sample(texel:vec4<f32>)->vec3<f32> {
    if(texel.a>0.){return texel.rgb/texel.a;}
    return vec3<f32>(0.);
}
// UI and scene-output images have one mip level. Explicit base LOD preserves
// their sampling while remaining valid after per-pixel clipping/early returns.
fn sampled_image(uv:vec2<f32>)->vec4<f32> {
    if((params.source&4u)==0u){return normalized_texel(textureSampleLevel(image,image_sampler,uv,0.));}
    let p=uv*vec2<f32>(textureDimensions(image))-vec2<f32>(0.5);
    let origin=vec2<i32>(floor(p));
    let fraction=fract(p);
    let top=mix(source_texel(origin),source_texel(origin+vec2<i32>(1,0)),fraction.x);
    let bottom=mix(source_texel(origin+vec2<i32>(0,1)),source_texel(origin+vec2<i32>(1,1)),fraction.x);
    return mix(top,bottom,fraction.y);
}
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
// Exact unsigned 32x32 -> 64 multiply, stored as (low,high). No f64 or
// optional shader features are needed for panned lines with large endpoints.
fn line_product(a:u32,b:u32)->vec2<u32>{
    let a0=a&65535u;let a1=a>>16u;let b0=b&65535u;let b1=b>>16u;
    let t=a0*b0;let u=a1*b0+(t>>16u);let v=a0*b1+(u&65535u);
    return vec2<u32>((v<<16u)|(t&65535u),a1*b1+(u>>16u)+(v>>16u));
}
fn line_minor(k:u32,minor:u32,major:u32)->u32{
    if(k==0u || minor==0u){return 0u;}if(k==major){return minor;}if(minor==major){return k;}
    var product=line_product(k,minor);let before=product.x;product.x+=major/2u;
    product.y+=select(0u,1u,product.x<before);
    if(product.y==0u){return product.x/major;}
    // The quotient fits u32 and the high-word remainder is below the divisor.
    var remainder=product.y;var quotient=0u;
    for(var bit=31;bit>=0;bit--){
        let carry=remainder>>31u;
        remainder=(remainder<<1u)|((product.x>>u32(bit))&1u);
        if(carry!=0u || remainder>=major){remainder-=major;quotient|=1u<<u32(bit);}
    }
    return quotient;
}
fn line_coverage(point:vec2<i32>,endpoints:vec4<i32>)->bool{
    let a=endpoints.xy;let b=endpoints.zw;
    if(all(a==b)){return all(point==a);}
    let af=bitcast<vec2<u32>>(a);let bf=bitcast<vec2<u32>>(b);let pf=bitcast<vec2<u32>>(point);
    let distance=select(af-bf,bf-af,b>=a);
    if(distance.x>=distance.y){
        if(point.x<min(a.x,b.x) || point.x>max(a.x,b.x)){return false;}
        let k=select(af.x-pf.x,pf.x-af.x,b.x>=a.x);
        let step=line_minor(k,distance.y,distance.x);
        return pf.y==af.y+select(0u-step,step,b.y>=a.y);
    }
    if(point.y<min(a.y,b.y) || point.y>max(a.y,b.y)){return false;}
    let k=select(af.y-pf.y,pf.y-af.y,b.y>=a.y);
    let step=line_minor(k,distance.x,distance.y);
    return pf.x==af.x+select(0u-step,step,b.x>=a.x);
}
fn triangle_edge(a:vec2<f32>, b:vec2<f32>, p:vec2<f32>)->f32 {
    return (p.x-a.x)*(b.y-a.y)-(p.y-a.y)*(b.x-a.x);
}
fn triangle_coverage(p:vec2<f32>)->bool {
    let a=params.color1.xy; let b=params.color1.zw; let c=params.gradient.xy;
    let e0=triangle_edge(a,b,p); let e1=triangle_edge(b,c,p); let e2=triangle_edge(c,a,p);
    return (e0>=0. && e1>=0. && e2>=0.) || (e0<=0. && e1<=0. && e2<=0.);
}
@fragment fn fs(i:Out)->@location(0)vec4<f32>{
    let mode=params.mode&255u;
    // Evaluate distance-field derivatives before any varying-dependent return.
    if(mode==4u){return field_color(i.local-params.gradient.xy,params.color0);}
    if(mode==18u){
        let q=i.position.xy-i.origin;
        let point=floor(vec2<f32>(dot(params.inverse.xy,q),dot(params.inverse.zw,q))-params.color1.xy);
        let size=params.color1.zw;
        if(any(point<vec2<f32>(0.)) || any(point>=size)){return vec4<f32>(0.);}
        let hue=point.x/size.x*6.;let band=u32(hue);
        let rem=hue-floor(hue*0.5)*2.;let x=1.-abs(rem-1.);
        var base=vec3<f32>(1.,0.,x);
        switch band {
            case 0u: {base=vec3<f32>(1.,x,0.);}
            case 1u: {base=vec3<f32>(x,1.,0.);}
            case 2u: {base=vec3<f32>(0.,1.,x);}
            case 3u: {base=vec3<f32>(0.,x,1.);}
            case 4u: {base=vec3<f32>(x,0.,1.);}
            default: {}
        }
        let fy=point.y/size.y;
        var rgb=base*(1.-(fy-0.5)*2.);
        if(fy<0.5){let t=1.-fy*2.;rgb=base*(1.-t)+vec3<f32>(t);}
        rgb=floor(clamp(rgb,vec3<f32>(0.),vec3<f32>(1.))*255.)/255.;
        return vec4<f32>(target_color(rgb),1.);
    }
    if(mode==16u){
        let bx=params.gradient.xy;let by=params.gradient.zw;let q=i.position.xy-i.origin;
        let logical=vec2<f32>(dot(params.inverse.xy,q),dot(params.inverse.zw,q));
        if(any(logical<i.color.xy) || any(logical>=i.color.xy+i.color.zw)){return vec4<f32>(0.);}
        return radial_color(logical,i.color);
    }
    if(mode==8u){return vec4<f32>(linear(textureSampleLevel(image,image_sampler,i.uv,0.).rgb),1.);}
    if(mode==9u){return vec4<f32>(textureSampleLevel(image,image_sampler,i.uv,0.).rgb,1.);}
    var c=params.color0;
    if(mode==15u){
        let bx=params.gradient.xy;let by=params.gradient.zw;let q=i.position.xy-i.origin;
        let logical=vec2<f32>(dot(params.inverse.xy,q),dot(params.inverse.zw,q));
        let pixel=floor(logical-i.color.xy);
        if(any(pixel<vec2<f32>(0.)) || any(pixel>=i.color.zw)){return vec4<f32>(0.);}
        let position=vec2<i32>(min(floor(pixel/params.color1.xy),vec2<f32>(textureDimensions(image))-vec2<f32>(1.)));
        let tex=target_image(normalized_texel(textureLoad(image,position,0)));
        let alpha=floor(clamp(tex.a*c.a,0.,1.)*255.+0.5)/255.;
        return vec4<f32>(straight_sample(tex)*c.rgb*alpha,alpha);
    }
    if(mode==14u || mode==19u){
        // Invert the draw basis at the physical center. Interpolated local
        // coordinates can drift across exact texel boundaries at fractional DPI.
        let bx=params.gradient.xy;let by=params.gradient.zw;let q=i.position.xy-i.origin;
        let logical=vec2<f32>(dot(params.inverse.xy,q),dot(params.inverse.zw,q));
        let alpha=select(0.,c.a,all(floor(logical)==i.pixel));
        return vec4<f32>(c.rgb*alpha,alpha);
    }
    if(mode==13u){
        let alpha=select(0.,c.a,triangle_coverage(floor(i.position.xy)+vec2<f32>(0.5)));
        return vec4<f32>(c.rgb*alpha,alpha);
    }
    if(mode==17u){
        let bx=params.gradient.xy;let by=params.gradient.zw;let q=i.position.xy-i.origin;
        let logical=vec2<f32>(dot(params.inverse.xy,q),dot(params.inverse.zw,q));
        let packed=(vec4<u32>(field[0].bounds)<<vec4<u32>(16u))|vec4<u32>(field[0].radii);
        let alpha=select(0.,c.a,line_coverage(vec2<i32>(floor(logical)),bitcast<vec4<i32>>(packed)));
        return vec4<f32>(c.rgb*alpha,alpha);
    }
    if(mode==12u){
        let packed=(vec4<u32>(params.color1)<<vec4<u32>(16u))|vec4<u32>(params.gradient);
        let alpha=select(0.,c.a,line_coverage(vec2<i32>(floor(i.position.xy)),bitcast<vec4<i32>>(packed)));
        return vec4<f32>(c.rgb*alpha,alpha);
    }
    if(mode==11u){
        let source=vec2<f32>(textureDimensions(image));
        let destination=max(params.gradient.xy,vec2<f32>(0.000001));
        let pixel=max(floor(i.uv*destination),vec2<f32>(0.));
        var position=vec2<i32>(min(floor(pixel*(source/destination)),source-vec2<f32>(1.)));
        // Snapped axis-aligned pixel rectangles use integer left-edge division,
        // exactly as CPU source-buffer composition. Bound products to u32;
        // general affine/fractional extents retain their floating lookup.
        if(all(destination==floor(destination)) && all(destination<=vec2<f32>(65535.)) && all(source<=vec2<f32>(65535.))){
            position=vec2<i32>(min(vec2<u32>(pixel)*vec2<u32>(source)/vec2<u32>(destination),vec2<u32>(source)-vec2<u32>(1u)));
        }
        let tex=target_image(normalized_texel(textureLoad(image,position,0)));
        return vec4<f32>(tex.rgb*c.rgb*c.a,tex.a*c.a);
    }
    if(mode==10u){
        let tex=target_image(sampled_image(i.uv));
        return vec4<f32>(tex.rgb*c.rgb+params.color1.rgb*(1.-tex.a),1.);
    }
    if(mode==6u){return outline(i.uv);}
    if(mode==3u){c=vec4<f32>(target_color(i.color.rgb),i.color.a);}
    if(mode==1u){let d=params.gradient.zw-params.gradient.xy;let t=clamp(dot(i.local-params.gradient.xy,d)/max(dot(d,d),0.000001),0.,1.);c=mix(params.color0,params.color1,t);}
    if(mode==2u || mode==5u || mode==7u){
        let tex=sampled_image(i.uv);
        if(mode==7u){
            let straight=straight_sample(tex);
            let bytes=round(clamp(srgb(straight),vec3<f32>(0.),vec3<f32>(1.))*255.);
            let tone=select(params.color0,params.color1,dot(bytes,params.gradient.rgb)>=params.gradient.a);
            let alpha=tex.a*tone.a;
            return vec4<f32>(tone.rgb*alpha,alpha);
        }
        if(mode==5u){
            let straight=straight_sample(tex);
            let encoded=srgb(straight);
            let gray=dot(encoded,vec3<f32>(0.299,0.587,0.114));
            let adjusted=clamp(max(mix(vec3<f32>(gray),encoded,params.color1.a)*params.color1.rgb+params.gradient.rgb,vec3<f32>(params.gradient.a)),vec3<f32>(0.),vec3<f32>(1.));
            return vec4<f32>(target_color(adjusted)*tex.a*c.rgb*c.a,tex.a*c.a);
        }
        let blended=target_image(tex);
        return vec4<f32>(blended.rgb*c.rgb*c.a,blended.a*c.a);
    }
    return vec4<f32>(c.rgb*c.a,c.a);
}
