use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockPropCreateMode {
    ReplaceSelection,
    KeepSelection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreatedBlockProp {
    pub asset_id: Uuid,
    pub instance_id: Option<Uuid>,
    pub source_object_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MadeUniqueBlockProp {
    pub instance_id: Uuid,
    pub source_asset_id: Uuid,
    pub unique_asset_id: Uuid,
}

/// Opens an authored Prefab as an isolated ordinary geometry map. The normal
/// 3D Object/Vertex/Edge/Face tools can therefore edit it without special
/// editor-only geometry code.
pub fn begin_prefab_editor(project: &mut Project, asset_id: Uuid) -> Result<(), String> {
    let asset = project
        .block_props
        .get(&asset_id)
        .ok_or_else(|| fl!("error_prefab_editor_project_asset"))?;
    let mut map = Map::default();
    map.name = asset.name.clone();
    let mut part_by_object = IndexMap::default();
    for part in &asset.parts {
        for object in part.geometry_source.geometry_objects() {
            part_by_object.insert(object.id, part.id);
            map.geometry_objects.push(object.clone());
        }
    }
    map.clear_selection();
    map.update_surfaces();
    project.prefab_editor_map = Some(map);
    project.prefab_editor_part_by_object = part_by_object;
    project.block_prop_paint.entry(asset_id).or_default();
    Ok(())
}

/// Returns a stable orbit target and distance that frames the isolated asset.
pub fn prefab_editor_camera_frame(project: &Project) -> Option<(Vec3<f32>, f32)> {
    let map = project.prefab_editor_map.as_ref()?;
    let mut min = Vec3::broadcast(f32::INFINITY);
    let mut max = Vec3::broadcast(f32::NEG_INFINITY);
    let mut found = false;

    for object in &map.geometry_objects {
        for vertex in &object.vertices {
            let point = object.transform_point(*vertex);
            if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
                continue;
            }
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
            found = true;
        }
    }

    found.then(|| {
        let center = (min + max) * 0.5;
        let radius = ((max - min) * 0.5).magnitude().max(0.5);
        (center, (radius * 3.0).clamp(2.5, 100.0))
    })
}

/// Writes the current isolated editor map back into the selected asset while
/// preserving the asset UUID and its placement instances.
pub fn sync_prefab_editor(project: &mut Project, asset_id: Uuid) -> Result<(), String> {
    let geometry_objects = project
        .prefab_editor_map
        .as_ref()
        .ok_or_else(|| fl!("error_prefab_editor_not_open"))?
        .geometry_objects
        .clone();
    let asset = project
        .block_props
        .get_mut(&asset_id)
        .ok_or_else(|| fl!("error_prefab_editor_project_asset"))?;
    if asset.parts.is_empty() {
        asset.parts.push(rusterix::BlockPropPart::new_authored(
            fl!("prefab_geometry_part"),
            Vec::new(),
        ));
    }

    let valid_part_ids = asset
        .parts
        .iter()
        .map(|part| part.id)
        .collect::<FxHashSet<_>>();
    let fallback_part_id = asset.parts[0].id;
    let mut objects_by_part: IndexMap<Uuid, Vec<rusterix::GeometryObject>> = asset
        .parts
        .iter()
        .map(|part| (part.id, Vec::new()))
        .collect();
    for object in geometry_objects {
        let part_id = project
            .prefab_editor_part_by_object
            .get(&object.id)
            .copied()
            .filter(|part_id| valid_part_ids.contains(part_id))
            .unwrap_or(fallback_part_id);
        project
            .prefab_editor_part_by_object
            .insert(object.id, part_id);
        objects_by_part.entry(part_id).or_default().push(object);
    }

    for part in &mut asset.parts {
        if let rusterix::BlockPropGeometrySource::Authored { geometry_objects } =
            &mut part.geometry_source
        {
            *geometry_objects = objects_by_part.shift_remove(&part.id).unwrap_or_default();
        }
    }
    Ok(())
}

/// Select every isolated Geometry Object owned by one stable Prefab part.
pub fn select_prefab_part(project: &mut Project, part_id: Uuid) -> bool {
    let Some(map) = project.prefab_editor_map.as_mut() else {
        return false;
    };
    let selected = project
        .prefab_editor_part_by_object
        .iter()
        .filter_map(|(object_id, owner)| (*owner == part_id).then_some(*object_id))
        .collect::<Vec<_>>();
    map.clear_selection();
    map.selected_geometry_objects = selected;
    !map.selected_geometry_objects.is_empty()
}

/// Move the current object selection into a new stable Prefab part.
pub fn create_prefab_part_from_selection(
    project: &mut Project,
    asset_id: Uuid,
    name: impl Into<String>,
) -> Result<Uuid, String> {
    let selected = project
        .prefab_editor_map
        .as_ref()
        .ok_or_else(|| fl!("error_prefab_editor_not_open"))?
        .selected_geometry_objects
        .clone();
    if selected.is_empty() {
        return Err(fl!("status_prefab_part_select_objects"));
    }

    let part = rusterix::BlockPropPart::new_authored(name, Vec::new());
    let part_id = part.id;
    project
        .block_props
        .get_mut(&asset_id)
        .ok_or_else(|| fl!("error_prefab_editor_project_asset"))?
        .parts
        .push(part);
    for object_id in selected {
        project
            .prefab_editor_part_by_object
            .insert(object_id, part_id);
    }
    sync_prefab_editor(project, asset_id)?;
    Ok(part_id)
}

pub fn rename_prefab_part(
    project: &mut Project,
    asset_id: Uuid,
    part_id: Uuid,
    name: String,
) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(fl!("status_prefab_part_name_required"));
    }
    let part = project
        .block_props
        .get_mut(&asset_id)
        .and_then(|asset| asset.parts.iter_mut().find(|part| part.id == part_id))
        .ok_or_else(|| fl!("status_prefab_part_missing"))?;
    part.name = name.to_string();
    Ok(())
}

pub fn set_prefab_part_parent(
    project: &mut Project,
    asset_id: Uuid,
    part_id: Uuid,
    parent_part_id: Option<Uuid>,
) -> Result<(), String> {
    let asset = project
        .block_props
        .get(&asset_id)
        .ok_or_else(|| fl!("error_prefab_editor_project_asset"))?;
    if asset.find_part(part_id).is_none() {
        return Err(fl!("status_prefab_part_missing"));
    }
    if let Some(parent_id) = parent_part_id {
        if asset.find_part(parent_id).is_none() {
            return Err(fl!("status_prefab_parent_missing"));
        }
        let mut current = Some(parent_id);
        let mut visited = FxHashSet::default();
        while let Some(candidate_id) = current {
            if candidate_id == part_id || !visited.insert(candidate_id) {
                return Err(fl!("status_prefab_parent_cycle"));
            }
            current = asset
                .find_part(candidate_id)
                .and_then(|candidate| candidate.parent_part_id);
        }
    }
    project
        .block_props
        .get_mut(&asset_id)
        .and_then(|asset| asset.parts.iter_mut().find(|part| part.id == part_id))
        .ok_or_else(|| fl!("status_prefab_part_missing"))?
        .parent_part_id = parent_part_id;
    Ok(())
}

pub fn move_prefab_selection_to_part(
    project: &mut Project,
    asset_id: Uuid,
    part_id: Uuid,
) -> Result<usize, String> {
    if project
        .block_props
        .get(&asset_id)
        .and_then(|asset| asset.find_part(part_id))
        .is_none()
    {
        return Err(fl!("status_prefab_part_missing"));
    }
    let selected = project
        .prefab_editor_map
        .as_ref()
        .ok_or_else(|| fl!("error_prefab_editor_not_open"))?
        .selected_geometry_objects
        .clone();
    if selected.is_empty() {
        return Err(fl!("status_prefab_part_select_objects"));
    }
    for object_id in &selected {
        project
            .prefab_editor_part_by_object
            .insert(*object_id, part_id);
    }
    sync_prefab_editor(project, asset_id)?;
    Ok(selected.len())
}

/// Remove a part without deleting its geometry. Objects are reassigned to the
/// first remaining part so this action is recoverable and never destroys mesh.
pub fn remove_prefab_part(
    project: &mut Project,
    asset_id: Uuid,
    part_id: Uuid,
) -> Result<Uuid, String> {
    let asset = project
        .block_props
        .get_mut(&asset_id)
        .ok_or_else(|| fl!("error_prefab_editor_project_asset"))?;
    if asset.parts.len() <= 1 {
        return Err(fl!("status_prefab_part_keep_one"));
    }
    let fallback_id = asset
        .parts
        .iter()
        .find(|part| part.id != part_id)
        .map(|part| part.id)
        .ok_or_else(|| fl!("status_prefab_part_keep_one"))?;
    asset.parts.retain(|part| part.id != part_id);
    for part in &mut asset.parts {
        if part.parent_part_id == Some(part_id) {
            part.parent_part_id = None;
        }
    }
    asset
        .support_surfaces
        .retain(|surface| surface.part_id != part_id);
    asset
        .interaction_targets
        .retain(|target| target.part_id != part_id);
    asset
        .components
        .retain(|component| component.properties.get_id("part_id") != Some(part_id));
    for owner in project.prefab_editor_part_by_object.values_mut() {
        if *owner == part_id {
            *owner = fallback_id;
        }
    }
    sync_prefab_editor(project, asset_id)?;
    Ok(fallback_id)
}

fn prefab_selection_center(project: &Project) -> Option<Vec3<f32>> {
    let map = project.prefab_editor_map.as_ref()?;
    if !map.selected_geometry_vertices.is_empty() {
        let mut sum = Vec3::zero();
        let mut count = 0usize;
        for (object_id, vertex_index) in &map.selected_geometry_vertices {
            let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
            else {
                continue;
            };
            if let Some(vertex) = object.vertices.get(*vertex_index) {
                sum += object.transform_point(*vertex);
                count += 1;
            }
        }
        if count > 0 {
            return Some(sum / count as f32);
        }
    }
    if !map.selected_geometry_faces.is_empty() {
        let mut points = FxHashSet::default();
        let mut sum = Vec3::zero();
        let mut count = 0usize;
        for (object_id, face_index) in &map.selected_geometry_faces {
            let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
            else {
                continue;
            };
            let Some(face) = object.faces.get(*face_index) else {
                continue;
            };
            for vertex_index in &face.indices {
                if points.insert((*object_id, *vertex_index))
                    && let Some(vertex) = object.vertices.get(*vertex_index)
                {
                    sum += object.transform_point(*vertex);
                    count += 1;
                }
            }
        }
        if count > 0 {
            return Some(sum / count as f32);
        }
    }
    let selected = map
        .selected_geometry_objects
        .iter()
        .copied()
        .collect::<FxHashSet<_>>();
    let mut sum = Vec3::zero();
    let mut count = 0usize;
    for object in map
        .geometry_objects
        .iter()
        .filter(|object| selected.contains(&object.id))
    {
        for vertex in &object.vertices {
            sum += object.transform_point(*vertex);
            count += 1;
        }
    }
    (count > 0).then(|| sum / count as f32)
}

pub fn set_prefab_part_pivot_from_selection(
    project: &mut Project,
    asset_id: Uuid,
    part_id: Uuid,
) -> Result<[f32; 3], String> {
    let center = prefab_selection_center(project)
        .ok_or_else(|| fl!("status_prefab_part_select_pivot_geometry"))?;
    let pivot = [center.x, center.y, center.z];
    let part = project
        .block_props
        .get_mut(&asset_id)
        .and_then(|asset| asset.parts.iter_mut().find(|part| part.id == part_id))
        .ok_or_else(|| fl!("status_prefab_part_missing"))?;
    part.pivot = pivot;
    Ok(pivot)
}

pub fn configure_prefab_door(
    project: &mut Project,
    asset_id: Uuid,
    part_id: Uuid,
    angle_degrees: f32,
) -> Result<Uuid, String> {
    if !angle_degrees.is_finite() || angle_degrees.abs() < 1.0 || angle_degrees.abs() > 180.0 {
        return Err(fl!("status_prefab_door_angle_invalid"));
    }
    let asset = project
        .block_props
        .get_mut(&asset_id)
        .ok_or_else(|| fl!("error_prefab_editor_project_asset"))?;
    if asset.find_part(part_id).is_none() {
        return Err(fl!("status_prefab_part_missing"));
    }
    if let Some(component) = asset.components.iter_mut().find(|component| {
        component.kind == "Door" && component.properties.get_id("part_id") == Some(part_id)
    }) {
        component
            .properties
            .set("angle_degrees", Value::Float(angle_degrees));
        if component.properties.get("interaction_range").is_none() {
            component
                .properties
                .set("interaction_range", Value::Float(3.0));
        }
        return Ok(component.id);
    }

    let mut component = rusterix::BlockPropComponent::new("Door");
    component.properties.set("part_id", Value::Id(part_id));
    component
        .properties
        .set("motion", Value::Str("Swing".to_string()));
    component
        .properties
        .set("angle_degrees", Value::Float(angle_degrees));
    component.properties.set("duration", Value::Float(0.35));
    component
        .properties
        .set("interaction_range", Value::Float(3.0));
    let component_id = component.id;
    asset.components.push(component);
    if asset.default_state.get("open").is_none() {
        asset.default_state.set("open", Value::Bool(false));
    }
    Ok(component_id)
}

pub fn create_prefab_interaction_target_from_selected_faces(
    project: &mut Project,
    asset_id: Uuid,
    part_id: Uuid,
    name: impl Into<String>,
) -> Result<Uuid, String> {
    let component_id = project
        .block_props
        .get(&asset_id)
        .and_then(|asset| {
            asset.components.iter().find(|component| {
                component.kind == "Door" && component.properties.get_id("part_id") == Some(part_id)
            })
        })
        .map(|component| component.id)
        .ok_or_else(|| fl!("status_prefab_door_required"))?;
    let map = project
        .prefab_editor_map
        .as_ref()
        .ok_or_else(|| fl!("error_prefab_editor_not_open"))?;
    if map.selected_geometry_faces.is_empty() {
        return Err(fl!("status_prefab_target_select_faces"));
    }
    let mut faces = Vec::new();
    for (object_id, face_index) in &map.selected_geometry_faces {
        if project.prefab_editor_part_by_object.get(object_id) != Some(&part_id) {
            return Err(fl!("status_prefab_target_wrong_part"));
        }
        let object = map
            .geometry_objects
            .iter()
            .find(|object| object.id == *object_id)
            .ok_or_else(|| fl!("status_prefab_target_face_missing"))?;
        let face = object
            .faces
            .get(*face_index)
            .ok_or_else(|| fl!("status_prefab_target_face_missing"))?;
        faces.push(rusterix::BlockPropFaceRef {
            object_id: *object_id,
            face_id: face.id,
        });
    }
    let pivot = project
        .block_props
        .get(&asset_id)
        .and_then(|asset| asset.find_part(part_id))
        .map(|part| part.pivot)
        .ok_or_else(|| fl!("status_prefab_part_missing"))?;
    let target = rusterix::BlockPropInteractionTarget {
        id: Uuid::new_v4(),
        name: name.into(),
        part_id,
        shape: rusterix::BlockPropSemanticShape::Faces(faces),
        interaction_anchor: pivot,
        facing_direction: [0.0, 0.0, 1.0],
        component_id: Some(component_id),
    };
    let target_id = target.id;
    project
        .block_props
        .get_mut(&asset_id)
        .ok_or_else(|| fl!("error_prefab_editor_project_asset"))?
        .interaction_targets
        .push(target);
    Ok(target_id)
}

/// Add each visible Prefab source paint layer used by a map to the layer that
/// is uploaded for the region. Surface IDs intentionally remain source-local,
/// so every linked instance samples the same authored paint.
pub fn merge_prefab_paint_for_map(
    target: &mut IsoPaintLayer,
    map: &Map,
    paint_catalog: &IndexMap<Uuid, IsoPaintLayer>,
) {
    if !target.visible {
        target.chunks.clear();
        target.baked_chunks.clear();
        target.surface_commit_strokes.clear();
    }
    let mut asset_ids = FxHashSet::default();
    for instance in &map.block_prop_instances {
        asset_ids.insert(instance.asset_id);
    }
    for asset_id in asset_ids {
        let Some(source) = paint_catalog.get(&asset_id).filter(|paint| paint.visible) else {
            continue;
        };
        target.visible = true;
        for (key, chunk) in &source.chunks {
            let mut chunk = chunk.clone();
            // Surface paint is shared directly. Free-standing paint stamps need
            // a per-instance transform expansion and are deliberately deferred.
            chunk.stamps.clear();
            if let Some(existing) = target.chunks.get_mut(key) {
                existing.revision = existing.revision.max(chunk.revision);
                existing.stamp_revision = existing.stamp_revision.max(chunk.stamp_revision);
                existing.strokes.extend(chunk.strokes);
            } else {
                target.chunks.insert(key.clone(), chunk);
            }
        }
        for (key, chunk) in &source.baked_chunks {
            target.baked_chunks.insert(key.clone(), chunk.clone());
        }
        for stroke_id in &source.surface_commit_strokes {
            if !target.surface_commit_strokes.contains(stroke_id) {
                target.surface_commit_strokes.push(*stroke_id);
            }
        }
    }
}

fn selected_geometry(map: &Map) -> Vec<rusterix::GeometryObject> {
    map.selected_geometry_objects
        .iter()
        .filter_map(|id| {
            map.geometry_objects
                .iter()
                .find(|object| object.id == *id)
                .cloned()
        })
        .collect()
}

fn bottom_center(objects: &[rusterix::GeometryObject]) -> Option<Vec3<f32>> {
    let mut min = Vec3::broadcast(f32::INFINITY);
    let mut max = Vec3::broadcast(f32::NEG_INFINITY);
    let mut found = false;

    for object in objects {
        for vertex in &object.vertices {
            let world = object.transform_point(*vertex);
            if !world.x.is_finite() || !world.y.is_finite() || !world.z.is_finite() {
                continue;
            }
            min.x = min.x.min(world.x);
            min.y = min.y.min(world.y);
            min.z = min.z.min(world.z);
            max.x = max.x.max(world.x);
            max.y = max.y.max(world.y);
            max.z = max.z.max(world.z);
            found = true;
        }
    }

    found.then(|| Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5))
}

fn localize_at_bottom_center(
    mut objects: Vec<rusterix::GeometryObject>,
) -> Result<(Vec<rusterix::GeometryObject>, Vec3<f32>), String> {
    let origin = bottom_center(&objects).ok_or_else(|| fl!("error_prefab_no_usable_vertices"))?;
    for object in &mut objects {
        object.transform[3][0] -= origin.x;
        object.transform[3][1] -= origin.y;
        object.transform[3][2] -= origin.z;
        object.kind = rusterix::GeometryObjectKind::Prop;
    }
    Ok((objects, origin))
}

fn remap_semantic_shape_ids(
    shape: &mut rusterix::BlockPropSemanticShape,
    object_ids: &FxHashMap<Uuid, Uuid>,
    face_ids: &FxHashMap<Uuid, Uuid>,
) {
    if let rusterix::BlockPropSemanticShape::Faces(faces) = shape {
        for face in faces {
            if let Some(id) = object_ids.get(&face.object_id) {
                face.object_id = *id;
            }
            if let Some(id) = face_ids.get(&face.face_id) {
                face.face_id = *id;
            }
        }
    }
}

fn regenerate_asset_internal_ids(asset: &mut rusterix::BlockPropAsset) -> FxHashMap<Uuid, Uuid> {
    let part_ids = asset
        .parts
        .iter()
        .map(|part| (part.id, Uuid::new_v4()))
        .collect::<FxHashMap<_, _>>();
    let component_ids = asset
        .components
        .iter()
        .map(|component| (component.id, Uuid::new_v4()))
        .collect::<FxHashMap<_, _>>();
    let mut object_ids = FxHashMap::default();
    let mut face_ids = FxHashMap::default();

    for part in &mut asset.parts {
        part.id = part_ids[&part.id];
        part.parent_part_id = part
            .parent_part_id
            .and_then(|parent_id| part_ids.get(&parent_id).copied());
        for attachment in &mut part.attachments {
            attachment.id = Uuid::new_v4();
        }
        let objects = match &mut part.geometry_source {
            rusterix::BlockPropGeometrySource::Authored { geometry_objects } => geometry_objects,
            rusterix::BlockPropGeometrySource::Recipe {
                generated_cache, ..
            } => generated_cache,
        };
        for object in objects {
            let old_object_id = object.id;
            object.id = Uuid::new_v4();
            object_ids.insert(old_object_id, object.id);
            for face in &mut object.faces {
                let old_face_id = face.id;
                face.id = Uuid::new_v4();
                face_ids.insert(old_face_id, face.id);
            }
        }
    }
    for component in &mut asset.components {
        if let Some(part_id) = component.properties.get_id("part_id")
            && let Some(remapped) = part_ids.get(&part_id)
        {
            component.properties.set("part_id", Value::Id(*remapped));
        }
        component.id = component_ids[&component.id];
    }

    let mut surface_ids = FxHashMap::default();
    for surface in &mut asset.support_surfaces {
        let old_id = surface.id;
        surface.id = Uuid::new_v4();
        surface_ids.insert(old_id, surface.id);
        if let Some(part_id) = part_ids.get(&surface.part_id) {
            surface.part_id = *part_id;
        }
        remap_semantic_shape_ids(&mut surface.shape, &object_ids, &face_ids);
    }
    for target in &mut asset.interaction_targets {
        target.id = Uuid::new_v4();
        if let Some(part_id) = part_ids.get(&target.part_id) {
            target.part_id = *part_id;
        }
        target.component_id = target
            .component_id
            .and_then(|component_id| component_ids.get(&component_id).copied());
        remap_semantic_shape_ids(&mut target.shape, &object_ids, &face_ids);
    }
    surface_ids
}

/// Creates an authored asset from the current object selection. Source objects
/// are stored relative to a bottom-center origin so the linked asset rests on
/// the placement surface when instantiated.
pub fn create_authored_block_prop(
    project: &mut Project,
    server_ctx: &ServerContext,
    name: impl Into<String>,
    mode: BlockPropCreateMode,
) -> Result<CreatedBlockProp, String> {
    let mut map = project
        .get_map(server_ctx)
        .cloned()
        .ok_or_else(|| fl!("error_prefab_needs_editable_map"))?;
    let source_objects = selected_geometry(&map);
    if source_objects.is_empty() {
        return Err(fl!("error_prefab_select_geometry"));
    }
    let (local_objects, origin) = localize_at_bottom_center(source_objects.clone())?;

    let mut asset = rusterix::BlockPropAsset::new_authored(name, local_objects);
    if mode == BlockPropCreateMode::KeepSelection {
        // The kept region geometry is independent. Give the Prefab source its
        // own surface identities so later asset-local paint cannot leak onto
        // the original objects that remained in the region.
        regenerate_asset_internal_ids(&mut asset);
    }
    asset.category = "Project Props".to_string();
    let asset_id = asset.id;
    project.block_props.insert(asset_id, asset);

    let instance_id = if mode == BlockPropCreateMode::ReplaceSelection {
        let selected: FxHashSet<Uuid> = map.selected_geometry_objects.iter().copied().collect();
        map.geometry_objects
            .retain(|object| !selected.contains(&object.id));
        map.clear_selection();

        let mut instance = rusterix::BlockPropInstance::new(asset_id);
        instance.world_transform[3][0] = origin.x;
        instance.world_transform[3][1] = origin.y;
        instance.world_transform[3][2] = origin.z;
        let instance_id = instance.id;
        map.block_prop_instances.push(instance);
        Some(instance_id)
    } else {
        None
    };

    let target_map = project
        .get_map_mut(server_ctx)
        .ok_or_else(|| fl!("error_prefab_map_changed_create"))?;
    *target_map = map;

    Ok(CreatedBlockProp {
        asset_id,
        instance_id,
        source_object_count: source_objects.len(),
    })
}

/// Replaces the authored geometry of an existing project prop while preserving
/// its UUID, metadata, and all linked instances. Semantics are cleared because
/// their part and face references belong to the previous source geometry.
pub fn update_authored_block_prop(
    project: &mut Project,
    server_ctx: &ServerContext,
    asset_id: Uuid,
) -> Result<usize, String> {
    let map = project
        .get_map(server_ctx)
        .ok_or_else(|| fl!("error_prefab_needs_editable_map"))?;
    let source_objects = selected_geometry(map);
    if source_objects.is_empty() {
        return Err(fl!("error_prefab_select_geometry"));
    }
    let object_count = source_objects.len();
    let (local_objects, _) = localize_at_bottom_center(source_objects)?;
    let asset = project
        .block_props
        .get_mut(&asset_id)
        .ok_or_else(|| fl!("error_prefab_select_project_prefab"))?;
    asset.parts = vec![rusterix::BlockPropPart::new_authored(
        "Geometry",
        local_objects,
    )];
    asset.support_surfaces.clear();
    asset.interaction_targets.clear();
    asset.components.clear();
    project.block_prop_paint.shift_remove(&asset_id);
    Ok(object_count)
}

/// Duplicates the asset referenced by the single selected linked instance and
/// redirects only that instance to the new asset UUID.
pub fn make_selected_block_prop_unique(
    project: &mut Project,
    server_ctx: &ServerContext,
) -> Result<MadeUniqueBlockProp, String> {
    let (instance_id, source_asset_id) = {
        let map = project
            .get_map(server_ctx)
            .ok_or_else(|| fl!("error_prefab_needs_editable_map"))?;
        if map.selected_block_prop_instances.len() != 1 {
            return Err(fl!("error_prefab_select_one_instance"));
        }
        let instance_id = map.selected_block_prop_instances[0];
        let instance = map
            .block_prop_instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .ok_or_else(|| fl!("error_prefab_instance_missing"))?;
        (instance_id, instance.asset_id)
    };

    let mut unique_asset = project
        .block_props
        .get(&source_asset_id)
        .cloned()
        .ok_or_else(|| fl!("error_prefab_unique_project_only"))?;
    unique_asset.id = Uuid::new_v4();
    unique_asset.name = format!("{} Copy", unique_asset.name);
    let surface_ids = regenerate_asset_internal_ids(&mut unique_asset);
    let unique_asset_id = unique_asset.id;
    project.block_props.insert(unique_asset_id, unique_asset);

    let map = project
        .get_map_mut(server_ctx)
        .ok_or_else(|| fl!("error_prefab_map_changed_duplicate"))?;
    let instance = map
        .block_prop_instances
        .iter_mut()
        .find(|instance| instance.id == instance_id)
        .ok_or_else(|| fl!("error_prefab_instance_missing"))?;
    instance.asset_id = unique_asset_id;
    for placement in &mut map.block_prop_surface_placements {
        if placement.prop_instance_id == instance_id
            && let Some(surface_id) = surface_ids.get(&placement.surface_id)
        {
            placement.surface_id = *surface_id;
        }
    }

    Ok(MadeUniqueBlockProp {
        instance_id,
        source_asset_id,
        unique_asset_id,
    })
}

/// Resolves the selected linked instances into ordinary editable Geometry
/// Objects and removes the links. Other instances remain connected to their
/// assets.
pub fn unpack_selected_block_props(
    project: &mut Project,
    server_ctx: &ServerContext,
) -> Result<usize, String> {
    let mut map = project
        .get_map(server_ctx)
        .cloned()
        .ok_or_else(|| fl!("error_prefab_needs_editable_map"))?;
    if map.selected_block_prop_instances.is_empty() {
        return Err(fl!("error_prefab_select_instances"));
    }
    let selected = map
        .selected_block_prop_instances
        .iter()
        .copied()
        .collect::<FxHashSet<_>>();
    let instances = map
        .block_prop_instances
        .iter()
        .filter(|instance| selected.contains(&instance.id))
        .cloned()
        .collect::<Vec<_>>();
    if instances.is_empty() {
        return Err(fl!("error_prefab_instances_missing"));
    }

    let resolution = rusterix::resolve_block_prop_geometry(&instances, &project.block_props);
    if resolution.geometry_objects.is_empty() || !resolution.diagnostics.is_empty() {
        return Err(fl!("error_prefab_unpack_resolution"));
    }
    let mut unpacked = resolution.geometry_objects;
    for object in &mut unpacked {
        object.kind = rusterix::GeometryObjectKind::Prop;
        object.group.clear();
        object.tags.retain(|tag| tag != "block_prop_placeholder");
    }
    let unpacked_ids = unpacked.iter().map(|object| object.id).collect::<Vec<_>>();
    map.block_prop_instances
        .retain(|instance| !selected.contains(&instance.id));
    map.block_prop_surface_placements
        .retain(|placement| !selected.contains(&placement.prop_instance_id));
    map.clear_selection();
    map.selected_geometry_objects = unpacked_ids;
    let count = unpacked.len();
    map.geometry_objects.extend(unpacked);

    let target_map = project
        .get_map_mut(server_ctx)
        .ok_or_else(|| fl!("error_prefab_map_changed_unpack"))?;
    *target_map = map;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with_selected_box() -> (Project, ServerContext, Uuid) {
        let mut project = Project::default();
        let mut region = Region::default();
        let region_id = region.id;
        let mut object = rusterix::GeometryObject::box_from_bounds(
            "Table",
            Vec3::new(4.0, 2.0, 8.0),
            Vec3::new(6.0, 3.0, 10.0),
        );
        object.transform[3][0] = 1.0;
        let object_id = object.id;
        region.map.geometry_objects.push(object);
        region.map.selected_geometry_objects.push(object_id);
        project.regions.push(region);

        let mut server_ctx = ServerContext::default();
        server_ctx.curr_region = region_id;
        server_ctx.pc = ProjectContext::Region(region_id);
        server_ctx.editor_view_mode = EditorViewMode::Orbit;
        (project, server_ctx, object_id)
    }

    #[test]
    fn create_and_replace_keeps_world_position_through_linked_instance() {
        let (mut project, server_ctx, object_id) = project_with_selected_box();
        let created = create_authored_block_prop(
            &mut project,
            &server_ctx,
            "Table",
            BlockPropCreateMode::ReplaceSelection,
        )
        .unwrap();

        let map = project.get_map(&server_ctx).unwrap();
        assert!(
            map.geometry_objects
                .iter()
                .all(|object| object.id != object_id)
        );
        assert_eq!(map.block_prop_instances.len(), 1);
        assert_eq!(map.block_prop_instances[0].asset_id, created.asset_id);
        assert_eq!(map.block_prop_instances[0].world_transform[3][0], 6.0);
        assert_eq!(map.block_prop_instances[0].world_transform[3][1], 2.0);
        assert_eq!(map.block_prop_instances[0].world_transform[3][2], 9.0);

        let resolved =
            rusterix::resolve_block_prop_geometry(&map.block_prop_instances, &project.block_props);
        let resolved = &resolved.geometry_objects[0];
        let mut min = Vec3::broadcast(f32::INFINITY);
        let mut max = Vec3::broadcast(f32::NEG_INFINITY);
        for vertex in &resolved.vertices {
            let world = resolved.transform_point(*vertex);
            min.x = min.x.min(world.x);
            min.y = min.y.min(world.y);
            min.z = min.z.min(world.z);
            max.x = max.x.max(world.x);
            max.y = max.y.max(world.y);
            max.z = max.z.max(world.z);
        }
        assert_eq!(min, Vec3::new(5.0, 2.0, 8.0));
        assert_eq!(max, Vec3::new(7.0, 3.0, 10.0));
    }

    #[test]
    fn create_and_keep_preserves_source_geometry() {
        let (mut project, server_ctx, object_id) = project_with_selected_box();
        let created = create_authored_block_prop(
            &mut project,
            &server_ctx,
            "Table",
            BlockPropCreateMode::KeepSelection,
        )
        .unwrap();

        let map = project.get_map(&server_ctx).unwrap();
        assert!(
            map.geometry_objects
                .iter()
                .any(|object| object.id == object_id)
        );
        assert!(map.block_prop_instances.is_empty());
        assert!(created.instance_id.is_none());
        assert!(project.block_props.contains_key(&created.asset_id));
        let prefab_object_id = project.block_props[&created.asset_id].parts[0]
            .geometry_source
            .geometry_objects()[0]
            .id;
        assert_ne!(prefab_object_id, object_id);
    }

    #[test]
    fn isolated_editor_keeps_geometry_assigned_to_stable_parts() {
        let (mut project, server_ctx, _) = project_with_selected_box();
        let mut second = rusterix::GeometryObject::box_from_bounds(
            "Door Leaf",
            Vec3::new(7.0, 2.0, 8.0),
            Vec3::new(8.0, 5.0, 9.0),
        );
        second.ensure_face_paint_data();
        let second_id = second.id;
        let map = project.get_map_mut(&server_ctx).unwrap();
        map.geometry_objects.push(second);
        map.selected_geometry_objects.push(second_id);

        let created = create_authored_block_prop(
            &mut project,
            &server_ctx,
            "Door",
            BlockPropCreateMode::ReplaceSelection,
        )
        .unwrap();
        begin_prefab_editor(&mut project, created.asset_id).unwrap();
        let editor_object_id = project.prefab_editor_map.as_ref().unwrap().geometry_objects[1].id;
        project
            .prefab_editor_map
            .as_mut()
            .unwrap()
            .selected_geometry_objects = vec![editor_object_id];
        let leaf_part_id =
            create_prefab_part_from_selection(&mut project, created.asset_id, "Door Leaf").unwrap();

        let asset = &project.block_props[&created.asset_id];
        assert_eq!(asset.parts.len(), 2);
        assert_eq!(asset.find_part(leaf_part_id).unwrap().name, "Door Leaf");
        assert_eq!(
            asset
                .find_part(leaf_part_id)
                .unwrap()
                .geometry_source
                .geometry_objects()
                .len(),
            1
        );
        assert_eq!(
            project.prefab_editor_part_by_object[&editor_object_id],
            leaf_part_id
        );
    }

    #[test]
    fn door_authoring_keeps_stable_hierarchy_pivot_and_face_target() {
        let mut project = Project::default();
        let frame = rusterix::GeometryObject::box_from_bounds(
            "Frame",
            Vec3::new(-0.2, 0.0, -0.2),
            Vec3::new(1.2, 2.2, 0.2),
        );
        let mut leaf = rusterix::GeometryObject::box_from_bounds(
            "Leaf",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 2.0, 0.1),
        );
        leaf.ensure_face_paint_data();
        let leaf_object_id = leaf.id;
        let mut asset = rusterix::BlockPropAsset::new_authored("Door", vec![frame]);
        let root_id = asset.parts[0].id;
        let leaf_part = rusterix::BlockPropPart::new_authored("Door Leaf", vec![leaf]);
        let leaf_part_id = leaf_part.id;
        asset.parts.push(leaf_part);
        let asset_id = asset.id;
        project.block_props.insert(asset_id, asset);
        begin_prefab_editor(&mut project, asset_id).unwrap();

        set_prefab_part_parent(&mut project, asset_id, leaf_part_id, Some(root_id)).unwrap();
        assert!(
            set_prefab_part_parent(&mut project, asset_id, root_id, Some(leaf_part_id)).is_err()
        );

        let map = project.prefab_editor_map.as_mut().unwrap();
        map.selected_geometry_objects = vec![leaf_object_id];
        map.selected_geometry_vertices = vec![(leaf_object_id, 0)];
        let pivot =
            set_prefab_part_pivot_from_selection(&mut project, asset_id, leaf_part_id).unwrap();
        assert_eq!(pivot, [0.0, 0.0, 0.0]);

        let component_id =
            configure_prefab_door(&mut project, asset_id, leaf_part_id, 95.0).unwrap();
        let map = project.prefab_editor_map.as_mut().unwrap();
        map.selected_geometry_faces = vec![(leaf_object_id, 0), (leaf_object_id, 1)];
        let target_id = create_prefab_interaction_target_from_selected_faces(
            &mut project,
            asset_id,
            leaf_part_id,
            "Door Interaction",
        )
        .unwrap();

        let asset = &project.block_props[&asset_id];
        assert_eq!(
            asset.find_part(leaf_part_id).unwrap().parent_part_id,
            Some(root_id)
        );
        assert_eq!(asset.components[0].id, component_id);
        let target = asset.find_interaction_target(target_id).unwrap();
        assert_eq!(target.part_id, leaf_part_id);
        assert_eq!(target.component_id, Some(component_id));
        assert!(matches!(
            &target.shape,
            rusterix::BlockPropSemanticShape::Faces(faces) if faces.len() == 2
        ));
    }

    #[test]
    fn prefab_paint_merge_includes_each_used_asset_once() {
        let asset_id = Uuid::new_v4();
        let mut map = Map::default();
        map.block_prop_instances
            .push(rusterix::BlockPropInstance::new(asset_id));
        map.block_prop_instances
            .push(rusterix::BlockPropInstance::new(asset_id));
        let mut source = IsoPaintLayer::default();
        source
            .chunks
            .insert("surface".to_string(), IsoPaintChunk::new([0, 0]));
        let mut catalog = IndexMap::default();
        catalog.insert(asset_id, source);
        let mut target = IsoPaintLayer::default();

        merge_prefab_paint_for_map(&mut target, &map, &catalog);

        assert_eq!(target.chunks.len(), 1);
        assert!(target.chunks.contains_key("surface"));
    }

    #[test]
    fn update_source_preserves_asset_and_instance_ids() {
        let (mut project, server_ctx, _) = project_with_selected_box();
        let created = create_authored_block_prop(
            &mut project,
            &server_ctx,
            "Table",
            BlockPropCreateMode::KeepSelection,
        )
        .unwrap();
        let mut instance = rusterix::BlockPropInstance::new(created.asset_id);
        let instance_id = instance.id;
        instance.world_transform[3][0] = 20.0;
        project
            .get_map_mut(&server_ctx)
            .unwrap()
            .block_prop_instances
            .push(instance);

        let count =
            update_authored_block_prop(&mut project, &server_ctx, created.asset_id).unwrap();
        assert_eq!(count, 1);
        assert!(project.block_props.contains_key(&created.asset_id));
        let map = project.get_map(&server_ctx).unwrap();
        assert_eq!(map.block_prop_instances[0].id, instance_id);
        assert_eq!(map.block_prop_instances[0].asset_id, created.asset_id);
    }

    #[test]
    fn make_unique_redirects_only_selected_instance_and_unpack_breaks_its_link() {
        let (mut project, server_ctx, _) = project_with_selected_box();
        let created = create_authored_block_prop(
            &mut project,
            &server_ctx,
            "Table",
            BlockPropCreateMode::ReplaceSelection,
        )
        .unwrap();
        let first_instance_id = created.instance_id.unwrap();
        let mut second = rusterix::BlockPropInstance::new(created.asset_id);
        let second_instance_id = second.id;
        second.world_transform[3][0] = 20.0;
        let map = project.get_map_mut(&server_ctx).unwrap();
        map.block_prop_instances.push(second);
        map.selected_block_prop_instances = vec![first_instance_id];

        let unique = make_selected_block_prop_unique(&mut project, &server_ctx).unwrap();
        assert_ne!(unique.unique_asset_id, created.asset_id);
        let map = project.get_map(&server_ctx).unwrap();
        assert_eq!(
            map.block_prop_instances
                .iter()
                .find(|instance| instance.id == first_instance_id)
                .unwrap()
                .asset_id,
            unique.unique_asset_id
        );
        assert_eq!(
            map.block_prop_instances
                .iter()
                .find(|instance| instance.id == second_instance_id)
                .unwrap()
                .asset_id,
            created.asset_id
        );

        let unpacked = unpack_selected_block_props(&mut project, &server_ctx).unwrap();
        assert_eq!(unpacked, 1);
        let map = project.get_map(&server_ctx).unwrap();
        assert!(
            map.block_prop_instances
                .iter()
                .all(|instance| instance.id != first_instance_id)
        );
        assert!(
            map.block_prop_instances
                .iter()
                .any(|instance| instance.id == second_instance_id)
        );
        assert_eq!(map.selected_geometry_objects.len(), 1);
    }

    #[test]
    fn phase_two_acceptance_reuses_updates_and_unpacks_without_changing_siblings() {
        let (mut project, server_ctx, source_object_id) = project_with_selected_box();
        let created = create_authored_block_prop(
            &mut project,
            &server_ctx,
            "Table",
            BlockPropCreateMode::KeepSelection,
        )
        .unwrap();

        let mut instance_ids = Vec::new();
        {
            let map = project.get_map_mut(&server_ctx).unwrap();
            for x in [0.0, 10.0, 20.0] {
                let mut instance = rusterix::BlockPropInstance::new(created.asset_id);
                instance.world_transform[3][0] = x;
                instance_ids.push(instance.id);
                map.block_prop_instances.push(instance);
            }
            let mut wider_source = rusterix::GeometryObject::box_from_bounds(
                "Wider Table",
                Vec3::new(-2.0, 0.0, -1.0),
                Vec3::new(2.0, 1.0, 1.0),
            );
            wider_source.id = source_object_id;
            map.geometry_objects[0] = wider_source;
            map.selected_geometry_objects = vec![source_object_id];
        }

        update_authored_block_prop(&mut project, &server_ctx, created.asset_id).unwrap();
        let map = project.get_map(&server_ctx).unwrap();
        let resolved =
            rusterix::resolve_block_prop_geometry(&map.block_prop_instances, &project.block_props);
        assert_eq!(resolved.geometry_objects.len(), 3);
        for object in &resolved.geometry_objects {
            let xs = object
                .vertices
                .iter()
                .map(|vertex| object.transform_point(*vertex).x)
                .collect::<Vec<_>>();
            let width = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - xs.iter().copied().fold(f32::INFINITY, f32::min);
            assert!((width - 4.0).abs() < 1e-5);
        }

        project
            .get_map_mut(&server_ctx)
            .unwrap()
            .selected_block_prop_instances = vec![instance_ids[1]];
        unpack_selected_block_props(&mut project, &server_ctx).unwrap();
        let map = project.get_map(&server_ctx).unwrap();
        assert_eq!(map.block_prop_instances.len(), 2);
        assert_eq!(map.block_prop_instances[0].id, instance_ids[0]);
        assert_eq!(map.block_prop_instances[1].id, instance_ids[2]);
        assert_eq!(map.selected_geometry_objects.len(), 1);
    }

    #[test]
    fn isolated_editor_writes_back_to_same_asset_and_part_ids() {
        let (mut project, server_ctx, _) = project_with_selected_box();
        let created = create_authored_block_prop(
            &mut project,
            &server_ctx,
            "Table",
            BlockPropCreateMode::KeepSelection,
        )
        .unwrap();
        let original_part_id = project.block_props[&created.asset_id].parts[0].id;

        begin_prefab_editor(&mut project, created.asset_id).unwrap();
        let edited = project.prefab_editor_map.as_mut().unwrap();
        edited.geometry_objects[0].transform[3][0] = 3.0;
        sync_prefab_editor(&mut project, created.asset_id).unwrap();

        let asset = &project.block_props[&created.asset_id];
        assert_eq!(asset.id, created.asset_id);
        assert_eq!(asset.parts[0].id, original_part_id);
        assert_eq!(
            asset.parts[0].geometry_source.geometry_objects()[0].transform[3][0],
            3.0
        );
    }
}
