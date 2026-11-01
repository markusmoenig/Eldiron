//! Whole-material operations lower to ordinary typed channel nodes. All backends,
//! parameter buffers and complexity checks therefore share the same semantics.
use crate::{
    Error, Result, format as ast,
    ir::{Binding, Channel, Node, NodeId, Parameter, Program, Reference},
    metadata::{Kernel, ValueType},
    tilelang,
};

fn reference(node: NodeId, width: usize) -> Reference {
    Reference {
        field: None,
        node,
        components: (0..width).collect(),
        mapping: None,
        distance: false,
    }
}
fn channel(p: &Program, index: Option<usize>, c: Channel) -> Binding {
    match index {
        Some(i) => p.materials[i]
            .outputs
            .iter()
            .find(|(k, _)| *k == c)
            .map(|(_, r)| Binding::From(r.clone()))
            .unwrap_or_else(|| {
                Binding::Constant(p.materials[i].default_value(c).unwrap().to_vec())
            }),
        None => Binding::Constant(c.default().to_vec()),
    }
}
pub(crate) fn node(
    p: &mut Program,
    id: &str,
    parameters: Vec<(&str, Binding)>,
    body: &str,
    width: usize,
) -> Result<Binding> {
    let mut source =
        "tile 1\nname \"Material mix\"\ncategory \"Material Mixer\"\ngroup \"Mix\" true\n"
            .to_owned();
    let mut params = Vec::new();
    for (name, binding) in parameters {
        let w = match &binding {
            Binding::Constant(v) => v.len(),
            Binding::From(r) => r.components.len(),
        };
        let kind = if w == 1 {
            ValueType::Float
        } else {
            ValueType::Vec3
        };
        source.push_str(&format!(
            "param {} {name} \"{name}\" default {}\n",
            kind.name(),
            vec!["0"; w].join(" ")
        ));
        params.push(Parameter {
            id: name.into(),
            kind,
            binding,
            choice_count: 0,
        });
    }
    source.push_str(body);
    let metadata = tilelang::definition(&source)?;
    let algorithm = tilelang::Algorithm::compile(&source, &metadata, Kernel::Color)?;
    let index = NodeId(p.nodes.len());
    p.nodes.push(Node {
        algorithm,
        code: source,
        kernel: Kernel::Color,
        parameters: params,
        input: None,
        coordinates: None,
        row_id: format!("material:{id}"),
        tile_index: 0,
        combine: ast::Combine::Replace,
        has_distance: false,
        value_dependencies: [1; 4],
        distance_dependencies: 1,
        uniform_distance: false,
        cache_input: false,
        image_execution: None,
    });
    Ok(Binding::From(reference(index, width)))
}

pub(crate) fn lower(p: &mut Program, project: &ast::Project, limit: usize) -> Result<()> {
    fn visit(
        p: &mut Program,
        project: &ast::Project,
        i: usize,
        state: &mut [u8],
        depth: usize,
        limit: usize,
    ) -> Result<()> {
        if state[i] == 2 {
            return Ok(());
        }
        if state[i] == 1 {
            return Err(Error(format!(
                "Material dependency cycle involving {:?}",
                project.materials[i].id
            )));
        }
        if depth > 64 {
            return Err(Error("Material dependency depth budget exceeded".into()));
        }
        state[i] = 1;
        if let Some(mix) = &project.materials[i].mixing {
            let count = if mix.kind == "UV Position" { 4 } else { 2 };
            if mix.inputs[count..].iter().any(Option::is_some) {
                return Err(Error("This mixer only accepts two materials".into()));
            }
            if mix.direction > 1
                || mix
                    .levels
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            {
                return Err(Error(
                    "Mixer levels must be finite in 0..1; direction must be x or y".into(),
                ));
            }
            let mut inputs = [None; 4];
            for (slot, input) in mix.inputs.iter().enumerate() {
                if let Some(id) = input {
                    let index = project
                        .materials
                        .iter()
                        .position(|m| m.id == *id)
                        .ok_or_else(|| Error(format!("Unknown material input {id:?}")))?;
                    visit(p, project, index, state, depth + 1, limit)?;
                    inputs[slot] = Some(index);
                }
            }
            let scalar = |v| Binding::Constant(vec![v]);
            let mask = match &mix.mask {
                ast::Binding::Constant(v) => {
                    if v.len() != 3 || v.iter().any(|v| !v.is_finite()) {
                        return Err(Error("Material mask needs three finite components".into()));
                    }
                    Binding::Constant(v.clone())
                }
                ast::Binding::From(r) => {
                    let mask_index = if let Some(id) = &mix.mask_material {
                        project
                            .materials
                            .iter()
                            .position(|m| m.id == *id)
                            .ok_or_else(|| Error(format!("Unknown mask material {id:?}")))?
                    } else {
                        i
                    };
                    let rows = p.materials[mask_index].rows.iter().cloned().collect();
                    Binding::From(crate::ir::resolve(r, &rows, 3)?)
                }
            };
            // One common selector preserves a coherent choice across every BSDF field.
            let selector = match mix.kind.as_str() {
                "Bump Height" => node(
                    p,
                    &format!("{i}/selector"),
                    vec![
                        ("a", channel(p, inputs[0], Channel::Height)),
                        ("b", channel(p, inputs[1], Channel::Height)),
                    ],
                    "image = vec4(select(greater_equal(a,b),0,1))\n",
                    1,
                )?,
                "Overlay" => node(
                    p,
                    &format!("{i}/selector"),
                    vec![
                        ("c", channel(p, inputs[1], Channel::BaseColor)),
                        ("level", scalar(mix.levels[0])),
                    ],
                    "image = vec4(select(greater((c.x+c.y+c.z)/3,level),1,0))\n",
                    1,
                )?,
                "UV Position" => node(
                    p,
                    &format!("{i}/selector"),
                    vec![
                        ("direction", scalar(mix.direction as f32)),
                        ("l1", scalar(mix.levels[0])),
                        ("l2", scalar(mix.levels[1])),
                        ("l3", scalar(mix.levels[2])),
                    ],
                    "let u = select(equal(direction,0),uv.x,uv.y)+0.5\nimage = vec4(select(less_equal(u,l1),0,select(less_equal(u,l2),1,select(less_equal(u,l3),2,3))))\n",
                    1,
                )?,
                "Mask" => mask,
                _ => return Err(Error("Unknown material mixer".into())),
            };
            for c in Channel::SURFACE {
                // Explicit local row outputs override a mixed channel.
                if p.materials[i].outputs.iter().any(|(k, _)| k == c) {
                    continue;
                }
                let width = c.width();
                let mut parameters = vec![
                    ("a", channel(p, inputs[0], *c)),
                    ("b", channel(p, inputs[1], *c)),
                    ("selector", selector.clone()),
                ];
                let mut expression = match mix.kind.as_str() {
                    "UV Position" => {
                        parameters.push(("c", channel(p, inputs[2], *c)));
                        parameters.push(("d", channel(p, inputs[3], *c)));
                        "select(equal(selector,0),a,select(equal(selector,1),b,select(equal(selector,2),c,d)))".to_owned()
                    }
                    "Mask" => {
                        parameters.push(("blend", scalar(f32::from(mix.blend))));
                        let mask = if *c == Channel::BaseColor {
                            "selector"
                        } else {
                            "selector.x"
                        };
                        // Old Mix-off used negative bump as an invisible sentinel.
                        // Give it defined semantics in the modern BSDF: zero opacity.
                        let off = if *c == Channel::Opacity { "0" } else { "a" };
                        format!(
                            "select(equal(selector.x,0)*equal(selector.y,0)*equal(selector.z,0),a,select(equal(selector.x,1)*equal(selector.y,1)*equal(selector.z,1),b,select(blend,mix(a,b,{mask}),{off})))"
                        )
                    }
                    "Overlay" if *c == Channel::Height => "select(selector,a+b,a)".into(),
                    _ => "select(selector,b,a)".into(),
                };
                if width == 3 {
                    expression = format!("vec4({expression},1)")
                } else {
                    expression = format!("vec4(vec3({expression}),1)")
                }
                let result = node(
                    p,
                    &format!("{i}/{}", c.name()),
                    parameters,
                    &format!("image = {expression}\n"),
                    width,
                )?;
                if let Binding::From(r) = result {
                    p.materials[i].outputs.push((*c, r));
                }
                if p.nodes.len() > limit {
                    return Err(Error("Node budget exceeded by material mixing".into()));
                }
            }
        }
        crate::periodic::lower_material(p, i, limit)?;
        state[i] = 2;
        Ok(())
    }
    let mut state = vec![0; project.materials.len()];
    for i in 0..project.materials.len() {
        visit(p, project, i, &mut state, 1, limit)?
    }
    Ok(())
}
