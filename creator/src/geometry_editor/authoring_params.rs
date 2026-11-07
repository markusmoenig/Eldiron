use super::*;
use presentation::{number, property};
pub fn parameters(
    op: &Operation,
    s: &rigging::Selection,
) -> (Vec<MaterialNumberView>, Vec<MaterialPropertyView>) {
    let (mut n, mut p) = (vec![], vec![]);
    let mut choice = |id: &str, label: &str, i: usize, options: Vec<String>| {
        n.push(number(
            id,
            label,
            i as f32,
            0.,
            options.len().saturating_sub(1) as f32,
        ));
        p.push(property(
            id,
            label,
            vec![id.into()],
            MaterialPropertyKind::Choice,
            options,
        ));
    };
    match op {
        Operation::Armature { joints } => {
            let i = s.joint.min(joints.len().saturating_sub(1));
            choice(
                "@joint",
                "Joint",
                i,
                joints.iter().map(|j| j.label.clone()).collect(),
            );
            if let Some(j) = joints.get(i) {
                choice(
                    "joint_parent",
                    "Parent",
                    j.parent
                        .as_ref()
                        .and_then(|id| joints.iter().position(|j| &j.id == id))
                        .map_or(0, |i| i + 1),
                    std::iter::once("Root".into())
                        .chain(joints.iter().map(|j| j.label.clone()))
                        .collect(),
                );
                vectors(
                    &mut n,
                    &mut p,
                    "joint_pos",
                    "Position",
                    &j.translation,
                    -3.,
                    3.,
                );
                vectors(
                    &mut n,
                    &mut p,
                    "joint_rot",
                    "Rotation",
                    &j.rotation,
                    -180.,
                    180.,
                );
            }
        }
        Operation::Profiles { parts, .. } => {
            choice(
                "@part",
                "Part",
                s.part.min(parts.len().saturating_sub(1)),
                parts.iter().map(|p| p.label.clone()).collect(),
            );
            if let Some(part) = parts.get(s.part.min(parts.len().saturating_sub(1))) {
                choice(
                    "@section",
                    "Ring",
                    s.section.min(part.sections.len() - 1),
                    part.sections
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("Ring {}", i + 1))
                        .collect(),
                );
                let ring = &part.sections[s.section.min(part.sections.len() - 1)];
                vectors(
                    &mut n,
                    &mut p,
                    "profile_pos",
                    "Center",
                    &ring.center,
                    -3.,
                    3.,
                );
                for (id, label, value, min, max) in [
                    ("profile_width", "Width", ring.radius[0] * 2., 0.002, 2.),
                    ("profile_depth", "Depth", ring.radius[1] * 2., 0.002, 2.),
                    (
                        "profile_roughness",
                        "Roughness",
                        part.surface.roughness,
                        0.,
                        1.,
                    ),
                    (
                        "profile_metallic",
                        "Metallic",
                        part.surface.metallic,
                        0.,
                        1.,
                    ),
                ] {
                    n.push(number(id, label, value, min, max));
                }
            }
        }
        Operation::Texture {
            preserve_uv,
            size,
            axes,
            origin,
            span,
            ..
        } => {
            choice("texture_part", "Target", 0, vec!["All parts".into()]);
            choice(
                "texture_uv",
                "Mapping",
                usize::from(*preserve_uv),
                vec!["Planar projection".into(), "Surface UVs".into()],
            );
            choice(
                "texture_fit",
                "Fit UV",
                0,
                vec!["Keep mapping".into(), "Fit target geometry".into()],
            );
            choice(
                "texture_source",
                "Material",
                0,
                vec!["Current material".into()],
            );
            choice(
                "texture_axes",
                "UV plane",
                if *axes == [0, 1] {
                    0
                } else if *axes == [0, 2] {
                    1
                } else {
                    2
                },
                vec!["XY".into(), "XZ".into(), "YZ".into()],
            );
            n.push(number(
                "texture_size",
                "Resolution",
                *size as f32,
                16.,
                512.,
            ));
            for k in 0..2 {
                n.push(number(
                    format!("texture_origin{k}"),
                    ["U origin", "V origin"][k],
                    origin[k],
                    -3.,
                    3.,
                ));
                n.push(number(
                    format!("texture_span{k}"),
                    ["U span", "V span"][k],
                    span[k],
                    0.001,
                    10.,
                ));
            }
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
    id: &str,
    label: &str,
    v: &[f32; 3],
    lo: f32,
    hi: f32,
) {
    for k in 0..3 {
        n.push(number(format!("{id}{k}"), ["X", "Y", "Z"][k], v[k], lo, hi));
    }
    p.push(property(
        id,
        label,
        (0..3).map(|k| format!("{id}{k}")).collect(),
        MaterialPropertyKind::Vector,
        vec![],
    ));
}
pub fn edit(
    op: &mut Operation,
    key: &str,
    value: f32,
    s: &rigging::Selection,
) -> Result<(), String> {
    match op {
        Operation::Armature { joints } => {
            let i = s.joint.min(joints.len() - 1);
            if key == "joint_parent" {
                let parent = if value.round() as usize == 0 {
                    None
                } else {
                    Some(
                        joints
                            .get(value.round() as usize - 1)
                            .ok_or("Missing parent")?
                            .id
                            .clone(),
                    )
                };
                let id = joints[i].id.clone();
                denrim_materials::geometry::authoring::reparent(joints, &id, parent)
                    .map_err(|e| e.to_string())?;
            } else {
                let j = &mut joints[i];
                if let Some(k) = key
                    .strip_prefix("joint_pos")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    *j.translation.get_mut(k).ok_or("Invalid axis")? = value;
                } else if let Some(k) = key
                    .strip_prefix("joint_rot")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    *j.rotation.get_mut(k).ok_or("Invalid axis")? = value;
                } else {
                    return Err("Unknown joint control".into());
                }
            }
        }
        Operation::Profiles { parts, .. } => {
            let len = parts.len();
            let p = parts
                .get_mut(s.part.min(len.saturating_sub(1)))
                .ok_or("Add a profile part first")?;
            let len = p.sections.len();
            let ring = &mut p.sections[s.section.min(len - 1)];
            match key {
                "profile_width" => ring.radius[0] = value / 2.,
                "profile_depth" => ring.radius[1] = value / 2.,
                "profile_roughness" => p.surface.roughness = value,
                "profile_metallic" => p.surface.metallic = value,
                _ => {
                    let k = key
                        .strip_prefix("profile_pos")
                        .and_then(|s| s.parse::<usize>().ok())
                        .ok_or("Unknown profile control")?;
                    *ring.center.get_mut(k).ok_or("Invalid axis")? = value;
                }
            }
        }
        Operation::Texture {
            preserve_uv,
            size,
            axes,
            origin,
            span,
            ..
        } => match key {
            "texture_uv" => *preserve_uv = value >= 0.5,
            "texture_size" => *size = value.round() as u32,
            "texture_axes" => {
                *axes = match value.round() as usize {
                    0 => [0, 1],
                    1 => [0, 2],
                    _ => [1, 2],
                }
            }
            _ => {
                if let Some(k) = key
                    .strip_prefix("texture_origin")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    *origin.get_mut(k).ok_or("Invalid UV axis")? = value;
                } else if let Some(k) = key
                    .strip_prefix("texture_span")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    *span.get_mut(k).ok_or("Invalid UV axis")? = value;
                } else {
                    return Err("Unknown texture control".into());
                }
            }
        },
        _ => return Err("Unknown authoring resource".into()),
    }
    Ok(())
}
