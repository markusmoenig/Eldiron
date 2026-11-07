//! Selection-driven MZ geometry shared by map construction and Prefab parts.
//! Recipes stay editable; GeometryObjects are stable, derived runtime caches.
use crate::project::Project;
use denrim_materials::geometry::{Document, Mesh, Part, Vertex};
use rusterix::{BlockPropGeometrySource, GeometryObject, Map, PixelSource, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use theframework::prelude::{Uuid, Vec2, Vec3};

pub const BINDING: &str = "mz_geometry_recipe";
const REVISION: &str = "mz_geometry_recipe_revision";
const DEFAULTS: &str = "mz_geometry_recipe_face_sources";
const FACE_KEYS: &str = "mz_geometry_recipe_face_keys";

pub fn face_ids_for_owner(object: &GeometryObject, owner: Uuid) -> BTreeMap<Uuid, Uuid> {
    let keys: BTreeMap<Uuid, String> = object
        .properties
        .get_str(FACE_KEYS)
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    keys.into_iter()
        .map(|(id, key)| (id, identity(owner, &key)))
        .collect()
}

pub fn preview_input(project: &Project, asset: Uuid) -> Option<(Mesh, String)> {
    for map in project
        .prefab_editor_map
        .iter()
        .chain(project.regions.iter().map(|r| &r.map))
    {
        let ids = map
            .selected_geometry_objects
            .iter()
            .copied()
            .chain(map.selected_geometry_faces.iter().map(|(id, _)| *id));
        for id in ids {
            let binding = map
                .geometry_objects
                .iter()
                .find(|o| o.id == id)
                .and_then(Binding::read);
            if let Some(binding) = binding
                && binding.asset == asset
            {
                let selection = binding.selection(map).ok()?;
                let key = serde_json::to_string(&selection).ok()?;
                return Some((input_mesh(&selection).ok()?.0, key));
            }
        }
    }
    None
}

/// Make Prefab Unique remaps references as well as cached object/face identities.
pub fn remap_ids(
    object: &mut GeometryObject,
    object_id: &dyn Fn(Uuid) -> Uuid,
    face_id: &dyn Fn(Uuid) -> Uuid,
) {
    if let Some(mut binding) = Binding::read(object) {
        binding.output = binding.output.map(object_id);
        for id in &mut binding.objects {
            *id = object_id(*id);
        }
        for (owner, id) in &mut binding.faces {
            *owner = object_id(*owner);
            *id = face_id(*id);
        }
        object.properties.set(
            BINDING,
            Value::Str(serde_json::to_string(&binding).unwrap()),
        );
        object.properties.remove(REVISION);
        for key in [DEFAULTS, FACE_KEYS] {
            if let Some(text) = object.properties.get_str(key)
                && let Ok(values) = serde_json::from_str::<BTreeMap<Uuid, serde_json::Value>>(text)
            {
                let remapped = values
                    .into_iter()
                    .map(|(id, value)| (face_id(id), value))
                    .collect::<BTreeMap<_, _>>();
                object
                    .properties
                    .set(key, Value::Str(serde_json::to_string(&remapped).unwrap()));
            }
        }
    }
}

pub fn choices(project: &Project) -> Vec<(Uuid, String, String)> {
    let id = identity(Uuid::nil(), "mz-recipe:pitched-roof");
    let mut choices = vec![(
        id,
        "Pitched roof".into(),
        project
            .procedural_appearances
            .get(&id)
            .cloned()
            .unwrap_or_else(|| denrim_materials::geometry::PITCHED_ROOF_RECIPE.into()),
    )];
    fn has_input(doc: &Document) -> bool {
        doc.rows.iter().flat_map(|r| &r.tiles).any(|t| match &t.op {
            denrim_materials::geometry::Operation::Input { name, .. } => name == "selection",
            denrim_materials::geometry::Operation::Group { document, .. }
            | denrim_materials::geometry::Operation::Asset { document, .. } => has_input(document),
            _ => false,
        })
    }
    for (asset, source) in &project.procedural_appearances {
        if *asset == id {
            continue;
        }
        if let Ok(doc) = Document::parse(source)
            && has_input(&doc)
        {
            choices.push((*asset, doc.label, source.clone()));
        }
    }
    choices
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    pub asset: Uuid,
    pub source: String,
    pub objects: Vec<Uuid>,
    pub faces: Vec<(Uuid, Uuid)>,
    #[serde(default)]
    pub output: Option<Uuid>,
}

/// Stable semantic IDs survive parameter changes and cache serialization.
fn identity(owner: Uuid, label: &str) -> Uuid {
    let mut a = 0xcbf29ce484222325u64;
    let mut b = 0x84222325cbf29ce4u64;
    for byte in owner.as_bytes().iter().copied().chain(label.bytes()) {
        a = (a ^ byte as u64).wrapping_mul(0x100000001b3);
        b = (b ^ byte as u64).wrapping_mul(0x100000001b3).rotate_left(7);
    }
    Uuid::from_u128(((a as u128) << 64) | b as u128)
}

impl Binding {
    pub fn from_selection(project: &Project, map: &Map, asset: Uuid) -> Result<Self, String> {
        let source = project
            .procedural_appearances
            .get(&asset)
            .ok_or("Recipe was removed")?
            .clone();
        let faces = map
            .selected_geometry_faces
            .iter()
            .map(|(object, index)| {
                let face = map
                    .geometry_objects
                    .iter()
                    .find(|o| o.id == *object)
                    .and_then(|o| o.faces.get(*index))
                    .ok_or("Selected face was removed")?;
                Ok((*object, face.id))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let objects = if faces.is_empty() {
            map.selected_geometry_objects.clone()
        } else {
            vec![]
        };
        if faces.is_empty() && objects.is_empty() {
            return Err("Select geometry or faces to supply the recipe input".into());
        }
        Ok(Self {
            asset,
            source,
            objects,
            faces,
            output: None,
        })
    }

    pub fn read(object: &GeometryObject) -> Option<Self> {
        serde_json::from_str(object.properties.get_str(BINDING)?).ok()
    }

    fn selection(&self, map: &Map) -> Result<Vec<GeometryObject>, String> {
        let mut selection = Vec::new();
        for id in &self.objects {
            let object =
                map.geometry_objects.iter().find(|o| o.id == *id).ok_or(
                    "Recipe input object was removed; retaining the last generated geometry",
                )?;
            if object.properties.get_str(BINDING).is_some() {
                return Err("Use authored geometry as the recipe input; recipe-output chains are not supported yet".into());
            }
            selection.push(object.clone());
        }
        let mut selected_faces = BTreeMap::<Uuid, Vec<Uuid>>::new();
        for (object, face) in &self.faces {
            selected_faces.entry(*object).or_default().push(*face);
        }
        for (id, faces) in selected_faces {
            let mut object = map
                .geometry_objects
                .iter()
                .find(|o| o.id == id)
                .ok_or("Recipe input object was removed")?
                .clone();
            if object.properties.get_str(BINDING).is_some() {
                return Err("Use authored geometry as the recipe input; recipe-output chains are not supported yet".into());
            }
            if faces
                .iter()
                .any(|id| !object.faces.iter().any(|f| f.id == *id))
            {
                return Err(
                    "Recipe input face was removed; retaining the last generated geometry".into(),
                );
            }
            object.faces.retain(|f| faces.contains(&f.id));
            selection.push(object);
        }
        if selection.is_empty() {
            return Err("Recipe selection is empty".into());
        }
        Ok(selection)
    }
}

/// The input uses a horizontal frame derived from its first horizontal edge, so
/// a rectangular frame rotated in the map receives a correspondingly rotated roof.
fn input_mesh(objects: &[GeometryObject]) -> Result<(Mesh, Vec3<f32>, Vec3<f32>), String> {
    let mut mesh = Mesh::default();
    let mut axis = None;
    for object in objects {
        let mut vertices = Vec::new();
        for face in &object.faces {
            let points = face
                .indices
                .iter()
                .map(|i| object.vertices.get(*i).map(|v| object.transform_point(*v)))
                .collect::<Option<Vec<_>>>()
                .ok_or("Invalid input face")?;
            if axis.is_none() {
                axis = points
                    .iter()
                    .zip(points.iter().cycle().skip(1))
                    .take(points.len())
                    .map(|(a, b)| *b - *a)
                    .find(|d| d.y.abs() < 0.001 && d.x * d.x + d.z * d.z > 0.0001)
                    .map(|d| Vec3::new(d.x, 0., d.z).normalized());
            }
            for (a, b, c) in
                rusterix::triangulate_geometry_polygon(&points).ok_or("Invalid input polygon")?
            {
                let n = (points[b] - points[a])
                    .cross(points[c] - points[a])
                    .normalized();
                for i in [a, b, c] {
                    vertices.push(Vertex {
                        position: points[i].into_array(),
                        normal: n.into_array(),
                        uv: [0.; 2],
                        skin: None,
                    });
                }
            }
        }
        mesh.parts.push(Part {
            id: object.id.to_string(),
            label: object.name.clone(),
            vertices,
            surface: None,
            texture: None,
        });
    }
    if mesh.vertex_count() == 0 {
        return Err("Selected geometry has no faces".into());
    }
    let x = axis.unwrap_or(Vec3::unit_x());
    let z = x.cross(Vec3::unit_y());
    for vertex in mesh.parts.iter_mut().flat_map(|p| &mut p.vertices) {
        let p = Vec3::from(vertex.position);
        let n = Vec3::from(vertex.normal);
        vertex.position = [p.dot(x), p.y, p.dot(z)];
        vertex.normal = [n.dot(x), n.y, n.dot(z)];
    }
    Ok((mesh, x, z))
}

fn texture(
    project: &mut Project,
    spec: &denrim_materials::geometry::Texture,
) -> Result<Uuid, String> {
    use denrim_materials::integration::EditableMaterialAsset;
    let mut asset = if let Some(bundle) = &spec.bundle {
        EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string())?
    } else if let Some(name) = spec.source.strip_prefix("ruleset:material/") {
        crate::ruleset_materials::editable(project, name)?
    } else {
        let id = Uuid::parse_str(&spec.source).map_err(
            |_| "Map recipes require a project/ruleset material or an embedded material bundle",
        )?;
        if let Some(bundle) = project.procedural_materials.get(&id) {
            EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string())?
        } else if project.tiles.contains_key(&id) {
            return Ok(id);
        } else {
            return Err("Recipe texture was removed".into());
        }
    };
    rusterix::ruleset_material::resolve_palette(&mut asset, &project.palette)?;
    let bundle = asset.save().map_err(|e| e.to_string())?;
    let id = identity(
        Uuid::nil(),
        &format!("mz-recipe-texture:{}:{}", spec.size, bundle),
    );
    if !project.tiles.contains_key(&id) {
        let baked = rusterix::ruleset_material::bake(&asset, &project.palette, spec.size)?;
        let mut tile = rusterix::Tile::from_texture(baked);
        tile.id = id;
        tile.baked_material_data = tile
            .textures
            .iter()
            .map(rusterix::Texture::material_bytes)
            .collect();
        project.tiles.insert(id, tile);
    }
    Ok(id)
}

fn surface(
    project: &mut Project,
    surface: &denrim_materials::geometry::Surface,
) -> Result<PixelSource, String> {
    let color = project
        .palette
        .colors
        .get(surface.palette as usize)
        .and_then(Option::as_ref)
        .ok_or_else(|| format!("Recipe material palette slot {} is empty", surface.palette))?
        .to_u8_array();
    let id = identity(
        Uuid::nil(),
        &format!(
            "mz-recipe-surface:{color:?}:{}:{}",
            surface.roughness.to_bits(),
            surface.metallic.to_bits()
        ),
    );
    project.tiles.entry(id).or_insert_with(|| {
        let mut texture = rusterix::Texture::from_color(color);
        texture.set_materials_all(surface.roughness, surface.metallic, 1., 0.);
        let mut tile = rusterix::Tile::from_texture(texture);
        tile.id = id;
        tile.alias = format!("MZ recipe surface {}", surface.palette);
        tile.baked_material_data = tile
            .textures
            .iter()
            .map(rusterix::Texture::material_bytes)
            .collect();
        tile
    });
    Ok(PixelSource::TileId(id))
}

pub fn generate(
    project: &mut Project,
    map: &Map,
    binding: &Binding,
    previous: Option<&GeometryObject>,
) -> Result<GeometryObject, String> {
    let selection = binding.selection(map)?;
    let source = project
        .procedural_appearances
        .get(&binding.asset)
        .unwrap_or(&binding.source)
        .clone();
    let doc = crate::appearance_assets::resolve_dependencies(
        project,
        &Document::parse(&source).map_err(|e| e.to_string())?,
    )?;
    fn materials<'a>(doc: &Document, project: &'a Project, out: &mut Vec<&'a str>) {
        use denrim_materials::geometry::Operation;
        for tile in doc.rows.iter().flat_map(|r| &r.tiles) {
            match &tile.op {
                Operation::Texture { source, .. } => {
                    let id = Uuid::parse_str(source).ok().or_else(|| {
                        source
                            .strip_prefix("ruleset:material/")
                            .map(crate::rulesets::material_asset_id)
                    });
                    if let Some(bundle) = id.and_then(|id| project.procedural_materials.get(&id)) {
                        out.push(bundle);
                    }
                }
                Operation::Group { document, .. }
                | Operation::Asset { document, .. }
                | Operation::Input {
                    preview: document, ..
                } => materials(document, project, out),
                _ => (),
            }
        }
    }
    let mut material_sources = vec![];
    materials(&doc, project, &mut material_sources);
    // Only referenced materials enter the key, so unrelated assets do not cause rebakes.
    let revision = identity(
        binding.asset,
        &serde_json::to_string(&(
            &doc,
            &selection,
            &project.palette,
            &material_sources,
            &project.config,
        ))
        .map_err(|e| e.to_string())?,
    )
    .to_string();
    if let Some(previous) = previous
        && previous.properties.get_str(REVISION) == Some(revision.as_str())
    {
        return Ok(previous.clone());
    }
    let (input, x, z) = input_mesh(&selection)?;
    let mesh = doc
        .evaluate_with_inputs(&BTreeMap::from([("selection".into(), input)]))
        .map_err(|e| e.to_string())?
        .mesh;
    if mesh.rig.is_some() || !mesh.effects.is_empty() {
        return Err("Map geometry recipes must be static; use character/item appearance for rigs and effects".into());
    }
    let mut output = previous
        .cloned()
        .unwrap_or_else(|| GeometryObject::new(&doc.label));
    if previous.is_none()
        && let Some(id) = binding.output
    {
        output.id = id;
    }
    // The recipe output is fitted in the input's current world/source coordinates.
    // Do not apply a previous drag transform a second time when both frame and
    // generated component were moved together.
    output.transform = GeometryObject::new("").transform;
    output.vertices.clear();
    output.faces.clear();
    let mut defaults = BTreeMap::new();
    let old_keys: BTreeMap<Uuid, String> = previous
        .and_then(|o| o.properties.get_str(FACE_KEYS))
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let keys_to_ids = old_keys
        .into_iter()
        .map(|(id, key)| (key, id))
        .collect::<HashMap<_, _>>();
    let mut face_keys = BTreeMap::new();
    let old_defaults: BTreeMap<String, Option<PixelSource>> = previous
        .and_then(|o| o.properties.get_str(DEFAULTS))
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let old_faces = previous
        .map(|o| o.faces.iter().map(|f| (f.id, f)).collect::<HashMap<_, _>>())
        .unwrap_or_default();
    let face_template = GeometryObject::box_("", Vec3::zero(), Vec3::one())
        .faces
        .remove(0);
    let mut occurrences = BTreeMap::<String, usize>::new();
    for part in mesh.parts {
        let occurrence = *occurrences
            .entry(part.id.clone())
            .and_modify(|n| *n += 1)
            .or_insert(0);
        let surface = match &part.texture {
            Some(spec) => Some(PixelSource::TileId(texture(project, spec)?)),
            None => part
                .surface
                .as_ref()
                .map(|s| surface(project, s))
                .transpose()?,
        };
        for (index, triangle) in part.vertices.chunks_exact(3).enumerate() {
            let start = output.vertices.len();
            output.vertices.extend(
                triangle.iter().map(|v| {
                    x * v.position[0] + Vec3::unit_y() * v.position[1] + z * v.position[2]
                }),
            );
            let mut face = face_template.clone();
            let key = format!("{}:{occurrence}:{index}", part.id);
            face.id = keys_to_ids
                .get(&key)
                .copied()
                .unwrap_or_else(|| identity(output.id, &key));
            face_keys.insert(face.id, key);
            face.indices = vec![start + 2, start + 1, start];
            face.tile = surface.clone();
            if part.texture.is_some() {
                face.auto_uv = false;
                face.uvs = triangle.iter().rev().map(|v| Vec2::from(v.uv)).collect();
            }
            defaults.insert(face.id.to_string(), surface.clone());
            if let Some(old) = old_faces.get(&face.id) {
                face.paint_surface_id = old.paint_surface_id;
                face.paint_uvs = old.paint_uvs.clone();
                face.texture_offset = old.texture_offset;
                face.texture_scale = old.texture_scale;
                face.texture_rotation = old.texture_rotation;
                if old_defaults
                    .get(&face.id.to_string())
                    .is_some_and(|default| &old.tile != default)
                {
                    face.tile = old.tile.clone();
                }
                face.tiles = old.tiles.clone();
            }
            output.faces.push(face);
        }
    }
    output.ensure_face_paint_data();
    let mut saved = binding.clone();
    saved.source = source;
    saved.output = Some(output.id);
    output.properties.set(
        BINDING,
        Value::Str(serde_json::to_string(&saved).map_err(|e| e.to_string())?),
    );
    output.properties.set(REVISION, Value::Str(revision));
    output.properties.set(
        DEFAULTS,
        Value::Str(serde_json::to_string(&defaults).map_err(|e| e.to_string())?),
    );
    output.properties.set(
        FACE_KEYS,
        Value::Str(serde_json::to_string(&face_keys).map_err(|e| e.to_string())?),
    );
    Ok(output)
}

/// Transactional per output: an invalid edit retains the last good mesh.
pub fn synchronize_map(project: &mut Project, map: &mut Map) -> (bool, Vec<String>) {
    let mut changed = false;
    let mut errors = Vec::new();
    for index in 0..map.geometry_objects.len() {
        let object = &map.geometry_objects[index];
        let Some(binding) = Binding::read(object) else {
            continue;
        };
        match generate(project, map, &binding, Some(object)) {
            Ok(next) if next != *object => {
                map.geometry_objects[index] = next;
                changed = true;
            }
            Ok(_) => (),
            Err(error) => errors.push(format!("{}: {error}", object.name)),
        }
    }
    (changed, errors)
}

pub fn synchronize(project: &mut Project) -> (bool, Vec<String>) {
    let mut changed = false;
    let mut errors = Vec::new();
    // Take only maps/assets, never the recipe library, out of the host while resolving.
    let mut regions = std::mem::take(&mut project.regions);
    for region in &mut regions {
        let (c, e) = synchronize_map(project, &mut region.map);
        changed |= c;
        errors.extend(e);
    }
    project.regions = regions;
    let mut assets = std::mem::take(&mut project.block_props);
    for asset in assets.values_mut() {
        // The typed part source owns the binding; cache metadata is derived too.
        for part in &mut asset.parts {
            if let BlockPropGeometrySource::MZ {
                binding,
                generated_cache,
            } = &mut part.geometry_source
            {
                for object in generated_cache {
                    if object.properties.get_str(BINDING) != Some(binding.as_str()) {
                        object.properties.set(BINDING, Value::Str(binding.clone()));
                        object.properties.remove(REVISION);
                    }
                }
            }
        }
        let mut map = Map::default();
        map.geometry_objects = asset
            .parts
            .iter()
            .flat_map(|p| p.geometry_source.geometry_objects().iter().cloned())
            .collect();
        for part in &mut asset.parts {
            if let BlockPropGeometrySource::MZ {
                binding,
                generated_cache,
            } = &mut part.geometry_source
                && generated_cache.is_empty()
            {
                match serde_json::from_str::<Binding>(binding)
                    .map_err(|e| e.to_string())
                    .and_then(|binding| generate(project, &map, &binding, None))
                {
                    Ok(object) => {
                        map.geometry_objects.push(object.clone());
                        generated_cache.push(object);
                        changed = true;
                    }
                    Err(error) => errors.push(format!("{} / {}: {error}", asset.name, part.name)),
                }
            }
        }
        let (c, e) = synchronize_map(project, &mut map);
        changed |= c;
        errors.extend(e);
        for part in &mut asset.parts {
            // Authored parts may contain bound map objects captured by Create Prefab.
            for object in part.geometry_source.geometry_objects_mut() {
                if let Some(next) = map.geometry_objects.iter().find(|o| o.id == object.id) {
                    *object = next.clone();
                }
            }
            if let BlockPropGeometrySource::MZ {
                binding,
                generated_cache,
            } = &mut part.geometry_source
            {
                for object in generated_cache {
                    if let Some(source) = object.properties.get_str(BINDING) {
                        *binding = source.into();
                    }
                }
            }
        }
    }
    project.block_props = assets;
    if let Some(mut map) = project.prefab_editor_map.take() {
        let (c, e) = synchronize_map(project, &mut map);
        changed |= c;
        errors.extend(e);
        project.prefab_editor_map = Some(map);
    }
    (changed, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusterix::{BlockPropAsset, BlockPropComponent, BlockPropPart};
    fn setup() -> (Project, Map, Binding) {
        let mut p = Project::default();
        let choice = choices(&p).remove(0);
        p.procedural_appearances.insert(choice.0, choice.2);
        let mut map = Map::default();
        let object = GeometryObject::box_("Frame", Vec3::new(3., 1., 5.), Vec3::new(8., 2., 6.));
        map.selected_geometry_objects = vec![object.id];
        map.geometry_objects = vec![object];
        let binding = Binding::from_selection(&p, &map, choice.0).unwrap();
        (p, map, binding)
    }
    #[test]
    fn selection_changes_regenerate_with_stable_faces_and_keep_paint_and_supports() {
        let (mut p, mut map, binding) = setup();
        let mut roof = generate(&mut p, &map, &binding, None).unwrap();
        let id = roof.id;
        let face_id = roof.faces[0].id;
        roof.faces[0].tile = Some(PixelSource::PaletteIndex(7));
        roof.faces[0].paint_surface_id = Some(Uuid::new_v4());
        let paint = roof.faces[0].paint_surface_id;
        let input = map.geometry_objects[0].clone();
        map.geometry_objects.push(roof);
        for point in &mut map.geometry_objects[0].vertices {
            point.x *= 1.5;
        }
        let (changed, errors) = synchronize_map(&mut p, &mut map);
        assert!(changed);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(map.geometry_objects[1].id, id);
        assert_eq!(map.geometry_objects[1].faces[0].id, face_id);
        assert_eq!(map.geometry_objects[1].faces[0].paint_surface_id, paint);
        assert_eq!(
            map.geometry_objects[1].faces[0].tile,
            Some(PixelSource::PaletteIndex(7))
        );
        assert_ne!(map.geometry_objects[0], input);
        assert!(map.geometry_objects[1].vertices.iter().any(|p| p.x > 10.));
        assert!(!synchronize_map(&mut p, &mut map).0);
        map.geometry_objects.remove(0);
        let before = map.geometry_objects[0].clone();
        let (changed, errors) = synchronize_map(&mut p, &mut map);
        assert!(!changed);
        assert!(!errors.is_empty());
        assert_eq!(map.geometry_objects[0], before);
    }
    #[test]
    fn rotated_frame_and_selected_face_supply_geometry_in_its_local_axes() {
        let (mut p, mut map, _) = setup();
        for point in &mut map.geometry_objects[0].vertices {
            let (sin, cos) = 0.7f32.sin_cos();
            let x = point.x;
            let z = point.z;
            point.x = cos * x - sin * z;
            point.z = sin * x + cos * z;
        }
        map.selected_geometry_objects.clear();
        map.selected_geometry_faces = vec![(map.geometry_objects[0].id, 4)];
        let choice = choices(&p).remove(0);
        let binding = Binding::from_selection(&p, &map, choice.0).unwrap();
        let roof = generate(&mut p, &map, &binding, None).unwrap();
        assert!(
            (roof
                .vertices
                .iter()
                .map(|v| v.y)
                .fold(f32::INFINITY, f32::min)
                - 2.)
                .abs()
                < 0.001
        );
        assert!(
            roof.vertices
                .iter()
                .all(|p| p.x.is_finite() && p.z.is_finite())
        );
    }
    #[test]
    fn moving_input_and_output_together_does_not_apply_translation_twice() {
        let (mut project, mut map, binding) = setup();
        let roof = generate(&mut project, &map, &binding, None).unwrap();
        let lo = roof.bbox().unwrap().min;
        map.geometry_objects.push(roof);
        for object in &mut map.geometry_objects {
            object.transform[3][0] += 9.;
            object.transform[3][2] -= 4.;
        }
        let (changed, errors) = synchronize_map(&mut project, &mut map);
        assert!(changed);
        assert!(errors.is_empty());
        let moved = map.geometry_objects[1].bbox().unwrap().min;
        assert!((moved.x - lo.x - 9.).abs() < 0.001);
        assert!((moved.y - lo.y + 4.).abs() < 0.001);
    }
    #[test]
    fn recipe_backed_prefab_roundtrip_preserves_part_identity_pivot_and_behavior() {
        let (mut p, map, binding) = setup();
        let roof = generate(&mut p, &map, &binding, None).unwrap();
        let mut asset = BlockPropAsset::new_authored("Frame", map.geometry_objects.clone());
        let mut part = BlockPropPart::new_authored("Roof", vec![]);
        part.pivot = [0., 2., 0.];
        part.geometry_source = BlockPropGeometrySource::MZ {
            binding: roof.properties.get_str(BINDING).unwrap().into(),
            generated_cache: vec![roof.clone()],
        };
        let part_id = part.id;
        asset.parts.push(part);
        let support = rusterix::BlockPropSupportSurface {
            id: Uuid::new_v4(),
            name: "Roof surface".into(),
            part_id,
            shape: rusterix::BlockPropSemanticShape::Faces(vec![rusterix::BlockPropFaceRef {
                object_id: roof.id,
                face_id: roof.faces[0].id,
            }]),
            snap_spacing: 0.25,
            allowed_item_tags: vec![],
            capacity: None,
            occupancy_policy: Default::default(),
        };
        asset.support_surfaces.push(support.clone());
        let mut door = BlockPropComponent::new("Door");
        door.properties.set("part_id", Value::Id(part_id));
        asset.components.push(door.clone());
        let id = asset.id;
        p.block_props.insert(id, asset);
        let mut reopened =
            crate::project_io::decode_project(&crate::project_io::encode_project(&p).unwrap())
                .unwrap();
        let source = reopened
            .procedural_appearances
            .get_mut(&binding.asset)
            .unwrap();
        let mut doc = Document::load(source).unwrap();
        let denrim_materials::geometry::Operation::Group { controls, .. } =
            &mut doc.rows[1].tiles[0].op
        else {
            panic!()
        };
        controls[0].value = 2.;
        *source = doc.save().unwrap();
        let (changed, errors) = synchronize(&mut reopened);
        assert!(changed);
        assert!(errors.is_empty(), "{errors:?}");
        let asset = &reopened.block_props[&id];
        assert_eq!(asset.parts[1].id, part_id);
        assert_eq!(asset.parts[1].pivot, [0., 2., 0.]);
        assert_eq!(asset.components, vec![door]);
        assert_eq!(asset.support_surfaces, vec![support]);
        let cache = asset.parts[1].geometry_source.geometry_objects();
        assert_eq!(cache[0].id, roof.id);
        assert_eq!(cache[0].faces[0].id, roof.faces[0].id);
        assert!(cache[0].vertices.iter().any(|v| v.y > 4.));
        // Runtime meshes are disposable caches; the source rebuilds the same IDs.
        let part = &mut reopened.block_props.get_mut(&id).unwrap().parts[1];
        part.geometry_source.geometry_objects_mut().clear();
        let (changed, errors) = synchronize(&mut reopened);
        assert!(changed);
        assert!(errors.is_empty(), "{errors:?}");
        let cache = reopened.block_props[&id].parts[1]
            .geometry_source
            .geometry_objects();
        assert_eq!(cache[0].id, roof.id);
        assert_eq!(cache[0].faces[0].id, roof.faces[0].id);
    }
}
