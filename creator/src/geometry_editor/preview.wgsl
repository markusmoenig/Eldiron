struct Input {
    @location(0) position: vec4f,
    @location(1) normal: vec3f,
    @location(2) color: vec3f,
    @location(3) material: vec2f,
    @location(4) uv: vec2f,
    @location(5) layer: f32,
};
struct Output {
    @builtin(position) position: vec4f,
    @location(0) normal: vec3f,
    @location(1) color: vec3f,
    @location(2) material: vec2f,
    @location(3) uv: vec2f,
    @location(4) @interpolate(flat) layer:f32,
};
@vertex fn vs(v: Input) -> Output {
    var o: Output; o.position=v.position; o.normal=v.normal; o.color=v.color; o.material=v.material;o.uv=v.uv;o.layer=v.layer; return o;
}
@group(0) @binding(0) var colors:texture_2d_array<f32>;
@group(0) @binding(1) var nearest:sampler;
@group(0) @binding(2) var materials:texture_2d_array<f32>;
@group(0) @binding(3) var emissions:texture_2d_array<f32>;
// Fixed Cook-Torrance inspection lighting. Geometry edits do not compile shaders.
@fragment fn fs(v: Output, @builtin(front_facing) front: bool) -> @location(0) vec4f {
    // Engine particle colors are emissive palette colors, unaffected by inspection lighting.
    if v.layer < -0.5 { return vec4f(v.color,1.0); }
    let uv=v.uv;let layer=i32(v.layer+0.5);
    let tex=textureSample(colors,nearest,uv,layer);
    let data=textureSample(materials,nearest,uv,layer);
    let glow=textureSample(emissions,nearest,uv,layer).rgb;
    if layer>0 && tex.a<0.05{discard;}
    let base=select(v.color,tex.rgb,layer>0);
    let n=normalize(v.normal)*select(-1.0,1.0,front);
    let eye=vec3f(0.0,0.0,1.0); let light=normalize(vec3f(-0.6,0.7,1.0));
    let h=normalize(eye+light);
    let nl=max(dot(n,light),0.0); let nv=max(dot(n,eye),0.001);
    let nh=max(dot(n,h),0.0); let vh=max(dot(eye,h),0.0);
    let rough=clamp(select(v.material.x,data.r,layer>0),0.06,1.0); let metal=select(v.material.y,data.g,layer>0);
    let a2=pow(rough,4.0); let d=a2/(3.14159265*pow(nh*nh*(a2-1.0)+1.0,2.0));
    let k=pow(rough+1.0,2.0)/8.0;
    let g=(nl/(nl*(1.0-k)+k))*(nv/(nv*(1.0-k)+k));
    let f0=mix(vec3f(0.04),base,metal); let f=f0+(1.0-f0)*pow(1.0-vh,5.0);
    let spec=d*g*f/max(4.0*nl*nv,0.001);
    let diffuse=(1.0-f)*(1.0-metal)*base/3.14159265;
    let fill=0.17+0.18*max(dot(n,normalize(vec3f(0.8,0.25,0.5))),0.0);
    let color=(diffuse+spec)*nl*2.5+base*fill*mix(1.0,2.5,metal)+select(vec3f(0.0),glow,layer>0);
    return vec4f(color/(vec3f(1.0)+color),1.0);
}
