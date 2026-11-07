//! Resolve shared geometry dependencies without changing the saved authoring graph.
use crate::project::Project;
use denrim_materials::geometry::{Document, Operation};
use theframework::prelude::Uuid;

pub fn asset_id(source: &str) -> Result<Uuid, String> {
    if let Some(name) = source.strip_prefix("ruleset:geometry/") {
        return Ok(if name == "humanoid" {
            Uuid::parse_str(crate::rulesets::HUMANOID_APPEARANCE_ASSET_ID).unwrap()
        } else {
            crate::rulesets::geometry_asset_id(name)
        });
    }
    Uuid::parse_str(source).map_err(|_| "Invalid geometry asset reference".into())
}

pub fn resolve(project: &Project, document: &Document) -> Result<Document, String> {
    let doc = resolve_dependencies(project, document)?;
    doc.evaluate().map_err(|e| e.to_string())?;
    Ok(doc)
}

/// Link shared snapshots without evaluating every intermediate body/garment.
/// Runtime consumers validate the linked result once when constructing an appearance.
pub fn resolve_dependencies(project: &Project, document: &Document) -> Result<Document, String> {
    fn visit(
        project: &Project,
        doc: &Document,
        stack: &mut Vec<Uuid>,
        depth: usize,
    ) -> Result<Document, String> {
        if depth > 6 {
            return Err("Geometry dependencies exceed six levels".into());
        }
        let mut doc = doc.clone();
        for tile in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
            match &mut tile.op {
                Operation::Asset { source, document } => {
                    if source == denrim_materials::geometry::WEARER_SOURCE {
                        let id =
                            Uuid::parse_str(crate::rulesets::HUMANOID_APPEARANCE_ASSET_ID).unwrap();
                        let wearer = project
                            .procedural_appearances
                            .get(&id)
                            .map(|text| Document::parse(text).and_then(|doc| doc.wearer_body()))
                            .transpose()
                            .map_err(|e| e.to_string())?;
                        let body = wearer.as_ref().unwrap_or(document);
                        **document = visit(project, body, stack, depth + 1)?;
                        continue;
                    }
                    let id = asset_id(source)?;
                    if stack.contains(&id) {
                        return Err("Cyclic geometry asset reference".into());
                    }
                    let dependency = if let Some(text) = project.procedural_appearances.get(&id) {
                        Document::parse(text).map_err(|e| e.to_string())?
                    } else {
                        document.as_ref().clone()
                    };
                    stack.push(id);
                    **document = visit(project, &dependency, stack, depth + 1)?;
                    stack.pop();
                }
                Operation::Group { document, .. } | Operation::Input { preview: document, .. } => {
                    **document = visit(project, document, stack, depth + 1)?
                }
                _ => {}
            }
        }
        Ok(doc)
    }
    visit(project, document, &mut vec![], 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orc_anatomy_has_independent_proportions_and_editable_mass_controls() {
        let assets = crate::rulesets::bundled_appearance_assets_for_ruleset(
            crate::rulesets::OFFICIAL_RULESET_ID,
            crate::rulesets::OFFICIAL_RULESET_VERSION,
        );
        let body = |id: &str| {
            let asset = assets.iter().find(|a| a.id == id).unwrap();
            let mut doc = Document::load(asset.source).unwrap();
            doc.rows.truncate(1);
            doc.output = "body".into();
            doc.grip = None;
            let mesh = doc.evaluate().unwrap().mesh;
            (doc, mesh)
        };
        let (_, human) = body("humanoid");
        let (doc, orc) = body("orc");
        let human_rig = human.rig.as_ref().unwrap();
        let orc_rig = orc.rig.as_ref().unwrap();
        assert_eq!(
            human_rig.joints.iter().map(|j| &j.id).collect::<Vec<_>>(),
            orc_rig.joints.iter().map(|j| &j.id).collect::<Vec<_>>()
        );
        let span = |mesh: &denrim_materials::geometry::Mesh, part: Option<&str>, axis: usize| {
            let positions = mesh
                .parts
                .iter()
                .filter(|p| part.is_none_or(|id| p.id == id))
                .flat_map(|p| p.vertices.iter().map(|v| v.position[axis]))
                .collect::<Vec<_>>();
            positions.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - positions.iter().copied().fold(f32::INFINITY, f32::min)
        };
        let hh = span(&human, None, 1);
        let oh = span(&orc, None, 1);
        assert!((oh - 1.95).abs() < 0.0001);
        assert!(span(&orc, None, 0) > span(&human, None, 0) * 1.45);
        assert!(span(&orc, Some("body-torso"), 0) > span(&human, Some("body-torso"), 0) * 1.3);
        assert!(span(&orc, Some("body-neck"), 0) > span(&human, Some("body-neck"), 0) * 1.7);
        assert!(span(&orc, Some("left-palm"), 0) > span(&human, Some("left-palm"), 0) * 1.4);
        let hw = human_rig.rest();
        let ow = orc_rig.rest();
        let leg = |r: &denrim_materials::geometry::rig::Rig,
                   w: &Vec<denrim_materials::geometry::rig::Pose>| {
            w[r.index("left-hip").unwrap()].translation[1]
                - w[r.index("left-foot").unwrap()].translation[1]
        };
        assert!(leg(orc_rig, &ow) / oh < leg(human_rig, &hw) / hh * 0.90);
        let arm = |r: &denrim_materials::geometry::rig::Rig| {
            ["left-elbow", "left-hand"]
                .iter()
                .map(|id| {
                    r.joints[r.index(id).unwrap()]
                        .local
                        .translation
                        .iter()
                        .map(|v| v * v)
                        .sum::<f32>()
                        .sqrt()
                })
                .sum::<f32>()
        };
        assert!(arm(orc_rig) > arm(human_rig) * 1.2);
        assert!(
            ow[orc_rig.index("head").unwrap()].translation[2]
                - ow[orc_rig.index("chest").unwrap()].translation[2]
                > 0.12
        );
        let Operation::Group { controls, .. } = &doc.rows[0].tiles[0].op else {
            panic!()
        };
        for name in [
            "hand-size",
            "forearm-bulk",
            "stoop",
            "height",
            "shoulder-width",
            "arm-length",
            "leg-length",
        ] {
            let at = controls.iter().position(|c| c.id == name).unwrap();
            for value in [controls[at].min, controls[at].max] {
                let mut changed = doc.clone();
                let Operation::Group { controls, .. } = &mut changed.rows[0].tiles[0].op else {
                    panic!()
                };
                controls[at].value = value;
                changed
                    .evaluate()
                    .unwrap_or_else(|e| panic!("{name}={value}: {e}"));
            }
        }
        assert_eq!(doc, Document::load(&doc.save().unwrap()).unwrap());
    }
    #[test]
    fn every_official_item_has_editable_textured_world_geometry() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let templates = crate::rulesets::ruleset_item_templates_from_source(
            crate::rulesets::latest_official_ruleset(),
        )
        .unwrap();
        assert_eq!(templates.len(), 41);
        let materials = crate::ruleset_materials::definitions(&project.config);
        for item in templates {
            let root = item.data.parse::<toml::Table>().unwrap();
            let attrs = root["attributes"].as_table().unwrap();
            for key in [
                "appearance_geometry",
                "appearance_geometry_off",
                "appearance_worn_geometry",
            ] {
                let Some(name) = attrs.get(key).and_then(toml::Value::as_str) else {
                    assert_ne!(key, "appearance_geometry", "{} has no world model", item.id);
                    continue;
                };
                let id = crate::rulesets::geometry_asset_id(name);
                let source = &project.procedural_appearances[&id];
                let doc = Document::load(source).unwrap();
                assert_eq!(doc, Document::load(&doc.save().unwrap()).unwrap());
                let mesh = resolve(&project, &doc)
                    .unwrap_or_else(|e| panic!("{name}: {e}"))
                    .evaluate()
                    .unwrap()
                    .mesh;
                assert!(!mesh.parts.is_empty(), "{name}");
                assert!(
                    mesh.vertex_count() < 12_000,
                    "{name}: {} vertices",
                    mesh.vertex_count()
                );
                for part in &mesh.parts {
                    let texture = part
                        .texture
                        .as_ref()
                        .unwrap_or_else(|| panic!("{name}/{} has no MZ surface", part.id));
                    let material = texture
                        .source
                        .strip_prefix("ruleset:material/")
                        .or_else(|| texture.source.strip_prefix("ruleset:"))
                        .unwrap();
                    assert!(
                        materials.iter().any(|m| m.id == material),
                        "{name}: {material}"
                    );
                }
                if let Operation::Group { controls, .. } = &doc.rows[0].tiles[0].op {
                    assert!(!controls.is_empty(), "{name}");
                    for (index, control) in controls.iter().enumerate() {
                        for value in [control.min, control.max] {
                            let mut changed = doc.clone();
                            let Operation::Group { controls, .. } =
                                &mut changed.rows[0].tiles[0].op
                            else {
                                unreachable!()
                            };
                            controls[index].value = value;
                            changed
                                .evaluate()
                                .unwrap_or_else(|e| panic!("{name}/{}: {e}", control.id));
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn equipment_proportions_curvature_and_dimension_controls_remain_editable() {
        let models = crate::rulesets::bundled_appearance_assets_for_ruleset(
            crate::rulesets::OFFICIAL_RULESET_ID,
            crate::rulesets::OFFICIAL_RULESET_VERSION,
        );
        for name in [
            "training_sword",
            "round_shield",
            "hand_axe",
            "novice_mace",
            "hunting_bow",
            "training_spear",
        ] {
            let source = models.iter().find(|m| m.id == name).unwrap();
            assert_eq!(
                asset_id(&format!("ruleset:geometry/{name}"))
                    .unwrap()
                    .to_string(),
                source.asset_id
            );
            let doc = Document::load(source.source).unwrap();
            assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
            let evaluation = doc.evaluate().unwrap();
            let (lo, hi) = evaluation.mesh.bounds();
            assert!(evaluation.grip.is_some());
            assert!(evaluation.mesh.vertex_count() < 5000, "{name}");
            if matches!(name, "novice_mace" | "hunting_bow" | "training_spear") {
                assert!(
                    evaluation.mesh.parts.iter().all(|p| p
                        .texture
                        .as_ref()
                        .is_some_and(|t| t.source.starts_with("ruleset:material/"))),
                    "{name}"
                );
            }
            if name == "novice_mace" {
                assert_eq!(
                    evaluation
                        .mesh
                        .parts
                        .iter()
                        .filter(|p| p.id.starts_with("mace-fin-"))
                        .count(),
                    6
                );
                assert!((hi[1] - lo[1] - 0.69).abs() < 0.001);
            }
            if name == "hunting_bow" {
                assert!(
                    evaluation
                        .mesh
                        .parts
                        .iter()
                        .any(|p| p.id == "bow-string-upper")
                );
                assert!(
                    evaluation
                        .mesh
                        .parts
                        .iter()
                        .any(|p| p.id == "bow-string-lower")
                );
                assert!(
                    evaluation
                        .mesh
                        .parts
                        .iter()
                        .any(|p| p.id == "bow-arrow-point")
                );
                assert!((hi[1] - lo[1] - 1.12).abs() < 0.001);
            }
            if name == "training_spear" {
                assert!((hi[1] - lo[1] - 1.725).abs() < 0.001);
            }
            if name == "training_sword" {
                assert!((0.8..0.95).contains(&(hi[1] - lo[1])));
            }
            if name == "round_shield" {
                assert!((hi[0] - lo[0] - 0.56).abs() < 0.001);
                assert!((hi[1] - lo[1] - 0.62).abs() < 0.001);
                let shell = &evaluation
                    .mesh
                    .parts
                    .iter()
                    .find(|p| p.id == "shield-shell")
                    .unwrap();
                let rear_center = shell
                    .vertices
                    .iter()
                    .filter(|v| {
                        v.normal[2] < -0.5
                            && v.position[0].abs() < 0.005
                            && v.position[1].abs() < 0.005
                    })
                    .map(|v| v.position[2])
                    .fold(f32::INFINITY, f32::min);
                let rear_edge = shell
                    .vertices
                    .iter()
                    .filter(|v| v.normal[2] < -0.5 && v.position[0].abs() > 0.26)
                    .map(|v| v.position[2])
                    .fold(f32::INFINITY, f32::min);
                assert!(rear_center.is_finite() && rear_edge.is_finite());
                assert!(
                    rear_center > rear_edge + 0.04,
                    "Back must be concave toward the wearer"
                );
                assert!(evaluation.grip.unwrap().translation[2] < lo[2] + 0.02);
            }
            let Operation::Group { controls, .. } = &doc.rows[0].tiles[0].op else {
                panic!()
            };
            for (index, control) in controls.iter().enumerate() {
                for value in [control.min, control.max] {
                    let mut changed = doc.clone();
                    let Operation::Group { controls, .. } = &mut changed.rows[0].tiles[0].op else {
                        panic!()
                    };
                    controls[index].value = value;
                    let mesh = changed
                        .evaluate()
                        .unwrap_or_else(|e| panic!("{name}/{}={value}: {e}", control.id))
                        .mesh;
                    assert!(
                        mesh.parts.iter().flat_map(|p| &p.vertices).all(|v| v
                            .position
                            .into_iter()
                            .chain(v.normal)
                            .all(f32::is_finite))
                    );
                }
            }
        }
    }
    #[test]
    fn shared_equipment_edits_propagate_and_cycles_are_rejected() {
        let mut p = Project::default();
        p.sync_ruleset_items().unwrap();
        let human = Uuid::parse_str(crate::rulesets::HUMANOID_APPEARANCE_ASSET_ID).unwrap();
        let sword = crate::rulesets::geometry_asset_id("training_sword");
        let source = Document::load(&p.procedural_appearances[&human]).unwrap();
        let original = resolve(&p, &source).unwrap().evaluate().unwrap().mesh;
        let mut model = Document::load(&p.procedural_appearances[&sword]).unwrap();
        let Operation::Group { controls, .. } = &mut model.rows[0].tiles[0].op else {
            panic!()
        };
        controls
            .iter_mut()
            .find(|c| c.id == "blade-length")
            .unwrap()
            .value = 1.1;
        p.procedural_appearances
            .insert(sword, model.save().unwrap());
        let changed = resolve(&p, &source).unwrap().evaluate().unwrap().mesh;
        assert_ne!(
            original
                .parts
                .iter()
                .flat_map(|p| &p.vertices)
                .map(|v| v.position)
                .collect::<Vec<_>>(),
            changed
                .parts
                .iter()
                .flat_map(|p| &p.vertices)
                .map(|v| v.position)
                .collect::<Vec<_>>()
        );
        let mut cycle = model.clone();
        cycle.rows = vec![denrim_materials::geometry::Row {
            id: "cycle".into(),
            label: "Cycle".into(),
            tiles: vec![denrim_materials::geometry::Tile {
                id: "dependency".into(),
                op: Operation::Asset {
                    source: sword.to_string(),
                    document: Box::new(model),
                },
            }],
        }];
        cycle.output = "cycle".into();
        cycle.grip = None;
        p.procedural_appearances
            .insert(sword, cycle.save().unwrap());
        assert!(resolve(&p, &source).unwrap_err().contains("Cyclic"));
        let bytes = crate::project_io::encode_project(&p).unwrap();
        let reopened = crate::project_io::decode_project(&bytes).unwrap();
        assert_eq!(p.procedural_appearances, reopened.procedural_appearances);
    }
}
