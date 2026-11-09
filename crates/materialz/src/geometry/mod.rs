//! Portable geometry-row prototype. Material/image rows keep their existing evaluator.
//! References always point upwards; topology is evaluated only after source edits.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
pub mod authoring;
pub mod connectors;
pub mod effects;
pub mod expressions;
pub mod footprint;
pub mod group;
mod loft;
mod mesh;
pub mod parameters;
pub mod rig;
pub use loft::Section;
pub use mesh::{Mesh, Part, Texture, Vertex, validate_contour};
pub const SWORD_EXAMPLE: &str = include_str!("../../examples/geometry/sword.mzgeometry.json");
pub const PITCHED_ROOF_RECIPE: &str =
    include_str!("../../examples/geometry/pitched-roof.mzgeometry.json");
pub const HUMAN_EXAMPLE: &str = include_str!("../../examples/geometry/human.mzgeometry.json");
pub const HUMAN_DETAIL_EXAMPLE: &str =
    include_str!("../../examples/geometry/human-detail.mzgeometry.json");
pub const SWORD_GROUP_EXAMPLE: &str =
    include_str!("../../examples/geometry/sword-group.mzgeometry.json");
/// A host-supplied geometry input; the embedded document provides a standalone preview.
pub const WEARER_SOURCE: &str = "geometry:input/wearer";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub version: u32,
    pub id: String,
    pub label: String,
    pub rows: Vec<Row>,
    pub output: String,
    pub grip: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub id: String,
    pub label: String,
    pub tiles: Vec<Tile>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tile {
    pub id: String,
    pub op: Operation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    /// Named geometry supplied by a host, with a portable standalone preview.
    Input {
        name: String,
        preview: Box<Document>,
    },
    /// Fit the current mesh to another row, independently along each axis.
    FitTo {
        target: String,
        axes: [bool; 3],
        source_anchor: [f32; 3],
        target_anchor: [f32; 3],
        padding: [f32; 3],
    },
    /// Portable dependency snapshot. Hosts resolve `source` to the current shared
    /// asset before evaluating; standalone documents retain a usable snapshot.
    Asset {
        source: String,
        document: Box<Document>,
    },
    Armature {
        joints: Vec<authoring::JointDef>,
    },
    Profiles {
        rig: String,
        parts: Vec<authoring::ProfilePart>,
    },
    ParticleEffect {
        effect: effects::ParticleEffect,
    },
    Texture {
        #[serde(default)]
        preserve_uv: bool,
        #[serde(default)]
        parts: Vec<String>,
        source: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundle: Option<String>,
        size: u32,
        axes: [usize; 2],
        origin: [f32; 2],
        span: [f32; 2],
    },
    /// Attach an editable X/Z Contour row as the static mesh placement footprint.
    Footprint {
        row: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        holes: Vec<String>,
        #[serde(default, skip_serializing_if = "is_false")]
        append: bool,
    },
    /// Invisible static placement socket. Local +Z is the outward direction.
    Connector {
        family: String,
        translation: [f32; 3],
        rotation: [f32; 3],
    },
    Group {
        label: String,
        document: Box<Document>,
        controls: Vec<group::Control>,
    },
    Loft {
        sections: Vec<Section>,
        segments: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        material: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skin: Option<group::Binding>,
    },
    Joint {
        parent: Option<String>,
        translation: [f32; 3],
        rotation: [f32; 3],
    },
    Skeleton {
        rows: Vec<String>,
    },
    Skin {
        rig: String,
        joints: Vec<String>,
        blend: f32,
    },
    Attach {
        #[serde(default = "attachment_enabled")]
        enabled: bool,
        rig: String,
        joint: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        grip: Option<String>,
        translation: [f32; 3],
        rotation: [f32; 3],
    },
    Motion {
        rig: String,
        duration: f32,
        looping: bool,
        tracks: Vec<rig::Track>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        events: Vec<rig::Event>,
    },
    Contour {
        points: Vec<[f32; 2]>,
    },
    Extrude {
        depth: f32,
    },
    Box {
        size: [f32; 3],
    },
    SelectParts {
        parts: Vec<String>,
    },
    SurfaceOffset {
        distance: f32,
    },
    Revolve {
        segments: u32,
    },
    Transform {
        translation: [f32; 3],
        rotation: [f32; 3],
        scale: [f32; 3],
    },
    /// Repeat static geometry with a fixed translation/rotation step per copy.
    Repeat {
        count: u32,
        translation: [f32; 3],
        rotation: [f32; 3],
    },
    Reference {
        row: String,
    },
    Assemble {
        rows: Vec<String>,
    },
    AssignMaterial {
        row: String,
    },
    Material {
        palette: u16,
        roughness: f32,
        metallic: f32,
    },
    Grip {
        translation: [f32; 3],
        rotation: [f32; 3],
    },
}
impl Operation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Input { .. } => "Geometry Input",
            Self::FitTo { .. } => "Fit To Geometry",
            Self::ParticleEffect { .. } => "Particle Effect",
            Self::Armature { .. } => "Skeleton",
            Self::Profiles { .. } => "Body Profiles",
            Self::Texture { .. } => "Texture",
            Self::Group { .. } => "Group",
            Self::Footprint { .. } => "Footprint",
            Self::Connector { .. } => "Connector",
            Self::Asset { .. } => "Geometry Asset",
            Self::Loft { .. } => "Loft",
            Self::Joint { .. } => "Joint",
            Self::Skeleton { .. } => "Skeleton",
            Self::Skin { .. } => "Skin",
            Self::Attach { .. } => "Attach",
            Self::Motion { .. } => "Motion",
            Self::Contour { .. } => "Contour",
            Self::Extrude { .. } => "Extrude",
            Self::Box { .. } => "Box",
            Self::SelectParts { .. } => "Select Parts",
            Self::SurfaceOffset { .. } => "Surface Offset",
            Self::Revolve { .. } => "Revolve",
            Self::Transform { .. } => "Transform",
            Self::Repeat { .. } => "Repeat",
            Self::Reference { .. } => "Row",
            Self::Assemble { .. } => "Assemble",
            Self::AssignMaterial { .. } => "Material",
            Self::Material { .. } => "PBR Material",
            Self::Grip { .. } => "Grip",
        }
    }
    pub fn references(&self) -> Vec<&str> {
        match self {
            Self::FitTo { target, .. } => vec![target],
            Self::Footprint { row, holes, .. } => std::iter::once(row.as_str())
                .chain(holes.iter().map(String::as_str))
                .collect(),
            Self::Profiles { rig, .. } => vec![rig],
            Self::Loft { material, skin, .. } => material
                .iter()
                .map(String::as_str)
                .chain(skin.iter().map(|b| b.rig.as_str()))
                .collect(),
            Self::Joint { parent, .. } => parent.iter().map(String::as_str).collect(),
            Self::Skeleton { rows } => rows.iter().map(String::as_str).collect(),
            Self::Skin { rig, .. } | Self::Motion { rig, .. } => vec![rig],
            Self::Attach { rig, grip, .. } => std::iter::once(rig.as_str())
                .chain(grip.iter().map(String::as_str))
                .collect(),
            Self::Reference { row } | Self::AssignMaterial { row } => {
                vec![row]
            }
            Self::Assemble { rows } => rows.iter().map(String::as_str).collect(),
            _ => vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Surface {
    pub palette: u16,
    pub roughness: f32,
    pub metallic: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
}
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Rig(rig::Rig),
    Motion(rig::Motion),
    Contour(Vec<[f32; 2]>),
    Mesh(Mesh),
    Material(Surface),
    Grip(Frame),
}
impl Value {
    pub fn rig(&self) -> Option<&rig::Rig> {
        match self {
            Self::Rig(r) => Some(r),
            Self::Mesh(m) => m.rig.as_ref(),
            _ => None,
        }
    }
    pub fn grip(&self) -> Option<&Frame> {
        match self {
            Self::Grip(f) => Some(f),
            Self::Mesh(m) => m.grip.as_ref(),
            _ => None,
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Rig(_) => "Rig",
            Self::Motion(_) => "Motion",
            Self::Contour(_) => "Contour",
            Self::Mesh(_) => "Mesh",
            Self::Material(_) => "Material",
            Self::Grip(_) => "Grip",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Evaluation {
    pub rows: BTreeMap<String, Value>,
    pub operations: BTreeMap<String, Value>,
    pub mesh: Mesh,
    pub grip: Option<Frame>,
}

fn is_false(v: &bool) -> bool {
    !*v
}
fn attachment_enabled() -> bool {
    true
}
fn fail(message: impl Into<String>) -> Error {
    Error(message.into())
}
pub fn finite(values: &[f32], limit: f32) -> Result<()> {
    if values.iter().any(|v| !v.is_finite() || v.abs() > limit) {
        return Err(fail("Non-finite or out-of-range geometry value"));
    }
    Ok(())
}
impl Document {
    pub fn wearer_context(&self) -> Option<&Document> {
        self.rows
            .iter()
            .flat_map(|r| &r.tiles)
            .find_map(|tile| match &tile.op {
                Operation::Asset { source, document } if source == WEARER_SOURCE => {
                    Some(document.as_ref())
                }
                Operation::Asset { document, .. } | Operation::Group { document, .. } => {
                    document.wearer_context()
                }
                _ => None,
            })
    }
    /// Publish the mesh that drives the motion rows, excluding held equipment.
    pub fn wearer_body(&self) -> Result<Self> {
        let mut body = self.clone();
        if let Some(row) = self.rows.iter().flat_map(|r| &r.tiles).find_map(|t| {
            if let Operation::Motion { rig, .. } = &t.op {
                Some(rig)
            } else {
                None
            }
        }) {
            let index = self
                .rows
                .iter()
                .position(|r| &r.id == row)
                .ok_or_else(|| fail("Missing wearer body row"))?;
            body.rows.truncate(index + 1);
            body.output = row.clone();
        }
        body.grip = None;
        Ok(body)
    }
    pub fn with_wearer(&self, wearer: &Document) -> Self {
        let mut doc = self.clone();
        for tile in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
            match &mut tile.op {
                Operation::Asset { source, document } if source == WEARER_SOURCE => {
                    **document = wearer.clone()
                }
                Operation::Asset { document, .. } | Operation::Group { document, .. } => {
                    **document = document.with_wearer(wearer)
                }
                _ => {}
            }
        }
        doc
    }
    /// Decode an editable document without generating meshes. Hosts resolving
    /// shared dependencies must evaluate the final linked graph before using it.
    pub fn parse(text: &str) -> Result<Self> {
        if text.len() > 4 * 1024 * 1024 {
            return Err(fail("Geometry document exceeds 4 MiB"));
        }
        let document: Self = serde_json::from_str(text).map_err(|e| fail(e.to_string()))?;
        if document.version != 1 {
            return Err(fail("Unsupported geometry document version"));
        }
        Ok(document)
    }
    pub fn load(text: &str) -> Result<Self> {
        let document = Self::parse(text)?;
        document.evaluate()?;
        Ok(document)
    }
    pub fn save(&self) -> Result<String> {
        self.evaluate()?;
        serde_json::to_string_pretty(self).map_err(|e| fail(e.to_string()))
    }
    pub fn evaluate(&self) -> Result<Evaluation> {
        self.evaluate_with_inputs(&BTreeMap::new())
    }
    pub fn evaluate_with_inputs(&self, inputs: &BTreeMap<String, Mesh>) -> Result<Evaluation> {
        self.evaluate_at(0, &mut 0, inputs)
    }
    fn evaluate_at(
        &self,
        depth: usize,
        retained_vertices: &mut usize,
        inputs: &BTreeMap<String, Mesh>,
    ) -> Result<Evaluation> {
        if depth > 6 {
            return Err(fail("Groups may be nested at most six levels"));
        }
        if self.version != 1 {
            return Err(fail("Unsupported geometry document version"));
        }
        if self.id.is_empty() || self.rows.is_empty() || self.rows.len() > 128 {
            return Err(fail("Geometry needs an identity and 1–128 rows"));
        }
        let mut ids = HashSet::new();
        let mut values = BTreeMap::new();
        let mut operations = BTreeMap::new();
        let mut total = 0;
        for row in &self.rows {
            if row.id.is_empty() || !ids.insert(row.id.clone()) {
                return Err(fail("Duplicate or empty row/tile identity"));
            }
            let mut value = None;
            for tile in &row.tiles {
                total += 1;
                if total > 512 || tile.id.is_empty() || !ids.insert(tile.id.clone()) {
                    return Err(fail("Duplicate tile identity or geometry limit exceeded"));
                }
                value = Some(
                    evaluate_tile(
                        &tile.op,
                        value,
                        &values,
                        if matches!(
                            tile.op,
                            Operation::ParticleEffect { .. } | Operation::Connector { .. }
                        ) {
                            &tile.id
                        } else {
                            &row.id
                        },
                        depth,
                        retained_vertices,
                        inputs,
                    )
                    .map_err(|e| fail(format!("{} / {}: {e}", row.label, tile.op.name())))?,
                );
                if let Some(Value::Mesh(mesh)) = &value {
                    if mesh.effects.len() > 32 {
                        return Err(fail("At most 32 particle attachments per model"));
                    }
                    for vertex in mesh.parts.iter().flat_map(|p| &p.vertices) {
                        finite(&vertex.position, 1_000_000.)?;
                        // Quaternion rotations can put a unit component a few
                        // ulps above one, especially on rigid worn accessories.
                        finite(&vertex.normal, 1.00001)?;
                    }
                    *retained_vertices += mesh.vertex_count()
                        + mesh.footprint.as_ref().map_or(0, |s| s.points().count());
                    if *retained_vertices > 1_000_000 {
                        return Err(fail("Geometry intermediates exceed 1000000 vertices"));
                    }
                }
                operations.insert(tile.id.clone(), value.as_ref().unwrap().clone());
            }
            if let Some(value) = value {
                values.insert(row.id.clone(), value);
            }
        }
        let Some(Value::Mesh(mesh)) = values.get(&self.output) else {
            return Err(fail("Geometry output must reference a mesh row"));
        };
        let grip = match &self.grip {
            Some(id) => Some(
                values
                    .get(id)
                    .and_then(Value::grip)
                    .ok_or_else(|| fail("Grip output must reference a grip or grouped item"))?
                    .clone(),
            ),
            None => None,
        };
        if mesh.parts.is_empty() && mesh.rig.is_none() {
            return Err(fail("Geometry output is empty"));
        }
        connectors::validate(&mesh.connectors)?;
        if let Some(shape) = &mesh.footprint {
            footprint::validate(&shape.map(|p| [p[0], p[2]]))?;
        }
        Ok(Evaluation {
            mesh: mesh.clone(),
            grip,
            rows: values,
            operations,
        })
    }
}
fn evaluate_tile(
    op: &Operation,
    input: Option<Value>,
    rows: &BTreeMap<String, Value>,
    owner: &str,
    depth: usize,
    retained_vertices: &mut usize,
    inputs: &BTreeMap<String, Mesh>,
) -> Result<Value> {
    let reference = |row: &String| {
        rows.get(row)
            .cloned()
            .ok_or_else(|| fail(format!("Missing or forward row reference {row}")))
    };
    let mesh_input = |value| match value {
        Some(Value::Mesh(m)) => Ok(m),
        _ => Err(fail("This tile requires a mesh input")),
    };
    Ok(match op {
        Operation::Input { name, preview } => {
            if name.is_empty() || name.len() > 128 {
                return Err(fail("Geometry input needs a name of 1–128 characters"));
            }
            let mesh = match inputs.get(name) {
                Some(mesh) => {
                    if mesh.vertex_count() == 0
                        || mesh.vertex_count() > 200_000
                        || mesh.parts.is_empty()
                    {
                        return Err(fail("Geometry input must contain 1–200000 vertices"));
                    }
                    mesh.clone()
                }
                None => {
                    preview
                        .evaluate_at(depth + 1, retained_vertices, inputs)?
                        .mesh
                }
            };
            Value::Mesh(mesh)
        }
        Operation::FitTo {
            target,
            axes,
            source_anchor,
            target_anchor,
            padding,
        } => {
            for anchor in [source_anchor, target_anchor] {
                if anchor
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                {
                    return Err(fail("Fit anchors must be between 0 and 1"));
                }
            }
            finite(padding, 100.)?;
            let target = mesh_input(Some(reference(target)?))?;
            let mut mesh = mesh_input(input)?;
            let (lo, hi) = mesh.bounds();
            let (target_lo, target_hi) = target.bounds();
            let mut scale = [1.; 3];
            let mut translation = [0.; 3];
            for axis in 0..3 {
                let length = hi[axis] - lo[axis];
                let target_length = target_hi[axis] - target_lo[axis] + 2. * padding[axis];
                if axes[axis] {
                    if length <= 0.00001 || target_length <= 0.00001 {
                        return Err(fail("Fit requires positive extents on each fitted axis"));
                    }
                    scale[axis] = target_length / length;
                }
                translation[axis] = target_lo[axis] - padding[axis]
                    + target_anchor[axis] * target_length
                    - (lo[axis] + source_anchor[axis] * length) * scale[axis];
            }
            mesh.transform(translation, [0.; 3], scale);
            Value::Mesh(mesh)
        }
        Operation::Asset { source, document } => {
            if source.is_empty() || source.len() > 256 {
                return Err(fail("Geometry asset needs a valid source"));
            }
            let evaluation = document.evaluate_at(depth + 1, retained_vertices, inputs)?;
            let mut mesh = evaluation.mesh;
            mesh.grip = evaluation.grip;
            Value::Mesh(mesh)
        }
        Operation::ParticleEffect { effect } => {
            effect.validate()?;
            let mut mesh = match input {
                Some(Value::Mesh(m)) => m,
                None => Mesh::default(),
                _ => return Err(fail("Particle Effect requires a mesh input")),
            };
            let joint = effect
                .joint
                .as_ref()
                .map(|id| {
                    mesh.rig
                        .as_ref()
                        .ok_or_else(|| fail("Particle attachment needs a skeleton"))?
                        .index(id)
                })
                .transpose()?;
            if mesh.effects.len() >= 32 {
                return Err(fail("At most 32 particle attachments per model"));
            }
            mesh.effects.push(effects::EffectAttachment {
                id: owner.into(),
                joint,
                definition: effect.clone(),
            });
            Value::Mesh(mesh)
        }
        Operation::Armature { joints } => Value::Rig(authoring::skeleton(joints)?),
        Operation::Profiles { rig: source, parts } => {
            let value = reference(source)?;
            let rig = value
                .rig()
                .ok_or_else(|| fail("Profiles need a skeleton"))?;
            Value::Mesh(authoring::profiles(owner, rig, parts)?)
        }
        Operation::Footprint { row, holes, append } => {
            if holes.len() > 32 {
                return Err(fail(
                    "One footprint region supports at most 32 hole contours",
                ));
            }
            let contour = |id: &String| -> Result<Vec<[f32; 3]>> {
                let Value::Contour(points) = reference(id)? else {
                    return Err(fail("Footprint requires X/Z Contour rows"));
                };
                Ok(points.into_iter().map(|p| [p[0], 0., p[1]]).collect())
            };
            let shape = footprint::Shape {
                regions: vec![footprint::Region {
                    outer: contour(row)?,
                    holes: holes
                        .iter()
                        .map(|h| contour(h))
                        .collect::<Result<Vec<_>>>()?,
                }],
            };
            footprint::validate(&shape.map(|p| [p[0], p[2]]))?;
            let mut mesh = mesh_input(input)?;
            if mesh.rig.is_some() {
                return Err(fail(
                    "Footprint describes a static mesh; add it before rigging",
                ));
            }
            if *append {
                footprint::merge(&mut mesh.footprint, Some(shape))?;
            } else {
                mesh.footprint = Some(shape);
            }
            Value::Mesh(mesh)
        }
        Operation::Connector {
            family,
            translation,
            rotation,
        } => {
            finite(translation, 1000.)?;
            finite(rotation, 3600.)?;
            let mut mesh = mesh_input(input)?;
            if mesh.rig.is_some() {
                return Err(fail(
                    "Connectors describe static placement; add them before rigging",
                ));
            }
            let connector = connectors::Connector {
                id: owner.into(),
                family: family.clone(),
                position: *translation,
                direction: mesh::rotate([0., 0., 1.], *rotation),
            };
            connector.validate()?;
            mesh.connectors.push(connector);
            connectors::validate(&mesh.connectors)?;
            Value::Mesh(mesh)
        }
        Operation::SelectParts { parts } => {
            let mut mesh = mesh_input(input)?;
            if parts.is_empty()
                || parts.len() > 128
                || parts.iter().collect::<HashSet<_>>().len() != parts.len()
                || parts
                    .iter()
                    .any(|id| !mesh.parts.iter().any(|p| &p.id == id))
            {
                return Err(fail("Select Parts needs unique existing mesh parts"));
            }
            if mesh.parts.len() != parts.len() {
                mesh.footprint = None;
                mesh.connectors.clear();
            }
            mesh.parts.retain(|p| parts.contains(&p.id));
            mesh.attachments.clear();
            mesh.grip = None;
            Value::Mesh(mesh)
        }
        Operation::SurfaceOffset { distance } => {
            if !distance.is_finite() || !(-0.1..=0.1).contains(distance) {
                return Err(fail("Surface offset must be -0.1–0.1 metres"));
            }
            let mut mesh = mesh_input(input)?;
            for vertex in mesh.parts.iter_mut().flat_map(|p| &mut p.vertices) {
                for k in 0..3 {
                    vertex.position[k] += vertex.normal[k] * distance;
                }
            }
            Value::Mesh(mesh)
        }
        Operation::Texture {
            preserve_uv,
            parts: targets,
            source,
            bundle,
            size,
            axes,
            origin,
            span,
        } => {
            if source.is_empty()
                || source.len() > 256
                || !(16..=512).contains(size)
                || axes[0] > 2
                || axes[1] > 2
                || axes[0] == axes[1]
                || span.iter().any(|s| !s.is_finite() || *s < 0.0001)
            {
                return Err(fail(
                    "Texture requires a source, valid UV axes, size and positive span",
                ));
            }
            finite(origin, 1000.)?;
            let mut mesh = mesh_input(input)?;
            for part in &mut mesh.parts {
                if !targets.is_empty() && !targets.contains(&part.id) {
                    continue;
                }
                part.texture = Some(mesh::Texture {
                    source: source.clone(),
                    bundle: bundle.clone(),
                    size: *size,
                });
                for v in &mut part.vertices {
                    v.uv = std::array::from_fn(|k| {
                        let coordinate = if *preserve_uv {
                            v.uv[k]
                        } else {
                            v.position[axes[k]]
                        };
                        let uv = (coordinate - origin[k]) / span[k];
                        if k == 1 && !*preserve_uv { 1. - uv } else { uv }
                    });
                }
            }
            Value::Mesh(mesh)
        }
        Operation::Group {
            document, controls, ..
        } => {
            let controls = expressions::resolve(controls)?;
            let bound = parameters::bind_resolved(document, &controls)?;
            let evaluation = bound.evaluate_at(depth + 1, retained_vertices, inputs)?;
            let mut mesh = evaluation.mesh;
            mesh.grip = evaluation.grip;
            group::apply_resolved(&mut mesh, &controls)?;
            Value::Mesh(mesh)
        }
        Operation::Loft {
            sections,
            segments,
            material,
            skin,
        } => {
            let mut mesh = loft::build(owner, sections, *segments)?;
            if let Some(id) = material {
                let Value::Material(surface) = reference(id)? else {
                    return Err(fail("Loft material must reference a material row"));
                };
                for p in &mut mesh.parts {
                    p.surface = Some(surface.clone());
                }
            }
            if let Some(binding) = skin {
                let value = reference(&binding.rig)?;
                let rig = value
                    .rig()
                    .ok_or_else(|| fail("Loft binding requires a skeleton"))?;
                rig::bind(&mut mesh, rig, &binding.joints, binding.blend)?;
            }
            Value::Mesh(mesh)
        }
        Operation::Joint {
            parent,
            translation,
            rotation,
        } => {
            let parent = parent.as_ref().map(|id| reference(id)).transpose()?;
            let parent = match parent {
                Some(Value::Rig(rig)) => Some(rig),
                None => None,
                _ => return Err(fail("Joint parent must be a joint row")),
            };
            Value::Rig(rig::Rig::joint(
                owner,
                parent.as_ref(),
                *translation,
                *rotation,
            )?)
        }
        Operation::Skeleton { rows: inputs } => {
            let mut rigs = vec![];
            for id in inputs {
                let Value::Rig(r) = reference(id)? else {
                    return Err(fail("Skeleton requires joint/rig rows"));
                };
                rigs.push(r);
            }
            Value::Rig(rig::Rig::merge(&rigs)?)
        }
        Operation::Skin {
            rig: source,
            joints,
            blend,
        } => {
            let value = reference(source)?;
            let rig = value
                .rig()
                .ok_or_else(|| fail("Skin requires a skeleton or skinned group"))?;
            let mut m = mesh_input(input)?;
            rig::bind(&mut m, rig, joints, *blend)?;
            Value::Mesh(m)
        }
        Operation::Attach {
            enabled,
            rig: source,
            joint,
            grip,
            translation,
            rotation,
        } => {
            if !enabled {
                return Ok(Value::Mesh(Mesh::default()));
            }
            let value = reference(source)?;
            let rig = value
                .rig()
                .ok_or_else(|| fail("Attach requires a skeleton or skinned group"))?;
            let mut m = mesh_input(input)?;
            let grip = match grip {
                Some(id) => reference(id)?.grip().cloned(),
                None => m.grip.clone(),
            }
            .ok_or_else(|| fail("Attach requires an item grip frame"))?;
            rig::attach(
                &mut m,
                rig,
                joint,
                &grip,
                Frame {
                    translation: *translation,
                    rotation: *rotation,
                },
                owner,
            )?;
            Value::Mesh(m)
        }
        Operation::Motion {
            rig: source,
            duration,
            looping,
            tracks,
            events,
        } => {
            let value = reference(source)?;
            let rig = value
                .rig()
                .ok_or_else(|| fail("Motion requires a skeleton or skinned group"))?
                .clone();
            let motion = rig::Motion {
                rig,
                duration: *duration,
                looping: *looping,
                tracks: tracks.clone(),
                events: events.clone(),
            };
            motion.validate()?;
            Value::Motion(motion)
        }
        Operation::Contour { points } => {
            mesh::validate_contour(points)?;
            Value::Contour(points.clone())
        }
        Operation::Extrude { depth } => {
            let Some(Value::Contour(points)) = input else {
                return Err(fail("Extrude requires a contour"));
            };
            Value::Mesh(mesh::extrude(owner, &points, *depth)?)
        }
        Operation::Box { size } => Value::Mesh(mesh::box_mesh(owner, *size)?),
        Operation::Revolve { segments } => {
            let Some(Value::Contour(points)) = input else {
                return Err(fail("Revolve requires a radius/height contour"));
            };
            Value::Mesh(mesh::revolve(owner, &points, *segments)?)
        }
        Operation::Transform {
            translation,
            rotation,
            scale,
        } => {
            finite(translation, 1000.)?;
            finite(rotation, 3600.)?;
            finite(scale, 100.)?;
            if scale.iter().any(|v| *v < 0.001) {
                return Err(fail("Scale must be positive (0.001–100)"));
            }
            let mut m = mesh_input(input)?;
            if m.rig.is_some() {
                return Err(fail("Transform geometry before skinning or attaching it"));
            }
            m.transform(*translation, *rotation, *scale);
            Value::Mesh(m)
        }
        Operation::Reference { row } => reference(row)?,
        Operation::Repeat {
            count,
            translation,
            rotation,
        } => {
            finite(translation, 1000.)?;
            finite(rotation, 3600.)?;
            let base = mesh_input(input)?;
            if !(1..=128).contains(count)
                || base
                    .vertex_count()
                    .checked_mul(*count as usize)
                    .is_none_or(|n| n > 200_000)
            {
                return Err(fail(
                    "Repeat requires 1–128 copies within the 200000 vertex budget",
                ));
            }
            if base.rig.is_some()
                || !base.attachments.is_empty()
                || !base.effects.is_empty()
                || base.grip.is_some()
            {
                return Err(fail(
                    "Repeat accepts static mesh parts; repeat before rigging or attachments",
                ));
            }
            let mut mesh = Mesh::default();
            for i in 0..*count {
                let mut copy = base.clone();
                copy.transform(
                    translation.map(|v| v * i as f32),
                    rotation.map(|v| v * i as f32),
                    [1.; 3],
                );
                for p in &mut copy.parts {
                    p.id = format!("{owner}/{i}/{}", p.id);
                    p.label = format!("{} {}", p.label, i + 1);
                }
                for connector in &mut copy.connectors {
                    connector.id = format!("{owner}/{i}/{}", connector.id);
                }
                mesh.connectors.append(&mut copy.connectors);
                connectors::validate(&mesh.connectors)?;
                footprint::merge(&mut mesh.footprint, copy.footprint.take())?;
                mesh.parts.extend(copy.parts);
            }
            Value::Mesh(mesh)
        }
        Operation::Assemble { rows: inputs } => {
            if inputs.is_empty() || inputs.len() > 64 {
                return Err(fail("Assemble needs 1–64 mesh rows"));
            }
            let mut m = Mesh::default();
            for id in inputs {
                let Value::Mesh(mut other) = reference(id)? else {
                    return Err(fail("Assemble accepts mesh rows only"));
                };
                m.align_rigs(&mut other)?;
                for connector in &mut other.connectors {
                    connector.id = format!("{id}/{}", connector.id);
                }
                m.connectors.append(&mut other.connectors);
                connectors::validate(&m.connectors)?;
                footprint::merge(&mut m.footprint, other.footprint.take())?;
                m.attachments.append(&mut other.attachments);
                m.effects.append(&mut other.effects);
                m.parts.append(&mut other.parts);
                if m.vertex_count() > 200_000 {
                    return Err(fail("Geometry exceeds 200000 vertices"));
                }
            }
            Value::Mesh(m)
        }
        Operation::AssignMaterial { row } => {
            let Value::Material(surface) = reference(row)? else {
                return Err(fail("Material input must reference a material row"));
            };
            let mut m = mesh_input(input)?;
            for part in &mut m.parts {
                part.surface = Some(surface.clone());
            }
            Value::Mesh(m)
        }
        Operation::Material {
            palette,
            roughness,
            metallic,
        } => {
            if *palette > 255
                || !roughness.is_finite()
                || !metallic.is_finite()
                || !(0.0..=1.).contains(roughness)
                || !(0.0..=1.).contains(metallic)
            {
                return Err(fail(
                    "Material requires a palette slot and roughness/metallic in 0–1",
                ));
            }
            Value::Material(Surface {
                palette: *palette,
                roughness: *roughness,
                metallic: *metallic,
            })
        }
        Operation::Grip {
            translation,
            rotation,
        } => {
            finite(translation, 1000.)?;
            finite(rotation, 3600.)?;
            Value::Grip(Frame {
                translation: *translation,
                rotation: *rotation,
            })
        }
    })
}
