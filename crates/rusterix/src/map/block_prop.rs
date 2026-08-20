use crate::{GeometryObject, GeometryObjectKind, Value, ValueContainer};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use vek::Vec3;

pub type BlockPropTransform = [[f32; 4]; 4];

pub fn identity_block_prop_transform() -> BlockPropTransform {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn multiply_block_prop_transforms(
    left: BlockPropTransform,
    right: BlockPropTransform,
) -> BlockPropTransform {
    let mut result = [[0.0; 4]; 4];
    for column in 0..4 {
        for row in 0..4 {
            result[column][row] = (0..4)
                .map(|index| left[column][index] * right[index][row])
                .sum();
        }
    }
    result
}

fn translation_block_prop_transform(x: f32, y: f32, z: f32) -> BlockPropTransform {
    let mut transform = identity_block_prop_transform();
    transform[3][0] = x;
    transform[3][1] = y;
    transform[3][2] = z;
    transform
}

fn rotation_y_block_prop_transform(angle_degrees: f32) -> BlockPropTransform {
    let mut transform = identity_block_prop_transform();
    let (sin, cos) = angle_degrees.to_radians().sin_cos();
    transform[0][0] = cos;
    transform[0][2] = -sin;
    transform[2][0] = sin;
    transform[2][2] = cos;
    transform
}

fn door_runtime_key(component_id: Uuid) -> String {
    format!("door_{component_id}_open")
}

fn block_prop_paint_surface_id(object_id: Uuid, face_id: Uuid) -> [u32; 4] {
    let value = face_id.as_u128() ^ object_id.as_u128().rotate_left(1);
    [
        value as u32,
        (value >> 32) as u32,
        (value >> 64) as u32,
        (value >> 96) as u32,
    ]
}

fn transform_block_prop_point(transform: BlockPropTransform, point: [f32; 3]) -> Vec3<f32> {
    Vec3::new(
        point[0] * transform[0][0]
            + point[1] * transform[1][0]
            + point[2] * transform[2][0]
            + transform[3][0],
        point[0] * transform[0][1]
            + point[1] * transform[1][1]
            + point[2] * transform[2][1]
            + transform[3][1],
        point[0] * transform[0][2]
            + point[1] * transform[1][2]
            + point[2] * transform[2][2]
            + transform[3][2],
    )
}

/// Change the independent open/closed state of one Door component instance.
/// Geometry and paint remain shared through the source asset.
pub fn set_block_prop_door_open(instance: &mut BlockPropInstance, component_id: Uuid, open: bool) {
    instance
        .runtime_state
        .set(&door_runtime_key(component_id), Value::Bool(open));
}

/// Return the independent open/closed state of one linked Door instance.
pub fn block_prop_door_is_open(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    component_id: Uuid,
) -> Option<bool> {
    let component = asset
        .components
        .iter()
        .find(|component| component.id == component_id && component.kind == "Door")?;
    let part_id = component.properties.get_id("part_id")?;
    asset.find_part(part_id)?;
    Some(
        instance
            .runtime_state
            .get_bool(&door_runtime_key(component_id))
            .or_else(|| instance.runtime_state.get_bool("open"))
            .unwrap_or_else(|| asset.default_state.get_bool_default("open", false)),
    )
}

fn door_part_motion(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    part: &BlockPropPart,
) -> BlockPropTransform {
    let Some(component) = asset.components.iter().find(|component| {
        component.kind == "Door" && component.properties.get_id("part_id") == Some(part.id)
    }) else {
        return identity_block_prop_transform();
    };
    let open = instance
        .runtime_state
        .get_bool(&door_runtime_key(component.id))
        .or_else(|| instance.runtime_state.get_bool("open"))
        .unwrap_or_else(|| asset.default_state.get_bool_default("open", false));
    let open_amount = instance
        .runtime_state
        .get_float("open_amount")
        .unwrap_or(if open { 1.0 } else { 0.0 })
        .clamp(0.0, 1.0);
    if open_amount <= f32::EPSILON {
        return identity_block_prop_transform();
    }

    let pivot = part.pivot;
    let to_origin = translation_block_prop_transform(-pivot[0], -pivot[1], -pivot[2]);
    let rotation = rotation_y_block_prop_transform(
        component
            .properties
            .get_float_default("angle_degrees", 90.0)
            * open_amount,
    );
    let from_origin = translation_block_prop_transform(pivot[0], pivot[1], pivot[2]);
    multiply_block_prop_transforms(
        multiply_block_prop_transforms(to_origin, rotation),
        from_origin,
    )
}

fn derived_instance_object_id(instance_id: Uuid, part_id: Uuid, object_id: Uuid) -> Uuid {
    let mut value = instance_id.as_u128()
        ^ part_id.as_u128().rotate_left(41)
        ^ object_id.as_u128().rotate_left(83);
    value ^= value >> 47;
    value = value.wrapping_mul(0x9E37_79B9_7F4A_7C15_6A09_E667_F3BC_C909);
    value ^= value >> 53;
    Uuid::from_u128(value)
}

fn placeholder_object(instance: &BlockPropInstance) -> GeometryObject {
    let placeholder_source_id = Uuid::from_u128(0xB10C_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF);
    let mut object = GeometryObject::box_from_bounds(
        format!("Missing Block / Prop {}", instance.asset_id),
        Vec3::new(-0.5, 0.0, -0.5),
        Vec3::new(0.5, 1.0, 0.5),
    );
    object.id = derived_instance_object_id(instance.id, instance.asset_id, placeholder_source_id);
    object.kind = GeometryObjectKind::Generated;
    object.solid = false;
    object.transform = instance.world_transform;
    object.tags.push("block_prop_placeholder".to_string());
    object
        .properties
        .set("block_prop_missing", Value::Bool(true));
    object
        .properties
        .set("block_prop_asset_id", Value::Id(instance.asset_id));
    object
        .properties
        .set("block_prop_instance_id", Value::Id(instance.id));
    object
}

fn default_rotation_step_degrees() -> f32 {
    90.0
}

fn default_footprint() -> [u32; 3] {
    [1, 1, 1]
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropAsset {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub alias: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub origin: [f32; 3],
    #[serde(default)]
    pub parts: Vec<BlockPropPart>,
    #[serde(default)]
    pub support_surfaces: Vec<BlockPropSupportSurface>,
    #[serde(default)]
    pub interaction_targets: Vec<BlockPropInteractionTarget>,
    #[serde(default)]
    pub components: Vec<BlockPropComponent>,
    #[serde(default)]
    pub default_state: ValueContainer,
    #[serde(default)]
    pub placement: BlockPropPlacementProfile,
}

impl BlockPropAsset {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            alias: String::new(),
            category: String::new(),
            tags: Vec::new(),
            origin: [0.0; 3],
            parts: Vec::new(),
            support_surfaces: Vec::new(),
            interaction_targets: Vec::new(),
            components: Vec::new(),
            default_state: ValueContainer::default(),
            placement: BlockPropPlacementProfile::default(),
        }
    }

    pub fn new_authored(name: impl Into<String>, geometry_objects: Vec<GeometryObject>) -> Self {
        let mut asset = Self::new(name);
        asset
            .parts
            .push(BlockPropPart::new_authored("Geometry", geometry_objects));
        asset
    }

    pub fn find_part(&self, part_id: Uuid) -> Option<&BlockPropPart> {
        self.parts.iter().find(|part| part.id == part_id)
    }

    pub fn find_support_surface(&self, surface_id: Uuid) -> Option<&BlockPropSupportSurface> {
        self.support_surfaces
            .iter()
            .find(|surface| surface.id == surface_id)
    }

    pub fn find_interaction_target(&self, target_id: Uuid) -> Option<&BlockPropInteractionTarget> {
        self.interaction_targets
            .iter()
            .find(|target| target.id == target_id)
    }
}

impl Default for BlockPropAsset {
    fn default() -> Self {
        Self::new("Untitled Block / Prop")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropPart {
    pub id: Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_part_id: Option<Uuid>,
    #[serde(default = "identity_block_prop_transform")]
    pub local_transform: BlockPropTransform,
    #[serde(default)]
    pub pivot: [f32; 3],
    #[serde(default)]
    pub attachments: Vec<BlockPropAttachment>,
    pub geometry_source: BlockPropGeometrySource,
}

impl BlockPropPart {
    pub fn new_authored(name: impl Into<String>, geometry_objects: Vec<GeometryObject>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            parent_part_id: None,
            local_transform: identity_block_prop_transform(),
            pivot: [0.0; 3],
            attachments: Vec::new(),
            geometry_source: BlockPropGeometrySource::Authored { geometry_objects },
        }
    }
}

impl Default for BlockPropPart {
    fn default() -> Self {
        Self::new_authored("Part", Vec::new())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum BlockPropGeometrySource {
    Authored {
        #[serde(default)]
        geometry_objects: Vec<GeometryObject>,
    },
    Recipe {
        recipe_id: Uuid,
        #[serde(default)]
        parameters: ValueContainer,
        #[serde(default)]
        seed: u64,
        #[serde(default)]
        generated_cache: Vec<GeometryObject>,
        #[serde(default)]
        source_signature: String,
    },
}

impl Default for BlockPropGeometrySource {
    fn default() -> Self {
        Self::Authored {
            geometry_objects: Vec::new(),
        }
    }
}

impl BlockPropGeometrySource {
    pub fn geometry_objects(&self) -> &[GeometryObject] {
        match self {
            Self::Authored { geometry_objects } => geometry_objects,
            Self::Recipe {
                generated_cache, ..
            } => generated_cache,
        }
    }

    pub fn is_generated(&self) -> bool {
        matches!(self, Self::Recipe { .. })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropAttachment {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub position: [f32; 3],
    #[serde(default = "default_attachment_direction")]
    pub direction: [f32; 3],
}

fn default_attachment_direction() -> [f32; 3] {
    [0.0, 0.0, 1.0]
}

impl Default for BlockPropAttachment {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Attachment".to_string(),
            position: [0.0; 3],
            direction: default_attachment_direction(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropPlacementProfile {
    #[serde(default = "default_footprint")]
    pub footprint: [u32; 3],
    #[serde(default = "default_rotation_step_degrees")]
    pub rotation_step_degrees: f32,
    #[serde(default)]
    pub snap_to_grid: bool,
    #[serde(default)]
    pub snap_to_surfaces: bool,
    #[serde(default)]
    pub supports_line_stroke: bool,
    #[serde(default)]
    pub supports_rectangle_stroke: bool,
    #[serde(default)]
    pub parameters: ValueContainer,
}

impl Default for BlockPropPlacementProfile {
    fn default() -> Self {
        Self {
            footprint: default_footprint(),
            rotation_step_degrees: default_rotation_step_degrees(),
            snap_to_grid: true,
            snap_to_surfaces: true,
            supports_line_stroke: false,
            supports_rectangle_stroke: false,
            parameters: ValueContainer::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropComponent {
    pub id: Uuid,
    pub kind: String,
    #[serde(default)]
    pub properties: ValueContainer,
}

impl BlockPropComponent {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: kind.into(),
            properties: ValueContainer::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockPropFaceRef {
    pub object_id: Uuid,
    pub face_id: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum BlockPropSemanticShape {
    Faces(Vec<BlockPropFaceRef>),
    Plane {
        origin: [f32; 3],
        axis_u: [f32; 3],
        axis_v: [f32; 3],
        size: [f32; 2],
    },
    Box {
        min: [f32; 3],
        max: [f32; 3],
    },
    NamedOutput(String),
}

impl Default for BlockPropSemanticShape {
    fn default() -> Self {
        Self::Plane {
            origin: [0.0; 3],
            axis_u: [1.0, 0.0, 0.0],
            axis_v: [0.0, 0.0, 1.0],
            size: [1.0, 1.0],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum BlockPropOccupancyPolicy {
    #[default]
    RejectOverlap,
    AllowOverlap,
    SingleOccupant,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropSupportSurface {
    pub id: Uuid,
    pub name: String,
    pub part_id: Uuid,
    #[serde(default)]
    pub shape: BlockPropSemanticShape,
    #[serde(default)]
    pub snap_spacing: f32,
    #[serde(default)]
    pub allowed_item_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u32>,
    #[serde(default)]
    pub occupancy_policy: BlockPropOccupancyPolicy,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropInteractionTarget {
    pub id: Uuid,
    pub name: String,
    pub part_id: Uuid,
    #[serde(default)]
    pub shape: BlockPropSemanticShape,
    #[serde(default)]
    pub interaction_anchor: [f32; 3],
    #[serde(default = "default_attachment_direction")]
    pub facing_direction: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropInstance {
    pub id: Uuid,
    pub asset_id: Uuid,
    #[serde(default = "identity_block_prop_transform")]
    pub world_transform: BlockPropTransform,
    #[serde(default)]
    pub parameter_overrides: ValueContainer,
    #[serde(default)]
    pub runtime_state: ValueContainer,
    #[serde(default)]
    pub overrides: ValueContainer,
}

impl BlockPropInstance {
    pub fn new(asset_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            asset_id,
            world_transform: identity_block_prop_transform(),
            parameter_overrides: ValueContainer::default(),
            runtime_state: ValueContainer::default(),
            overrides: ValueContainer::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum BlockPropOccupant {
    Item(u32),
    Entity(u32),
    PropInstance(Uuid),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropSurfacePlacement {
    pub id: Uuid,
    pub prop_instance_id: Uuid,
    pub surface_id: Uuid,
    pub occupant: BlockPropOccupant,
    #[serde(default = "identity_block_prop_transform")]
    pub local_transform: BlockPropTransform,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockPropAssetLayer {
    Project,
    Ruleset,
    Bundled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockPropGeometryDiagnosticKind {
    MissingAsset,
    MissingParentPart,
    CyclicPartHierarchy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockPropGeometryDiagnostic {
    pub kind: BlockPropGeometryDiagnosticKind,
    pub instance_id: Uuid,
    pub asset_id: Uuid,
    pub part_id: Option<Uuid>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockPropGeometryResolution {
    pub geometry_objects: Vec<GeometryObject>,
    pub diagnostics: Vec<BlockPropGeometryDiagnostic>,
}

/// Stable authoring identity resolved from one rendered linked-instance hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockPropInteractionHit {
    pub instance_id: Uuid,
    pub asset_id: Uuid,
    pub part_id: Uuid,
    pub target_id: Uuid,
    pub component_id: Uuid,
}

fn resolved_part_transform(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    part: &BlockPropPart,
    diagnostics: &mut Vec<BlockPropGeometryDiagnostic>,
) -> BlockPropTransform {
    let mut transform = identity_block_prop_transform();
    let mut current = Some(part);
    let mut visited = HashSet::new();

    while let Some(candidate) = current {
        if !visited.insert(candidate.id) {
            diagnostics.push(BlockPropGeometryDiagnostic {
                kind: BlockPropGeometryDiagnosticKind::CyclicPartHierarchy,
                instance_id: instance.id,
                asset_id: asset.id,
                part_id: Some(candidate.id),
                message: format!(
                    "Block / Prop '{}' contains a cyclic part hierarchy at '{}'",
                    asset.name, candidate.name
                ),
            });
            break;
        }

        transform = multiply_block_prop_transforms(transform, candidate.local_transform);
        transform =
            multiply_block_prop_transforms(transform, door_part_motion(asset, instance, candidate));
        current = match candidate.parent_part_id {
            Some(parent_id) => match asset.find_part(parent_id) {
                Some(parent) => Some(parent),
                None => {
                    diagnostics.push(BlockPropGeometryDiagnostic {
                        kind: BlockPropGeometryDiagnosticKind::MissingParentPart,
                        instance_id: instance.id,
                        asset_id: asset.id,
                        part_id: Some(candidate.id),
                        message: format!(
                            "Block / Prop '{}' part '{}' references missing parent {}",
                            asset.name, candidate.name, parent_id
                        ),
                    });
                    None
                }
            },
            None => None,
        };
    }

    transform
}

/// Resolve a rendered object/face pick back to its linked Prefab interaction target.
/// Face targets require an exact persistent source-face match; other semantic target
/// shapes match their owning part.
pub fn resolve_block_prop_interaction_hit(
    instances: &[BlockPropInstance],
    assets: &IndexMap<Uuid, BlockPropAsset>,
    rendered_object_id: Uuid,
    paint_surface_id: Option<[u32; 4]>,
) -> Option<BlockPropInteractionHit> {
    for instance in instances {
        let Some(asset) = assets.get(&instance.asset_id) else {
            continue;
        };
        for part in &asset.parts {
            for source_object in part.geometry_source.geometry_objects() {
                if derived_instance_object_id(instance.id, part.id, source_object.id)
                    != rendered_object_id
                {
                    continue;
                }

                for target in asset
                    .interaction_targets
                    .iter()
                    .filter(|target| target.part_id == part.id)
                {
                    let Some(component_id) = target.component_id else {
                        continue;
                    };
                    let component_matches = asset.components.iter().any(|component| {
                        component.id == component_id
                            && component.properties.get_id("part_id") == Some(part.id)
                    });
                    if !component_matches {
                        continue;
                    }
                    let shape_matches = match &target.shape {
                        BlockPropSemanticShape::Faces(faces) => {
                            let Some(picked_surface_id) = paint_surface_id else {
                                continue;
                            };
                            faces.iter().any(|face_ref| {
                                if face_ref.object_id != source_object.id {
                                    return false;
                                }
                                source_object
                                    .faces
                                    .iter()
                                    .find(|face| face.id == face_ref.face_id)
                                    .map(|face| {
                                        block_prop_paint_surface_id(
                                            source_object.id,
                                            crate::geometry_face_effective_paint_surface_id(face),
                                        ) == picked_surface_id
                                    })
                                    .unwrap_or(false)
                            })
                        }
                        BlockPropSemanticShape::Plane { .. }
                        | BlockPropSemanticShape::Box { .. }
                        | BlockPropSemanticShape::NamedOutput(_) => true,
                    };
                    if shape_matches {
                        return Some(BlockPropInteractionHit {
                            instance_id: instance.id,
                            asset_id: asset.id,
                            part_id: part.id,
                            target_id: target.id,
                            component_id,
                        });
                    }
                }
            }
        }
    }
    None
}

/// Context-sensitive default verb for a target on one linked instance.
pub fn block_prop_interaction_verb(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    target_id: Uuid,
) -> Option<&'static str> {
    let target = asset.find_interaction_target(target_id)?;
    let component_id = target.component_id?;
    block_prop_door_is_open(asset, instance, component_id)
        .map(|open| if open { "close" } else { "open" })
}

/// World-space interaction anchor used for authoritative range validation.
pub fn block_prop_interaction_world_anchor(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    target_id: Uuid,
) -> Option<Vec3<f32>> {
    let target = asset.find_interaction_target(target_id)?;
    let part = asset.find_part(target.part_id)?;
    let mut diagnostics = Vec::new();
    let part_transform = resolved_part_transform(asset, instance, part, &mut diagnostics);
    if !diagnostics.is_empty() {
        return None;
    }
    let world_transform = multiply_block_prop_transforms(part_transform, instance.world_transform);
    Some(transform_block_prop_point(
        world_transform,
        target.interaction_anchor,
    ))
}

/// Resolve linked instances into ordinary Geometry Objects for render and
/// collision consumers. The stored map remains linked and is never mutated.
pub fn resolve_block_prop_geometry(
    instances: &[BlockPropInstance],
    assets: &IndexMap<Uuid, BlockPropAsset>,
) -> BlockPropGeometryResolution {
    let mut resolution = BlockPropGeometryResolution::default();

    for instance in instances {
        let Some(asset) = assets.get(&instance.asset_id) else {
            resolution
                .geometry_objects
                .push(placeholder_object(instance));
            resolution.diagnostics.push(BlockPropGeometryDiagnostic {
                kind: BlockPropGeometryDiagnosticKind::MissingAsset,
                instance_id: instance.id,
                asset_id: instance.asset_id,
                part_id: None,
                message: format!(
                    "Block / Prop instance {} references missing asset {}",
                    instance.id, instance.asset_id
                ),
            });
            continue;
        };

        for part in &asset.parts {
            let part_transform =
                resolved_part_transform(asset, instance, part, &mut resolution.diagnostics);
            let part_instance_transform =
                multiply_block_prop_transforms(part_transform, instance.world_transform);

            for source_object in part.geometry_source.geometry_objects() {
                let mut object = source_object.clone();
                object.id = derived_instance_object_id(instance.id, part.id, source_object.id);
                object.kind = GeometryObjectKind::Prop;
                object.transform = multiply_block_prop_transforms(
                    source_object.transform,
                    part_instance_transform,
                );
                object.group = format!("block_prop:{}", instance.id);
                object
                    .properties
                    .set("block_prop_asset_id", Value::Id(asset.id));
                object
                    .properties
                    .set("block_prop_instance_id", Value::Id(instance.id));
                object
                    .properties
                    .set("block_prop_part_id", Value::Id(part.id));
                object
                    .properties
                    .set("block_prop_source_object_id", Value::Id(source_object.id));
                resolution.geometry_objects.push(object);
            }
        }
    }

    resolution
}

/// Resolve one asset for an editor-only state preview while preserving source
/// object IDs. The result must never be written back as authored geometry.
pub fn resolve_block_prop_preview_geometry(
    asset: &BlockPropAsset,
    runtime_state: ValueContainer,
) -> BlockPropGeometryResolution {
    let instance = BlockPropInstance {
        id: Uuid::nil(),
        asset_id: asset.id,
        world_transform: identity_block_prop_transform(),
        parameter_overrides: ValueContainer::default(),
        runtime_state,
        overrides: ValueContainer::default(),
    };
    let mut resolution = BlockPropGeometryResolution::default();
    for part in &asset.parts {
        let part_transform =
            resolved_part_transform(asset, &instance, part, &mut resolution.diagnostics);
        for source_object in part.geometry_source.geometry_objects() {
            let mut object = source_object.clone();
            object.transform =
                multiply_block_prop_transforms(source_object.transform, part_transform);
            resolution.geometry_objects.push(object);
        }
    }
    resolution
}

/// Resolve a block/prop asset with project overrides taking precedence over
/// ruleset assets, followed by bundled fallback content.
pub fn resolve_block_prop_asset<'a>(
    asset_id: Uuid,
    project: &'a IndexMap<Uuid, BlockPropAsset>,
    ruleset: &'a IndexMap<Uuid, BlockPropAsset>,
    bundled: &'a IndexMap<Uuid, BlockPropAsset>,
) -> Option<(BlockPropAssetLayer, &'a BlockPropAsset)> {
    project
        .get(&asset_id)
        .map(|asset| (BlockPropAssetLayer::Project, asset))
        .or_else(|| {
            ruleset
                .get(&asset_id)
                .map(|asset| (BlockPropAssetLayer::Ruleset, asset))
        })
        .or_else(|| {
            bundled
                .get(&asset_id)
                .map(|asset| (BlockPropAssetLayer::Bundled, asset))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vek::Vec3;

    #[test]
    fn authored_asset_keeps_geometry_and_semantic_face_references() {
        let object = GeometryObject::box_("Table Top", Vec3::zero(), Vec3::one());
        let object_id = object.id;
        let face_id = object.faces[0].id;
        let mut asset = BlockPropAsset::new_authored("Table", vec![object]);
        let part_id = asset.parts[0].id;
        let surface_id = Uuid::new_v4();
        asset.support_surfaces.push(BlockPropSupportSurface {
            id: surface_id,
            name: "Table Top".to_string(),
            part_id,
            shape: BlockPropSemanticShape::Faces(vec![BlockPropFaceRef { object_id, face_id }]),
            snap_spacing: 0.1,
            allowed_item_tags: vec!["tabletop".to_string()],
            capacity: Some(8),
            occupancy_policy: BlockPropOccupancyPolicy::RejectOverlap,
        });

        let serialized = serde_json::to_string(&asset).expect("serialize block/prop asset");
        let restored: BlockPropAsset =
            serde_json::from_str(&serialized).expect("deserialize block/prop asset");

        assert_eq!(restored, asset);
        assert!(restored.find_part(part_id).is_some());
        assert!(restored.find_support_surface(surface_id).is_some());
    }

    #[test]
    fn resolver_prefers_project_then_ruleset_then_bundled() {
        let asset_id = Uuid::new_v4();
        let mut project = IndexMap::new();
        let mut ruleset = IndexMap::new();
        let mut bundled = IndexMap::new();

        let mut bundled_asset = BlockPropAsset::new("Bundled");
        bundled_asset.id = asset_id;
        bundled.insert(asset_id, bundled_asset);
        let mut ruleset_asset = BlockPropAsset::new("Ruleset");
        ruleset_asset.id = asset_id;
        ruleset.insert(asset_id, ruleset_asset);
        let mut project_asset = BlockPropAsset::new("Project");
        project_asset.id = asset_id;
        project.insert(asset_id, project_asset);

        let (layer, asset) =
            resolve_block_prop_asset(asset_id, &project, &ruleset, &bundled).unwrap();
        assert_eq!(layer, BlockPropAssetLayer::Project);
        assert_eq!(asset.name, "Project");

        project.clear();
        let (layer, asset) =
            resolve_block_prop_asset(asset_id, &project, &ruleset, &bundled).unwrap();
        assert_eq!(layer, BlockPropAssetLayer::Ruleset);
        assert_eq!(asset.name, "Ruleset");

        ruleset.clear();
        let (layer, asset) =
            resolve_block_prop_asset(asset_id, &project, &ruleset, &bundled).unwrap();
        assert_eq!(layer, BlockPropAssetLayer::Bundled);
        assert_eq!(asset.name, "Bundled");
    }

    #[test]
    fn linked_instance_resolves_authored_geometry_with_composed_transforms() {
        let mut object = GeometryObject::box_("Top", Vec3::zero(), Vec3::one());
        object.transform[3][0] = 1.0;
        let source_object_id = object.id;
        let mut asset = BlockPropAsset::new_authored("Table", vec![object]);
        asset.parts[0].local_transform[3][1] = 2.0;
        let mut root = BlockPropPart::new_authored("Root", Vec::new());
        root.local_transform[3][0] = 4.0;
        asset.parts[0].parent_part_id = Some(root.id);
        asset.parts.push(root);

        let mut instance = BlockPropInstance::new(asset.id);
        instance.world_transform[3][2] = 3.0;
        let instance_id = instance.id;
        let part_id = asset.parts[0].id;
        let asset_id = asset.id;
        let assets = IndexMap::from([(asset_id, asset)]);

        let resolution = resolve_block_prop_geometry(&[instance], &assets);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(resolution.geometry_objects.len(), 1);
        let resolved = &resolution.geometry_objects[0];
        assert_ne!(resolved.id, source_object_id);
        assert_eq!(resolved.kind, GeometryObjectKind::Prop);
        assert_eq!(resolved.transform[3], [5.0, 2.0, 3.0, 1.0]);
        assert_eq!(
            resolved.properties.get_id("block_prop_instance_id"),
            Some(instance_id)
        );
        assert_eq!(
            resolved.properties.get_id("block_prop_part_id"),
            Some(part_id)
        );
    }

    #[test]
    fn door_instances_keep_independent_runtime_state() {
        let mut leaf_object =
            GeometryObject::box_("Leaf", Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 2.0, 0.2));
        leaf_object.transform[3][0] = 1.0;
        let mut asset = BlockPropAsset::new_authored("Door", vec![leaf_object]);
        let leaf_id = asset.parts[0].id;
        asset.parts[0].pivot = [0.0, 0.0, 0.0];
        let mut door = BlockPropComponent::new("Door");
        door.properties.set("part_id", Value::Id(leaf_id));
        door.properties.set("angle_degrees", Value::Float(90.0));
        let door_id = door.id;
        asset.components.push(door);

        let closed = BlockPropInstance::new(asset.id);
        let mut open = BlockPropInstance::new(asset.id);
        open.world_transform[3][0] = 10.0;
        set_block_prop_door_open(&mut open, door_id, true);
        let assets = IndexMap::from([(asset.id, asset)]);

        let resolution = resolve_block_prop_geometry(&[closed, open], &assets);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(resolution.geometry_objects.len(), 2);
        assert_eq!(resolution.geometry_objects[0].transform[3][0], 1.0);
        assert!((resolution.geometry_objects[1].transform[3][0] - 10.0).abs() < 0.001);
        assert!((resolution.geometry_objects[1].transform[3][2] + 1.0).abs() < 0.001);
    }

    #[test]
    fn rendered_face_pick_resolves_stable_door_target_and_context_verb() {
        let object = GeometryObject::box_("Leaf", Vec3::zero(), Vec3::one());
        let source_object_id = object.id;
        let source_face_id = object.faces[0].id;
        let effective_face_id = crate::geometry_face_effective_paint_surface_id(&object.faces[0]);
        let mut asset = BlockPropAsset::new_authored("Door", vec![object]);
        let part_id = asset.parts[0].id;
        asset.parts[0].pivot = [0.0, 1.0, 0.0];
        let mut component = BlockPropComponent::new("Door");
        component.properties.set("part_id", Value::Id(part_id));
        let component_id = component.id;
        asset.components.push(component);
        let target_id = Uuid::new_v4();
        asset.interaction_targets.push(BlockPropInteractionTarget {
            id: target_id,
            name: "Handle".to_string(),
            part_id,
            shape: BlockPropSemanticShape::Faces(vec![BlockPropFaceRef {
                object_id: source_object_id,
                face_id: source_face_id,
            }]),
            interaction_anchor: [0.0, 1.0, 0.0],
            facing_direction: [0.0, 0.0, 1.0],
            component_id: Some(component_id),
        });
        let mut instance = BlockPropInstance::new(asset.id);
        instance.world_transform[3][0] = 4.0;
        let rendered_object_id = derived_instance_object_id(instance.id, part_id, source_object_id);
        let paint_surface_id = block_prop_paint_surface_id(source_object_id, effective_face_id);
        let assets = IndexMap::from([(asset.id, asset.clone())]);

        let hit = resolve_block_prop_interaction_hit(
            std::slice::from_ref(&instance),
            &assets,
            rendered_object_id,
            Some(paint_surface_id),
        )
        .expect("face hit should resolve");
        assert_eq!(hit.instance_id, instance.id);
        assert_eq!(hit.target_id, target_id);
        assert_eq!(
            block_prop_interaction_verb(&asset, &instance, target_id),
            Some("open")
        );
        assert_eq!(
            block_prop_interaction_world_anchor(&asset, &instance, target_id),
            Some(Vec3::new(4.0, 1.0, 0.0))
        );

        set_block_prop_door_open(&mut instance, component_id, true);
        assert_eq!(
            block_prop_interaction_verb(&asset, &instance, target_id),
            Some("close")
        );
        assert!(
            resolve_block_prop_interaction_hit(
                &[instance],
                &assets,
                rendered_object_id,
                Some([0; 4]),
            )
            .is_none()
        );
    }

    #[test]
    fn missing_asset_resolves_to_visible_non_solid_placeholder() {
        let missing_asset_id = Uuid::new_v4();
        let instance = BlockPropInstance::new(missing_asset_id);

        let resolution = resolve_block_prop_geometry(&[instance], &IndexMap::new());
        assert_eq!(resolution.geometry_objects.len(), 1);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(
            resolution.diagnostics[0].kind,
            BlockPropGeometryDiagnosticKind::MissingAsset
        );
        let placeholder = &resolution.geometry_objects[0];
        assert!(!placeholder.solid);
        assert!(
            placeholder
                .tags
                .iter()
                .any(|tag| tag == "block_prop_placeholder")
        );
        assert!(
            placeholder
                .properties
                .get_bool_default("block_prop_missing", false)
        );
    }
}
