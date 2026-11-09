//! Reusable, editable subgraphs with data-defined controls. No character-specific evaluator.
use super::*;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub rig: String,
    pub joints: Vec<String>,
    pub blend: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub id: String,
    pub label: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub help: String,
    #[serde(default)]
    pub order: u16,
    pub value: f32,
    pub reference: f32,
    pub min: f32,
    pub max: f32,
    /// Optional scalar formula; references other control IDs in this Group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    pub modifiers: Vec<Modifier>,
}
impl Control {
    pub fn is_integer(&self) -> bool {
        matches!(self.modifiers.as_slice(), [Modifier::Parameter { target, factor, offset }]
            if matches!(target.parameter.as_str(), "count" | "segments") && *factor == 1. && *offset == 0.)
    }
    pub fn is_palette(&self) -> bool {
        !self.modifiers.is_empty()
            && self
                .modifiers
                .iter()
                .all(|m| matches!(m, Modifier::Palette { .. }))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Modifier {
    /// Replace a typed construction input before evaluating the child recipe.
    Parameter {
        target: parameters::Target,
        #[serde(default = "unit_factor")]
        factor: f32,
        #[serde(default)]
        offset: f32,
    },
    /// Fit one dimension, preserving the other two dimensions.
    FitAxis {
        axis: usize,
    },
    /// Uniformly fit a dimension, preserving existing character proportions.
    FitSize {
        axis: usize,
    },
    Affine {
        parts: Vec<String>,
        joints: Vec<String>,
        pivot: [f32; 3],
        scale: [f32; 3],
        translation: [f32; 3],
        #[serde(default, skip_serializing_if = "Option::is_none")]
        weight: Option<Ramp>,
        #[serde(default)]
        frames: bool,
    },
    Palette {
        parts: Vec<String>,
    },
}
fn unit_factor() -> f32 {
    1.
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ramp {
    pub axis: usize,
    pub from: f32,
    pub to: f32,
}
fn matches(ids: &[String], id: &str) -> bool {
    ids.iter().any(|s| s == "*" || s == id)
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
impl Modifier {
    fn map(
        &self,
        p: [f32; 3],
        normal: Option<[f32; 3]>,
        amount: f32,
    ) -> Result<([f32; 3], Option<[f32; 3]>)> {
        let Self::Affine {
            pivot,
            scale,
            translation,
            weight,
            ..
        } = self
        else {
            return Ok((p, normal));
        };
        let (w, gradient) = if let Some(r) = weight {
            let t = (p[r.axis] - r.from) / (r.to - r.from);
            let mut gradient = [0.; 3];
            if t > 0. && t < 1. {
                gradient[r.axis] = 1. / (r.to - r.from);
            }
            (t.clamp(0., 1.), gradient)
        } else {
            (1., [0.; 3])
        };
        let delta: [f32; 3] =
            std::array::from_fn(|k| scale[k] * (p[k] - pivot[k]) + translation[k]);
        let out = std::array::from_fn(|k| p[k] + amount * w * delta[k]);
        let n = if let Some(n) = normal {
            let columns: [[f32; 3]; 3] = std::array::from_fn(|j| {
                std::array::from_fn(|i| {
                    if i == j {
                        1. + amount * w * scale[i] + amount * delta[i] * gradient[j]
                    } else {
                        amount * delta[i] * gradient[j]
                    }
                })
            });
            let cofactors = [
                cross(columns[1], columns[2]),
                cross(columns[2], columns[0]),
                cross(columns[0], columns[1]),
            ];
            let determinant = (0..3).map(|k| columns[0][k] * cofactors[0][k]).sum::<f32>();
            if !determinant.is_finite() || determinant < 0.00001 {
                return Err(fail("Group control would fold or flatten the geometry"));
            }
            let n: [f32; 3] = std::array::from_fn(|k| (0..3).map(|j| n[j] * cofactors[j][k]).sum());
            let length = n.iter().map(|v| v * v).sum::<f32>().sqrt();
            if length < 1e-8 {
                return Err(fail("Group control produced a degenerate normal"));
            }
            Some(n.map(|x| x / length))
        } else {
            None
        };
        Ok((out, n))
    }
}
pub fn validate(controls: &[Control]) -> Result<()> {
    if controls.len() > 64 {
        return Err(fail("A group supports at most 64 controls"));
    }
    let mut ids = HashSet::new();
    for control in controls {
        finite(
            &[control.value, control.reference, control.min, control.max],
            1000.,
        )?;
        if control.id.is_empty()
            || !ids.insert(&control.id)
            || control.min > control.max
            || (control.expression.is_none()
                && (control.value < control.min || control.value > control.max))
            || control.reference < control.min
            || control.reference > control.max
            || control.modifiers.len() > 64
        {
            return Err(fail(
                "Group controls require unique IDs, valid ranges and at most 64 modifiers",
            ));
        }
        if control.is_palette() && control.expression.is_some() {
            return Err(fail(
                "Formulas require numeric controls, not palette controls",
            ));
        }
        if control.is_palette()
            && (control.value.fract() != 0. || control.min < 0. || control.max > 255.)
        {
            return Err(fail("Group colors require project palette indices"));
        }
        if control.is_integer()
            && (control.expression.is_none() && control.value.fract() != 0.
                || [control.reference, control.min, control.max]
                    .iter()
                    .any(|v| v.fract() != 0.))
        {
            return Err(fail(
                "Count and segment controls require integer values and bounds",
            ));
        }
    }
    Ok(())
}
pub fn apply(mesh: &mut Mesh, controls: &[Control]) -> Result<()> {
    let controls = super::expressions::resolve(controls)?;
    apply_resolved(mesh, &controls)
}
pub(super) fn apply_resolved(mesh: &mut Mesh, controls: &[Control]) -> Result<()> {
    let mut poses = mesh.rig.as_ref().map(rig::Rig::rest).unwrap_or_default();
    let mut sockets = mesh
        .attachments
        .iter()
        .map(|a| poses[a.joint].compose(rig::Pose::from_frame(&a.socket)))
        .collect::<Vec<_>>();
    for control in controls {
        for modifier in &control.modifiers {
            if matches!(modifier, Modifier::Parameter { .. }) {
                continue;
            }
            let fitted;
            let amount;
            let modifier = if let Modifier::FitSize { axis } | Modifier::FitAxis { axis } = modifier
            {
                if *axis > 2 || control.min <= 0. {
                    return Err(fail("Size control needs a valid axis and positive size"));
                }
                let (lo, hi) = mesh.bounds();
                let length = hi[*axis] - lo[*axis];
                if length < 0.0001 {
                    return Err(fail("Cannot fit an empty geometry dimension"));
                }
                let mut pivot = [0.; 3];
                pivot[*axis] = lo[*axis];
                let factor = control.value / length - 1.;
                let mut scale = [factor; 3];
                if matches!(modifier, Modifier::FitAxis { .. }) {
                    scale = [0.; 3];
                    scale[*axis] = factor;
                }
                fitted = Modifier::Affine {
                    parts: vec!["*".into()],
                    joints: if mesh.rig.is_some() {
                        vec!["*".into()]
                    } else {
                        vec![]
                    },
                    pivot,
                    scale,
                    translation: [0.; 3],
                    weight: None,
                    frames: true,
                };
                amount = 1.;
                &fitted
            } else {
                amount = control.value - control.reference;
                modifier
            };
            let parts = match modifier {
                Modifier::Affine { parts, .. } | Modifier::Palette { parts } => parts,
                Modifier::FitSize { .. }
                | Modifier::FitAxis { .. }
                | Modifier::Parameter { .. } => unreachable!(),
            };
            if parts.len() > 128 {
                return Err(fail("Too many group part targets"));
            }
            for id in parts {
                if id != "*" && !mesh.parts.iter().any(|p| &p.id == id) {
                    return Err(fail(format!("Unknown group part {id}")));
                }
            }
            match modifier {
                Modifier::FitSize { .. }
                | Modifier::FitAxis { .. }
                | Modifier::Parameter { .. } => unreachable!(),
                Modifier::Palette { parts } => {
                    if !control.is_palette() {
                        return Err(fail("Palette and deformation controls must be separate"));
                    }
                    for part in &mut mesh.parts {
                        if matches(parts, &part.id) {
                            part.surface
                                .as_mut()
                                .ok_or_else(|| fail("Group palette target requires a material"))?
                                .palette = control.value as u16;
                        }
                    }
                }
                Modifier::Affine {
                    parts,
                    joints,
                    pivot,
                    scale,
                    translation,
                    weight,
                    frames,
                } => {
                    if joints.len() > 64 {
                        return Err(fail("Too many group joint targets"));
                    }
                    finite(pivot, 1000.)?;
                    finite(scale, 100.)?;
                    finite(translation, 1000.)?;
                    if let Some(w) = weight {
                        finite(&[w.from, w.to], 1000.)?;
                        if w.axis > 2 || (w.to - w.from).abs() < 1e-5 {
                            return Err(fail(
                                "Group weighting needs a valid axis and two distinct bounds",
                            ));
                        }
                    }
                    for id in joints {
                        if id != "*"
                            && !mesh
                                .rig
                                .as_ref()
                                .is_some_and(|r| r.joints.iter().any(|j| &j.id == id))
                        {
                            return Err(fail(format!("Unknown group joint {id}")));
                        }
                    }
                    if amount == 0. {
                        continue;
                    }
                    if parts.iter().any(|id| id == "*") {
                        for connector in &mut mesh.connectors {
                            let (p, n) = modifier.map(
                                connector.position,
                                Some(connector.direction),
                                amount,
                            )?;
                            connector.position = p;
                            connector.direction = n.unwrap();
                        }
                        if let Some(points) = &mut mesh.footprint {
                            for p in points.points_mut() {
                                *p = modifier.map(*p, None, amount)?.0;
                            }
                        }
                    }
                    for part in &mut mesh.parts {
                        if matches(parts, &part.id) {
                            for v in &mut part.vertices {
                                let (p, n) = modifier.map(v.position, Some(v.normal), amount)?;
                                v.position = p;
                                v.normal = n.unwrap();
                            }
                        }
                    }
                    if let Some(r) = &mesh.rig {
                        for (i, joint) in r.joints.iter().enumerate() {
                            if matches(joints, &joint.id) {
                                poses[i].translation =
                                    modifier.map(poses[i].translation, None, amount)?.0;
                            }
                        }
                        for (i, a) in mesh.attachments.iter().enumerate() {
                            if matches(joints, &r.joints[a.joint].id) {
                                sockets[i].translation =
                                    modifier.map(sockets[i].translation, None, amount)?.0;
                            }
                        }
                    }
                    if *frames {
                        for effect in &mut mesh.effects {
                            effect.definition.position =
                                modifier.map(effect.definition.position, None, amount)?.0;
                        }
                        if let Some(grip) = &mut mesh.grip {
                            grip.translation = modifier.map(grip.translation, None, amount)?.0;
                        }
                    }
                }
            }
        }
    }
    if let Some(r) = &mut mesh.rig {
        for (i, j) in r.joints.iter_mut().enumerate() {
            j.local = j
                .parent
                .map_or(poses[i], |p| poses[p].inverse().compose(poses[i]));
        }
        for (i, a) in mesh.attachments.iter_mut().enumerate() {
            a.socket.translation = poses[a.joint].inverse().point(sockets[i].translation);
        }
    }
    Ok(())
}

impl Document {
    pub fn group_document(&self, path: &[String]) -> Result<&Document> {
        let mut doc = self;
        for id in path {
            let Operation::Group { document, .. } = &doc
                .rows
                .iter()
                .flat_map(|r| &r.tiles)
                .find(|t| &t.id == id)
                .ok_or_else(|| fail("Group no longer exists"))?
                .op
            else {
                return Err(fail("Selected operation is not a group"));
            };
            doc = document;
        }
        Ok(doc)
    }
    pub fn group_document_mut(&mut self, path: &[String]) -> Result<&mut Document> {
        let mut doc = self;
        for id in path {
            let Operation::Group { document, .. } = &mut doc
                .rows
                .iter_mut()
                .flat_map(|r| &mut r.tiles)
                .find(|t| &t.id == id)
                .ok_or_else(|| fail("Group no longer exists"))?
                .op
            else {
                return Err(fail("Selected operation is not a group"));
            };
            doc = document;
        }
        Ok(doc)
    }
}
