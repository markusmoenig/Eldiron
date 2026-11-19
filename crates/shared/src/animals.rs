//! Headless fixtures can construct editable animal characters and sample behavior.
//! Creator authors species through the Animal entity node and behavior separately.
use crate::{character::Character, project::Project};
use theframework_core::graph::*;

#[derive(Clone, Debug)]
pub struct AnimalTemplate {
    pub character: Character,
    pub behavior: serde_json::Value,
}

pub fn template(source: &str, id: &str) -> Result<AnimalTemplate, String> {
    use crate::graph_authoring::{connect, node, number};
    let definitions = crate::rulesets::animals::from_source(source)?;
    let animal = definitions
        .get(id)
        .ok_or_else(|| format!("Unknown animal '{id}'"))?;
    let mut character = Character::default();
    character.name = animal.name.clone();
    character.map.name = character.name.clone();
    character.data = format!(
        "[attributes]\nanimal = {id:?}\nrace = {:?}\nclass = {:?}\nname = {:?}\nplayer = false\n",
        animal.race, animal.class, animal.name
    );
    character.authoring = format!("description = {:?}\n", animal.description);
    let mut doc = GraphDocument::default();
    let mut routine = node("routine", "Roam", [82, 123, 73, 255], vec![], true, "flow");
    routine.branch = Some(routine.id);
    let mut walk = node(
        "random_walk",
        "Wander and rest",
        [82, 123, 73, 255],
        vec![
            (
                "area",
                "Area (optional)",
                GraphControlValue::Text(String::new()),
            ),
            (
                "distance",
                "Distance",
                number(animal.roam_distance, 0.1, 10., 0.1),
            ),
            ("speed", "Speed", number(animal.roam_speed, 0.1, 10., 0.1)),
            (
                "pause",
                "Pause (game minutes)",
                number(animal.pause_minutes as f32, 0., 60., 1.),
            ),
        ],
        false,
        "flow",
    );
    // Random Walk exposes its actual completion/failure ports in the editor.
    walk.ports.retain(|p| p.direction == PortDirection::Input);
    for (key, side) in [
        ("started", PortSide::Right),
        ("outside_area", PortSide::Bottom),
    ] {
        let mut port = GraphPort::new(key, PortDirection::Output, side, 0.5);
        port.key = Some(key.into());
        port.kind = "flow".into();
        walk.ports.push(port);
    }
    walk.position = [300., 0.];
    walk.branch = Some(routine.id);
    connect(&mut doc, &routine, &walk);
    doc.nodes.extend([routine, walk]);
    let behavior = serde_json::json!({"version":1,"nodes":[],"connections":[],"branches":[doc]});
    rusterix::server::nodes::Registry::shared().compile(&behavior)?;
    Ok(AnimalTemplate {
        character,
        behavior,
    })
}

impl AnimalTemplate {
    pub fn install(&self, project: &mut Project) {
        project.node_graphs.insert(
            format!("behavior/character/{}", self.character.id),
            self.behavior.clone(),
        );
        project.add_character(self.character.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animals_are_editable_characters_and_survive_project_storage() {
        let mut project = Project::new();
        let source = project.rules_source().unwrap();
        for id in crate::rulesets::animals::from_source(&source)
            .unwrap()
            .keys()
        {
            template(&source, id).unwrap().install(&mut project);
        }
        let saved = crate::project_io::encode_project(&project).unwrap();
        let mut restored = crate::project_io::decode_project(&saved).unwrap();
        crate::entity_graph::synchronize(&mut restored).unwrap();
        for (id, c) in &project.characters {
            let configuration: GraphDocument = serde_json::from_value(
                restored.node_graphs[&crate::entity_graph::character_key(*id)].clone(),
            )
            .unwrap();
            assert!(
                configuration
                    .nodes
                    .iter()
                    .any(|node| { node.definition.as_deref() == Some("entity_animal") })
            );
            assert_eq!(
                restored.characters[id].data.parse::<toml::Table>().unwrap(),
                c.data.parse::<toml::Table>().unwrap()
            );
            let key = format!("behavior/character/{id}");
            assert_eq!(restored.node_graphs[&key], project.node_graphs[&key]);
            rusterix::server::nodes::Registry::shared()
                .compile(&restored.node_graphs[&key])
                .unwrap();
        }
    }
}
