//! Host-independent particle authoring modules. Prefabs, entities and other graph hosts
//! can register these same definitions and compile into the existing emitter format.
use crate::graph_authoring::*;
use serde::{Deserialize, Serialize};
use vek::Vec3;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticleEmitterDef {
    pub direction: Vec3<f32>,
    pub spread: f32,
    pub rate: f32,
    pub color: [u8; 4],
    pub color_ramp: Option<[[u8; 4]; 4]>,
    pub color_variation: u8,
    pub lifetime_range: (f32, f32),
    pub radius_range: (f32, f32),
    pub speed_range: (f32, f32),
    pub spawn_area: [f32; 3],
    pub emission_shape: String,
    pub flame_base: bool,
    pub size_curve: [f32; 4],
    pub opacity_curve: [f32; 4],
    pub gravity: [f32; 3],
    pub turbulence: f32,
}
impl Default for ParticleEmitterDef {
    fn default() -> Self {
        Self {
            direction: Vec3::unit_y(),
            spread: std::f32::consts::FRAC_PI_4,
            rate: 30.,
            color: [255, 160, 0, 255],
            color_ramp: None,
            color_variation: 30,
            lifetime_range: (0.5, 1.5),
            radius_range: (0.05, 0.15),
            speed_range: (0.5, 1.5),
            spawn_area: [0.; 3],
            emission_shape: "Box".into(),
            flame_base: false,
            size_curve: [1., 0.92, 0.68, 0.28],
            opacity_curve: [1., 0.9, 0.55, 0.],
            gravity: [0.; 3],
            turbulence: 0.,
        }
    }
}
use theframework::thegraph::*;

pub trait ParticleNodeModule {
    fn definition(&self) -> GraphNode;
    fn apply(&self, node: &GraphNode, emitter: &mut ParticleEmitterDef) -> Result<(), String>;
}
struct Emission;
struct Motion;
struct Lifetime;
impl ParticleNodeModule for Emission {
    fn definition(&self) -> GraphNode {
        node(
            "particle_emission",
            "Particle Emission",
            [168, 112, 52, 255],
            vec![
                ("rate", "Rate", number(10., 0., 500., 1.)),
                (
                    "spread",
                    "Spread",
                    number(0.1, 0., std::f32::consts::PI, 0.01),
                ),
            ],
            false,
            "particle.settings",
        )
    }
    fn apply(&self, node: &GraphNode, emitter: &mut ParticleEmitterDef) -> Result<(), String> {
        emitter.rate = read_number(node, "rate")?.max(0.);
        emitter.spread = read_number(node, "spread")?.max(0.);
        Ok(())
    }
}
impl ParticleNodeModule for Motion {
    fn definition(&self) -> GraphNode {
        node(
            "particle_motion",
            "Particle Motion",
            [52, 112, 158, 255],
            vec![
                ("speed_min", "Minimum speed", number(0.1, 0., 20., 0.01)),
                ("speed_max", "Maximum speed", number(1., 0., 20., 0.01)),
                ("turbulence", "Turbulence", number(0., 0., 10., 0.01)),
            ],
            false,
            "particle.settings",
        )
    }
    fn apply(&self, node: &GraphNode, emitter: &mut ParticleEmitterDef) -> Result<(), String> {
        let min = read_number(node, "speed_min")?.max(0.);
        let max = read_number(node, "speed_max")?.max(0.);
        if min > max {
            return Err("Particle Motion: minimum speed exceeds maximum".into());
        }
        emitter.speed_range = (min, max);
        emitter.turbulence = read_number(node, "turbulence")?.max(0.);
        Ok(())
    }
}
impl ParticleNodeModule for Lifetime {
    fn definition(&self) -> GraphNode {
        node(
            "particle_lifetime",
            "Particle Lifetime",
            [116, 87, 147, 255],
            vec![
                ("min", "Minimum seconds", number(1., 0.01, 60., 0.05)),
                ("max", "Maximum seconds", number(2., 0.01, 60., 0.05)),
            ],
            false,
            "particle.settings",
        )
    }
    fn apply(&self, node: &GraphNode, emitter: &mut ParticleEmitterDef) -> Result<(), String> {
        let min = read_number(node, "min")?;
        let max = read_number(node, "max")?;
        if min <= 0. || min > max {
            return Err("Particle Lifetime: invalid lifetime range".into());
        }
        emitter.lifetime_range = (min, max);
        Ok(())
    }
}
pub fn modules() -> Vec<Box<dyn ParticleNodeModule>> {
    vec![
        Box::new(Emission),
        Box::new(Motion),
        Box::new(Lifetime),
        Box::new(Extra("particle_size")),
        Box::new(Extra("particle_color")),
        Box::new(Extra("particle_direction")),
        Box::new(Extra("particle_spawn")),
        Box::new(Extra("particle_curves")),
    ]
}
/// Applies an already ordered chain transactionally, keeping all unexposed emitter settings.
pub fn compile(
    nodes: &[GraphNode],
    base: &ParticleEmitterDef,
) -> Result<ParticleEmitterDef, String> {
    compile_with_modules(nodes, base, &modules())
}
/// Hosts may add modules without adding cases to the compiler or copying emitter logic.
pub fn compile_with_modules(
    nodes: &[GraphNode],
    base: &ParticleEmitterDef,
    modules: &[Box<dyn ParticleNodeModule>],
) -> Result<ParticleEmitterDef, String> {
    let mut emitter = base.clone();
    for node in nodes.iter().filter(|n| !n.disabled) {
        let module = modules
            .iter()
            .find(|module| module.definition().definition == node.definition)
            .ok_or_else(|| format!("Unknown particle node: {}", node.title))?;
        module.apply(node, &mut emitter)?;
    }
    validate_settings(&emitter)?;
    Ok(emitter)
}

pub fn definitions() -> GraphDefinitions {
    let mut definitions = GraphDefinitions::default();
    for module in modules() {
        let node = module.definition();
        definitions
            .register_node(GraphNodeDefinition::from_template(
                node.definition.as_deref().unwrap(),
                "Particles",
                &node,
            ))
            .expect("valid shared particle definition");
    }
    definitions
}

struct Extra(&'static str);
impl ParticleNodeModule for Extra {
    fn definition(&self) -> GraphNode {
        let (title, rows) = match self.0 {
            "particle_size" => (
                "Particle Size",
                vec![
                    ("min", "Minimum radius", number(0.05, 0.001, 10., 0.01)),
                    ("max", "Maximum radius", number(0.15, 0.001, 10., 0.01)),
                ],
            ),
            "particle_color" => (
                "Particle Color",
                vec![
                    (
                        "color_0",
                        "Birth color (RGBA)",
                        GraphControlValue::Text("#ffa000ff".into()),
                    ),
                    (
                        "color_1",
                        "Early color (RGBA)",
                        GraphControlValue::Text("#ffa000ff".into()),
                    ),
                    (
                        "color_2",
                        "Late color (RGBA)",
                        GraphControlValue::Text("#ffa000ff".into()),
                    ),
                    (
                        "color_3",
                        "End color (RGBA)",
                        GraphControlValue::Text("#ffa000ff".into()),
                    ),
                    ("variation", "Color variation", number(30., 0., 255., 1.)),
                ],
            ),
            "particle_direction" => (
                "Particle Direction",
                vec![
                    ("x", "Direction X", number(0., -1., 1., 0.01)),
                    ("y", "Direction Y", number(1., -1., 1., 0.01)),
                    ("z", "Direction Z", number(0., -1., 1., 0.01)),
                    ("gravity_x", "Gravity X", number(0., -20., 20., 0.01)),
                    ("gravity_y", "Gravity Y", number(0., -20., 20., 0.01)),
                    ("gravity_z", "Gravity Z", number(0., -20., 20., 0.01)),
                ],
            ),
            "particle_spawn" => (
                "Particle Spawn Area",
                vec![
                    ("x", "Width", number(0., 0., 20., 0.01)),
                    ("y", "Height", number(0., 0., 20., 0.01)),
                    ("z", "Depth", number(0., 0., 20., 0.01)),
                    (
                        "shape",
                        "Distribution",
                        GraphControlValue::Choice {
                            options: vec!["Point".into(), "Box".into(), "Surface".into()],
                            selected: 1,
                        },
                    ),
                    ("flame_base", "Flame base", GraphControlValue::Toggle(false)),
                ],
            ),
            _ => (
                "Particle Lifetime Curves",
                (0..4)
                    .flat_map(|i| {
                        vec![
                            (
                                ["size_0", "size_1", "size_2", "size_3"][i],
                                ["Birth size", "Early size", "Late size", "End size"][i],
                                number([1., 0.92, 0.68, 0.28][i], 0., 10., 0.01),
                            ),
                            (
                                ["opacity_0", "opacity_1", "opacity_2", "opacity_3"][i],
                                [
                                    "Birth opacity",
                                    "Early opacity",
                                    "Late opacity",
                                    "End opacity",
                                ][i],
                                number([1., 0.9, 0.55, 0.][i], 0., 1., 0.01),
                            ),
                        ]
                    })
                    .collect(),
            ),
        };
        node(
            self.0,
            title,
            [78, 124, 146, 255],
            rows,
            false,
            "particle.settings",
        )
    }
    fn apply(&self, n: &GraphNode, e: &mut ParticleEmitterDef) -> Result<(), String> {
        match self.0 {
            "particle_size" => {
                let min = read_number(n, "min")?;
                let max = read_number(n, "max")?;
                if min <= 0. || min > max {
                    return Err("Particle Size: invalid radius range".into());
                }
                e.radius_range = (min, max);
            }
            "particle_color" => {
                let mut ramp = [[0; 4]; 4];
                for (i, key) in ["color_0", "color_1", "color_2", "color_3"]
                    .iter()
                    .enumerate()
                {
                    ramp[i] = parse_color(&text(n, key)?)?;
                }
                e.color = ramp[0];
                e.color_ramp = Some(ramp);
                e.color_variation = read_number(n, "variation")?.clamp(0., 255.) as u8;
            }
            "particle_direction" => {
                let direction = Vec3::new(
                    read_number(n, "x")?,
                    read_number(n, "y")?,
                    read_number(n, "z")?,
                );
                if direction.magnitude_squared() < 1e-8 {
                    return Err("Particle Direction cannot be zero".into());
                }
                e.direction = direction.normalized();
                e.gravity = [
                    read_number(n, "gravity_x")?,
                    read_number(n, "gravity_y")?,
                    read_number(n, "gravity_z")?,
                ];
            }
            "particle_spawn" => {
                e.spawn_area = [
                    read_number(n, "x")?,
                    read_number(n, "y")?,
                    read_number(n, "z")?,
                ];
                if e.spawn_area.iter().any(|v| *v < 0.) {
                    return Err("Spawn area cannot be negative".into());
                }
                if let GraphControlValue::Choice { options, selected } = value(n, "shape")? {
                    e.emission_shape = options
                        .get(*selected)
                        .filter(|s| matches!(s.as_str(), "Point" | "Box" | "Surface"))
                        .ok_or("Invalid particle distribution")?
                        .clone();
                } else {
                    return Err("Missing particle distribution".into());
                }
                if let GraphControlValue::Toggle(v) = value(n, "flame_base")? {
                    e.flame_base = *v;
                }
            }
            _ => {
                for i in 0..4 {
                    e.size_curve[i] = read_number(n, ["size_0", "size_1", "size_2", "size_3"][i])?;
                    e.opacity_curve[i] =
                        read_number(n, ["opacity_0", "opacity_1", "opacity_2", "opacity_3"][i])?;
                    if e.size_curve[i] < 0. || !(0. ..=1.).contains(&e.opacity_curve[i]) {
                        return Err("Invalid particle lifetime curve".into());
                    }
                }
            }
        }
        Ok(())
    }
}
pub fn parse_color(raw: &str) -> Result<[u8; 4], String> {
    let raw = raw
        .trim()
        .strip_prefix('#')
        .ok_or("Use #RRGGBB or #RRGGBBAA")?;
    if !raw.is_ascii() || !matches!(raw.len(), 6 | 8) {
        return Err("Use #RRGGBB or #RRGGBBAA".into());
    }
    let mut color = [255; 4];
    for i in 0..raw.len() / 2 {
        color[i] =
            u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16).map_err(|_| "Invalid color channel")?;
    }
    Ok(color)
}

/// FX hosts use the same parameter modules as prefab particle authoring.
pub fn root_definition() -> GraphNode {
    let mut root = node(
        "rules_fx",
        "Particle FX Preset",
        [168, 112, 52, 255],
        vec![
            (
                "path",
                "Preset path",
                GraphControlValue::Text("/fx/presets/new_effect".into()),
            ),
            (
                "description",
                "Description",
                GraphControlValue::Text(String::new()),
            ),
            (
                "duration",
                "Effect duration (seconds)",
                number(0.55, 0.05, 60., 0.05),
            ),
            (
                "size_scale",
                "Effect size multiplier",
                number(1., 0.1, 10., 0.05),
            ),
            (
                "preview",
                "Preview",
                GraphControlValue::Preview {
                    asset: "rules_particle_preview".into(),
                    caption: "Particle FX".into(),
                },
            ),
        ],
        true,
        "particle.settings",
    );
    root.width = 320.;
    root
}

/// Compile the connected chain, rejecting cycles, merges and disconnected modules.
pub fn compile_branch(doc: &GraphDocument) -> Result<toml::Table, String> {
    let roots: Vec<_> = doc
        .nodes
        .iter()
        .filter(|n| n.definition.as_deref() == Some("rules_fx"))
        .collect();
    if roots.len() != 1 {
        return Err("An FX branch needs one Particle FX Preset".into());
    }
    let root = roots[0];
    let path = text(root, "path")?;
    if !path.starts_with("/fx/presets/") || path[12..].is_empty() || path[12..].contains('/') {
        return Err("FX paths must be /fx/presets/name".into());
    }
    let mut next = std::collections::BTreeMap::new();
    let mut incoming = std::collections::BTreeSet::new();
    for c in &doc.connections {
        let (from, output) = doc.port(c.from).ok_or("Missing particle output")?;
        let (to, input) = doc.port(c.to).ok_or("Missing particle input")?;
        if output.direction != PortDirection::Output
            || input.direction != PortDirection::Input
            || output.kind != "particle.settings"
            || input.kind != "particle.settings"
        {
            return Err("Particle modules need particle settings connections".into());
        }
        if next.insert(from.id, to.id).is_some() || !incoming.insert(to.id) {
            return Err(
                "Particle settings form a single chain; branches and merges are not allowed".into(),
            );
        }
    }
    let mut visited = std::collections::BTreeSet::from([root.id]);
    let mut ordered = Vec::new();
    let mut current = root.id;
    while let Some(id) = next.get(&current) {
        if !visited.insert(*id) {
            return Err("Particle connections must not loop".into());
        }
        let n = doc
            .nodes
            .iter()
            .find(|n| n.id == *id)
            .ok_or("Missing particle module")?;
        ordered.push(n.clone());
        current = *id;
    }
    if doc
        .nodes
        .iter()
        .any(|n| !n.disabled && !visited.contains(&n.id))
    {
        return Err("Connect every particle module to its FX preset".into());
    }
    let emitter = compile(&ordered, &ParticleEmitterDef::default())?;
    let duration = read_number(root, "duration")?;
    let scale = read_number(root, "size_scale")?;
    if duration < 0.05 || scale < 0.1 {
        return Err("FX duration and size must be positive".into());
    }
    let mut table = toml::Table::new();
    table.insert("kind".into(), toml::Value::String("particles".into()));
    table.insert(
        "description".into(),
        toml::Value::String(text(root, "description")?),
    );
    table.insert("duration".into(), toml::Value::Float(duration as f64));
    table.insert("size_scale".into(), toml::Value::Float(scale as f64));
    table.insert(
        "emitter".into(),
        toml::Value::try_from(&emitter).map_err(|e| e.to_string())?,
    );
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn branch() -> GraphDocument {
        let mut doc = GraphDocument::default();
        let mut previous = root_definition();
        doc.nodes.push(previous.clone());
        for module in modules() {
            let n = module.definition();
            connect(&mut doc, &previous, &n);
            doc.nodes.push(n.clone());
            previous = n;
        }
        doc
    }
    #[test]
    fn connected_modules_compile_round_trip_and_reject_invalid_drafts() {
        let doc = branch();
        let table = compile_branch(&doc).unwrap();
        let restored: toml::Table = toml::from_str(&toml::to_string(&table).unwrap()).unwrap();
        let emitter: ParticleEmitterDef = restored["emitter"].clone().try_into().unwrap();
        assert_eq!(emitter.rate, 10.);
        assert_eq!(emitter.spawn_area, [0.; 3]);
        let mut invalid = doc.clone();
        invalid.connections.pop();
        assert!(compile_branch(&invalid).is_err());
        let mut invalid = doc.clone();
        let n = invalid
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("particle_size"))
            .unwrap();
        set(n, "min", number(2., 0., 10., 0.1));
        n.disabled = true;
        assert!(compile_branch(&invalid).is_ok());
        let n = invalid
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("particle_size"))
            .unwrap();
        n.disabled = false;
        assert!(compile_branch(&invalid).is_err());
        let n = invalid
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("particle_size"))
            .unwrap();
        n.disabled = true;
        assert!(compile_branch(&invalid).is_ok());
        let mut cycle = doc.clone();
        let from = cycle.nodes.last().unwrap().clone();
        let to = cycle.nodes[1].clone();
        connect(&mut cycle, &from, &to);
        assert!(compile_branch(&cycle).is_err());
        let mut invalid = doc;
        let n = invalid
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("particle_color"))
            .unwrap();
        set(n, "color_0", GraphControlValue::Text("blue".into()));
        assert!(compile_branch(&invalid).is_err());
    }
}

/// Shared validation also covers generic Set Attribute authored emitter tables.
pub fn validate_settings(e: &ParticleEmitterDef) -> Result<(), String> {
    let ranges = [e.lifetime_range, e.radius_range, e.speed_range];
    let finite = [
        e.direction.x,
        e.direction.y,
        e.direction.z,
        e.spread,
        e.rate,
        e.turbulence,
    ]
    .into_iter()
    .chain(e.spawn_area)
    .chain(e.gravity)
    .chain(e.size_curve)
    .chain(e.opacity_curve)
    .chain(ranges.into_iter().flat_map(|(a, b)| [a, b]))
    .all(|v| v.is_finite());
    if !finite
        || e.direction.magnitude_squared() < 1e-8
        || !(0. ..=std::f32::consts::PI).contains(&e.spread)
        || e.rate < 0.
        || e.turbulence < 0.
        || ranges.iter().any(|(a, b)| a > b || *a < 0.)
        || e.lifetime_range.0 <= 0.
        || e.radius_range.0 <= 0.
        || e.spawn_area.iter().any(|v| *v < 0.)
        || e.size_curve.iter().any(|v| *v < 0.)
        || e.opacity_curve.iter().any(|v| !(0. ..=1.).contains(v))
        || !matches!(e.emission_shape.as_str(), "Point" | "Box" | "Surface")
    {
        return Err("Invalid particle emitter settings".into());
    }
    Ok(())
}
