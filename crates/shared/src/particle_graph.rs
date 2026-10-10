//! Reusable particle modules backed by the ruleset-independent authoring compiler.
#[cfg(test)]
use crate::graph_authoring::*;
use rusterix::ParticleEmitterDef;
use theframework::thegraph::*;
pub trait ParticleNodeModule {
    fn definition(&self) -> GraphNode;
    fn apply(&self, node: &GraphNode, emitter: &mut ParticleEmitterDef) -> Result<(), String>;
}
struct SharedModule(Box<dyn eldiron_ruleset::particle_graph::ParticleNodeModule>);
impl ParticleNodeModule for SharedModule {
    fn definition(&self) -> GraphNode {
        self.0.definition()
    }
    fn apply(&self, node: &GraphNode, emitter: &mut ParticleEmitterDef) -> Result<(), String> {
        let mut settings =
            serde_json::from_value(serde_json::to_value(&*emitter).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        self.0.apply(node, &mut settings)?;
        *emitter =
            serde_json::from_value(serde_json::to_value(settings).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        Ok(())
    }
}
pub fn modules() -> Vec<Box<dyn ParticleNodeModule>> {
    eldiron_ruleset::particle_graph::modules()
        .into_iter()
        .map(|m| Box::new(SharedModule(m)) as Box<dyn ParticleNodeModule>)
        .collect()
}
pub fn definitions() -> GraphDefinitions {
    eldiron_ruleset::particle_graph::definitions()
}
pub fn compile(
    nodes: &[GraphNode],
    base: &ParticleEmitterDef,
) -> Result<ParticleEmitterDef, String> {
    compile_with_modules(nodes, base, &modules())
}
pub fn compile_with_modules(
    nodes: &[GraphNode],
    base: &ParticleEmitterDef,
    modules: &[Box<dyn ParticleNodeModule>],
) -> Result<ParticleEmitterDef, String> {
    let mut emitter = base.clone();
    for node in nodes.iter().filter(|n| !n.disabled) {
        let module = modules
            .iter()
            .find(|m| m.definition().definition == node.definition)
            .ok_or_else(|| format!("Unknown particle node: {}", node.title))?;
        module.apply(node, &mut emitter)?;
    }
    Ok(emitter)
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
