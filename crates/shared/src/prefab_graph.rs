//! Independent prefab part branches, compiled into ordinary runtime assets.
//! Meshes, paint ownership, attachments and component IDs remain stable.
use crate::graph_authoring::*;
use rusterix::{
    BlockPropAsset, BlockPropComponent, BlockPropInteractionTarget, BlockPropSemanticShape, Value,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use theframework::{prelude::Uuid, thegraph::*};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PrefabGraphs {
    pub branches: Vec<PrefabBranch>,
    /// Component seeds retain paired leaves, motion axes and future/unexposed settings.
    #[serde(default)]
    pub door_components: Vec<BlockPropComponent>,
    #[serde(default)]
    pub door_targets: Vec<BlockPropInteractionTarget>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrefabBranch {
    pub part_id: Uuid,
    pub graph: GraphDocument,
}
pub trait PrefabNodeModule {
    fn definition(&self) -> GraphNode;
    fn apply(
        &self,
        node: &GraphNode,
        part_id: Uuid,
        asset: &mut BlockPropAsset,
    ) -> Result<(), String>;
}
struct Part;
struct Transform;
struct Door;
impl PrefabNodeModule for Part {
    fn definition(&self) -> GraphNode {
        node(
            "prefab_part",
            "Part",
            [47, 124, 136, 255],
            vec![("name", "Name", GraphControlValue::Text(String::new()))],
            true,
            "prefab.part",
        )
    }
    fn apply(&self, node: &GraphNode, id: Uuid, asset: &mut BlockPropAsset) -> Result<(), String> {
        let part = asset
            .parts
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or("Missing prefab part")?;
        part.name = text(node, "name")?;
        Ok(())
    }
}
impl PrefabNodeModule for Transform {
    fn definition(&self) -> GraphNode {
        node(
            "prefab_transform",
            "Transform",
            [116, 87, 147, 255],
            vec![
                ("pivot_x", "Pivot X", GraphControlValue::Text("0".into())),
                ("pivot_y", "Pivot Y", GraphControlValue::Text("0".into())),
                ("pivot_z", "Pivot Z", GraphControlValue::Text("0".into())),
            ],
            false,
            "prefab.part",
        )
    }
    fn apply(&self, node: &GraphNode, id: Uuid, asset: &mut BlockPropAsset) -> Result<(), String> {
        let pivot = [
            read_number(node, "pivot_x")?,
            read_number(node, "pivot_y")?,
            read_number(node, "pivot_z")?,
        ];
        asset
            .parts
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or("Missing prefab part")?
            .pivot = pivot;
        Ok(())
    }
}
impl PrefabNodeModule for Door {
    fn definition(&self) -> GraphNode {
        node(
            "prefab_door",
            "Door",
            [45, 130, 99, 255],
            vec![
                (
                    "motion",
                    "Motion",
                    GraphControlValue::Choice {
                        options: vec!["Swing".into(), "Slide".into()],
                        selected: 0,
                    },
                ),
                ("angle_degrees", "Open angle", number(90., -180., 180., 1.)),
                (
                    "slide_distance",
                    "Slide distance",
                    number(1., 0., 20., 0.01),
                ),
                (
                    "interaction_range",
                    "Use distance",
                    number(2., 0., 20., 0.1),
                ),
            ],
            false,
            "prefab.part",
        )
    }
    fn apply(
        &self,
        node: &GraphNode,
        part_id: Uuid,
        asset: &mut BlockPropAsset,
    ) -> Result<(), String> {
        if !asset.components.iter().any(|c| c.id == node.id) {
            let mut component = BlockPropComponent::new("Door");
            component.id = node.id;
            component.properties.set("part_id", Value::Id(part_id));
            component.properties.set("duration", Value::Float(0.35));
            let axis = asset
                .find_part(part_id)
                .and_then(|part| {
                    part.geometry_source
                        .geometry_objects()
                        .iter()
                        .find_map(|object| object.properties.get_vec3("fitted_motion_axis"))
                })
                .unwrap_or([1., 0., 0.]);
            component.properties.set("slide_axis", Value::Vec3(axis));
            asset.components.push(component);
        }
        let component = asset
            .components
            .iter_mut()
            .find(|c| c.id == node.id && c.kind == "Door")
            .ok_or("Invalid door component")?;
        if component.properties.get_id("part_id") != Some(part_id) {
            return Err("Door belongs to a different part".into());
        }
        let motion = match value(node, "motion")? {
            GraphControlValue::Choice { options, selected } => {
                options.get(*selected).ok_or("Invalid door motion")?.clone()
            }
            _ => return Err("Invalid door motion".into()),
        };
        if !matches!(motion.as_str(), "Swing" | "Slide") {
            return Err("Invalid door motion".into());
        }
        if component.properties.get("motion").is_some() || motion != "Swing" {
            component.properties.set("motion", Value::Str(motion));
        }
        for (key, default) in [
            ("angle_degrees", 90.),
            ("slide_distance", 1.),
            ("interaction_range", 2.),
        ] {
            let value = read_number(node, key)?;
            if component.properties.get(key).is_some() || value != default {
                component.properties.set(key, Value::Float(value));
            }
        }
        let secondary = component.properties.get_id("secondary_part_id");
        if asset.default_state.get("open").is_none() {
            asset.default_state.set("open", Value::Bool(false));
        }
        for id in std::iter::once(part_id).chain(secondary) {
            if !asset
                .interaction_targets
                .iter()
                .any(|target| target.component_id == Some(node.id) && target.part_id == id)
            {
                let part = asset
                    .find_part(id)
                    .ok_or("Door references a missing part")?;
                let target = BlockPropInteractionTarget {
                    id: Uuid::new_v4(),
                    name: "Door Interaction".into(),
                    part_id: id,
                    shape: BlockPropSemanticShape::Part,
                    interaction_anchor: part.pivot,
                    facing_direction: [0., 0., 1.],
                    component_id: Some(node.id),
                };
                asset.interaction_targets.push(target);
            }
        }
        Ok(())
    }
}
pub fn modules() -> Vec<Box<dyn PrefabNodeModule>> {
    vec![Box::new(Part), Box::new(Transform), Box::new(Door)]
}
pub fn definitions() -> GraphDefinitions {
    let mut defs = GraphDefinitions::default();
    for module in modules() {
        let template = module.definition();
        defs.register_node(GraphNodeDefinition::from_template(
            template.definition.as_deref().unwrap(),
            "Prefab",
            &template,
        ))
        .expect("valid prefab definition");
    }
    defs
}
impl PrefabGraphs {
    pub fn import(asset: &BlockPropAsset) -> Self {
        let defs = definitions();
        let branches =
            asset
                .parts
                .iter()
                .map(|part| {
                    let mut graph = GraphDocument::default();
                    let mut root = defs.node("prefab_part").unwrap().instantiate([0., 0.]);
                    root.id = part.id;
                    set(
                        &mut root,
                        "name",
                        GraphControlValue::Text(part.name.clone()),
                    );
                    let mut transform = defs
                        .node("prefab_transform")
                        .unwrap()
                        .instantiate([280., 0.]);
                    for (key, val) in ["pivot_x", "pivot_y", "pivot_z"]
                        .into_iter()
                        .zip(part.pivot)
                    {
                        set(
                            &mut transform,
                            key,
                            GraphControlValue::Text(val.to_string()),
                        );
                    }
                    connect(&mut graph, &root, &transform);
                    graph.nodes = vec![root, transform.clone()];
                    for component in asset.components.iter().filter(|c| {
                        c.kind == "Door" && c.properties.get_id("part_id") == Some(part.id)
                    }) {
                        let mut door = defs.node("prefab_door").unwrap().instantiate([560., 0.]);
                        door.id = component.id;
                        set(
                            &mut door,
                            "motion",
                            GraphControlValue::Choice {
                                options: vec!["Swing".into(), "Slide".into()],
                                selected: usize::from(
                                    component
                                        .properties
                                        .get_str_default("motion", "Swing".into())
                                        == "Slide",
                                ),
                            },
                        );
                        for (key, default, min, max, step) in [
                            ("angle_degrees", 90., -180., 180., 1.),
                            ("slide_distance", 1., 0., 20., 0.01),
                            ("interaction_range", 2., 0., 20., 0.1),
                        ] {
                            set(
                                &mut door,
                                key,
                                number(
                                    component.properties.get_float_default(key, default),
                                    min,
                                    max,
                                    step,
                                ),
                            );
                        }
                        connect(&mut graph, &transform, &door);
                        graph.nodes.push(door);
                    }
                    PrefabBranch {
                        part_id: part.id,
                        graph,
                    }
                })
                .collect();
        Self {
            branches,
            door_components: asset
                .components
                .iter()
                .filter(|c| c.kind == "Door")
                .cloned()
                .collect(),
            door_targets: asset
                .interaction_targets
                .iter()
                .filter(|target| {
                    target.component_id.is_some_and(|id| {
                        asset
                            .components
                            .iter()
                            .any(|c| c.id == id && c.kind == "Door")
                    })
                })
                .cloned()
                .collect(),
        }
    }
    /// Remove the former pass-through geometry nodes without reconnecting cut branches.
    /// Numeric text retains the full stored pivot precision rather than slider increments.
    pub fn normalize(&mut self) {
        for branch in &mut self.branches {
            let graph = &mut branch.graph;
            while let Some(index) = graph
                .nodes
                .iter()
                .position(|n| n.definition.as_deref() == Some("prefab_geometry"))
            {
                let node = graph.nodes.remove(index);
                let ports: HashSet<_> = node.ports.iter().map(|p| p.id).collect();
                let incoming: Vec<_> = graph
                    .connections
                    .iter()
                    .filter(|c| ports.contains(&c.to))
                    .cloned()
                    .collect();
                let outgoing: Vec<_> = graph
                    .connections
                    .iter()
                    .filter(|c| ports.contains(&c.from))
                    .cloned()
                    .collect();
                graph
                    .connections
                    .retain(|c| !ports.contains(&c.from) && !ports.contains(&c.to));
                for input in &incoming {
                    for output in &outgoing {
                        graph.connections.push(GraphConnection {
                            id: Uuid::new_v4(),
                            from: input.from,
                            to: output.to,
                        });
                    }
                }
            }
            for node in &mut graph.nodes {
                if node.definition.as_deref() == Some("prefab_transform") {
                    for row in &mut node.rows {
                        if matches!(row.key.as_deref(), Some("pivot_x" | "pivot_y" | "pivot_z")) {
                            if let GraphControlValue::Number { value, .. } = &row.value {
                                row.value = GraphControlValue::Text(value.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    /// Adds new mesh parts without discarding authored branches or layouts.
    pub fn reconcile(&mut self, asset: &BlockPropAsset) {
        self.normalize();
        self.branches
            .retain(|branch| asset.parts.iter().any(|part| part.id == branch.part_id));
        for branch in Self::import(asset).branches {
            if !self.branches.iter().any(|b| b.part_id == branch.part_id) {
                self.branches.push(branch);
            }
        }
    }
    pub fn compile(&self, base: &BlockPropAsset) -> Result<BlockPropAsset, String> {
        self.compile_with_modules(base, &modules())
    }
    /// Register new authoring capabilities through modules, retaining transactional compilation.
    pub fn compile_with_modules(
        &self,
        base: &BlockPropAsset,
        modules: &[Box<dyn PrefabNodeModule>],
    ) -> Result<BlockPropAsset, String> {
        let mut normalized = self.clone();
        normalized.normalize();
        let registry: HashMap<_, _> = modules
            .iter()
            .map(|m| (m.definition().definition.unwrap(), m))
            .collect();
        let mut asset = base.clone();
        asset
            .components
            .retain(|c| !self.door_components.iter().any(|seed| seed.id == c.id));
        for branch in &normalized.branches {
            let doc = &branch.graph;
            let mut structure = doc.clone();
            structure.connections.clear();
            let mut links = HashSet::new();
            for connection in &doc.connections {
                structure.validate_connection(connection.from, connection.to)?;
                if !links.insert((connection.from, connection.to)) {
                    return Err("Duplicate prefab connection".into());
                }
            }
            let mut next = Some(branch.part_id);
            let mut visited = HashSet::new();
            while let Some(id) = next {
                if !visited.insert(id) {
                    return Err("Prefab branch contains a cycle".into());
                }
                let node = doc
                    .nodes
                    .iter()
                    .find(|n| n.id == id)
                    .ok_or("Missing prefab node")?;
                let module = registry
                    .get(node.definition.as_deref().unwrap_or_default())
                    .ok_or_else(|| format!("Unknown prefab node: {}", node.title))?;
                if let Some(seed) = self.door_components.iter().find(|seed| seed.id == node.id) {
                    asset.components.push(seed.clone());
                    for target in self
                        .door_targets
                        .iter()
                        .filter(|target| target.component_id == Some(seed.id))
                    {
                        if !asset
                            .interaction_targets
                            .iter()
                            .any(|existing| existing.id == target.id)
                        {
                            asset.interaction_targets.push(target.clone());
                        }
                    }
                }
                module.apply(node, branch.part_id, &mut asset)?;
                let outgoing: Vec<_> = doc
                    .connections
                    .iter()
                    .filter(|c| {
                        node.ports
                            .iter()
                            .any(|p| p.id == c.from && p.direction == PortDirection::Output)
                    })
                    .collect();
                if outgoing.len() > 1 {
                    return Err("Prefab part chains require one downstream connection".into());
                }
                next = outgoing
                    .first()
                    .and_then(|c| {
                        doc.nodes.iter().find(|n| {
                            n.ports
                                .iter()
                                .any(|p| p.id == c.to && p.direction == PortDirection::Input)
                        })
                    })
                    .map(|n| n.id);
            }
        }
        asset.interaction_targets.retain(|target| {
            !target.component_id.is_some_and(|id| {
                self.door_components.iter().any(|seed| seed.id == id)
                    && !asset.components.iter().any(|c| c.id == id)
            })
        });
        Ok(asset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn gate() -> crate::project::Project {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects/Gate.eldiron");
        crate::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap()
    }
    #[test]
    fn gate_import_preserves_runtime_assets_and_independent_branches() {
        let project = gate();
        assert_eq!(project.block_props.len(), 2);
        for asset in project.block_props.values() {
            let imported = PrefabGraphs::import(asset);
            assert_eq!(imported.compile(asset).unwrap(), *asset);
            assert_eq!(imported.branches.len(), asset.parts.len());
            let persisted = &project.prefab_graphs[&asset.id];
            assert_eq!(persisted.compile(asset).unwrap(), *asset);
            let serialized = serde_json::to_string(persisted).unwrap();
            let restored: PrefabGraphs = serde_json::from_str(&serialized).unwrap();
            assert_eq!(restored, *persisted);
            let mut ids = HashSet::new();
            for branch in &restored.branches {
                for node in &branch.graph.nodes {
                    assert!(ids.insert(node.id));
                }
            }
        }
    }
    #[test]
    fn cutting_and_reconnecting_door_retains_paired_leaf_and_paint_owners() {
        let project = gate();
        let asset = project
            .block_props
            .values()
            .find(|a| a.parts.len() == 3)
            .unwrap();
        let mut graphs = PrefabGraphs::import(asset);
        let branch = graphs
            .branches
            .iter_mut()
            .find(|b| {
                b.graph
                    .nodes
                    .iter()
                    .any(|n| n.definition.as_deref() == Some("prefab_door"))
            })
            .unwrap();
        let link = branch.graph.connections.pop().unwrap();
        let compiled = graphs.compile(asset).unwrap();
        assert!(compiled.components.iter().all(|c| c.kind != "Door"));
        assert_eq!(compiled.parts, asset.parts);
        graphs
            .branches
            .iter_mut()
            .find(|b| {
                b.graph
                    .nodes
                    .iter()
                    .any(|n| n.definition.as_deref() == Some("prefab_door"))
            })
            .unwrap()
            .graph
            .connections
            .push(link);
        assert_eq!(graphs.compile(&compiled).unwrap(), *asset);
    }
    #[test]
    fn connected_new_door_creates_stable_interaction_binding() {
        let project = gate();
        let mut asset = project
            .block_props
            .values()
            .find(|a| a.parts.len() == 1)
            .unwrap()
            .clone();
        asset.components.clear();
        asset.interaction_targets.clear();
        let mut graphs = PrefabGraphs::import(&asset);
        let branch = &mut graphs.branches[0];
        let tail = branch.graph.nodes.last().unwrap().clone();
        let door = definitions()
            .node("prefab_door")
            .unwrap()
            .instantiate([840., 0.]);
        connect(&mut branch.graph, &tail, &door);
        branch.graph.nodes.push(door.clone());
        let compiled = graphs.compile(&asset).unwrap();
        assert_eq!(compiled.components.len(), 1);
        assert_eq!(compiled.components[0].id, door.id);
        assert_eq!(compiled.interaction_targets.len(), 1);
        assert_eq!(compiled.interaction_targets[0].component_id, Some(door.id));
        assert_eq!(graphs.compile(&compiled).unwrap(), compiled);
    }
    #[test]
    fn legacy_geometry_migration_preserves_connections_and_precise_pivots() {
        let project = gate();
        let asset = project.block_props.values().next().unwrap();
        let mut graphs = PrefabGraphs::import(asset);
        let graph = &mut graphs.branches[0].graph;
        graph.connections.remove(0);
        let root = graph.nodes[0].clone();
        let transform = graph.nodes[1].clone();
        let geometry = node(
            "prefab_geometry",
            "Geometry",
            [0, 0, 0, 255],
            vec![],
            false,
            "prefab.part",
        );
        connect(graph, &root, &geometry);
        connect(graph, &geometry, &transform);
        graph.nodes.push(geometry);
        graphs.normalize();
        assert!(
            !graphs.branches[0]
                .graph
                .nodes
                .iter()
                .any(|n| n.definition.as_deref() == Some("prefab_geometry"))
        );
        assert_eq!(graphs.compile(asset).unwrap(), *asset);
        let graph = &mut graphs.branches[0].graph;
        let transform = graph
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("prefab_transform"))
            .unwrap();
        set(
            transform,
            "pivot_x",
            GraphControlValue::Text("0.1234567".into()),
        );
        let compiled = graphs.compile(asset).unwrap();
        assert_eq!(compiled.parts[0].pivot[0], 0.1234567);
        set(
            graphs.branches[0]
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.definition.as_deref() == Some("prefab_transform"))
                .unwrap(),
            "pivot_x",
            GraphControlValue::Text("NaN".into()),
        );
        assert!(graphs.compile(asset).is_err());
    }
    #[test]
    fn invalid_branch_is_transactional() {
        let project = gate();
        let asset = project.block_props.values().next().unwrap();
        let mut graphs = PrefabGraphs::import(asset);
        let root = graphs.branches[0].graph.nodes[0].id;
        graphs.branches[0].graph.nodes.retain(|n| n.id != root);
        assert!(graphs.compile(asset).is_err());
        assert_eq!(project.block_props[&asset.id], *asset);
    }
}
