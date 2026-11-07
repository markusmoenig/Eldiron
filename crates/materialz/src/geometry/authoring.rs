//! Editable joint trees and profiles with explicit bind frames. Shared by every creature.
use super::*;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointDef {
    pub id: String,
    pub label: String,
    pub parent: Option<String>,
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub joint: String,
    pub frame: Frame,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilePart {
    pub id: String,
    pub label: String,
    pub sections: Vec<Section>,
    pub segments: u32,
    pub anchors: Vec<Anchor>,
    pub blend: f32,
    pub surface: Surface,
}
pub fn skeleton(defs: &[JointDef]) -> Result<rig::Rig> {
    if defs.is_empty() || defs.len() > 64 || defs.iter().filter(|j| j.parent.is_none()).count() != 1
    {
        return Err(fail("A skeleton needs one root and 1–64 joints"));
    }
    let mut ids = HashSet::new();
    for j in defs {
        if j.id.is_empty() || !ids.insert(&j.id) {
            return Err(fail("Joint identities must be unique"));
        }
        finite(&j.translation, 1000.)?;
        finite(&j.rotation, 3600.)?;
    }
    let mut joints: Vec<rig::Joint> = vec![];
    while joints.len() < defs.len() {
        let before = joints.len();
        for d in defs {
            if joints.iter().any(|j| j.id == d.id) {
                continue;
            }
            let parent = match &d.parent {
                None => None,
                Some(id) => {
                    let Some(i) = joints.iter().position(|j| &j.id == id) else {
                        continue;
                    };
                    Some(i)
                }
            };
            joints.push(rig::Joint {
                id: d.id.clone(),
                label: d.label.clone(),
                parent,
                local: rig::Pose::from_frame(&Frame {
                    translation: d.translation,
                    rotation: d.rotation,
                }),
            });
        }
        if before == joints.len() {
            return Err(fail("Skeleton contains a cycle or missing parent"));
        }
    }
    Ok(rig::Rig { joints, tip: None })
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|k| a[k] - b[k])
}
fn normalized(v: [f32; 3]) -> [f32; 3] {
    let n = dot(v, v).sqrt().max(1e-8);
    v.map(|v| v / n)
}
fn rotate_between(p: [f32; 3], from: [f32; 3], to: [f32; 3]) -> [f32; 3] {
    let a = normalized(from);
    let b = normalized(to);
    let cosine = dot(a, b).clamp(-1., 1.);
    if cosine > 0.999999 {
        return p;
    }
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let axis = if cosine < -0.99999 {
        normalized(cross(
            a,
            if a[0].abs() < 0.8 {
                [1., 0., 0.]
            } else {
                [0., 0., 1.]
            },
        ))
    } else {
        normalized(cross(a, b))
    };
    let sine = (1. - cosine * cosine).sqrt();
    let c = cross(axis, p);
    std::array::from_fn(|k| p[k] * cosine + c[k] * sine + axis[k] * dot(axis, p) * (1. - cosine))
}
struct BindingFrame {
    index: usize,
    old: rig::Pose,
    current: rig::Pose,
    ends: Option<([f32; 3], [f32; 3], [f32; 3], f32)>,
}
impl ProfilePart {
    fn weights(&self, p: [f32; 3]) -> Vec<(usize, f32)> {
        if self.anchors.len() == 1 {
            return vec![(0, 1.)];
        }
        let mut best = f32::INFINITY;
        let mut result = vec![];
        for (i, a) in self.anchors.windows(2).enumerate() {
            let start = a[0].frame.translation;
            let d = sub(a[1].frame.translation, start);
            let length = dot(d, d);
            if length < 1e-8 {
                continue;
            }
            let t = (dot(sub(p, start), d) / length).clamp(0., 1.);
            let distance = dot(
                sub(p, std::array::from_fn(|k| start[k] + t * d[k])),
                sub(p, std::array::from_fn(|k| start[k] + t * d[k])),
            );
            if distance < best {
                best = distance;
                let w = ((t - (1. - self.blend)) / self.blend).clamp(0., 1.);
                let w = w * w * (3. - 2. * w);
                result = vec![(i, 1. - w), (i + 1, w)];
            }
        }
        result
    }
    fn prepare(&self, rig: &rig::Rig) -> Result<Vec<BindingFrame>> {
        let rest = rig.rest();
        self.anchors
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let index = rig.index(&a.joint)?;
                let old = rig::Pose::from_frame(&a.frame).inverse();
                let current = rest[index];
                let ends = if let Some(end) = self.anchors.get(i + 1) {
                    let before = old.point(end.frame.translation);
                    let after = current
                        .inverse()
                        .point(rest[rig.index(&end.joint)?].translation);
                    let length = dot(before, before).sqrt();
                    let target = dot(after, after).sqrt();
                    if length < 1e-5 || target < 1e-5 {
                        return Err(fail("Profile bones need nonzero length"));
                    }
                    Some((before, after, before.map(|x| x / length), target / length))
                } else if i > 0 {
                    let previous = &self.anchors[i - 1];
                    let before = old.vector(sub(a.frame.translation, previous.frame.translation));
                    let after = current.inverse().vector(sub(
                        rest[index].translation,
                        rest[rig.index(&previous.joint)?].translation,
                    ));
                    let length = dot(before, before).sqrt();
                    if length < 1e-5 || dot(after, after) < 1e-10 {
                        return Err(fail("Profile bones need nonzero length"));
                    }
                    Some((before, after, before.map(|v| v / length), 1.))
                } else {
                    None
                };
                Ok(BindingFrame {
                    index,
                    old,
                    current,
                    ends,
                })
            })
            .collect()
    }
    pub fn map(
        &self,
        rig: &rig::Rig,
        p: [f32; 3],
        n: [f32; 3],
    ) -> Result<([f32; 3], [f32; 3], rig::Weights)> {
        self.map_prepared(&self.prepare(rig)?, p, n)
    }
    fn map_prepared(
        &self,
        frames: &[BindingFrame],
        p: [f32; 3],
        n: [f32; 3],
    ) -> Result<([f32; 3], [f32; 3], rig::Weights)> {
        let mut position = [0.; 3];
        let mut normal = [0.; 3];
        let mut weights = rig::Weights {
            joints: [0; 4],
            weights: [0.; 4],
        };
        for (slot, (i, w)) in self.weights(p).into_iter().enumerate() {
            let f = &frames[i];
            let mut local = f.old.point(p);
            let mut ln = f.old.vector(n);
            if let Some((before, after, axis, scale)) = f.ends {
                let projection = dot(local, axis);
                let np = dot(ln, axis);
                local = std::array::from_fn(|k| local[k] + axis[k] * projection * (scale - 1.));
                ln = std::array::from_fn(|k| ln[k] + axis[k] * np * (1. / scale - 1.));
                local = rotate_between(local, before, after);
                ln = rotate_between(ln, before, after);
            }
            let out = f.current.point(local);
            let out_n = f.current.vector(ln);
            for k in 0..3 {
                position[k] += w * out[k];
                normal[k] += w * out_n[k];
            }
            weights.joints[slot] = f.index as u16;
            weights.weights[slot] = w;
        }
        if weights.weights.iter().sum::<f32>() < 0.99 {
            return Err(fail("Profile has no valid binding segment"));
        }
        Ok((position, normalized(normal), weights))
    }
}
pub fn profiles(owner: &str, rig: &rig::Rig, parts: &[ProfilePart]) -> Result<Mesh> {
    if parts.len() > 128 {
        return Err(fail("At most 128 profile parts"));
    }
    let mut ids = HashSet::new();
    let mut mesh = Mesh {
        rig: Some(rig.clone()),
        ..Default::default()
    };
    for p in parts {
        if p.id.is_empty()
            || !ids.insert(&p.id)
            || p.anchors.is_empty()
            || p.anchors.len() > 4
            || !p.blend.is_finite()
            || !(0.01..=1.).contains(&p.blend)
        {
            return Err(fail(
                "Profiles need unique IDs, 1–4 anchors and a valid blend",
            ));
        }
        for a in &p.anchors {
            rig.index(&a.joint)?;
            finite(&a.frame.translation, 1000.)?;
            finite(&a.frame.rotation, 3600.)?;
        }
        if p.surface.palette > 255
            || !(0.0..=1.).contains(&p.surface.roughness)
            || !(0.0..=1.).contains(&p.surface.metallic)
        {
            return Err(fail("Invalid profile surface"));
        }
        let mut m = super::loft::build(&p.id, &p.sections, p.segments)?;
        let frames = p.prepare(rig)?;
        for part in &mut m.parts {
            part.label = p.label.clone();
            part.surface = Some(p.surface.clone());
            for v in &mut part.vertices {
                let (out, n, w) = p.map_prepared(&frames, v.position, v.normal)?;
                v.position = out;
                v.normal = n;
                v.skin = Some(w);
            }
        }
        mesh.parts.append(&mut m.parts);
        if mesh.vertex_count() > 200_000 {
            return Err(fail(format!("{owner}: profile vertex limit exceeded")));
        }
    }
    Ok(mesh)
}
pub fn set_world(defs: &mut [JointDef], id: &str, position: [f32; 3]) -> Result<()> {
    let rig = skeleton(defs)?;
    let rest = rig.rest();
    let index = rig.index(id)?;
    let local = rig.joints[index]
        .parent
        .map_or(position, |i| rest[i].inverse().point(position));
    defs.iter_mut().find(|j| j.id == id).unwrap().translation = local;
    Ok(())
}
pub fn reparent(defs: &mut [JointDef], id: &str, parent: Option<String>) -> Result<()> {
    let rig = skeleton(defs)?;
    let old = rig.rest()[rig.index(id)?];
    let p = parent
        .as_ref()
        .map(|id| rig.index(id).map(|i| rig.rest()[i]))
        .transpose()?;
    let local = p.map_or(old, |p| p.inverse().compose(old));
    let d = defs
        .iter_mut()
        .find(|j| j.id == id)
        .ok_or_else(|| fail("Missing joint"))?;
    d.parent = parent;
    d.translation = local.translation;
    d.rotation = local.frame().rotation;
    skeleton(defs)?;
    Ok(())
}
