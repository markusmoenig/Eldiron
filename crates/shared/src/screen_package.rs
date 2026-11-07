//! Portable screen artwork with editable recipes and project-palette remapping.
use crate::{
    material_composition::{MapMaterialBinding, MaterialComposition},
    project::Project,
    screen::Screen,
};
use denrim_materials::integration::EditableMaterialAsset;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use theframework::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScreenPackage {
    pub format: String,
    pub version: u32,
    pub screen: Screen,
    pub tiles: IndexMap<Uuid, rusterix::Tile>,
    pub materials: IndexMap<Uuid, String>,
    pub compositions: IndexMap<Uuid, MaterialComposition>,
    pub bindings: IndexMap<Uuid, MapMaterialBinding>,
    pub palette: ThePalette,
}

#[cfg(test)]
mod tests {
    use super::*;
    use denrim_materials::{
        image_source::ImageSource,
        integration::{InstanceRecord, PaletteBinding, ParameterKey},
    };
    use rusterix::{PixelSource, Texture, Value};

    #[test]
    fn composed_screen_tiles_keep_editable_layers_and_all_source_tiles() {
        let mut source = Project::default();
        let tiles = [[20, 40, 60, 255], [60, 80, 100, 255], [40, 60, 80, 255]]
            .map(|color| rusterix::Tile::from_texture(Texture::from_color(color)));
        for tile in &tiles {
            source.tiles.insert(tile.id, tile.clone());
        }
        source.material_compositions.insert(
            tiles[2].id,
            MaterialComposition {
                base: tiles[0].id,
                layers: vec![crate::material_composition::CompositionLayer::split(
                    tiles[1].id,
                )],
                tiling: Default::default(),
            },
        );
        let mut screen = Screen::new();
        screen
            .map
            .properties
            .set("source", Value::Source(PixelSource::TileId(tiles[2].id)));
        let package = ScreenPackage::capture(&source, &screen).unwrap();
        assert_eq!(package.tiles.len(), 3);
        let (project, id) = package.imported_project(&Project::default()).unwrap();
        let PixelSource::TileId(root) = project.screens[&id]
            .map
            .properties
            .get_default_source()
            .unwrap()
        else {
            panic!("missing imported composition")
        };
        let composition = &project.material_compositions[root];
        assert_ne!(composition.base, tiles[0].id);
        assert_ne!(composition.layers[0].material, tiles[1].id);
        assert!(project.tiles.contains_key(&composition.base));
        assert!(project.tiles.contains_key(&composition.layers[0].material));
        assert_eq!(
            composition.layers[0].shape,
            source.material_compositions[&tiles[2].id].layers[0].shape
        );
    }

    #[test]
    fn screen_roundtrip_remaps_assets_palette_and_linked_images_without_touching_existing_assets() {
        let mut source = Project::default();
        source.palette.colors = vec![
            Some(TheColor::from_hex("#e03020")),
            Some(TheColor::from_hex("#2050e0")),
        ];
        let a = rusterix::Tile::from_texture(Texture::from_color([224, 48, 32, 128]));
        let image = rusterix::Tile::from_texture(Texture::from_color([32, 80, 224, 255]));
        let unrelated = rusterix::Tile::from_texture(Texture::from_color([0, 0, 0, 255]));
        for tile in [&a, &image, &unrelated] {
            source.tiles.insert(tile.id, tile.clone());
        }
        let mut asset = EditableMaterialAsset::parse(
            &a.id.to_string(),
            r#"project 1
material "m" called "Panel"
  mode pbr
  row "color"
    use "Color" id "color"
  row "image"
    use "Image / Tile Source" id "image"
  output base color from row "color" xyz
"#,
            "m",
        )
        .unwrap();
        let mut record = InstanceRecord::new("eldiron-palette");
        record.palette_bindings.push(PaletteBinding {
            key: ParameterKey::new(&a.id.to_string(), "color", "color"),
            index: 0,
        });
        asset.instances.push(record);
        let mut pixels = ImageSource::rgba8(1, 1, &[vec![32, 80, 224, 255]]).unwrap();
        pixels.reference = Some(format!("eldiron:tile:{}", image.id));
        asset.root.project.materials[0].rows[1].tiles[0].image = Some(pixels);
        source
            .procedural_materials
            .insert(a.id, asset.save().unwrap());
        let mut screen = Screen::new();
        screen
            .map
            .properties
            .set("source", Value::Source(PixelSource::TileId(a.id)));
        screen
            .map
            .properties
            .set("data", Value::Str(format!("[ui]\ntile_id = \"{}\"", a.id)));
        source.map_material_bindings.insert(
            a.id,
            MapMaterialBinding {
                map: screen.map.id,
                surface: crate::material_composition::CompositionSurface::TerrainTile {
                    cell: [0, 0],
                },
            },
        );
        let package = ScreenPackage::capture(&source, &screen).unwrap();
        assert_eq!(package.tiles.len(), 2);
        assert!(!package.tiles.contains_key(&unrelated.id));
        let package: ScreenPackage =
            serde_json::from_str(&serde_json::to_string(&package).unwrap()).unwrap();
        let mut target = Project::default();
        target.palette.colors = vec![
            Some(TheColor::from_hex("#3050d0")),
            Some(TheColor::from_hex("#d03020")),
        ];
        // Deliberately collide with the imported tile: its pixels must survive.
        target.tiles.insert(a.id, unrelated.clone());
        let existing = serde_json::to_string(&target.tiles).unwrap();
        let (next, id) = package.imported_project(&target).unwrap();
        assert_eq!(serde_json::to_string(&target.tiles).unwrap(), existing);
        assert_eq!(next.palette.colors, target.palette.colors);
        assert_eq!(next.tiles[&a.id], unrelated);
        assert_ne!(id, screen.id);
        let imported = &next.screens[&id];
        let PixelSource::TileId(new_id) = imported.map.properties.get_default_source().unwrap()
        else {
            panic!("missing imported source")
        };
        assert_ne!(*new_id, a.id);
        assert_eq!(next.map_material_bindings[new_id].map, imported.map.id);
        assert!(
            imported
                .map
                .properties
                .get_str("data")
                .unwrap()
                .contains(&new_id.to_string())
        );
        assert_eq!(next.tiles[new_id].textures[0].data, vec![208, 48, 32, 128]);
        let asset = EditableMaterialAsset::reopen(&next.procedural_materials[new_id]).unwrap();
        assert_eq!(asset.instances[0].palette_bindings[0].index, 1);
        let program = asset
            .program(&InstanceRecord::new("verify-import"))
            .unwrap();
        let rgb = denrim_materials::cpu::Evaluator::new(&program)
            .sample("m", denrim_materials::ir::Channel::BaseColor, [0.5; 2])
            .unwrap();
        let expected = rusterix::ruleset_material::linear_color(&target.palette, 1).unwrap();
        assert!(
            rgb[..3]
                .iter()
                .zip(&expected)
                .all(|(a, b)| (a - b).abs() < 0.000001)
        );
        assert_eq!(
            asset.instances[0].palette_bindings[0].key.asset,
            new_id.to_string()
        );
        let linked = asset.root.project.materials[0].rows[1].tiles[0]
            .image
            .as_ref()
            .unwrap()
            .reference
            .as_ref()
            .unwrap();
        let linked = Uuid::parse_str(linked.strip_prefix("eldiron:tile:").unwrap()).unwrap();
        assert_ne!(linked, image.id);
        assert!(next.tiles.contains_key(&linked));
        let (again, another) = package.imported_project(&next).unwrap();
        assert_ne!(id, another);
        assert!(again.screens.contains_key(&id));
        assert_ne!(again.screens[&id].name, again.screens[&another].name);
        assert_eq!(again.tiles.len(), next.tiles.len() + 2);
    }
}

fn references(value: &impl Serialize) -> Result<BTreeSet<Uuid>, String> {
    static UUIDS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let pattern = UUIDS.get_or_init(|| {
        regex::Regex::new(r"(?i)[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
            .unwrap()
    });
    let text = serde_json::to_string(value).map_err(|e| e.to_string())?;
    Ok(pattern
        .find_iter(&text)
        .filter_map(|m| Uuid::parse_str(m.as_str()).ok())
        .collect())
}

fn rewrite(
    value: &mut serde_json::Value,
    ids: &HashMap<Uuid, Uuid>,
    palette: &HashMap<usize, usize>,
) {
    match value {
        serde_json::Value::String(text) => {
            // Rewrite once per string; newly allocated UUIDs cannot be remapped twice.
            static UUIDS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            let pattern = UUIDS.get_or_init(|| {
                regex::Regex::new(
                    r"(?i)[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
                )
                .unwrap()
            });
            *text = pattern
                .replace_all(text, |m: &regex::Captures| {
                    Uuid::parse_str(&m[0])
                        .ok()
                        .and_then(|id| ids.get(&id))
                        .map_or_else(|| m[0].to_string(), ToString::to_string)
                })
                .into_owned();
        }
        serde_json::Value::Array(values) => {
            for value in values {
                rewrite(value, ids, palette);
            }
        }
        serde_json::Value::Object(values) => {
            if let Some(index) = values.get_mut("PaletteIndex")
                && let Some(next) = index.as_u64().and_then(|i| palette.get(&(i as usize)))
            {
                *index = serde_json::json!(next);
            }
            for value in values.values_mut() {
                rewrite(value, ids, palette);
            }
        }
        _ => {}
    }
}

impl ScreenPackage {
    pub fn capture(project: &Project, screen: &Screen) -> Result<Self, String> {
        let mut package = Self {
            format: "eldiron.screen".into(),
            version: 1,
            screen: screen.clone(),
            tiles: IndexMap::new(),
            materials: IndexMap::new(),
            compositions: IndexMap::new(),
            bindings: IndexMap::new(),
            palette: project.palette.clone(),
        };
        let mut pending = references(screen)?;
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop_first() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(tile) = project.tiles.get(&id) {
                package.tiles.insert(id, tile.clone());
            }
            if let Some(recipe) = project.procedural_materials.get(&id) {
                // Bundles embed MZ documents and images; linked project tile images
                // must travel too so subsequent edits can refresh their pixels.
                EditableMaterialAsset::reopen(recipe).map_err(|e| e.to_string())?;
                pending.extend(references(recipe)?);
                package.materials.insert(id, recipe.clone());
            }
            if let Some(binding) = project
                .map_material_bindings
                .get(&id)
                .filter(|b| b.map == screen.map.id)
            {
                package.bindings.insert(id, binding.clone());
            }
            if let Some(composition) = project.material_compositions.get(&id) {
                pending.extend(references(composition)?);
                package.compositions.insert(id, composition.clone());
            }
        }
        Ok(package)
    }

    /// Prepare a complete replacement project for atomic import and undo.
    /// Existing palette slots and assets are never overwritten by an import.
    pub fn imported_project(&self, project: &Project) -> Result<(Project, Uuid), String> {
        if self.format != "eldiron.screen" || self.version != 1 {
            return Err("Unsupported screen package".into());
        }
        let target: Vec<_> = project
            .palette
            .colors
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.as_ref().map(|c| (i, c.clone())))
            .collect();
        if target.is_empty() {
            return Err("The project palette has no colors".into());
        }
        let colors: Vec<_> = target.iter().map(|(_, c)| c.clone()).collect();
        let palette: HashMap<_, _> = self
            .palette
            .colors
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                c.as_ref()
                    .and_then(|c| crate::palette::closest(c, &colors))
                    .map(|next| (i, target[next].0))
            })
            .collect();
        let mut ids: HashMap<_, _> = self
            .tiles
            .keys()
            .chain(self.materials.keys())
            .chain(self.compositions.keys())
            .chain(self.bindings.keys())
            .map(|id| (*id, Uuid::new_v4()))
            .collect();
        ids.insert(self.screen.id, Uuid::new_v4());
        ids.insert(self.screen.map.id, Uuid::new_v4());
        let mut next = project.clone();
        let mut screen = serde_json::to_value(&self.screen).map_err(|e| e.to_string())?;
        rewrite(&mut screen, &ids, &palette);
        let mut screen: Screen = serde_json::from_value(screen).map_err(|e| e.to_string())?;
        for sector in &mut screen.map.sectors {
            sector.creator_id = Uuid::new_v4();
        }
        let base = if screen.name.trim().is_empty() {
            "Screen"
        } else {
            screen.name.trim()
        }
        .to_string();
        let mut name = base.clone();
        let mut suffix = 2;
        while project
            .screens
            .values()
            .any(|s| s.name.eq_ignore_ascii_case(&name) || s.map.name.eq_ignore_ascii_case(&name))
        {
            name = format!("{base} ({suffix})");
            suffix += 1;
        }
        screen.name = name.clone();
        screen.map.name = name;
        let screen_id = screen.id;
        let mut matched = HashMap::new();
        for (id, tile) in &self.tiles {
            let mut value = serde_json::to_value(tile).map_err(|e| e.to_string())?;
            rewrite(&mut value, &ids, &palette);
            let mut tile: rusterix::Tile =
                serde_json::from_value(value).map_err(|e| e.to_string())?;
            for texture in &mut tile.textures {
                for pixel in texture.data.chunks_exact_mut(4) {
                    if pixel[3] == 0 {
                        continue;
                    }
                    let rgb: [u8; 3] = pixel[..3].try_into().unwrap();
                    let color = matched.entry(rgb).or_insert_with(|| {
                        let source = TheColor::from_u8_array([rgb[0], rgb[1], rgb[2], 255]);
                        colors[crate::palette::closest(&source, &colors).unwrap()].to_u8_array()
                    });
                    pixel[..3].copy_from_slice(&color[..3]);
                }
            }
            next.tiles.insert(ids[id], tile);
        }
        for (id, recipe) in &self.materials {
            let mut value = serde_json::json!(recipe);
            rewrite(&mut value, &ids, &palette);
            let mut asset = EditableMaterialAsset::reopen(value.as_str().unwrap())
                .map_err(|e| e.to_string())?;
            for record in &mut asset.instances {
                for binding in &mut record.palette_bindings {
                    binding.index = *palette
                        .get(&(binding.index as usize))
                        .ok_or("Screen material references an empty palette slot")?
                        as u32;
                }
            }
            rusterix::ruleset_material::resolve_palette(&mut asset, &project.palette)?;
            next.procedural_materials
                .insert(ids[id], asset.save().map_err(|e| e.to_string())?);
        }
        for (id, composition) in &self.compositions {
            let mut value = serde_json::to_value(composition).map_err(|e| e.to_string())?;
            rewrite(&mut value, &ids, &palette);
            next.material_compositions.insert(
                ids[id],
                serde_json::from_value(value).map_err(|e| e.to_string())?,
            );
        }
        for (id, binding) in &self.bindings {
            let mut value = serde_json::to_value(binding).map_err(|e| e.to_string())?;
            rewrite(&mut value, &ids, &palette);
            next.map_material_bindings.insert(
                ids[id],
                serde_json::from_value(value).map_err(|e| e.to_string())?,
            );
        }
        next.screens.insert(screen_id, screen);
        Ok((next, screen_id))
    }
}
