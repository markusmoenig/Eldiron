use super::*;
use denrim_materials::geometry::effects::{ParticleEffect, ParticlePreset};
use presentation::{number, property};
pub fn parameters(
    effect: &ParticleEffect,
    rig: Option<&denrim_materials::geometry::rig::Rig>,
) -> (Vec<MaterialNumberView>, Vec<MaterialPropertyView>) {
    let mut n = vec![number(
        "effect_preset",
        "Effect",
        match effect.preset {
            ParticlePreset::Flame => 0.,
            ParticlePreset::Smoke => 1.,
            ParticlePreset::Sparks => 2.,
        },
        0.,
        2.,
    )];
    let mut p = vec![property(
        "effect_preset",
        "Effect",
        vec!["effect_preset".into()],
        MaterialPropertyKind::Choice,
        vec!["Flame".into(), "Smoke".into(), "Sparks".into()],
    )];
    for (id, label, value, lo, hi) in [
        ("rate", "Particles / second", effect.rate, 0., 256.),
        ("size", "Size (m)", effect.size, 0.001, 1.),
        ("lifetime", "Lifetime (s)", effect.lifetime, 0.05, 10.),
        ("speed", "Rise speed", effect.speed, 0., 10.),
        ("turbulence", "Turbulence", effect.turbulence, 0., 5.),
        ("light", "Light strength", effect.light, 0., 20.),
        ("range", "Light range", effect.range, 0., 30.),
    ] {
        n.push(number(id, label, value, lo, hi));
        p.push(property(
            id,
            label,
            vec![id.into()],
            MaterialPropertyKind::Number,
            vec![],
        ));
    }
    for k in 0..3 {
        n.push(number(
            format!("position{k}"),
            ["X", "Y", "Z"][k],
            effect.position[k],
            -3.,
            3.,
        ));
    }
    p.push(property(
        "position",
        "Attachment position",
        (0..3).map(|k| format!("position{k}")).collect(),
        MaterialPropertyKind::Vector,
        vec![],
    ));
    if let Some(rig) = rig {
        let selected = effect
            .joint
            .as_ref()
            .and_then(|id| rig.index(id).ok())
            .map_or(0, |i| i + 1);
        n.push(number(
            "effect_joint",
            "Follow joint",
            selected as f32,
            0.,
            rig.joints.len() as f32,
        ));
        p.push(property(
            "effect_joint",
            "Follow joint",
            vec!["effect_joint".into()],
            MaterialPropertyKind::Choice,
            std::iter::once("Model".into())
                .chain(rig.joints.iter().map(|j| j.label.clone()))
                .collect(),
        ));
    }
    (n, p)
}
pub fn edit(
    effect: &mut ParticleEffect,
    key: &str,
    value: f32,
    rig: Option<&denrim_materials::geometry::rig::Rig>,
) -> Result<(), String> {
    match key {
        "effect_preset" => {
            let preset = match value.round() as u32 {
                0 => ParticlePreset::Flame,
                1 => ParticlePreset::Smoke,
                _ => ParticlePreset::Sparks,
            };
            let position = effect.position;
            let joint = effect.joint.clone();
            *effect = ParticleEffect::default();
            effect.position = position;
            effect.joint = joint;
            effect.preset = preset;
            match preset {
                ParticlePreset::Smoke => {
                    effect.rate = 12.;
                    effect.size = 0.04;
                    effect.lifetime = 1.8;
                    effect.speed = 0.12;
                    effect.light = 0.;
                    effect.palette = [5, 4, 3, 2];
                }
                ParticlePreset::Sparks => {
                    effect.rate = 16.;
                    effect.size = 0.012;
                    effect.lifetime = 0.7;
                    effect.speed = 0.8;
                    effect.light = 0.;
                    effect.turbulence = 0.05;
                }
                ParticlePreset::Flame => {}
            }
        }
        "effect_joint" => {
            effect.joint = rig
                .and_then(|r| r.joints.get((value.round() as usize).wrapping_sub(1)))
                .map(|j| j.id.clone());
        }
        "rate" => effect.rate = value,
        "size" => effect.size = value,
        "lifetime" => effect.lifetime = value,
        "speed" => effect.speed = value,
        "turbulence" => effect.turbulence = value,
        "light" => effect.light = value,
        "range" => effect.range = value,
        _ if key.starts_with("position") => {
            let k = key[8..].parse::<usize>().map_err(|_| "Invalid axis")?;
            *effect.position.get_mut(k).ok_or("Invalid axis")? = value;
        }
        _ => return Err("Unknown particle parameter".into()),
    }
    effect.validate().map_err(|e| e.to_string())
}
