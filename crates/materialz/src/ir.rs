//! Typed procedural computation, scopes, and bounded per-pass cost validation.
use crate::{
    Error, Result, format as ast,
    metadata::{Kernel, ValueType},
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub field: Option<String>,
    pub distance: bool,
    pub node: NodeId,
    pub components: Vec<usize>,
    pub mapping: Option<(f32, f32)>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    Constant(Vec<f32>),
    From(Reference),
}
#[derive(Debug, Clone)]
pub struct Parameter {
    pub id: String,
    pub kind: ValueType,
    pub binding: Binding,
    pub choice_count: usize,
}
#[derive(Debug, Clone)]
pub struct Node {
    pub algorithm: crate::tilelang::Algorithm,
    pub code: String,
    pub kernel: Kernel,
    pub parameters: Vec<Parameter>,
    pub input: Option<Reference>,
    pub coordinates: Option<Reference>,
    pub row_id: String,
    pub tile_index: usize,
    pub combine: ast::Combine,
    pub has_distance: bool,
    /// Value dependencies per RGBA component: pixel/position (1), instance (2), footprint (4).
    pub value_dependencies: [u8; 4],
    pub distance_dependencies: u8,
    pub uniform_distance: bool,
    pub cache_input: bool,
    pub image_execution: Option<(String, String)>,
}
impl Node {
    pub fn parameter(&self, id: &str) -> &Binding {
        &self
            .parameters
            .iter()
            .find(|p| p.id == id)
            .expect("Validated kernel parameter")
            .binding
    }
    /// Uniform constant transforms preserve a metric distance. A dynamic or
    /// nonuniform transform still renders its image but clears the distance.
    pub fn distance_scale(&self) -> Option<f32> {
        if !matches!(self.parameter("rot"), Binding::Constant(_))
            || !matches!(self.parameter("offset"), Binding::Constant(_))
        {
            return None;
        }
        let Binding::Constant(g) = self.parameter("globalScale") else {
            return None;
        };
        let Binding::Constant(l) = self.parameter("localScale") else {
            return None;
        };
        if l[0].abs() != l[1].abs() {
            return None;
        }
        Some((g[0] * l[0]).abs())
    }
    pub fn dependencies(&self) -> Vec<NodeId> {
        let mut ids = vec![];
        if let Some(r) = &self.input {
            ids.push(r.node);
        }
        if let Some(r) = &self.coordinates {
            ids.push(r.node);
        }
        // Parameter references share one sample at this node's UV/context.
        // Input and coordinate samples retain their separate domain semantics.
        let mut parameters = vec![];
        for p in &self.parameters {
            if let Binding::From(r) = &p.binding
                && !parameters.contains(&r.node)
            {
                parameters.push(r.node);
                ids.push(r.node);
            }
        }
        ids
    }
}
pub use crate::bsdf::Channel;
#[derive(Debug, Clone)]
pub struct Material {
    pub tiling: crate::periodic::Tiling,
    pub defaults: Vec<(Channel, Vec<f32>)>,
    pub id: String,
    pub label: String,
    pub outputs: Vec<(Channel, Reference)>,
    pub rows: Vec<(String, NodeId)>,
}
impl Material {
    pub fn default_value(&self, channel: Channel) -> Result<&[f32]> {
        if channel == Channel::Distance {
            return Err(Error("Distance has no material default".into()));
        }
        Ok(self
            .defaults
            .iter()
            .find(|(c, _)| *c == channel)
            .map(|(_, v)| v.as_slice())
            .unwrap_or_else(|| channel.default()))
    }
}
#[derive(Debug, Clone)]
pub struct Program {
    pub nodes: Vec<Node>,
    pub materials: Vec<Material>,
    pub order: Vec<NodeId>,
    pub costs: Vec<Cost>,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Cost {
    pub operations: usize,
    pub depth: usize,
    pub expanded_calls: usize,
}
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub nodes: usize,
    pub depth: usize,
    pub expanded_calls: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            nodes: 256,
            depth: 64,
            expanded_calls: 4096,
        }
    }
}

pub(crate) fn resolve(
    r: &ast::Reference,
    rows: &HashMap<String, NodeId>,
    width: usize,
) -> Result<Reference> {
    let node = *rows
        .get(&r.row)
        .ok_or_else(|| Error(format!("Unknown row {:?}", r.row)))?;
    let (field, selection) = r
        .components
        .split_once('.')
        .map(|(a, b)| (Some(a.to_owned()), b))
        .unwrap_or((None, r.components.as_str()));
    let distance = selection == "distance";
    let components = if distance {
        vec![0; width]
    } else {
        selection
            .chars()
            .map(|c| match c {
                'x' => Ok(0),
                'y' => Ok(1),
                'z' => Ok(2),
                'w' => Ok(3),
                _ => Err(Error(format!(
                    "Invalid component {c:?}; rows return xyzw or distance"
                ))),
            })
            .collect::<Result<Vec<_>>>()?
    };
    if components.len() != width {
        return Err(Error(format!(
            "Expected {width} component(s), got {} in row {:?}",
            components.len(),
            r.row
        )));
    }
    if r.mapping
        .is_some_and(|(a, b)| !a.is_finite() || !b.is_finite() || !(b - a).is_finite())
    {
        return Err(Error("Mapping range must be finite".into()));
    }
    Ok(Reference {
        field,
        distance,
        node,
        components,
        mapping: r.mapping,
    })
}
impl Program {
    pub fn lower(project: &ast::Project) -> Result<Self> {
        Self::lower_with_limits(project, Limits::default())
    }
    pub fn lower_with_limits(project: &ast::Project, limits: Limits) -> Result<Self> {
        ast::validate_tile_ids(project)?;
        ast::validate_row_connections(project)?;
        if let Some(result) = &project.result
            && !project.materials.iter().any(|m| m.id == *result)
        {
            return Err(Error(format!("Unknown result material {result:?}")));
        }
        let total = project
            .materials
            .iter()
            .flat_map(|m| &m.rows)
            .map(|r| r.tiles.len().max(1))
            .sum::<usize>();
        if total > limits.nodes {
            return Err(Error(format!(
                "Node budget exceeded: {total} > {}",
                limits.nodes
            )));
        }
        let mut program = Self {
            nodes: vec![],
            materials: vec![],
            order: vec![],
            costs: vec![],
        };
        let mut material_ids = HashSet::new();
        for m in &project.materials {
            if m.id.is_empty() || !material_ids.insert(&m.id) {
                return Err(Error(format!("Empty or duplicate material ID {:?}", m.id)));
            }
            let mut rows = HashMap::new();
            let mut next = program.nodes.len();
            for r in &m.rows {
                if r.tiles.is_empty() && r.incoming.is_empty() {
                    return Err(Error(format!("Row {:?} needs a tile", r.id)));
                }
                if r.id.is_empty()
                    || rows
                        .insert(r.id.clone(), NodeId(next + r.tiles.len().max(1) - 1))
                        .is_some()
                {
                    return Err(Error(format!("Empty or duplicate row ID {:?}", r.id)));
                }
                next += r.tiles.len().max(1);
            }
            for r in &m.rows {
                let mut previous = None;
                // Keep an empty authoring row when it owns connections. It has a
                // neutral white output; the retained incoming links remain dormant.
                let empty = ast::Tile {
                    id: String::new(),
                    kernel: "Color".into(),
                    parameters: vec![("color".into(), ast::Binding::Constant(vec![1.; 4]))],
                    input: None,
                    input_disconnected: true,
                    code: None,
                    image: None,
                    combine: ast::Combine::Replace,
                };
                let tiles = if r.tiles.is_empty() {
                    std::slice::from_ref(&empty)
                } else {
                    &r.tiles
                };
                for (tile_index, t) in tiles.iter().enumerate() {
                    let node_id = NodeId(program.nodes.len());
                    let kernel = Kernel::find(&t.kernel).ok_or_else(|| {
                        Error(format!("Unknown source {:?} in row {:?}", t.kernel, r.id))
                    })?;
                    let metadata = kernel.metadata();
                    let mut parameters = vec![];
                    let mut names = HashSet::new();
                    for (id, _) in &t.parameters {
                        if !names.insert(id) {
                            return Err(Error(format!("Duplicate parameter {id:?}")));
                        }
                        if !metadata.parameters.iter().any(|p| &p.id == id) {
                            return Err(Error(format!("Unknown parameter {id:?} on {}", t.kernel)));
                        }
                    }
                    for p in &metadata.parameters {
                        let binding = match t
                            .parameters
                            .iter()
                            .find(|(id, _)| id == &p.id)
                            .map(|(_, b)| b)
                        {
                            None => Binding::Constant(p.default.clone()),
                            Some(ast::Binding::Constant(v)) => {
                                if v.len() != p.kind.width() || v.iter().any(|x| !x.is_finite()) {
                                    return Err(Error(format!(
                                        "{} needs {} finite component(s)",
                                        p.id,
                                        p.kind.width()
                                    )));
                                }
                                if p.kind == ValueType::Bool && !matches!(v[0], 0. | 1.) {
                                    return Err(Error(format!("{} must be true or false", p.id)));
                                }
                                Binding::Constant(v.clone())
                            }
                            Some(ast::Binding::From(r)) => {
                                if p.kind == ValueType::Bool {
                                    return Err(Error(
                                        "Boolean row bindings are not supported in version 1"
                                            .into(),
                                    ));
                                }
                                Binding::From(resolve(r, &rows, p.kind.width())?)
                            }
                        };
                        let divisor = (matches!(kernel, Kernel::Perlin | Kernel::Transform)
                            && matches!(p.id.as_str(), "globalScale" | "localScale"))
                            || (kernel == Kernel::Disc
                                && matches!(p.id.as_str(), "globalScale" | "inBEVEL"));
                        if divisor
                            && let Binding::Constant(v) = &binding
                            && v.iter().any(|x| x.abs() < 1e-6)
                        {
                            return Err(Error(format!(
                                "{} on {} must have magnitude at least 0.000001",
                                p.id, t.kernel
                            )));
                        }
                        if let Binding::Constant(v) = &binding {
                            let positive = (kernel == Kernel::Circle && p.id == "radius")
                                || (kernel == Kernel::RoundedRectangle && p.id == "halfSize")
                                || (kernel.is_shape() && p.id == "smoothness");
                            let nonnegative = (kernel == Kernel::RoundedRectangle
                                && p.id == "radius")
                                || (kernel == Kernel::Border && p.id == "width");
                            if (positive && v.iter().any(|x| *x < 1e-6))
                                || (nonnegative && v.iter().any(|x| *x < 0.))
                            {
                                return Err(Error(format!("Invalid shape dimension {}", p.id)));
                            }
                        }
                        parameters.push(Parameter {
                            choice_count: p.choices.len(),
                            id: p.id.clone(),
                            kind: p.kind,
                            binding,
                        });
                    }
                    if t.combine != ast::Combine::Replace && !kernel.is_shape() {
                        return Err(Error("Only shapes support combine options".into()));
                    }
                    let needs_input = kernel.requires_input() || t.combine != ast::Combine::Replace;
                    let input = if let Some(input) = &t.input {
                        if !kernel.accepts_input() {
                            return Err(Error(format!("{} does not consume an input", t.kernel)));
                        }
                        Some(resolve(input, &rows, 3)?)
                    } else if t.input_disconnected {
                        None
                    } else if needs_input {
                        Some(Reference {
                            field: None,
                            distance: false,
                            node: previous.ok_or_else(|| {
                                Error(format!(
                                    "{} needs input from a row or preceding tile",
                                    t.kernel
                                ))
                            })?,
                            components: vec![0, 1, 2],
                            mapping: None,
                        })
                    } else if (matches!(kernel, Kernel::Library(_)) && metadata.category == "Noise")
                        || matches!(
                            kernel,
                            Kernel::Perlin
                                | Kernel::Color
                                | Kernel::ColorBlend
                                | Kernel::CellData
                                | Kernel::DirectionalGrain
                        )
                    {
                        previous.map(|node| Reference {
                            field: None,
                            node,
                            distance: false,
                            components: vec![0, 1, 2],
                            mapping: None,
                        })
                    } else {
                        None
                    };
                    // Attach coordinates to the row entry point, wrapping all preceding tiles.
                    let coordinates = if tile_index + 1 == tiles.len() {
                        r.coordinates
                            .as_ref()
                            .map(|c| {
                                resolve(
                                    c,
                                    &rows,
                                    if c.components
                                        .rsplit('.')
                                        .next()
                                        .unwrap_or(&c.components)
                                        .len()
                                        == 3
                                    {
                                        3
                                    } else {
                                        2
                                    },
                                )
                            })
                            .transpose()?
                    } else {
                        None
                    };
                    let code = t
                        .code
                        .as_deref()
                        .unwrap_or_else(|| crate::tilelang::source(kernel));
                    if crate::tilelang::definition(code)? != metadata {
                        return Err(Error("Tile source parameter declarations must match the built-in definition; edit the algorithm body for this instance".into()));
                    }
                    let mut algorithm =
                        crate::tilelang::Algorithm::compile(code, &metadata, kernel)?;
                    if metadata.name == "Image / Tile Source" {
                        if t.code.is_some() {
                            return Err(Error("Edit the image asset in Properties; Image / Tile Source has no custom shader body".into()));
                        }
                        let constant = |name: &str| -> Result<usize> {
                            let p = parameters.iter().find(|p| p.id == name).unwrap();
                            if let Binding::Constant(v) = &p.binding {
                                if v[0] >= 0. && v[0].fract() == 0. {
                                    Ok(v[0] as usize)
                                } else {
                                    Err(Error("Image options must be nonnegative integers".into()))
                                }
                            } else {
                                Err(Error("Image sampling options must be constants".into()))
                            }
                        };
                        algorithm.raster =
                            Some(std::sync::Arc::new(crate::raster::Raster::from_image(
                                t.image.as_ref(),
                                constant("address")?,
                                constant("filter")?,
                                constant("channel")?,
                                constant("uvMode")? == 0,
                            )?));
                    } else if t.image.is_some() {
                        return Err(Error("Only Image / Tile Source accepts image data".into()));
                    }
                    program.nodes.push(Node {
                        algorithm,
                        code: code.into(),
                        kernel,
                        parameters,
                        input,
                        coordinates,
                        row_id: r.id.clone(),
                        tile_index,
                        combine: t.combine,
                        has_distance: false,
                        value_dependencies: [1; 4],
                        distance_dependencies: 1,
                        uniform_distance: false,
                        cache_input: false,
                        image_execution: metadata.image_execution.clone(),
                    });
                    previous = Some(node_id);
                }
            }
            let mut defaults = vec![];
            for (name, values) in &m.defaults {
                let channel = Channel::parse(name)?;
                if channel == Channel::Distance
                    || values.len() != channel.width()
                    || values.iter().any(|v| !v.is_finite())
                {
                    return Err(Error(format!(
                        "Invalid material default for {name:?}: expected {} finite values",
                        channel.width()
                    )));
                }
                if defaults.iter().any(|(c, _)| *c == channel) {
                    return Err(Error(format!("Duplicate material default {name:?}")));
                }
                defaults.push((channel, values.clone()));
            }
            let mut outputs = vec![];
            for o in &m.outputs {
                let channel = Channel::parse(&o.channel)?;
                if outputs.iter().any(|(c, _)| *c == channel) {
                    return Err(Error(format!("Duplicate {} output", channel.name())));
                }
                outputs.push((channel, resolve(&o.source, &rows, channel.width())?));
            }
            program.materials.push(Material {
                tiling: {
                    m.animation.validate()?;
                    m.tiling.validate()?
                },
                defaults,
                id: m.id.clone(),
                label: m.label.clone(),
                outputs,
                rows: m.rows.iter().map(|r| (r.id.clone(), rows[&r.id])).collect(),
            });
        }
        crate::material_mix::lower(&mut program, project, limits.nodes)?;
        crate::fields::resolve(&mut program)?;
        crate::scope::specialize_cell_data(&mut program)?;
        if program.nodes.len() > limits.nodes.saturating_mul(16) {
            return Err(Error(
                "Named field expansion exceeds the internal node budget".into(),
            ));
        }
        program.costs.resize(program.nodes.len(), Cost::default());
        let mut state = vec![0u8; program.nodes.len()];
        fn visit(
            p: &mut Program,
            id: NodeId,
            state: &mut [u8],
            limits: Limits,
            stack_depth: usize,
        ) -> Result<Cost> {
            if state[id.0] == 1 {
                return Err(Error(format!(
                    "Dependency cycle involving row {:?}",
                    p.nodes[id.0].row_id
                )));
            }
            if state[id.0] == 2 {
                return Ok(p.costs[id.0]);
            }
            if stack_depth > limits.depth {
                return Err(Error("Dependency depth budget exceeded".into()));
            }
            state[id.0] = 1;
            let mut cost = Cost {
                depth: 1,
                expanded_calls: 1,
                operations: p.nodes[id.0].algorithm.operations(),
            };
            for dep in p.nodes[id.0].dependencies() {
                let child = visit(p, dep, state, limits, stack_depth + 1)?;
                cost.depth = cost.depth.max(child.depth.saturating_add(1));
                cost.expanded_calls = cost.expanded_calls.saturating_add(child.expanded_calls);
            }
            if cost.depth > limits.depth || cost.expanded_calls > limits.expanded_calls {
                return Err(Error(format!(
                    "Complexity budget exceeded in row {:?}: depth {}, expanded calls {}. Split or bake this computation.",
                    p.nodes[id.0].row_id, cost.depth, cost.expanded_calls
                )));
            }
            state[id.0] = 2;
            p.costs[id.0] = cost;
            p.order.push(id);
            Ok(cost)
        }
        for i in 0..program.nodes.len() {
            visit(&mut program, NodeId(i), &mut state, limits, 1)?;
        }
        crate::scope::analyze(&mut program);
        for id in program.order.clone() {
            let n = &program.nodes[id.0];
            for r in
                n.input
                    .iter()
                    .chain(n.coordinates.iter())
                    .chain(n.parameters.iter().filter_map(|p| {
                        if let Binding::From(r) = &p.binding {
                            Some(r)
                        } else {
                            None
                        }
                    }))
            {
                if r.distance && !program.nodes[r.node.0].has_distance {
                    return Err(Error(format!(
                        "Row {:?} has no valid distance field",
                        program.nodes[r.node.0].row_id
                    )));
                }
            }
            let incoming = n
                .input
                .as_ref()
                .is_some_and(|r| program.nodes[r.node.0].has_distance);
            if (n.combine != ast::Combine::Replace) && !incoming {
                return Err(Error(format!(
                    "{} in row {:?} requires an incoming distance field",
                    n.kernel.metadata().name,
                    n.row_id
                )));
            }
            let preserves = match n.kernel {
                kernel if kernel.is_shape() => {
                    n.algorithm.shape.is_some()
                        && n.parameters.iter().all(|p| {
                            crate::scope::binding(&program, &p.binding)
                                .iter()
                                .all(|d| d & 1 == 0)
                        })
                }
                Kernel::Disc => false,
                Kernel::Transform => {
                    incoming
                        && n.uniform_distance
                        && n.code == crate::tilelang::source(Kernel::Transform)
                }
                _ => incoming,
            };
            program.nodes[id.0].has_distance =
                preserves && n.coordinates.is_none() && !n.kernel.metadata().clears_distance;
        }
        for m in &program.materials {
            for (channel, r) in &m.outputs {
                if r.distance && !program.nodes[r.node.0].has_distance {
                    return Err(Error(format!(
                        "Output refers to row without a valid distance field: {:?}",
                        program.nodes[r.node.0].row_id
                    )));
                }
                if *channel == Channel::Distance && !r.distance {
                    return Err(Error("Distance output must select distance".into()));
                }
            }
        }
        // Image-only Border samples its source 48 more times. Account for those
        // calls after capability resolution so the distance fast path stays cheap.
        for id in &program.order {
            let n = &program.nodes[id.0];
            let raster_border = n.kernel == Kernel::Border
                && n.input
                    .as_ref()
                    .is_some_and(|r| !program.nodes[r.node.0].has_distance);
            let mut calls = 1usize;
            let params = n
                .parameters
                .iter()
                .map(|p| {
                    if let Binding::Constant(v) = &p.binding {
                        Some(v.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let (mut operations, sampling) = n.algorithm.cost(&params);
            let inline_calls = 1usize.saturating_add(
                n.dependencies()
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        program.costs[d.0].expanded_calls.saturating_mul(
                            if i == 0 && n.input.is_some() {
                                sampling
                            } else {
                                1
                            },
                        )
                    })
                    .fold(0usize, usize::saturating_add),
            );
            let inline_operations = operations.saturating_add(
                n.dependencies()
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        program.costs[d.0].operations.saturating_mul(
                            if i == 0 && n.input.is_some() {
                                sampling
                            } else {
                                1
                            },
                        )
                    })
                    .fold(0usize, usize::saturating_add),
            );
            let cache_input = crate::execution::is_filter(&program, *id)
                && (inline_calls > limits.expanded_calls
                    || inline_operations > limits.expanded_calls.saturating_mul(32));
            for (i, dep) in n.dependencies().iter().enumerate() {
                let cached = i == 0 && n.input.is_some() && cache_input;
                let count = if i == 0 && n.input.is_some() {
                    if raster_border { 49 } else { sampling }
                } else {
                    1
                };
                let dependency = if cached {
                    Cost {
                        operations: 8,
                        expanded_calls: 1,
                        depth: 1,
                    }
                } else {
                    program.costs[dep.0]
                };
                operations = operations.saturating_add(dependency.operations.saturating_mul(count));
                calls = calls.saturating_add(dependency.expanded_calls.saturating_mul(count));
            }
            if calls > limits.expanded_calls {
                return Err(Error(format!(
                    "Sampling complexity budget exceeded in row {:?}: {calls} expanded calls",
                    n.row_id
                )));
            }
            if operations > limits.expanded_calls.saturating_mul(32) {
                return Err(Error(format!(
                    "Tile instruction complexity budget exceeded in row {:?}: {operations} operations",
                    n.row_id
                )));
            }
            program.nodes[id.0].cache_input = cache_input;
            program.costs[id.0].operations = operations;
            program.costs[id.0].expanded_calls = calls;
        }
        Ok(program)
    }
    pub fn tile(&self, material: &str, row: &str, tile: usize) -> Result<NodeId> {
        let m = self.material(material)?;
        let end = m
            .rows
            .iter()
            .find(|(r, _)| r == row)
            .ok_or_else(|| Error(format!("Unknown row {row:?}")))?
            .1;
        let last = self.nodes[end.0].tile_index;
        if tile > last {
            return Err(Error("Unknown tile index".into()));
        }
        Ok(NodeId(end.0 - last + tile))
    }
    pub fn material(&self, id: &str) -> Result<&Material> {
        self.materials
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| Error(format!("Unknown material {id:?}")))
    }
}
