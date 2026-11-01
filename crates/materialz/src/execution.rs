//! Raster execution boundaries beneath the procedural row graph.
//! Filters may materialize their inputs; instance-dependent work stays callable.
use crate::{
    Error, Result,
    ir::{Binding, NodeId, Program},
};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub inputs: Vec<NodeId>,
    pub pixel_padding: f32,
    pub uv_padding: f32,
    pub domains: Vec<[f32; 6]>,
    pub fixed_domains: Vec<[f32; 4]>,
    pub domain_fixed: Vec<bool>,
}
pub fn is_filter(program: &Program, id: NodeId) -> bool {
    program.nodes[id.0].image_execution.is_some()
}
#[derive(Clone, Copy)]
struct Domain {
    matrix: [f32; 6],
    fixed: Option<[f32; 4]>,
}
pub fn build(program: &Program, root: NodeId) -> Result<Plan> {
    let mut plan = Plan::default();
    let mut seen = HashSet::new();
    fn visit(
        p: &Program,
        id: NodeId,
        instanced: bool,
        plan: &mut Plan,
        seen: &mut HashSet<(NodeId, bool, Vec<u32>)>,
        domain: Option<Domain>,
    ) -> Result<()> {
        let marker = domain
            .map(|d| {
                d.matrix
                    .into_iter()
                    .chain(d.fixed.unwrap_or([f32::NAN; 4]))
                    .map(f32::to_bits)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !seen.insert((id, instanced, marker)) {
            return Ok(());
        }
        let n = &p.nodes[id.0];
        if let Some((radius_control, unit_control)) = &n.image_execution {
            let constant = |name: &str, default: f32| match n.parameter(name) {
                Binding::Constant(v) => v[0],
                _ => default,
            };
            let radius = constant(radius_control, 64.).clamp(0., 64.);
            if constant(unit_control, 1.) < 0.5 {
                plan.pixel_padding += radius + 2.
            } else {
                plan.uv_padding += radius;
                plan.pixel_padding += 2.
            }
        }
        if n.cache_input {
            let domain=domain.ok_or_else(||Error("This image pass is reached through a spatially varying domain or instance placement. Place the filter after that operation so its image domain is explicit.".into()))?;
            let domain = if n.code.contains("name \"Periodic Image\"") {
                if let Binding::Constant(period) = n.parameter("period") {
                    let shift = if n
                        .algorithm
                        .cost(
                            &n.parameters
                                .iter()
                                .map(|p| {
                                    if let Binding::Constant(v) = &p.binding {
                                        Some(v.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>(),
                        )
                        .1
                        > 1
                    {
                        1.5
                    } else {
                        0.5
                    };
                    Domain {
                        matrix: [1., 0., 0., 1., 0., 0.],
                        fixed: Some([
                            -shift * period[0],
                            -shift * period[1],
                            (shift + 0.5) * period[0],
                            (shift + 0.5) * period[1],
                        ]),
                    }
                } else {
                    domain
                }
            } else {
                domain
            };
            if !plan
                .domains
                .iter()
                .zip(&plan.domain_fixed)
                .any(|(m, f)| *m == domain.matrix && *f == domain.fixed.is_some())
            {
                plan.domains.push(domain.matrix);
                plan.domain_fixed.push(domain.fixed.is_some());
            }
            if let Some(rect) = domain.fixed {
                let m = domain.matrix;
                let mut lo = [f32::INFINITY; 2];
                let mut hi = [f32::NEG_INFINITY; 2];
                for x in [rect[0], rect[0] + rect[2]] {
                    for y in [rect[1], rect[1] + rect[3]] {
                        let v = [m[0] * x + m[1] * y + m[4], m[2] * x + m[3] * y + m[5]];
                        for i in 0..2 {
                            lo[i] = lo[i].min(v[i]);
                            hi[i] = hi[i].max(v[i]);
                        }
                    }
                }
                let bounds = [lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]];
                if !plan.fixed_domains.contains(&bounds) {
                    plan.fixed_domains.push(bounds)
                }
            }
            let input = n
                .input
                .as_ref()
                .ok_or_else(|| Error("Cached filter needs an input".into()))?;
            if instanced && crate::scope::is_context_dependent(p, input.node) {
                return Err(Error("An image pass cannot cache instance-dependent data across instances. Move the filter after placement, or filter an instance-independent motif before placement.".into()));
            }
            visit(p, input.node, instanced, plan, seen, Some(domain))?;
            if !plan.inputs.contains(&input.node) {
                plan.inputs.push(input.node)
            }
            for dep in n.dependencies().into_iter().filter(|d| *d != input.node) {
                visit(p, dep, instanced, plan, seen, Some(domain))?
            }
        } else {
            let nested = instanced
                || matches!(
                    n.kernel.source_id(),
                    "trans_tiler" | "scatter" | "partition"
                );
            for dep in n.dependencies() {
                let is_input = n.input.as_ref().is_some_and(|r| r.node == dep);
                let mut next = domain;
                if n.coordinates.is_some() {
                    next = None
                }
                if is_input {
                    if n.kernel == crate::metadata::Kernel::Transform
                        && n.code.contains("name \"Periodic Image\"")
                    {
                        if let Binding::Constant(period) = n.parameter("period") {
                            let shift = if n
                                .algorithm
                                .cost(
                                    &n.parameters
                                        .iter()
                                        .map(|p| {
                                            if let Binding::Constant(v) = &p.binding {
                                                Some(v.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<_>>(),
                                )
                                .1
                                > 1
                            {
                                1.5
                            } else {
                                0.5
                            };
                            next = Some(Domain {
                                matrix: [1., 0., 0., 1., 0., 0.],
                                fixed: Some([
                                    -shift * period[0],
                                    -shift * period[1],
                                    (shift + 0.5) * period[0],
                                    (shift + 0.5) * period[1],
                                ]),
                            });
                        } else {
                            next = None
                        }
                    } else if n.kernel == crate::metadata::Kernel::Transform
                        && n.code == crate::tilelang::source(n.kernel)
                    {
                        let get = |name: &str| {
                            if let Binding::Constant(v) = n.parameter(name) {
                                Some(v.as_slice())
                            } else {
                                None
                            }
                        };
                        next = if let (Some(m), Some(g), Some(l), Some(o), Some(r)) = (
                            next,
                            get("globalScale"),
                            get("localScale"),
                            get("offset"),
                            get("rot"),
                        ) {
                            let domain = m;
                            let m = domain.matrix;
                            let angle = r[0].to_radians();
                            let c = angle.cos();
                            let s = angle.sin();
                            let sx = crate::cpu::nonzero(g[0] * l[0]);
                            let sy = crate::cpu::nonzero(g[0] * l[1]);
                            let a = [c / sx, s / sy, -s / sx, c / sy];
                            let b = [-a[0] * o[0] - a[1] * o[1], -a[2] * o[0] - a[3] * o[1]];
                            Some(Domain {
                                fixed: domain.fixed,
                                matrix: [
                                    a[0] * m[0] + a[1] * m[2],
                                    a[0] * m[1] + a[1] * m[3],
                                    a[2] * m[0] + a[3] * m[2],
                                    a[2] * m[1] + a[3] * m[3],
                                    a[0] * m[4] + a[1] * m[5] + b[0],
                                    a[2] * m[4] + a[3] * m[5] + b[1],
                                ],
                            })
                        } else {
                            None
                        };
                    } else if nested
                        || n.algorithm.domain.is_some()
                        || (n.image_execution.is_none()
                            && n.algorithm
                                .cost(
                                    &n.parameters
                                        .iter()
                                        .map(|p| {
                                            if let Binding::Constant(v) = &p.binding {
                                                Some(v.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<_>>(),
                                )
                                .1
                                > 1)
                    {
                        next = None
                    }
                }
                visit(p, dep, nested, plan, seen, next)?;
            }
        }
        Ok(())
    }
    visit(
        program,
        root,
        false,
        &mut plan,
        &mut seen,
        Some(Domain {
            matrix: [1., 0., 0., 1., 0., 0.],
            fixed: None,
        }),
    )?;
    if plan.inputs.len() > 16 {
        return Err(Error(
            "Execution plan exceeds 16 intermediate images".into(),
        ));
    }
    Ok(plan)
}
pub fn cuts(plan: &Plan) -> HashMap<NodeId, usize> {
    plan.inputs
        .iter()
        .enumerate()
        .map(|(i, n)| (*n, i))
        .collect()
}
