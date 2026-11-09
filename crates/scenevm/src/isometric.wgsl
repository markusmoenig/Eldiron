struct Uniforms {projection:vec4<f32>,sun:vec4<f32>,sun_color:vec4<f32>,ambient:vec4<f32>,counts:vec4<u32>,shadow:vec4<f32>};
struct PointLight {position:vec4<f32>,color:vec4<f32>,parameters:vec4<f32>};
struct Caster {lo:vec4<f32>,hi:vec4<f32>,position_scale:vec4<f32>,grid:vec4<u32>};
struct Particle {position_radius:vec4<f32>,color:vec4<f32>};
@group(0) @binding(0) var<uniform> u:Uniforms;
@group(0) @binding(1) var<storage,read> albedo:array<u32>;
@group(0) @binding(2) var<storage,read> surfaces:array<vec2<u32>>;
@group(0) @binding(3) var<storage,read> lights:array<PointLight>;
@group(0) @binding(4) var<storage,read> casters:array<Caster>;
@group(0) @binding(5) var<storage,read> columns:array<vec2<f32>>;
@group(0) @binding(6) var<storage,read> particles:array<Particle>;
@group(0) @binding(7) var output:texture_storage_2d<rgba8unorm,write>;
struct ShadowView {right:vec4<f32>,up:vec4<f32>,forward:vec4<f32>,origin:vec4<f32>,projection:vec4<f32>,settings:vec4<u32>};
@group(0) @binding(8) var<storage,read> shadow_views:array<ShadowView>;
@group(0) @binding(9) var sun_depth:texture_depth_2d_array;
@group(0) @binding(10) var point_depth:texture_depth_2d_array;
// PCSS: blocker search, geometric penumbra estimate, then disk filtering.
// https://developer.nvidia.com/gpugems/gpugems3/part-ii-light-and-shadows/chapter-8-summed-area-variance-shadow-maps
const SHADOW_DISK = array<vec2<f32>,32>(
    vec2<f32>(0.12500000,0.00000000),
    vec2<f32>(-0.15964505,0.14624794),
    vec2<f32>(0.02443623,-0.27843827),
    vec2<f32>(0.20122224,0.26245878),
    vec2<f32>(-0.36926756,-0.06531823),
    vec2<f32>(0.34980247,-0.22251570),
    vec2<f32>(-0.11700208,0.43524190),
    vec2<f32>(-0.22313565,-0.42963412),
    vec2<f32>(0.48411512,0.17679806),
    vec2<f32>(-0.50364111,0.20789573),
    vec2<f32>(0.24278829,-0.51882448),
    vec2<f32>(0.17941437,0.57200130),
    vec2<f32>(-0.54075701,-0.31337974),
    vec2<f32>(0.63436952,-0.13946436),
    vec2<f32>(-0.38714585,0.55067513),
    vec2<f32>(-0.08943965,-0.69019964),
    vec2<f32>(0.54907176,0.46275826),
    vec2<f32>(-0.73887847,0.03055494),
    vec2<f32>(0.53895513,-0.53633233),
    vec2<f32>(-0.03605819,0.77979152),
    vec2<f32>(-0.51281753,-0.61452679),
    vec2<f32>(0.81235959,0.10930183),
    vec2<f32>(-0.68831064,0.47890862),
    vec2<f32>(0.18808606,-0.83606138),
    vec2<f32>(0.43503327,0.75919105),
    vec2<f32>(-0.85044841,-0.27131624),
    vec2<f32>(0.82610240,-0.38168026),
    vec2<f32>(-0.35788820,0.85515556),
    vec2<f32>(-0.31940733,-0.88803376),
    vec2<f32>(0.84990864,0.44668816),
    vec2<f32>(-0.94403465,0.24884450),
    vec2<f32>(0.53659581,-0.83452977)
);
fn cube_face(ray:vec3<f32>)->u32 {
    let a=abs(ray);
    if a.x>=a.y&&a.x>=a.z {return select(1u,0u,ray.x>=0.0);}
    if a.y>=a.z {return select(3u,2u,ray.y>=0.0);}
    return select(5u,4u,ray.z>=0.0);
}
fn point_blocker(light:u32,ray:vec3<f32>)->f32 {
    let index=1u+light*6u+cube_face(ray);let camera=shadow_views[index];
    if camera.settings.z==0u {return 1e20;}
    let z=dot(ray,camera.forward.xyz);
    let xy=vec2<f32>(dot(ray,camera.right.xyz),dot(ray,camera.up.xyz))/z;
    let uv=vec2<f32>(xy.x*0.5+0.5,0.5-xy.y*0.5);
    let dim=textureDimensions(point_depth);
    let at=clamp(vec2<i32>(uv*vec2<f32>(dim)),vec2<i32>(0),vec2<i32>(dim)-1);
    let depth=textureLoad(point_depth,at,i32(index-1u),0);
    if depth>=0.999999 {return 1e20;}
    let near=camera.projection.z;let far=camera.projection.w;
    let linear=near*far/max(far-depth*(far-near),0.000001);
    return linear*length(ray)/z;
}
fn plane_distance(delta:vec3<f32>,normal:vec3<f32>,ray:vec3<f32>)->f32 {
    let denom=dot(normal,ray);
    if abs(denom)<0.0001 {return length(delta);}
    let t=dot(normal,delta)/denom;
    return select(length(delta),t*length(ray),t>0.0);
}
fn soft_point_visibility(point:vec3<f32>,normal:vec3<f32>,light:u32)->f32 {
    let delta=point-lights[light].position.xyz;let distance=max(length(delta),0.001);
    let direction=delta/distance;
    let axis=select(vec3<f32>(0.0,1.0,0.0),vec3<f32>(0.0,0.0,1.0),abs(direction.y)>0.95);
    let right=normalize(cross(direction,axis));let up=cross(right,direction);
    let source=lights[light].parameters.w;
    let search=clamp(source/max(distance*0.15,0.08),0.002,0.75);
    var blockers=0.0;var total=0.0;
    for(var i=0u;i<16u;i++) {
        let offset=SHADOW_DISK[i*2u]*search;
        let ray=direction+right*offset.x+up*offset.y;
        let depth=point_blocker(light,ray);
        if depth<plane_distance(delta,normal,ray)-0.008 {total+=depth;blockers+=1.0;}
    }
    if blockers==0.0 {return 1.0;}
    let blocker=total/blockers;
    let penumbra=clamp(source*max(distance-blocker,0.0)/(distance*max(blocker,0.02)),0.002,0.8);
    var visible=0.0;
    for(var i=0u;i<32u;i++) {
        let offset=SHADOW_DISK[i]*penumbra;
        let ray=direction+right*offset.x+up*offset.y;
        let receiver=plane_distance(delta,normal,ray);
        visible+=select(0.0,1.0,receiver-0.008<=point_blocker(light,ray));
    }
    return 1.0-u.shadow.y*(1.0-visible/32.0);
}
fn mesh_visibility(point:vec3<f32>,normal:vec3<f32>,direction:vec3<f32>,light:i32)->f32 {
    if light>=0 && lights[u32(light)].parameters.w>0.0 {
        return soft_point_visibility(point+normal*0.025+direction*0.012,normal,u32(light));
    }
    var index=0u;
    if light>=0 {
        let delta=point-lights[u32(light)].position.xyz; let a=abs(delta); var face=0u;
        if a.x>=a.y&&a.x>=a.z {face=select(1u,0u,delta.x>=0.0);}
        else if a.y>=a.z {face=select(3u,2u,delta.y>=0.0);}
        else {face=select(5u,4u,delta.z>=0.0);}
        index=1u+u32(light)*6u+face;
    }
    let camera=shadow_views[index]; if camera.settings.z==0u {return 1.0;}
    let p=point+normal*0.025+direction*0.012-camera.origin.xyz;
    let z=dot(p,camera.forward.xyz); let near=camera.projection.z; let far=camera.projection.w;
    var xy=vec2<f32>(dot(p,camera.right.xyz),dot(p,camera.up.xyz))/camera.projection.xy;
    var depth=(z-near)/(far-near);
    if camera.settings.x==1u {
        if z<=near {return 1.0;}
        xy/=z; depth=far/(far-near)-near*far/((far-near)*z);
    }
    let uv=vec2<f32>(xy.x*0.5+0.5,0.5-xy.y*0.5);
    if any(uv<vec2<f32>(0.0))||any(uv>=vec2<f32>(1.0))||depth<0.0||depth>1.0{return 1.0;}
    var dimensions=textureDimensions(sun_depth);
    if light>=0 {dimensions=textureDimensions(point_depth);}
    let pixel=vec2<i32>(uv*vec2<f32>(dimensions));
    var visible=0.0;
    for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
        let at=clamp(pixel+vec2<i32>(x,y),vec2<i32>(0),vec2<i32>(dimensions)-1);
        var stored=1.0;
        if light<0 {stored=textureLoad(sun_depth,at,0,0);}
        else {stored=textureLoad(point_depth,at,i32(index-1u),0);}
        visible+=select(0.0,1.0,depth-0.00012<=stored);
    }}
    return 1.0-u.shadow.y*(1.0-visible/9.0);
}
fn decode_normal(p:u32)->vec3<f32> {
    let xy=vec2<f32>(f32(p&255u),f32((p>>8u)&255u))/255.0*2.0-1.0;
    var n=vec3<f32>(xy,1.0-abs(xy.x)-abs(xy.y));
    if n.z<0.0 {n=vec3<f32>((1.0-abs(xy.yx))*select(vec2<f32>(-1.0),vec2<f32>(1.0),xy>=vec2<f32>(0.0)),n.z);}
    return normalize(n);
}
fn world_at(pixel:vec2<f32>,height:f32)->vec3<f32> {
    let q=(pixel-u.projection.xy)/u.projection.z;
    return vec3<f32>(q.x*0.5-q.y-height*1.2247449,height,q.x*0.5+q.y+height*1.2247449);
}
fn blocked(c:Caster,origin:vec3<f32>,direction:vec3<f32>,distance:f32)->bool {
    let o=(origin-c.position_scale.xyz)/c.position_scale.w;let d=direction/c.position_scale.w;
    var begin=0.001;var end=distance;
    for(var axis=0u;axis<3u;axis++) {
        if abs(d[axis])<0.0000001 {if o[axis]<c.lo[axis]||o[axis]>c.hi[axis]{return false;}}
        else {let a=(c.lo[axis]-o[axis])/d[axis];let b=(c.hi[axis]-o[axis])/d[axis];begin=max(begin,min(a,b));end=min(end,max(a,b));}
    }
    if begin>=end{return false;}
    let cell_size=max((c.hi.xz-c.lo.xz)/vec2<f32>(c.grid.xy),vec2<f32>(0.00001));
    let entry=o+d*(begin+0.00001);
    var cell=vec2<i32>(floor((entry.xz-c.lo.xz)/cell_size));
    let step=vec2<i32>(select(vec2<f32>(-1.0),vec2<f32>(1.0),d.xz>=vec2<f32>(0.0)));
    var next=vec2<f32>(1e30);var delta=vec2<f32>(1e30);
    for(var axis=0u;axis<2u;axis++) {let v=d.xz[axis];if abs(v)>0.0000001 {let edge=cell[axis]+select(0,1,step[axis]>0);next[axis]=(c.lo.xz[axis]+f32(edge)*cell_size[axis]-o.xz[axis])/v;delta[axis]=cell_size[axis]/abs(v);}}
    for(var k=0u;k<258u;k++) {
        if any(cell<vec2<i32>(0))||any(cell>=vec2<i32>(c.grid.xy))||begin>end {break;}
        let stop=min(min(next.x,next.y),end);
        let layers=c.grid.w>>8u;
        for(var layer=0u;layer<layers;layer++) {
            let interval=columns[c.grid.z+(u32(cell.y)*c.grid.x+u32(cell.x))*layers+layer];
            let a=o.y+d.y*begin;let b=o.y+d.y*stop;
            if interval.x<=interval.y && min(a,b)<=interval.y+0.008 && max(a,b)>=interval.x-0.008{return true;}
        }
        if stop>=end{break;}
        if next.x<next.y{begin=next.x;next.x+=delta.x;cell.x+=step.x;}else{begin=next.y;next.y+=delta.y;cell.y+=step.y;}
    }
    return false;
}
fn visibility(point:vec3<f32>,normal:vec3<f32>,direction:vec3<f32>,distance:f32,light:i32)->f32 {
    if u.shadow.x<=0.0{return 1.0;}
    var visible=1.0;
    if u.shadow.w>0.5 {visible=mesh_visibility(point,normal,direction,light);}
    let origin=point+normal*0.045+direction*0.025;
    for(var i=0u;i<u.counts.w;i++) {
        let c=casters[i];
        if light>=0 && (c.grid.w&1u)!=0u {
            let source=(lights[u32(light)].position.xyz-c.position_scale.xyz)/c.position_scale.w;
            if all(source>=c.lo.xyz-0.01)&&all(source<=c.hi.xyz+0.01){continue;}
        }
        if blocked(c,origin,direction,min(distance,u.shadow.x)){return 1.0-u.shadow.y;}
    }
    return visible;
}
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=u.counts.x||id.y>=u.counts.y{return;}
    let index=id.y*u.counts.x+id.x;let rgba=unpack4x8unorm(albedo[index]);let surface=surfaces[index];
    let has_surface=surface.y!=0x7f800000u;
    let pixel=vec2<f32>(id.xy)+0.5;let height=select(0.0,bitcast<f32>(surface.y&0xffffff00u),has_surface);let point=world_at(pixel,height);
    var rgb=pow(rgba.rgb,vec3<f32>(2.2));
    if has_surface && (surface.y&255u)!=1u {
        let tint=(surface.y&255u)==2u;
        let normal=decode_normal(surface.x);let rough=f32((surface.x>>16u)&15u)/15.0;let metal=f32((surface.x>>20u)&15u)/15.0;let emission=f32(surface.x>>24u)/255.0;
        var illumination=u.ambient.xyz;var specular=vec3<f32>(0.0);
        let sun=select(max(dot(normal,u.sun.xyz),0.0),max(u.sun.y,0.0),tint);
        if sun>0.0&&any(u.sun_color.xyz>vec3<f32>(0.0)){illumination+=u.sun_color.xyz*sun*visibility(point,normal,u.sun.xyz,u.shadow.x,-1);}
        for(var i=0u;i<u.counts.z;i++) {
            let light=lights[i];let v=light.position.xyz-point;let d2=max(dot(v,v),0.01);let distance=sqrt(d2);
            if distance>=light.parameters.y {continue;}
            let direction=v/distance;let diffuse=select(max(dot(normal,direction),0.0),1.0,tint);if diffuse<=0.0{continue;}
            let linear_fall=clamp((light.parameters.y-distance)/max(light.parameters.y-light.parameters.z,0.001),0.0,1.0);
            let fall=select(linear_fall,linear_fall*linear_fall*(3.0-2.0*linear_fall),light.parameters.w>0.0);
            let radiance=light.color.xyz*(light.parameters.x*fall/d2)*visibility(point,normal,direction,distance,i32(i));
            illumination+=radiance*diffuse;
            let half_vector=direction+vec3<f32>(-0.61237246,0.5,0.61237246);
            if !tint && dot(half_vector,half_vector)>0.000001 {specular+=radiance*pow(max(dot(normal,normalize(half_vector)),0.0),2.0+(1.0-rough)*62.0)*(0.04+metal*0.6)*(1.0-rough*0.75);}
        }
        rgb=rgb*(illumination+emission)+specular;
    }
    textureStore(output,vec2<i32>(id.xy),vec4<f32>(pow(clamp(rgb,vec3<f32>(0.0),vec3<f32>(1.0)),vec3<f32>(1.0/2.2)),rgba.a));
}

// Instanced billboards cost fragments only where particles cover the canvas.
// They share the deferred geometry for occlusion, rather than looping over every
// particle for every pixel in the lighting pass.
struct ParticleVertex {
    @builtin(position) position:vec4<f32>,
    @location(0) uv:vec2<f32>,
    @location(1) color:vec4<f32>,
    @location(2) @interpolate(flat) depth:f32,
};
@vertex
fn particle_vertex(@builtin(vertex_index) vertex:u32,@builtin(instance_index) instance:u32)->ParticleVertex {
    let corners=array<vec2<f32>,6>(vec2<f32>(-1.0,-1.0),vec2<f32>(1.0,-1.0),vec2<f32>(1.0,1.0),vec2<f32>(-1.0,-1.0),vec2<f32>(1.0,1.0),vec2<f32>(-1.0,1.0));
    let particle=particles[instance];let p=particle.position_radius.xyz;
    let center=u.projection.xy+vec2<f32>((p.x+p.z)*u.projection.z,(p.z-p.x)*u.projection.z*0.5-p.y*u.projection.z*1.2247449);
    let radius=max(particle.position_radius.w*u.projection.z,0.75);
    let pixel=center+corners[vertex]*vec2<f32>(radius,radius*1.4);
    var out:ParticleVertex;
    out.position=vec4<f32>((pixel/vec2<f32>(u.counts.xy)*2.0-1.0)*vec2<f32>(1.0,-1.0),0.0,1.0);
    out.uv=corners[vertex];out.color=particle.color;
    out.depth=p.z-p.x+p.y*0.8164966;
    return out;
}
@fragment
fn particle_fragment(in:ParticleVertex)->@location(0) vec4<f32> {
    let coord=vec2<u32>(in.position.xy);let surface=surfaces[coord.y*u.counts.x+coord.x];
    if surface.y!=0x7f800000u {
        let point=world_at(in.position.xy,bitcast<f32>(surface.y&0xffffff00u));
        if point.z-point.x+point.y*0.8164966>in.depth+0.04 {discard;}
    }
    let alpha=in.color.a*pow(max(1.0-dot(in.uv,in.uv),0.0),1.5);
    return vec4<f32>(pow(in.color.rgb,vec3<f32>(1.0/2.2))*alpha,alpha);
}
