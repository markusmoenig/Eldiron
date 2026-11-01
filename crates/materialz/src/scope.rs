//! Conservative per-component dependencies, independent of the row interface.
//! Instance data remains uniform within each sample_scoped invocation.
use crate::{
    ir::{Binding, NodeId, Program, Reference},
    metadata::Kernel,
    tilelang::{Control, Expr},
};

pub const PIXEL: u8 = 1;
pub const INSTANCE: u8 = 2;
pub const FOOTPRINT: u8 = 4;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Uniform,
    Instance,
    Pixel,
}
pub fn classify(mask: u8) -> Scope {
    if mask & PIXEL != 0 {
        Scope::Pixel
    } else if mask & INSTANCE != 0 {
        Scope::Instance
    } else {
        Scope::Uniform
    }
}
pub fn reference(program: &Program, r: &Reference) -> Vec<u8> {
    r.components
        .iter()
        .map(|&c| {
            if r.distance {
                program.nodes[r.node.0].distance_dependencies
            } else {
                program.nodes[r.node.0].value_dependencies[c]
            }
        })
        .collect()
}
pub fn binding(program: &Program, b: &Binding) -> Vec<u8> {
    match b {
        Binding::Constant(v) => vec![0; v.len()],
        Binding::From(r) => reference(program, r),
    }
}
fn union(v: &[u8]) -> u8 {
    v.iter().fold(0, |a, b| a | b)
}
fn expr(e: &Expr, values: &[Vec<u8>], input: [u8; 4]) -> Vec<u8> {
    match e {
        Expr::Number(_) => vec![0],
        Expr::Variable(i) => values[*i].clone(),
        Expr::Neg(a) => expr(a, values, input),
        Expr::Swizzle(a, sw) => {
            let a = expr(a, values, input);
            sw.iter().map(|i| a[*i]).collect()
        }
        Expr::Binary(_, a, b) => {
            let a = expr(a, values, input);
            let b = expr(b, values, input);
            (0..a.len().max(b.len()))
                .map(|i| a[i % a.len()] | b[i % b.len()])
                .collect()
        }
        Expr::Call(name, args) => {
            let a = args
                .iter()
                .map(|a| expr(a, values, input))
                .collect::<Vec<_>>();
            if matches!(name.as_str(), "vec2" | "vec3" | "vec4") {
                let n = name.as_bytes()[3] as usize - b'0' as usize;
                let v = a.into_iter().flatten().collect::<Vec<_>>();
                return if v.len() == 1 { vec![v[0]; n] } else { v };
            }
            if matches!(
                name.as_str(),
                "sample" | "sample_image" | "sample_context" | "sample_scoped"
            ) {
                let mut v = input;
                if name == "sample_scoped" {
                    for d in &mut v {
                        if *d & INSTANCE != 0 {
                            *d = (*d & !INSTANCE)
                                | PIXEL
                                | union(&a.iter().skip(2).flatten().copied().collect::<Vec<_>>())
                        }
                    }
                }
                return if name == "sample" {
                    v[..3].to_vec()
                } else {
                    v.to_vec()
                };
            }
            let scalar = matches!(
                name.as_str(),
                "dot"
                    | "length"
                    | "distance"
                    | "equal"
                    | "different"
                    | "less"
                    | "greater"
                    | "less_equal"
                    | "greater_equal"
                    | "logical_and"
                    | "logical_or"
                    | "component"
            );
            let all = union(&a.iter().flatten().copied().collect::<Vec<_>>());
            if scalar {
                return vec![all];
            }
            // Coupled functions use all components; componentwise functions preserve lanes.
            if matches!(name.as_str(), "normalize" | "hash33" | "hash22") {
                return vec![all; a[0].len()];
            }
            let n = a.iter().map(Vec::len).max().unwrap_or(1);
            (0..n)
                .map(|i| a.iter().fold(0, |d, v| d | v[i % v.len()]))
                .collect()
        }
    }
}
pub fn analyze(program: &mut Program) {
    for id in program.order.clone() {
        let n = &program.nodes[id.0];
        let input = n
            .input
            .as_ref()
            .map(|r| {
                let v = reference(program, r);
                [
                    v[0],
                    v[1],
                    v[2],
                    program.nodes[r.node.0].value_dependencies[3],
                ]
            })
            .unwrap_or([0; 4]);
        let params = n
            .parameters
            .iter()
            .map(|p| binding(program, &p.binding))
            .collect::<Vec<_>>();
        let mut values = vec![
            vec![PIXEL; 2],
            vec![FOOTPRINT],
            input.to_vec(),
            vec![PIXEL; 3],
            vec![PIXEL; 3],
            vec![PIXEL; 3],
            vec![PIXEL; 3],
            vec![PIXEL],
            vec![PIXEL; 3],
            vec![0],
            vec![INSTANCE; 3],
            vec![INSTANCE; 3],
            vec![INSTANCE],
        ];
        values.extend([vec![INSTANCE; 3], vec![INSTANCE; 3], vec![INSTANCE; 3]]);
        values.extend(params.iter().cloned());
        let base = values.len();
        values.extend(n.algorithm.steps.iter().map(|s| vec![0; s.width]));
        // Monotone fixed point, including loop-count/condition dependencies.
        loop {
            let previous = values.clone();
            let mut controls = Vec::new();
            let mut control = 0;
            for (i, s) in n.algorithm.steps.iter().enumerate() {
                if let Control::Repeat { count, .. } = &s.control {
                    controls.push(control);
                    control |= union(&expr(count, &values, input))
                        | union(&expr(&s.expression, &values, input));
                }
                if matches!(s.control, Control::End { .. }) {
                    control = controls.pop().unwrap_or(0)
                }
                let mut v = expr(&s.expression, &values, input);
                v.resize(s.width, union(&v));
                for d in &mut v {
                    *d |= control
                }
                for (to, from) in values[base + i].iter_mut().zip(&v) {
                    *to |= *from
                }
                if let Control::Update { target } = s.control {
                    for (to, from) in values[target].iter_mut().zip(&v) {
                        *to |= *from
                    }
                }
            }
            if values == previous {
                break;
            }
        }
        let mut out = if let Some(i) = n.algorithm.image {
            let v = &values[base + i];
            [v[0], v[1], v[2], v[3]]
        } else {
            [
                0,
                0,
                0,
                PIXEL | union(&params.iter().flatten().copied().collect::<Vec<_>>()),
            ]
        };
        if n.kernel == Kernel::CellData && n.code == crate::tilelang::source(Kernel::CellData) {
            let constant = |name: &str| match n.parameter(name) {
                Binding::Constant(v) => Some(v[0].round() as i32),
                _ => None,
            };
            let d = if constant("output")
                .is_some_and(|mode| mode > 10 || (constant("scope") == Some(1) && mode != 3))
            {
                INSTANCE | union(&binding(program, n.parameter("seed")))
            } else {
                PIXEL
            };
            out = [d, d, d, input[3]];
        }
        if n.algorithm.primitive || n.algorithm.raster.is_some() {
            out = [PIXEL; 4]
        }
        if n.coordinates.is_some() {
            for d in &mut out {
                *d |= PIXEL
            }
        }
        let geometry = params.iter().flatten().fold(0, |a, b| a | b);
        let uniform_distance = if n.kernel == Kernel::Transform {
            let uniform = match n.parameter("localScale") {
                Binding::Constant(v) => v[0].abs() == v[1].abs(),
                Binding::From(r) => r.components[0] == r.components[1],
            };
            uniform && geometry & PIXEL == 0
        } else {
            false
        };
        let n = &mut program.nodes[id.0];
        n.value_dependencies = out;
        n.distance_dependencies = PIXEL | geometry;
        n.uniform_distance = uniform_distance;
    }
}

pub fn is_context_dependent(program: &Program, id: NodeId) -> bool {
    program.nodes[id.0]
        .value_dependencies
        .iter()
        .any(|d| d & INSTANCE != 0)
}

/// Specialize standard Cell Data modes without baking editable numeric controls.
/// This keeps instance-constant data out of the repeated pixel work of filters.
pub fn specialize_cell_data(program: &mut Program) -> crate::Result<()> {
    for n in &mut program.nodes {
        if n.kernel != Kernel::CellData || n.code != crate::tilelang::source(Kernel::CellData) {
            continue;
        }
        let constant = |name: &str| {
            if let Binding::Constant(v) = n.parameter(name) {
                Some(v[0].round() as i32)
            } else {
                None
            }
        };
        let (Some(scope), Some(output)) = (constant("scope"), constant("output")) else {
            continue;
        };
        let prefix = n.code.split("let count =").next().unwrap();
        let mut body = String::new();
        if scope == 1 {
            body.push_str("let key = select(instance_valid, instance_cell, vec3(0))\n")
        } else {
            body.push_str("let count = clamp(round(abs(cells)), 1, 1024)\nlet p = (uv + 0.5) * count + offset\nlet cell = floor(p)\nlet key = vec3(mod(cell, count), 0)\n")
        }
        let value = match output {
            0 => "vec3(hash33(key + vec3(0, 0, floor(seed))).x)",
            1 => "vec3(hash33(vec3(0, key.y, key.z + floor(seed))).x)",
            2 => "vec3(hash33(vec3(key.x, 0, key.z + floor(seed))).x)",
            3 => {
                if scope == 1 {
                    "vec3(uv + 0.5, 0)"
                } else {
                    "vec3(fract(p), 0)"
                }
            }
            4 => {
                if scope == 1 {
                    "vec3(0)"
                } else {
                    "vec3((cell + 0.5 - offset) / count - 0.5, 0)"
                }
            }
            5 => "hash33(key + vec3(0, 0, floor(seed)))",
            6 => "vec3(key.xy, 0)",
            7 => "vec3(mod(key.x + key.y, 2))",
            11 => "instance_origin",
            12 => "vec3(length(instance_axis_x), length(instance_axis_y), 1)",
            13 => "vec3(atan2(instance_axis_x.y, nonzero(instance_axis_x.x)) * 57.2957795131)",
            8 | 9 => {
                body.push_str("let group = select(instance_valid, instance_group, key)\n");
                if output == 8 {
                    "vec3(hash33(group + vec3(0, 0, floor(seed))).x)"
                } else {
                    "group"
                }
            }
            _ => "key",
        };
        body.push_str(&format!("image = vec4({value}, input.a)\n"));
        n.algorithm = crate::tilelang::Algorithm::compile(
            &format!("{prefix}{body}"),
            &n.kernel.metadata(),
            n.kernel,
        )?;
    }
    Ok(())
}
