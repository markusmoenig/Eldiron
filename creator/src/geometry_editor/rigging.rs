//! Inspector controls for reusable lofts, skeletal bindings and editable motion tracks.
use super::*;
use presentation::{number, property};
#[derive(Clone, Debug, Default)]
pub struct Selection {
    pub joint: usize,
    pub part: usize,
    pub category: usize,
    pub section: usize,
    pub track: usize,
    pub key: usize,
}

pub fn parameters(
    op: &Operation,
    doc: &Document,
    evaluation: &Evaluation,
    selection: &Selection,
) -> (Vec<MaterialNumberView>, Vec<MaterialPropertyView>) {
    let mut n = vec![];
    let mut p = vec![];
    let mut choice = |id: &str, label: &str, selected: usize, choices: Vec<String>| {
        n.push(number(
            id,
            label,
            selected as f32,
            0.,
            choices.len().saturating_sub(1) as f32,
        ));
        p.push(property(
            id,
            label,
            vec![id.into()],
            MaterialPropertyKind::Choice,
            choices,
        ));
    };
    let row_index = |id: &str| doc.rows.iter().position(|r| r.id == id).unwrap_or(0);
    let rows = || {
        doc.rows
            .iter()
            .map(|r| r.label.clone())
            .collect::<Vec<String>>()
    };
    match op {
        Operation::Loft {
            sections,
            segments,
            material,
            skin,
        } => {
            choice(
                "@section",
                "Profile ring",
                selection.section.min(sections.len() - 1),
                sections
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("Ring {}", i + 1))
                    .collect(),
            );
            let optional_rows = || {
                std::iter::once("None".into())
                    .chain(rows().into_iter())
                    .collect()
            };
            choice(
                "loft_material",
                "Material",
                material.as_ref().map_or(0, |id| row_index(id) + 1),
                optional_rows(),
            );
            choice(
                "loft_rig",
                "Skeleton",
                skin.as_ref().map_or(0, |b| row_index(&b.rig) + 1),
                optional_rows(),
            );
            if let Some(binding) = skin {
                if let Some(r) = evaluation.rows.get(&binding.rig).and_then(Value::rig) {
                    for (i, j) in binding.joints.iter().enumerate() {
                        choice(
                            &format!("loft_joint{i}"),
                            &format!("Joint {}", i + 1),
                            r.index(j).unwrap_or(0),
                            r.joints.iter().map(|j| j.label.clone()).collect(),
                        );
                    }
                    choice(
                        "loft_binding_action",
                        "Binding chain",
                        0,
                        vec![
                            "Keep chain".into(),
                            "Add child joint".into(),
                            "Remove last joint".into(),
                        ],
                    );
                }
            }
            let ring = &sections[selection.section.min(sections.len() - 1)];
            n.push(number("segments", "Segments", *segments as f32, 4., 64.));
            for k in 0..3 {
                n.push(number(
                    format!("center{k}"),
                    ["Center X", "Height", "Center Z"][k],
                    ring.center[k],
                    -3.,
                    3.,
                ));
            }
            n.push(number(
                "radius0",
                "Width (m)",
                ring.radius[0] * 2.,
                0.002,
                2.,
            ));
            n.push(number(
                "radius1",
                "Depth (m)",
                ring.radius[1] * 2.,
                0.002,
                2.,
            ));
            if let Some(binding) = skin {
                n.push(number("loft_blend", "Joint blend", binding.blend, 0.01, 1.));
            }
        }
        Operation::Joint {
            parent,
            translation,
            rotation,
        } => {
            let mut labels = vec!["Root".into()];
            labels.extend(rows());
            choice(
                "parent",
                "Parent joint",
                parent.as_ref().map_or(0, |id| row_index(id) + 1),
                labels,
            );
            vectors(&mut n, &mut p, translation, rotation);
        }
        Operation::Skeleton { rows: branches } => {
            for (i, id) in branches.iter().enumerate() {
                choice(
                    &format!("row{i}"),
                    &format!("Branch {}", i + 1),
                    row_index(id),
                    rows(),
                );
            }
        }
        Operation::Skin { rig, joints, blend } => {
            choice("rig", "Skeleton", row_index(rig), rows());
            if let Some(r) = evaluation.rows.get(rig).and_then(Value::rig) {
                let labels = r.joints.iter().map(|j| j.label.clone()).collect::<Vec<_>>();
                for (i, id) in joints.iter().enumerate() {
                    choice(
                        &format!("joint{i}"),
                        &format!("Joint {}", i + 1),
                        r.index(id).unwrap_or(0),
                        labels.clone(),
                    );
                }
            }
            n.push(number("blend", "Joint blend", *blend, 0.01, 1.));
        }
        Operation::Attach {
            enabled,
            rig,
            joint,
            grip,
            translation,
            rotation,
        } => {
            choice(
                "enabled",
                "Attachment",
                usize::from(*enabled),
                vec!["Disabled".into(), "Enabled".into()],
            );
            choice("rig", "Skeleton", row_index(rig), rows());
            choice(
                "grip",
                "Item grip",
                grip.as_ref().map_or(0, |id| row_index(id) + 1),
                std::iter::once("Own group grip".into())
                    .chain(rows().into_iter())
                    .collect(),
            );
            if let Some(r) = evaluation.rows.get(rig).and_then(Value::rig) {
                choice(
                    "joint",
                    "Socket joint",
                    r.index(joint).unwrap_or(0),
                    r.joints.iter().map(|j| j.label.clone()).collect(),
                );
            }
            vectors(&mut n, &mut p, translation, rotation);
        }
        Operation::Motion {
            rig,
            duration,
            looping,
            tracks,
            ..
        } => {
            choice("rig", "Skeleton", row_index(rig), rows());
            choice(
                "looping",
                "Playback",
                usize::from(*looping),
                vec!["Action · hold last pose".into(), "Loop".into()],
            );
            choice(
                "@track",
                "Joint track",
                selection.track.min(tracks.len().saturating_sub(1)),
                tracks
                    .iter()
                    .map(|t| {
                        evaluation
                            .rows
                            .get(rig)
                            .and_then(Value::rig)
                            .and_then(|r| r.joints.iter().find(|j| j.id == t.joint))
                            .map_or_else(|| t.joint.clone(), |j| j.label.clone())
                    })
                    .collect(),
            );
            if let Some(track) = tracks.get(selection.track.min(tracks.len().saturating_sub(1))) {
                if let Some(r) = evaluation.rows.get(rig).and_then(Value::rig) {
                    choice(
                        "track_joint",
                        "Track joint",
                        r.index(&track.joint).unwrap_or(0),
                        r.joints.iter().map(|j| j.label.clone()).collect(),
                    );
                }
                choice(
                    "@key",
                    "Keyframe",
                    selection.key.min(track.keys.len() - 1),
                    track
                        .keys
                        .iter()
                        .enumerate()
                        .map(|(i, k)| format!("{} · {:.0}%", i + 1, k.time * 100.))
                        .collect(),
                );
                choice(
                    "track_action",
                    "Track / key actions",
                    0,
                    vec![
                        "Edit selected key".into(),
                        "Add joint track".into(),
                        "Remove track".into(),
                        "Insert key after selected".into(),
                        "Delete selected key".into(),
                    ],
                );
                let key = &track.keys[selection.key.min(track.keys.len() - 1)];
                choice(
                    "visible",
                    "Joint branch",
                    usize::from(key.visible),
                    vec!["Hidden".into(), "Visible".into()],
                );
                n.push(number("phase", "Phase", key.time, 0., 1.));
                vectors(&mut n, &mut p, &key.translation, &key.rotation);
            }
            n.push(number("duration", "Duration (s)", *duration, 0.1, 10.));
        }
        _ => {}
    }
    for v in &n {
        if !p.iter().any(|p| p.components.contains(&v.id)) {
            p.push(property(
                &v.id,
                &v.label,
                vec![v.id.clone()],
                MaterialPropertyKind::Number,
                vec![],
            ));
        }
    }
    (n, p)
}
fn vectors(
    n: &mut Vec<MaterialNumberView>,
    p: &mut Vec<MaterialPropertyView>,
    t: &[f32; 3],
    r: &[f32; 3],
) {
    for (name, label, v, range) in [
        ("position", "Position (m)", t, (-3., 3.)),
        ("rotation", "Rotation (degrees)", r, (-180., 180.)),
    ] {
        for k in 0..3 {
            n.push(number(
                format!("{name}{k}"),
                ["X", "Y", "Z"][k],
                v[k],
                range.0,
                range.1,
            ));
        }
        p.push(property(
            name,
            label,
            (0..3).map(|k| format!("{name}{k}")).collect(),
            MaterialPropertyKind::Vector,
            vec![],
        ));
    }
}
pub fn edit(
    op: &mut Operation,
    key: &str,
    value: f32,
    rows: &[String],
    skeleton: Option<&denrim_materials::geometry::rig::Rig>,
    selection: &Selection,
) -> Result<(), String> {
    let index = value.round().max(0.) as usize;
    let row = || {
        rows.get(index)
            .cloned()
            .ok_or("Missing source row".to_string())
    };
    let joint = || {
        skeleton
            .and_then(|r| r.joints.get(index))
            .map(|j| j.id.clone())
            .ok_or("Missing joint".to_string())
    };
    match op {
        Operation::Loft {
            sections,
            segments,
            material,
            skin,
        } => {
            if key == "loft_material" {
                *material = if index == 0 {
                    None
                } else {
                    Some(rows.get(index - 1).cloned().ok_or("Missing material")?)
                };
                return Ok(());
            }
            if key == "loft_rig" {
                *skin = if index == 0 {
                    None
                } else {
                    Some(denrim_materials::geometry::group::Binding {
                        rig: rows.get(index - 1).cloned().ok_or("Missing skeleton")?,
                        joints: vec![
                            skeleton
                                .and_then(|r| r.joints.first())
                                .ok_or("Choose a skeleton row")?
                                .id
                                .clone(),
                        ],
                        blend: 0.35,
                    })
                };
                return Ok(());
            }
            if let Some(binding) = skin {
                if key == "loft_blend" {
                    binding.blend = value;
                    return Ok(());
                }
                if key == "loft_binding_action" {
                    match index {
                        1 => {
                            let r = skeleton.ok_or("Missing skeleton")?;
                            let parent = r
                                .index(binding.joints.last().ok_or("Missing joint")?)
                                .map_err(|e| e.to_string())?;
                            let child = r
                                .joints
                                .iter()
                                .find(|j| j.parent == Some(parent))
                                .ok_or("This joint has no child")?;
                            binding.joints.push(child.id.clone());
                        }
                        2 => {
                            if binding.joints.len() > 1 {
                                binding.joints.pop();
                            }
                        }
                        _ => {}
                    }
                    return Ok(());
                }
                if let Some(i) = key
                    .strip_prefix("loft_joint")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    *binding.joints.get_mut(i).ok_or("Missing joint")? = joint()?;
                    return Ok(());
                }
            }
            if key == "segments" {
                *segments = index as u32;
            } else {
                let len = sections.len();
                let ring = &mut sections[selection.section.min(len - 1)];
                if let Some(k) = key
                    .strip_prefix("center")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    *ring.center.get_mut(k).ok_or("Invalid axis")? = value;
                } else if let Some(k) = key
                    .strip_prefix("radius")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    *ring.radius.get_mut(k).ok_or("Invalid axis")? = value / 2.;
                } else {
                    return Err("Invalid profile control".into());
                }
            }
        }
        Operation::Joint {
            parent,
            translation,
            rotation,
        } => {
            if key == "parent" {
                *parent = if index == 0 {
                    None
                } else {
                    Some(rows.get(index - 1).cloned().ok_or("Missing parent")?)
                };
            } else {
                presentation::set_transform(key, value, translation, rotation, None)?;
            }
        }
        Operation::Skeleton { rows: branches } => {
            let i = key
                .strip_prefix("row")
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or("Invalid branch")?;
            *branches.get_mut(i).ok_or("Missing branch")? = row()?;
        }
        Operation::Skin { rig, joints, blend } => {
            if key == "rig" {
                *rig = row()?;
            } else if key == "blend" {
                *blend = value;
            } else {
                let i = key
                    .strip_prefix("joint")
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or("Invalid joint")?;
                *joints.get_mut(i).ok_or("Missing joint")? = joint()?;
            }
        }
        Operation::Attach {
            enabled,
            rig,
            joint: bone,
            grip,
            translation,
            rotation,
        } => match key {
            "enabled" => *enabled = value >= 0.5,
            "rig" => *rig = row()?,
            "joint" => *bone = joint()?,
            "grip" => {
                *grip = if index == 0 {
                    None
                } else {
                    Some(rows.get(index - 1).cloned().ok_or("Missing grip")?)
                }
            }
            _ => presentation::set_transform(key, value, translation, rotation, None)?,
        },
        Operation::Motion {
            rig,
            duration,
            looping,
            tracks,
            ..
        } => match key {
            "rig" => *rig = row()?,
            "duration" => *duration = value,
            "looping" => *looping = index == 1,
            "track_action" => {
                use denrim_materials::geometry::rig::{Key, Track};
                let at = selection.track.min(tracks.len().saturating_sub(1));
                match index {
                    0 => {}
                    1 => {
                        let bone = skeleton
                            .and_then(|r| {
                                r.joints
                                    .iter()
                                    .find(|j| !tracks.iter().any(|t| t.joint == j.id))
                            })
                            .ok_or("Every joint already has a track")?;
                        tracks.push(Track {
                            joint: bone.id.clone(),
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
                    2 => {
                        if tracks.len() <= 1 {
                            return Err("Keep at least one motion track".into());
                        }
                        tracks.remove(at);
                    }
                    3 => {
                        let track = &mut tracks[at];
                        if track.keys.len() >= 64 {
                            return Err("At most 64 keys per track".into());
                        }
                        let i = selection.key.min(track.keys.len() - 1);
                        let next = (i + 1).min(track.keys.len() - 1);
                        if i == next {
                            return Err("Select a key before the final key".into());
                        }
                        let a = &track.keys[i];
                        let b = &track.keys[next];
                        let k = Key {
                            visible: a.visible,
                            time: (a.time + b.time) / 2.,
                            translation: std::array::from_fn(|j| {
                                (a.translation[j] + b.translation[j]) / 2.
                            }),
                            rotation: std::array::from_fn(|j| (a.rotation[j] + b.rotation[j]) / 2.),
                        };
                        track.keys.insert(next, k);
                    }
                    4 => {
                        let track = &mut tracks[at];
                        let i = selection.key.min(track.keys.len() - 1);
                        if track.keys.len() <= 1
                            || (*looping && (i == 0 || i == track.keys.len() - 1))
                        {
                            return Err("Keep a key and the loop endpoints".into());
                        }
                        track.keys.remove(i);
                    }
                    _ => return Err("Unknown motion action".into()),
                }
            }
            _ => {
                let len = tracks.len();
                let track = tracks
                    .get_mut(selection.track.min(len.saturating_sub(1)))
                    .ok_or("Missing track")?;
                let len = track.keys.len();
                if key == "track_joint" {
                    track.joint = joint()?;
                    return Ok(());
                }
                let selected = selection.key.min(len - 1);
                let k = &mut track.keys[selected];
                if key == "phase" {
                    k.time = value;
                } else if key == "visible" {
                    k.visible = index == 1;
                } else {
                    presentation::set_transform(
                        key,
                        value,
                        &mut k.translation,
                        &mut k.rotation,
                        None,
                    )?;
                }
                if *looping && len > 1 && key != "phase" && (selected == 0 || selected == len - 1) {
                    let copy = k.clone();
                    let other = &mut track.keys[if selected == 0 { len - 1 } else { 0 }];
                    other.translation = copy.translation;
                    other.rotation = copy.rotation;
                    other.visible = copy.visible;
                }
            }
        },
        _ => return Err("Unsupported geometry control".into()),
    }
    Ok(())
}

pub fn profile(op: &Operation) -> Vec<[f32; 2]> {
    match op {
        Operation::Contour { points } => points.clone(),
        Operation::Loft { sections, .. } => sections
            .iter()
            .map(|s| [s.center[0] - s.radius[0], s.center[1]])
            .chain(
                sections
                    .iter()
                    .rev()
                    .map(|s| [s.center[0] + s.radius[0], s.center[1]]),
            )
            .collect(),
        _ => vec![],
    }
}
pub fn move_point(op: &mut Operation, index: usize, p: [f32; 2]) -> Result<(), String> {
    match op {
        Operation::Contour { points } => *points.get_mut(index).ok_or("Missing point")? = p,
        Operation::Loft { sections, .. } => {
            let len = sections.len();
            if index >= len * 2 {
                return Err("Missing ring".into());
            }
            let left = index < len;
            let s = &mut sections[if left { index } else { 2 * len - index - 1 }];
            let other = s.center[0] + if left { s.radius[0] } else { -s.radius[0] };
            if (left && p[0] >= other - 0.002) || (!left && p[0] <= other + 0.002) {
                return Err("Ring width must stay positive".into());
            }
            s.center[0] = (other + p[0]) / 2.;
            s.radius[0] = (other - p[0]).abs() / 2.;
            s.center[1] = p[1];
        }
        _ => return Err("Select a Contour or Loft".into()),
    }
    Ok(())
}
pub fn ring_context(op: &mut Operation, hit: Option<usize>, p: [f32; 2]) -> Result<(), String> {
    let Operation::Loft { sections, .. } = op else {
        return Err("Select a Loft".into());
    };
    let len = sections.len();
    if let Some(index) = hit {
        if len <= 2 {
            return Err("A loft needs at least two rings".into());
        }
        sections.remove(if index < len {
            index
        } else {
            2 * len - index - 1
        });
    } else {
        if len >= 32 {
            return Err("At most 32 profile rings".into());
        }
        let at = sections.partition_point(|s| s.center[1] < p[1]);
        let a = &sections[at.saturating_sub(1)];
        let b = &sections[at.min(len - 1)];
        let t = if a.center[1] == b.center[1] {
            0.
        } else {
            ((p[1] - a.center[1]) / (b.center[1] - a.center[1])).clamp(0., 1.)
        };
        let mut s = denrim_materials::geometry::Section {
            center: std::array::from_fn(|k| a.center[k] + (b.center[k] - a.center[k]) * t),
            radius: std::array::from_fn(|k| a.radius[k] + (b.radius[k] - a.radius[k]) * t),
        };
        s.center[1] = p[1];
        s.radius[0] = (p[0] - s.center[0]).abs().max(0.001);
        sections.insert(at, s);
    }
    Ok(())
}
