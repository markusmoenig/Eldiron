use crate::{math::*, *};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct Joint {
    pub name: String,
    pub parent: Option<usize>,
    pub translation: V3,
}
#[derive(Clone, Debug, Serialize)]
pub struct Material {
    pub name: String,
    pub color: [f32; 4],
    pub channel: u8,
    pub texture: Option<MaterialTexture>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Vertex {
    pub position: V3,
    pub normal: V3,
    pub uv: [f32; 2],
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}
#[derive(Clone, Debug, Serialize)]
pub struct Primitive {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct Socket {
    pub name: String,
    pub joint: usize,
    pub translation: V3,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Motion {
    Idle,
    Walk,
    Cast,
    Death,
    Sit,
    Use,
    Attack,
    Parry,
}
impl Motion {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Walk => "walk",
            Self::Cast => "cast",
            Self::Death => "death",
            Self::Sit => "sit",
            Self::Use => "use",
            Self::Attack => "attack",
            Self::Parry => "parry",
        }
    }
    pub fn looping(self) -> bool {
        matches!(self, Self::Idle | Self::Walk)
    }
    /// Atlas loops exclude their repeated endpoint; actions include their final pose.
    pub fn frame_time(self, frame: u32, frames: u32) -> f32 {
        frame as f32 / if self.looping() { frames } else { frames - 1 } as f32 * self.duration()
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Idle => 2.0,
            Self::Walk => 1.0,
            Self::Cast => 1.4,
            Self::Death => 1.2,
            Self::Sit => 0.9,
            Self::Use => 1.0,
            Self::Attack => 1.0,
            Self::Parry => 0.85,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct CharacterAsset {
    pub name: String,
    pub height: f32,
    pub style: ResolvedStyle,
    pub joints: Vec<Joint>,
    pub materials: Vec<Material>,
    pub primitives: Vec<Primitive>,
    pub sockets: Vec<Socket>,
    pub motions: Vec<Motion>,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Transform {
    pub translation: V3,
    pub rotation: Q4,
}
impl CharacterAsset {
    pub fn local_pose(&self, motion: Motion, time: f32) -> Vec<Transform> {
        let t = if motion.looping() {
            time.rem_euclid(motion.duration())
        } else {
            time.clamp(0.0, motion.duration())
        };
        let progress = t / motion.duration();
        let phase = std::f32::consts::TAU * progress;
        let ease = |t: f32| {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        // Raise, briefly hold, then lower for reusable actions. Sit/death hold their endpoint.
        let gesture = ease(progress / 0.3) * (1.0 - ease((progress - 0.7) / 0.3));
        let settle = ease(progress);
        let wind = ease(progress / 0.32);
        let strike = ease((progress - 0.32) / 0.20);
        let recover = 1.0 - ease((progress - 0.60) / 0.40);
        let recoil = ease((progress - 0.40) / 0.10) * (1.0 - ease((progress - 0.50) / 0.18));
        let (shift, twist) = match motion {
            Motion::Attack => (
                [
                    (-0.012 * wind - 0.008 * strike) * recover,
                    -(0.035 * wind + 0.01 * strike) * recover,
                    (-0.018 * wind + 0.055 * strike) * recover,
                ],
                (0.18 * wind - 0.42 * strike) * recover,
            ),
            Motion::Parry => (
                [
                    -0.008 * gesture,
                    -0.035 * gesture - 0.012 * recoil,
                    -0.018 * gesture - 0.012 * recoil,
                ],
                0.06 * gesture,
            ),
            Motion::Cast => (
                [0.004 * gesture, -0.018 * gesture, -0.012 * gesture],
                0.10 * gesture,
            ),
            Motion::Use => (
                [-0.006 * gesture, -0.020 * gesture, 0.015 * gesture],
                -0.08 * gesture,
            ),
            Motion::Walk => ([0.0; 3], -0.045 * phase.cos()),
            _ => ([0.0; 3], 0.0),
        };
        let mut local: Vec<Transform> = self
            .joints
            .iter()
            .map(|j| {
                let mut translation = j.translation;
                let angle = match motion {
                    Motion::Idle => {
                        if j.name.ends_with("upper_arm") {
                            0.025 * phase.sin()
                        } else {
                            0.0
                        }
                    }
                    Motion::Walk => {
                        let side = if j.name.starts_with("left") {
                            1.0
                        } else {
                            -1.0
                        };
                        let stride = phase.sin() * side;
                        if j.name.ends_with("thigh") {
                            0.48 * stride
                        } else if j.name.ends_with("shin") {
                            0.65 * stride.max(0.0)
                        } else if j.name.ends_with("upper_arm") {
                            let cycle = (progress + if side > 0.0 { 0.0 } else { 0.5 }).fract();
                            let travel = if cycle < 0.5 {
                                -1.0 + 2.0 * ease(cycle * 2.0)
                            } else {
                                1.0 - 4.0 * (cycle - 0.5)
                            };
                            0.32 * travel
                        } else if j.name.ends_with("forearm") {
                            -0.22
                        } else {
                            0.0
                        }
                    }
                    Motion::Cast => {
                        if j.name.ends_with("upper_arm") {
                            -1.35 * gesture
                        } else if j.name.ends_with("forearm") {
                            -0.7 * gesture
                        } else if j.name == "torso" {
                            -0.08 * gesture
                        } else if j.name == "head" {
                            0.12 * gesture
                        } else {
                            0.0
                        }
                    }
                    Motion::Use => {
                        let work = if (0.3..0.7).contains(&progress) {
                            ((progress - 0.3) / 0.4 * std::f32::consts::TAU).sin()
                        } else {
                            0.0
                        };
                        if j.name == "right_upper_arm" {
                            -0.9 * gesture
                        } else if j.name == "right_forearm" {
                            (-0.7 + 0.2 * work) * gesture
                        } else if j.name == "torso" {
                            -0.12 * gesture
                        } else if j.name == "head" {
                            0.2 * gesture
                        } else {
                            0.0
                        }
                    }
                    Motion::Attack => {
                        let wind = ease(progress / 0.32);
                        let strike = ease((progress - 0.32) / 0.20);
                        let recover = 1.0 - ease((progress - 0.60) / 0.40);
                        if j.name == "right_upper_arm" {
                            (-1.1 * wind + 1.45 * strike) * recover
                        } else if j.name == "right_forearm" {
                            (-0.65 * wind + 0.80 * strike) * recover
                        } else if j.name == "left_upper_arm" {
                            -0.4 * gesture
                        } else if j.name == "left_forearm" {
                            -0.45 * gesture
                        } else if j.name == "left_hand" {
                            0.85 * gesture
                        } else if j.name == "torso" {
                            (-0.06 * wind + 0.15 * strike) * recover
                        } else {
                            0.0
                        }
                    }
                    Motion::Parry => {
                        let recoil =
                            ease((progress - 0.40) / 0.10) * (1.0 - ease((progress - 0.50) / 0.18));
                        if j.name == "right_upper_arm" {
                            -0.8 * gesture + 0.15 * recoil
                        } else if j.name == "right_forearm" {
                            -0.9 * gesture + 0.10 * recoil
                        } else if j.name == "left_upper_arm" {
                            -0.75 * gesture + 0.15 * recoil
                        } else if j.name == "left_forearm" {
                            -0.40 * gesture
                        }
                        // Keep a carried shield facing forward as the arm raises it.
                        else if j.name == "left_hand" {
                            1.15 * gesture - 0.15 * recoil
                        } else if j.name == "torso" {
                            -0.03 * gesture + 0.08 * recoil
                        } else {
                            0.0
                        }
                    }
                    Motion::Sit => {
                        if j.name.ends_with("thigh") {
                            -std::f32::consts::FRAC_PI_2 * settle
                        } else if j.name.ends_with("shin") {
                            std::f32::consts::FRAC_PI_2 * settle
                        } else if j.name.ends_with("upper_arm") {
                            -0.45 * settle
                        } else if j.name.ends_with("forearm") {
                            -0.65 * settle
                        } else {
                            0.0
                        }
                    }
                    Motion::Death => {
                        if j.name == "root" {
                            -std::f32::consts::FRAC_PI_2 * settle
                        } else if j.name == "head" {
                            0.15 * (std::f32::consts::PI * progress).sin()
                        } else {
                            0.0
                        }
                    }
                };
                if j.name == "root" {
                    translation[1] += match motion {
                        Motion::Idle => 0.002 * self.height * phase.sin(),
                        Motion::Walk => 0.007 * self.height * (2.0 * phase).cos(),
                        Motion::Sit => -0.22 * self.height * settle,
                        Motion::Death => {
                            let angle = std::f32::consts::FRAC_PI_2 * settle;
                            self.height * (0.49 * (angle.cos() - 1.0) + 0.09 * angle.sin())
                        }
                        Motion::Cast | Motion::Use | Motion::Attack | Motion::Parry => 0.0,
                    };
                }
                if j.name == "root" {
                    translation = add(translation, mul(shift, self.height));
                }
                Transform {
                    translation,
                    rotation: if j.name == "torso" {
                        qmul(qy(twist), qx(angle))
                    } else if j.name == "head" && motion == Motion::Walk {
                        qy(-twist)
                    } else {
                        qx(angle)
                    },
                }
            })
            .collect();
        if motion == Motion::Walk {
            self.walk_feet(&mut local, progress);
        }
        if matches!(
            motion,
            Motion::Attack | Motion::Parry | Motion::Cast | Motion::Use
        ) && progress > 0.0
            && progress < 1.0
        {
            let step = if motion == Motion::Attack {
                let advance = ((progress - 0.12) / 0.30).clamp(0.0, 1.0);
                let retreat = ((progress - 0.64) / 0.30).clamp(0.0, 1.0);
                let lift = |t: f32| (std::f32::consts::PI * t).sin().powi(2);
                [
                    0.0,
                    self.height * (0.025 * lift(advance) + 0.020 * lift(retreat)),
                    self.height * 0.12 * ease(advance) * (1.0 - ease(retreat)),
                ]
            } else {
                [0.0; 3]
            };
            self.plant_action_feet(&mut local, step);
        }
        local
    }
    /// Solve leg chains toward planted/rest targets, with an optional right-foot step.
    /// A forward knee pole keeps knee flexion anatomical as hips transfer weight.
    fn plant_action_feet(&self, local: &mut [Transform], right_step: V3) {
        let rest = self.bind_pose();
        let foot = |side: &str| {
            self.joints
                .iter()
                .position(|j| j.name == format!("{side}_foot"))
                .unwrap()
        };
        self.solve_feet(
            local,
            [
                rest[foot("left")].translation,
                add(rest[foot("right")].translation, right_step),
            ],
        );
    }
    fn walk_feet(&self, local: &mut [Transform], progress: f32) {
        let rest = self.bind_pose();
        let foot = |side: &str| {
            self.joints
                .iter()
                .position(|j| j.name == format!("{side}_foot"))
                .unwrap()
        };
        let root = self.joints.iter().position(|j| j.name == "root").unwrap();
        let ease = |t: f32| t * t * (3.0 - 2.0 * t);
        let mut targets = [
            rest[foot("left")].translation,
            rest[foot("right")].translation,
        ];
        let mut support = 0;
        for (side, target) in targets.iter_mut().enumerate() {
            let cycle = (progress + side as f32 * 0.5).fract();
            let (z, lift) = if cycle < 0.5 {
                (
                    -1.0 + 2.0 * ease(cycle * 2.0),
                    (std::f32::consts::TAU * cycle).sin().powi(2) * 0.028,
                )
            } else {
                support = side;
                (1.0 - 4.0 * (cycle - 0.5), 0.0)
            };
            target[2] += self.height * 0.10 * z;
            target[1] += self.height * lift;
        }
        // Hip height follows the stance foot, keeping a small bend in its knee.
        let distance = self.height * 0.44 * 0.10_f32.cos();
        let z = targets[support][2] - local[root].translation[2];
        local[root].translation[1] = targets[support][1] + (distance * distance - z * z).sqrt();
        self.solve_feet(local, targets);
    }
    fn solve_feet(&self, local: &mut [Transform], targets: [V3; 2]) {
        let root = self.joints.iter().position(|j| j.name == "root").unwrap();
        for (side, target) in ["left", "right"].into_iter().zip(targets) {
            let index = |suffix: &str| {
                self.joints
                    .iter()
                    .position(|j| j.name == format!("{side}_{suffix}"))
                    .unwrap()
            };
            let thigh = index("thigh");
            let shin = index("shin");
            let foot = index("foot");
            let hip = add(local[root].translation, local[thigh].translation);
            let delta = sub(target, hip);
            let l1 = dot(self.joints[shin].translation, self.joints[shin].translation).sqrt();
            let l2 = dot(self.joints[foot].translation, self.joints[foot].translation).sqrt();
            let distance = dot(delta, delta).sqrt().clamp(1e-6, l1 + l2);
            let direction = unit(delta);
            let pole = unit(sub([0.0, 0.0, 1.0], mul(direction, direction[2])));
            let along = (l1 * l1 - l2 * l2 + distance * distance) / (2.0 * distance);
            let bend = (l1 * l1 - along * along).max(0.0).sqrt();
            let upper = add(mul(direction, along), mul(pole, bend));
            let lower = sub(delta, upper);
            let upper_rotation = swing_down(upper);
            let lower_rotation = swing_down(lower);
            local[thigh].rotation = upper_rotation;
            local[shin].rotation = qmul(conjugate(upper_rotation), lower_rotation);
            local[foot].rotation = conjugate(lower_rotation);
        }
    }
    pub fn world_pose(&self, local: &[Transform]) -> Vec<Transform> {
        let mut out: Vec<Transform> = Vec::new();
        for (i, j) in self.joints.iter().enumerate() {
            let t = local[i];
            out.push(if let Some(p) = j.parent {
                Transform {
                    translation: add(out[p].translation, rotate(out[p].rotation, t.translation)),
                    rotation: qmul(out[p].rotation, t.rotation),
                }
            } else {
                t
            });
        }
        out
    }
    pub fn bind_pose(&self) -> Vec<Transform> {
        self.world_pose(
            &self
                .joints
                .iter()
                .map(|j| Transform {
                    translation: j.translation,
                    rotation: [0.0, 0.0, 0.0, 1.0],
                })
                .collect::<Vec<_>>(),
        )
    }
    pub fn posed_vertex(
        &self,
        vertex: &Vertex,
        bind: &[Transform],
        pose: &[Transform],
    ) -> (V3, V3) {
        let mut point = [0.0; 3];
        let mut normal = [0.0; 3];
        for i in 0..4 {
            let w = vertex.weights[i];
            if w == 0.0 {
                continue;
            }
            let j = vertex.joints[i] as usize;
            point = add(
                point,
                mul(
                    add(
                        pose[j].translation,
                        rotate(pose[j].rotation, sub(vertex.position, bind[j].translation)),
                    ),
                    w,
                ),
            );
            normal = add(normal, mul(rotate(pose[j].rotation, vertex.normal), w));
        }
        (point, unit(normal))
    }
}
fn joint(joints: &mut Vec<Joint>, name: &str, parent: Option<usize>, translation: V3) -> usize {
    let id = joints.len();
    joints.push(Joint {
        name: name.into(),
        parent,
        translation,
    });
    id
}
fn mat(asset: &mut CharacterAsset, name: &str, rgba: [f32; 4], channel: u8) -> usize {
    let id = asset.materials.len();
    asset.materials.push(Material {
        name: name.into(),
        color: rgba,
        channel,
        texture: None,
    });
    id
}
#[derive(Clone, Copy)]
struct Point {
    p: V3,
    uv: [f32; 2],
    joints: [u16; 4],
    weights: [f32; 4],
}
fn point(p: V3, j: usize) -> Point {
    Point {
        p,
        uv: [0.0; 2],
        joints: [j as u16, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    }
}
fn triangle(mesh: &mut Primitive, a: Point, b: Point, c: Point) {
    let n = unit(cross(sub(b.p, a.p), sub(c.p, a.p)));
    let base = mesh.vertices.len() as u32;
    for p in [a, b, c] {
        mesh.vertices.push(Vertex {
            position: p.p,
            uv: p.uv,
            normal: n,
            joints: std::array::from_fn(|i| if p.weights[i] == 0.0 { 0 } else { p.joints[i] }),
            weights: p.weights,
        });
    }
    mesh.indices.extend([base, base + 1, base + 2]);
}
fn quad(mesh: &mut Primitive, mut a: Point, mut b: Point, mut c: Point, mut d: Point) {
    a.uv = [0.0, 0.0];
    b.uv = [0.0, 1.0];
    c.uv = [1.0, 1.0];
    d.uv = [1.0, 0.0];
    triangle(mesh, a, b, c);
    triangle(mesh, a, c, d);
}
fn mesh(name: &str, material: usize) -> Primitive {
    Primitive {
        name: name.into(),
        material,
        vertices: vec![],
        indices: vec![],
    }
}
fn box_part(
    asset: &mut CharacterAsset,
    name: &str,
    center: V3,
    size: V3,
    j: usize,
    material: usize,
) {
    let corners: Vec<Point> = (0..8)
        .map(|i| {
            point(
                add(
                    center,
                    std::array::from_fn(|axis| {
                        if i & (1 << axis) == 0 {
                            -size[axis] * 0.5
                        } else {
                            size[axis] * 0.5
                        }
                    }),
                ),
                j,
            )
        })
        .collect();
    let mut m = mesh(name, material);
    for [a, b, c, d] in [
        [0, 4, 6, 2],
        [1, 3, 7, 5],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 2, 3, 1],
        [4, 5, 7, 6],
    ] {
        quad(&mut m, corners[a], corners[b], corners[c], corners[d]);
    }
    asset.primitives.push(m);
}
// Rectangular rings produce connected tapered limbs. Blend weights across the elbow/knee.
fn limb(
    asset: &mut CharacterAsset,
    name: &str,
    x: f32,
    levels: [f32; 3],
    width: f32,
    joints: [usize; 2],
    material: usize,
) {
    if asset.style.preset != StylePreset::Blocky {
        rounded_limb(asset, name, x, levels, width, joints, material);
        return;
    }
    let [top, mid, bottom] = levels;
    let [upper, lower] = joints;
    let blend = (top - bottom) * 0.075;
    let rings = [
        (top, width, 0.0),
        (mid + blend, width * 0.83, 0.0),
        (mid, width * 0.8, 0.5),
        (mid - blend, width * 0.8, 1.0),
        (bottom, width * 0.62, 1.0),
    ];
    let points: Vec<[Point; 4]> = rings
        .iter()
        .map(|&(y, w, t)| {
            std::array::from_fn(|i| {
                let (sx, sz) = [(-1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (1.0, -1.0)][i];
                Point {
                    p: [
                        x + sx * w * 0.5
                            + if name.ends_with("_arm") {
                                asset.joints[lower].translation[0]
                                    * ((top - y) / (top - mid)).clamp(0.0, 1.0)
                            } else {
                                0.0
                            },
                        y,
                        sz * w * 0.5,
                    ],
                    uv: [0.0; 2],
                    joints: [upper as u16, lower as u16, 0, 0],
                    weights: [1.0 - t, t, 0.0, 0.0],
                }
            })
        })
        .collect();
    let mut m = mesh(name, material);
    for k in 0..points.len() - 1 {
        for i in 0..4 {
            let n = (i + 1) % 4;
            quad(
                &mut m,
                points[k][i],
                points[k + 1][i],
                points[k + 1][n],
                points[k][n],
            );
        }
    }
    quad(
        &mut m,
        points[0][0],
        points[0][1],
        points[0][2],
        points[0][3],
    );
    let end = points.len() - 1;
    quad(
        &mut m,
        points[end][3],
        points[end][2],
        points[end][1],
        points[end][0],
    );
    asset.primitives.push(m);
}
pub(crate) fn generate(
    id: &str,
    c: &CharacterDefinition,
    items: &BTreeMap<String, ItemDefinition>,
    style: ResolvedStyle,
) -> Result<CharacterAsset> {
    let h = c.height;
    let bulk = 0.85 + 0.35 * c.build;
    let shoulder = h * 0.115 * bulk;
    let arm_spread = h * 0.025 * bulk;
    let hip = h * 0.065 * bulk;
    let mut a = CharacterAsset {
        name: id.into(),
        height: h,
        style,
        joints: vec![],
        materials: vec![],
        primitives: vec![],
        sockets: vec![],
        motions: vec![
            Motion::Idle,
            Motion::Walk,
            Motion::Cast,
            Motion::Death,
            Motion::Sit,
            Motion::Use,
            Motion::Attack,
            Motion::Parry,
        ],
    };
    let root = joint(&mut a.joints, "root", None, [0.0, h * 0.49, 0.0]);
    let torso = joint(&mut a.joints, "torso", Some(root), [0.0, h * 0.12, 0.0]);
    let head = joint(&mut a.joints, "head", Some(torso), [0.0, h * 0.23, 0.0]);
    let mut limbs = Vec::new();
    for (side, sign) in [("left", 1.0), ("right", -1.0)] {
        let arm = joint(
            &mut a.joints,
            &format!("{side}_upper_arm"),
            Some(torso),
            [sign * shoulder, h * 0.17, 0.0],
        );
        let forearm = joint(
            &mut a.joints,
            &format!("{side}_forearm"),
            Some(arm),
            [sign * arm_spread, -h * 0.15, 0.0],
        );
        let hand = joint(
            &mut a.joints,
            &format!("{side}_hand"),
            Some(forearm),
            [0.0, -h * 0.14, 0.0],
        );
        let thigh = joint(
            &mut a.joints,
            &format!("{side}_thigh"),
            Some(root),
            [sign * hip, 0.0, 0.0],
        );
        let shin = joint(
            &mut a.joints,
            &format!("{side}_shin"),
            Some(thigh),
            [0.0, -h * 0.22, 0.0],
        );
        let foot = joint(
            &mut a.joints,
            &format!("{side}_foot"),
            Some(shin),
            [0.0, -h * 0.22, 0.0],
        );
        limbs.push((side, sign, arm, forearm, hand, thigh, shin, foot));
        a.sockets.push(Socket {
            name: if side == "right" {
                "main_hand"
            } else {
                "off_hand"
            }
            .into(),
            joint: hand,
            translation: [0.0, -h * 0.025, h * 0.025],
        });
    }
    let skin = mat(&mut a, "skin", color(&c.skin)?, 1);
    let hair = mat(&mut a, "hair", color(&c.hair)?, 6);
    let eyes = mat(&mut a, "eyes", color(&c.head.eye_color)?, 7);
    let whites = mat(&mut a, "eye_whites", color("#e4dfd2")?, 7);
    let pupils = mat(&mut a, "pupils", color("#222631")?, 7);
    let lips = mat(&mut a, "mouth", color("#754d45")?, 8);
    let mut clothing = BTreeMap::new();
    for item_id in &c.outfit {
        let item = &items[item_id];
        let channel = match item.generator {
            ItemKind::Shirt => 3,
            ItemKind::Trousers => 5,
            ItemKind::Boots => 9,
            ItemKind::Helmet => 11,
            _ => 0,
        };
        clothing.insert(
            item.generator,
            mat(&mut a, item_id, color(&item.color)?, channel),
        );
    }
    let shirt = *clothing.get(&ItemKind::Shirt).unwrap_or(&skin);
    let trousers = *clothing.get(&ItemKind::Trousers).unwrap_or(&skin);
    let boots = *clothing.get(&ItemKind::Boots).unwrap_or(&skin);
    let dressed = if shirt != skin { 1.08 } else { 1.0 };
    if style.preset == StylePreset::Blocky {
        body_part(
            &mut a,
            "torso",
            [0.0, h * 0.66, 0.0],
            [h * 0.25 * bulk * dressed, h * 0.26, h * 0.14 * dressed],
            torso,
            shirt,
        );
    }
    if style.preset == StylePreset::Blocky {
        body_part(
            &mut a,
            "pelvis",
            [0.0, h * 0.48, 0.0],
            [h * 0.22 * bulk, h * 0.12, h * 0.14],
            root,
            trousers,
        );
    }
    if style.preset == StylePreset::Blocky {
        body_part(
            &mut a,
            "neck",
            [0.0, h * 0.82, 0.0],
            [h * 0.07, h * 0.07, h * 0.07],
            head,
            skin,
        );
    }
    body_part(
        &mut a,
        "head",
        [0.0, h * 0.91, 0.0],
        [h * 0.13, h * 0.18, h * 0.12],
        head,
        skin,
    );
    shape_head(&mut a, &c.head);
    ears(&mut a, head, skin, c.head.ear_size);
    if !clothing.contains_key(&ItemKind::Helmet) {
        hair_cap(&mut a, head, hair);
    }
    if style.preset == StylePreset::Blocky {
        body_part(
            &mut a,
            "nose",
            [0.0, h * 0.90, h * (0.0565 + 0.0125 * c.head.nose_length)],
            [
                h * 0.022 * c.head.nose_width,
                h * 0.034,
                h * 0.025 * c.head.nose_length,
            ],
            head,
            skin,
        );
    }
    face_features(
        &mut a,
        head,
        [whites, eyes, pupils],
        hair,
        lips,
        c.head.eye_spacing,
    );
    for (side, sign, arm, forearm, hand, thigh, shin, foot) in limbs {
        if style.preset == StylePreset::Blocky {
            limb(
                &mut a,
                &format!("{side}_arm"),
                sign * shoulder,
                [h * 0.78, h * 0.63, h * 0.49],
                h * 0.07 * bulk,
                [arm, forearm],
                shirt,
            );
        }
        body_part(
            &mut a,
            &format!("{side}_hand"),
            [sign * (shoulder + arm_spread), h * 0.455, 0.0],
            [h * 0.055, h * 0.08, h * 0.065],
            hand,
            skin,
        );
        if style.preset == StylePreset::Blocky {
            limb(
                &mut a,
                &format!("{side}_leg"),
                sign * hip,
                [h * 0.49, h * 0.27, h * 0.055],
                h * 0.105 * bulk,
                [thigh, shin],
                trousers,
            );
        }
        if boots != skin {
            body_part(
                &mut a,
                &format!("{side}_boot_shaft"),
                [sign * hip, h * 0.105, 0.0],
                [h * 0.078 * bulk, h * 0.10, h * 0.08],
                shin,
                boots,
            );
        }
        body_part(
            &mut a,
            &format!("{side}_foot"),
            [sign * hip, h * 0.035, h * 0.025],
            [h * 0.085 * bulk, h * 0.07, h * 0.15],
            foot,
            boots,
        );
    }
    if style.preset != StylePreset::Blocky {
        let (neck_base, waist) = rounded_upper_body(&mut a, bulk, dressed, shirt);
        rounded_lower_body(&mut a, &waist, bulk, trousers);
        integrated_neck(&mut a, &neck_base, head, skin);
    }
    if let Some(&material) = clothing.get(&ItemKind::Helmet) {
        helmet(&mut a, head, material);
    }
    let bind = a.bind_pose();
    for item_id in &c.equipment {
        let item = &items[item_id];
        let socket_name = match item.attachment.unwrap() {
            Attachment::MainHand => "main_hand",
            Attachment::OffHand => "off_hand",
        };
        let socket = a.sockets.iter().find(|s| s.name == socket_name).unwrap();
        let j = socket.joint;
        let grip = add(bind[j].translation, socket.translation);
        let metal = mat(&mut a, item_id, color(&item.color)?, 10);
        let leather = mat(&mut a, &format!("{item_id}_grip"), color("#46332b")?, 10);
        if item.generator == ItemKind::Shield {
            let size = item.size.unwrap_or([0.42, 0.55, 0.055]);
            shield(
                &mut a,
                item_id,
                add(grip, [0.0, 0.06, 0.065]),
                size,
                j,
                metal,
            );
            box_part(
                &mut a,
                &format!("{item_id}_grip"),
                grip,
                [0.025, 0.12, 0.025],
                j,
                leather,
            );
            continue;
        }
        if item.generator == ItemKind::Spike {
            let size = item.size.unwrap_or([0.06, 0.06, 0.65]);
            box_part(
                &mut a,
                &format!("{item_id}_grip"),
                grip,
                [0.025, 0.028, 0.12],
                j,
                leather,
            );
            spike(&mut a, item_id, add(grip, [0.0, 0.0, 0.06]), size, j, metal);
            continue;
        }

        // Blade points forward (+Z) from the held grip.
        box_part(
            &mut a,
            &format!("{item_id}_grip"),
            grip,
            [0.026, 0.028, 0.12],
            j,
            leather,
        );
        box_part(
            &mut a,
            &format!("{item_id}_guard"),
            add(grip, [0.0, 0.0, 0.075]),
            [0.18, 0.028, 0.025],
            j,
            metal,
        );
        let length = item.blade_length.unwrap_or(0.65);
        let width = item.blade_width.unwrap_or(0.045);
        box_part(
            &mut a,
            &format!("{item_id}_blade"),
            add(grip, [0.0, 0.0, 0.09 + length * 0.5]),
            [width, 0.015, length],
            j,
            metal,
        );
    }
    if style.shading == Shading::Smooth {
        smooth_body_normals(&mut a);
        for primitive in &mut a.primitives {
            if primitive.name == "upper_body" {
                weld_vertices(primitive);
            }
            // Eyes and weapons retain hard faces regardless of body shading.
            if primitive.name != "eye"
                && !["upper_body", "lower_body", "neck", "head"].contains(&primitive.name.as_str())
                && a.materials[primitive.material].channel != 10
            {
                smooth_normals(primitive);
            }
        }
    }
    Ok(a)
}

#[derive(Clone, Copy)]
struct Ring {
    y: f32,
    width: f32,
    depth: f32,
    blend: f32,
}
fn loft(
    asset: &mut CharacterAsset,
    name: &str,
    center: V3,
    rings: &[Ring],
    joints: [usize; 2],
    material: usize,
) {
    let count = asset.style.segments as usize;
    let mut mesh = mesh(name, material);
    let top = rings[0].y;
    let bottom = rings[rings.len() - 1].y;
    let vertex = |k: usize, i: usize| {
        let ring = rings[k];
        let angle = (i % count) as f32 / count as f32 * std::f32::consts::TAU;
        let (s, c) = angle.sin_cos();
        Point {
            p: add(center, [c * ring.width * 0.5, ring.y, s * ring.depth * 0.5]),
            uv: [i as f32 / count as f32, (top - ring.y) / (top - bottom)],
            joints: [joints[0] as u16, joints[1] as u16, 0, 0],
            weights: [1.0 - ring.blend, ring.blend, 0.0, 0.0],
        }
    };
    // Clockwise in the XZ plane; side triangles face outward.
    for k in 0..rings.len() - 1 {
        for i in 0..count {
            let a = vertex(k, i);
            let b = vertex(k, i + 1);
            let c = vertex(k + 1, i + 1);
            let d = vertex(k + 1, i);
            triangle(&mut mesh, a, b, c);
            triangle(&mut mesh, a, c, d);
        }
    }
    for (k, top_cap) in [(0, true), (rings.len() - 1, false)] {
        let r = rings[k];
        let center = Point {
            p: add(center, [0.0, r.y, 0.0]),
            uv: [0.5, 0.5],
            joints: [joints[0] as u16, joints[1] as u16, 0, 0],
            weights: [1.0 - r.blend, r.blend, 0.0, 0.0],
        };
        for i in 0..count {
            let a = vertex(k, i);
            let b = vertex(k, i + 1);
            if top_cap {
                triangle(&mut mesh, center, b, a);
            } else {
                triangle(&mut mesh, center, a, b);
            }
        }
    }
    asset.primitives.push(mesh);
}
fn rounded_limb(
    asset: &mut CharacterAsset,
    name: &str,
    x: f32,
    levels: [f32; 3],
    width: f32,
    joints: [usize; 2],
    material: usize,
) {
    let [top, mid, bottom] = levels;
    let blend = (top - bottom) * 0.075;
    let widths = if asset.style.preset == StylePreset::Natural {
        [0.90, 1.0, 0.78, 0.66, 0.72, 0.88, 0.56]
    } else {
        [1.0, 1.0, 0.90, 0.84, 0.84, 0.80, 0.66]
    };
    let ys = [
        top,
        top - (top - mid) * 0.25,
        mid + blend,
        mid,
        mid - blend,
        mid - (mid - bottom) * 0.4,
        bottom,
    ];
    let blends = [0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0];
    let rings: Vec<_> = (0..7)
        .map(|i| Ring {
            y: ys[i],
            width: width * widths[i],
            depth: width * widths[i] * 0.92,
            blend: blends[i],
        })
        .collect();
    loft(asset, name, [x, 0.0, 0.0], &rings, joints, material);
}
// Join two oriented loops, allowing torso ports and limb rings to have different
// tessellation. Boundary Points are reused exactly, including their skin weights.
fn bridge(mesh: &mut Primitive, a: &[Point], b: &[Point], mirrored: bool) {
    let mut i = 0;
    let mut j = 0;
    let mut emit = |p, q, r| {
        if mirrored {
            triangle(mesh, p, r, q);
        } else {
            triangle(mesh, p, q, r);
        }
    };
    while i < a.len() || j < b.len() {
        let next_a = (i + 1) * b.len();
        let next_b = (j + 1) * a.len();
        let p = a[i % a.len()];
        let q = b[j % b.len()];
        if next_a <= next_b && i < a.len() {
            emit(p, a[(i + 1) % a.len()], q);
            i += 1;
        } else {
            emit(p, b[(j + 1) % b.len()], q);
            j += 1;
        }
    }
}

fn rounded_upper_body(
    asset: &mut CharacterAsset,
    bulk: f32,
    dressed: f32,
    material: usize,
) -> (Vec<Point>, Vec<Point>) {
    let h = asset.height;
    // Even torso rings give symmetrical ports, including at odd detail settings.
    let n = (asset.style.segments as usize).max(8).next_multiple_of(4);
    let k = (n / 8).max(1);
    let joint = |name: &str| asset.joints.iter().position(|j| j.name == name).unwrap();
    let torso = joint("torso");
    let arms = [joint("left_upper_arm"), joint("right_upper_arm")];
    let forearms = [joint("left_forearm"), joint("right_forearm")];
    let mut body = mesh("upper_body", material);
    let profiles = [
        (0.815, 0.064, 0.066),
        (0.790, 0.21 * bulk * dressed, 0.13 * dressed),
        (0.680, 0.205 * bulk * dressed, 0.14 * dressed),
        (0.600, 0.20 * bulk * dressed, 0.13 * dressed),
        (0.550, 0.18 * bulk * dressed, 0.12 * dressed),
        (0.530, 0.20 * bulk * dressed, 0.126 * dressed),
    ];
    let rings: Vec<Vec<Point>> = profiles
        .iter()
        .enumerate()
        .map(|(row, &(y, w, d))| {
            (0..n)
                .map(|i| {
                    let angle = i as f32 / n as f32 * std::f32::consts::TAU;
                    let (sin, cos) = angle.sin_cos();
                    let front = if sin >= 0.0 { 1.08 } else { 0.88 };
                    let mut p = point([h * w * 0.5 * cos, h * y, h * d * 0.5 * sin * front], torso);
                    if row == 0 {
                        p.p[2] = h * d * 0.5 * sin - h * 0.003;
                    }
                    p.uv = [
                        i as f32 / n as f32,
                        ((0.815 - y) / 0.285_f32).clamp(0.0, 1.0),
                    ];
                    if row >= 3 {
                        let root = joint("root");
                        let weight = match row {
                            3 => 0.25,
                            4 => 0.75,
                            _ => 1.0,
                        };
                        p.joints = [torso as u16, root as u16, 0, 0];
                        p.weights = [1.0 - weight, weight, 0.0, 0.0];
                    }
                    if row == 1 || row == 2 {
                        let proximity = cos.abs().powi(8);
                        let blend = proximity * if row == 1 { 0.22 } else { 0.10 };
                        p.joints = [torso as u16, arms[usize::from(cos < 0.0)] as u16, 0, 0];
                        p.weights = [1.0 - blend, blend, 0.0, 0.0];
                    }
                    p
                })
                .collect()
        })
        .collect();
    for row in 0..rings.len() - 1 {
        for i in 0..n {
            // Remove the side faces between the shoulder and armpit rows. These
            // openings are filled by the shoulder bridges, with no internal cap.
            let side_distance = |center: usize| (i + n - center + k) % n;
            if row == 1 && (side_distance(0) < 2 * k || side_distance(n / 2) < 2 * k) {
                continue;
            }
            let next = (i + 1) % n;
            triangle(
                &mut body,
                rings[row][i],
                rings[row][next],
                rings[row + 1][next],
            );
            triangle(
                &mut body,
                rings[row][i],
                rings[row + 1][next],
                rings[row + 1][i],
            );
        }
    }
    for side in 0..2 {
        let sign = if side == 0 { 1.0 } else { -1.0 };
        let center = if side == 0 { 0 } else { n / 2 };
        let index = |offset: isize| {
            (center as isize + (sign as isize) * offset).rem_euclid(n as isize) as usize
        };
        let mut port = Vec::new();
        for offset in -(k as isize)..=k as isize {
            port.push(rings[1][index(offset)]);
        }
        for offset in (-(k as isize)..=k as isize).rev() {
            port.push(rings[2][index(offset)]);
        }
        let count = asset.style.segments as usize;
        let phase = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI / port.len() as f32;
        // The first ring is oblique: the outside follows the deltoid dome, while
        // the inside forms the armpit. Subsequent rings descend toward the elbow.
        let sleeve = [
            (0.752, 0.125, 0.80, 1.25, 0.025, 0.28, 0.0),
            (0.731, 0.136, 0.88, 1.08, 0.008, 0.12, 0.0),
            (0.700, 0.140, 0.85, 0.96, 0.0, 0.0, 0.0),
            (0.652, 0.140, 0.78, 0.78, 0.0, 0.0, 0.0),
            (0.630, 0.140, 0.68, 0.68, 0.0, 0.0, 0.5),
            (0.608, 0.140, 0.72, 0.72, 0.0, 0.0, 1.0),
            (0.574, 0.140, 0.88, 0.88, 0.0, 0.0, 1.0),
            (0.490, 0.140, 0.56, 0.56, 0.0, 0.0, 1.0),
        ];
        let width = h * 0.07 * bulk;
        for &(y, x, w, d, tilt, chest, lower) in &sleeve {
            let next: Vec<_> = (0..count)
                .map(|i| {
                    let angle = phase + i as f32 / count as f32 * std::f32::consts::TAU;
                    let (sin, cos) = angle.sin_cos();
                    Point {
                        p: [
                            sign * (h * x * bulk + cos * width * w * 0.5),
                            h * (y + tilt * cos),
                            sin * width * d * 0.5,
                        ],
                        uv: [
                            i as f32 / count as f32,
                            ((0.815 - y) / 0.325_f32).clamp(0.0, 1.0),
                        ],
                        joints: [torso as u16, arms[side] as u16, forearms[side] as u16, 0],
                        weights: [
                            chest,
                            (1.0 - chest) * (1.0 - lower),
                            (1.0 - chest) * lower,
                            0.0,
                        ],
                    }
                })
                .collect();
            bridge(&mut body, &port, &next, side == 1);
            port = next;
        }
        let mut center = point([sign * h * 0.14 * bulk, h * 0.49, 0.0], forearms[side]);
        center.uv = [0.5, 0.5];
        for i in 0..count {
            let a = port[i];
            let b = port[(i + 1) % count];
            if side == 0 {
                triangle(&mut body, center, a, b);
            } else {
                triangle(&mut body, center, b, a);
            }
        }
    }
    asset.primitives.push(body);
    (rings[0].clone(), rings.last().unwrap().clone())
}

// Hip loops contain a semicircular outer rim and an inner crotch seam. Their
// samples are not equally spaced in angle. Matching actual angular positions
// prevents the pubic/inner-thigh surface from spiraling around the leg axis.
fn bridge_hip(mesh: &mut Primitive, a: &[Point], b: &[Point], hip_x: f32) {
    let phase = a[0].p[2].atan2(a[0].p[0] - hip_x);
    let parameters = |ring: &[Point]| {
        let mut values: Vec<_> = ring
            .iter()
            .map(|p| {
                (p.p[2].atan2(p.p[0] - hip_x) - phase).rem_euclid(std::f32::consts::TAU)
                    / std::f32::consts::TAU
            })
            .collect();
        values[0] = 0.0;
        values.push(1.0);
        values
    };
    let at = parameters(a);
    let bt = parameters(b);
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        let p = a[i % a.len()];
        let q = b[j % b.len()];
        if i < a.len() && (j == b.len() || at[i + 1] <= bt[j + 1]) {
            triangle(mesh, p, a[(i + 1) % a.len()], q);
            i += 1;
        } else {
            triangle(mesh, p, b[(j + 1) % b.len()], q);
            j += 1;
        }
    }
}

fn rounded_lower_body(asset: &mut CharacterAsset, waist: &[Point], bulk: f32, material: usize) {
    let h = asset.height;
    let n = waist.len();
    let joint = |name: &str| asset.joints.iter().position(|j| j.name == name).unwrap();
    let root = joint("root");
    let thighs = [joint("left_thigh"), joint("right_thigh")];
    let shins = [joint("left_shin"), joint("right_shin")];
    let mut body = mesh("lower_body", material);
    let mut port = waist.to_vec();
    for (y, w, front_depth, back_depth) in [
        (0.495, 0.235, 0.072, 0.065),
        (0.470, 0.230, 0.068, 0.066),
        (0.448, 0.210, 0.060, 0.050),
    ] {
        let next: Vec<_> = (0..n)
            .map(|i| {
                let (sin, cos) = (i as f32 / n as f32 * std::f32::consts::TAU).sin_cos();
                let blend = ((0.53 - y) / 0.10_f32).clamp(0.0, 1.0) * cos.abs() * 0.45;
                Point {
                    p: [
                        h * w * bulk * 0.5 * cos,
                        h * y,
                        h * if sin >= 0.0 {
                            front_depth * sin
                        } else {
                            // Round the two posterior lobes rather than putting a
                            // pointed ridge on the fixed pelvis midline.
                            back_depth * sin * (0.80 + 0.20 * cos.abs())
                        },
                    ],
                    uv: [i as f32 / n as f32, (0.53 - y) / 0.475],
                    joints: [root as u16, thighs[usize::from(cos < 0.0)] as u16, 0, 0],
                    weights: [1.0 - blend, blend, 0.0, 0.0],
                }
            })
            .collect();
        bridge(&mut body, &port, &next, false);
        port = next;
    }
    // Split the pelvis along one shared crotch edge. Each half becomes a thigh
    // opening; there is no pelvis bottom cap or overlapping thigh top cap.
    // Use exact midline points even when the configured segment count is odd.
    let mut front = port[n / 4];
    front.p[0] = 0.0;
    front.joints = [root as u16, 0, 0, 0];
    front.weights = [1.0, 0.0, 0.0, 0.0];
    let mut back = port[3 * n / 4];
    back.p[0] = 0.0;
    back.joints = [root as u16, 0, 0, 0];
    back.weights = [1.0, 0.0, 0.0, 0.0];
    // Update copies already emitted at these boundary positions as well.
    for v in &mut body.vertices {
        for (original, replacement) in [(port[n / 4], front), (port[3 * n / 4], back)] {
            if v.position == original.p {
                v.position = replacement.p;
                v.joints = replacement.joints;
                v.weights = replacement.weights;
            }
        }
    }
    port[n / 4] = front;
    port[3 * n / 4] = back;
    let crotch: Vec<_> = [0.025_f32, 0.0, -0.025]
        .into_iter()
        .map(|z| {
            let mut p = point([0.0, h * (0.437 - 0.08 * z.abs()), h * z], root);
            p.uv = [0.5, 0.22];
            p
        })
        .collect();
    for side in 0..2 {
        let sign = if side == 0 { 1.0 } else { -1.0 };
        let start = if side == 0 { 3 * n / 4 } else { n / 4 };
        let mut leg: Vec<_> = (0..=n / 2).map(|i| port[(start + i) % n]).collect();
        if side == 0 {
            leg.extend(crotch.iter().copied());
        } else {
            leg.extend(crotch.iter().rev().copied());
        }
        let count = asset.style.segments as usize;
        let hip_x = h * sign * 0.065 * bulk;
        let phase = leg[0].p[2].atan2(leg[0].p[0] - hip_x);
        for (y, w, d, pelvis, lower) in [
            (0.425, 0.103, 0.12, 0.35, 0.0),
            (0.390, 0.103, 0.11, 0.08, 0.0),
            (0.30, 0.083, 0.087, 0.0, 0.0),
            (0.27, 0.075, 0.078, 0.0, 0.5),
            (0.245, 0.076, 0.082, 0.0, 1.0),
            (0.19, 0.08, 0.086, 0.0, 1.0),
            (0.055, 0.059, 0.062, 0.0, 1.0),
        ] {
            let next: Vec<_> = (0..count)
                .map(|i| {
                    let (sin, cos) =
                        (phase + i as f32 / count as f32 * std::f32::consts::TAU).sin_cos();
                    let rx = h * w * bulk * 0.5;
                    let rz = h * d * 0.5;
                    let radius = 1.0 / ((cos / rx).powi(2) + (sin / rz).powi(2)).sqrt();
                    Point {
                        p: [hip_x + radius * cos, h * y, radius * sin],
                        uv: [
                            i as f32 / count as f32,
                            ((0.53 - y) / 0.475_f32).clamp(0.0, 1.0),
                        ],
                        joints: [root as u16, thighs[side] as u16, shins[side] as u16, 0],
                        weights: [
                            pelvis,
                            (1.0 - pelvis) * (1.0 - lower),
                            (1.0 - pelvis) * lower,
                            0.0,
                        ],
                    }
                })
                .collect();
            bridge_hip(&mut body, &leg, &next, hip_x);
            leg = next;
        }
        let center = point([h * sign * 0.065 * bulk, h * 0.055, 0.0], shins[side]);
        for i in 0..count {
            triangle(&mut body, center, leg[i], leg[(i + 1) % count]);
        }
    }
    asset.primitives.push(body);
}

fn integrated_neck(asset: &mut CharacterAsset, base: &[Point], joint: usize, material: usize) {
    let h = asset.height;
    let head = asset
        .primitives
        .iter_mut()
        .find(|p| p.name == "head")
        .unwrap();
    let bottom = head
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let mut boundary = BTreeMap::new();
    let mut indices = Vec::new();
    for tri in head.indices.as_chunks::<3>().0 {
        if tri
            .iter()
            .all(|i| (head.vertices[*i as usize].position[1] - bottom).abs() < 1e-6)
        {
            continue;
        }
        indices.extend(tri);
        for i in tri {
            let v = &head.vertices[*i as usize];
            if (v.position[1] - bottom).abs() < 1e-6 {
                boundary.insert(v.position.map(f32::to_bits), point(v.position, joint));
            }
        }
    }
    head.indices = indices;
    // Discard cap vertices so bounds and later normal accumulation see the surface only.
    let used: Vec<_> = head
        .indices
        .iter()
        .map(|i| head.vertices[*i as usize].clone())
        .collect();
    head.indices = (0..used.len() as u32).collect();
    head.vertices = used;
    let mut top: Vec<_> = boundary.into_values().collect();
    top.sort_by(|a, b| {
        a.p[2]
            .atan2(a.p[0])
            .rem_euclid(std::f32::consts::TAU)
            .total_cmp(&b.p[2].atan2(b.p[0]).rem_euclid(std::f32::consts::TAU))
    });
    let top_count = top.len();
    for (i, p) in top.iter_mut().enumerate() {
        p.uv = [i as f32 / top_count as f32, 0.0];
        p.uv[0] = p.uv[0].min(1.0);
    }
    let mut neck = mesh("neck", material);
    let torso = asset.joints.iter().position(|j| j.name == "torso").unwrap();
    let mid: Vec<_> = base
        .iter()
        .map(|p| {
            let mut q = *p;
            q.p = [
                p.p[0] * 0.97,
                (bottom + h * 0.815) * 0.5,
                (p.p[2] + h * 0.003) * 0.97 - h * 0.003,
            ];
            q.joints = [torso as u16, joint as u16, 0, 0];
            q.weights = [0.5, 0.5, 0.0, 0.0];
            q
        })
        .collect();
    bridge(&mut neck, &top, &mid, false);
    bridge(&mut neck, &mid, base, false);
    asset.primitives.push(neck);
}

fn body_part(
    asset: &mut CharacterAsset,
    name: &str,
    mut center: V3,
    mut size: V3,
    j: usize,
    material: usize,
) {
    let style = asset.style;
    let h = asset.height;
    if name == "head" && style.preset != StylePreset::Blocky {
        if style.preset == StylePreset::Natural {
            center[1] = h * 0.925;
            size[1] = h * 0.15;
            size[0] *= 0.94;
            size[2] *= 1.06;
        } else {
            size[0] *= 1.08;
            size[2] *= 1.08;
        }
    }
    if name == "nose" && style.preset == StylePreset::Natural {
        center[1] = h * 0.915;
    }
    if name == "hair" && style.preset != StylePreset::Blocky {
        size[1] = h * 0.052;
        if style.preset == StylePreset::Natural {
            size[0] *= 0.90;
        }
    }
    if ["head", "hair", "nose", "eye"].contains(&name) {
        // Scale the complete face around the head center, retaining eyes/nose alignment.
        let pivot = [
            0.0,
            h * if style.preset == StylePreset::Natural {
                0.925
            } else {
                0.91
            },
            0.0,
        ];
        center = add(pivot, mul(sub(center, pivot), style.head_scale));
        size = mul(size, style.head_scale);
        if name == "head" && style.preset != StylePreset::Blocky {
            let half_height = if style.preset == StylePreset::Natural {
                0.075
            } else {
                0.09
            };
            center[1] += h * half_height * (style.head_scale - 1.0).max(0.0);
        }
    }
    if style.preset == StylePreset::Blocky || name == "eye" {
        box_part(asset, name, center, size, j, material);
        return;
    }
    let profiles: Vec<(f32, f32, f32)> = match name {
        "torso" => vec![
            (0.60, 0.32, 0.55),
            (0.49, 0.68, 0.80),
            (0.34, 1.0, 1.0),
            (-0.05, 0.85, 0.90),
            (-0.35, 0.70, 0.84),
            (-0.5, 0.80, 0.90),
        ],
        "pelvis" => vec![(0.5, 0.86, 0.86), (0.12, 1.0, 1.0), (-0.5, 0.83, 0.90)],
        "head" => vec![
            (0.5, 0.28, 0.35),
            (0.38, 0.78, 0.85),
            (0.12, 1.0, 1.0),
            (-0.18, 0.96, 0.93),
            (-0.38, 0.82, 0.78),
            (-0.5, 0.52, 0.54),
        ],
        "hair" => vec![
            (0.5, 0.26, 0.32),
            (0.18, 0.75, 0.82),
            (-0.30, 1.0, 1.0),
            (-0.5, 1.0, 1.0),
        ],
        "neck" => vec![(0.5, 0.85, 0.85), (-0.5, 1.0, 1.0)],
        _ => vec![
            (0.5, 0.55, 0.60),
            (0.28, 0.95, 0.95),
            (-0.28, 1.0, 1.0),
            (-0.5, 0.70, 0.74),
        ],
    };
    let rings: Vec<_> = profiles
        .into_iter()
        .map(|(y, w, d)| Ring {
            y: y * size[1],
            width: w * size[0],
            depth: d * size[2],
            blend: 0.0,
        })
        .collect();
    loft(asset, name, center, &rings, [j, j], material);
}
fn smooth_body_normals(asset: &mut CharacterAsset) {
    let names = ["upper_body", "lower_body", "neck", "head"];
    let mut sums: BTreeMap<[u32; 3], V3> = BTreeMap::new();
    for mesh in asset
        .primitives
        .iter()
        .filter(|p| names.contains(&p.name.as_str()))
    {
        for tri in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = tri.map(|i| mesh.vertices[i as usize].position);
            let normal = cross(sub(b, a), sub(c, a));
            for i in tri {
                let key = mesh.vertices[*i as usize].position.map(f32::to_bits);
                let sum = sums.entry(key).or_insert([0.0; 3]);
                *sum = add(*sum, normal);
            }
        }
    }
    for mesh in asset
        .primitives
        .iter_mut()
        .filter(|p| names.contains(&p.name.as_str()))
    {
        for v in &mut mesh.vertices {
            v.normal = unit(sums[&v.position.map(f32::to_bits)]);
        }
    }
}

fn smooth_normals(mesh: &mut Primitive) {
    let mut sums: BTreeMap<[u32; 3], V3> = BTreeMap::new();
    for tri in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = tri.map(|i| mesh.vertices[i as usize].position);
        let normal = cross(sub(b, a), sub(c, a));
        for i in tri {
            let key = mesh.vertices[*i as usize].position.map(f32::to_bits);
            let sum = sums.entry(key).or_insert([0.0; 3]);
            *sum = add(*sum, normal);
        }
    }
    for v in &mut mesh.vertices {
        v.normal = unit(sums[&v.position.map(f32::to_bits)]);
    }
}

// Retain UV seams and hard normals, but share identical rendered vertices.
fn weld_vertices(mesh: &mut Primitive) {
    let mut unique = BTreeMap::new();
    let mut remap = Vec::new();
    for vertex in std::mem::take(&mut mesh.vertices) {
        let mut key = [0u32; 16];
        key[..3].copy_from_slice(&vertex.position.map(f32::to_bits));
        key[3..6].copy_from_slice(&vertex.normal.map(f32::to_bits));
        key[6..8].copy_from_slice(&vertex.uv.map(f32::to_bits));
        key[8..12].copy_from_slice(&vertex.joints.map(u32::from));
        key[12..].copy_from_slice(&vertex.weights.map(f32::to_bits));
        let index = *unique.entry(key).or_insert_with(|| {
            let index = mesh.vertices.len() as u32;
            mesh.vertices.push(vertex);
            index
        });
        remap.push(index);
    }
    for index in &mut mesh.indices {
        *index = remap[*index as usize];
    }
}

fn shield(asset: &mut CharacterAsset, name: &str, center: V3, size: V3, j: usize, material: usize) {
    let n = if asset.style.preset == StylePreset::Blocky {
        8
    } else {
        asset.style.segments as usize
    };
    let mut m = mesh(name, material);
    let layers = [(-0.5, 0.90), (-0.30, 1.0), (0.30, 1.0), (0.5, 0.90)];
    let ring = |k: usize, i: usize| {
        let angle = (i % n) as f32 / n as f32 * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        let mut p = point(
            add(
                center,
                [
                    cos * size[0] * 0.5 * layers[k].1,
                    sin * size[1] * 0.5 * layers[k].1,
                    size[2] * layers[k].0,
                ],
            ),
            j,
        );
        p.uv = [cos * 0.5 + 0.5, sin * 0.5 + 0.5];
        p
    };
    for k in 0..3 {
        for i in 0..n {
            triangle(&mut m, ring(k, i), ring(k, i + 1), ring(k + 1, i + 1));
            triangle(&mut m, ring(k, i), ring(k + 1, i + 1), ring(k + 1, i));
        }
    }
    for (k, front) in [(0, false), (3, true)] {
        let mut c = point(add(center, [0.0, 0.0, size[2] * layers[k].0]), j);
        c.uv = [0.5, 0.5];
        for i in 0..n {
            if front {
                triangle(&mut m, c, ring(k, i), ring(k, i + 1));
            } else {
                triangle(&mut m, c, ring(k, i + 1), ring(k, i));
            }
        }
    }
    asset.primitives.push(m);
    spike(
        asset,
        &format!("{name}_boss"),
        add(center, [0.0, 0.0, size[2] * 0.5]),
        [size[0] * 0.22, size[1] * 0.17, size[2] * 0.8],
        j,
        material,
    );
}

fn spike(asset: &mut CharacterAsset, name: &str, base: V3, size: V3, j: usize, material: usize) {
    let n = asset.style.segments as usize;
    let mut m = mesh(name, material);
    for i in 0..n {
        let ring = |i: usize| {
            let angle = i as f32 / n as f32 * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            let mut p = point(
                add(base, [cos * size[0] * 0.5, sin * size[1] * 0.5, 0.0]),
                j,
            );
            p.uv = [i as f32 / n as f32, 0.0];
            p
        };
        let mut tip = point(add(base, [0.0, 0.0, size[2]]), j);
        tip.uv = [(i as f32 + 0.5) / n as f32, 1.0];
        triangle(&mut m, ring(i), ring(i + 1), tip);
        let mut c = point(base, j);
        c.uv = [0.5, 0.5];
        triangle(&mut m, c, ring(i + 1), ring(i));
    }
    asset.primitives.push(m);
}

/// Closed, hollow dome with a joined brim; fitted to the generated head, including head scale.
fn helmet(asset: &mut CharacterAsset, j: usize, material: usize) {
    let head = asset.primitives.iter().find(|p| p.name == "head").unwrap();
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in &head.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.position[i]);
            max[i] = max[i].max(v.position[i]);
        }
    }
    let center = [
        (min[0] + max[0]) * 0.5,
        asset
            .primitives
            .iter()
            .filter(|p| p.name == "eyebrow")
            .flat_map(|p| &p.vertices)
            .map(|v| v.position[1])
            .fold(min[1], f32::max)
            + asset.height * 0.004,
        (min[2] + max[2]) * 0.5,
    ];
    let padding = asset.height * 0.007;
    let rx = (max[0] - min[0]) * 0.5 + padding;
    let rz = (max[2] - min[2]) * 0.5 + padding;
    let height = max[1] - center[1] + padding * 2.0;
    let mut profiles = [
        (0.0, 1.0),
        (0.35, 1.0),
        (0.70, 0.88),
        (0.92, 0.50),
        (1.0, 0.06),
    ];
    // A box head has full-width top corners; a tapered dome would expose them.
    if asset.style.preset == StylePreset::Blocky {
        for profile in &mut profiles {
            profile.1 = 1.0;
        }
    }
    let n = if asset.style.preset == StylePreset::Blocky {
        4
    } else {
        asset.style.segments as usize
    };
    let mut m = mesh("helmet", material);
    let ring = |inner: bool, k: usize, i: usize| {
        // Four-sided helmet encloses the box head rather than using an inscribed diamond.
        let angle = i as f32 / n as f32 * std::f32::consts::TAU
            + if n == 4 {
                std::f32::consts::FRAC_PI_4
            } else {
                0.0
            };
        let (sin, cos) = angle.sin_cos();
        let scale = if n == 4 { 2.0f32.sqrt() } else { 1.0 };
        let inset = if inner { padding * 0.6 } else { 0.0 };
        let mut p = point(
            add(
                center,
                [
                    cos * (rx - inset) * profiles[k].1 * scale,
                    profiles[k].0 * (height - inset),
                    sin * (rz - inset) * profiles[k].1 * scale,
                ],
            ),
            j,
        );
        p.uv = [i as f32 / n as f32, profiles[k].0];
        p
    };
    for inner in [false, true] {
        for k in 0..profiles.len() - 1 {
            for i in 0..n {
                let (a, b, c, d) = (
                    ring(inner, k, i),
                    ring(inner, k + 1, i),
                    ring(inner, k + 1, i + 1),
                    ring(inner, k, i + 1),
                );
                if inner {
                    triangle(&mut m, a, c, b);
                    triangle(&mut m, a, d, c);
                } else {
                    triangle(&mut m, a, b, c);
                    triangle(&mut m, a, c, d);
                }
            }
        }
        let top = profiles.len() - 1;
        let inset = if inner { padding * 0.6 } else { 0.0 };
        let mut c = point(add(center, [0.0, height - inset, 0.0]), j);
        c.uv = [0.5, 1.0];
        for i in 0..n {
            if inner {
                triangle(&mut m, c, ring(inner, top, i), ring(inner, top, i + 1));
            } else {
                triangle(&mut m, c, ring(inner, top, i + 1), ring(inner, top, i));
            }
        }
    }
    for i in 0..n {
        triangle(
            &mut m,
            ring(false, 0, i),
            ring(false, 0, i + 1),
            ring(true, 0, i + 1),
        );
        triangle(
            &mut m,
            ring(false, 0, i),
            ring(true, 0, i + 1),
            ring(true, 0, i),
        );
    }
    asset.primitives.push(m);
}

/// Read the actual front skin surface so facial marks follow every head profile/detail setting.
fn face_surface(head: &Primitive, x: f32, y: f32) -> Option<f32> {
    head.indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter_map(|tri| {
            let [a, b, c] = tri.map(|i| head.vertices[i as usize].position);
            let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if det.abs() < 1e-10 {
                return None;
            }
            let u = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / det;
            let v = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / det;
            let w = 1.0 - u - v;
            if u >= -1e-5 && v >= -1e-5 && w >= -1e-5 {
                Some(u * a[2] + v * b[2] + w * c[2])
            } else {
                None
            }
        })
        .max_by(f32::total_cmp)
}
fn face_features(
    asset: &mut CharacterAsset,
    joint: usize,
    eyes: [usize; 3],
    brows: usize,
    lips: usize,
    spacing: f32,
) {
    let head = asset
        .primitives
        .iter()
        .find(|p| p.name == "head")
        .unwrap()
        .clone();
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in &head.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.position[i]);
            max[i] = max[i].max(v.position[i]);
        }
    }
    let width = max[0] - min[0];
    let height = max[1] - min[1];
    for (name, material, x, y, w, h) in [
        (
            "eye",
            eyes[0],
            -width * 0.23 * spacing,
            min[1] + height * 0.57,
            width * 0.20,
            height * 0.12,
        ),
        (
            "eye",
            eyes[0],
            width * 0.23 * spacing,
            min[1] + height * 0.57,
            width * 0.20,
            height * 0.12,
        ),
        (
            "iris",
            eyes[1],
            -width * 0.23 * spacing,
            min[1] + height * 0.57,
            width * 0.11,
            height * 0.095,
        ),
        (
            "iris",
            eyes[1],
            width * 0.23 * spacing,
            min[1] + height * 0.57,
            width * 0.11,
            height * 0.095,
        ),
        (
            "pupil",
            eyes[2],
            -width * 0.23 * spacing,
            min[1] + height * 0.57,
            width * 0.052,
            height * 0.074,
        ),
        (
            "pupil",
            eyes[2],
            width * 0.23 * spacing,
            min[1] + height * 0.57,
            width * 0.052,
            height * 0.074,
        ),
        (
            "eyebrow",
            brows,
            -width * 0.23 * spacing,
            min[1] + height * 0.68,
            width * 0.18,
            height * 0.04,
        ),
        (
            "eyebrow",
            brows,
            width * 0.23 * spacing,
            min[1] + height * 0.68,
            width * 0.18,
            height * 0.04,
        ),
        (
            "mouth",
            lips,
            0.0,
            min[1] + height * 0.25,
            width * 0.23,
            height * 0.05,
        ),
    ] {
        let mut m = mesh(name, material);
        // Subdivide across head facets; a single flat quad can cut into a curved cheek.
        let sample = |column: usize, row: usize| {
            let px = x - w * 0.5 + w * column as f32 / 4.0;
            let u = column as f32 / 4.0;
            // A tapered closed lip, rather than a rectangular open mouth.
            let taper = if name == "mouth" {
                0.40 + 0.60 * (std::f32::consts::PI * u).sin()
            } else {
                1.0
            };
            let py = y + (row as f32 / 2.0 - 0.5) * h * taper;
            point(
                [
                    px,
                    py,
                    face_surface(&head, px, py).expect("face mark lies on head")
                        + asset.height
                            * match name {
                                "iris" => 0.003,
                                "pupil" => 0.004,
                                _ => 0.002,
                            },
                ],
                joint,
            )
        };
        for column in 0..4 {
            for row in 0..2 {
                quad(
                    &mut m,
                    sample(column, row),
                    sample(column + 1, row),
                    sample(column + 1, row + 1),
                    sample(column, row + 1),
                );
            }
        }
        asset.primitives.push(m);
    }
}

fn shape_head(asset: &mut CharacterAsset, settings: &HeadDefinition) {
    let rounded = asset.style.preset != StylePreset::Blocky;
    let h = asset.height;
    let scale = asset.style.head_scale;
    let p = asset
        .primitives
        .iter_mut()
        .find(|p| p.name == "head")
        .unwrap();
    if rounded {
        for _ in 0..2 {
            let mut refined = mesh("head", p.material);
            for tri in p.indices.as_chunks::<3>().0 {
                let points = tri.map(|i| {
                    let v = &p.vertices[i as usize];
                    Point {
                        p: v.position,
                        uv: v.uv,
                        joints: v.joints,
                        weights: v.weights,
                    }
                });
                let midpoint = |a: Point, b: Point| Point {
                    p: mul(add(a.p, b.p), 0.5),
                    uv: [(a.uv[0] + b.uv[0]) * 0.5, (a.uv[1] + b.uv[1]) * 0.5],
                    ..a
                };
                let [a, b, c] = points;
                let ab = midpoint(a, b);
                let bc = midpoint(b, c);
                let ca = midpoint(c, a);
                for [a, b, c] in [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]] {
                    triangle(&mut refined, a, b, c);
                }
            }
            *p = refined;
        }
    }
    let lo = p
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let hi = p
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    for v in &mut p.vertices {
        let y = (v.position[1] - lo) / (hi - lo);
        let jaw = (1.0 - y / 0.55).clamp(0.0, 1.0)
            * if rounded {
                (y / 0.12).clamp(0.0, 1.0)
            } else {
                1.0
            };
        let cheek = (1.0 - ((y - 0.52) / 0.28).abs()).max(0.0);
        v.position[0] *= 1.0 + (settings.jaw - 1.0) * jaw + (settings.cheekbones - 1.0) * cheek;
        if rounded {
            // Head sits slightly forward of the spine, while the throat attaches
            // behind the chin. Preserve that offset in every yaw/view direction.
            v.position[2] += h * scale * (0.012 - 0.012 * (1.0 - y / 0.28).clamp(0.0, 1.0));
        }
        if rounded && v.position[2] > h * scale * 0.03 {
            let x = v.position[0] / (h * scale * 0.008 * settings.nose_width);
            let tip = (-2.0 * x * x - ((y - 0.40) / 0.075).powi(2)).exp();
            let bridge = (-3.0 * x * x - ((y - 0.51) / 0.12).powi(2)).exp();
            v.position[2] += h * scale * settings.nose_length * (0.0085 * tip + 0.0025 * bridge);
        }
    }
    for tri in p.indices.as_chunks::<3>().0 {
        let [a, b, c] = tri.map(|i| p.vertices[i as usize].position);
        let normal = unit(cross(sub(b, a), sub(c, a)));
        for i in tri {
            p.vertices[*i as usize].normal = normal;
        }
    }
}
fn ears(asset: &mut CharacterAsset, joint: usize, material: usize, scale: f32) {
    let head = asset.primitives.iter().find(|p| p.name == "head").unwrap();
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in &head.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.position[i]);
            max[i] = max[i].max(v.position[i]);
        }
    }
    let width = max[0] - min[0];
    let height = max[1] - min[1];
    let depth = max[2] - min[2];
    for (name, side) in [("left_ear", 1.0), ("right_ear", -1.0)] {
        body_part(
            asset,
            name,
            [
                side * width * 0.51,
                min[1] + height * 0.48,
                (min[2] + max[2]) * 0.5,
            ],
            [
                width * 0.16 * scale,
                height * 0.24 * scale,
                depth * 0.20 * scale,
            ],
            joint,
            material,
        );
    }
}

/// Clip an offset copy of the scalp surface, so the cap fits every jaw/cheek/head-size variant.
fn hair_cap(asset: &mut CharacterAsset, joint: usize, material: usize) {
    let head = asset.primitives.iter().find(|p| p.name == "head").unwrap();
    let lo = head
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let hi = head
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let z_min = head
        .vertices
        .iter()
        .map(|v| v.position[2])
        .fold(f32::INFINITY, f32::min);
    let z_max = head
        .vertices
        .iter()
        .map(|v| v.position[2])
        .fold(f32::NEG_INFINITY, f32::max);
    let z_center = (z_min + z_max) * 0.5;
    let z_radius = (z_max - z_min) * 0.5;
    let x_radius = head
        .vertices
        .iter()
        .map(|v| v.position[0].abs())
        .fold(0.0_f32, f32::max);
    // Forehead stays exposed; hair wraps around the temples and down the nape.
    // Clip each refined face against a varying hairline instead of a flat cap.
    let distance = |p: [f32; 3]| {
        let front = ((p[2] - z_center) / z_radius).clamp(-1.0, 1.0);
        let fraction = 0.53
            + if front > 0.0 {
                0.25 * front
            } else {
                0.19 * front
            };
        let angle = (p[2] - z_center).atan2(p[0]);
        let irregular = 0.032 * (5.0 * angle + 0.7).sin()
            + 0.018 * (9.0 * angle - 0.4).sin()
            + 0.025 * (p[0] / x_radius) * front.max(0.0);
        let hairline = if front > 0.65 {
            (fraction + irregular).max(0.73)
        } else {
            fraction + irregular
        };
        p[1] - (lo + (hi - lo) * hairline)
    };
    let padding = asset.height * 0.003;
    let mut cap = mesh("hair", material);
    for tri in head.indices.as_chunks::<3>().0 {
        let points = tri.map(|i| point(head.vertices[i as usize].position, joint));
        let mut polygon = Vec::new();
        let mut previous = points[2];
        for current in points {
            let previous_distance = distance(previous.p);
            let current_distance = distance(current.p);
            if (previous_distance >= 0.0) != (current_distance >= 0.0) {
                let t = previous_distance / (previous_distance - current_distance);
                polygon.push(point(
                    add(previous.p, mul(sub(current.p, previous.p), t)),
                    joint,
                ));
            }
            if current_distance >= 0.0 {
                polygon.push(current);
            }
            previous = current;
        }
        for p in &mut polygon {
            let dz = p.p[2] - z_center;
            let radius = (p.p[0] * p.p[0] + dz * dz).sqrt();
            let x = p.p[0] / x_radius;
            let z = dz / z_radius;
            // Broad, deterministic locks: offset outward so the scalp remains
            // covered, and use Cartesian waves to avoid a singular crown spike.
            let locks = (0.50
                + 0.30 * (7.0 * x + 3.0 * z + 0.8).sin()
                + 0.20 * (4.0 * x - 8.0 * z - 0.3).sin())
            .clamp(0.0, 1.0);
            let crown = ((p.p[1] - lo) / (hi - lo) - 0.50).max(0.0) * 2.0;
            let volume = padding + asset.height * 0.006 * locks;
            if radius > 1e-6 {
                p.p[0] *= 1.0 + volume / radius;
                p.p[2] = z_center + dz * (1.0 + volume / radius);
            }
            p.p[0] += asset.height * 0.004 * crown;
            p.p[1] += padding + asset.height * 0.014 * locks * crown;
        }
        for i in 1..polygon.len().saturating_sub(1) {
            let area = cross(
                sub(polygon[i].p, polygon[0].p),
                sub(polygon[i + 1].p, polygon[0].p),
            );
            if dot(area, area) > 1e-12 {
                triangle(&mut cap, polygon[0], polygon[i], polygon[i + 1]);
            }
        }
    }
    asset.primitives.push(cap);
}
