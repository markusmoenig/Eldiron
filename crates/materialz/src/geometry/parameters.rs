//! Typed, whitelisted recipe inputs addressed by stable operation identities.
//! Bindings replace values on an evaluation copy, never the authored construction.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
    pub operation: String,
    pub parameter: String,
}
#[derive(Clone, Debug)]
pub struct Parameter {
    pub target: Target,
    pub label: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub integer: bool,
}
#[derive(Clone, Debug)]
pub struct Field {
    pub id: String,
    pub label: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub integer: bool,
}
/// Enumerate numeric inputs, excluding identities, references and host dependencies.
pub fn fields(op: &Operation) -> Vec<Field> {
    let mut result = vec![];
    let mut add = |id: String, label: String, value, min, max, integer| {
        result.push(Field {
            id,
            label,
            value,
            min,
            max,
            integer,
        })
    };
    let axes = ["X", "Y", "Z"];
    match op {
        Operation::Box { size } => {
            for k in 0..3 {
                add(
                    format!("size{k}"),
                    ["Width", "Height", "Depth"][k].into(),
                    size[k],
                    0.001,
                    100.,
                    false,
                );
            }
        }
        Operation::Extrude { depth } => add(
            "depth".into(),
            "Thickness".into(),
            *depth,
            0.001,
            100.,
            false,
        ),
        Operation::SurfaceOffset { distance } => add(
            "distance".into(),
            "Surface offset".into(),
            *distance,
            -0.1,
            0.1,
            false,
        ),
        Operation::Contour { points } => {
            for (i, p) in points.iter().enumerate() {
                for k in 0..2 {
                    add(
                        format!("point{i}:{k}"),
                        format!("Point {} {}", i + 1, axes[k]),
                        p[k],
                        -100.,
                        100.,
                        false,
                    );
                }
            }
        }
        Operation::Connector {
            translation,
            rotation,
            ..
        } => {
            for k in 0..3 {
                add(
                    format!("position{k}"),
                    format!("Position {}", axes[k]),
                    translation[k],
                    -1000.,
                    1000.,
                    false,
                );
                add(
                    format!("rotation{k}"),
                    format!("Rotation {}", axes[k]),
                    rotation[k],
                    -1000.,
                    1000.,
                    false,
                );
            }
        }
        Operation::Transform {
            translation,
            rotation,
            scale,
        } => {
            for (name, label, values, min, max) in [
                ("position", "Position", translation, -1000., 1000.),
                ("rotation", "Rotation", rotation, -1000., 1000.),
                ("scale", "Scale", scale, 0.001, 100.),
            ] {
                for k in 0..3 {
                    add(
                        format!("{name}{k}"),
                        format!("{label} {}", axes[k]),
                        values[k],
                        min,
                        max,
                        false,
                    );
                }
            }
        }
        Operation::Repeat {
            count,
            translation,
            rotation,
        } => {
            add(
                "count".into(),
                "Count".into(),
                *count as f32,
                1.,
                128.,
                true,
            );
            for (name, label, values, min, max) in [
                ("step", "Spacing", translation, -1000., 1000.),
                ("rotation", "Rotation step", rotation, -1000., 1000.),
            ] {
                for k in 0..3 {
                    add(
                        format!("{name}{k}"),
                        format!("{label} {}", axes[k]),
                        values[k],
                        min,
                        max,
                        false,
                    );
                }
            }
        }
        Operation::Loft {
            segments, sections, ..
        } => {
            add(
                "segments".into(),
                "Segments".into(),
                *segments as f32,
                4.,
                64.,
                true,
            );
            for (i, s) in sections.iter().enumerate() {
                for k in 0..3 {
                    add(
                        format!("center{i}:{k}"),
                        format!("Ring {} center {}", i + 1, axes[k]),
                        s.center[k],
                        -1000.,
                        1000.,
                        false,
                    );
                }
                for k in 0..2 {
                    add(
                        format!("radius{i}:{k}"),
                        format!("Ring {} radius {}", i + 1, ["X", "Z"][k]),
                        s.radius[k],
                        0.001,
                        100.,
                        false,
                    );
                }
            }
        }
        Operation::Revolve { segments } => add(
            "segments".into(),
            "Segments".into(),
            *segments as f32,
            3.,
            64.,
            true,
        ),
        Operation::Material {
            roughness,
            metallic,
            ..
        } => {
            add(
                "roughness".into(),
                "Roughness".into(),
                *roughness,
                0.,
                1.,
                false,
            );
            add(
                "metallic".into(),
                "Metallic".into(),
                *metallic,
                0.,
                1.,
                false,
            );
        }
        Operation::Group { controls, .. } => {
            for c in controls
                .iter()
                .filter(|c| !c.is_palette() && c.expression.is_none())
            {
                add(
                    format!("control:{}", c.id),
                    c.label.clone(),
                    c.value,
                    c.min,
                    c.max,
                    c.is_integer(),
                );
            }
        }
        _ => {}
    }
    result
}
/// Nested groups with published controls expose those controls as their interface.
/// Empty groups expose construction inputs; linked assets remain owned by their source.
pub fn catalog(doc: &Document) -> Vec<Parameter> {
    fn visit(
        doc: &Document,
        path: &mut Vec<String>,
        prefix: &str,
        depth: usize,
        out: &mut Vec<Parameter>,
    ) {
        if depth > 6 {
            return;
        }
        for row in &doc.rows {
            for tile in &row.tiles {
                for f in fields(&tile.op) {
                    out.push(Parameter {
                        target: Target {
                            path: path.clone(),
                            operation: tile.id.clone(),
                            parameter: f.id,
                        },
                        label: if matches!(tile.op,Operation::Connector {..}) {
                            format!("{prefix}{} / Connector {} / {}",row.label,tile.id,f.label)
                        } else { format!("{prefix}{} / {} / {}", row.label, tile.op.name(), f.label) },
                        value: f.value,
                        min: f.min,
                        max: f.max,
                        integer: f.integer,
                    });
                }
                if let Operation::Group {
                    document, controls, ..
                } = &tile.op
                {
                    if controls.is_empty() {
                        path.push(tile.id.clone());
                        visit(
                            document,
                            path,
                            &format!("{prefix}{} / ", row.label),
                            depth + 1,
                            out,
                        );
                        path.pop();
                    }
                }
            }
        }
    }
    let mut out = vec![];
    visit(doc, &mut vec![], "", 0, &mut out);
    out
}
fn pair(key: &str, prefix: &str) -> Option<(usize, usize)> {
    let (i, k) = key.strip_prefix(prefix)?.split_once(':')?;
    Some((i.parse().ok()?, k.parse().ok()?))
}
pub fn set(doc: &mut Document, target: &Target, value: f32) -> Result<()> {
    if target.path.len() > 6 {
        return Err(fail("Parameter group path is too deep"));
    }
    let doc = doc.group_document_mut(&target.path)?;
    let tile = doc
        .rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find(|t| t.id == target.operation)
        .ok_or_else(|| fail(format!("Missing parameter operation {}", target.operation)))?;
    let field = fields(&tile.op)
        .into_iter()
        .find(|p| p.id == target.parameter)
        .ok_or_else(|| {
            fail(format!(
                "Unknown recipe input {} / {}",
                target.operation, target.parameter
            ))
        })?;
    if !value.is_finite() || value < field.min || value > field.max {
        return Err(fail(format!(
            "{} must be in {}–{}",
            field.label, field.min, field.max
        )));
    }
    let value = if field.integer { value.round() } else { value };
    let key = target.parameter.as_str();
    let axis = |prefix: &str| {
        key.strip_prefix(prefix)
            .and_then(|s| s.parse::<usize>().ok())
    };
    let mut written = false;
    let mut put = |v: Option<&mut f32>| {
        if let Some(v) = v {
            *v = value;
            written = true;
        }
    };
    match &mut tile.op {
        Operation::Box { size } => put(axis("size").and_then(|k| size.get_mut(k))),
        Operation::Extrude { depth } => put(Some(depth)),
        Operation::SurfaceOffset { distance } => put(Some(distance)),
        Operation::Contour { points } => {
            put(pair(key, "point").and_then(|(i, k)| points.get_mut(i)?.get_mut(k)))
        }
        Operation::Connector {
            translation,
            rotation,
            ..
        } => {
            if let Some(k) = axis("position") {
                put(translation.get_mut(k));
            } else if let Some(k) = axis("rotation") {
                put(rotation.get_mut(k));
            }
        }
        Operation::Transform {
            translation,
            rotation,
            scale,
        } => {
            if let Some(k) = axis("position") {
                put(translation.get_mut(k));
            } else if let Some(k) = axis("rotation") {
                put(rotation.get_mut(k));
            } else if let Some(k) = axis("scale") {
                put(scale.get_mut(k));
            }
        }
        Operation::Repeat {
            count,
            translation,
            rotation,
        } => {
            if key == "count" {
                *count = value as u32;
                written = true;
            } else if let Some(k) = axis("step") {
                put(translation.get_mut(k));
            } else if let Some(k) = axis("rotation") {
                put(rotation.get_mut(k));
            }
        }
        Operation::Loft {
            segments, sections, ..
        } => {
            if key == "segments" {
                *segments = value as u32;
                written = true;
            } else if let Some((i, k)) = pair(key, "center") {
                put(sections.get_mut(i).and_then(|s| s.center.get_mut(k)));
            } else if let Some((i, k)) = pair(key, "radius") {
                put(sections.get_mut(i).and_then(|s| s.radius.get_mut(k)));
            }
        }
        Operation::Revolve { segments } => {
            *segments = value as u32;
            written = true;
        }
        Operation::Material {
            roughness,
            metallic,
            ..
        } => {
            if key == "roughness" {
                put(Some(roughness));
            } else if key == "metallic" {
                put(Some(metallic));
            }
        }
        Operation::Group { controls, .. } => {
            if let Some(c) = controls
                .iter_mut()
                .find(|c| key == format!("control:{}", c.id))
            {
                c.value = value;
                written = true;
            }
        }
        _ => {}
    }
    if !written {
        return Err(fail("Unsupported recipe parameter"));
    }
    Ok(())
}
fn describe(doc: &Document, target: &Target) -> Result<Field> {
    if target.path.len() > 6 {
        return Err(fail("Parameter group path is too deep"));
    }
    let doc = doc.group_document(&target.path)?;
    let tile = doc
        .rows
        .iter()
        .flat_map(|r| &r.tiles)
        .find(|t| t.id == target.operation)
        .ok_or_else(|| fail(format!("Missing parameter operation {}", target.operation)))?;
    fields(&tile.op)
        .into_iter()
        .find(|p| p.id == target.parameter)
        .ok_or_else(|| {
            fail(format!(
                "Unknown recipe input {} / {}",
                target.operation, target.parameter
            ))
        })
}
/// Apply all inputs first; mesh modifiers run only after construction evaluation.
pub fn bind<'a>(
    doc: &'a Document,
    controls: &[group::Control],
) -> Result<std::borrow::Cow<'a, Document>> {
    let controls = expressions::resolve(controls)?;
    bind_resolved(doc, &controls)
}
pub(super) fn bind_resolved<'a>(
    doc: &'a Document,
    controls: &[group::Control],
) -> Result<std::borrow::Cow<'a, Document>> {
    if !controls
        .iter()
        .flat_map(|c| &c.modifiers)
        .any(|m| matches!(m, group::Modifier::Parameter { .. }))
    {
        return Ok(std::borrow::Cow::Borrowed(doc));
    }
    let mut result = doc.clone();
    let mut used = std::collections::BTreeSet::new();
    for c in controls {
        for m in &c.modifiers {
            if let group::Modifier::Parameter {
                target,
                factor,
                offset,
            } = m
            {
                finite(&[*factor, *offset], 1000.)?;
                if !used.insert(target.clone()) {
                    return Err(fail(format!(
                        "Recipe input {} / {} is bound more than once",
                        target.operation, target.parameter
                    )));
                }
                let field = describe(&result, target)?;
                let range = [c.min * factor + offset, c.max * factor + offset];
                if range
                    .iter()
                    .any(|v| !v.is_finite() || *v < field.min || *v > field.max)
                {
                    return Err(fail(format!(
                        "{}: published range maps outside {}–{}",
                        field.label, field.min, field.max
                    )));
                }
                set(&mut result, target, c.value * factor + offset)?;
            }
        }
    }
    Ok(std::borrow::Cow::Owned(result))
}
