use super::*;
fn input_mesh<'a>(
    doc: &Document,
    evaluation: &'a Evaluation,
    operation: &str,
) -> Option<&'a denrim_materials::geometry::Mesh> {
    for row in &doc.rows {
        if let Some(index) = row.tiles.iter().position(|t| t.id == operation) {
            return index
                .checked_sub(1)
                .and_then(|i| evaluation.operations.get(&row.tiles[i].id))
                .and_then(|value| {
                    if let Value::Mesh(mesh) = value {
                        Some(mesh)
                    } else {
                        None
                    }
                });
        }
    }
    None
}
pub(super) fn number(
    id: impl Into<String>,
    label: impl Into<String>,
    value: f32,
    min: f32,
    max: f32,
) -> MaterialNumberView {
    MaterialNumberView {
        id: id.into(),
        label: label.into(),
        value,
        min,
        max,
    }
}
pub(super) fn property(
    id: &str,
    label: &str,
    components: Vec<String>,
    kind: MaterialPropertyKind,
    choices: Vec<String>,
) -> MaterialPropertyView {
    MaterialPropertyView {
        id: id.into(),
        label: label.into(),
        group: String::new(),
        kind,
        components,
        choices,
        tooltip: String::new(),
    }
}
fn parameters(
    operation: &str,
    op: &Operation,
    doc: &Document,
    evaluation: &Evaluation,
    selection: &super::rigging::Selection,
) -> (Vec<MaterialNumberView>, Vec<MaterialPropertyView>) {
    let mut n = vec![];
    let mut p = vec![];
    let mut scalar = |id: &str, label: &str, value: f32, min: f32, max: f32| {
        n.push(number(id, label, value, min, max));
    };
    match op {
        Operation::Armature { .. } | Operation::Profiles { .. } | Operation::Texture { .. } => {
            return super::authoring_params::parameters(op, selection);
        }
        Operation::Connector {
            translation,
            rotation,
            ..
        } => {
            for k in 0..3 {
                scalar(
                    &format!("position{k}"),
                    &format!("Position {}", ["X", "Y", "Z"][k]),
                    translation[k],
                    -1000.,
                    1000.,
                );
                scalar(
                    &format!("rotation{k}"),
                    &format!("Rotation {}", ["X", "Y", "Z"][k]),
                    rotation[k],
                    -1000.,
                    1000.,
                );
            }
        }
        Operation::Group { controls, .. } => return super::groups::parameters(controls, selection),
        Operation::Asset { .. } | Operation::Input { .. } => {}
        Operation::Footprint { row, holes, append } => {
            let contours = doc
                .rows
                .iter()
                .take_while(|r| r.tiles.iter().all(|t| t.id != operation))
                .filter(|r| matches!(evaluation.rows.get(&r.id), Some(Value::Contour(_))))
                .collect::<Vec<_>>();
            scalar(
                "footprint_row",
                "Contour (X/Z)",
                contours.iter().position(|r| &r.id == row).unwrap_or(0) as f32,
                0.,
                contours.len().saturating_sub(1) as f32,
            );
            p.push(property(
                "footprint_row",
                "Contour (X/Z)",
                vec!["footprint_row".into()],
                MaterialPropertyKind::Choice,
                contours.iter().map(|r| r.label.clone()).collect(),
            ));
            for (i, hole) in holes.iter().enumerate() {
                let key = format!("footprint_hole:{i}");
                scalar(
                    &key,
                    &format!("Hole {}", i + 1),
                    contours
                        .iter()
                        .position(|r| &r.id == hole)
                        .map_or(0, |i| i + 1) as f32,
                    0.,
                    contours.len() as f32,
                );
                p.push(property(
                    &key,
                    &format!("Hole {}", i + 1),
                    vec![key.clone()],
                    MaterialPropertyKind::Choice,
                    std::iter::once("Remove hole".into())
                        .chain(contours.iter().map(|r| r.label.clone()))
                        .collect(),
                ));
            }
            let available = contours
                .iter()
                .filter(|r| &r.id != row && !holes.contains(&r.id))
                .collect::<Vec<_>>();
            if holes.len() < 32 && !available.is_empty() {
                scalar(
                    "footprint_add_hole",
                    "Add hole",
                    0.,
                    0.,
                    available.len() as f32,
                );
                p.push(property(
                    "footprint_add_hole",
                    "Add hole",
                    vec!["footprint_add_hole".into()],
                    MaterialPropertyKind::Choice,
                    std::iter::once("Choose contour…".into())
                        .chain(available.iter().map(|r| r.label.clone()))
                        .collect(),
                ));
            }
            scalar(
                "footprint_append",
                "Add region",
                if *append { 1. } else { 0. },
                0.,
                1.,
            );
            p.push(property(
                "footprint_append",
                "Add region",
                vec!["footprint_append".into()],
                MaterialPropertyKind::Boolean,
                vec![],
            ));
            p.last_mut().unwrap().tooltip="Append this outer contour and its holes to the current footprint. Leave off to replace it. Repeat and Assemble retain all regions.".into();
        }
        Operation::FitTo {
            target,
            axes,
            source_anchor,
            target_anchor,
            padding,
        } => {
            scalar(
                "target",
                "Target row",
                doc.rows.iter().position(|r| &r.id == target).unwrap_or(0) as f32,
                0.,
                doc.rows.len().saturating_sub(1) as f32,
            );
            p.push(property(
                "target",
                "Target row",
                vec!["target".into()],
                MaterialPropertyKind::Choice,
                doc.rows.iter().map(|r| r.label.clone()).collect(),
            ));
            for k in 0..3 {
                scalar(
                    &format!("fit{k}"),
                    &format!("Fit {}", ["X", "Y", "Z"][k]),
                    if axes[k] { 1. } else { 0. },
                    0.,
                    1.,
                );
                for (key, label, values, low, high) in [
                    ("source_anchor", "Source anchor", source_anchor, 0., 1.),
                    ("target_anchor", "Target anchor", target_anchor, 0., 1.),
                    ("padding", "Padding", padding, -10., 10.),
                ] {
                    scalar(
                        &format!("{key}{k}"),
                        &format!("{label} {}", ["X", "Y", "Z"][k]),
                        values[k],
                        low,
                        high,
                    );
                }
            }
        }
        Operation::ParticleEffect { effect } => {
            return super::effects::parameters(
                effect,
                evaluation.operations.get(operation).and_then(Value::rig),
            );
        }
        Operation::SurfaceOffset { distance } => {
            scalar("distance", "Offset (m)", *distance, -0.1, 0.1)
        }
        Operation::SelectParts { parts } => {
            if let Some(mesh) = input_mesh(doc, evaluation, operation) {
                for part in &mesh.parts {
                    let key = format!("selected:{}", part.id);
                    scalar(
                        &key,
                        &part.label,
                        if parts.contains(&part.id) { 1. } else { 0. },
                        0.,
                        1.,
                    );
                    p.push(property(
                        &key,
                        &part.label,
                        vec![key.clone()],
                        MaterialPropertyKind::Boolean,
                        vec![],
                    ));
                }
            }
        }
        Operation::Loft { .. }
        | Operation::Joint { .. }
        | Operation::Skeleton { .. }
        | Operation::Skin { .. }
        | Operation::Attach { .. }
        | Operation::Motion { .. } => {
            return super::rigging::parameters(op, doc, evaluation, selection);
        }
        Operation::Contour { points } => {
            let footprint =
                doc.rows
                    .iter()
                    .find(|r| r.tiles.iter().any(|t| t.id == operation))
                    .is_some_and(|owner| {
                        doc.rows.iter().flat_map(|r| &r.tiles).any(
                            |t| matches!(&t.op, Operation::Footprint { row, holes, .. } if row == &owner.id || holes.contains(&owner.id)),
                        )
                    });
            let axes = if footprint { ["X", "Z"] } else { ["X", "Y"] };
            for (i, point) in points.iter().enumerate() {
                for k in 0..2 {
                    scalar(&format!("point{i}:{k}"), axes[k], point[k], -10., 10.);
                }
                p.push(property(
                    &format!("point{i}"),
                    &format!("Point {} X / {}", i + 1, axes[1]),
                    (0..2).map(|k| format!("point{i}:{k}")).collect(),
                    MaterialPropertyKind::Vector,
                    vec![],
                ));
            }
        }
        Operation::Extrude { depth } => scalar("depth", "Thickness (m)", *depth, 0.001, 2.),
        Operation::Repeat { .. } => {
            for f in denrim_materials::geometry::parameters::fields(op) {
                scalar(&f.id, &f.label, f.value, f.min, f.max);
            }
        }
        Operation::Box { size } => {
            for k in 0..3 {
                scalar(
                    &format!("size{k}"),
                    ["Width", "Height", "Depth"][k],
                    size[k],
                    0.001,
                    10.,
                );
            }
        }
        Operation::Revolve { segments } => {
            scalar("segments", "Segments", *segments as f32, 3., 64.)
        }
        Operation::Transform {
            translation,
            rotation,
            scale,
        } => {
            for (name, values, range) in [
                ("position", translation, (-10., 10.)),
                ("rotation", rotation, (-180., 180.)),
                ("scale", scale, (0.001, 10.)),
            ] {
                for k in 0..3 {
                    scalar(
                        &format!("{name}{k}"),
                        ["X", "Y", "Z"][k],
                        values[k],
                        range.0,
                        range.1,
                    );
                }
                p.push(property(
                    name,
                    name,
                    (0..3).map(|k| format!("{name}{k}")).collect(),
                    MaterialPropertyKind::Vector,
                    vec![],
                ));
            }
        }
        Operation::Grip {
            translation,
            rotation,
        } => {
            for (name, values, range) in [
                ("position", translation, (-10., 10.)),
                ("rotation", rotation, (-180., 180.)),
            ] {
                for k in 0..3 {
                    scalar(
                        &format!("{name}{k}"),
                        ["X", "Y", "Z"][k],
                        values[k],
                        range.0,
                        range.1,
                    );
                }
                p.push(property(
                    name,
                    name,
                    (0..3).map(|k| format!("{name}{k}")).collect(),
                    MaterialPropertyKind::Vector,
                    vec![],
                ));
            }
        }
        Operation::Material {
            palette,
            roughness,
            metallic,
        } => {
            scalar("palette", "Palette slot", *palette as f32, 0., 255.);
            scalar("roughness", "Roughness", *roughness, 0., 1.);
            scalar("metallic", "Metallic", *metallic, 0., 1.);
        }
        Operation::Reference { row } | Operation::AssignMaterial { row } => {
            scalar(
                "row",
                "Source row",
                doc.rows.iter().position(|r| &r.id == row).unwrap_or(0) as f32,
                0.,
                doc.rows.len().saturating_sub(1) as f32,
            );
            p.push(property(
                "row",
                "Source row",
                vec!["row".into()],
                MaterialPropertyKind::Choice,
                doc.rows.iter().map(|r| r.label.clone()).collect(),
            ));
        }
        Operation::StateSwitch { off, on } => {
            for (key, label, row) in [("off", "Off / Closed", off), ("on", "On / Open", on)] {
                scalar(key, label, doc.rows.iter().position(|r| &r.id == row).unwrap_or(0) as f32,
                    0., doc.rows.len().saturating_sub(1) as f32);
                p.push(property(key, label, vec![key.into()], MaterialPropertyKind::Choice,
                    doc.rows.iter().map(|r| r.label.clone()).collect()));
            }
        }
        Operation::Assemble { rows } => {
            for (i, row) in rows.iter().enumerate() {
                let id = format!("row{i}");
                scalar(
                    &id,
                    &format!("Part {}", i + 1),
                    doc.rows.iter().position(|r| &r.id == row).unwrap_or(0) as f32,
                    0.,
                    doc.rows.len().saturating_sub(1) as f32,
                );
                p.push(property(
                    &id,
                    &format!("Part {}", i + 1),
                    vec![id.clone()],
                    MaterialPropertyKind::Choice,
                    doc.rows.iter().map(|r| r.label.clone()).collect(),
                ));
            }
        }
    }
    // When a tile uses grouped controls, retain any remaining scalar controls.
    if !p.is_empty() {
        for value in &n {
            if !p.iter().any(|p| p.components.contains(&value.id)) {
                p.push(property(
                    &value.id,
                    &value.label,
                    vec![value.id.clone()],
                    MaterialPropertyKind::Number,
                    vec![],
                ));
            }
        }
    }
    (n, p)
}
#[cfg(test)]
pub fn view(doc: &Document, evaluation: &Evaluation, palette: &ThePalette) -> MaterialEditorView {
    view_selected(doc, evaluation, palette, &Default::default())
}
pub fn view_selected(
    doc: &Document,
    evaluation: &Evaluation,
    palette: &ThePalette,
    selection: &super::rigging::Selection,
) -> MaterialEditorView {
    let mut view = MaterialEditorView {
        geometry: true,
        row_types: evaluation
            .rows
            .iter()
            .map(|(id, v)| (id.clone(), v.kind().into()))
            .collect(),
        title: doc.label.clone(),
        ..Default::default()
    };
    view.palette = palette
        .colors
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            c.as_ref().map(|c| MaterialPaletteEntry {
                index: i as u16,
                label: format!("{} {}", i, c.to_hex()),
                color: c.to_u8_array(),
            })
        })
        .collect();
    view.rows = doc
        .rows
        .iter()
        .map(|row| MaterialRowView {
            id: row.id.clone(),
            label: format!(
                "{} · {}",
                row.label,
                evaluation.rows.get(&row.id).map_or("Empty", Value::kind)
            ),
            operations: row
                .tiles
                .iter()
                .map(|tile| {
                    let (mut parameters, mut properties) =
                        parameters(&tile.id, &tile.op, doc, evaluation, selection);
                    if let Operation::Texture { parts: targets, .. } = &tile.op {
                        if let Some(Value::Mesh(mesh)) = evaluation.operations.get(&tile.id) {
                            if let Some(p) = properties.iter_mut().find(|p| p.id == "texture_part")
                            {
                                p.choices = std::iter::once("All parts".into())
                                    .chain(mesh.parts.iter().map(|p| p.label.clone()))
                                    .collect();
                            }
                            if let Some(n) = parameters.iter_mut().find(|n| n.id == "texture_part")
                            {
                                n.max = mesh.parts.len() as f32;
                                n.value = targets
                                    .first()
                                    .and_then(|id| mesh.parts.iter().position(|p| &p.id == id))
                                    .map_or(0., |i| (i + 1) as f32);
                            }
                        }
                    }
                    super::groups::colors(
                        tile,
                        palette,
                        &mut parameters,
                        &mut view.palette_bindings,
                    );
                    let palette_index = match &tile.op {
                        Operation::Material { palette, .. } => Some(*palette),
                        Operation::Profiles { parts, .. } => parts
                            .get(selection.part.min(parts.len().saturating_sub(1)))
                            .map(|p| p.surface.palette),
                        _ => None,
                    };
                    if let Some(index) = palette_index {
                        if let Some(c) = palette.colors.get(index as usize).and_then(Option::as_ref)
                        {
                            let color = c.to_u8_array();
                            for k in 0..3 {
                                let s = color[k] as f32 / 255.;
                                let v = if s <= 0.04045 {
                                    s / 12.92
                                } else {
                                    ((s + 0.055) / 1.055).powf(2.4)
                                };
                                parameters.push(number(format!("color:{k}"), "Color", v, 0., 1.));
                            }
                            if properties.is_empty() {
                                for n in &parameters {
                                    if !n.id.starts_with("color:") && n.id != "palette" {
                                        properties.push(property(
                                            &n.id,
                                            &n.label,
                                            vec![n.id.clone()],
                                            MaterialPropertyKind::Number,
                                            vec![],
                                        ));
                                    }
                                }
                            }
                            properties.insert(
                                0,
                                property(
                                    "color",
                                    "Color",
                                    (0..3).map(|k| format!("color:{k}")).collect(),
                                    MaterialPropertyKind::Color,
                                    vec![],
                                ),
                            );
                            view.palette_bindings.push(MaterialPaletteBindingView {
                                operation: tile.id.clone(),
                                parameter: "color".into(),
                                index,
                            });
                        }
                    }
                    if let Operation::ParticleEffect { effect } = &tile.op {
                        for i in 0..4 {
                            let key = format!("effect_color{i}");
                            let color = palette
                                .colors
                                .get(effect.palette[i] as usize)
                                .and_then(Option::as_ref)
                                .map(TheColor::to_array)
                                .unwrap_or([0.; 4]);
                            for k in 0..3 {
                                parameters.push(number(
                                    format!("{key}:{k}"),
                                    "Color",
                                    color[k],
                                    0.,
                                    1.,
                                ));
                            }
                            properties.push(property(
                                &key,
                                ["Core", "Flame", "Edge", "Fade"][i],
                                (0..3).map(|k| format!("{key}:{k}")).collect(),
                                MaterialPropertyKind::Color,
                                vec![],
                            ));
                            view.palette_bindings.push(MaterialPaletteBindingView {
                                operation: tile.id.clone(),
                                parameter: key,
                                index: effect.palette[i],
                            });
                        }
                    }
                    MaterialOperationView {
                        id: tile.id.clone(),
                        label: match &tile.op {
                            Operation::Group { label, .. } => label.clone(),
                            Operation::Connector { .. } => tile.id.replace('-', " "),
                            Operation::Texture { source, .. }
                                if source.starts_with("ruleset:material/") =>
                            {
                                source
                                    .trim_start_matches("ruleset:material/")
                                    .replace('-', " ")
                            }
                            _ => tile.op.name().into(),
                        },
                        thumbnail: tile.id.clone(),
                        parameters,
                        properties,
                        palette: None,
                        combine: None,
                        source: if let Operation::Connector { family, .. } = &tile.op {
                            Some(MaterialSourceCodeView { code: family.clone(), help: "Connector family: enter a matching key such as wall or roof. Opposing connectors in the same family can snap. Local +Z points out of the connection.".into(), references: "Use letters, digits, _, -, . or /. Position and rotation can be bound to published recipe controls.".into() })
                        } else { None },
                    }
                })
                .collect(),
            inputs: row
                .tiles
                .iter()
                .flat_map(|tile| {
                    tile.op
                        .references()
                        .into_iter()
                        .enumerate()
                        .map(move |(i, source)| MaterialInputView {
                            target: MaterialInputTarget::RowLink {
                                row: row.id.clone(),
                                connection: format!("{}:{i}", tile.id),
                            },
                            label: tile.op.name().into(),
                            components: "x".into(),
                            binding: MaterialInputBinding::Row(MaterialReferenceView {
                                row: source.into(),
                                components: "x".into(),
                                mapping: None,
                            }),
                        })
                })
                .collect(),
        })
        .collect();
    view.outputs = vec![
        MaterialOutputView {
            id: "mesh".into(),
            label: "Mesh".into(),
            row: Some(doc.output.clone()),
            fallback: String::new(),
        },
        MaterialOutputView {
            id: "grip".into(),
            label: "Grip".into(),
            row: doc.grip.clone(),
            fallback: "None".into(),
        },
    ];
    view.sources = [
        ("contour", "Contour", "Geometry"),
        ("extrude", "Extrude", "Geometry"),
        ("footprint", "Footprint", "Geometry"),
        ("connector", "Connector", "Geometry"),
        ("box", "Box", "Geometry"),
        ("input", "Selection input", "Geometry"),
        ("fit_to", "Fit To Geometry", "Geometry"),
        ("wearer", "Wearer body", "Geometry"),
        ("select_parts", "Select Parts", "Geometry"),
        ("surface_offset", "Surface Offset", "Geometry"),
        ("revolve", "Revolve", "Geometry"),
        ("transform", "Transform", "Transform"),
        ("repeat", "Repeat", "Geometry"),
        ("reference", "Row", "Geometry"),
        ("assemble", "Assemble", "Geometry"),
        ("state_switch", "State Switch (Off / On)", "Geometry"),
        ("material", "PBR Material", "Color"),
        ("assign", "Material", "Color"),
        ("grip", "Grip", "Rigging"),
        ("loft", "Loft", "Geometry"),
        ("joint", "Joint", "Rigging"),
        ("skeleton", "Skeleton", "Rigging"),
        ("skin", "Skin", "Rigging"),
        ("attach", "Attach", "Rigging"),
        ("motion", "Motion", "Animation"),
        ("armature", "Skeleton Tree", "Rigging"),
        ("profiles", "Bone Profiles", "Geometry"),
        ("texture", "Material Texture", "Color"),
        ("particle_effect", "Particle Effect", "Effects"),
        ("group", "Group", "Geometry"),
    ]
    .into_iter()
    .map(|(id, label, category)| MaterialSourceView {
        id: id.into(),
        label: label.into(),
        category: category.into(),
    })
    .collect();
    view
}
pub fn add_project_sources(
    view: &mut MaterialEditorView,
    project: &Project,
    selected: Option<Uuid>,
) {
    for (id, source) in &project.procedural_appearances {
        if Some(*id) == selected {
            continue;
        }
        if let Ok(doc) = Document::load(source) {
            view.sources.push(MaterialSourceView {
                id: format!("geometry_asset:{id}"),
                label: doc.label,
                category: "Geometry".into(),
            });
        }
    }
}
fn tile_mut<'a>(doc: &'a mut Document, id: &str) -> Result<&'a mut Tile, String> {
    doc.rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find(|t| t.id == id)
        .ok_or("Operation no longer exists".into())
}
#[cfg(test)]
pub fn edit(doc: &mut Document, command: MaterialEditorCommand) -> Result<(), String> {
    edit_selected(doc, command, &Default::default())
}
pub fn edit_selected(
    doc: &mut Document,
    command: MaterialEditorCommand,
    selection: &super::rigging::Selection,
) -> Result<(), String> {
    match command {
        MaterialEditorCommand::SetSource { operation, code } => {
            let Operation::Connector { family, .. } = &mut tile_mut(doc, &operation)?.op else {
                return Err("This operation has no editable source".into());
            };
            *family = code.trim().into();
        }
        MaterialEditorCommand::SetNumber {
            operation,
            parameter,
            value,
        } => {
            if !value.is_finite() {
                return Err("Geometry values must be finite".into());
            }
            let row_id = doc
                .rows
                .get(value.round().max(0.) as usize)
                .map(|r| r.id.clone());
            let evaluation = doc.evaluate().map_err(|e| e.to_string())?;
            let rows = doc.rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
            let footprint_rows = doc
                .rows
                .iter()
                .take_while(|r| r.tiles.iter().all(|t| t.id != operation))
                .filter(|r| matches!(evaluation.rows.get(&r.id), Some(Value::Contour(_))))
                .map(|r| r.id.clone())
                .collect::<Vec<_>>();
            let tile = tile_mut(doc, &operation)?;
            if let Operation::SelectParts { parts } = &mut tile.op {
                if let Some(id) = parameter.strip_prefix("selected:") {
                    if value >= 0.5 {
                        if !parts.iter().any(|part| part == id) {
                            parts.push(id.into());
                        }
                    } else {
                        parts.retain(|part| part != id);
                    }
                    return doc.evaluate().map(|_| ()).map_err(|e| e.to_string());
                }
            }
            if let Operation::Texture {
                parts,
                axes,
                origin,
                span,
                ..
            } = &mut tile.op
            {
                if let Some(Value::Mesh(mesh)) = evaluation.operations.get(&operation) {
                    if parameter == "texture_part" {
                        *parts = if value < 0.5 {
                            vec![]
                        } else {
                            vec![
                                mesh.parts
                                    .get(value.round() as usize - 1)
                                    .ok_or("Missing target")?
                                    .id
                                    .clone(),
                            ]
                        };
                        return doc.evaluate().map(|_| ()).map_err(|e| e.to_string());
                    }
                    if parameter == "texture_fit" {
                        let selected = mesh
                            .parts
                            .iter()
                            .filter(|p| parts.is_empty() || parts.contains(&p.id))
                            .flat_map(|p| &p.vertices)
                            .collect::<Vec<_>>();
                        if selected.is_empty() {
                            return Err("No target geometry".into());
                        }
                        for k in 0..2 {
                            let lo = selected
                                .iter()
                                .map(|v| v.position[axes[k]])
                                .fold(f32::INFINITY, f32::min);
                            let hi = selected
                                .iter()
                                .map(|v| v.position[axes[k]])
                                .fold(f32::NEG_INFINITY, f32::max);
                            origin[k] = lo;
                            span[k] = (hi - lo).max(0.001);
                        }
                        return doc.evaluate().map(|_| ()).map_err(|e| e.to_string());
                    }
                }
            }
            let rig = match &tile.op {
                Operation::Loft { skin, .. } => {
                    let source = if parameter == "loft_rig" {
                        rows.get(value.round().max(1.) as usize - 1)
                            .map(String::as_str)
                    } else {
                        skin.as_ref().map(|b| b.rig.as_str())
                    };
                    source
                        .and_then(|id| evaluation.rows.get(id))
                        .and_then(Value::rig)
                }
                Operation::ParticleEffect { .. } => {
                    evaluation.operations.get(&operation).and_then(Value::rig)
                }
                Operation::Skin { rig, .. }
                | Operation::Attach { rig, .. }
                | Operation::Motion { rig, .. } => evaluation.rows.get(rig).and_then(Value::rig),
                _ => None,
            };
            match &mut tile.op {
                Operation::ParticleEffect { effect } => {
                    super::effects::edit(effect, &parameter, value, rig)?
                }
                Operation::Asset { .. } => {
                    return Err("Open the linked geometry asset to edit its construction".into());
                }
                Operation::Input { .. } => return Err("Geometry inputs are supplied by the host selection; edit their portable preview in the recipe source".into()),
                Operation::Footprint { row, holes, append } => {
                    if parameter=="footprint_row" { *row=footprint_rows.get(value.round().max(0.) as usize).cloned().ok_or("Missing contour row")?; }
                    else if parameter=="footprint_append" { *append=value>=0.5; }
                    else if parameter=="footprint_add_hole" {
                        if value>=1. && holes.len()<32 {
                            let available=footprint_rows.iter().filter(|r|r.as_str()!=row.as_str() && !holes.contains(r)).cloned().collect::<Vec<_>>();
                            holes.push(available.get(value.round() as usize-1).cloned().ok_or("Missing hole contour")?);
                        }
                    } else if let Some(i)=parameter.strip_prefix("footprint_hole:").and_then(|s|s.parse::<usize>().ok()).filter(|i|*i<holes.len()) {
                        if value<0.5 {holes.remove(i);} else {holes[i]=footprint_rows.get(value.round() as usize-1).cloned().ok_or("Missing hole contour")?;}
                    } else {return Err("Unknown footprint parameter".into());}
                }
                Operation::FitTo { target, axes, source_anchor, target_anchor, padding } => {
                    if parameter == "target" { *target = row_id.ok_or("Missing target row")?; }
                    else {
                        let k=parameter.chars().last().and_then(|c|c.to_digit(10)).filter(|k|*k<3).ok_or("Invalid fit axis")? as usize;
                        if parameter.starts_with("fit") { axes[k]=value>=0.5; }
                        else if parameter.starts_with("source_anchor") { source_anchor[k]=value; }
                        else if parameter.starts_with("target_anchor") { target_anchor[k]=value; }
                        else if parameter.starts_with("padding") { padding[k]=value; }
                        else { return Err("Unknown fit parameter".into()); }
                    }
                }
                op @ (Operation::Armature { .. }
                | Operation::Profiles { .. }
                | Operation::Texture { .. }) => {
                    super::authoring_params::edit(op, &parameter, value, selection)?
                }
                Operation::Group { controls, .. } => {
                    super::groups::edit(controls, &parameter, value, selection)?
                }
                op @ (Operation::Loft { .. }
                | Operation::Joint { .. }
                | Operation::Skeleton { .. }
                | Operation::Skin { .. }
                | Operation::Attach { .. }
                | Operation::Motion { .. }) => {
                    super::rigging::edit(op, &parameter, value, &rows, rig, selection)?
                }
                Operation::Contour { points } => {
                    let (i, k) = parameter
                        .strip_prefix("point")
                        .and_then(|s| s.split_once(':'))
                        .ok_or("Invalid point")?;
                    let p = points
                        .get_mut(i.parse::<usize>().map_err(|_| "Invalid point")?)
                        .ok_or("Missing point")?;
                    let k = k.parse::<usize>().map_err(|_| "Invalid axis")?;
                    *p.get_mut(k).ok_or("Invalid axis")? = value;
                }
                Operation::Extrude { depth } => *depth = value,
                Operation::Repeat { count, translation, rotation } => {
                    if parameter == "count" { *count = value.round().max(0.) as u32; }
                    else if let Some(k) = parameter.strip_prefix("step").and_then(|s| s.parse::<usize>().ok()) {
                        *translation.get_mut(k).ok_or("Invalid repeat axis")? = value;
                    } else if let Some(k) = parameter.strip_prefix("rotation").and_then(|s| s.parse::<usize>().ok()) {
                        *rotation.get_mut(k).ok_or("Invalid repeat rotation")? = value;
                    } else { return Err("Unknown repeat input".into()); }
                }
                Operation::SurfaceOffset { distance } => *distance = value,
                Operation::SelectParts { .. } => return Err("Choose a mesh part".into()),
                Operation::Revolve { segments } => *segments = value.round().max(0.) as u32,
                Operation::Box { size } => {
                    let k = parameter
                        .strip_prefix("size")
                        .and_then(|s| s.parse::<usize>().ok())
                        .ok_or("Invalid size axis")?;
                    *size.get_mut(k).ok_or("Invalid size axis")? = value;
                }
                Operation::Transform {
                    translation,
                    rotation,
                    scale,
                } => set_transform(&parameter, value, translation, rotation, Some(scale))?,
                Operation::Grip { translation, rotation } | Operation::Connector { translation, rotation, .. } => set_transform(&parameter, value, translation, rotation, None)?,
                Operation::Material {
                    palette,
                    roughness,
                    metallic,
                } => match parameter.as_str() {
                    "palette" => *palette = value.round().clamp(0., 255.) as u16,
                    "roughness" => *roughness = value,
                    "metallic" => *metallic = value,
                    _ => return Err("Use the project palette picker for colors".into()),
                },
                Operation::Reference { row } | Operation::AssignMaterial { row } => {
                    *row = row_id.ok_or("Missing source row")?
                }
                Operation::StateSwitch { off, on } => {
                    let row = row_id.ok_or("Missing state row")?;
                    match parameter.as_str() { "off" => *off = row, "on" => *on = row,
                        _ => return Err("Unknown state input".into()) }
                }
                Operation::Assemble { rows } => {
                    let i = parameter
                        .strip_prefix("row")
                        .and_then(|s| s.parse::<usize>().ok())
                        .ok_or("Invalid part")?;
                    *rows.get_mut(i).ok_or("Missing part")? = row_id.ok_or("Missing row")?;
                }
            }
        }
        MaterialEditorCommand::SetPaletteParameter {
            operation,
            parameter,
            index,
        } => match &mut tile_mut(doc, &operation)?.op {
            Operation::Profiles { parts, .. } => {
                let len = parts.len();
                parts
                    .get_mut(selection.part.min(len.saturating_sub(1)))
                    .ok_or("No profile selected")?
                    .surface
                    .palette = index;
            }
            Operation::Group { controls, .. } => {
                super::groups::edit(controls, &parameter, index as f32, selection)?
            }
            Operation::ParticleEffect { effect } => {
                let i = parameter
                    .strip_prefix("effect_color")
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or("Invalid particle color")?;
                *effect.palette.get_mut(i).ok_or("Invalid particle color")? = index;
            }
            Operation::Material { palette, .. } => *palette = index,
            _ => return Err("Select a palette color control".into()),
        },
        MaterialEditorCommand::SetPalette { operation, index } => {
            let Operation::Material { palette, .. } = &mut tile_mut(doc, &operation)?.op else {
                return Err("Select a material row".into());
            };
            *palette = index;
        }
        MaterialEditorCommand::ConnectOutput { output, row } => {
            if output == "mesh" {
                doc.output = row;
            } else if output == "grip" {
                doc.grip = Some(row);
            }
        }
        MaterialEditorCommand::DisconnectOutput(output) => {
            if output == "grip" {
                doc.grip = None;
            } else {
                return Err("A geometry asset needs a Mesh output".into());
            }
        }
        MaterialEditorCommand::RemoveOperation(id) => {
            for row in &mut doc.rows {
                row.tiles.retain(|t| t.id != id);
            }
        }
        MaterialEditorCommand::AddSource { source, row, after } => {
            let evaluation = doc.evaluate().map_err(|e| e.to_string())?;
            let owner = row
                .as_ref()
                .and_then(|id| doc.rows.iter().position(|r| &r.id == id))
                .unwrap_or(doc.rows.len());
            let earlier = &doc.rows[..owner];
            let first_mesh = earlier
                .iter()
                .find(|r| matches!(evaluation.rows.get(&r.id), Some(Value::Mesh(_))))
                .map(|r| r.id.clone());
            let first_material = earlier
                .iter()
                .find(|r| matches!(evaluation.rows.get(&r.id), Some(Value::Material(_))))
                .map(|r| r.id.clone());
            let rig_row = earlier
                .iter()
                .rev()
                .find(|r| evaluation.rows.get(&r.id).and_then(Value::rig).is_some())
                .map(|r| r.id.clone());
            let first_joint = rig_row
                .as_ref()
                .and_then(|id| evaluation.rows.get(id))
                .and_then(Value::rig)
                .and_then(|r| r.joints.first().map(|j| j.id.clone()));
            let op = match source.as_str() {
                "armature"=>Operation::Armature{joints:vec![denrim_materials::geometry::authoring::JointDef{id:Uuid::new_v4().to_string(),label:"Root".into(),parent:None,translation:[0.;3],rotation:[0.;3]}]},
                "profiles"=>Operation::Profiles{rig:rig_row.clone().ok_or("Add a skeleton first")?,parts:vec![]},
                "particle_effect"=>Operation::ParticleEffect{effect:Default::default()},
                "texture"=>Operation::Texture{preserve_uv:false,parts:vec![],source:"ruleset:icon/round_shield/on".into(),bundle:None,size:128,axes:[0,1],origin:[-0.5;2],span:[1.;2]},
                "group"=>Operation::Group{label:"Group".into(),document:Box::new(Document{version:1,id:Uuid::new_v4().to_string(),label:"Group".into(),rows:vec![Row{id:"material".into(),label:"Material".into(),tiles:vec![Tile{id:"group-material".into(),op:Operation::Material{palette:7,roughness:0.5,metallic:0.}}]},Row{id:"box".into(),label:"Box".into(),tiles:vec![Tile{id:"group-box".into(),op:Operation::Box{size:[0.2,0.2,0.2]}},Tile{id:"group-surface".into(),op:Operation::AssignMaterial{row:"material".into()}}]}],output:"box".into(),grip:None}),controls:vec![]},
                "loft"=>Operation::Loft{sections:vec![denrim_materials::geometry::Section{center:[0.,0.,0.],radius:[0.1,0.1]},denrim_materials::geometry::Section{center:[0.,0.3,0.],radius:[0.1,0.1]}],segments:12,material:None,skin:None},
                "joint"=>Operation::Joint{parent:earlier.iter().rev().find(|r|matches!(evaluation.rows.get(&r.id),Some(Value::Rig(rig)) if rig.tip.is_some())).map(|r|r.id.clone()),translation:[0.,0.2,0.],rotation:[0.;3]},
                "skeleton"=>Operation::Skeleton{rows:earlier.iter().filter(|r|matches!(evaluation.rows.get(&r.id),Some(Value::Rig(_)))).map(|r|r.id.clone()).collect()},
                "skin"=>Operation::Skin{rig:rig_row.ok_or("Add a skeleton first")?,joints:vec![first_joint.ok_or("Add a joint first")?],blend:0.5},
                "attach"=>Operation::Attach{enabled:true,rig:rig_row.ok_or("Add a skeleton first")?,joint:first_joint.ok_or("Add a joint first")?,grip:None,translation:[0.;3],rotation:[0.;3]},
                "motion"=>Operation::Motion{events:vec![],rig:rig_row.ok_or("Add a skeleton first")?,duration:1.,looping:false,tracks:vec![denrim_materials::geometry::rig::Track{joint:first_joint.ok_or("Add a joint first")?,keys:vec![denrim_materials::geometry::rig::Key{visible:true,time:0.,translation:[0.;3],rotation:[0.;3]},denrim_materials::geometry::rig::Key{visible:true,time:1.,translation:[0.;3],rotation:[0.;3]}]}]},
                "contour" => Operation::Contour {
                    points: vec![[-0.1, 0.], [0.1, 0.], [0.1, 0.3], [-0.1, 0.3]],
                },
                "extrude" => Operation::Extrude { depth: 0.03 },
                "connector" => Operation::Connector { family:"wall".into(),translation:[0.;3],rotation:[0.;3] },
                "footprint" => Operation::Footprint { holes:vec![],append:false,row: earlier.iter().rev().find(|r| matches!(evaluation.rows.get(&r.id), Some(Value::Contour(_)))).map(|r| r.id.clone()).ok_or("Add a footprint Contour row above this mesh row first")? },
                "wearer" => Operation::Asset{source:denrim_materials::geometry::WEARER_SOURCE.into(),document:Box::new(Document::load(denrim_materials::geometry::HUMAN_EXAMPLE).and_then(|d|d.wearer_body()).map_err(|e|e.to_string())?)},
                "select_parts" => {
                    let mesh = row.as_ref().and_then(|id|doc.rows.iter().find(|r|&r.id==id))
                        .and_then(|r| after.as_ref().and_then(|id|r.tiles.iter().find(|t|&t.id==id)).or_else(||r.tiles.last()))
                        .and_then(|tile|evaluation.operations.get(&tile.id));
                    let Some(Value::Mesh(mesh))=mesh else{return Err("Add Select Parts after a mesh source".into())};
                    Operation::SelectParts{parts:mesh.parts.iter().map(|p|p.id.clone()).collect()}
                },
                "surface_offset" => Operation::SurfaceOffset{distance:0.01},
                "box" => Operation::Box {
                    size: [0.2, 0.2, 0.2],
                },
                "input" => Operation::Input { name:"selection".into(), preview:Box::new(Document { version:1,id:"selection-preview".into(),label:"Selection preview".into(),rows:vec![Row{id:"preview".into(),label:"Preview bounds".into(),tiles:vec![Tile{id:"preview-box".into(),op:Operation::Box {size:[4.,2.,4.]}}]}],output:"preview".into(),grip:None }) },
                "fit_to" => Operation::FitTo { target:first_mesh.ok_or("Add a selection input row first")?, axes:[true,false,true],source_anchor:[0.5,0.,0.5],target_anchor:[0.5,1.,0.5],padding:[0.;3] },
                "revolve" => Operation::Revolve { segments: 12 },
                "repeat" => Operation::Repeat { count: 3, translation:[0.3,0.,0.], rotation:[0.;3] },
                "transform" => Operation::Transform {
                    translation: [0.; 3],
                    rotation: [0.; 3],
                    scale: [1.; 3],
                },
                "reference" => Operation::Reference {
                    row: first_mesh.ok_or("Add a mesh row first")?,
                },
                "state_switch" => {
                    let row = first_mesh.ok_or("Add Off and On mesh rows first")?;
                    Operation::StateSwitch { off: row.clone(), on: row }
                }
                "assemble" => Operation::Assemble {
                    rows: earlier
                        .iter()
                        .filter(|r| matches!(evaluation.rows.get(&r.id), Some(Value::Mesh(_))))
                        .map(|r| r.id.clone())
                        .collect(),
                },
                "material" => Operation::Material {
                    palette: 7,
                    roughness: 0.5,
                    metallic: 0.,
                },
                "assign" => Operation::AssignMaterial {
                    row: first_material.ok_or("Add a material row above this row")?,
                },
                "grip" => Operation::Grip {
                    translation: [0.; 3],
                    rotation: [0.; 3],
                },
                _ => return Err("Unsupported geometry source".into()),
            };
            if owner == doc.rows.len() {
                doc.rows.push(Row {
                    id: Uuid::new_v4().to_string(),
                    label: op.name().into(),
                    tiles: vec![],
                });
            }
            let tiles = &mut doc.rows[owner].tiles;
            let index = after
                .and_then(|id| tiles.iter().position(|t| t.id == id))
                .map_or(0, |i| i + 1);
            tiles.insert(
                index,
                Tile {
                    id: Uuid::new_v4().to_string(),
                    op,
                },
            );
        }
        MaterialEditorCommand::MoveOperation {
            operation,
            row,
            after,
        } => {
            let tile = tile_mut(doc, &operation)?.clone();
            for r in &mut doc.rows {
                r.tiles.retain(|t| t.id != operation);
            }
            let tiles = &mut doc
                .rows
                .iter_mut()
                .find(|r| r.id == row)
                .ok_or("Missing target row")?
                .tiles;
            let at = after
                .and_then(|id| tiles.iter().position(|t| t.id == id))
                .map_or(0, |i| i + 1);
            tiles.insert(at, tile);
        }
        MaterialEditorCommand::SetInput {
            target: MaterialInputTarget::RowLink { connection, .. },
            binding: MaterialInputBinding::Row(reference),
        } => {
            let (id, index) = connection.rsplit_once(':').ok_or("Missing input")?;
            match &mut tile_mut(doc, id)?.op {
                Operation::Profiles { rig, .. } => *rig = reference.row,
                Operation::Loft { material, skin, .. } => {
                    if material.is_some() && index == "0" {
                        *material = Some(reference.row);
                    } else {
                        skin.as_mut().ok_or("No skin binding")?.rig = reference.row;
                    }
                }
                Operation::Joint { parent, .. } => *parent = Some(reference.row),
                Operation::Skin { rig, .. } | Operation::Motion { rig, .. } => *rig = reference.row,
                Operation::Attach { rig, grip, .. } => {
                    if index == "0" {
                        *rig = reference.row
                    } else {
                        *grip = Some(reference.row)
                    }
                }
                Operation::Footprint { row, holes, .. } => {
                    let i = index
                        .parse::<usize>()
                        .map_err(|_| "Invalid footprint input")?;
                    if i == 0 {
                        *row = reference.row;
                    } else {
                        *holes.get_mut(i - 1).ok_or("Missing footprint hole")? = reference.row;
                    }
                }
                Operation::Reference { row } | Operation::AssignMaterial { row } => {
                    *row = reference.row
                }
                Operation::StateSwitch { off, on } => match index {
                    "0" | "off" => *off = reference.row,
                    "1" | "on" => *on = reference.row,
                    _ => return Err("Invalid state input".into()),
                },
                Operation::Assemble { rows } | Operation::Skeleton { rows } => {
                    let i = index.parse::<usize>().map_err(|_| "Invalid input")?;
                    *rows.get_mut(i).ok_or("Missing input")? = reference.row;
                }
                _ => return Err("This tile has no row input".into()),
            }
        }
        _ => return Err("This control is not available for geometry yet".into()),
    }
    doc.evaluate().map_err(|e| e.to_string())?;
    Ok(())
}
pub(super) fn set_transform(
    key: &str,
    value: f32,
    t: &mut [f32; 3],
    r: &mut [f32; 3],
    s: Option<&mut [f32; 3]>,
) -> Result<(), String> {
    for (name, vector) in [("position", Some(t)), ("rotation", Some(r)), ("scale", s)] {
        if let Some(index) = key.strip_prefix(name).and_then(|s| s.parse::<usize>().ok()) {
            *vector
                .ok_or("No scale on this frame")?
                .get_mut(index)
                .ok_or("Invalid axis")? = value;
            return Ok(());
        }
    }
    Err("Invalid transform component".into())
}
#[derive(Clone)]
pub struct ContourFrame {
    center: [f32; 2],
    scale: f32,
    size: [f32; 2],
    screen: Vec<[f32; 2]>,
}
impl ContourFrame {
    pub fn hit(&self, p: Vec2<i32>) -> Option<usize> {
        self.screen
            .iter()
            .enumerate()
            .filter(|(_, v)| (v[0] - p.x as f32).hypot(v[1] - p.y as f32) < 12.)
            .min_by(|(_, a), (_, b)| {
                ((a[0] - p.x as f32).hypot(a[1] - p.y as f32))
                    .total_cmp(&((b[0] - p.x as f32).hypot(b[1] - p.y as f32)))
            })
            .map(|(i, _)| i)
    }
    pub fn unproject(&self, p: Vec2<i32>) -> [f32; 2] {
        [
            (p.x as f32 - self.size[0] / 2.) / self.scale + self.center[0],
            -(p.y as f32 - self.size[1] / 2.) / self.scale + self.center[1],
        ]
    }
    fn project(&self, p: [f32; 2]) -> [f32; 2] {
        [
            (p[0] - self.center[0]) * self.scale + self.size[0] / 2.,
            -(p[1] - self.center[1]) * self.scale + self.size[1] / 2.,
        ]
    }
}
pub fn contour_image(
    points: &[[f32; 2]],
    width: i32,
    height: i32,
    theme: &dyn TheTheme,
    fixed: Option<&ContourFrame>,
) -> (TheRGBABuffer, Option<ContourFrame>) {
    let w = width.max(1) as usize;
    let h = height.max(1) as usize;
    let mut pixels = theme.color(MaterialCanvas).repeat(w * h);
    let mut frame = None;
    if !points.is_empty() {
        let lo: [f32; 2] =
            std::array::from_fn(|k| points.iter().map(|p| p[k]).fold(f32::INFINITY, f32::min));
        let hi: [f32; 2] = std::array::from_fn(|k| {
            points
                .iter()
                .map(|p| p[k])
                .fold(f32::NEG_INFINITY, f32::max)
        });
        let mut f = fixed.cloned().unwrap_or(ContourFrame {
            center: std::array::from_fn(|k| (lo[k] + hi[k]) / 2.),
            scale: ((w as f32 - if w >= 160 { 50. } else { 16. }).max(1.)
                / (hi[0] - lo[0]).max(0.01))
            .min((h as f32 - if w >= 160 { 65. } else { 16. }).max(1.) / (hi[1] - lo[1]).max(0.01)),
            size: [w as f32, h as f32],
            screen: vec![],
        });
        f.screen = points.iter().map(|p| f.project(*p)).collect();
        for i in 0..points.len() {
            line(
                &mut pixels,
                w,
                h,
                f.screen[i],
                f.screen[(i + 1) % points.len()],
                *theme.color(MaterialWire),
            );
        }
        for p in &f.screen {
            for y in p[1] as i32 - 4..=p[1] as i32 + 4 {
                for x in p[0] as i32 - 4..=p[0] as i32 + 4 {
                    if (x as f32 - p[0]).hypot(y as f32 - p[1]) <= 4.5
                        && x >= 0
                        && y >= 0
                        && (x as usize) < w
                        && (y as usize) < h
                    {
                        let at = (y as usize * w + x as usize) * 4;
                        pixels[at..at + 4].copy_from_slice(theme.color(SelectedTextEditBorder1));
                    }
                }
            }
        }
        frame = Some(f);
    }
    let label = if points.is_empty() {
        "Select a Contour or Loft tile to edit its profile"
    } else {
        "Profile · drag points or rings"
    };
    if w >= 160 {
        TheDraw2D::new().text_rect(
            &mut pixels,
            &(8, 4, w.saturating_sub(16), 24),
            w,
            label,
            TheFontSettings {
                size: 11.,
                ..Default::default()
            },
            theme.color(MaterialText),
            theme.color(MaterialCanvas),
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
    }
    (TheRGBABuffer::from(pixels, w as u32, h as u32), frame)
}
pub(super) fn line(pixels: &mut [u8], w: usize, h: usize, a: [f32; 2], b: [f32; 2], color: RGBA) {
    let steps = ((a[0] - b[0]).abs().max((a[1] - b[1]).abs()).ceil() as usize)
        .max(1)
        .min(4096);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = (a[0] + (b[0] - a[0]) * t).round() as i32;
        let y = (a[1] + (b[1] - a[1]) * t).round() as i32;
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            let at = (y as usize * w + x as usize) * 4;
            pixels[at..at + 4].copy_from_slice(&color);
        }
    }
}

/// Small inspection thumbnails; the minimap uses the GPU renderer.
pub fn thumbnail(value: &Value, palette: &ThePalette, theme: &dyn TheTheme) -> TheRGBABuffer {
    const W: usize = 64;
    let bg = *theme.color(MaterialControl);
    let mut pixels = bg.repeat(W * W);
    match value {
        Value::Rig(rig) | Value::Motion(denrim_materials::geometry::rig::Motion { rig, .. }) => {
            let poses = rig.rest();
            let lo: [f32; 3] = std::array::from_fn(|k| {
                poses
                    .iter()
                    .map(|p| p.translation[k])
                    .fold(f32::INFINITY, f32::min)
            });
            let hi: [f32; 3] = std::array::from_fn(|k| {
                poses
                    .iter()
                    .map(|p| p.translation[k])
                    .fold(f32::NEG_INFINITY, f32::max)
            });
            let scale = 48. / (hi[1] - lo[1]).max(hi[0] - lo[0]).max(0.1);
            let project = |p: [f32; 3]| {
                [
                    32. + (p[0] - (lo[0] + hi[0]) / 2.) * scale,
                    32. - (p[1] - (lo[1] + hi[1]) / 2.) * scale,
                ]
            };
            for (i, j) in rig.joints.iter().enumerate() {
                if let Some(parent) = j.parent {
                    line(
                        &mut pixels,
                        W,
                        W,
                        project(poses[i].translation),
                        project(poses[parent].translation),
                        *theme.color(MaterialWire),
                    );
                }
            }
        }
        Value::Contour(points) => return contour_image(points, W as i32, W as i32, theme, None).0,
        Value::Material(surface) => {
            if let Some(c) = palette
                .colors
                .get(surface.palette as usize)
                .and_then(Option::as_ref)
            {
                let color = c.to_u8_array();
                for y in 7..57 {
                    for x in 7..57 {
                        pixels[(y * W + x) * 4..(y * W + x) * 4 + 4].copy_from_slice(&color);
                    }
                }
            }
        }
        Value::Grip(_) => {
            line(
                &mut pixels,
                W,
                W,
                [12., 44.],
                [52., 44.],
                *theme.color(SelectedTextEditBorder1),
            );
            line(
                &mut pixels,
                W,
                W,
                [32., 54.],
                [32., 10.],
                *theme.color(MaterialText),
            );
        }
        Value::Mesh(mesh) => {
            let (lo, hi) = mesh.bounds();
            let center: [f32; 3] = std::array::from_fn(|i| (lo[i] + hi[i]) / 2.);
            let extent = (0..3)
                .map(|i| (hi[i] - lo[i]).powi(2))
                .sum::<f32>()
                .sqrt()
                .max(0.01)
                * 0.6;
            let project = |p: [f32; 3]| {
                let p: [f32; 3] = std::array::from_fn(|i| p[i] - center[i]);
                let x = 0.866 * p[0] - 0.5 * p[2];
                let z = 0.5 * p[0] + 0.866 * p[2];
                [
                    32. + x / extent * 29.,
                    32. - (0.98 * p[1] - 0.2 * z) / extent * 29.,
                    z,
                ]
            };
            let edge = |a: [f32; 3], b: [f32; 3], x: f32, y: f32| {
                (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
            };
            let mut depth = vec![f32::NEG_INFINITY; W * W];
            for part in &mesh.parts {
                let color = part
                    .surface
                    .as_ref()
                    .and_then(|s| palette.colors.get(s.palette as usize))
                    .and_then(Option::as_ref)
                    .map(TheColor::to_u8_array)
                    .unwrap_or(*theme.color(MaterialText));
                for tri in part.vertices.chunks_exact(3) {
                    let t = [
                        project(tri[0].position),
                        project(tri[1].position),
                        project(tri[2].position),
                    ];
                    let area = edge(t[0], t[1], t[2][0], t[2][1]);
                    if area.abs() < 1e-5 {
                        continue;
                    }
                    let minx = t
                        .iter()
                        .map(|p| p[0])
                        .fold(f32::INFINITY, f32::min)
                        .floor()
                        .max(0.) as usize;
                    let maxx = t
                        .iter()
                        .map(|p| p[0])
                        .fold(f32::NEG_INFINITY, f32::max)
                        .ceil()
                        .clamp(0., 63.) as usize;
                    let miny = t
                        .iter()
                        .map(|p| p[1])
                        .fold(f32::INFINITY, f32::min)
                        .floor()
                        .max(0.) as usize;
                    let maxy = t
                        .iter()
                        .map(|p| p[1])
                        .fold(f32::NEG_INFINITY, f32::max)
                        .ceil()
                        .clamp(0., 63.) as usize;
                    let n = tri[0].normal;
                    let shade =
                        (0.4 + 0.6 * (n[0] * -0.4 + n[1] * 0.5 + n[2] * 0.7).abs()).clamp(0., 1.);
                    for y in miny..=maxy {
                        for x in minx..=maxx {
                            let w = [
                                edge(t[1], t[2], x as f32 + 0.5, y as f32 + 0.5) / area,
                                edge(t[2], t[0], x as f32 + 0.5, y as f32 + 0.5) / area,
                                edge(t[0], t[1], x as f32 + 0.5, y as f32 + 0.5) / area,
                            ];
                            if w.iter().any(|v| *v < 0.) {
                                continue;
                            }
                            let z = (0..3).map(|i| w[i] * t[i][2]).sum::<f32>();
                            let at = y * W + x;
                            if z <= depth[at] {
                                continue;
                            }
                            depth[at] = z;
                            pixels[at * 4..at * 4 + 4].copy_from_slice(&[
                                (color[0] as f32 * shade) as u8,
                                (color[1] as f32 * shade) as u8,
                                (color[2] as f32 * shade) as u8,
                                255,
                            ]);
                        }
                    }
                }
            }
        }
    }
    TheRGBABuffer::from(pixels, W as u32, W as u32)
}
