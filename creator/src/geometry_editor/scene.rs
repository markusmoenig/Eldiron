//! Viewport authoring; all mutations are portable geometry data, not generator settings.
use super::*;
use denrim_materials::geometry::{
    authoring::*,
    rig::{Key, Pose, Track},
};
#[derive(Clone, Debug)]
pub struct Projection {
    pub bounds: ([f32; 3], [f32; 3]),
    center: [f32; 3],
    extent: f32,
    aspect: f32,
    size: [f32; 2],
    yaw: f32,
    pitch: f32,
}
impl Projection {
    pub fn new(
        bounds: ([f32; 3], [f32; 3]),
        width: i32,
        height: i32,
        yaw: f32,
        pitch: f32,
    ) -> Self {
        let (a, b) = bounds;
        Self {
            bounds,
            center: std::array::from_fn(|k| (a[k] + b[k]) / 2.),
            extent: (0..3)
                .map(|k| (b[k] - a[k]).powi(2))
                .sum::<f32>()
                .sqrt()
                .max(0.01)
                * 0.58,
            aspect: width.max(1) as f32 / height.max(1) as f32,
            size: [width as f32, height as f32],
            yaw,
            pitch,
        }
    }
    fn rotate(&self, p: [f32; 3], inverse: bool) -> [f32; 3] {
        let (sy, cy) = self.yaw.to_radians().sin_cos();
        let (sp, cp) = self.pitch.to_radians().sin_cos();
        if inverse {
            let y = cp * p[1] + sp * p[2];
            let z = -sp * p[1] + cp * p[2];
            [cy * p[0] - sy * z, y, sy * p[0] + cy * z]
        } else {
            let x = cy * p[0] + sy * p[2];
            let z = -sy * p[0] + cy * p[2];
            [x, cp * p[1] - sp * z, sp * p[1] + cp * z]
        }
    }
    pub fn project(&self, p: [f32; 3]) -> [f32; 2] {
        let p = self.rotate(std::array::from_fn(|k| p[k] - self.center[k]), false);
        [
            (p[0] / (self.extent * self.aspect.max(1.)) * 0.5 + 0.5) * self.size[0],
            (0.5 - p[1] / (self.extent / self.aspect.min(1.)) * 0.5) * self.size[1],
        ]
    }
    pub fn delta(&self, d: [f32; 2]) -> [f32; 3] {
        self.rotate(
            [
                d[0] / self.size[0] * 2. * self.extent * self.aspect.max(1.),
                -d[1] / self.size[1] * 2. * self.extent / self.aspect.min(1.),
                0.,
            ],
            true,
        )
    }
}
#[derive(Clone, Debug)]
pub enum Handle {
    Joint(String),
    Profile(usize, usize, Option<usize>),
    Socket(String),
    Grip(String),
}
#[derive(Clone)]
pub struct Drag {
    pub handle: Handle,
    pub start: Vec2<i32>,
    pub source: String,
    pub point: [f32; 3],
    pub frame: Projection,
}
#[derive(Default)]
pub struct Scene {
    pub select_after_refresh: Option<String>,
    pub tool: usize,
    pub drag: Option<Drag>,
    pub projection: Option<Projection>,
    pub shift: bool,
    pub alt: bool,
    pub view_bounds: Option<([f32; 3], [f32; 3])>,
    pub pending_clip: Option<String>,
    pub posed: Vec<Pose>,
    pub previewing_animation: bool,
}
pub fn find_path(doc: &Document, predicate: fn(&Operation) -> bool) -> Option<Vec<String>> {
    if doc
        .rows
        .iter()
        .flat_map(|r| &r.tiles)
        .any(|t| predicate(&t.op))
    {
        return Some(vec![]);
    }
    for tile in doc.rows.iter().flat_map(|r| &r.tiles) {
        if let Operation::Group { document, .. } = &tile.op {
            if let Some(mut path) = find_path(document, predicate) {
                path.insert(0, tile.id.clone());
                return Some(path);
            }
        }
    }
    None
}
pub fn armature(doc: &Document) -> Option<(&str, &[JointDef])> {
    doc.rows.iter().flat_map(|r| &r.tiles).find_map(|t| {
        if let Operation::Armature { joints } = &t.op {
            Some((t.id.as_str(), joints.as_slice()))
        } else {
            None
        }
    })
}
pub fn profiles(doc: &Document) -> Option<(&str, &str, &[ProfilePart])> {
    doc.rows.iter().flat_map(|r| &r.tiles).find_map(|t| {
        if let Operation::Profiles { rig, parts } = &t.op {
            Some((t.id.as_str(), rig.as_str(), parts.as_slice()))
        } else {
            None
        }
    })
}
pub fn armature_mut(doc: &mut Document) -> Result<&mut Vec<JointDef>, String> {
    doc.rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find_map(|t| {
            if let Operation::Armature { joints } = &mut t.op {
                Some(joints)
            } else {
                None
            }
        })
        .ok_or("Select a skeleton resource".into())
}
pub fn parts_mut(doc: &mut Document) -> Result<&mut Vec<ProfilePart>, String> {
    doc.rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find_map(|t| {
            if let Operation::Profiles { parts, .. } = &mut t.op {
                Some(parts)
            } else {
                None
            }
        })
        .ok_or("Select a profile resource".into())
}
pub fn new_document() -> Document {
    Document {
        version: 1,
        id: Uuid::new_v4().to_string(),
        label: "Model".into(),
        rows: vec![
            Row {
                id: "rig".into(),
                label: "Skeleton".into(),
                tiles: vec![Tile {
                    id: "armature".into(),
                    op: Operation::Armature {
                        joints: vec![JointDef {
                            id: "root".into(),
                            label: "Root".into(),
                            parent: None,
                            translation: [0., 0.5, 0.],
                            rotation: [0.; 3],
                        }],
                    },
                }],
            },
            Row {
                id: "body".into(),
                label: "Profiles".into(),
                tiles: vec![Tile {
                    id: "profiles".into(),
                    op: Operation::Profiles {
                        rig: "rig".into(),
                        parts: vec![],
                    },
                }],
            },
        ],
        output: "body".into(),
        grip: None,
    }
}
pub fn add_joint(doc: &mut Document, parent: &str, position: [f32; 3]) -> Result<String, String> {
    let defs = armature_mut(doc)?;
    let rig = denrim_materials::geometry::authoring::skeleton(defs).map_err(|e| e.to_string())?;
    let index = rig.index(parent).map_err(|e| e.to_string())?;
    let rest = rig.rest();
    if (0..3)
        .map(|k| (position[k] - rest[index].translation[k]).powi(2))
        .sum::<f32>()
        < 0.0001
    {
        return Err("Place the new joint away from its parent".into());
    }
    let id = Uuid::new_v4().to_string();
    defs.push(JointDef {
        id: id.clone(),
        label: format!("Joint {}", defs.len() + 1),
        parent: Some(parent.into()),
        translation: rest[index].inverse().point(position),
        rotation: [0.; 3],
    });
    add_profile(doc, parent, Some(&id))?;
    Ok(id)
}
pub fn add_profile(doc: &mut Document, joint: &str, end: Option<&str>) -> Result<usize, String> {
    add_profile_with(doc, joint, end, false)
}
pub fn add_rigid_profile(doc: &mut Document, joint: &str) -> Result<usize, String> {
    add_profile_with(doc, joint, None, true)
}
fn add_profile_with(
    doc: &mut Document,
    joint: &str,
    end: Option<&str>,
    rigid: bool,
) -> Result<usize, String> {
    let defs = armature(doc).ok_or("No skeleton")?.1;
    let rig = denrim_materials::geometry::authoring::skeleton(defs).map_err(|e| e.to_string())?;
    let rest = rig.rest();
    let i = rig.index(joint).map_err(|e| e.to_string())?;
    let end = if rigid {
        None
    } else {
        end.map(str::to_string).or_else(|| {
            rig.joints
                .iter()
                .find(|j| j.parent == Some(i))
                .map(|j| j.id.clone())
        })
    };
    let length = end
        .as_ref()
        .map(|id| {
            rig.index(id).map(|j| {
                (0..3)
                    .map(|k| (rest[j].translation[k] - rest[i].translation[k]).powi(2))
                    .sum::<f32>()
                    .sqrt()
            })
        })
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or(0.2)
        .max(0.03);
    let anchors = if let Some(end) = end {
        vec![
            Anchor {
                joint: joint.into(),
                frame: denrim_materials::geometry::Frame {
                    translation: [0.; 3],
                    rotation: [0.; 3],
                },
            },
            Anchor {
                joint: end,
                frame: denrim_materials::geometry::Frame {
                    translation: [0., length, 0.],
                    rotation: [0.; 3],
                },
            },
        ]
    } else {
        vec![Anchor {
            joint: joint.into(),
            frame: rest[i].frame(),
        }]
    };
    let sections = if anchors.len() == 1 {
        vec![
            denrim_materials::geometry::Section {
                center: [
                    rest[i].translation[0],
                    rest[i].translation[1] - 0.08,
                    rest[i].translation[2],
                ],
                radius: [0.035, 0.035],
            },
            denrim_materials::geometry::Section {
                center: [
                    rest[i].translation[0],
                    rest[i].translation[1] + 0.08,
                    rest[i].translation[2],
                ],
                radius: [0.035, 0.035],
            },
        ]
    } else {
        [0., 0.15, 0.85, 1.]
            .into_iter()
            .map(|t| denrim_materials::geometry::Section {
                center: [0., length * t, 0.],
                radius: [if t == 0. || t == 1. { 0.025 } else { 0.045 }; 2],
            })
            .collect()
    };
    let parts = parts_mut(doc)?;
    parts.push(ProfilePart {
        id: Uuid::new_v4().to_string(),
        label: format!("Part {}", parts.len() + 1),
        sections,
        segments: 12,
        anchors,
        blend: 0.35,
        surface: denrim_materials::geometry::Surface {
            palette: 22,
            roughness: 0.8,
            metallic: 0.,
        },
    });
    Ok(parts.len() - 1)
}
pub fn mirror(doc: &mut Document, joint: &str) -> Result<String, String> {
    let defs = armature(doc).ok_or("No skeleton")?.1.to_vec();
    let rig = denrim_materials::geometry::authoring::skeleton(&defs).map_err(|e| e.to_string())?;
    let rest = rig.rest();
    let mut ids = std::collections::HashSet::from([joint.to_string()]);
    loop {
        let n = ids.len();
        for d in &defs {
            if d.parent.as_ref().is_some_and(|p| ids.contains(p)) {
                ids.insert(d.id.clone());
            }
        }
        if n == ids.len() {
            break;
        }
    }
    let mapping = ids
        .iter()
        .map(|id| (id.clone(), Uuid::new_v4().to_string()))
        .collect::<std::collections::HashMap<_, _>>();
    let mut result = defs.clone();
    for d in &defs {
        if !ids.contains(&d.id) {
            continue;
        }
        if d.parent.is_none() {
            return Err("Select a branch below the root to mirror".into());
        }
        let p = rest[rig.index(&d.id).map_err(|e| e.to_string())?];
        let mut world = p.frame();
        world.translation[0] *= -1.;
        world.rotation[1] *= -1.;
        world.rotation[2] *= -1.;
        let parent = d
            .parent
            .as_ref()
            .map(|p| mapping.get(p).unwrap_or(p).clone());
        let parent_world = if let Some(original) = d.parent.as_ref() {
            let mut p = rest[rig.index(original).map_err(|e| e.to_string())?].frame();
            if ids.contains(original) {
                p.translation[0] *= -1.;
                p.rotation[1] *= -1.;
                p.rotation[2] *= -1.;
            }
            Some(Pose::from_frame(&p))
        } else {
            None
        };
        let local = parent_world
            .map_or(Pose::from_frame(&world), |p| {
                p.inverse().compose(Pose::from_frame(&world))
            })
            .frame();
        result.push(JointDef {
            id: mapping[&d.id].clone(),
            label: format!("{} mirrored", d.label),
            parent,
            translation: local.translation,
            rotation: local.rotation,
        });
    }
    denrim_materials::geometry::authoring::skeleton(&result).map_err(|e| e.to_string())?;
    *armature_mut(doc)? = result;
    let mut ancestors = std::collections::HashSet::new();
    let mut parent = defs
        .iter()
        .find(|d| d.id == joint)
        .and_then(|d| d.parent.clone());
    while let Some(id) = parent {
        ancestors.insert(id.clone());
        parent = defs
            .iter()
            .find(|d| d.id == id)
            .and_then(|d| d.parent.clone());
    }
    let copies = profiles(doc)
        .map(|(_, _, parts)| {
            parts
                .iter()
                .filter(|p| {
                    p.anchors.iter().any(|a| ids.contains(&a.joint))
                        && p.anchors
                            .iter()
                            .all(|a| ids.contains(&a.joint) || ancestors.contains(&a.joint))
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for mut p in copies {
        p.id = Uuid::new_v4().to_string();
        p.label.push_str(" mirrored");
        for s in &mut p.sections {
            s.center[0] *= -1.;
        }
        for a in &mut p.anchors {
            a.joint = mapping.get(&a.joint).unwrap_or(&a.joint).clone();
            a.frame.translation[0] *= -1.;
            a.frame.rotation[1] *= -1.;
            a.frame.rotation[2] *= -1.;
        }
        parts_mut(doc)?.push(p);
    }
    Ok(mapping[joint].clone())
}
pub fn motion_key(
    doc: &mut Document,
    row: &str,
    joint: &str,
    phase: f32,
    delta: [f32; 3],
) -> Result<(), String> {
    let interpolated =
        if let Some(Value::Motion(m)) = doc.evaluate().map_err(|e| e.to_string())?.rows.get(row) {
            let index = m.rig.index(joint).map_err(|e| e.to_string())?;
            let pose = m
                .rig
                .sample(Some(m), phase * m.duration)
                .map_err(|e| e.to_string())?;
            let local = m.rig.joints[index]
                .parent
                .map_or(pose[index], |p| pose[p].inverse().compose(pose[index]));
            Some(m.rig.joints[index].local.inverse().compose(local).frame())
        } else {
            None
        };
    let op = doc
        .rows
        .iter_mut()
        .find(|r| r.id == row)
        .and_then(|r| {
            r.tiles
                .iter_mut()
                .find(|t| matches!(t.op, Operation::Motion { .. }))
        })
        .ok_or("Choose or create a motion clip")?;
    let Operation::Motion {
        tracks, looping, ..
    } = &mut op.op
    else {
        unreachable!()
    };
    if !tracks.iter().any(|t| t.joint == joint) {
        tracks.push(Track {
            joint: joint.into(),
            keys: vec![
                Key {
                    visible: true,
                    time: 0.,
                    translation: [0.; 3],
                    rotation: [0.; 3],
                },
                Key {
                    visible: true,
                    time: 1.,
                    translation: [0.; 3],
                    rotation: [0.; 3],
                },
            ],
        });
    }
    let track = tracks.iter_mut().find(|t| t.joint == joint).unwrap();
    let at = track
        .keys
        .iter()
        .position(|k| (k.time - phase).abs() < 0.0001);
    if let Some(at) = at {
        for k in 0..3 {
            track.keys[at].rotation[k] += delta[k];
        }
    } else {
        if track.keys.len() >= 64 {
            return Err("A track supports at most 64 keys".into());
        }
        let before = track
            .keys
            .iter()
            .rev()
            .find(|k| k.time < phase)
            .unwrap_or(&track.keys[0])
            .clone();
        track.keys.push(Key {
            visible: before.visible,
            time: phase,
            translation: interpolated
                .as_ref()
                .map_or(before.translation, |p| p.translation),
            rotation: std::array::from_fn(|k| {
                interpolated
                    .as_ref()
                    .map_or(before.rotation[k], |p| p.rotation[k])
                    + delta[k]
            }),
        });
        track.keys.sort_by(|a, b| a.time.total_cmp(&b.time));
    }
    if *looping && (phase < 0.0001 || phase > 0.9999) {
        let a = if phase < 0.5 {
            track.keys[0].clone()
        } else {
            track.keys.last().unwrap().clone()
        };
        track.keys[0].rotation = a.rotation;
        track.keys.last_mut().unwrap().rotation = a.rotation;
    }
    Ok(())
}
impl Scene {
    pub fn handles(
        &self,
        doc: &Document,
        e: &Evaluation,
        selection: &rigging::Selection,
    ) -> Vec<(Handle, [f32; 3], String)> {
        let mut out = vec![];
        if self.tool == 5 {
            for t in doc.rows.iter().flat_map(|r| &r.tiles) {
                if let Operation::Grip { translation, .. } = &t.op {
                    out.push((Handle::Grip(t.id.clone()), *translation, t.id.clone()));
                }
            }
        }
        if self.tool == 5 {
            for t in doc.rows.iter().flat_map(|r| &r.tiles) {
                if let Operation::ParticleEffect { effect } = &t.op {
                    out.push((Handle::Grip(t.id.clone()), effect.position, t.id.clone()));
                }
            }
        }
        let Some(rig) = e.mesh.rig.as_ref() else {
            return out;
        };
        if matches!(self.tool, 1 | 2 | 4) {
            let poses = if self.posed.len() == rig.joints.len() {
                self.posed.clone()
            } else {
                rig.rest()
            };
            for (i, j) in rig.joints.iter().enumerate() {
                out.push((
                    Handle::Joint(j.id.clone()),
                    poses[i].translation,
                    j.id.clone(),
                ));
            }
        }
        if self.tool == 3 && !self.previewing_animation {
            if let Some((_, _, parts)) = profiles(doc) {
                if let Some(p) = parts.get(selection.part.min(parts.len().saturating_sub(1))) {
                    for (i, s) in p.sections.iter().enumerate() {
                        for axis in [None, Some(0), Some(1)] {
                            let mut point = s.center;
                            if let Some(k) = axis {
                                point[if k == 0 { 0 } else { 2 }] += s.radius[k];
                            }
                            if let Ok((point, _, _)) = p.map(rig, point, [0., 1., 0.]) {
                                out.push((
                                    Handle::Profile(selection.part, i, axis),
                                    point,
                                    String::new(),
                                ));
                            }
                        }
                    }
                }
            }
        }
        if self.tool == 5 {
            for a in &e.mesh.attachments {
                let poses = if self.posed.len() == rig.joints.len() {
                    self.posed.clone()
                } else {
                    rig.rest()
                };
                let pose = poses[a.joint].compose(Pose::from_frame(&a.socket));
                out.push((Handle::Socket(a.id.clone()), pose.translation, a.id.clone()));
            }
        }
        out
    }
    pub fn hit(
        &self,
        doc: &Document,
        e: &Evaluation,
        selection: &rigging::Selection,
        p: Vec2<i32>,
    ) -> Option<(Handle, [f32; 3])> {
        let frame = self.projection.as_ref()?;
        self.handles(doc, e, selection)
            .into_iter()
            .filter_map(|(h, v, _)| {
                let q = frame.project(v);
                let d = (q[0] - p.x as f32).hypot(q[1] - p.y as f32);
                (d < 12.).then_some((h, v, d))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(h, p, _)| (h, p))
    }
    pub fn paint(
        &self,
        image: &mut TheRGBABuffer,
        doc: &Document,
        e: &Evaluation,
        selection: &rigging::Selection,
        theme: &dyn TheTheme,
    ) {
        let Some(frame) = &self.projection else {
            return;
        };
        if self.tool == 0 {
            return;
        }
        let (w, h) = (image.dim().width as usize, image.dim().height as usize);
        let pixels = image.pixels_mut();
        for (handle, p, _) in self.handles(doc, e, selection) {
            let q = frame.project(p);
            let active = match handle {
                Handle::Joint(id) => armature(doc)
                    .and_then(|(_, j)| j.get(selection.joint))
                    .map(|j| &j.id)
                    .or_else(|| {
                        e.mesh
                            .rig
                            .as_ref()
                            .and_then(|r| r.joints.get(selection.joint))
                            .map(|j| &j.id)
                    })
                    .is_some_and(|j| *j == id),
                Handle::Profile(_, i, _) => i == selection.section,
                Handle::Socket(_) | Handle::Grip(_) => true,
            };
            let color = *theme.color(if active {
                SelectedTextEditBorder1
            } else {
                MaterialText
            });
            for y in q[1] as i32 - 4..=q[1] as i32 + 4 {
                for x in q[0] as i32 - 4..=q[0] as i32 + 4 {
                    if x >= 0
                        && y >= 0
                        && (x as usize) < w
                        && (y as usize) < h
                        && (x as f32 - q[0]).hypot(y as f32 - q[1]) < 4.5
                    {
                        let at = (y as usize * w + x as usize) * 4;
                        pixels[at..at + 4].copy_from_slice(&color);
                    }
                }
            }
        }
    }
}
#[cfg(test)]
pub(super) fn quadruped() -> Document {
    let mut d = new_document();
    armature_mut(&mut d).unwrap()[0].translation = [0., 0.8, -0.30];
    d.label = "Creature built in the visual editor".into();
    fn add(d: &mut Document, parent: &str, p: [f32; 3], label: &str, radius: [f32; 2]) -> String {
        let id = add_joint(d, parent, p).unwrap();
        armature_mut(d).unwrap().last_mut().unwrap().label = label.into();
        let p = parts_mut(d).unwrap().last_mut().unwrap();
        p.label = label.into();
        p.surface.palette = 19;
        for (i, s) in p.sections.iter_mut().enumerate() {
            s.radius = radius.map(|r| r * if i == 0 || i == 3 { 0.85 } else { 1. });
        }
        id
    }
    let chest = add(&mut d, "root", [0., 0.82, 0.35], "Chest", [0.21, 0.22]);
    let neck = add(&mut d, &chest, [0., 1.02, 0.52], "Neck", [0.12, 0.13]);
    let head = add(&mut d, &neck, [0., 1.08, 0.80], "Head", [0.105, 0.12]);
    add(&mut d, &head, [0., 1.02, 0.96], "Muzzle", [0.07, 0.07]);
    let mut hips = vec![];
    for (parent, z, label) in [
        (&chest, 0.35, "Front leg"),
        (&"root".to_string(), -0.30, "Hind leg"),
    ] {
        let hip = add(&mut d, parent, [0.17, 0.76, z], label, [0.07, 0.08]);
        let knee = add(&mut d, &hip, [0.18, 0.43, z + 0.04], "Knee", [0.06, 0.065]);
        let foot = add(
            &mut d,
            &knee,
            [0.18, 0.105, z - 0.015],
            "Ankle",
            [0.04, 0.045],
        );
        let toe = add(
            &mut d,
            &foot,
            [0.18, 0.065, z + 0.15],
            "Paw",
            [0.045, 0.045],
        );
        parts_mut(&mut d)
            .unwrap()
            .last_mut()
            .unwrap()
            .surface
            .palette = 4;
        let mirrored = mirror(&mut d, &hip).unwrap();
        hips.push(hip);
        hips.push(mirrored);
        let _ = toe;
    }
    let tail = add(&mut d, "root", [0., 0.96, -0.63], "Tail", [0.065, 0.065]);
    add(&mut d, &tail, [0., 1.02, -0.87], "Tail tip", [0.035, 0.035]);
    let ear = add(&mut d, &head, [0.10, 1.28, 0.76], "Ear", [0.045, 0.05]);
    let p = parts_mut(&mut d).unwrap().last_mut().unwrap();
    for (i, s) in p.sections.iter_mut().enumerate() {
        s.radius = [0.055 * (1. - i as f32 / 3.) + 0.003; 2];
    }
    mirror(&mut d, &ear).unwrap();
    for sign in [-1., 1.] {
        for (palette, z, r) in [(9, 0.816, 0.015), (4, 0.83, 0.008)] {
            let i = add_rigid_profile(&mut d, &head).unwrap();
            let p = &mut parts_mut(&mut d).unwrap()[i];
            p.label = "Eye".into();
            p.surface.palette = palette;
            p.sections = vec![
                denrim_materials::geometry::Section {
                    center: [sign * 0.075, 1.12 - r, z],
                    radius: [r, 0.008],
                },
                denrim_materials::geometry::Section {
                    center: [sign * 0.075, 1.12 + r, z],
                    radius: [r, 0.008],
                },
            ];
        }
    }
    d.rows.push(Row {
        id: "trot".into(),
        label: "Authored trot".into(),
        tiles: vec![Tile {
            id: "trot-track".into(),
            op: Operation::Motion {
                events: vec![],
                rig: "rig".into(),
                duration: 1.2,
                looping: true,
                tracks: vec![],
            },
        }],
    });
    for (i, j) in hips.iter().enumerate() {
        let sign = if i == 0 || i == 3 { 1. } else { -1. };
        for (t, a) in [(0., 0.), (0.25, 18. * sign), (0.75, -18. * sign), (1., 0.)] {
            motion_key(&mut d, "trot", j, t, [a, 0., 0.]).unwrap();
        }
    }
    d.evaluate().unwrap();
    d
}
