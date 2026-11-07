//! Build the standalone procedural remake without modifying Hideout2D.eldiron.
use denrim_materials::integration::{
    EditableMaterialAsset, InstanceRecord, PaletteBinding, ParameterKey,
};
use serde::Deserialize;
use shared::project_io::{decode_project, encode_project};
use std::{collections::BTreeMap, path::PathBuf};
use theframework::prelude::*;
#[derive(Deserialize)]
struct Recipe {
    index: usize,
    id: Uuid,
    name: String,
    source: String,
    palette: String,
    frames: usize,
    family: String,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from("test_projects/Hideout2D-MZ");
    let options = std::env::args().skip(1).collect::<Vec<_>>();
    let probe = options.iter().any(|s| s == "--probe");
    let mut project = decode_project(&std::fs::read("test_projects/Hideout2D.eldiron")?)?;
    let reference = project.clone();
    let cached = (!options.iter().any(|s| s == "--fresh"))
        .then(|| {
            std::fs::read("test_projects/Hideout2D-MZ.eldiron")
                .ok()
                .and_then(|b| decode_project(&b).ok())
        })
        .flatten()
        .filter(|p| p.palette.colors == project.palette.colors);
    let recipes: Vec<Recipe> = serde_json::from_slice(&std::fs::read(root.join("manifest.json"))?)?;
    let indices = recipes
        .iter()
        .map(|r| (r.id, r.index))
        .collect::<BTreeMap<_, _>>();
    let ids = recipes
        .iter()
        .map(|r| (r.index, r.id))
        .collect::<BTreeMap<_, _>>();
    project.procedural_materials.clear();
    project.procedural_material_cache_keys.clear();
    let previews = root.join("previews");
    std::fs::create_dir_all(&previews)?;
    let started = std::time::Instant::now();
    for recipe in recipes {
        if probe
            && ![
                0, 9, 19, 23, 32, 34, 37, 52, 55, 56, 57, 67, 73, 81, 99, 105, 106, 132, 153,
            ]
            .contains(&recipe.index)
        {
            continue;
        }
        let source = std::fs::read_to_string(root.join(&recipe.source))?;
        let mut asset = EditableMaterialAsset::parse(&recipe.id.to_string(), &source, "surface")
            .map_err(|e| format!("{}: {e}", recipe.source))?;
        let bindings: BTreeMap<String, BTreeMap<String, u32>> =
            serde_json::from_slice(&std::fs::read(root.join(&recipe.palette))?)?;
        let mut palette = InstanceRecord::new("eldiron-palette");
        for (op, params) in bindings {
            for (param, index) in params {
                palette.palette_bindings.push(PaletteBinding {
                    key: ParameterKey::new(&asset.root.id, &op, &param),
                    index,
                });
            }
        }
        asset.instances.push(palette);
        rusterix::ruleset_material::resolve_palette(&mut asset, &project.palette)?;
        asset.validate()?;
        let bundle = asset.save()?;
        let tile = project
            .tiles
            .get_mut(&recipe.id)
            .ok_or("Missing tile identity")?;
        assert!(
            !source.contains("Image Source"),
            "No reference bitmap wrappers"
        );
        tile.textures.clear();
        tile.uv_pages = None;
        tile.module = None;
        tile.alias = recipe.name;
        let bake_key = |bundle: &str| {
            format!("palette-tick-v4:16\n{bundle}")
                .bytes()
                .fold(0xcbf29ce484222325u64, |h, b| {
                    (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
                })
        };
        let cached_tile = cached
            .as_ref()
            .filter(|p| {
                p.procedural_materials.get(&recipe.id) == Some(&bundle)
                    && p.procedural_material_cache_keys.get(&recipe.id) == Some(&bake_key(&bundle))
            })
            .and_then(|p| p.tiles.get(&recipe.id))
            .filter(|t| {
                t.textures.len() == recipe.frames
                    && t.uv_pages.is_none()
                    && t.textures.iter().all(|t| t.width == 16 && t.height == 16)
            });
        if let Some(previous) = cached_tile {
            tile.textures = previous.textures.clone();
            for (texture, bytes) in tile.textures.iter_mut().zip(&previous.baked_material_data) {
                assert!(texture.apply_material_bytes(bytes));
            }
        } else {
            for frame in 0..recipe.frames {
                tile.textures.push(rusterix::ruleset_material::bake_at_time(
                    &asset,
                    &project.palette,
                    16,
                    frame as f32 / recipe.frames as f32,
                )?);
            }
        }
        assert!(
            tile.textures
                .iter()
                .all(|t| t.data.chunks_exact(4).any(|p| p[3] > 0)),
            "Every rebuilt tile/frame must be visible: {}",
            recipe.index
        );
        if matches!(
            recipe.family.as_str(),
            "grass" | "furrows" | "road" | "masonry" | "deck" | "water" | "ship"
        ) {
            assert!(
                tile.textures
                    .iter()
                    .all(|t| t.data.chunks_exact(4).all(|p| p[3] == 255)),
                "Terrain must cover every pixel in every animation frame: {}",
                recipe.index
            );
        }
        if recipe.index == 81 || recipe.index == 82 {
            assert!(
                tile.textures
                    .iter()
                    .skip(1)
                    .any(|t| t.data != tile.textures[0].data),
                "Water must actually animate"
            );
        }
        tile.baked_material_data = tile
            .textures
            .iter()
            .map(rusterix::Texture::material_bytes)
            .collect();
        let tex = &tile.textures[0];
        std::fs::write(
            previews.join(format!("{:03}.png", recipe.index)),
            TheRGBABuffer::from(tex.data.clone(), tex.width as u32, tex.height as u32).to_png()?,
        )?;
        let key = bake_key(&bundle);
        project.procedural_materials.insert(recipe.id, bundle);
        project
            .procedural_material_cache_keys
            .insert(recipe.id, key);
        eprintln!(
            "{:03} {} ({:.1}s)",
            recipe.index,
            recipe.source,
            started.elapsed().as_secs_f32()
        );
    }
    if probe {
        return Ok(());
    }
    // Characters inherit the ruleset appearance for their race. Remove explicit legacy
    // sprite overrides from templates and placements while preserving all other setup.
    fn ruleset_character(data: &str) -> Result<String, Box<dyn std::error::Error>> {
        if data.trim().is_empty() {
            return Ok(String::new());
        }
        let mut doc = data.parse::<toml_edit::DocumentMut>()?;
        if let Some(attrs) = doc.get_mut("attributes").and_then(|v| v.as_table_mut()) {
            attrs.remove("tile_id");
            attrs.remove("source");
        }
        Ok(doc.to_string())
    }
    // Remove the corresponding startup graph overrides too; otherwise graph
    // synchronization would restore the old sprites when opening the project.
    for (key, graph) in &mut project.node_graphs {
        if !key.starts_with("entity/character/") && !key.starts_with("entity/instance/") {
            continue;
        }
        let mut ports = std::collections::BTreeSet::new();
        if let Some(nodes) = graph["nodes"].as_array_mut() {
            nodes.retain(|n| {
                if n["definition"] == "entity_set_tile" {
                    if let Some(ps) = n["ports"].as_array() {
                        for p in ps {
                            if let Some(id) = p["id"].as_str() {
                                ports.insert(id.to_owned());
                            }
                        }
                    }
                    false
                } else {
                    true
                }
            });
        }
        if let Some(cs) = graph["connections"].as_array_mut() {
            cs.retain(|c| {
                !ports.contains(c["from"].as_str().unwrap_or(""))
                    && !ports.contains(c["to"].as_str().unwrap_or(""))
            });
        }
    }
    for c in project.characters.values_mut() {
        c.data = ruleset_character(&c.data)?;
    }
    for r in &mut project.regions {
        for c in r.characters.values_mut() {
            c.data = ruleset_character(&c.data)?;
        }
    }
    // Archive JSON can reorder object keys. Tile UUIDs, rather than decoded map
    // order, distinguish the six retired sprites from the 150 generated assets.
    project
        .tiles
        .retain(|id, _| project.procedural_materials.contains_key(id));
    // Visual ground under old decorations; geometry and collision stay unchanged.
    for region in &mut project.regions {
        let floors = region
            .map
            .sectors
            .iter()
            .filter_map(|sector| {
                let rusterix::PixelSource::TileId(id) = sector.properties.get_default_source()?
                else {
                    return None;
                };
                let index = *indices.get(id)?;
                let base = match index {
                    19 | 23 | 54 => index,
                    0 | 1 | 3 | 4 | 5 | 8 | 11 | 12 | 13 | 14 | 30 | 99 | 103 => 0,
                    _ => return None,
                };
                Some((sector.center(&region.map)?, base))
            })
            .collect::<Vec<_>>();
        // Every cutout used as map terrain needs ground, including fragments,
        // mountain silhouettes, trees and fence ends. Inspect coverage instead
        // of maintaining a partial list of prop indices.
        let overlays = region
            .map
            .sectors
            .iter()
            .filter_map(|sector| {
                let rusterix::PixelSource::TileId(id) = sector.properties.get_default_source()?
                else {
                    return None;
                };
                let tile = project.tiles.get(id)?;
                if !tile
                    .textures
                    .iter()
                    .any(|t| t.data.chunks_exact(4).any(|p| p[3] < 255))
                {
                    return None;
                }
                let center = sector.center(&region.map)?;
                let base = if matches!(indices.get(id), Some(100 | 105..=110)) {
                    0
                } else {
                    floors
                        .iter()
                        .min_by(|(a, _), (b, _)| {
                            ((*a - center).magnitude_squared())
                                .total_cmp(&((*b - center).magnitude_squared()))
                        })?
                        .1
                };
                Some((sector.id, ids[&base]))
            })
            .collect::<Vec<_>>();
        for (id, base) in overlays {
            let sector = region.map.sectors.iter_mut().find(|s| s.id == id).unwrap();
            sector.properties.set(
                "underlay_source",
                rusterix::Value::Source(rusterix::PixelSource::TileId(base)),
            );
        }
    }
    project.tilemaps.clear();
    for r in &mut project.regions {
        r.map.entities.clear();
        r.map.items.clear();
    }
    project.tile_board_tiles.clear();
    project.tile_board_empty_slots.clear();
    for (i, id) in project.tiles.keys().enumerate() {
        project
            .tile_board_tiles
            .insert(*id, Vec2::new((i % 13) as i32, (i / 13) as i32));
    }
    project.tile_board_cols = 13;
    project.tile_board_rows = 12;
    project.name = "Hideout2D — MZ procedural experiment".into();
    let mut config = project.config.parse::<toml_edit::DocumentMut>()?;
    config["materials"]["pixels_per_tile"] = toml_edit::value(16);
    config["appearance"]["sprite_pixels_2d"] = toml_edit::value(32);
    config["materials"]["background_2d"] = toml_edit::value("transparent");
    project.config = config.to_string();
    // Persist the ruleset library too: opening the experiment is not a bake job.
    if let Some(cached) = &cached {
        for def in shared::ruleset_materials::definitions(&project.config) {
            let id = shared::rulesets::material_asset_id(def.id);
            let bundle = rusterix::ruleset_material::editable(&def, &project.palette)?.save()?;
            if cached.procedural_materials.get(&id) == Some(&bundle)
                && let (Some(tile), Some(key)) = (
                    cached.tiles.get(&id),
                    cached.procedural_material_cache_keys.get(&id),
                )
            {
                project.procedural_materials.insert(id, bundle);
                project.tiles.insert(id, tile.clone());
                project.procedural_material_cache_keys.insert(id, *key);
            }
        }
    }
    shared::ruleset_materials::install(&mut project)?;
    let path = "test_projects/Hideout2D-MZ.eldiron";
    std::fs::write(path, encode_project(&project)?)?;
    let reopened = decode_project(&std::fs::read(path)?)?;
    assert_eq!(
        reopened
            .tiles
            .keys()
            .filter(|id| indices.contains_key(id))
            .count(),
        150
    );
    assert!(
        reopened
            .procedural_materials
            .keys()
            .all(|id| reopened.tiles.contains_key(id))
    );
    assert!(reopened.tilemaps.is_empty());
    for bundle in reopened.procedural_materials.values() {
        EditableMaterialAsset::reopen(bundle)?.validate()?;
    }
    assert_eq!(reopened.regions.len(), reference.regions.len());
    for (id, tile) in &reopened.tiles {
        let Some(original) = reference.tiles.get(id) else {
            assert!(shared::ruleset_materials::is_library_tile(
                &reopened.config,
                id
            ));
            continue;
        };
        assert_eq!(tile.blocking, original.blocking);
        assert_eq!(tile.role, original.role);
        assert_eq!(tile.scale, original.scale);
        assert!(
            tile.textures
                .iter()
                .all(|t| t.width == 16 && t.height == 16)
        );
    }
    for region in &reopened.regions {
        for sector in &region.map.sectors {
            let Some(rusterix::PixelSource::TileId(id)) = sector.properties.get_default_source()
            else {
                continue;
            };
            let Some(tile) = reopened.tiles.get(id) else {
                continue;
            };
            if tile
                .textures
                .iter()
                .any(|t| t.data.chunks_exact(4).any(|p| p[3] < 255))
            {
                let Some(rusterix::PixelSource::TileId(base)) =
                    sector.properties.get_source("underlay_source")
                else {
                    panic!(
                        "{} sector {} has transparent terrain without ground",
                        region.name, sector.id
                    );
                };
                assert!(
                    reopened.tiles[base]
                        .textures
                        .iter()
                        .all(|t| t.data.chunks_exact(4).all(|p| p[3] == 255)),
                    "Ground beneath {} sector {} must be opaque",
                    region.name,
                    sector.id
                );
            }
        }
    }
    assert_eq!(reopened.palette, reference.palette);
    assert_eq!(reopened.characters.len(), reference.characters.len());
    assert_eq!(reopened.items.len(), reference.items.len());
    for (a, b) in reopened.regions.iter().zip(&reference.regions) {
        assert_eq!(a.map.vertices, b.map.vertices);
        assert_eq!(a.map.linedefs, b.map.linedefs);
        let mut sectors = a.map.sectors.clone();
        for sector in &mut sectors {
            sector.properties.remove("underlay_source");
        }
        assert_eq!(sectors, b.map.sectors);
        assert_eq!(a.characters.len(), b.characters.len());
        assert_eq!(
            serde_json::to_value(&a.items)?,
            serde_json::to_value(&b.items)?
        );
        for (id, c) in &a.characters {
            assert_eq!(c.position, b.characters[id].position);
            assert_eq!(c.orientation, b.characters[id].orientation);
            assert_eq!(c.character_id, b.characters[id].character_id);
        }
    }
    eprintln!(
        "Saved {path}: {} recipes in {:.1}s; roundtrip and unchanged map geometry verified",
        reopened.procedural_materials.len(),
        started.elapsed().as_secs_f32()
    );
    Ok(())
}
