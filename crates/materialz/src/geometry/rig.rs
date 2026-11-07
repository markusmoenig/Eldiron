//! Data-driven joints, inverse-bind skinning, grip attachment and keyed motion.
use super::*;
#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    pub id: String,
    pub label: String,
    pub parent: Option<usize>,
    pub local: Pose,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Rig {
    pub joints: Vec<Joint>,
    pub tip: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}
#[derive(Clone, Debug, PartialEq)]
pub struct Weights {
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}
#[derive(Clone, Debug, PartialEq)]
pub struct Attachment {
    pub id: String,
    pub joint: usize,
    pub socket: Frame,
    pub grip: Frame,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    pub time: f32,
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    #[serde(default = "key_visible")]
    pub visible: bool,
}
fn key_visible() -> bool {
    true
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub joint: String,
    pub keys: Vec<Key>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// Normalized clip phase, including 0 at the start and 1 at completion.
    pub time: f32,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
    pub rig: Rig,
    pub duration: f32,
    pub looping: bool,
    pub tracks: Vec<Track>,
    pub events: Vec<Event>,
}
#[derive(Clone, Debug)]
pub struct Posed {
    pub mesh: Mesh,
    pub joints: Vec<Pose>,
    pub attachments: BTreeMap<String, Pose>,
}
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}
fn normalize(q: [f32; 4]) -> [f32; 4] {
    let l = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    q.map(|v| v / l)
}
fn conjugate(q: [f32; 4]) -> [f32; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}
fn rotate(q: [f32; 4], p: [f32; 3]) -> [f32; 3] {
    let r = mul(mul(q, [p[0], p[1], p[2], 0.]), conjugate(q));
    [r[0], r[1], r[2]]
}
fn euler(degrees: [f32; 3]) -> [f32; 4] {
    let mut q = [0., 0., 0., 1.];
    for (axis, a) in degrees.into_iter().enumerate() {
        let (s, c) = (a.to_radians() * 0.5).sin_cos();
        let mut r = [0., 0., 0., c];
        r[axis] = s;
        q = mul(r, q);
    }
    normalize(q)
}
fn slerp(a: [f32; 4], mut b: [f32; 4], t: f32) -> [f32; 4] {
    let mut d = (0..4).map(|i| a[i] * b[i]).sum::<f32>();
    if d < 0. {
        b = b.map(|x| -x);
        d = -d;
    }
    if d > 0.9995 {
        return normalize(std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t));
    }
    let angle = d.clamp(-1., 1.).acos();
    let den = angle.sin();
    let x = ((1. - t) * angle).sin() / den;
    let y = (t * angle).sin() / den;
    normalize(std::array::from_fn(|i| x * a[i] + y * b[i]))
}
// Skinning applies each joint to many vertices. Expand its rotation once rather
// than doing two quaternion products for every position and normal.
struct SkinTransform {
    rotation: [[f32; 3]; 3],
    translation: [f32; 3],
}
impl SkinTransform {
    fn new(pose: Pose) -> Self {
        let x = pose.vector([1., 0., 0.]);
        let y = pose.vector([0., 1., 0.]);
        let z = pose.vector([0., 0., 1.]);
        Self {
            rotation: [[x[0], y[0], z[0]], [x[1], y[1], z[1]], [x[2], y[2], z[2]]],
            translation: pose.translation,
        }
    }
    #[inline(always)]
    fn vector(&self, p: [f32; 3]) -> [f32; 3] {
        let m = &self.rotation;
        [
            m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2],
            m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2],
            m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2],
        ]
    }
}

impl Pose {
    pub fn frame(self) -> Frame {
        let [x, y, z, w] = self.rotation;
        Frame {
            translation: self.translation,
            rotation: [
                (2. * (w * x + y * z))
                    .atan2(1. - 2. * (x * x + y * y))
                    .to_degrees(),
                (2. * (w * y - z * x)).clamp(-1., 1.).asin().to_degrees(),
                (2. * (w * z + x * y))
                    .atan2(1. - 2. * (y * y + z * z))
                    .to_degrees(),
            ],
        }
    }
    pub fn identity() -> Self {
        Self {
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
        }
    }
    pub fn from_frame(frame: &Frame) -> Self {
        Self {
            translation: frame.translation,
            rotation: euler(frame.rotation),
        }
    }
    pub fn compose(self, other: Self) -> Self {
        Self {
            translation: add(self.translation, rotate(self.rotation, other.translation)),
            rotation: normalize(mul(self.rotation, other.rotation)),
        }
    }
    pub fn inverse(self) -> Self {
        let q = conjugate(self.rotation);
        Self {
            translation: rotate(q, self.translation.map(|v| -v)),
            rotation: q,
        }
    }
    pub fn point(self, p: [f32; 3]) -> [f32; 3] {
        add(self.translation, rotate(self.rotation, p))
    }
    pub fn vector(self, n: [f32; 3]) -> [f32; 3] {
        rotate(self.rotation, n)
    }
}
struct SkinPose {
    joints: Vec<Pose>,
    transforms: Vec<Pose>,
    visible: Vec<bool>,
}

impl Rig {
    fn skin_pose(
        &self,
        motion: Option<&Motion>,
        time: f32,
        joints: Option<&[Pose]>,
    ) -> Result<SkinPose> {
        let rest = self.rest();
        let pose = if let Some(joints) = joints {
            if joints.len() != self.joints.len()
                || joints
                    .iter()
                    .flat_map(|p| p.translation.iter().chain(&p.rotation))
                    .any(|v| !v.is_finite())
            {
                return Err(fail("Transition pose does not match the skeleton"));
            }
            joints.to_vec()
        } else {
            self.sample(motion, time)?
        };
        let transforms = pose
            .iter()
            .zip(&rest)
            .map(|(p, b)| p.compose(b.inverse()))
            .collect::<Vec<_>>();
        let mut visible = vec![true; self.joints.len()];
        if let Some(m) = motion {
            let phase = if m.looping {
                time.rem_euclid(m.duration)
            } else {
                time.clamp(0., m.duration)
            } / m.duration;
            for track in &m.tracks {
                let key = track
                    .keys
                    .iter()
                    .rev()
                    .find(|k| k.time <= phase)
                    .unwrap_or(&track.keys[0]);
                visible[self.index(&track.joint)?] = key.visible;
            }
            for (i, j) in self.joints.iter().enumerate() {
                if let Some(parent) = j.parent {
                    visible[i] &= visible[parent];
                }
            }
        }
        Ok(SkinPose {
            joints: pose,
            transforms,
            visible,
        })
    }
    pub fn joint(
        id: &str,
        parent: Option<&Rig>,
        translation: [f32; 3],
        rotation: [f32; 3],
    ) -> Result<Self> {
        finite(&translation, 1000.)?;
        finite(&rotation, 3600.)?;
        let mut rig = parent.cloned().unwrap_or(Self {
            joints: vec![],
            tip: None,
        });
        let p = if parent.is_some() {
            Some(rig.index(rig.tip.as_deref().ok_or_else(|| {
                fail("Choose a Joint row as parent, rather than a merged skeleton")
            })?)?)
        } else {
            None
        };
        if rig.joints.len() >= 64 || rig.joints.iter().any(|j| j.id == id) {
            return Err(fail("Rig requires unique joints and at most 64 joints"));
        }
        rig.joints.push(Joint {
            id: id.into(),
            label: id.into(),
            parent: p,
            local: Pose::from_frame(&Frame {
                translation,
                rotation,
            }),
        });
        rig.tip = Some(id.into());
        Ok(rig)
    }
    pub fn index(&self, id: &str) -> Result<usize> {
        self.joints
            .iter()
            .position(|j| j.id == id)
            .ok_or_else(|| fail(format!("Unknown rig joint {id}")))
    }
    pub fn merge(rigs: &[Rig]) -> Result<Self> {
        if rigs.is_empty() || rigs.len() > 64 {
            return Err(fail("Skeleton needs 1–64 joint branches"));
        }
        let mut result = Self {
            joints: vec![],
            tip: None,
        };
        for rig in rigs {
            for joint in &rig.joints {
                let parent = joint.parent.map(|i| rig.joints[i].id.as_str());
                if let Some(existing) = result.joints.iter().find(|j| j.id == joint.id) {
                    let old_parent = existing.parent.map(|i| result.joints[i].id.as_str());
                    if existing.local != joint.local || old_parent != parent {
                        return Err(fail("Joint definitions disagree between branches"));
                    }
                } else {
                    let parent = parent.map(|id| result.index(id)).transpose()?;
                    result.joints.push(Joint {
                        id: joint.id.clone(),
                        label: joint.label.clone(),
                        parent,
                        local: joint.local,
                    });
                }
            }
        }
        if result.joints.len() > 64
            || result.joints.iter().filter(|j| j.parent.is_none()).count() != 1
        {
            return Err(fail("Skeleton needs one root and at most 64 joints"));
        }
        Ok(result)
    }
    pub fn rest(&self) -> Vec<Pose> {
        self.world(&self.joints.iter().map(|j| j.local).collect::<Vec<_>>())
    }
    fn world(&self, locals: &[Pose]) -> Vec<Pose> {
        let mut result: Vec<Pose> = vec![];
        for (i, j) in self.joints.iter().enumerate() {
            result.push(j.parent.map_or(locals[i], |p| result[p].compose(locals[i])));
        }
        result
    }
    /// Blend in joint-local space to preserve bone lengths and attachment frames.
    pub fn blend(&self, from: &[Pose], to: &[Pose], weight: f32) -> Result<Vec<Pose>> {
        if from.len() != self.joints.len()
            || to.len() != self.joints.len()
            || !weight.is_finite()
            || !(0. ..=1.).contains(&weight)
        {
            return Err(fail("Invalid joint transition"));
        }
        if weight == 0. {
            return Ok(from.to_vec());
        }
        if weight == 1. {
            return Ok(to.to_vec());
        }
        let locals = self
            .joints
            .iter()
            .enumerate()
            .map(|(i, j)| {
                let a = j
                    .parent
                    .map_or(from[i], |p| from[p].inverse().compose(from[i]));
                let b = j.parent.map_or(to[i], |p| to[p].inverse().compose(to[i]));
                Pose {
                    translation: std::array::from_fn(|k| {
                        a.translation[k] + (b.translation[k] - a.translation[k]) * weight
                    }),
                    rotation: slerp(a.rotation, b.rotation, weight),
                }
            })
            .collect::<Vec<_>>();
        Ok(self.world(&locals))
    }
    pub fn sample(&self, motion: Option<&Motion>, time: f32) -> Result<Vec<Pose>> {
        let mut locals = self.joints.iter().map(|j| j.local).collect::<Vec<_>>();
        if let Some(m) = motion {
            if &m.rig != self {
                return Err(fail("Motion and mesh must use the same skeleton"));
            }
            if !time.is_finite() {
                return Err(fail("Motion time must be finite"));
            }
            let phase = if m.looping {
                time.rem_euclid(m.duration)
            } else {
                time.clamp(0., m.duration)
            } / m.duration;
            for track in &m.tracks {
                let index = self.index(&track.joint)?;
                let (a, b) = if phase <= track.keys[0].time {
                    (&track.keys[0], &track.keys[0])
                } else {
                    track
                        .keys
                        .windows(2)
                        .find(|p| phase <= p[1].time)
                        .map(|p| (&p[0], &p[1]))
                        .unwrap_or_else(|| {
                            let k = track.keys.last().unwrap();
                            (k, k)
                        })
                };
                let t = if a.time == b.time {
                    0.
                } else {
                    (phase - a.time) / (b.time - a.time)
                };
                let delta = Pose {
                    translation: std::array::from_fn(|i| {
                        a.translation[i] + (b.translation[i] - a.translation[i]) * t
                    }),
                    rotation: slerp(euler(a.rotation), euler(b.rotation), t),
                };
                locals[index] = locals[index].compose(delta);
            }
        }
        Ok(self.world(&locals))
    }
}
impl Motion {
    /// Half-open sampling interval avoids duplicates at repeated redraws and
    /// includes markers crossed by skipped frames. Call again from the previous
    /// upper bound, and reset the cursor when an animation is interrupted.
    pub fn events_between(&self, from: f32, to: f32) -> Vec<&Event> {
        if !from.is_finite() || !to.is_finite() || to < from || to < 0. {
            return vec![];
        }
        if self.events.is_empty() {
            return vec![];
        }
        let mut result = Vec::new();
        let first = if self.looping {
            (from.max(0.) / self.duration).floor() as u64
        } else {
            0
        };
        let last = if self.looping {
            (to / self.duration).floor() as u64
        } else {
            0
        };
        for cycle in first..=last.min(first.saturating_add(63)) {
            for event in &self.events {
                let at = cycle as f32 * self.duration + event.time * self.duration;
                if at > from && at <= to {
                    result.push(event);
                }
            }
        }
        result
    }
    pub fn validate(&self) -> Result<()> {
        if !self.duration.is_finite()
            || !(0.1..=120.).contains(&self.duration)
            || self.tracks.len() > 64
        {
            return Err(fail(
                "Motion requires a duration of 0.1–120 seconds and at most 64 tracks",
            ));
        }
        if self.events.len() > 64
            || self.events.iter().enumerate().any(|(i, e)| {
                !e.time.is_finite()
                    || !(0. ..=1.).contains(&e.time)
                    || e.name.trim().is_empty()
                    || e.name.len() > 80
                    || (i > 0 && self.events[i - 1].time > e.time)
            })
        {
            return Err(fail(
                "Animation events need ordered phases in 0–1 and nonempty names (at most 64)",
            ));
        }
        let mut seen = HashSet::new();
        for track in &self.tracks {
            self.rig.index(&track.joint)?;
            if !seen.insert(&track.joint) || track.keys.is_empty() || track.keys.len() > 64 {
                return Err(fail("Motion tracks need unique joints and 1–64 keys"));
            }
            for (i, k) in track.keys.iter().enumerate() {
                finite(&k.translation, 10.)?;
                finite(&k.rotation, 3600.)?;
                if !k.time.is_finite()
                    || !(0.0..=1.).contains(&k.time)
                    || (i > 0 && k.time - track.keys[i - 1].time < 0.0001)
                {
                    return Err(fail("Key phases must increase in the 0–1 interval"));
                }
            }
            if self.looping && track.keys.len() > 1 {
                let first = &track.keys[0];
                let last = track.keys.last().unwrap();
                if first.time != 0.
                    || last.time != 1.
                    || first.translation != last.translation
                    || first.visible != last.visible
                    || dot_quat(euler(first.rotation), euler(last.rotation)).abs() < 0.9999
                {
                    return Err(fail(
                        "Looping tracks need matching endpoint poses at phases 0 and 1",
                    ));
                }
            }
        }
        Ok(())
    }
}
fn dot_quat(a: [f32; 4], b: [f32; 4]) -> f32 {
    (0..4).map(|i| a[i] * b[i]).sum()
}
pub fn bind(mesh: &mut Mesh, rig: &Rig, joints: &[String], blend: f32) -> Result<()> {
    if mesh.rig.is_some() {
        return Err(fail("Skin/Attach can only bind an unbound mesh"));
    }
    if joints.is_empty() || joints.len() > 64 || !blend.is_finite() || !(0.01..=1.).contains(&blend)
    {
        return Err(fail("Skin requires a joint chain and blend in 0.01–1"));
    }
    let indices = joints
        .iter()
        .map(|id| rig.index(id))
        .collect::<Result<Vec<_>>>()?;
    if indices
        .windows(2)
        .any(|p| rig.joints[p[1]].parent != Some(p[0]))
    {
        return Err(fail("Skin joints must follow a parent-to-child chain"));
    }
    let rest = rig.rest();
    for v in mesh.parts.iter_mut().flat_map(|p| &mut p.vertices) {
        let mut binding = Weights {
            joints: [indices[0] as u16; 4],
            weights: [1., 0., 0., 0.],
        };
        if indices.len() > 1 {
            let mut best = f32::INFINITY;
            for segment in indices.windows(2) {
                let a = rest[segment[0]].translation;
                let d = sub(rest[segment[1]].translation, a);
                let length = dot(d, d);
                if length < 1e-8 {
                    return Err(fail("Skin chain contains coincident joints"));
                }
                let t = (dot(sub(v.position, a), d) / length).clamp(0., 1.);
                let closest = add(a, d.map(|x| x * t));
                let delta = sub(v.position, closest);
                let distance = dot(delta, delta);
                if distance < best {
                    best = distance;
                    let w = ((t - (1. - blend)) / blend).clamp(0., 1.);
                    let w = w * w * (3. - 2. * w);
                    binding.joints[0] = segment[0] as u16;
                    binding.joints[1] = segment[1] as u16;
                    binding.weights = [1. - w, w, 0., 0.];
                }
            }
        }
        v.skin = Some(binding);
    }
    mesh.rig = Some(rig.clone());
    Ok(())
}
pub fn attach(
    mesh: &mut Mesh,
    rig: &Rig,
    joint: &str,
    grip: &Frame,
    socket: Frame,
    id: &str,
) -> Result<()> {
    finite(&socket.translation, 1000.)?;
    finite(&socket.rotation, 3600.)?;
    let index = rig.index(joint)?;
    let mount = Pose::from_frame(&socket).compose(Pose::from_frame(grip).inverse());
    let transform = rig.rest()[index].compose(mount);
    let mut combined = rig.clone();
    let child_offset = combined.joints.len();
    if let Some(child) = &mesh.rig {
        if child_offset + child.joints.len() > 64
            || child.joints.iter().any(|j| rig.index(&j.id).is_ok())
        {
            return Err(fail(
                "Attached rigs need unique joint IDs and at most 64 combined joints",
            ));
        }
        for j in &child.joints {
            combined.joints.push(Joint {
                id: j.id.clone(),
                label: j.label.clone(),
                parent: Some(j.parent.map_or(index, |p| p + child_offset)),
                local: if j.parent.is_none() {
                    mount.compose(j.local)
                } else {
                    j.local
                },
            });
        }
        for a in &mut mesh.attachments {
            a.joint += child_offset;
        }
    }
    for effect in &mut mesh.effects {
        effect.id = format!("{id}/{}", effect.id);
        effect.definition.position = transform.point(effect.definition.position);
        effect.joint = Some(effect.joint.map_or(index, |j| j + child_offset));
    }
    for v in mesh.parts.iter_mut().flat_map(|p| &mut p.vertices) {
        v.position = transform.point(v.position);
        v.normal = transform.vector(v.normal);
        if let Some(skin) = &mut v.skin {
            for j in &mut skin.joints {
                *j += child_offset as u16;
            }
        } else {
            v.skin = Some(Weights {
                joints: [index as u16; 4],
                weights: [1., 0., 0., 0.],
            });
        }
    }
    mesh.rig = Some(combined);
    mesh.attachments.push(Attachment {
        id: id.into(),
        joint: index,
        socket,
        grip: grip.clone(),
    });
    Ok(())
}
impl Mesh {
    pub fn posed(&self, motion: Option<&Motion>, time: f32) -> Result<Posed> {
        self.posed_with_joints(motion, time, None)
    }
    /// Sample rig-bound particles/lights without cloning or skinning mesh vertices.
    /// Hosts only need full geometry when deriving a flattened item's posed bounds.
    pub fn posed_effects_with_joints(
        &self,
        motion: Option<&Motion>,
        time: f32,
        joints: Option<&[Pose]>,
    ) -> Result<Vec<effects::EffectAttachment>> {
        let Some(rig) = &self.rig else {
            return Ok(self.effects.clone());
        };
        let posed = rig.skin_pose(motion, time, joints)?;
        Ok(self.effects_from_pose(&posed.transforms, &posed.visible))
    }
    fn effects_from_pose(
        &self,
        transforms: &[Pose],
        visible: &[bool],
    ) -> Vec<effects::EffectAttachment> {
        self.effects
            .iter()
            .filter(|effect| effect.joint.is_none_or(|j| visible[j]))
            .cloned()
            .map(|mut effect| {
                if let Some(j) = effect.joint.take() {
                    effect.definition.position = transforms[j].point(effect.definition.position);
                }
                effect
            })
            .collect()
    }

    /// Host transition poses use the same skinning, attachments and visibility
    /// path as ordinary clips. Joint transforms must belong to this skeleton.
    pub fn posed_with_joints(
        &self,
        motion: Option<&Motion>,
        time: f32,
        joints: Option<&[Pose]>,
    ) -> Result<Posed> {
        let Some(rig) = &self.rig else {
            return Ok(Posed {
                mesh: self.clone(),
                joints: vec![],
                attachments: BTreeMap::new(),
            });
        };
        let SkinPose {
            joints: pose,
            transforms,
            visible,
        } = rig.skin_pose(motion, time, joints)?;
        let skin_transforms: Vec<_> = transforms.iter().copied().map(SkinTransform::new).collect();
        let mut mesh = self.clone();
        // Retain semantic part order, so host material bindings stay stable.
        for part in &mut mesh.parts {
            if part.vertices.iter().all(|v| {
                v.skin.as_ref().is_some_and(|w| {
                    (0..4).all(|i| w.weights[i] == 0. || !visible[w.joints[i] as usize])
                })
            }) {
                part.vertices.clear();
            }
        }
        for v in mesh.parts.iter_mut().flat_map(|p| &mut p.vertices) {
            if let Some(binding) = v.skin.take() {
                let mut p = [0.; 3];
                let mut n = [0.; 3];
                // Rigid regions and attached equipment commonly have one unit
                // influence. Avoid the weighted accumulation loop for them.
                if binding.weights == [1., 0., 0., 0.] {
                    let transform = &skin_transforms[binding.joints[0] as usize];
                    let a = transform.vector(v.position);
                    p = [
                        a[0] + transform.translation[0],
                        a[1] + transform.translation[1],
                        a[2] + transform.translation[2],
                    ];
                    n = transform.vector(v.normal);
                } else {
                    for i in 0..4 {
                        if binding.weights[i] == 0. {
                            continue;
                        }
                        let transform = &skin_transforms[binding.joints[i] as usize];
                        let a = transform.vector(v.position);
                        let b = transform.vector(v.normal);
                        let w = binding.weights[i];
                        p[0] += (a[0] + transform.translation[0]) * w;
                        p[1] += (a[1] + transform.translation[1]) * w;
                        p[2] += (a[2] + transform.translation[2]) * w;
                        n[0] += b[0] * w;
                        n[1] += b[1] * w;
                        n[2] += b[2] * w;
                    }
                }
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-10);
                v.position = p;
                v.normal = [n[0] / len, n[1] / len, n[2] / len];
            }
        }
        mesh.effects = self.effects_from_pose(&transforms, &visible);
        mesh.rig = None;
        let attachments = self
            .attachments
            .iter()
            .map(|a| {
                (
                    a.id.clone(),
                    pose[a.joint].compose(Pose::from_frame(&a.socket)),
                )
            })
            .collect();
        Ok(Posed {
            mesh,
            joints: pose,
            attachments,
        })
    }
}
