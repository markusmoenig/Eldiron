//! Compile geometry-owned terrain material settings into ordinary MZ mixtures.
//! One derived result per painted background preserves existing tile/blend edits.
use super::*;
use rusterix::chunkbuilder::terrain_material::{self, Outputs};
use rusterix::{PixelSource, Value, ValueContainer};
use shared::material_composition::CompositionGeometry;
use std::collections::BTreeSet;

pub(super) fn authored(project: &Project) -> bool {
    project.regions.iter().any(|r| {
        r.map.linedefs.iter().any(|l| has_source(&l.properties))
            || r.map.sectors.iter().any(|s| has_source(&s.properties))
    })
}
fn has_source(p: &ValueContainer) -> bool {
    p.get_source("terrain_material_source").is_some()
        || p.get_source("terrain_material_verge_source").is_some()
}
fn layers(map: &Map) -> Vec<CompositionLayer> {
    let mut shapes: Vec<_> = map
        .sectors
        .iter()
        .map(|s| {
            (
                s.properties.get_int_default("terrain_material_order", 0),
                false,
                s.id,
                &s.properties,
            )
        })
        .chain(map.linedefs.iter().map(|l| {
            (
                l.properties.get_int_default("terrain_material_order", 0),
                true,
                l.id,
                &l.properties,
            )
        }))
        .collect();
    shapes.sort_by_key(|(order, line, id, _)| (*order, *line, *id));
    let mut groups: Vec<(i32, CompositionLayer)> = vec![];
    for (order, line, id, p) in shapes {
        for verge in if line { vec![true, false] } else { vec![false] } {
            let key = if verge {
                "terrain_material_verge_source"
            } else {
                "terrain_material_source"
            };
            let Some(PixelSource::TileId(material)) = p.get_source(key) else {
                continue;
            };
            let mut layer = CompositionLayer::split(*material);
            layer.shape = if line {
                CompositionShape::MapPath
            } else {
                CompositionShape::MapArea
            };
            layer.position = if line {
                p.get_float_default(
                    if verge {
                        "terrain_material_verge_width"
                    } else {
                        "terrain_material_width"
                    },
                    if verge { 3.2 } else { 1.35 },
                )
                .clamp(0.1, 64.)
            } else {
                p.get_float_default("terrain_material_inset", 0.)
                    .clamp(0., 64.)
            };
            layer.feather = p
                .get_float_default("terrain_material_soft_edge", 0.45)
                .clamp(0., 8.);
            layer.wobble = p
                .get_float_default("terrain_material_noisy_edge", 0.13)
                .clamp(0., 2.);
            layer.wobble_scale = p
                .get_float_default("terrain_material_noise_scale", 2.7)
                .clamp(0.1, 16.);
            let same = |l: &CompositionLayer| {
                l.material == layer.material
                    && l.shape == layer.shape
                    && l.position == layer.position
                    && l.feather == layer.feather
                    && l.wobble == layer.wobble
                    && l.wobble_scale == layer.wobble_scale
            };
            if let Some((_, existing)) = groups.iter_mut().find(|(o, l)| *o == order && same(l)) {
                let geometry = existing.geometry.as_mut().unwrap();
                if line {
                    geometry.linedefs.push(id);
                } else {
                    geometry.sectors.push(id);
                }
            } else {
                layer.geometry = Some(CompositionGeometry {
                    map: map.id,
                    linedefs: if line { vec![id] } else { vec![] },
                    sectors: if line { vec![] } else { vec![id] },
                });
                groups.push((order, layer));
            }
        }
    }
    groups.into_iter().map(|(_, l)| l).collect()
}
pub(super) fn sync(project: &mut Project) {
    let mut wanted = BTreeSet::new();
    for index in 0..project.regions.len() {
        let map = &project.regions[index].map;
        let layers = if map.properties.get_bool_default("terrain_enabled", false) {
            layers(map)
        } else {
            vec![]
        };
        let mut bases = BTreeSet::new();
        let mut solids = std::collections::BTreeMap::new();
        let mut add = |source: &PixelSource| match source {
            PixelSource::TileId(id) => {
                bases.insert(*id);
            }
            PixelSource::Color(color) => {
                let id = PixelSource::color_tile_uuid(color);
                bases.insert(id);
                solids.insert(id, color.clone());
            }
            PixelSource::PaletteIndex(index) => {
                if let Some(Some(color)) = project.palette.colors.get(*index as usize) {
                    let id = PixelSource::palette_tile_uuid(*index);
                    bases.insert(id);
                    solids.insert(id, color.clone());
                }
            }
            _ => {}
        };
        if !layers.is_empty() {
            if let Some(source) = map.properties.get_source("default_terrain_tile") {
                add(source);
            }
            for p in
                std::iter::once(&map.properties).chain(map.sectors.iter().map(|s| &s.properties))
            {
                if let Some(Value::TileOverrides(tiles)) = p.get("tiles") {
                    for source in tiles.values() {
                        add(source);
                    }
                }
                if let Some(Value::BlendOverrides(tiles)) = p.get("blend_tiles") {
                    for (_, source) in tiles.values() {
                        add(source);
                    }
                }
            }
            for s in &map.sectors {
                if s.properties.get_float_default("floor_height", 0.).abs() < 0.001 {
                    if let Some(source) = s.properties.get_default_source() {
                        if matches!(source, PixelSource::TileId(id) if project.tiles.get(id).is_some_and(|t| t.isometric.as_ref().is_some_and(|s| s.ground)))
                        {
                            add(source);
                        }
                    }
                }
            }
        }
        let map_id = map.id;
        for (id, color) in solids {
            wanted.insert(id);
            if !project.tiles.contains_key(&id) {
                let mut tile = rusterix::Tile::from_texture(rusterix::Texture::from_color(
                    color.to_u8_array(),
                ));
                tile.id = id;
                tile.alias = format!("{}background/{id}", terrain_material::PREFIX);
                project.tiles.insert(id, tile);
            }
        }
        let mut outputs = Outputs::new();
        for base in bases {
            let Some(source) = project.tiles.get(&base).filter(|t| {
                !t.alias.starts_with(terrain_material::PREFIX)
                    || t.alias
                        .starts_with(&format!("{}background/", terrain_material::PREFIX))
            }) else {
                continue;
            };
            let output = terrain_material::output_id(map_id, base);
            let mut layers = layers.clone();
            for (i, layer) in layers.iter_mut().enumerate() {
                layer.id = Uuid::from_u128(output.as_u128().wrapping_add(i as u128 + 1));
            }
            let composition = MaterialComposition {
                base,
                layers,
                tiling: Default::default(),
            };
            if project.material_compositions.get(&output) != Some(&composition) {
                project.material_compositions.insert(output, composition);
            }
            if !project.tiles.contains_key(&output) {
                let mut tile = source.clone();
                tile.id = output;
                tile.alias = format!("{}{map_id}/{base}", terrain_material::PREFIX);
                tile.isometric = None;
                project.tiles.insert(output, tile);
            }
            outputs.insert(base, output);
            wanted.insert(output);
        }
        let map = &mut project.regions[index].map;
        let next =
            (!outputs.is_empty()).then(|| Value::Str(serde_json::to_string(&outputs).unwrap()));
        if map.properties.get(terrain_material::OUTPUTS) != next.as_ref() {
            if let Some(value) = next {
                map.properties.set(terrain_material::OUTPUTS, value);
            } else {
                map.properties.remove(terrain_material::OUTPUTS);
            }
            map.changed = map.changed.wrapping_add(1);
        }
    }
    let obsolete: Vec<_> = project
        .tiles
        .iter()
        .filter(|(id, t)| t.alias.starts_with(terrain_material::PREFIX) && !wanted.contains(id))
        .map(|(id, _)| *id)
        .collect();
    for id in obsolete {
        project.tiles.shift_remove(&id);
        project.procedural_materials.shift_remove(&id);
        project.procedural_material_cache_keys.shift_remove(&id);
        project.material_compositions.shift_remove(&id);
        project.tile_board_tiles.shift_remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(p: &mut Project, rgb: [f32; 3], roughness: f32) -> Uuid {
        let tile =
            rusterix::Tile::from_texture(rusterix::Texture::from_color([100, 100, 100, 255]));
        let id = tile.id;
        let recipe = format!(
            "project 1\nmaterial \"surface\" called \"Source\"\n  mode pbr\n  default base color to {} {} {}\n  default roughness to {roughness}\n",
            rgb[0], rgb[1], rgb[2]
        );
        let asset = EditableMaterialAsset::parse(&id.to_string(), &recipe, "surface").unwrap();
        p.tiles.insert(id, tile);
        p.procedural_materials.insert(id, asset.save().unwrap());
        id
    }
    fn sample(
        p: &Project,
        id: Uuid,
        xy: [f32; 2],
        channel: denrim_materials::ir::Channel,
    ) -> Vec<f32> {
        let a = EditableMaterialAsset::reopen(&bundle(id, p, None).unwrap()).unwrap();
        let program = denrim_materials::compile(&format::write(
            &a.resolved(&InstanceRecord::new("test")).unwrap(),
        ))
        .unwrap();
        denrim_materials::cpu::Evaluator::new(&program)
            .sample(&a.selected_material, channel, xy)
            .unwrap()
            .to_vec()
    }
    #[test]
    fn geometry_layers_follow_guides_mix_over_each_background_persist_and_clear() {
        use denrim_materials::ir::Channel;
        let mut p = Project::default();
        let grass = source(&mut p, [0., 1., 0.], 0.95);
        let blue = source(&mut p, [0., 0., 1.], 0.8);
        let rock = source(&mut p, [0.4; 3], 0.35);
        let earth = source(&mut p, [1., 0., 0.], 0.91);
        let originals = p.procedural_materials.clone();
        let mut region = shared::region::Region::new();
        region.map = Map::default();
        let map = &mut region.map;
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_edge_falloff", Value::Float(0.));
        map.properties.set(
            "default_terrain_tile",
            Value::Source(PixelSource::TileId(grass)),
        );
        map.properties.set(
            "tiles",
            Value::TileOverrides([((5, 5), PixelSource::TileId(blue))].into_iter().collect()),
        );
        let v = [[-4., -4.], [4., -4.], [4., 4.], [-4., 4.]].map(|p| map.add_vertex_at(p[0], p[1]));
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        let area = map.close_polygon_manual().unwrap();
        map.clear_temp();
        let v = [[-6., 0.], [0., 0.], [6., 0.]].map(|p| map.add_vertex_at(p[0], p[1]));
        for i in 0..2 {
            let id = map.create_linedef_manual(v[i], v[i + 1]);
            let props = &mut map.find_linedef_mut(id).unwrap().properties;
            props.set(
                "terrain_material_source",
                Value::Source(PixelSource::TileId(earth)),
            );
            props.set(
                "terrain_material_verge_source",
                Value::Source(PixelSource::TileId(grass)),
            );
            props.set("terrain_material_soft_edge", Value::Float(0.2));
            props.set("terrain_material_noisy_edge", Value::Float(0.));
        }
        let props = &mut map.find_sector_mut(area).unwrap().properties;
        props.set(
            "terrain_material_source",
            Value::Source(PixelSource::TileId(rock)),
        );
        props.set("terrain_material_noisy_edge", Value::Float(0.));
        p.regions = vec![region];
        assert!(prepare_project(&mut p).is_empty());
        let outputs = terrain_material::outputs(&p.regions[0].map);
        let result = outputs[&grass];
        assert_eq!(outputs.len(), 2);
        assert_eq!(p.material_compositions[&result].layers.len(), 3);
        assert_eq!(
            p.material_compositions[&result].layers[2]
                .geometry
                .as_ref()
                .unwrap()
                .linedefs
                .len(),
            2
        );
        assert_eq!(
            sample(&p, result, [0., 0.], Channel::BaseColor),
            vec![1., 0., 0.]
        );
        assert_eq!(
            sample(&p, result, [0., 1.], Channel::BaseColor),
            vec![0., 1., 0.]
        );
        assert_eq!(
            sample(&p, result, [0., 3.], Channel::BaseColor),
            vec![0.4; 3]
        );
        assert_eq!(
            sample(&p, outputs[&blue], [5., 5.], Channel::BaseColor),
            vec![0., 0., 1.]
        );
        assert!((sample(&p, result, [0., 0.], Channel::Roughness)[0] - 0.91).abs() < 0.001);
        assert!((sample(&p, result, [0., 3.], Channel::Roughness)[0] - 0.35).abs() < 0.001);
        let revision = p.regions[0].map.changed;
        assert!(prepare_project(&mut p).is_empty());
        assert_eq!(
            p.regions[0].map.changed, revision,
            "derived outputs must stabilize"
        );
        for id in v {
            p.regions[0].map.find_vertex_mut(id).unwrap().y = 2.;
        }
        p.regions[0].map.changed += 1;
        assert!(prepare_project(&mut p).is_empty());
        assert_eq!(
            sample(&p, result, [0., 2.], Channel::BaseColor),
            vec![1., 0., 0.]
        );
        assert_eq!(
            sample(&p, result, [0., 0.], Channel::BaseColor),
            vec![0.4; 3]
        );
        let data = shared::project_io::encode_project(&p).unwrap();
        let mut reopened = shared::project_io::decode_project(&data).unwrap();
        assert_eq!(terrain_material::outputs(&reopened.regions[0].map), outputs);
        for line in &mut reopened.regions[0].map.linedefs {
            line.properties.remove("terrain_material_source");
            line.properties.remove("terrain_material_verge_source");
        }
        for sector in &mut reopened.regions[0].map.sectors {
            sector.properties.remove("terrain_material_source");
        }
        assert!(prepare_project(&mut reopened).is_empty());
        assert!(terrain_material::outputs(&reopened.regions[0].map).is_empty());
        assert!(!reopened.tiles.contains_key(&result));
        for (id, source) in originals {
            let before = EditableMaterialAsset::reopen(&source).unwrap();
            let after = EditableMaterialAsset::reopen(&reopened.procedural_materials[&id]).unwrap();
            assert_eq!(format::write(&after.root.project), format::write(&before.root.project));
        }
    }
}
