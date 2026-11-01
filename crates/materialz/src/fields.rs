//! Named source fields resolve to shared, specialized evaluations, without
//! changing row identities or forcing data into the four image channels.
use crate::{
    Error, Result,
    ir::{Binding, NodeId, Program, Reference},
};
use std::collections::HashMap;
pub fn resolve(program: &mut Program) -> Result<()> {
    let mut cache = HashMap::new();
    fn fix(
        r: &mut Reference,
        p: &mut Program,
        cache: &mut HashMap<(NodeId, String), NodeId>,
    ) -> Result<()> {
        let Some(field) = r.field.take() else {
            return Ok(());
        };
        if r.distance {
            return Err(Error(
                "Named fields use component selections, not distance".into(),
            ));
        }
        let key = (r.node, field.clone());
        let source = &p.nodes[r.node.0];
        let meta = source.kernel.metadata();
        let f = meta
            .output_fields
            .iter()
            .find(|f| f.id == field)
            .ok_or_else(|| Error(format!("{} has no field {field:?}", meta.name)))?;
        let mask = f
            .components
            .chars()
            .map(|c| "xyzw".find(c).unwrap())
            .collect::<Vec<_>>();
        for c in &mut r.components {
            *c = *mask.get(*c).ok_or_else(|| {
                Error(format!(
                    "Field {field:?} has only {} components",
                    mask.len()
                ))
            })?
        }
        if let Some(id) = cache.get(&key) {
            r.node = *id;
            return Ok(());
        }
        let mut node = source.clone();
        let parameter = node
            .parameters
            .iter_mut()
            .find(|p| p.id == "output")
            .ok_or_else(|| Error("Named fields need an Output selector".into()))?;
        parameter.binding = Binding::Constant(vec![f.mode]);
        let id = NodeId(p.nodes.len());
        p.nodes.push(node.clone());
        cache.insert(key, id);
        for r in node
            .input
            .iter_mut()
            .chain(node.coordinates.iter_mut())
            .chain(node.parameters.iter_mut().filter_map(|p| {
                if let Binding::From(r) = &mut p.binding {
                    Some(r)
                } else {
                    None
                }
            }))
        {
            fix(r, p, cache)?
        }
        p.nodes[id.0] = node;
        r.node = id;
        Ok(())
    }
    let original = program.nodes.len();
    for i in 0..original {
        let mut node = program.nodes[i].clone();
        for r in node
            .input
            .iter_mut()
            .chain(node.coordinates.iter_mut())
            .chain(node.parameters.iter_mut().filter_map(|p| {
                if let Binding::From(r) = &mut p.binding {
                    Some(r)
                } else {
                    None
                }
            }))
        {
            fix(r, program, &mut cache)?
        }
        program.nodes[i] = node;
    }
    for m in 0..program.materials.len() {
        let mut out = program.materials[m].outputs.clone();
        for (_, r) in &mut out {
            fix(r, program, &mut cache)?
        }
        program.materials[m].outputs = out;
    }
    Ok(())
}
