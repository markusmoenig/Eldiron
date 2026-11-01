//! WGSL backend. Reuses the bounded C-style expression/function emitter, lowers
//! its GLSL dialect through Naga, and validates before exposing shader source.
//! No Metal device, compiler, or platform API is involved.
use crate::{
    Error, Result,
    ir::{Channel, Program},
    metal,
};

#[derive(Clone, Debug)]
pub struct Shader {
    pub source: String,
    pub passes: Vec<String>,
    pub(crate) plan: metal::Shader,
}
impl Shader {
    pub fn parameters(&self) -> &[[f32; 4]] {
        self.plan.parameters()
    }
    pub fn parameter_bytes(&self) -> Vec<u8> {
        self.plan.parameter_bytes()
    }
    pub fn refresh(&mut self, program: &Program) -> Result<()> {
        // Source/layout compatibility is checked before any state changes.
        self.plan.refresh(program)
    }
}
pub fn compile(program: &Program, material: &str, channel: Channel) -> Result<Shader> {
    let plan = metal::compile(program, material, channel)?;
    let source = lower(&plan.c_functions, &plan.c_result, None)?;
    let passes = plan
        .passes
        .iter()
        .map(|p| lower(&p.c_functions, &p.c_result, Some(p.cache_index)))
        .collect::<Result<_>>()?;
    Ok(Shader {
        source,
        passes,
        plan,
    })
}

// The intermediate is deliberately limited to the engine's emitted dialect,
// not an arbitrary Metal-to-GLSL translator. Keep the native and WGSL frontends
// on the same typed algorithms, sample contexts, and parameter layout.
fn dialect(source: &str) -> String {
    let mut s = source
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("using namespace"))
        .collect::<Vec<_>>()
        .join("\n");
    s = s.replace(include_str!("../shaders/metal/cache.metal").trim_end(), "");
    s = s.replace(", constant float4* params", "")
        .replace(", texture2d_array<float,access::read_write> mz_cache, constant float4& mz_cache_region", "")
        .replace(", params, mz_cache, mz_cache_region", "")
        .replace(", params", "")
        .replace("mz_cache_lookup(mz_cache,mz_cache_region,", "mz_cache_lookup(")
        .replace("precise::", "")
        .replace("isnan(count)", "(count!=count)")
        .replace("all(cell.xy == float2(0))", "(cell.x==0.0f && cell.y==0.0f)");
    s = s.replace(
        "constant float2 MZ_BORDER_DIRECTIONS[16] = {",
        "const float2 MZ_BORDER_DIRECTIONS[16] = float2[16](",
    );
    if let Some(i) = s.find("const float2 MZ_BORDER_DIRECTIONS") {
        if let Some(j) = s[i..].find("\n};") {
            s.replace_range(i + j..i + j + 3, "\n);");
        }
    }
    while let Some(i) = s.find("return {") {
        let j = s[i..].find("};").expect("Generated sample constructor") + i;
        s.replace_range(j..j + 1, ")");
        s.replace_range(i..i + 8, "return MzSample(");
    }
    if let Some(i) = s.find("MzContext v={") {
        let j = s[i..].find("};").expect("Generated context constructor") + i;
        s.replace_range(j..j + 1, ")");
        s.replace_range(i..i + 13, "MzContext v=MzContext(");
    }
    // Replace identifiers only; never touch substrings in parameter names.
    let mut out = String::new();
    let mut at = 0;
    let bytes = s.as_bytes();
    while at < bytes.len() {
        let start = at;
        if bytes[at].is_ascii_alphabetic() || bytes[at] == b'_' {
            at += 1;
            while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_') {
                at += 1;
            }
            let word = &s[start..at];
            out.push_str(match word {
                "float2" => "vec2",
                "float3" => "vec3",
                "float4" => "vec4",
                "uint2" => "uvec2",
                "uint3" => "uvec3",
                "uint4" => "uvec4",
                "int2" => "ivec2",
                "int3" => "ivec3",
                "int4" => "ivec4",
                "atan2" => "atan",
                "constant" => "const",
                "input" => "mz_incoming",
                "layout" => "mz_layout",
                "output" => "mz_output",
                "sample" => "mz_sample",
                "filter" => "mz_filter",
                "active" => "mz_active",
                "common" => "mz_common",
                "partition" => "mz_partition",
                "resource" => "mz_resource",
                _ => word,
            });
        } else {
            // All shader syntax is ASCII, but preserve UTF-8 source comments too.
            let ch = s[at..].chars().next().unwrap();
            out.push(ch);
            at += ch.len_utf8();
        }
    }
    out
}
const HEADER: &str = r#"#version 450
layout(local_size_x=8, local_size_y=8) in;
layout(set=0,binding=0,std430) buffer OutputBuffer { vec4 pixels[]; };
layout(set=0,binding=1,std430) readonly buffer RequestBuffer { vec4 cfg[]; };
layout(set=0,binding=2,std430) readonly buffer ParameterBuffer { vec4 params[]; };
layout(set=0,binding=3,std430) buffer CacheBuffer { vec4 cache_pixels[]; };
"#;
const CACHE: &str = r#"
uint mz_mip_offset(uint level) {
    uvec2 size=uvec2(cfg[0].zw);uint offset=0u;
    for(uint i=0u;i<level;i++){offset+=size.x*size.y;size=max(size/2u,uvec2(1u));}
    return offset;
}
vec4 mz_cache_level(vec2 q,uint layer,uint level) {
    uvec2 size=max(uvec2(cfg[0].zw)>>level,uvec2(1u));
    vec2 p=q*vec2(size)-0.5;ivec2 lo=ivec2(floor(p));vec2 f=fract(p);
    vec4 v[4];for(uint i=0u;i<4u;i++) {
        uvec2 xy=uvec2(clamp(lo+ivec2(i&1u,i>>1u),ivec2(0),ivec2(size)-1));
        v[i]=cache_pixels[layer*uint(cfg[2].z)+mz_mip_offset(level)+xy.y*size.x+xy.x];
    }
    return mix(mix(v[0],v[1],f.x),mix(v[2],v[3],f.x),f.y);
}
MzSample mz_cache_lookup(vec2 uv,float aa,uint layer) {
    vec2 q=(uv-cfg[3].xy)/cfg[3].zw;
    float pixel=max(abs(cfg[3].z)/cfg[0].z,abs(cfg[3].w)/cfg[0].w);
    float lod=clamp(log2(max(aa/max(pixel,0.000001),1.0)),0.0,cfg[2].w-1.0);
    uint lo=uint(floor(lod)),hi=min(lo+1u,uint(cfg[2].w)-1u);float f=fract(lod);
    vec4 c=mix(mz_cache_level(q,layer,lo),mz_cache_level(q,layer,hi),f);
    float d=mix(mz_cache_level(q,layer+1u,lo).x,mz_cache_level(q,layer+1u,hi).x,f);
    return MzSample(vec4(c.rgb/max(c.a,0.000001),c.a),d);
}
"#;
fn lower(functions: &str, result: &str, capture: Option<usize>) -> Result<String> {
    let mut body = dialect(functions);
    // Insert the cache helpers after the sample structure, before their callers.
    if let Some(i) = body.find("struct MzSample") {
        let end = i + body[i..].find("};").unwrap() + 2;
        body.insert_str(end, CACHE);
    }
    let result = dialect(result);
    let store = if let Some(index) = capture {
        format!(
            "MzSample v={result};uint at=gid.y*size.x+gid.x;uint stride=uint(cfg[2].z);cache_pixels[{}u*stride+at]=vec4(v.color.rgb*v.color.a,v.color.a);cache_pixels[{}u*stride+at]=vec4(v.distance);",
            index * 2,
            index * 2 + 1
        )
    } else {
        format!("pixels[gid.y*size.x+gid.x]={result};")
    };
    let context = if functions.is_empty() {
        String::new()
    } else {
        "MzContext ctx=mz_context(vec3(uv,0),vec3(0),vec3(0),vec3(0));ctx.valid=0.0;ctx.time=cfg[4].x;".into()
    };
    let glsl = format!(
        "{HEADER}\n{body}\nvoid main(){{uvec2 gid=gl_GlobalInvocationID.xy;uvec2 size=uvec2(cfg[0].xy);if(any(greaterThanEqual(gid,size)))return;vec2 f=(vec2(gid)+0.5)/vec2(size);vec2 uv=cfg[1].xy+f.x*cfg[1].zw+f.y*cfg[2].xy;float aa=max(length(cfg[1].zw)/float(size.x),length(cfg[2].xy)/float(size.y));{context}{store}}}"
    );
    let mut parser = naga::front::glsl::Frontend::default();
    let module = parser
        .parse(
            &naga::front::glsl::Options::from(naga::ShaderStage::Compute),
            &glsl,
        )
        .map_err(|e| Error(format!("WGSL lowering: {}", e.emit_to_string(&glsl))))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| Error(format!("WGSL validation: {}", e.emit_to_string(&glsl))))?;
    naga::back::wgsl::write_string(
        &module,
        &info,
        naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
    )
    .map_err(|e| Error(format!("WGSL emission: {e}")))
}
