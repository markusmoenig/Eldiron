//! Host-independent particle authoring modules. Prefabs, entities and other graph hosts
//! can register these same definitions and compile into the existing emitter format.
use crate::graph_authoring::*;
use rusterix::ParticleEmitterDef;
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
                ("spread", "Spread", number(0.1, 0., 3.14, 0.01)),
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
    vec![Box::new(Emission), Box::new(Motion), Box::new(Lifetime)]
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
    for node in nodes {
        let module = modules
            .iter()
            .find(|module| module.definition().definition == node.definition)
            .ok_or_else(|| format!("Unknown particle node: {}", node.title))?;
        module.apply(node, &mut emitter)?;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modules_keep_unexposed_settings_and_reject_invalid_ranges() {
        let mut base = ParticleEmitterDef::default();
        base.gravity = [0., -0.5, 0.];
        base.flame_base = true;
        let defs = definitions();
        let mut motion = defs.node("particle_motion").unwrap().instantiate([0., 0.]);
        set(&mut motion, "speed_min", number(0.2, 0., 20., 0.01));
        let result = compile(&[motion.clone()], &base).unwrap();
        assert_eq!(result.gravity, base.gravity);
        assert!(result.flame_base);
        assert_eq!(result.speed_range, (0.2, 1.));
        set(&mut motion, "speed_max", number(0.1, 0., 20., 0.01));
        assert!(compile(&[motion], &base).is_err());
    }
}
