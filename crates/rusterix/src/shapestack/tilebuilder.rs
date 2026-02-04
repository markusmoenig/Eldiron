use crate::ShapeStack;
use crate::prelude::*;

use indexmap::IndexMap;
use theframework::prelude::FxHashMap;
use vek::{Vec2, Vec4};

/// Builds tiles for entities and items
pub fn tile_builder(map: &mut Map, assets: &mut Assets) {
    let size = 64;

    for entity in map.entities.iter() {
        if entity.attributes.contains("source") {
            continue;
        }

        // Check if we have a sequence to build / check
        if let Some(PixelSource::Sequence(name)) = entity.attributes.get_source("_source_seq") {
            if let Some(entity_tiles) = assets.entity_tiles.get(&entity.id) {
                if !entity_tiles.contains_key(name) {
                    // No sequence of this name for the entity, build the sequence
                    println!(
                        "No sequences ({}) for {}",
                        name,
                        entity.attributes.get_str_default("name", "unknown".into())
                    );

                    if let Some(Value::Str(class_name)) = entity.attributes.get("class_name") {
                        if let Some(character_map) = assets.character_maps.get(class_name) {
                            let sector_overrides = compute_sector_overrides(character_map, entity);
                            let tile = build_tile(
                                character_map,
                                assets,
                                name,
                                size,
                                &sector_overrides,
                                Some(entity),
                            );
                            if let Some(entity_tiles) = assets.entity_tiles.get_mut(&entity.id) {
                                entity_tiles.insert(name.clone(), tile);
                            }
                        }
                    }
                }
            } else {
                // No sequences for this character at all, build the sequence
                // println!(
                //     "No sequences at all ({}) for character {}",
                //     name,
                //     entity.attributes.get_str_default("name", "unknown".into())
                // );

                if let Some(Value::Str(class_name)) = entity.attributes.get("class_name") {
                    if let Some(character_map) = assets.character_maps.get(class_name) {
                        let sector_overrides = compute_sector_overrides(character_map, entity);
                        let tile = build_tile(
                            character_map,
                            assets,
                            name,
                            size,
                            &sector_overrides,
                            Some(entity),
                        );
                        let mut states: IndexMap<String, Tile> = IndexMap::default();
                        states.insert(name.clone(), tile);

                        assets.entity_tiles.insert(entity.id, states);
                    }
                }
            }
        }
    }

    for item in map.items.iter() {
        if item.attributes.contains("source") {
            continue;
        }

        // Check if we have a sequence to build / check
        if let Some(PixelSource::Sequence(name)) = item.attributes.get_source("_source_seq") {
            if let Some(item_tiles) = assets.item_tiles.get(&item.id) {
                if !item_tiles.contains_key(name) {
                    // No sequence of this name for the entity, build the sequence
                    println!(
                        "No sequences ({}) for {}",
                        name,
                        item.attributes.get_str_default("name", "unknown".into())
                    );

                    if let Some(Value::Str(class_name)) = item.attributes.get("class_name") {
                        if let Some(item_map) = assets.item_maps.get(class_name) {
                            let tile = build_tile(
                                item_map,
                                assets,
                                name,
                                size,
                                &FxHashMap::default(),
                                None,
                            );
                            if let Some(item_tiles) = assets.entity_tiles.get_mut(&item.id) {
                                item_tiles.insert(name.clone(), tile);
                            }
                        }
                    }
                }
            } else {
                // No sequences for this item at all, build the sequence
                // println!(
                //     "No sequences at all ({}) for item {}",
                //     name,
                //     item.attributes.get_str_default("name", "unknown".into())
                // );

                if let Some(Value::Str(class_name)) = item.attributes.get("class_name") {
                    if let Some(item_map) = assets.item_maps.get(class_name) {
                        let tile =
                            build_tile(item_map, assets, name, size, &FxHashMap::default(), None);
                        let mut states: IndexMap<String, Tile> = IndexMap::default();
                        states.insert(name.clone(), tile);

                        assets.item_tiles.insert(item.id, states);
                    }
                }
            }
        }
    }
}

fn build_tile(
    map: &Map,
    assets: &Assets,
    base_sequence: &str,
    size: i32,
    sector_overrides: &FxHashMap<u32, Vec4<f32>>,
    entity: Option<&Entity>,
) -> Tile {
    let mut matched_rigs: Vec<(&SoftRig, usize)> = map
        .softrigs
        .values()
        .filter_map(|rig| {
            let name = rig.name.to_lowercase();
            let base = base_sequence.to_lowercase();

            if name.starts_with(&base) {
                let suffix = &rig.name[base.len()..];
                let num = suffix
                    .trim_start_matches(|c: char| !c.is_ascii_digit())
                    .parse::<usize>()
                    .unwrap_or(0);
                Some((rig, num))
            } else {
                None
            }
        })
        .collect();

    matched_rigs.sort_by_key(|(_, num)| *num);

    // for (rig, index) in &matched_rigs {
    //     println!("{} {}", rig.name, index);
    // }

    let mut forward_textures = Vec::new();
    let frames_per_transition = 3;

    match matched_rigs.len() {
        0 => {
            // Nothing matched
            let mut texture = Texture::alloc(size as usize, size as usize);
            let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
            stack.render_geometry(&mut texture, map, assets, false, sector_overrides);

            if let Some(entity) = entity {
                let map = extract_anchored_geometry(entity, map, assets);
                if !map.vertices.is_empty() {
                    stack.render_geometry(&mut texture, &map, assets, false, &FxHashMap::default());
                }
            }
            forward_textures.push(texture);
        }
        1 => {
            // Only one rig
            let rig = matched_rigs[0].0;
            let mut temp_map = map.geometry_clone();
            temp_map.editing_rig = Some(rig.id);
            temp_map.softrigs.insert(rig.id, rig.clone());

            let mut texture = Texture::alloc(size as usize, size as usize);
            let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
            stack.render_geometry(&mut texture, &temp_map, assets, false, sector_overrides);

            if let Some(entity) = entity {
                let map = extract_anchored_geometry(entity, &temp_map, assets);
                if !map.vertices.is_empty() {
                    stack.render_geometry(&mut texture, &map, assets, false, &FxHashMap::default());
                }
            }
            forward_textures.push(texture);
        }
        _ => {
            let skip_last_frame_each = true;
            let skip_last_frame_final = true;
            let loop_back_to_start = true;

            let rig_count = matched_rigs.len();
            let transition_count = if loop_back_to_start {
                rig_count
            } else {
                rig_count.saturating_sub(1)
            };

            for i in 0..transition_count {
                let rig_a = matched_rigs[i].0;
                let rig_b = matched_rigs[(i + 1) % rig_count].0;

                let is_final = i == transition_count - 1;

                let skip = skip_last_frame_each || (is_final && skip_last_frame_final);
                let max_f = if skip {
                    frames_per_transition - 1
                } else {
                    frames_per_transition
                };

                for f in 0..max_f {
                    let t = f as f32 / (frames_per_transition - 1) as f32;

                    let blended = SoftRigAnimator::blend_softrigs(rig_a, rig_b, t, map);

                    let mut temp_map = map.geometry_clone();
                    temp_map.editing_rig = Some(blended.id);
                    temp_map.softrigs.insert(blended.id, blended);

                    let mut texture = Texture::alloc(size as usize, size as usize);
                    let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
                    stack.render_geometry(&mut texture, &temp_map, assets, false, sector_overrides);

                    if let Some(entity) = entity {
                        let map = extract_anchored_geometry(entity, &temp_map, assets);
                        if !map.vertices.is_empty() {
                            stack.render_geometry(
                                &mut texture,
                                &map,
                                assets,
                                false,
                                &FxHashMap::default(),
                            );
                        }
                    }
                    forward_textures.push(texture);
                }
            }
        }
    }

    Tile::from_textures(forward_textures)
}

/// Creates a new map with the extracted and transformed item geometries which are anchored to slots in the character_map.
fn extract_anchored_geometry(entity: &Entity, character_map: &Map, assets: &Assets) -> Map {
    let mut new_map = Map::default();

    for (_, item) in entity.equipped.iter() {
        let targets: Vec<&str> =
            if let Some(Value::StrArray(geo_mods)) = item.attributes.get("geo_targets") {
                geo_mods.iter().map(|s| s.as_str()).collect()
            } else if let Some(Value::Str(slot_name)) = item.attributes.get("slot") {
                vec![slot_name.as_str()]
            } else {
                continue;
            };

        let Some(Value::Str(class_name)) = item.attributes.get("class_name") else {
            continue;
        };
        let Some(item_map) = assets.item_maps.get(class_name) else {
            continue;
        };

        for (id, graph) in &item_map.shapefx_graphs {
            new_map.shapefx_graphs.insert(*id, graph.clone());
        }

        // Find linedef in character map with name == target_name
        for target_name in targets {
            let Some(target_linedef) = character_map
                .linedefs
                .iter()
                .find(|l| l.name == target_name)
            else {
                continue;
            };

            let Some(v0) = character_map.get_vertex(target_linedef.start_vertex) else {
                continue;
            };
            let Some(v1) = character_map.get_vertex(target_linedef.end_vertex) else {
                continue;
            };

            let target_mid = (v0 + v1) * 0.5;

            // Find origin (0,0) in item map
            let item_origin = Vec2::zero(); // You may later allow item-origin markup if needed

            let offset = target_mid - item_origin;

            // Create vertex ID mapping
            let mut id_map = FxHashMap::default();

            for v in &item_map.vertices {
                let new_id = new_map.vertices.len() as u32;
                id_map.insert(v.id, new_id);

                let mut new_v = v.clone();
                new_v.id = new_id;
                new_v.x += offset.x;
                new_v.y += offset.y;

                new_map.vertices.push(new_v);
            }

            for l in &item_map.linedefs {
                let mut new_l = l.clone();
                new_l.id = new_map.linedefs.len() as u32;
                new_l.start_vertex = *id_map.get(&l.start_vertex).unwrap();
                new_l.end_vertex = *id_map.get(&l.end_vertex).unwrap();
                new_map.linedefs.push(new_l);
            }

            for s in &item_map.sectors {
                let mut new_s = s.clone();

                new_s.id = new_map.sectors.len() as u32;
                new_s.linedefs = s
                    .linedefs
                    .iter()
                    .map(|id| {
                        let orig = item_map.linedefs.iter().find(|l| l.id == *id).unwrap();
                        new_map
                            .linedefs
                            .iter()
                            .find(|l2| {
                                l2.name == orig.name
                                    && l2.start_vertex == *id_map.get(&orig.start_vertex).unwrap()
                            })
                            .map(|l2| l2.id)
                            .unwrap_or(0)
                    })
                    .collect();
                new_map.sectors.push(new_s);
            }
        }
    }

    new_map
}

/// Get the color overrides of items for the geometry.
fn compute_sector_overrides(map: &Map, entity: &Entity) -> FxHashMap<u32, Vec4<f32>> {
    let mut sector_overrides: FxHashMap<u32, Vec4<f32>> = FxHashMap::default();

    for (_, item) in entity.equipped.iter() {
        if let Some(Value::Color(color)) = item.attributes.get("color") {
            if let Some(Value::StrArray(color_targets)) = item.attributes.get("color_targets") {
                for sector in &map.sectors {
                    if color_targets.contains(&sector.name) {
                        sector_overrides.insert(sector.id, color.to_vec4());
                    }
                }
            }
        }
    }

    sector_overrides
}
