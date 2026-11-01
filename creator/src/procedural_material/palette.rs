//! Eldiron palette references for all editable procedural color constants.
//!
//! The native palette binding retains the index. Resolved linear RGBA values are
//! also written to the editable recipe so workers and flattened compositions can
//! consume a self-contained bundle without consulting UI/global palette state.
use super::*;
use denrim_materials::integration::{InstanceRecord, PaletteBinding, ParameterKey};
use std::collections::HashSet;

const RECORD: &str = "eldiron-palette";
const STARTER: &str = r#"project 1
material "surface" called "Procedural Tile"
  mode pbr
  tiling raw
  default roughness to 0.6
  default emission strength to 0
  row "palette-dark" called "Palette A"
    use "Color" id "palette-a"
  row "palette-light" called "Palette B"
    use "Color" id "palette-b"
  row "grain" called "Noise"
    use "Perlin Noise" id "color-noise"
  row "color" called "Surface"
    use "Mix" id "pigment"
      input from row "grain" xyz
      set "color1" from row "palette-dark" xyzw
      set "color2" from row "palette-light" xyzw
      set "factor" to 1 1 1
  output base color from row "color" xyz
  output opacity from row "color" w
  output roughness from row "grain" x
"#;

pub(super) fn starter(id: Uuid, project: &Project) -> Result<EditableMaterialAsset, String> {
    let mut asset = EditableMaterialAsset::parse(&id.to_string(), STARTER, "surface")
        .map_err(|e| e.to_string())?;
    let filled: Vec<_> = project
        .art_palette
        .colors
        .iter()
        .enumerate()
        .filter_map(|(index, color)| color.as_ref().map(|_| index as u16))
        .collect();
    let first = filled
        .first()
        .copied()
        .ok_or("The project palette has no colors")?;
    let preferred = |index: u16, fallback: u16| {
        if filled.contains(&index) {
            index
        } else {
            fallback
        }
    };
    // Eldiron's default palette has suitable dark/light greens at these slots;
    // custom palettes keep their own indices and fall back to their first colors.
    let a = preferred(37, first);
    let b = preferred(34, filled.get(1).copied().unwrap_or(first));
    asset = bind_color(&asset, "palette-a", a, &project.art_palette)?;
    asset = bind_color(&asset, "palette-b", b, &project.art_palette)?;
    ensure_color_bindings(&mut asset, &project.art_palette)?;
    Ok(asset)
}

pub(super) fn key(palette: &ThePalette) -> u64 {
    cache_key(&serde_json::to_string(&palette.colors).unwrap_or_default())
}

pub(super) fn binding_index(asset: &EditableMaterialAsset, operation: &str) -> Option<u16> {
    asset
        .instances
        .iter()
        .find(|record| record.id == RECORD)?
        .palette_bindings
        .iter()
        .find(|binding| {
            binding.key.asset == asset.root.id
                && binding.key.tile == operation
                && binding.key.parameter == "color"
        })
        .and_then(|binding| u16::try_from(binding.index).ok())
}

pub(super) fn bind_color(
    asset: &EditableMaterialAsset,
    operation: &str,
    index: u16,
    palette: &ThePalette,
) -> Result<EditableMaterialAsset, String> {
    bind_parameter(asset, operation, "color", index, palette)
}

pub(super) fn bind_parameter(
    asset: &EditableMaterialAsset,
    operation: &str,
    parameter: &str,
    index: u16,
    palette: &ThePalette,
) -> Result<EditableMaterialAsset, String> {
    linear_color(palette, index)?;
    let mut next = asset.clone();
    let operation = if operation == "@surface" {
        format!("@material:{}", asset.selected_material)
    } else {
        operation.into()
    };
    let key = ParameterKey::new(&next.root.id, &operation, parameter);
    let info = next
        .parameter_metadata()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|p| p.key == key)
        .ok_or("Color parameter is missing")?;
    if info.definition.kind != denrim_materials::metadata::ValueType::Color
        || !matches!(info.binding, Binding::Constant(_))
    {
        return Err("Select an editable color constant".into());
    }
    let record = palette_record(&mut next);
    record.palette_bindings.retain(|binding| binding.key != key);
    record.palette_bindings.push(PaletteBinding {
        key,
        index: u32::from(index),
    });
    resolve(&mut next, palette)?;
    next.validate().map_err(|e| e.to_string())?;
    Ok(next)
}

/// Give newly dropped Color generators the selected project color. Imported
/// standalone MaterialZ color constants are assigned a project palette entry.
pub(super) fn ensure_color_bindings(
    asset: &mut EditableMaterialAsset,
    palette: &ThePalette,
) -> Result<bool, String> {
    prune_bindings(asset);
    let bound: HashSet<_> = asset
        .instances
        .iter()
        .find(|record| record.id == RECORD)
        .map(|record| {
            record
                .palette_bindings
                .iter()
                .map(|binding| binding.key.clone())
                .collect()
        })
        .unwrap_or_default();
    let mut additions = vec![];
    for info in asset.parameter_metadata().map_err(|e| e.to_string())? {
        if info.definition.kind != denrim_materials::metadata::ValueType::Color {
            continue;
        }
        let Binding::Constant(color) = info.binding else {
            continue;
        };
        if bound.contains(&info.key) {
            continue;
        }
        let newly_dropped_color = info.key.parameter == "color"
            && asset.documents().any(|d| {
                d.id == info.key.asset
                    && d.project
                        .materials
                        .iter()
                        .flat_map(|m| &m.rows)
                        .flat_map(|r| &r.tiles)
                        .any(|t| {
                            t.id == info.key.tile
                                && t.kernel == "Color"
                                && !t.parameters.iter().any(|(name, _)| name == "color")
                        })
            });
        let index = if newly_dropped_color
            && palette
                .colors
                .get(palette.current_index as usize)
                .is_some_and(Option::is_some)
        {
            palette.current_index
        } else {
            nearest_index(&color, palette)?
        };
        additions.push(PaletteBinding {
            key: info.key,
            index: u32::from(index),
        });
    }
    let changed = !additions.is_empty();
    if !additions.is_empty() {
        palette_record(asset).palette_bindings.extend(additions);
    }
    Ok(resolve(asset, palette)? || changed)
}

/// Structural deletion/disconnection must not leave saved instance keys dangling.
/// A Color parameter explicitly connected to a row is no longer palette-driven.
pub(super) fn prune_bindings(asset: &mut EditableMaterialAsset) {
    let Ok(parameters) = asset.parameter_metadata() else {
        return;
    };
    let all: HashSet<_> = parameters
        .iter()
        .map(|parameter| parameter.key.clone())
        .collect();
    let colors: HashSet<_> = parameters
        .iter()
        .filter(|parameter| {
            parameter.definition.kind == denrim_materials::metadata::ValueType::Color
                && matches!(parameter.binding, Binding::Constant(_))
        })
        .map(|parameter| parameter.key.clone())
        .collect();
    for record in &mut asset.instances {
        record
            .palette_bindings
            .retain(|binding| colors.contains(&binding.key));
        record.overrides.retain(|value| all.contains(&value.key));
    }
}

/// Run after a palette color edit, before composition regeneration. Palette
/// selection alone is not a content edit and does not require rebaking.
pub(super) fn refresh(project: &mut Project) -> Vec<String> {
    let mut errors = vec![];
    for (id, bundle) in &mut project.procedural_materials {
        // A generated composition already contains resolved source values and
        // is rebuilt by composition::sync_compositions when source bundles change.
        if project.material_compositions.contains_key(id) {
            continue;
        }
        let result = EditableMaterialAsset::reopen(bundle)
            .map_err(|e| e.to_string())
            .and_then(|mut asset| {
                ensure_color_bindings(&mut asset, &project.art_palette)?;
                asset.validate().map_err(|e| e.to_string())?;
                asset.save().map_err(|e| e.to_string())
            });
        match result {
            Ok(updated) => *bundle = updated,
            Err(error) => errors.push(format!("Palette material {id}: {error}")),
        }
    }
    errors
}

fn palette_record(asset: &mut EditableMaterialAsset) -> &mut InstanceRecord {
    let index = if let Some(index) = asset
        .instances
        .iter()
        .position(|record| record.id == RECORD)
    {
        index
    } else {
        asset.instances.push(InstanceRecord::new(RECORD));
        asset.instances.len() - 1
    };
    &mut asset.instances[index]
}
fn nearest_index(color: &[f32], palette: &ThePalette) -> Result<u16, String> {
    palette
        .colors
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            entry.as_ref()?;
            let resolved = linear_color(palette, index as u16).ok()?;
            let distance: f32 = resolved
                .iter()
                .zip(color)
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            Some((index as u16, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
        .ok_or_else(|| "The project palette has no colors".into())
}

pub(super) fn bindings(asset: &EditableMaterialAsset) -> Vec<MaterialPaletteBindingView> {
    asset
        .instances
        .iter()
        .filter(|r| r.id == RECORD)
        .flat_map(|r| &r.palette_bindings)
        .filter(|b| b.key.asset == asset.root.id)
        .map(|b| MaterialPaletteBindingView {
            operation: if b.key.tile == format!("@material:{}", asset.selected_material) {
                "@surface".into()
            } else {
                b.key.tile.clone()
            },
            parameter: b.key.parameter.clone(),
            index: b.index as u16,
        })
        .collect()
}
fn linear_color(palette: &ThePalette, index: u16) -> Result<Vec<f32>, String> {
    let color = palette
        .colors
        .get(usize::from(index))
        .and_then(Option::as_ref)
        .ok_or_else(|| format!("Palette color {index} is empty"))?;
    let linear = |value: f32| {
        let value = value.clamp(0., 1.);
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    if [color.r, color.g, color.b, color.a]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(format!("Palette color {index} is not finite"));
    }
    Ok(vec![
        linear(color.r),
        linear(color.g),
        linear(color.b),
        color.a.clamp(0., 1.),
    ])
}
fn resolve(asset: &mut EditableMaterialAsset, palette: &ThePalette) -> Result<bool, String> {
    let bindings = asset
        .instances
        .iter()
        .find(|record| record.id == RECORD)
        .map(|record| record.palette_bindings.clone())
        .unwrap_or_default();
    let mut changed = false;
    for binding in bindings {
        let index = u16::try_from(binding.index).map_err(|_| "Palette index is out of range")?;
        let value = linear_color(palette, index)?;
        let document = if asset.root.id == binding.key.asset {
            &mut asset.root
        } else {
            asset
                .dependencies
                .iter_mut()
                .find(|document| document.id == binding.key.asset)
                .ok_or("Palette binding document is missing")?
        };
        if let Some(id) = binding.key.tile.strip_prefix("@material:") {
            let material = document
                .project
                .materials
                .iter_mut()
                .find(|m| m.id == id)
                .ok_or("Palette material is missing")?;
            let rgb = value[..3].to_vec();
            if let Some((_, old)) = material
                .defaults
                .iter_mut()
                .find(|(name, _)| name == &binding.key.parameter)
            {
                if *old != rgb {
                    *old = rgb;
                    changed = true;
                }
            } else {
                material.defaults.push((binding.key.parameter, rgb));
                changed = true;
            }
            continue;
        }
        let tile = document
            .project
            .materials
            .iter_mut()
            .flat_map(|material| &mut material.rows)
            .flat_map(|row| &mut row.tiles)
            .find(|tile| tile.id == binding.key.tile)
            .ok_or("Palette Color tile is missing")?;
        if let Some((_, parameter)) = tile
            .parameters
            .iter_mut()
            .find(|(id, _)| id == &binding.key.parameter)
        {
            let next = Binding::Constant(value);
            if *parameter != next {
                *parameter = next;
                changed = true;
            }
        } else {
            tile.parameters
                .push((binding.key.parameter, Binding::Constant(value)));
            changed = true;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn color_tile(asset: &EditableMaterialAsset, id: &str) -> format::Tile {
        asset
            .root
            .project
            .materials
            .iter()
            .flat_map(|m| &m.rows)
            .flat_map(|r| &r.tiles)
            .find(|t| t.id == id)
            .unwrap()
            .clone()
    }
    #[test]
    fn ramps_mixers_and_material_defaults_keep_palette_indices_through_edit_save_and_refresh() {
        let mut project = Project::default();
        let id = Uuid::new_v4();
        let source = r#"project 1
material "surface" called "Surface"
  mode pbr
  row "ramp"
    use "Perlin Noise" id "noise"
    use "Color Ramp" id "ramp"
  row "mix"
    use "Mix" id "mix"
      input from row "ramp" xyz
  output base color from row "mix" xyz
"#;
        let mut asset = EditableMaterialAsset::parse(&id.to_string(), source, "surface").unwrap();
        ensure_color_bindings(&mut asset, &project.art_palette).unwrap();
        let colors = asset
            .parameter_metadata()
            .unwrap()
            .into_iter()
            .filter(|p| {
                p.definition.kind == denrim_materials::metadata::ValueType::Color
                    && matches!(p.binding, Binding::Constant(_))
            })
            .count();
        assert_eq!(bindings(&asset).len(), colors);
        for (op, param, index) in [
            ("ramp", "color2", 41),
            ("mix", "color1", 37),
            ("@surface", "emission", 26),
            ("@surface", "base color", 19),
        ] {
            asset = edited_for_project(
                &asset,
                MaterialEditorCommand::SetPaletteParameter {
                    operation: op.into(),
                    parameter: param.into(),
                    index,
                },
                &project,
            )
            .unwrap();
        }
        let saved = asset.save().unwrap();
        let reopened = EditableMaterialAsset::reopen(&saved).unwrap();
        assert!(
            bindings(&reopened)
                .iter()
                .any(|b| b.operation == "ramp" && b.parameter == "color2" && b.index == 41)
        );
        project.procedural_materials.insert(id, saved.clone());
        project.art_palette.colors[41] = Some(TheColor::new(1., 0., 0., 1.));
        assert!(refresh(&mut project).is_empty());
        let updated = EditableMaterialAsset::reopen(&project.procedural_materials[&id]).unwrap();
        assert_eq!(
            color_tile(&updated, "ramp")
                .parameters
                .iter()
                .find(|(id, _)| id == "color2")
                .unwrap()
                .1,
            Binding::Constant(vec![1., 0., 0., 1.])
        );
        assert_eq!(bindings(&updated).len(), colors);
        // Undo retains the exact indexed recipe; defaults also survive reopening.
        let undo = EditableMaterialAsset::reopen(&saved).unwrap();
        assert!(
            bindings(&undo)
                .iter()
                .any(|b| b.operation == "@surface" && b.parameter == "emission" && b.index == 26)
        );
        assert!(
            view(&updated)
                .unwrap()
                .palette_bindings
                .iter()
                .any(|b| b.operation == "mix" && b.parameter == "color1" && b.index == 37)
        );
    }

    #[test]
    fn starter_mixes_two_palette_color_rows_and_retains_opacity() {
        let project = Project::default();
        let asset = starter(Uuid::new_v4(), &project).unwrap();
        let mat = material(&asset).unwrap();
        assert_eq!(
            mat.rows
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["palette-dark", "palette-light", "grain", "color"]
        );
        assert_eq!(
            mat.rows
                .iter()
                .flat_map(|row| &row.tiles)
                .filter(|tile| tile.kernel == "Perlin Noise")
                .count(),
            1
        );
        assert_eq!(
            mat.outputs
                .iter()
                .find(|output| output.channel == "roughness")
                .unwrap()
                .source
                .row,
            "grain"
        );
        let pigment = color_tile(&asset, "pigment");
        assert_eq!(pigment.input.as_ref().unwrap().row, "grain");
        for key in ["color1", "color2"] {
            assert!(matches!(
                pigment
                    .parameters
                    .iter()
                    .find(|(id, _)| id == key)
                    .unwrap()
                    .1,
                Binding::From(_)
            ));
        }
        assert_eq!(binding_index(&asset, "palette-a"), Some(37));
        assert_eq!(binding_index(&asset, "palette-b"), Some(34));
        let reopened = EditableMaterialAsset::reopen(&asset.save().unwrap()).unwrap();
        assert_eq!(binding_index(&reopened, "palette-a"), Some(37));
        assert!(
            reopened.root.project.materials[0]
                .outputs
                .iter()
                .any(|output| output.channel == "opacity" && output.source.components == "w")
        );
    }
    #[test]
    fn palette_updates_linearize_rgb_preserve_alpha_and_keep_indices() {
        let mut project = Project::default();
        project.art_palette.colors[37] = Some(TheColor::new(0.5, 0.25, 1., 0.4));
        let id = Uuid::new_v4();
        let asset = starter(id, &project).unwrap();
        let color = color_tile(&asset, "palette-a")
            .parameters
            .into_iter()
            .find(|(id, _)| id == "color")
            .unwrap()
            .1;
        let Binding::Constant(values) = color else {
            panic!()
        };
        assert!((values[0] - 0.21404114).abs() < 1e-6);
        assert!((values[3] - 0.4).abs() < 1e-6);
        project
            .procedural_materials
            .insert(id, asset.save().unwrap());
        project.art_palette.colors[37] = Some(TheColor::new(1., 0., 0., 0.2));
        assert!(refresh(&mut project).is_empty());
        let changed = EditableMaterialAsset::reopen(&project.procedural_materials[&id]).unwrap();
        assert_eq!(binding_index(&changed, "palette-a"), Some(37));
        assert_eq!(
            color_tile(&changed, "palette-a").parameters[0].1,
            Binding::Constant(vec![1., 0., 0., 0.2])
        );
        let before = project.procedural_materials[&id].clone();
        assert!(refresh(&mut project).is_empty());
        assert_eq!(project.procedural_materials[&id], before);
    }
    #[test]
    fn new_colors_bind_selected_slot_and_invalid_selection_is_transactional() {
        let mut project = Project::default();
        project.art_palette.current_index = 8;
        let mut asset=EditableMaterialAsset::parse("color","project 1\nmaterial \"surface\" called \"Color\"\n  mode pbr\n  row \"color\"\n    use \"Color\" id \"paint\"\n  output base color from row \"color\" xyz\n","surface").unwrap();
        assert!(ensure_color_bindings(&mut asset, &project.art_palette).unwrap());
        assert_eq!(binding_index(&asset, "paint"), Some(8));
        let saved = asset.save().unwrap();
        assert!(bind_color(&asset, "paint", 255, &project.art_palette).is_err());
        assert_eq!(asset.save().unwrap(), saved);
        let changed = bind_color(&asset, "paint", 0, &project.art_palette).unwrap();
        assert_eq!(binding_index(&changed, "paint"), Some(0));
        let fingerprint = key(&project.art_palette);
        project.art_palette.current_index = 4;
        assert_eq!(key(&project.art_palette), fingerprint);
    }
    #[test]
    fn deleted_or_connected_colors_prune_palette_keys_without_rebinding_masks() {
        let project = Project::default();
        let mut asset = starter(Uuid::new_v4(), &project).unwrap();
        asset.root.project.materials[0]
            .rows
            .iter_mut()
            .find(|row| row.id == "palette-dark")
            .unwrap()
            .tiles[0]
            .parameters[0]
            .1 = Binding::From(Reference {
            row: "grain".into(),
            components: "xxxw".into(),
            mapping: None,
        });
        prune_bindings(&mut asset);
        ensure_color_bindings(&mut asset, &project.art_palette).unwrap();
        assert_eq!(binding_index(&asset, "palette-a"), None);
        asset.root.project.materials[0]
            .rows
            .retain(|row| row.id != "palette-light");
        prune_bindings(&mut asset);
        assert_eq!(binding_index(&asset, "palette-b"), None);
    }
    #[test]
    fn palette_identity_survives_duplication_and_embedded_dependencies() {
        let project = Project::default();
        let id = Uuid::new_v4();
        let mut asset = starter(id, &project).unwrap();
        let dependency=EditableMaterialAsset::parse("shared-colors","project 1\nmaterial \"dependency\" called \"Shared Color\"\n  mode pbr\n  row \"color\"\n    use \"Color\" id \"shared-color\"\n  output base color from row \"color\" xyz\n","dependency").unwrap();
        asset.dependencies.push(dependency.root);
        ensure_color_bindings(&mut asset, &project.art_palette).unwrap();
        asset.validate().unwrap();
        let duplicate =
            super::super::duplicate_bundle(&asset.save().unwrap(), Uuid::new_v4()).unwrap();
        let duplicate = EditableMaterialAsset::reopen(&duplicate).unwrap();
        assert_eq!(binding_index(&duplicate, "palette-a"), Some(37));
        assert!(
            duplicate
                .instances
                .iter()
                .flat_map(|record| &record.palette_bindings)
                .any(|binding| binding.key.asset == "shared-colors"
                    && binding.key.tile == "shared-color")
        );
        let changed = bind_color(&duplicate, "palette-a", 8, &project.art_palette).unwrap();
        assert_eq!(binding_index(&changed, "palette-a"), Some(8));
        assert_eq!(binding_index(&asset, "palette-a"), Some(37));
    }
    #[test]
    fn shared_palette_edits_regenerate_compositions_from_canonical_source_bindings() {
        use shared::material_composition::{CompositionLayer, MaterialComposition};
        let mut project = Project::default();
        let mut ids = vec![];
        for _ in 0..2 {
            let tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(4, 4)]);
            let id = tile.id;
            project
                .procedural_materials
                .insert(id, starter(id, &project).unwrap().save().unwrap());
            project.tiles.insert(id, tile);
            ids.push(id);
        }
        let id = Uuid::new_v4();
        let model = MaterialComposition {
            tiling: Default::default(),
            base: ids[0],
            layers: vec![CompositionLayer::split(ids[1])],
        };
        project.material_compositions.insert(id, model);
        let before = super::super::composition::bundle(id, &project, None).unwrap();
        project.procedural_materials.insert(id, before.clone());
        project.art_palette.colors[37] = Some(TheColor::new(1., 0., 1., 0.7));
        assert!(refresh(&mut project).is_empty());
        let after = super::super::composition::bundle(id, &project, None).unwrap();
        assert_ne!(before, after);
        assert_eq!(
            project.procedural_materials[&id], before,
            "Derived bundles are replaced by the host composition synchronizer"
        );
        let source = EditableMaterialAsset::reopen(&project.procedural_materials[&ids[0]]).unwrap();
        assert_eq!(binding_index(&source, "palette-a"), Some(37));
    }
}
