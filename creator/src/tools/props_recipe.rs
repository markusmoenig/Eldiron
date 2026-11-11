//! Direct recipe-backed 3D placement. Prefabs are optional, never an MZ wrapper.
use crate::prelude::*;
use denrim_materials::geometry::{Document, Operation, group::Control};
use shared::geometry_recipe::{self, Binding};

pub fn candidates(project: &Project) -> Vec<(Uuid, String, String)> {
    fn needs_selection(doc: &Document) -> bool {
        doc.rows.iter().flat_map(|r| &r.tiles).any(|t| match &t.op {
            Operation::Input { .. }
            | Operation::Armature { .. }
            | Operation::Joint { .. }
            | Operation::Skeleton { .. }
            | Operation::ParticleEffect { .. } => true,
            Operation::Asset { document, .. } | Operation::Group { document, .. } => {
                needs_selection(document)
            }
            _ => false,
        })
    }
    let mut entries = project
        .procedural_appearances
        .iter()
        .filter_map(|(id, source)| {
            let doc = Document::parse(source).ok()?;
            if needs_selection(&doc) {
                return None;
            }
            Some((*id, doc.label, source.clone()))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    entries
}

#[cfg(test)]
pub fn catalog(project: &Project) -> Vec<(Uuid, String, String)> {
    candidates(project)
        .into_iter()
        .filter(|(_, _, source)| {
            denrim_materials::geometry::Document::parse(source)
                .ok()
                .and_then(|d| shared::appearance_assets::resolve_dependencies(project, &d).ok())
                .and_then(|d| d.evaluate().ok())
                .is_some_and(|e| {
                    !e.mesh.parts.is_empty() && e.mesh.rig.is_none() && e.mesh.effects.is_empty()
                })
        })
        .collect()
}
/// Geometry/material sources only. Do not clone regions or the large iso sprite/shadow caches.
pub fn preview_snapshot(project: &Project) -> Project {
    let mut host = Project::default();
    host.regions.clear();
    host.procedural_appearances = project.procedural_appearances.clone();
    host.procedural_materials = project.procedural_materials.clone();
    host.palette = project.palette.clone();
    host.config = project.config.clone();
    host.rules = project.rules.clone();
    host.rules_document = project.rules_document.clone();
    // Only direct bitmap inputs are needed. Copying every painted world page
    // here made opening a browser depend on the size of the entire map.
    fn bitmap_inputs(doc: &Document, project: &Project, ids: &mut std::collections::HashSet<Uuid>) {
        for tile in doc.rows.iter().flat_map(|r| &r.tiles) {
            match &tile.op {
                Operation::Texture { source, .. } => {
                    if let Ok(id) = Uuid::parse_str(source)
                        && !project.procedural_materials.contains_key(&id)
                    {
                        ids.insert(id);
                    }
                }
                Operation::Group { document, .. }
                | Operation::Asset { document, .. }
                | Operation::Input {
                    preview: document, ..
                } => bitmap_inputs(document, project, ids),
                _ => (),
            }
        }
    }
    let mut inputs = std::collections::HashSet::new();
    for source in project.procedural_appearances.values() {
        if let Ok(doc) = Document::parse(source) {
            bitmap_inputs(&doc, project, &mut inputs);
        }
    }
    host.tiles = inputs
        .into_iter()
        .filter_map(|id| project.tiles.get(&id).map(|t| (id, t.clone())))
        .collect();
    host
}

pub fn template(
    project: &mut Project,
    asset: Uuid,
    previous: Option<&rusterix::GeometryObject>,
) -> Result<rusterix::GeometryObject, String> {
    let source = project
        .procedural_appearances
        .get(&asset)
        .ok_or("Recipe was removed")?
        .clone();
    let binding = Binding {
        asset,
        source,
        objects: vec![],
        faces: vec![],
        output: previous.map(|o| o.id),
    };
    geometry_recipe::generate(project, &Map::default(), &binding, previous)
}

pub fn prepare(project: &mut Project, server: &mut ServerContext) -> Result<(), String> {
    let Some(asset) = server
        .curr_block_asset_id
        .filter(|id| project.procedural_appearances.contains_key(id))
    else {
        server.block_recipe_template = None;
        server.block_recipe_pending = false;
        return Ok(());
    };
    if server.block_recipe_pending {
        return Ok(());
    }
    let previous = server
        .block_recipe_template
        .as_ref()
        .filter(|o| Binding::read(o).is_some_and(|b| b.asset == asset));
    let next = template(project, asset, previous)?;
    server.curr_block_asset_name = Some(next.name.clone());
    server.block_recipe_template = Some(next);
    Ok(())
}

/// Same transform for the ghost and committed objects. Recipe origin/pivot is retained.
pub fn placed(
    template: &rusterix::GeometryObject,
    cell: Vec3<i32>,
    base_y: f32,
    step: f32,
    rotation: i32,
) -> rusterix::GeometryObject {
    let mut object = template.clone();
    let old_id = object.id;
    let new_id = Uuid::new_v4();
    let face_ids = geometry_recipe::face_ids_for_owner(&object, new_id);
    geometry_recipe::remap_ids(
        &mut object,
        &|id| if id == old_id { new_id } else { id },
        &|id| face_ids.get(&id).copied().unwrap_or(id),
    );
    object.id = new_id;
    for face in &mut object.faces {
        face.id = face_ids.get(&face.id).copied().unwrap_or_else(Uuid::new_v4);
        face.paint_surface_id = None;
    }
    object.transform = placement_transform(cell, base_y, step, rotation);
    object.group = format!("block:{new_id}");
    object
        .properties
        .set("block_instance_id", Value::Id(new_id));
    object.properties.set("block_cell_size", Value::Float(step));
    for (key, value) in [
        ("block_grid_x", cell.x),
        ("block_grid_y", cell.y),
        ("block_grid_z", cell.z),
    ] {
        object.properties.set(key, Value::Int(value));
    }
    object.ensure_face_paint_data();
    object
}

pub fn placement_transform(
    cell: Vec3<i32>,
    base_y: f32,
    step: f32,
    rotation: i32,
) -> [[f32; 4]; 4] {
    let angle = rotation.rem_euclid(4) as f32 * std::f32::consts::FRAC_PI_2;
    let (s, c) = angle.sin_cos();
    [
        [c, 0., -s, 0.],
        [0., 1., 0., 0.],
        [s, 0., c, 0.],
        [cell.x as f32 * step, base_y, cell.z as f32 * step, 1.],
    ]
}

#[derive(Clone)]
pub struct Published {
    pub path: Vec<String>,
    pub operation: String,
    pub control: Control,
}
pub fn published(doc: &Document) -> Vec<Published> {
    fn visit(doc: &Document, path: &mut Vec<String>, out: &mut Vec<Published>) {
        for tile in doc.rows.iter().flat_map(|r| &r.tiles) {
            if let Operation::Group {
                document, controls, ..
            } = &tile.op
            {
                let resolved = denrim_materials::geometry::expressions::resolve(controls)
                    .unwrap_or_else(|_| controls.clone().into());
                let mut controls = resolved.iter().cloned().collect::<Vec<_>>();
                controls.sort_by(|a, b| a.category.cmp(&b.category).then(a.order.cmp(&b.order)));
                out.extend(controls.into_iter().map(|control| Published {
                    path: path.clone(),
                    operation: tile.id.clone(),
                    control,
                }));
                path.push(tile.id.clone());
                visit(document, path, out);
                path.pop();
            }
        }
    }
    let mut out = vec![];
    visit(doc, &mut vec![], &mut out);
    out
}
pub fn set_control(
    project: &mut Project,
    asset: Uuid,
    index: usize,
    value: f32,
) -> Result<(), String> {
    if !value.is_finite() {
        return Err("Control must be finite".into());
    }
    let mut doc = Document::parse(
        project
            .procedural_appearances
            .get(&asset)
            .ok_or("Recipe was removed")?,
    )
    .map_err(|e| e.to_string())?;
    let field = published(&doc)
        .get(index)
        .cloned()
        .ok_or("Control was removed")?;
    if field.control.expression.is_some() {
        return Err("Calculated controls cannot be edited".into());
    }
    let tile = doc
        .group_document_mut(&field.path)
        .map_err(|e| e.to_string())?
        .rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find(|t| t.id == field.operation)
        .ok_or("Group was removed")?;
    if let Operation::Group { controls, .. } = &mut tile.op {
        let control = controls
            .iter_mut()
            .find(|c| c.id == field.control.id)
            .ok_or("Control was removed")?;
        control.value = value.clamp(control.min, control.max);
        if control.is_palette() || control.is_integer() {
            control.value = control.value.round();
        }
    }
    shared::appearance_assets::resolve_dependencies(project, &doc)?
        .evaluate()
        .map_err(|e| e.to_string())?;
    project
        .procedural_appearances
        .insert(asset, doc.save().map_err(|e| e.to_string())?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_snapshot_copies_only_explicit_bitmap_inputs() {
        let mut project = Project::default();
        let bitmap = rusterix::Tile::from_texture(rusterix::Texture::from_color([20, 40, 60, 255]));
        let id = bitmap.id;
        project.tiles.insert(id, bitmap.clone());
        let unrelated = rusterix::Tile::from_texture(rusterix::Texture::alloc(512, 512));
        project.tiles.insert(unrelated.id, unrelated);
        assert!(preview_snapshot(&project).tiles.is_empty());
        let mut doc = Document::parse(include_str!(
            "../../embedded/meshes/recipes/fence.mzgeometry.json"
        ))
        .unwrap();
        doc.rows[0].tiles[0].op = Operation::Texture {
            source: id.to_string(),
            bundle: None,
            size: 64,
            axes: [0, 2],
            origin: [0.; 2],
            span: [1.; 2],
            parts: vec![],
            preserve_uv: false,
        };
        project
            .procedural_appearances
            .insert(Uuid::new_v4(), serde_json::to_string(&doc).unwrap());
        let snapshot = preview_snapshot(&project);
        assert_eq!(snapshot.tiles.len(), 1);
        assert_eq!(snapshot.tiles[&id], bitmap);
        assert!(snapshot.regions.is_empty());
    }

    #[test]
    fn direct_mesh_placements_rebuild_from_controls_and_keep_independent_transforms_and_paint() {
        let mut project = Project::default();
        let id = Uuid::new_v4();
        project.procedural_appearances.insert(
            id,
            include_str!("../../embedded/meshes/recipes/fence.mzgeometry.json").into(),
        );
        let entries = catalog(&project);
        assert_eq!(entries.len(), 1);
        let template = template(&mut project, id, None).unwrap();
        let first = placed(&template, Vec3::new(3, 2, -4), 2., 0.5, 1);
        let second = placed(&template, Vec3::new(-2, 0, 1), 0., 0.5, 0);
        assert_ne!(first.id, second.id);
        assert!(
            first
                .faces
                .iter()
                .all(|f| second.faces.iter().all(|g| f.id != g.id
                    && rusterix::geometry_face_effective_paint_surface_id(f)
                        != rusterix::geometry_face_effective_paint_surface_id(g)))
        );
        assert!(Binding::read(&first).unwrap().objects.is_empty());
        let transform = first.transform;
        let face = first.faces[0].id;
        project.regions[0].map.geometry_objects = vec![first, second];
        project.regions[0].map.geometry_objects[0].faces[0].tile =
            Some(rusterix::PixelSource::Pixel([255, 0, 255, 255]));
        let before = project.regions[0].map.geometry_objects[0].vertices.len();
        let controls = published(&Document::parse(&project.procedural_appearances[&id]).unwrap());
        let count = controls
            .iter()
            .position(|p| p.control.id == "count")
            .unwrap();
        let length = controls
            .iter()
            .position(|p| p.control.id == "length")
            .unwrap();
        assert!(set_control(&mut project, id, length, 90.).is_err());
        set_control(&mut project, id, count, 8.).unwrap();
        let (changed, errors) = geometry_recipe::synchronize(&mut project);
        assert!(changed);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(
            project.block_props.is_empty(),
            "direct MZ placement must not create prefab wrappers"
        );
        let object = &project.regions[0].map.geometry_objects[0];
        assert_eq!(object.transform, transform);
        assert!(object.vertices.len() > before);
        assert_eq!(object.faces[0].id, face);
        assert_eq!(
            object.faces[0].tile,
            Some(rusterix::PixelSource::Pixel([255, 0, 255, 255]))
        );
        let controls = published(&Document::parse(&project.procedural_appearances[&id]).unwrap());
        let spacing = controls
            .iter()
            .find(|p| p.control.id == "spacing")
            .unwrap()
            .control
            .value;
        assert!((controls[length].control.value - 8. * spacing).abs() < 0.0001);
        let encoded = shared::project_io::encode_project(&project).unwrap();
        let mut reopened = shared::project_io::decode_project(&encoded).unwrap();
        let (changed, errors) = geometry_recipe::synchronize(&mut reopened);
        assert!(!changed);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(reopened.regions[0].map.geometry_objects[0], *object);
        // A bad source edit retains the last good placed cache.
        reopened.procedural_appearances.insert(id, "invalid".into());
        let (_, errors) = geometry_recipe::synchronize(&mut reopened);
        assert_eq!(errors.len(), 2);
        assert_eq!(reopened.regions[0].map.geometry_objects[0], *object);
    }
}
