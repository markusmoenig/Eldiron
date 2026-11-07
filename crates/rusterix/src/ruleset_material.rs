//! Ruleset-owned MZ surfaces. Palette indices are authoring data; texture maps are derived.
use crate::Texture;
pub use denrim_materials::integration::EditableMaterialAsset as EditableAsset;
use denrim_materials::{
    format::Binding,
    integration::{EditableMaterialAsset, InstanceRecord, PaletteBinding, ParameterKey},
    ir::Channel,
};
use rayon::prelude::*;
use std::collections::BTreeMap;
use theframework::prelude::*;

pub fn editable(
    def: &eldiron_ruleset::BundledMaterialAsset,
    palette: &ThePalette,
) -> Result<EditableMaterialAsset, String> {
    let id = eldiron_ruleset::material_asset_id(def.id).to_string();
    let mut asset =
        EditableMaterialAsset::parse(&id, def.source, def.output).map_err(|e| e.to_string())?;
    let bindings: BTreeMap<String, BTreeMap<String, u16>> =
        serde_json::from_str(def.palette).map_err(|e| e.to_string())?;
    let mut record = InstanceRecord::new("eldiron-palette");
    for (operation, params) in bindings {
        for (parameter, index) in params {
            let material_key = format!("@material:{}", def.output);
            let operation = if operation == "@surface" {
                &material_key
            } else {
                &operation
            };
            let index = available_index(palette, index)?;
            record.palette_bindings.push(PaletteBinding {
                key: ParameterKey::new(&id, operation, &parameter),
                index: u32::from(index),
            });
        }
    }
    asset.instances.push(record);
    resolve_palette(&mut asset, palette)?;
    asset.validate().map_err(|e| e.to_string())?;
    Ok(asset)
}
fn available_index(palette: &ThePalette, preferred: u16) -> Result<u16, String> {
    if palette
        .colors
        .get(preferred as usize)
        .is_some_and(Option::is_some)
    {
        return Ok(preferred);
    }
    // Fresh small palettes map the canonical palette slot to an existing project slot.
    static CANONICAL: std::sync::OnceLock<ThePalette> = std::sync::OnceLock::new();
    let canonical = CANONICAL.get_or_init(|| eldiron_ruleset::palette_template_from_source(eldiron_ruleset::latest_official_ruleset()).unwrap());
    let target = canonical.colors[preferred as usize]
        .as_ref()
        .ok_or("Unknown palette slot")?
        .to_u8_array();
    palette
        .colors
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|c| (i, c.to_u8_array())))
        .min_by_key(|(_, c)| {
            (0..3)
                .map(|k| (c[k] as i32 - target[k] as i32).pow(2))
                .sum::<i32>()
        })
        .map(|(i, _)| i as u16)
        .ok_or("Project palette is empty".into())
}
/// Evaluate all PBR channels with the MZ reference backend once per source/palette change.
/// The exact same immutable maps feed directional sprite projection and native 3D meshes.
pub fn bake(
    asset: &EditableMaterialAsset,
    palette: &ThePalette,
    size: u32,
) -> Result<Texture, String> {
    bake_at_time(asset, palette, size, 0.)
}
pub fn bake_at_time(
    asset: &EditableMaterialAsset,
    palette: &ThePalette,
    size: u32,
    time: f32,
) -> Result<Texture, String> {
    if !(1..=512).contains(&size) {
        return Err("Ruleset surface resolution must be 1..512".into());
    }
    let mut asset = asset.clone();
    resolve_palette(&mut asset, palette)?;
    let program = asset
        .program(&InstanceRecord::new("ruleset-bake"))
        .map_err(|e| e.to_string())?;
    let evaluator = denrim_materials::cpu::Evaluator::new(&program);
    let channels = [
        Channel::BaseColor,
        Channel::Roughness,
        Channel::Metallic,
        Channel::Opacity,
        Channel::Emission,
        Channel::EmissionStrength,
    ];
    let maps = channels
        .par_iter()
        .map(|channel| {
            let rows = (0..size)
                .into_par_iter()
                .map(|y| {
                    (0..size)
                        .map(|x| {
                            evaluator.sample_with_footprint_at_time(
                                &asset.selected_material,
                                *channel,
                                [
                                    (x as f32 + 0.5) / size as f32 - 0.5,
                                    (y as f32 + 0.5) / size as f32 - 0.5,
                                ],
                                1. / size as f32,
                                time,
                            )
                        })
                        .collect::<denrim_materials::Result<Vec<_>>>()
                })
                .collect::<denrim_materials::Result<Vec<_>>>()?;
            Ok(rows.into_iter().flatten().collect::<Vec<_>>())
        })
        .collect::<denrim_materials::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    let colors = palette
        .colors
        .iter()
        .flatten()
        .map(TheColor::to_u8_array)
        .collect::<Vec<_>>();
    if colors.is_empty() {
        return Err("Project palette has no colors".into());
    }
    let srgb = |v: f32| {
        let v = v.clamp(0., 1.);
        ((if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1. / 2.4) - 0.055
        }) * 255.)
            .round() as u8
    };
    let mut texture = Texture::new(
        vec![0; (size * size * 4) as usize],
        size as usize,
        size as usize,
    );
    texture.data_ext = Some([0, 15, 128, 128].repeat((size * size) as usize));
    for i in 0..(size * size) as usize {
        let rgb: [u8; 3] = std::array::from_fn(|k| srgb(maps[0][i][k]));
        let color = colors
            .iter()
            .min_by_key(|c| {
                (0..3)
                    .map(|k| (c[k] as i32 - rgb[k] as i32).pow(2))
                    .sum::<i32>()
            })
            .unwrap();
        texture.data[i * 4..i * 4 + 3].copy_from_slice(&color[..3]);
        texture.data[i * 4 + 3] = (maps[3][i][0].clamp(0., 1.) * 255.).round() as u8;
        let emission = maps[4][i].iter().copied().fold(0., f32::max) * maps[5][i][0];
        texture.set_materials(
            i as u32 % size,
            i as u32 / size,
            maps[1][i][0],
            maps[2][i][0],
            1., // Alpha already carries opacity; SceneVM multiplies these fields.
            emission,
        );
        if let Some(ext) = &mut texture.data_ext {
            if ext[i * 4] == 0xfe {
                ext[i * 4] = 0xff;
            }
        }
    }
    Ok(texture)
}

pub fn linear_color(palette: &ThePalette, index: u16) -> Result<Vec<f32>, String> {
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
pub fn resolve_palette(
    asset: &mut EditableMaterialAsset,
    palette: &ThePalette,
) -> Result<bool, String> {
    let bindings = asset
        .instances
        .iter()
        .find(|record| record.id == "eldiron-palette")
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
    use denrim_materials::{format::Binding, metadata::ValueType};
    #[test]
    fn torch_flicker_keeps_the_diagonal_handle_still_and_uses_four_frames() {
        let palette = eldiron_ruleset::palette_template_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let def = eldiron_ruleset::bundled_icon_material_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        )
        .into_iter()
        .find(|d| d.id == "icon/torch/on")
        .unwrap();
        let asset = editable(&def, &palette).unwrap();
        assert_eq!(asset.root.project.materials[0].animation.frames, 4);
        let frames = (0..4)
            .map(|n| bake_at_time(&asset, &palette, 32, n as f32 / 4.).unwrap())
            .collect::<Vec<_>>();
        for frame in &frames {
            assert_eq!(
                &frames[0].data[14 * 32 * 4..],
                &frame.data[14 * 32 * 4..],
                "the handle must not wobble with the flame"
            );
        }
        assert!((1..4).all(|i| frames[i].data != frames[0].data));
        assert_eq!(
            frames[0].data,
            bake_at_time(&asset, &palette, 32, 1.).unwrap().data
        );
        // Thin diagonal silhouette: the original torch's visual weight, not a large upright head.
        let visible = frames[0].data.chunks_exact(4).filter(|p| p[3] > 0).count();
        assert!((35..100).contains(&visible), "{visible} visible pixels");
    }

    #[test]
    fn native_ruleset_icons_are_palette_bound_32px_and_transparent() {
        let palette = eldiron_ruleset::palette_template_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        for def in eldiron_ruleset::bundled_icon_material_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        ) {
            let started = std::time::Instant::now();
            let asset = editable(&def, &palette).unwrap();
            assert_eq!(
                EditableAsset::reopen(&asset.save().unwrap())
                    .unwrap()
                    .save()
                    .unwrap(),
                asset.save().unwrap()
            );
            assert!(
                asset
                    .root
                    .project
                    .materials
                    .iter()
                    .flat_map(|m| &m.rows)
                    .flat_map(|r| &r.tiles)
                    .all(|t| t.image.is_none() && t.code.is_none()),
                "{} must be editable stock MZ rows",
                def.id
            );
            let texture = bake(&asset, &palette, 32).unwrap();
            eprintln!("{} bake {:?}", def.id, started.elapsed());
            assert_eq!((texture.width, texture.height), (32, 32));
            assert_eq!(texture.data[3], 0);
            let visible = texture.data.chunks_exact(4).filter(|p| p[3] > 128).count();
            assert!(
                (25..850).contains(&visible),
                "{}: {visible} visible pixels",
                def.id
            );
            assert!(
                texture
                    .data
                    .chunks_exact(4)
                    .all(|p| p[3] == 0 || p[3] == 255)
            );
            assert!(texture.data.chunks_exact(4).filter(|p| p[3] > 0).all(|p| {
                palette
                    .colors
                    .iter()
                    .flatten()
                    .any(|c| c.to_u8_array()[..3] == p[..3])
            }));
            for parameter in asset.parameter_metadata().unwrap() {
                if parameter.definition.kind == ValueType::Color
                    && matches!(parameter.binding, Binding::Constant(_))
                {
                    assert!(
                        asset
                            .instances
                            .iter()
                            .flat_map(|i| &i.palette_bindings)
                            .any(|b| b.key == parameter.key),
                        "Unbound icon color: {:?}",
                        parameter.key
                    );
                }
            }
        }
    }
    #[test]
    fn every_official_item_has_its_own_native_icon_binding() {
        let defs = eldiron_ruleset::bundled_icon_material_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        );
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        assert_eq!(
            defs.len(),
            templates.len() + 1,
            "one icon per item plus the unlit torch"
        );
        for item in templates {
            let table = item.data.parse::<toml::Table>().unwrap();
            let name = format!("icon/{}/on", item.id);
            assert_eq!(
                table["attributes"]["appearance_icon"].as_str(),
                Some(name.as_str())
            );
            assert!(defs.iter().any(|d| d.id == name));
            if item.id == "torch" {
                assert_eq!(
                    table["attributes"]["appearance_icon_off"].as_str(),
                    Some("icon/torch/off")
                );
            }
        }
    }
    #[test]
    fn official_icon_colors_resolve_to_valid_indices_in_small_project_palettes() {
        let canonical = eldiron_ruleset::palette_template_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let defs = eldiron_ruleset::bundled_icon_material_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        );
        for size in [16, 32] {
            let mut palette = canonical.clone();
            palette.colors.truncate(size);
            for def in &defs {
                let asset = editable(def, &palette).unwrap();
                assert!(
                    asset
                        .instances
                        .iter()
                        .flat_map(|r| &r.palette_bindings)
                        .all(|b| b.index < size as u32)
                );
                if matches!(
                    def.id,
                    "icon/hunting_bow/on" | "icon/moonwater/on" | "icon/torch/on"
                ) {
                    let texture = bake(&asset, &palette, 32).unwrap();
                    assert!(texture.data.chunks_exact(4).filter(|p| p[3] > 0).all(|p| {
                        palette
                            .colors
                            .iter()
                            .flatten()
                            .any(|c| c.to_u8_array()[..3] == p[..3])
                    }));
                }
            }
        }
    }
    #[test]
    fn new_weapon_surfaces_use_the_same_palette_slots_as_their_icons() {
        let surfaces = eldiron_ruleset::bundled_material_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        );
        let icons = eldiron_ruleset::bundled_icon_material_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        );
        for (surface, weapon, surface_tile, icon_tile) in [
            (
                "weapon-iron",
                "novice_mace",
                "surface-colors",
                "flanges-colors",
            ),
            ("weapon-ash", "novice_mace", "surface-colors", "haft-colors"),
            (
                "weapon-wrap",
                "novice_mace",
                "surface-colors",
                "grip-colors",
            ),
            ("bow-grip", "hunting_bow", "surface-colors", "grip-colors"),
            ("bow-string", "hunting_bow", "color", "string-color"),
            ("arrow-shaft", "hunting_bow", "color", "arrow-color"),
            ("arrow-feather", "hunting_bow", "color", "fletching-color"),
            (
                "spear-binding",
                "training_spear",
                "surface-colors",
                "socket-colors",
            ),
        ] {
            let a: serde_json::Value =
                serde_json::from_str(surfaces.iter().find(|d| d.id == surface).unwrap().palette)
                    .unwrap();
            let b: serde_json::Value = serde_json::from_str(
                icons
                    .iter()
                    .find(|d| d.id == format!("icon/{weapon}/on"))
                    .unwrap()
                    .palette,
            )
            .unwrap();
            assert_eq!(
                a[surface_tile], b[icon_tile],
                "{surface} must match {weapon}'s icon palette"
            );
        }
    }
    #[test]
    fn ruleset_surfaces_are_palette_bound_with_authored_garment_details() {
        let palette = eldiron_ruleset::palette_template_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let defs = eldiron_ruleset::bundled_material_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        );
        for def in &defs {
            let asset = editable(def, &palette).unwrap_or_else(|e| panic!("{}: {e}", def.id));
            let bindings = &asset.instances[0].palette_bindings;
            for parameter in asset.parameter_metadata().unwrap() {
                if parameter.definition.kind == ValueType::Color
                    && matches!(parameter.binding, Binding::Constant(_))
                {
                    assert!(
                        bindings.iter().any(|b| b.key == parameter.key),
                        "{} has an unbound authored color: {:?}",
                        def.id,
                        parameter.key
                    );
                }
            }
            let texture = bake(
                &asset,
                &palette,
                if def.id.starts_with("chainmail") {
                    64
                } else {
                    32
                },
            )
            .unwrap();
            let colors = texture
                .data
                .chunks_exact(4)
                .map(|p| p[..3].to_vec())
                .collect::<std::collections::HashSet<_>>();
            if matches!(def.id, "skin" | "eye-white" | "lips") {
                assert_eq!(colors.len(), 1, "{} should stay clean", def.id);
            }
            if matches!(
                def.id,
                "tunic" | "tunic-sleeve" | "trouser-waist" | "leather" | "hair"
            ) {
                assert!(colors.len() >= 2, "{} lost its constructed details", def.id);
            }
            if !matches!(def.id, "steel" | "wood") {
                assert!(
                    asset
                        .root
                        .project
                        .materials
                        .iter()
                        .flat_map(|m| &m.rows)
                        .flat_map(|r| &r.tiles)
                        .all(|t| t.code.is_none()
                            && !t.kernel.contains("Noise")
                            && t.kernel != "Weave"),
                    "Clothing and anatomy use editable shapes, not noise or bespoke code"
                );
            }
            if matches!(def.id, "tunic" | "tunic-sleeve" | "trousers") {
                let base = palette.colors[if def.id == "trousers" { 12 } else { 41 }]
                    .as_ref()
                    .unwrap()
                    .to_u8_array();
                let plain = texture
                    .data
                    .chunks_exact(4)
                    .filter(|p| p[..3] == base[..3])
                    .count();
                assert!(
                    plain * 100 > 65 * 32 * 32,
                    "{} must remain predominantly plain cloth",
                    def.id
                );
            }
            for p in texture.data.chunks_exact(4) {
                assert!(
                    palette
                        .colors
                        .iter()
                        .flatten()
                        .any(|c| c.to_u8_array()[..3] == p[..3])
                );
            }
            let values = (0..texture.height as u32)
                .flat_map(|y| (0..texture.width as u32).map(move |x| (x, y)))
                .map(|(x, y)| texture.get_materials(x, y))
                .collect::<Vec<_>>();
            assert!(values.iter().all(|v| v.0 >= 0.2 && v.0 <= 1.));
            if matches!(def.id, "steel" | "weapon-iron") {
                assert!(values.iter().all(|v| v.1 > 0.8));
            } else if matches!(def.id, "item-gold" | "item-copper") {
                assert!(values.iter().all(|v| v.1 >= 0.7));
            } else if def.id.starts_with("chainmail") {
                assert!(
                    values.iter().any(|v| v.1 > 0.5),
                    "Chain rings must be metallic"
                );
                assert!(
                    values.iter().any(|v| v.1 < 0.1),
                    "{}: padding between chain links must stay nonmetallic (min {})",
                    def.id,
                    values.iter().map(|v| v.1).fold(f32::INFINITY, f32::min)
                );
            } else {
                assert!(values.iter().all(|v| v.1 == 0.));
            }
            if def.id.starts_with("item-flame") {
                assert!(values.iter().all(|v| v.3 > 0.));
            } else {
                assert!(values.iter().all(|v| v.3 == 0.));
            }
        }
        let mut small = palette.clone();
        small.colors.truncate(16);
        let custom = editable(&defs[0], &small).unwrap();
        assert!(
            custom.instances[0]
                .palette_bindings
                .iter()
                .all(|b| b.index < 16)
        );
        bake(&custom, &small, 16).unwrap();
    }

    #[test]
    fn garment_details_are_placed_in_their_intended_uv_regions() {
        let palette = eldiron_ruleset::palette_template_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let defs = eldiron_ruleset::bundled_material_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        );
        for (name, samples) in [
            (
                "tunic",
                vec![(0.35, 0.6, 41), (0.25, 0.95, 23), (0.25, 0.09, 23)],
            ),
            ("tunic-sleeve", vec![(0.25, 0.09, 41), (0.25, 0.91, 23)]),
            ("trouser-waist", vec![(0.8, 0.27, 18), (0.8, 0.65, 12)]),
        ] {
            let def = defs.iter().find(|d| d.id == name).unwrap();
            let texture = bake(&editable(def, &palette).unwrap(), &palette, 64).unwrap();
            for (u, v, index) in samples {
                let offset = (((v * 64.) as usize) * 64 + (u * 64.) as usize) * 4;
                let expected = palette.colors[index].as_ref().unwrap().to_u8_array();
                assert_eq!(
                    texture.data[offset..offset + 3],
                    expected[..3],
                    "{name} at ({u},{v})"
                );
            }
        }
    }
}
