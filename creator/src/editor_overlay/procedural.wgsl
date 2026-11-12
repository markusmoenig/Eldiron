struct Params { words: array<vec4<u32>,6>, };
@group(0) @binding(0) var<uniform> params: Params;
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));
    return vec4(p[index],0.,1.);
}
fn trunc_fract(x: f32) -> f32 { return x-trunc(x); }
fn hash_color(p: vec2<f32>) -> f32 {
    var p3 = vec3(trunc_fract(p.x*0.13),trunc_fract(p.y*0.13),trunc_fract(p.x*0.13));
    p3 += dot(p3,p3.yzx+vec3(3.333));
    return trunc_fract((p3.x+p3.y)*p3.z);
}
fn color_noise(x: vec2<f32>) -> f32 {
    let i = floor(x); let f = vec2(trunc_fract(x.x),trunc_fract(x.y));
    let a = hash_color(i); let b = hash_color(i+vec2(1.,0.));
    let c = hash_color(i+vec2(0.,1.)); let d = hash_color(i+vec2(1.,1.));
    let u = f*f*(vec2(3.)-2.*f);
    return fma(u.x,b-a,a)+(c-a)*u.y*(1.-u.x)+(d-b)*u.x*u.y;
}
fn fbm(p: vec2<f32>) -> f32 {
    var x = p*8.; var v = 0.; var a = 0.5;
    // The reference rotates through Vek's row/column convention.
    let rot = mat2x2<f32>(vec2(0.87758255,-0.47942555),vec2(0.47942555,0.87758255));
    for (var i=0; i<4; i++) { v += a*color_noise(x); x=rot*x*2.+vec2(100.); a*=0.5; }
    return v;
}
fn hue_angle(input: f32,x: f32,y: f32) -> f32 {
    var hue=input;
    if hue<0. { hue+=1.; } else if hue>1. { hue-=1.; }
    if hue<1./6. { return x+(y-x)*6.*hue; }
    if hue<0.5 { return y; }
    if hue<2./3. { return x+(y-x)*(2./3.-hue)*6.; }
    return x;
}
fn from_hsl(h: f32,s: f32,l: f32) -> vec3<f32> {
    if s==0. { return vec3(l); }
    var y=l+s-l*s; if l<0.5 { y=l*(1.+s); }
    let x=2.*l-y;
    // Preserve the original to_tile call's hue unit convention exactly.
    let hue=h/360.;
    return vec3(hue_angle(hue+1./3.,x,y),hue_angle(hue,x,y),hue_angle(hue-1./3.,x,y));
}
fn hash_noise(x: i32,y: i32,channel: u32) -> f32 {
    let period=i32(params.words[5].x);
    let px=u32(((x%period)+period)%period); let py=u32(((y%period)+period)%period);
    var h=params.words[5].y^(px*0x9e3779b9u)^(py*0x85ebca6bu)^channel;
    h^=h>>16u; h*=0x7feb352du; h^=h>>15u; h*=0x846ca68bu; h^=h>>16u;
    return f32(h)/4294967296.;
}
fn sample_noise(p: vec2<f32>) -> f32 {
    let position=p*f32(params.words[5].x); let i=vec2<i32>(floor(position)); let f=position-floor(position);
    if params.words[5].z!=0u {
        var distance=1e30;
        for (var dy=-1; dy<=1; dy++) { for (var dx=-1; dx<=1; dx++) {
            let px=f32(dx)+hash_noise(i.x+dx,i.y+dy,0u);
            let py=f32(dy)+hash_noise(i.x+dx,i.y+dy,0xa511e9b3u);
            distance=min(distance,(px-f.x)*(px-f.x)+(py-f.y)*(py-f.y));
        }}
        return clamp(sqrt(distance),0.,1.);
    }
    let weight=f*f*(vec2(3.)-2.*f);
    let a=hash_noise(i.x,i.y,0u)*(1.-weight.x)+hash_noise(i.x+1,i.y,0u)*weight.x;
    let b=hash_noise(i.x,i.y+1,0u)*(1.-weight.x)+hash_noise(i.x+1,i.y+1,0u)*weight.x;
    return a*(1.-weight.y)+b*weight.y;
}
@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let size=f32(params.words[0].y); let pixel=floor(position.xy);
    if params.words[0].x==1u {
        let value=sample_noise(pixel/size);
        let low=vec4<f32>(params.words[3]); let high=vec4<f32>(params.words[4]);
        return floor(clamp(low+(high-low)*value,vec4(0.),vec4(255.))+vec4(0.5))/255.;
    }
    let block=f32(params.words[0].w); let p=floor(pixel/block)*block/size;
    let hsl=bitcast<vec4<f32>>(params.words[2]);
    let noise=fbm(p)*hsl.w; let uniform_noise=(noise*2.-1.)*hsl.w;
    var rgb=vec3<f32>(params.words[1].xyz)*(1.+uniform_noise);
    if params.words[0].z==1u { rgb=from_hsl(trunc_fract(hsl.x+uniform_noise),hsl.y,hsl.z)*255.; }
    if params.words[0].z==2u { rgb=from_hsl(hsl.x,hsl.y,clamp(hsl.z+uniform_noise,0.,1.))*255.; }
    return vec4(floor(clamp(rgb,vec3(0.),vec3(255.)))/255.,1.);
}
