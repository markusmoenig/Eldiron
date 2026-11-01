//! Metal source backend. Produces one bounded compute entry point per request.
use crate::{
    Error, Result,
    cpu::PreviewMode,
    ir::{Binding, Channel, NodeId, Program, Reference},
    metadata::{Kernel, ValueType},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

/// Selected built-in definition, rather than the complete generated material.
pub fn tile_source(program: &Program, material: &str, row: &str, tile: usize) -> Result<String> {
    let id = program.tile(material, row, tile)?;
    Ok(format!(
        "{{\"language\":\"MaterialZ Tile Language 1\",\"text\":{}}}",
        crate::json::string(&program.nodes[id.0].code)
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSlot {
    pub node: NodeId,
    pub row: String,
    pub tile: usize,
    pub parameter: String,
    pub kind: ValueType,
    /// Index of a 16-byte float4 in Metal buffer(2).
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct Shader {
    #[cfg(feature = "wgpu")]
    pub(crate) c_functions: String,
    #[cfg(feature = "wgpu")]
    pub(crate) c_result: String,
    pub source: String,
    pub passes: Vec<ImagePass>,
    pub pixel_padding: f32,
    pub uv_padding: f32,
    pub domains: Vec<[f32; 6]>,
    pub fixed_domains: Vec<[f32; 4]>,
    pub domain_fixed: Vec<bool>,
    pub material: String,
    pub channel: Channel,
    pub slots: Vec<ParameterSlot>,
    values: Vec<[f32; 4]>,
    pub preview: Option<TilePreview>,
}
#[derive(Debug, Clone)]
pub struct ImagePass {
    #[cfg(feature = "wgpu")]
    pub(crate) c_functions: String,
    #[cfg(feature = "wgpu")]
    pub(crate) c_result: String,
    #[cfg(feature = "wgpu")]
    pub(crate) cache_index: usize,
    pub source: String,
    pub parameter_start: usize,
    pub parameter_count: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilePreview {
    pub row: String,
    pub tile: usize,
    pub mode: PreviewMode,
}
impl Shader {
    pub fn layout_json(&self) -> String {
        use crate::json::string as j;
        let execution = format!(
            "{{\"passes\":{},\"pixel_padding\":{},\"uv_padding\":{}}}",
            self.passes.len(),
            self.pixel_padding,
            self.uv_padding
        );
        let slots = self.slots.iter().map(|p| format!("{{\"row\":{},\"tile\":{},\"parameter\":{},\"type\":{},\"index\":{},\"byte_offset\":{},\"width\":{}}}",j(&p.row),p.tile,j(&p.parameter),j(p.kind.name()),p.index,p.index*16,p.kind.width())).collect::<Vec<_>>().join(",");
        format!(
            "{{\"version\":1,\"execution\":{execution},\"material\":{},\"channel\":{},\"buffer_index\":2,\"stride_bytes\":16,\"float_count\":{},\"preview\":{},\"slots\":[{}]}}",
            j(&self.material),
            j(self.channel.name()),
            self.values.len() * 4,
            self.preview
                .as_ref()
                .map(|p| format!(
                    "{{\"row\":{},\"tile\":{},\"mode\":{}}}",
                    j(&p.row),
                    p.tile,
                    j(p.mode.name())
                ))
                .unwrap_or_else(|| "null".into()),
            slots
        )
    }
    pub fn pass_key(&self, index: usize) -> Result<String> {
        let pass = self
            .passes
            .get(index)
            .ok_or_else(|| Error("Unknown execution pass".into()))?;
        let mut key = String::new();
        // Prior dependencies are included; edits after this pass do not invalidate it.
        for p in &self.passes[..=index] {
            key.push_str(&p.source);
            key.push_str(&format!(
                "{:?}",
                &self.values[p.parameter_start..p.parameter_start + p.parameter_count]
            ));
        }
        key.push_str(&format!(
            "{}:{}:{}",
            pass.parameter_start, self.pixel_padding, self.uv_padding
        ));
        Ok(key)
    }
    pub fn parameters(&self) -> &[[f32; 4]] {
        &self.values
    }
    pub fn parameter_bytes(&self) -> Vec<u8> {
        self.values
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }
    /// Refresh constants without changing the generated source or layout.
    /// A changed graph returns an error and leaves the old buffer untouched.
    pub fn refresh(&mut self, program: &Program) -> Result<()> {
        let next = if let Some(p) = &self.preview {
            compile_tile_preview(program, &self.material, &p.row, p.tile, p.mode)?
        } else {
            compile(program, &self.material, self.channel)?
        };
        if next.source != self.source || next.slots != self.slots {
            return Err(Error(
                "Shader structure changed; compile a new shader".into(),
            ));
        }
        if self.passes.iter().map(|p| &p.source).collect::<Vec<_>>()
            != next.passes.iter().map(|p| &p.source).collect::<Vec<_>>()
        {
            return Err(Error("Execution plan changed; compile a new shader".into()));
        }
        self.values = next.values;
        self.pixel_padding = next.pixel_padding;
        self.uv_padding = next.uv_padding;
        self.domains = next.domains;
        self.fixed_domains = next.fixed_domains;
        self.domain_fixed = next.domain_fixed;
        Ok(())
    }
}

fn literal(x: f32) -> String {
    format!("{x:.9e}f")
}
fn constant(v: &[f32]) -> String {
    if v.len() == 1 {
        literal(v[0])
    } else {
        format!(
            "float{}({})",
            v.len(),
            v.iter().map(|x| literal(*x)).collect::<Vec<_>>().join(", ")
        )
    }
}
fn call(id: NodeId, uv: &str, aa: &str, buffered: bool) -> String {
    let params = if buffered { ", params" } else { "" };
    format!(
        "mz_node_{}({uv}, {aa}, mz_with_uv(ctx, {uv}){params})",
        id.0
    )
}
fn select(r: &Reference, sample: &str) -> String {
    let value = if r.distance {
        if r.components.len() > 1 {
            format!("float{}({sample}.distance)", r.components.len())
        } else {
            format!("{sample}.distance")
        }
    } else {
        let swizzle: String = r
            .components
            .iter()
            .map(|i| ['x', 'y', 'z', 'w'][*i])
            .collect();
        format!("{sample}.color.{swizzle}")
    };
    if let Some((lo, hi)) = r.mapping {
        format!("({} + {value} * {})", literal(lo), literal(hi - lo))
    } else {
        value
    }
}
fn reference(r: &Reference, uv: &str, buffered: bool) -> String {
    select(r, &call(r.node, uv, "aa", buffered))
}
fn binding(b: &Binding, uv: &str, buffered: bool) -> String {
    match b {
        Binding::Constant(v) => constant(v),
        Binding::From(r) => reference(r, uv, buffered),
    }
}

/// Standalone diagnostic source with constants embedded.
pub fn generate(program: &Program, material: &str, channel: Channel) -> Result<String> {
    if let Some((_, r)) = program
        .material(material)?
        .outputs
        .iter()
        .find(|(c, _)| *c == channel)
        && !crate::execution::build(program, r.node)?.inputs.is_empty()
    {
        return Err(Error("This output requires an execution plan; use metal::compile and its prerequisite image passes".into()));
    }
    Ok(generate_internal(program, material, channel, false, None)?.source)
}

/// Live source plus an explicit float4 parameter-buffer layout.
pub fn compile(program: &Program, material: &str, channel: Channel) -> Result<Shader> {
    compile_execution(program, material, channel, None)
}

/// A shared RGBA thumbnail path for every tile, independent of material outputs.
pub fn compile_tile_preview(
    program: &Program,
    material: &str,
    row: &str,
    tile: usize,
    mode: PreviewMode,
) -> Result<Shader> {
    compile_execution(
        program,
        material,
        Channel::BaseColor,
        Some(TilePreview {
            row: row.into(),
            tile,
            mode,
        }),
    )
}
fn compile_execution(
    program: &Program,
    material: &str,
    channel: Channel,
    preview: Option<TilePreview>,
) -> Result<Shader> {
    let prepared = if let Some(target) = &preview {
        let tiling = program.material(material)?.tiling;
        if target.mode == PreviewMode::Image && tiling.mode != crate::periodic::Mode::Raw {
            let mut copy = program.clone();
            let source = copy.tile(material, &target.row, target.tile)?;
            let id = crate::periodic::append(
                &mut copy,
                Reference {
                    field: None,
                    node: source,
                    components: vec![0, 1, 2],
                    mapping: None,
                    distance: false,
                },
                tiling,
                &target.row,
            )?;
            copy.nodes[id.0].tile_index = target.tile;
            let m = copy
                .materials
                .iter_mut()
                .find(|m| m.id == material)
                .unwrap();
            m.tiling.mode = crate::periodic::Mode::Raw;
            m.rows.iter_mut().find(|(r, _)| r == &target.row).unwrap().1 = id;
            Some(copy)
        } else {
            None
        }
    } else {
        None
    };
    let program = prepared.as_ref().unwrap_or(program);
    let root = if let Some(p) = &preview {
        Some(program.tile(material, &p.row, p.tile)?)
    } else {
        program
            .material(material)?
            .outputs
            .iter()
            .find(|(c, _)| *c == channel)
            .map(|(_, r)| r.node)
    };
    let plan = if let Some(root) = root {
        crate::execution::build(program, root)?
    } else {
        crate::execution::Plan::default()
    };
    if plan.inputs.is_empty() {
        return generate_internal(program, material, channel, true, preview);
    }
    let mut cuts = HashMap::new();
    let mut values = Vec::new();
    let mut slots = Vec::new();
    let mut passes = Vec::new();
    fn offset(source: String, by: usize) -> String {
        let mut output = String::new();
        let mut rest = source.as_str();
        while let Some(i) = rest.find("params[") {
            output.push_str(&rest[..i + 7]);
            rest = &rest[i + 7..];
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            if digits > 0 {
                let n = rest[..digits].parse::<usize>().expect("Parameter offset");
                output.push_str(&(n + by).to_string());
                rest = &rest[digits..];
            } else {
                output.push_str(rest);
                rest = "";
                break;
            }
        }
        output.push_str(rest);
        output
    }
    for (i, id) in plan.inputs.iter().enumerate() {
        let part = generate_part(
            program,
            material,
            channel,
            true,
            None,
            Part {
                root: Some(*id),
                cached: &cuts,
                capture: Some(i),
            },
        )?;
        let start = values.len();
        let count = part.values.len();
        passes.push(ImagePass {
            #[cfg(feature = "wgpu")]
            c_functions: offset(part.c_functions, start),
            #[cfg(feature = "wgpu")]
            c_result: offset(part.c_result, start),
            #[cfg(feature = "wgpu")]
            cache_index: i,
            source: offset(part.source, start),
            parameter_start: start,
            parameter_count: count,
        });
        slots.extend(part.slots.into_iter().map(|mut p| {
            p.index += start;
            p
        }));
        values.extend(part.values);
        cuts.insert(*id, i);
    }
    let mut result = generate_part(
        program,
        material,
        channel,
        true,
        preview,
        Part {
            root: None,
            cached: &cuts,
            capture: None,
        },
    )?;
    let start = values.len();
    result.source = offset(result.source, start);
    #[cfg(feature = "wgpu")]
    {
        result.c_functions = offset(result.c_functions, start);
        result.c_result = offset(result.c_result, start);
    }
    slots.extend(result.slots.into_iter().map(|mut p| {
        p.index += start;
        p
    }));
    values.extend(result.values);
    result.values = values;
    result.slots = slots;
    result.passes = passes;
    result.pixel_padding = plan.pixel_padding;
    result.uv_padding = plan.uv_padding;
    result.domains = plan.domains;
    result.fixed_domains = plan.fixed_domains;
    result.domain_fixed = plan.domain_fixed;
    Ok(result)
}

fn generate_internal(
    program: &Program,
    material: &str,
    channel: Channel,
    buffered: bool,
    preview: Option<TilePreview>,
) -> Result<Shader> {
    generate_part(
        program,
        material,
        channel,
        buffered,
        preview,
        Part {
            root: None,
            cached: &HashMap::new(),
            capture: None,
        },
    )
}
struct Part<'a> {
    root: Option<NodeId>,
    cached: &'a HashMap<NodeId, usize>,
    capture: Option<usize>,
}
fn generate_part(
    program: &Program,
    material: &str,
    channel: Channel,
    buffered: bool,
    preview: Option<TilePreview>,
    part: Part<'_>,
) -> Result<Shader> {
    let Part {
        root,
        cached,
        capture,
    } = part;
    let periodic_program = if let Some(target) = &preview {
        if target.mode == PreviewMode::Image
            && program.material(material)?.tiling.mode != crate::periodic::Mode::Raw
        {
            let mut copy = program.clone();
            let source = copy.tile(material, &target.row, target.tile)?;
            let tiling = copy.material(material)?.tiling;
            let id = crate::periodic::append(
                &mut copy,
                Reference {
                    field: None,
                    node: source,
                    components: vec![0, 1, 2],
                    mapping: None,
                    distance: false,
                },
                tiling,
                &target.row,
            )?;
            copy.nodes[id.0].tile_index = target.tile;
            copy.materials
                .iter_mut()
                .find(|m| m.id == material)
                .unwrap()
                .rows
                .iter_mut()
                .find(|(r, _)| r == &target.row)
                .unwrap()
                .1 = id;
            Some(copy)
        } else {
            None
        }
    } else {
        None
    };
    let program = periodic_program.as_ref().unwrap_or(program);
    let m = program.material(material)?;
    if root.is_none() && preview.is_none() && !m.outputs.iter().any(|(c, _)| *c == channel) {
        let value = m.default_value(channel)?;
        let mut packed = [0.; 4];
        packed[..value.len()].copy_from_slice(value);
        let expression = if buffered {
            format!("params[0].{}", if value.len() == 1 { "x" } else { "xyz" })
        } else {
            constant(value)
        };
        let rgba = if value.len() == 1 {
            format!("float4(float3({expression}),1.0f)")
        } else {
            format!("float4({expression},1.0f)")
        };
        let args = if buffered {
            ", constant float4* params [[buffer(2)]]"
        } else {
            ""
        };
        let mut source = format!(
            "// Material default: {}\n#include <metal_stdlib>\nusing namespace metal;\n#pragma clang fp contract(off)\nkernel void material_preview(device float4* pixels [[buffer(0)]], constant uint2& dimensions [[buffer(1)]], uint2 gid [[thread_position_in_grid]]{args}) {{if(any(gid>=dimensions))return;pixels[gid.y*dimensions.x+gid.x]={rgba};}}\n",
            channel.name()
        );
        writeln!(source,"kernel void material_preview_region(device float4* pixels [[buffer(0)]], constant uint2& dimensions [[buffer(1)]], constant float4& region [[buffer(3)]], uint2 gid [[thread_position_in_grid]]{args}) {{if(any(gid>=dimensions))return;pixels[gid.y*dimensions.x+gid.x]={rgba};}}").unwrap();
        return Ok(Shader {
            #[cfg(feature = "wgpu")]
            c_functions: String::new(),
            #[cfg(feature = "wgpu")]
            c_result: rgba,
            source,
            passes: vec![],
            domains: vec![],
            fixed_domains: vec![],
            domain_fixed: vec![],
            pixel_padding: 0.,
            uv_padding: 0.,
            material: material.into(),
            channel,
            slots: vec![],
            values: if buffered { vec![packed] } else { vec![] },
            preview: None,
        });
    }

    let output = if let Some(node) = root {
        Reference {
            field: None,
            node,
            components: vec![0, 1, 2],
            mapping: None,
            distance: false,
        }
    } else if let Some(p) = &preview {
        let node = program.tile(material, &p.row, p.tile)?;
        if p.mode != PreviewMode::Image && !program.nodes[node.0].has_distance {
            return Err(Error(
                "This tile has no valid distance preview; use image mode".into(),
            ));
        }
        Reference {
            field: None,
            node,
            components: vec![0, 1, 2],
            mapping: None,
            distance: false,
        }
    } else {
        m.outputs
            .iter()
            .find(|(c, _)| *c == channel)
            .ok_or_else(|| Error(format!("No {} output on {material:?}", channel.name())))?
            .1
            .clone()
    };
    let mut reachable = HashSet::new();
    fn mark(
        program: &Program,
        id: NodeId,
        seen: &mut HashSet<NodeId>,
        cached: &HashMap<NodeId, usize>,
    ) {
        if seen.insert(id) && !cached.contains_key(&id) {
            for d in program.nodes[id.0].dependencies() {
                mark(program, d, seen, cached);
            }
        }
    }
    mark(program, output.node, &mut reachable, cached);
    let mut s = String::from(
        "// Generated by denrim-materials, project format 1.\n// Compile with fast math disabled for reference comparisons.\n#include <metal_stdlib>\nusing namespace metal;\n#pragma clang fp contract(off)\n\n",
    );
    s.push_str(include_str!("../shaders/metal/library.metal"));
    s.push_str(include_str!("../shaders/metal/sdf.metal"));
    let partitioned = !cached.is_empty() || capture.is_some();
    if partitioned {
        s.push_str(include_str!("../shaders/metal/cache.metal"));
    }
    let mut slots = vec![];
    let mut values = vec![];
    for id in &program.order {
        if !reachable.contains(id) {
            continue;
        }
        let n = &program.nodes[id.0];
        let signature = if buffered {
            "float2 uv, float aa, MzContext ctx, constant float4* params"
        } else {
            "float2 uv, float aa, MzContext ctx"
        };
        if let Some(index) = cached.get(id) {
            writeln!(s,"\nMzSample mz_node_{}({signature}) {{return mz_cache_lookup(mz_cache,mz_cache_region,uv,aa,{}u);}}",id.0,index*2).unwrap();
            continue;
        }
        if let Some(input) = &n.input {
            let params = if buffered { ", params" } else { "" };
            writeln!(s, "\nfloat4 mz_rgba_{}({signature}) {{ MzSample v = mz_node_{}(uv, aa, ctx{params}); return float4({}, v.color.a); }}", id.0, input.node.0, select(input, "v")).unwrap();
        }
        if let Some(raster) = n.algorithm.raster.as_ref().filter(|_| !buffered) {
            writeln!(s, "constant float4 mz_bitmap_{}[] = {{", id.0).unwrap();
            for pixel in &raster.pixels {
                writeln!(
                    s,
                    "float4({},{},{},{}),",
                    pixel[0], pixel[1], pixel[2], pixel[3]
                )
                .unwrap();
            }
            s.push_str("};\n");
        }
        writeln!(s, "\nMzSample mz_node_{}({signature}) {{", id.0).unwrap();
        if let Some(c) = &n.coordinates {
            let kind = if c.components.len() == 3 {
                "float3"
            } else {
                "float2"
            };
            writeln!(
                s,
                "    {kind} mz_domain = {};",
                reference(c, "uv", buffered)
            )
            .unwrap();
            writeln!(
                s,
                "    {kind} mz_domain_x = {};",
                reference(c, "uv+float2(aa,0)", buffered)
            )
            .unwrap();
            writeln!(
                s,
                "    {kind} mz_domain_y = {};",
                reference(c, "uv+float2(0,aa)", buffered)
            )
            .unwrap();
            s.push_str("    aa=max(max(length(mz_domain_x-mz_domain),length(mz_domain_y-mz_domain)),0.000001f);\n    uv=mz_domain.xy;ctx=mz_with_uv(ctx,uv);\n");
            if c.components.len() == 3 {
                s.push_str("    ctx.position.z=mz_domain.z;\n")
            }
        }
        if let Some(raster) = &n.algorithm.raster {
            let index = format!(
                "uint(floor(fract(uv.y)*{}.0f))*{}u + uint(floor(fract(uv.x)*{}.0f))",
                raster.height, raster.width, raster.width
            );
            let load = if buffered {
                let offset = values.len();
                values.extend_from_slice(&raster.pixels);
                format!("params[{offset} + {index}]")
            } else {
                format!("mz_bitmap_{}[{index}]", id.0)
            };
            writeln!(s,"    float4 mz_pixel = {load};\n    MzSample result = mz_image(mz_pixel.rgb);\n    result.color = mz_pixel;\n    return result;\n}}").unwrap();
            continue;
        }
        let mut parameter_sources = HashMap::new();
        for p in &n.parameters {
            if let Binding::From(reference) = &p.binding {
                *parameter_sources.entry(reference.node).or_insert(0usize) += 1;
            }
        }
        let mut shared = HashSet::new();
        for p in &n.parameters {
            if let Binding::From(reference) = &p.binding
                && parameter_sources[&reference.node] > 1
                && shared.insert(reference.node)
            {
                writeln!(
                    s,
                    "    MzSample mz_parameter_{} = {};",
                    reference.node.0,
                    call(reference.node, "uv", "aa", buffered)
                )
                .unwrap();
            }
            let kind = match p.kind {
                ValueType::Float => "float",
                ValueType::Vec2 => "float2",
                ValueType::Vec3 => "float3",
                ValueType::Color => "float4",
                ValueType::Bool => "float",
            };
            // Parameter IDs come from validated MaterialZ tile declarations.
            let expression = if let Binding::From(reference) = &p.binding
                && parameter_sources[&reference.node] > 1
            {
                select(reference, &format!("mz_parameter_{}", reference.node.0))
            } else if buffered {
                if let Binding::Constant(v) = &p.binding {
                    let index = values.len();
                    let mut packed = [0.; 4];
                    packed[..v.len()].copy_from_slice(v);
                    values.push(packed);
                    slots.push(ParameterSlot {
                        node: *id,
                        row: n.row_id.clone(),
                        tile: n.tile_index,
                        parameter: p.id.clone(),
                        kind: p.kind,
                        index,
                    });
                    let swizzle = match p.kind.width() {
                        1 => "x",
                        2 => "xy",
                        3 => "xyz",
                        _ => "xyzw",
                    };
                    let load = format!("params[{index}].{swizzle}");
                    load
                } else {
                    binding(&p.binding, "uv", true)
                }
            } else {
                binding(&p.binding, "uv", false)
            };
            let expression = if p.choice_count > 0 {
                format!("clamp(round({expression}),0.0f,{}.0f)", p.choice_count - 1)
            } else {
                expression
            };
            writeln!(s, "    {kind} {} = {expression};", p.id).unwrap();
        }
        if n.algorithm.primitive && n.input.is_none() {
            s.push_str("    MzSample result = mz_image(float3(1.0f));\n");
        } else if !n.algorithm.primitive {
            s.push_str("    MzSample result = mz_image(float3(1.0f));\n    float4 mz_input = float4(1.0f);\n");
            if n.kernel == Kernel::Transform && n.code == crate::tilelang::source(Kernel::Transform)
            {
                s.push_str("    ctx=mz_transform_frame(ctx,globalScale,localScale,offset,rot);\n");
            }
            let input_code = |domain: &str| {
                if let Some(input) = &n.input {
                    let aa = if n.kernel == Kernel::Transform
                        && n.code == crate::tilelang::source(Kernel::Transform)
                    {
                        "aa / mz_nonzero(abs(globalScale) * min(abs(localScale.x),abs(localScale.y)))"
                    } else {
                        "aa"
                    };
                    format!(
                        "    result = {};\n    mz_input = float4({}, result.color.a);\n",
                        call(input.node, domain, aa, buffered),
                        select(input, "result")
                    )
                } else {
                    String::new()
                }
            };
            s.push_str(&n.algorithm.emit(input_code, |args| {
                if n.input.is_some() {
                    let params=if buffered {", params"}else{""};
                    if args.len()==8 {
                        format!("mz_rgba_{}({}, {}, mz_scope_frame(ctx, {}, {}, {}, {}, {}, {}){params})",id.0,args[0],args[1],args[0],args[2],args[3],args[4],args[5],args[6])
                    } else if args.len()==7 {
                        format!("mz_rgba_{}({}, aa, mz_resample_context(ctx, {}, {}, {}, {}, {}, {}){params})",id.0,args[0],args[1],args[4],args[5],args[6],args[2],args[3])
                    } else if args.len()==4 {
                        format!("mz_rgba_{}({}, {}, mz_scope(ctx, {}, {}, {}){params})",id.0,args[0],args[1],args[0],args[2],args[3])
                    } else {
                        format!("mz_rgba_{}({}, aa, mz_with_uv(ctx, {}){params})",id.0,args[0],args[0])
                    }
                } else {
                    "float4(1.0f)".into()
                }
            }));
            if n.algorithm.shape.is_some() {
                let mode = match n.combine {
                    crate::format::Combine::Replace => 0,
                    crate::format::Combine::Union => 1,
                    crate::format::Combine::Subtract => 2,
                    crate::format::Combine::Intersect => 3,
                    crate::format::Combine::SmoothUnion => 4,
                };
                if mode == 0 {
                    s.push_str("    result = mz_shape(mz_shape_distance, aa);\n")
                } else {
                    writeln!(s,"    result = mz_shape(mz_combine(result.distance, mz_shape_distance, {mode}, smoothness), aa);").unwrap()
                }
            } else if n.kernel == Kernel::Transform && n.has_distance {
                s.push_str("    result.distance *= abs(globalScale * localScale.x);\n")
            }
        } else {
            if let Some(input) = &n.input
                && n.kernel != Kernel::Transform
            {
                writeln!(
                    s,
                    "    MzSample inputSample = {};",
                    call(input.node, "uv", "aa", buffered)
                )
                .unwrap();
            }
            let expr: String = match n.kernel {
                Kernel::Border => {
                    writeln!(
                        s,
                        "    inputSample.color.rgb = {};",
                        select(n.input.as_ref().unwrap(), "inputSample")
                    )
                    .unwrap();
                    if program.nodes[n.input.as_ref().unwrap().node.0].has_distance {
                        s.push_str("    float outer = mz_coverage(inputSample.distance, aa);\n    float band = clamp(outer-mz_coverage(inputSample.distance+max(width,0.0f),aa),0.0f,1.0f);\n");
                    } else {
                        s.push_str("    float outer = clamp(inputSample.color.a,0.0f,1.0f);\n    float eroded = outer;\n    if (width>0.0f) {\n        for (int ring=1;ring<=3;ring++) {\n            for (int i=0;i<16;i++) {\n                float radius = width*float(ring)/3.0f;\n                float2 sampleUV = uv+MZ_BORDER_DIRECTIONS[i]*radius;\n");
                        writeln!(
                            s,
                            "                eroded = min(eroded,clamp({}.color.a,0.0f,1.0f));",
                            call(n.input.as_ref().unwrap().node, "sampleUV", "aa", buffered)
                        )
                        .unwrap();
                        s.push_str("            }\n        }\n    }\n    float band = clamp(outer-eroded,0.0f,1.0f);\n");
                    }
                    "mz_border(inputSample, color, outer, band)".into()
                }
                _ => unreachable!("Only Border uses the bounded sampling primitive"),
            };
            if !expr.is_empty() {
                writeln!(s, "    MzSample result = {expr};").unwrap();
            }
        }
        if !n.has_distance {
            s.push_str("    result.distance = 0.0f;\n");
        }
        s.push_str("    return result;\n}\n");
    }
    let result = if capture.is_some() {
        format!("{}.color", call(output.node, "uv", "aa", buffered))
    } else if let Some(p) = &preview {
        writeln!(s, "\n// Tile preview: {}", p.mode.name()).unwrap();
        let sample = call(output.node, "uv", "aa", buffered);
        match p.mode {
            PreviewMode::Image => format!("{sample}.color"),
            PreviewMode::Shape => {
                format!("float4(float3(mz_coverage({sample}.distance, aa)), 1.0f)")
            }
            PreviewMode::Distance => {
                format!("float4(float3(clamp(0.5f+{sample}.distance*4.0f,0.0f,1.0f)),1.0f)")
            }
        }
    } else {
        let value = reference(&output, "uv", buffered);
        if channel.width() == 1 {
            format!("float4(float3({value}), 1.0f)")
        } else {
            format!("float4({value}, 1.0f)")
        }
    };
    if partitioned {
        s=s.replace("MzContext ctx, constant float4* params)","MzContext ctx, constant float4* params, texture2d_array<float,access::read_write> mz_cache, constant float4& mz_cache_region)");
        s = s.replace(", params)", ", params, mz_cache, mz_cache_region)");
    }
    let result = if partitioned {
        result.replace(", params)", ", params, mz_cache, mz_cache_region)")
    } else {
        result
    };
    if let Some(index) = capture {
        let sample = call(output.node, "uv", "aa", true)
            .replace(", params)", ", params, mz_cache, mz_cache_region)");
        writeln!(s,"\nkernel void material_cache(texture2d_array<float,access::read_write> mz_cache [[texture(0)]], constant uint2& dimensions [[buffer(1)]], constant float4* params [[buffer(2)]], constant float4& region [[buffer(3)]], constant float4& mz_cache_region [[buffer(4)]],constant float& sample_time [[buffer(5)]],uint2 gid [[thread_position_in_grid]]) {{if(any(gid>=dimensions))return;float2 uv=region.xy+(float2(gid)+0.5f)/float2(dimensions)*region.zw;float aa=max(abs(region.z)/float(dimensions.x),abs(region.w)/float(dimensions.y));MzContext ctx=mz_context(float3(uv,0),float3(0),float3(0),float3(0));ctx.valid=0.0f;ctx.time=sample_time;MzSample v={sample};mz_cache.write(float4(v.color.rgb*v.color.a,v.color.a),gid,{}u);mz_cache.write(float4(v.distance),gid,{}u);}}",index*2,index*2+1).unwrap();
        return Ok(Shader {
            #[cfg(feature = "wgpu")]
            c_functions: s.split("\nkernel void").next().unwrap().into(),
            #[cfg(feature = "wgpu")]
            c_result: sample,
            source: s,
            passes: vec![],
            domains: vec![],
            fixed_domains: vec![],
            domain_fixed: vec![],
            pixel_padding: 0.,
            uv_padding: 0.,
            material: material.into(),
            channel,
            slots,
            values,
            preview,
        });
    }
    #[cfg(feature = "wgpu")]
    let c_functions = s.clone();
    #[cfg(feature = "wgpu")]
    let c_result = result.clone();
    let parameter_arg = if buffered {
        ", constant float4* params [[buffer(2)]]"
    } else {
        ""
    };
    let cache_args = if partitioned {
        ", texture2d_array<float,access::read_write> mz_cache [[texture(0)]], constant float4& mz_cache_region [[buffer(4)]]"
    } else {
        ""
    };
    writeln!(s,"\nkernel void material_preview(device float4* pixels [[buffer(0)]],\n    constant uint2& dimensions [[buffer(1)]], uint2 gid [[thread_position_in_grid]]{parameter_arg}{cache_args}, constant float& sample_time [[buffer(5)]]) {{\n    if (any(gid >= dimensions)) return;\n    float2 uv = (float2(gid) + 0.5f) / float2(dimensions) - 0.5f;\n    float aa = 1.0f / float(min(dimensions.x, dimensions.y));\n    MzContext ctx=mz_context(float3(uv,0),float3(0),float3(0),float3(0));\n    ctx.valid=0.0f;ctx.time=sample_time;\n    pixels[gid.y * dimensions.x + gid.x] = {result};\n}}").unwrap();
    writeln!(s,"\nkernel void material_preview_region(device float4* pixels [[buffer(0)]],\n    constant uint2& dimensions [[buffer(1)]], constant float4& region [[buffer(3)]], uint2 gid [[thread_position_in_grid]]{parameter_arg}{cache_args}, constant float& sample_time [[buffer(5)]]) {{\n    if (any(gid >= dimensions)) return;\n    float2 uv = region.xy + (float2(gid) + 0.5f) / float2(dimensions) * region.zw;\n    float aa = max(abs(region.z) / float(dimensions.x), abs(region.w) / float(dimensions.y));\n    MzContext ctx=mz_context(float3(uv,0),float3(0),float3(0),float3(0));\n    ctx.valid=0.0f;ctx.time=sample_time;\n    pixels[gid.y * dimensions.x + gid.x] = {result};\n}}\n").unwrap();
    Ok(Shader {
        #[cfg(feature = "wgpu")]
        c_functions,
        #[cfg(feature = "wgpu")]
        c_result,
        source: s,
        passes: vec![],
        domains: vec![],
        fixed_domains: vec![],
        domain_fixed: vec![],
        pixel_padding: 0.,
        uv_padding: 0.,
        material: material.into(),
        channel,
        slots,
        values,
        preview,
    })
}
