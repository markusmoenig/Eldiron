use crate::{GeometryObject, GeometryObjectKind, ParticleEmitterDef, Value, ValueContainer};
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

/// Whether a Door component controls this authored part. Legacy doors only
/// have `part_id`; split doors add `secondary_part_id` while keeping the same
/// component and runtime state for both leaves.
pub fn block_prop_door_controls_part(component: &BlockPropComponent, part_id: Uuid) -> bool {
    component.kind == "Door"
        && (component.properties.get_id("part_id") == Some(part_id)
            || component.properties.get_id("secondary_part_id") == Some(part_id))
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

fn transform_block_prop_direction(transform: BlockPropTransform, direction: [f32; 3]) -> Vec3<f32> {
    Vec3::new(
        direction[0] * transform[0][0]
            + direction[1] * transform[1][0]
            + direction[2] * transform[2][0],
        direction[0] * transform[0][1]
            + direction[1] * transform[1][1]
            + direction[2] * transform[2][1],
        direction[0] * transform[0][2]
            + direction[1] * transform[1][2]
            + direction[2] * transform[2][2],
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
    let primary_part_id = component.properties.get_id("part_id")?;
    asset.find_part(primary_part_id)?;
    if let Some(secondary_part_id) = component.properties.get_id("secondary_part_id") {
        asset.find_part(secondary_part_id)?;
    }
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
    let Some(component) = asset
        .components
        .iter()
        .find(|component| block_prop_door_controls_part(component, part.id))
    else {
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

    let secondary_part_id = component.properties.get_id("secondary_part_id");
    let is_split = secondary_part_id.is_some();
    let leaf_sign = if is_split {
        if secondary_part_id == Some(part.id) {
            1.0
        } else {
            -1.0
        }
    } else {
        1.0
    };
    let motion = component.properties.get_str("motion").unwrap_or("Swing");
    if motion.eq_ignore_ascii_case("Slide") {
        let axis = component
            .properties
            .get_vec3("slide_axis")
            .unwrap_or([1.0, 0.0, 0.0]);
        let magnitude = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        if magnitude <= f32::EPSILON {
            return identity_block_prop_transform();
        }
        let distance = component
            .properties
            .get_float_default("slide_distance", 1.0)
            * open_amount
            * leaf_sign;
        return translation_block_prop_transform(
            axis[0] / magnitude * distance,
            axis[1] / magnitude * distance,
            axis[2] / magnitude * distance,
        );
    }

    let pivot = part.pivot;
    let to_origin = translation_block_prop_transform(-pivot[0], -pivot[1], -pivot[2]);
    let angle = component
        .properties
        .get_float_default("angle_degrees", 90.0)
        * open_amount
        * leaf_sign;
    let rotation = rotation_y_block_prop_transform(angle);
    let from_origin = translation_block_prop_transform(pivot[0], pivot[1], pivot[2]);
    multiply_block_prop_transforms(
        multiply_block_prop_transforms(to_origin, rotation),
        from_origin,
    )
}

pub fn block_prop_instance_object_id(instance_id: Uuid, part_id: Uuid, object_id: Uuid) -> Uuid {
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
    object.id =
        block_prop_instance_object_id(instance.id, instance.asset_id, placeholder_source_id);
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
    /// TOML-authored presentation and interaction metadata shared by every
    /// linked instance of this Prefab.
    #[serde(default)]
    pub authoring: String,
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
    /// Visual effects authored as part of the Prefab. These are independent of
    /// Tiles and procedural recipes and follow their owning part hierarchy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub particle_effects: Vec<BlockPropParticleEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub light_effects: Vec<BlockPropLightEffect>,
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
            authoring: String::new(),
            alias: String::new(),
            category: String::new(),
            tags: Vec::new(),
            origin: [0.0; 3],
            parts: Vec::new(),
            support_surfaces: Vec::new(),
            interaction_targets: Vec::new(),
            components: Vec::new(),
            particle_effects: Vec::new(),
            light_effects: Vec::new(),
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
    /// Local up vector. Together with `direction` this preserves roll and gives
    /// surface-mounted effects a complete authoring frame.
    #[serde(default = "default_attachment_up")]
    pub up: [f32; 3],
}

fn default_attachment_direction() -> [f32; 3] {
    [0.0, 0.0, 1.0]
}

fn default_attachment_up() -> [f32; 3] {
    [0.0, 1.0, 0.0]
}

impl Default for BlockPropAttachment {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Attachment".to_string(),
            position: [0.0; 3],
            direction: default_attachment_direction(),
            up: default_attachment_up(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropParticleEffect {
    pub id: Uuid,
    pub name: String,
    pub part_id: Uuid,
    pub attachment_id: Uuid,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub emitter: ParticleEmitterDef,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockPropLightEffect {
    pub id: Uuid,
    pub name: String,
    pub part_id: Uuid,
    pub attachment_id: Uuid,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_prefab_light_color")]
    pub color: [u8; 4],
    #[serde(default = "default_prefab_light_intensity")]
    pub intensity: f32,
    #[serde(default = "default_prefab_light_range")]
    pub range: f32,
    #[serde(default)]
    pub flicker: f32,
    #[serde(default)]
    pub lift: f32,
}

fn default_true() -> bool {
    true
}

fn default_prefab_light_color() -> [u8; 4] {
    [255, 160, 72, 255]
}

fn default_prefab_light_intensity() -> f32 {
    2.0
}

fn default_prefab_light_range() -> f32 {
    4.0
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedBlockPropParticleEffect {
    pub instance_id: Uuid,
    pub asset_id: Uuid,
    pub effect_id: Uuid,
    pub origin: Vec3<f32>,
    pub direction: Vec3<f32>,
    pub emitter: ParticleEmitterDef,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedBlockPropLightEffect {
    pub instance_id: Uuid,
    pub asset_id: Uuid,
    pub effect_id: Uuid,
    pub position: Vec3<f32>,
    pub color: [u8; 4],
    pub intensity: f32,
    pub range: f32,
    pub flicker: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockPropEffectResolution {
    pub particles: Vec<ResolvedBlockPropParticleEffect>,
    pub lights: Vec<ResolvedBlockPropLightEffect>,
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
    #[serde(default)]
    pub mode: BlockPropPlacementMode,
    /// Distance between the Prefab origin and the picked mounting surface.
    #[serde(default)]
    pub surface_offset: f32,
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
            mode: BlockPropPlacementMode::Ground,
            surface_offset: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlockPropPlacementMode {
    #[default]
    Ground,
    Wall,
    AnySurface,
    Free,
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
    /// Every rendered Geometry Object owned by the semantic overlay's part.
    Part,
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
    /// Optional semantic host. The cached world transform remains usable when
    /// the host disappears, while valid hosts can recompute it after edits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_attachment: Option<BlockPropHostAttachment>,
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
            host_attachment: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum BlockPropHostAttachment {
    WallSpan {
        assembly_id: Uuid,
        span_id: Uuid,
        /// Arc distance from the span's start node.
        along: f32,
        /// Height above the wall path.
        height: f32,
        /// Canonical perpendicular side: -1 or +1.
        side: f32,
        /// Additional distance away from the wall center plane.
        offset: f32,
        /// Rotation around the mounting normal in quarter turns.
        #[serde(default)]
        rotation_quarters: i32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum BlockPropOccupant {
    Item(u32),
    /// Stable Creator-side item instance identity. Runtime-spawned items keep
    /// using `Item(u32)`, while authored region items must not rely on their
    /// transient numeric runtime ID.
    ItemInstance(Uuid),
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
    pub target_id: Option<Uuid>,
    pub component_id: Option<Uuid>,
}

/// Stable support-surface identity resolved from one rendered linked-instance
/// face hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockPropSupportSurfaceHit {
    pub instance_id: Uuid,
    pub asset_id: Uuid,
    pub part_id: Uuid,
    pub surface_id: Uuid,
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

/// Resolve a rendered object/face pick back to its linked Prefab. Every rendered
/// part resolves so general intents such as Look can target any Prefab. Component
/// targets remain optional metadata used by built-in behaviors such as Door.
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
                if block_prop_instance_object_id(instance.id, part.id, source_object.id)
                    != rendered_object_id
                {
                    continue;
                }

                let generic_hit = BlockPropInteractionHit {
                    instance_id: instance.id,
                    asset_id: asset.id,
                    part_id: part.id,
                    target_id: None,
                    component_id: None,
                };

                for target in asset
                    .interaction_targets
                    .iter()
                    .filter(|target| target.part_id == part.id)
                {
                    let Some(component_id) = target.component_id else {
                        continue;
                    };
                    let Some(component) = asset.components.iter().find(|component| {
                        component.id == component_id
                            && block_prop_door_controls_part(component, part.id)
                    }) else {
                        continue;
                    };
                    let shape_matches = component.kind == "Door"
                        || match &target.shape {
                            BlockPropSemanticShape::Part => true,
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
                                                crate::geometry_face_effective_paint_surface_id(
                                                    face,
                                                ),
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
                            target_id: Some(target.id),
                            component_id: Some(component_id),
                        });
                    }
                }
                return Some(generic_hit);
            }
        }
    }
    None
}

/// Resolve a rendered linked-instance face to an authored support surface.
/// Face identity remains stable across source edits and instance resolution.
pub fn resolve_block_prop_support_surface_hit(
    instances: &[BlockPropInstance],
    assets: &IndexMap<Uuid, BlockPropAsset>,
    rendered_object_id: Uuid,
    paint_surface_id: [u32; 4],
) -> Option<BlockPropSupportSurfaceHit> {
    for instance in instances {
        let Some(asset) = assets.get(&instance.asset_id) else {
            continue;
        };
        for part in &asset.parts {
            for source_object in part.geometry_source.geometry_objects() {
                if block_prop_instance_object_id(instance.id, part.id, source_object.id)
                    != rendered_object_id
                {
                    continue;
                }
                for surface in asset
                    .support_surfaces
                    .iter()
                    .filter(|surface| surface.part_id == part.id)
                {
                    let BlockPropSemanticShape::Faces(face_refs) = &surface.shape else {
                        continue;
                    };
                    let matches = face_refs.iter().any(|face_ref| {
                        face_ref.object_id == source_object.id
                            && source_object
                                .faces
                                .iter()
                                .find(|face| face.id == face_ref.face_id)
                                .is_some_and(|face| {
                                    block_prop_paint_surface_id(
                                        source_object.id,
                                        crate::geometry_face_effective_paint_surface_id(face),
                                    ) == paint_surface_id
                                })
                    });
                    if matches {
                        return Some(BlockPropSupportSurfaceHit {
                            instance_id: instance.id,
                            asset_id: asset.id,
                            part_id: part.id,
                            surface_id: surface.id,
                        });
                    }
                }
                return None;
            }
        }
    }
    None
}

fn support_surface_point_inside_face(points: &[Vec3<f32>], point: Vec3<f32>) -> bool {
    if points.len() < 3 {
        return false;
    }
    let origin = points[0];
    let Some(normal) = (1..points.len()).find_map(|second| {
        (second + 1..points.len()).find_map(|third| {
            (points[second] - origin)
                .cross(points[third] - origin)
                .try_normalized()
        })
    }) else {
        return false;
    };
    // Rendered tiled faces are nudged very slightly to avoid z-fighting.
    if (point - origin).dot(normal).abs() > 0.025 {
        return false;
    }

    let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
    let project = |point: Vec3<f32>| {
        if abs.x >= abs.y && abs.x >= abs.z {
            [point.y, point.z]
        } else if abs.y >= abs.z {
            [point.x, point.z]
        } else {
            [point.x, point.y]
        }
    };
    let point = project(point);
    let projected = points.iter().copied().map(project).collect::<Vec<_>>();

    let mut inside = false;
    for index in 0..projected.len() {
        let a = projected[index];
        let b = projected[(index + 1) % projected.len()];
        let edge = [b[0] - a[0], b[1] - a[1]];
        let to_point = [point[0] - a[0], point[1] - a[1]];
        let edge_length_squared = edge[0] * edge[0] + edge[1] * edge[1];
        if edge_length_squared > 1e-8 {
            let t = ((to_point[0] * edge[0] + to_point[1] * edge[1]) / edge_length_squared)
                .clamp(0.0, 1.0);
            let dx = point[0] - (a[0] + edge[0] * t);
            let dy = point[1] - (a[1] + edge[1] * t);
            if dx * dx + dy * dy <= 0.025 * 0.025 {
                return true;
            }
        }
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

/// Resolve a linked-instance support surface from the actual world-space hit.
/// This is the robust fallback for render paths that cannot provide the stable
/// face paint ID (for example, a face split into several tiled render polygons).
fn resolve_block_prop_support_surface_hit_at_point_filtered(
    instances: &[BlockPropInstance],
    assets: &IndexMap<Uuid, BlockPropAsset>,
    rendered_object_id: Option<Uuid>,
    world_point: Vec3<f32>,
) -> Option<BlockPropSupportSurfaceHit> {
    for instance in instances {
        let Some(asset) = assets.get(&instance.asset_id) else {
            continue;
        };
        for part in &asset.parts {
            for source_object in part.geometry_source.geometry_objects() {
                if rendered_object_id.is_some_and(|rendered_object_id| {
                    block_prop_instance_object_id(instance.id, part.id, source_object.id)
                        != rendered_object_id
                }) {
                    continue;
                }

                let mut diagnostics = Vec::new();
                let part_transform =
                    resolved_part_transform(asset, instance, part, &mut diagnostics);
                if !diagnostics.is_empty() {
                    return None;
                }
                let world_transform = multiply_block_prop_transforms(
                    source_object.transform,
                    multiply_block_prop_transforms(part_transform, instance.world_transform),
                );

                for surface in asset
                    .support_surfaces
                    .iter()
                    .filter(|surface| surface.part_id == part.id)
                {
                    let BlockPropSemanticShape::Faces(face_refs) = &surface.shape else {
                        continue;
                    };
                    let matches = face_refs.iter().any(|face_ref| {
                        if face_ref.object_id != source_object.id {
                            return false;
                        }
                        let Some(face) = source_object
                            .faces
                            .iter()
                            .find(|face| face.id == face_ref.face_id)
                        else {
                            return false;
                        };
                        let points = face
                            .indices
                            .iter()
                            .filter_map(|index| source_object.vertices.get(*index))
                            .map(|point| {
                                transform_block_prop_point(
                                    world_transform,
                                    [point.x, point.y, point.z],
                                )
                            })
                            .collect::<Vec<_>>();
                        support_surface_point_inside_face(&points, world_point)
                    });
                    if matches {
                        return Some(BlockPropSupportSurfaceHit {
                            instance_id: instance.id,
                            asset_id: asset.id,
                            part_id: part.id,
                            surface_id: surface.id,
                        });
                    }
                }
                if rendered_object_id.is_some() {
                    return None;
                }
            }
        }
    }
    None
}

pub fn resolve_block_prop_support_surface_hit_at_point(
    instances: &[BlockPropInstance],
    assets: &IndexMap<Uuid, BlockPropAsset>,
    rendered_object_id: Uuid,
    world_point: Vec3<f32>,
) -> Option<BlockPropSupportSurfaceHit> {
    resolve_block_prop_support_surface_hit_at_point_filtered(
        instances,
        assets,
        Some(rendered_object_id),
        world_point,
    )
}

/// Resolve a support surface solely from the world-space hit. This avoids
/// depending on the renderer preserving the resolved Prefab object's GeoId.
pub fn resolve_block_prop_support_surface_hit_at_world_point(
    instances: &[BlockPropInstance],
    assets: &IndexMap<Uuid, BlockPropAsset>,
    world_point: Vec3<f32>,
) -> Option<BlockPropSupportSurfaceHit> {
    resolve_block_prop_support_surface_hit_at_point_filtered(instances, assets, None, world_point)
}

fn support_surface_part_frame(
    asset: &BlockPropAsset,
    surface: &BlockPropSupportSurface,
) -> Option<BlockPropTransform> {
    let (origin, axis_u, axis_v) = match &surface.shape {
        BlockPropSemanticShape::Faces(face_refs) => {
            let part = asset.find_part(surface.part_id)?;
            let mut frame = None;
            for face_ref in face_refs {
                let Some(object) = part
                    .geometry_source
                    .geometry_objects()
                    .iter()
                    .find(|object| object.id == face_ref.object_id)
                else {
                    continue;
                };
                let Some(face) = object.faces.iter().find(|face| face.id == face_ref.face_id)
                else {
                    continue;
                };
                let points = face
                    .indices
                    .iter()
                    .filter_map(|index| object.vertices.get(*index))
                    .map(|point| object.transform_point(*point))
                    .collect::<Vec<_>>();
                let Some(origin) = points.first().copied() else {
                    continue;
                };
                for second in 1..points.len() {
                    let Some(axis_u) = (points[second] - origin).try_normalized() else {
                        continue;
                    };
                    for third in second + 1..points.len() {
                        let Some(normal) = axis_u.cross(points[third] - origin).try_normalized()
                        else {
                            continue;
                        };
                        let axis_v = normal.cross(axis_u);
                        frame = Some((origin, axis_u, axis_v));
                        break;
                    }
                    if frame.is_some() {
                        break;
                    }
                }
                if frame.is_some() {
                    break;
                }
            }
            frame?
        }
        BlockPropSemanticShape::Plane {
            origin,
            axis_u,
            axis_v,
            ..
        } => {
            let origin = Vec3::from(*origin);
            let axis_u = Vec3::from(*axis_u).try_normalized()?;
            let normal = axis_u.cross(Vec3::from(*axis_v)).try_normalized()?;
            (origin, axis_u, normal.cross(axis_u))
        }
        _ => return None,
    };
    let normal = axis_u.cross(axis_v).try_normalized()?;
    let mut frame = identity_block_prop_transform();
    frame[0][0] = axis_u.x;
    frame[0][1] = axis_u.y;
    frame[0][2] = axis_u.z;
    frame[1][0] = normal.x;
    frame[1][1] = normal.y;
    frame[1][2] = normal.z;
    frame[2][0] = axis_v.x;
    frame[2][1] = axis_v.y;
    frame[2][2] = axis_v.z;
    frame[3][0] = origin.x;
    frame[3][1] = origin.y;
    frame[3][2] = origin.z;
    Some(frame)
}

/// Surface-local-to-world transform. Local X/Z lie on the support surface and
/// local Y follows its normal.
pub fn block_prop_support_surface_world_transform(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    surface_id: Uuid,
) -> Option<BlockPropTransform> {
    let surface = asset.find_support_surface(surface_id)?;
    let part = asset.find_part(surface.part_id)?;
    let surface_frame = support_surface_part_frame(asset, surface)?;
    let mut diagnostics = Vec::new();
    let part_transform = resolved_part_transform(asset, instance, part, &mut diagnostics);
    diagnostics.is_empty().then(|| {
        multiply_block_prop_transforms(
            multiply_block_prop_transforms(surface_frame, part_transform),
            instance.world_transform,
        )
    })
}

pub fn block_prop_support_surface_world_point(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    surface_id: Uuid,
    local_point: [f32; 3],
) -> Option<Vec3<f32>> {
    block_prop_support_surface_world_transform(asset, instance, surface_id)
        .map(|transform| transform_block_prop_point(transform, local_point))
}

/// Convert a world point to one support surface's local X/normal/Z basis.
pub fn block_prop_support_surface_local_point(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    surface_id: Uuid,
    world_point: Vec3<f32>,
) -> Option<Vec3<f32>> {
    let transform = block_prop_support_surface_world_transform(asset, instance, surface_id)?;
    let origin = Vec3::new(transform[3][0], transform[3][1], transform[3][2]);
    let x = Vec3::new(transform[0][0], transform[0][1], transform[0][2]);
    let y = Vec3::new(transform[1][0], transform[1][1], transform[1][2]);
    let z = Vec3::new(transform[2][0], transform[2][1], transform[2][2]);
    let determinant = x.dot(y.cross(z));
    if determinant.abs() <= 1e-7 {
        return None;
    }
    let delta = world_point - origin;
    Some(Vec3::new(
        delta.dot(y.cross(z)) / determinant,
        delta.dot(z.cross(x)) / determinant,
        delta.dot(x.cross(y)) / determinant,
    ))
}

pub fn block_prop_surface_placement_world_position(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    placement: &BlockPropSurfacePlacement,
) -> Option<Vec3<f32>> {
    block_prop_support_surface_world_point(
        asset,
        instance,
        placement.surface_id,
        [
            placement.local_transform[3][0],
            placement.local_transform[3][1],
            placement.local_transform[3][2],
        ],
    )
}

/// Reapply persistent support-surface relationships to item world positions.
/// This keeps authored and runtime items attached when a Prefab instance,
/// parent part, or source surface moves.
pub fn sync_block_prop_surface_item_positions(
    instances: &[BlockPropInstance],
    placements: &[BlockPropSurfacePlacement],
    items: &mut [crate::Item],
    assets: &IndexMap<Uuid, BlockPropAsset>,
) -> usize {
    let mut updated = 0;
    for placement in placements {
        let item = match &placement.occupant {
            BlockPropOccupant::Item(id) => items.iter_mut().find(|item| item.id == *id),
            BlockPropOccupant::ItemInstance(id) => {
                items.iter_mut().find(|item| item.creator_id == *id)
            }
            _ => None,
        };
        let Some(item) = item else {
            continue;
        };
        let Some(instance) = instances
            .iter()
            .find(|instance| instance.id == placement.prop_instance_id)
        else {
            continue;
        };
        let Some(asset) = assets.get(&instance.asset_id) else {
            continue;
        };
        let Some(position) =
            block_prop_surface_placement_world_position(asset, instance, placement)
        else {
            continue;
        };
        item.position = position;
        updated += 1;
    }
    updated
}

/// World-space anchor for whole-Prefab authoring interactions. The selected
/// part pivot provides a stable, server-verifiable point even when no component
/// interaction target exists.
pub fn block_prop_part_world_anchor(
    asset: &BlockPropAsset,
    instance: &BlockPropInstance,
    part_id: Uuid,
) -> Option<Vec3<f32>> {
    let part = asset.find_part(part_id)?;
    let mut diagnostics = Vec::new();
    let part_transform = resolved_part_transform(asset, instance, part, &mut diagnostics);
    if !diagnostics.is_empty() {
        return None;
    }
    let world_transform = multiply_block_prop_transforms(part_transform, instance.world_transform);
    Some(transform_block_prop_point(world_transform, part.pivot))
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
                object.id = block_prop_instance_object_id(instance.id, part.id, source_object.id);
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

/// Resolve all Prefab-owned effects into world space. Effects use the same
/// part hierarchy and runtime motion as geometry, so an attachment on a moving
/// door or lid remains synchronized without invisible proxy geometry.
pub fn resolve_block_prop_effects(
    instances: &[BlockPropInstance],
    assets: &IndexMap<Uuid, BlockPropAsset>,
) -> BlockPropEffectResolution {
    let mut resolution = BlockPropEffectResolution::default();

    for instance in instances {
        let Some(asset) = assets.get(&instance.asset_id) else {
            continue;
        };

        let resolve_attachment = |part_id: Uuid, attachment_id: Uuid| {
            let part = asset.find_part(part_id)?;
            let attachment = part
                .attachments
                .iter()
                .find(|attachment| attachment.id == attachment_id)?;
            let mut diagnostics = Vec::new();
            let part_transform = resolved_part_transform(asset, instance, part, &mut diagnostics);
            if !diagnostics.is_empty() {
                return None;
            }
            let world_transform =
                multiply_block_prop_transforms(part_transform, instance.world_transform);
            let origin = transform_block_prop_point(world_transform, attachment.position);
            let direction = transform_block_prop_direction(world_transform, attachment.direction)
                .try_normalized()
                .unwrap_or(Vec3::unit_y());
            Some((origin, direction))
        };

        for effect in asset
            .particle_effects
            .iter()
            .filter(|effect| effect.enabled)
        {
            let Some((origin, attachment_direction)) =
                resolve_attachment(effect.part_id, effect.attachment_id)
            else {
                continue;
            };
            // The attachment frame is shared by every effect on it and is the
            // authoritative direction edited by the Prefab viewport gizmo.
            let direction = attachment_direction;
            resolution.particles.push(ResolvedBlockPropParticleEffect {
                instance_id: instance.id,
                asset_id: asset.id,
                effect_id: effect.id,
                origin,
                direction,
                emitter: effect.emitter.clone(),
            });
        }

        for effect in asset.light_effects.iter().filter(|effect| effect.enabled) {
            let Some((origin, _)) = resolve_attachment(effect.part_id, effect.attachment_id) else {
                continue;
            };
            resolution.lights.push(ResolvedBlockPropLightEffect {
                instance_id: instance.id,
                asset_id: asset.id,
                effect_id: effect.id,
                position: origin + Vec3::new(0.0, effect.lift, 0.0),
                color: effect.color,
                intensity: effect.intensity,
                range: effect.range,
                flicker: effect.flicker,
            });
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
        host_attachment: None,
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
    fn prefab_effects_follow_part_and_instance_transforms() {
        let mut asset = BlockPropAsset::new_authored("Torch", Vec::new());
        let part_id = asset.parts[0].id;
        let attachment_id = Uuid::new_v4();
        asset.parts[0].attachments.push(BlockPropAttachment {
            id: attachment_id,
            name: "Flame".to_string(),
            position: [0.25, 1.5, -0.4],
            direction: [0.0, 1.0, 0.0],
            up: [0.0, 0.0, 1.0],
        });
        let particle_id = Uuid::new_v4();
        asset.particle_effects.push(BlockPropParticleEffect {
            id: particle_id,
            name: "Flame".to_string(),
            part_id,
            attachment_id,
            enabled: true,
            emitter: ParticleEmitterDef::default(),
        });
        let light_id = Uuid::new_v4();
        asset.light_effects.push(BlockPropLightEffect {
            id: light_id,
            name: "Fire Light".to_string(),
            part_id,
            attachment_id,
            enabled: true,
            color: [255, 128, 32, 255],
            intensity: 2.0,
            range: 4.0,
            flicker: 0.2,
            lift: 0.1,
        });

        let mut instance = BlockPropInstance::new(asset.id);
        instance.world_transform[3][0] = 4.0;
        instance.world_transform[3][1] = 2.0;
        instance.world_transform[3][2] = 8.0;
        let mut assets = IndexMap::new();
        assets.insert(asset.id, asset);

        let resolved = resolve_block_prop_effects(&[instance], &assets);
        assert_eq!(resolved.particles.len(), 1);
        assert_eq!(resolved.lights.len(), 1);
        assert_eq!(resolved.particles[0].effect_id, particle_id);
        assert_eq!(resolved.particles[0].origin, Vec3::new(4.25, 3.5, 7.6));
        assert_eq!(resolved.lights[0].effect_id, light_id);
        assert_eq!(resolved.lights[0].position, Vec3::new(4.25, 3.6, 7.6));
    }

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
    fn support_surface_hit_and_local_placement_follow_instance_transform() {
        let object = GeometryObject::box_(
            "Table Top",
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(2.0, 1.0, 2.0),
        );
        let object_id = object.id;
        let face_id = object.faces[4].id;
        let mut asset = BlockPropAsset::new_authored("Table", vec![object]);
        let part_id = asset.parts[0].id;
        let surface_id = Uuid::new_v4();
        asset.support_surfaces.push(BlockPropSupportSurface {
            id: surface_id,
            name: "Tabletop".to_string(),
            part_id,
            shape: BlockPropSemanticShape::Faces(vec![BlockPropFaceRef { object_id, face_id }]),
            snap_spacing: 0.25,
            allowed_item_tags: vec!["placeable".to_string()],
            capacity: None,
            occupancy_policy: BlockPropOccupancyPolicy::RejectOverlap,
        });
        let asset_id = asset.id;
        let mut instance = BlockPropInstance::new(asset_id);
        instance.world_transform[3][0] = 5.0;
        instance.world_transform[3][1] = 2.0;
        instance.world_transform[3][2] = 7.0;
        let rendered_object_id = block_prop_instance_object_id(instance.id, part_id, object_id);
        let paint_surface_id = block_prop_paint_surface_id(object_id, face_id);
        let assets = IndexMap::from([(asset_id, asset)]);

        assert_eq!(
            resolve_block_prop_support_surface_hit(
                std::slice::from_ref(&instance),
                &assets,
                rendered_object_id,
                paint_surface_id,
            ),
            Some(BlockPropSupportSurfaceHit {
                instance_id: instance.id,
                asset_id,
                part_id,
                surface_id,
            })
        );

        let asset = &assets[&asset_id];
        let local = Vec3::new(0.5, 0.0, 0.25);
        let world = block_prop_support_surface_world_point(
            asset,
            &instance,
            surface_id,
            [local.x, local.y, local.z],
        )
        .unwrap();
        assert_eq!(
            resolve_block_prop_support_surface_hit_at_point(
                std::slice::from_ref(&instance),
                &assets,
                rendered_object_id,
                world,
            ),
            Some(BlockPropSupportSurfaceHit {
                instance_id: instance.id,
                asset_id,
                part_id,
                surface_id,
            })
        );
        assert_eq!(
            resolve_block_prop_support_surface_hit_at_world_point(
                std::slice::from_ref(&instance),
                &assets,
                world,
            ),
            Some(BlockPropSupportSurfaceHit {
                instance_id: instance.id,
                asset_id,
                part_id,
                surface_id,
            })
        );
        let round_trip =
            block_prop_support_surface_local_point(asset, &instance, surface_id, world).unwrap();
        assert!((round_trip - local).magnitude() < 1e-5);

        let item_creator_id = Uuid::new_v4();
        let mut item = crate::Item::new();
        item.creator_id = item_creator_id;
        let mut local_transform = identity_block_prop_transform();
        local_transform[3][0] = local.x;
        local_transform[3][2] = local.z;
        let placement = BlockPropSurfacePlacement {
            id: Uuid::new_v4(),
            prop_instance_id: instance.id,
            surface_id,
            occupant: BlockPropOccupant::ItemInstance(item_creator_id),
            local_transform,
        };
        assert_eq!(
            sync_block_prop_surface_item_positions(
                std::slice::from_ref(&instance),
                std::slice::from_ref(&placement),
                std::slice::from_mut(&mut item),
                &assets,
            ),
            1
        );
        assert!((item.position - world).magnitude() < 1e-5);

        instance.world_transform[3][0] += 2.0;
        sync_block_prop_surface_item_positions(
            std::slice::from_ref(&instance),
            std::slice::from_ref(&placement),
            std::slice::from_mut(&mut item),
            &assets,
        );
        assert!((item.position.x - (world.x + 2.0)).abs() < 1e-5);
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
    fn split_door_leaves_share_state_and_move_in_opposite_directions() {
        let left =
            GeometryObject::box_("Left", Vec3::new(-0.5, 1.0, 0.0), Vec3::new(1.0, 2.0, 0.2));
        let right =
            GeometryObject::box_("Right", Vec3::new(0.5, 1.0, 0.0), Vec3::new(1.0, 2.0, 0.2));
        let mut asset = BlockPropAsset::new("Split Door");
        let left_part = BlockPropPart::new_authored("Left", vec![left]);
        let left_part_id = left_part.id;
        let right_part = BlockPropPart::new_authored("Right", vec![right]);
        let right_part_id = right_part.id;
        asset.parts.extend([left_part, right_part]);
        let mut door = BlockPropComponent::new("Door");
        door.properties.set("part_id", Value::Id(left_part_id));
        door.properties
            .set("secondary_part_id", Value::Id(right_part_id));
        door.properties
            .set("motion", Value::Str("Slide".to_string()));
        door.properties
            .set("slide_axis", Value::Vec3([1.0, 0.0, 0.0]));
        door.properties.set("slide_distance", Value::Float(2.0));
        let door_id = door.id;
        asset.components.push(door);
        let mut instance = BlockPropInstance::new(asset.id);
        set_block_prop_door_open(&mut instance, door_id, true);

        let assets = IndexMap::from([(asset.id, asset)]);
        let resolution = resolve_block_prop_geometry(&[instance], &assets);
        assert_eq!(resolution.geometry_objects.len(), 2);
        assert!((resolution.geometry_objects[0].transform[3][0] + 2.0).abs() < 0.001);
        assert!((resolution.geometry_objects[1].transform[3][0] - 2.0).abs() < 0.001);
    }

    #[test]
    fn split_swing_uses_opposite_angles() {
        let mut asset = BlockPropAsset::new("Split Gate");
        let mut left = BlockPropPart::new_authored(
            "Left",
            vec![GeometryObject::box_("Left", Vec3::zero(), Vec3::one())],
        );
        left.pivot = [-1.0, 0.0, 0.0];
        let left_id = left.id;
        let mut right = BlockPropPart::new_authored(
            "Right",
            vec![GeometryObject::box_("Right", Vec3::zero(), Vec3::one())],
        );
        right.pivot = [1.0, 0.0, 0.0];
        let right_id = right.id;
        asset.parts.extend([left, right]);
        let mut door = BlockPropComponent::new("Door");
        door.properties.set("part_id", Value::Id(left_id));
        door.properties
            .set("secondary_part_id", Value::Id(right_id));
        door.properties
            .set("motion", Value::Str("Swing".to_string()));
        door.properties.set("angle_degrees", Value::Float(90.0));
        let door_id = door.id;
        asset.components.push(door);
        let mut instance = BlockPropInstance::new(asset.id);
        set_block_prop_door_open(&mut instance, door_id, true);

        let assets = IndexMap::from([(asset.id, asset)]);
        let resolution = resolve_block_prop_geometry(&[instance], &assets);
        assert!(resolution.geometry_objects[0].transform[0][2] > 0.9);
        assert!(resolution.geometry_objects[1].transform[0][2] < -0.9);
    }

    #[test]
    fn legacy_door_face_target_resolves_whole_part_and_context_verb() {
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
        let rendered_object_id =
            block_prop_instance_object_id(instance.id, part_id, source_object_id);
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
        assert_eq!(hit.target_id, Some(target_id));
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
            .is_some()
        );
    }

    #[test]
    fn rendered_object_pick_resolves_whole_part_door_target() {
        let object = GeometryObject::box_("Leaf", Vec3::zero(), Vec3::one());
        let source_object_id = object.id;
        let mut asset = BlockPropAsset::new_authored("Door", vec![object]);
        let part_id = asset.parts[0].id;
        let mut component = BlockPropComponent::new("Door");
        component.properties.set("part_id", Value::Id(part_id));
        let component_id = component.id;
        asset.components.push(component);
        let target_id = Uuid::new_v4();
        asset.interaction_targets.push(BlockPropInteractionTarget {
            id: target_id,
            name: "Door Interaction".to_string(),
            part_id,
            shape: BlockPropSemanticShape::Part,
            interaction_anchor: [0.0, 0.0, 0.0],
            facing_direction: [0.0, 0.0, 1.0],
            component_id: Some(component_id),
        });
        let instance = BlockPropInstance::new(asset.id);
        let rendered_object_id =
            block_prop_instance_object_id(instance.id, part_id, source_object_id);
        let assets = IndexMap::from([(asset.id, asset)]);

        let hit = resolve_block_prop_interaction_hit(
            std::slice::from_ref(&instance),
            &assets,
            rendered_object_id,
            None,
        )
        .expect("any object in the Door part should resolve");
        assert_eq!(hit.target_id, Some(target_id));
        assert_eq!(hit.component_id, Some(component_id));
    }

    #[test]
    fn rendered_object_pick_resolves_prefab_without_interaction_component() {
        let object = GeometryObject::box_("Table", Vec3::zero(), Vec3::one());
        let source_object_id = object.id;
        let asset = BlockPropAsset::new_authored("Table", vec![object]);
        let part_id = asset.parts[0].id;
        let instance = BlockPropInstance::new(asset.id);
        let rendered_object_id =
            block_prop_instance_object_id(instance.id, part_id, source_object_id);
        let assets = IndexMap::from([(asset.id, asset)]);

        let hit = resolve_block_prop_interaction_hit(
            std::slice::from_ref(&instance),
            &assets,
            rendered_object_id,
            None,
        )
        .expect("any linked Prefab geometry should resolve for authoring intents");
        assert_eq!(hit.instance_id, instance.id);
        assert_eq!(hit.part_id, part_id);
        assert_eq!(hit.target_id, None);
        assert_eq!(hit.component_id, None);
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
