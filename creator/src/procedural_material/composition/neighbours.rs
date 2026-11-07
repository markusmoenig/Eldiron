//! Derive a continuous road field from authored placements. No neighbour state
//! is saved: painting, erasing, copying and undo all regenerate the same field.
use super::*;
use rusterix::{PixelSource, ValueContainer};
use shared::material_composition::CompositionSurface;
use std::collections::BTreeSet;

fn painted(props: &ValueContainer, id: Uuid) -> Vec<[i32; 2]> {
    let mut cells = BTreeSet::new();
    for value in props.values() {
        match value {
            Value::TileOverrides(tiles) => cells.extend(
                tiles
                    .iter()
                    .filter(|(_, s)| matches!(s,PixelSource::TileId(tile) if *tile==id))
                    .map(|(&(x, y), _)| [x, y]),
            ),
            Value::BlendOverrides(tiles) => cells.extend(
                tiles
                    .iter()
                    .filter(|(_, (_, s))| matches!(s,PixelSource::TileId(tile) if *tile==id))
                    .map(|(&(x, y), _)| [x, y]),
            ),
            _ => (),
        }
    }
    cells.into_iter().collect()
}
/// Picker presets acquire their context from actual placements. Prefer a live
/// placement to the original owner, which may have been erased or repainted.
pub(super) fn binding(
    project: &Project,
    id: Uuid,
) -> Result<Option<shared::material_composition::MapMaterialBinding>, String> {
    use shared::material_composition::MapMaterialBinding;
    let mut contexts = std::collections::BTreeMap::new();
    for region in &project.regions {
        let map = &region.map;
        let mut add = |scope: String, surface| {
            contexts
                .entry((map.id, scope))
                .or_insert(MapMaterialBinding {
                    map: map.id,
                    surface,
                });
        };
        for sector in &map.sectors {
            if matches!(sector.properties.get_default_source(),Some(PixelSource::TileId(tile)) if *tile==id)
            {
                let b = sector.bounding_box(map);
                if (b.max.x - b.min.x - 1.).abs() < 0.001 && (b.max.y - b.min.y - 1.).abs() < 0.001
                {
                    add(
                        "floor".into(),
                        CompositionSurface::RectTile { sector: sector.id },
                    );
                } else {
                    add(
                        format!("sector:{}", sector.id),
                        CompositionSurface::Sector {
                            id: sector.id,
                            ceiling: false,
                        },
                    );
                }
            }
            if let Some(cell) = painted(&sector.properties, id).first() {
                add(
                    format!("sector:{}", sector.id),
                    CompositionSurface::SectorTile {
                        sector: sector.id,
                        cell: *cell,
                    },
                );
            }
        }
        if let Some(cell) = painted(&map.properties, id).first() {
            add(
                "terrain".into(),
                CompositionSurface::TerrainTile { cell: *cell },
            );
        }
        for object in &map.geometry_objects {
            for (face, surface) in object.faces.iter().enumerate() {
                if let Some((&(x, y), _)) = surface
                    .tiles
                    .iter()
                    .find(|(_, s)| matches!(s,PixelSource::TileId(tile) if *tile==id))
                {
                    add(
                        format!("geometry:{}:{face}", object.id),
                        CompositionSurface::GeometryTile {
                            object: object.id,
                            face,
                            cell: [x, y],
                        },
                    );
                }
            }
        }
    }
    if contexts.len() > 1 {
        return Err("This auto-road recipe is used on multiple map grids. Save a separate composition copy for each grid.".into());
    }
    Ok(contexts
        .into_values()
        .next()
        .or_else(|| project.map_material_bindings.get(&id).cloned()))
}

fn matches(project: &Project, source: &PixelSource, material: Uuid) -> bool {
    let PixelSource::TileId(id) = source else {
        return false;
    };
    *id == material
        || project.material_compositions.get(id).is_some_and(|c| {
            c.layers
                .iter()
                .any(|l| l.material == material && l.shape == CompositionShape::AutoRoad)
        })
}
fn overrides(
    project: &Project,
    props: &ValueContainer,
    material: Uuid,
    cells: &mut BTreeSet<[i32; 2]>,
) {
    for value in props.values() {
        match value {
            Value::TileOverrides(tiles) => {
                for (&(x, y), source) in tiles {
                    if matches(project, source, material) {
                        cells.insert([x, y]);
                    }
                }
            }
            Value::BlendOverrides(tiles) => {
                for (&(x, y), (_, source)) in tiles {
                    if matches(project, source, material) {
                        cells.insert([x, y]);
                    }
                }
            }
            _ => (),
        }
    }
}
pub(super) fn distance(
    project: &Project,
    id: Uuid,
    layer: &CompositionLayer,
) -> Result<Option<String>, String> {
    if layer.shape != CompositionShape::AutoRoad {
        return Ok(None);
    }
    let Some(binding) = binding(project, id)? else {
        return Ok(None);
    };
    let Some(map) = project
        .regions
        .iter()
        .find(|r| r.map.id == binding.map)
        .map(|r| &r.map)
    else {
        return Ok(None);
    };
    let mut cells = BTreeSet::new();
    match binding.surface {
        CompositionSurface::GeometryTile { object, face, cell } => {
            if let Some(face) = map.geometry_objects.iter().find(|o| o.id == object).and_then(|o| o.faces.get(face)) {
                for (&(x,y), source) in &face.tiles {
                    if matches(project, source, layer.material) { cells.insert([x,y]); }
                }
            }
            cells.insert(cell);
        }
        CompositionSurface::TerrainTile { cell } => {
            overrides(project, &map.properties, layer.material, &mut cells);
            cells.insert(cell);
        }
        CompositionSurface::SectorTile { sector, cell } => {
            if let Some(sector) = map.find_sector(sector) { overrides(project, &sector.properties, layer.material, &mut cells); }
            cells.insert(cell);
        }
        CompositionSurface::RectTile { .. } => {
            for sector in &map.sectors {
                if sector.properties.get_default_source().is_some_and(|s| matches(project, s, layer.material)) {
                    let b = sector.bounding_box(map);
                    if (b.max.x-b.min.x-1.).abs() < 0.001 && (b.max.y-b.min.y-1.).abs() < 0.001 {
                        cells.insert([b.min.x.round() as i32, b.min.y.round() as i32]);
                    }
                }
            }
            let origin = super::surface_origin(project, &binding);
            cells.insert(origin.map(|v| v.round() as i32));
        }
        _ => return Err("Auto road needs painted cells. Use the Rect tool to compose individual floor, terrain or wall cells.".into()),
    }
    // Baking remains bounded. This limit is about a connected authoring field,
    // not the number of textures or material instances in the runtime.
    if cells.len() > 256 {
        return Err("Auto road currently supports 256 cells per surface. Split a larger road into separate surfaces.".into());
    }
    Ok(Some(field(&cells, layer.position / 2.)))
}
fn field(cells: &BTreeSet<[i32; 2]>, radius: f32) -> String {
    let mut terms = Vec::new();
    for &[x, y] in cells {
        let center = [x as f32 + 0.5, y as f32 + 0.5];
        // One segment per undirected edge; isolated cells retain rounded ends.
        let mut joined = false;
        for axis in 0..2 {
            let mut neighbour = [x, y];
            neighbour[axis] += 1;
            if cells.contains(&neighbour) {
                joined = true;
                let mut min = center;
                let mut max = center;
                max[axis] += 1.;
                // Distance to the axis-aligned segment gives rounded corners
                // and exactly identical values on both sides of a shared edge.
                min[1 - axis] = center[1 - axis];
                terms.push(format!(
                    "length(uv - clamp(uv, vec2({}, {}), vec2({}, {})))",
                    min[0], min[1], max[0], max[1]
                ));
            }
        }
        if !joined && !(cells.contains(&[x - 1, y]) || cells.contains(&[x, y - 1])) {
            terms.push(format!("length(uv - vec2({}, {}))", center[0], center[1]));
        }
    }
    if terms.is_empty() {
        terms.push("1000".into());
    }
    while terms.len() > 1 {
        terms = terms
            .chunks(2)
            .map(|p| {
                if p.len() == 2 {
                    format!("min({}, {})", p[0], p[1])
                } else {
                    p[0].clone()
                }
            })
            .collect();
    }
    format!("{radius} - ({})", terms[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auto_roads_generate_one_edge_per_pair_and_stable_end_caps() {
        let cells = [[-1, 0], [0, 0], [0, 1], [1, 0]].into_iter().collect();
        let code = field(&cells, 0.2);
        assert_eq!(code.matches("clamp(").count(), 3);
        assert!(code.contains("vec2(-0.5, 0.5)"));
        assert_eq!(
            field(&[[0, 0]].into_iter().collect(), 0.2),
            "0.2 - (length(uv - vec2(0.5, 0.5)))"
        );
    }
    fn fixture() -> (Project, Uuid) {
        use shared::material_composition::{CompositionTiling, MapMaterialBinding};
        let mut p = Project::default();
        let mut tile =
            rusterix::Tile::from_textures(vec![rusterix::Texture::new(vec![0, 0, 0, 255], 1, 1)]);
        let base = tile.id;
        p.tiles.insert(base, tile.clone());
        tile.id = Uuid::new_v4();
        tile.textures[0].data = vec![255; 4];
        let road = tile.id;
        p.tiles.insert(road, tile.clone());
        tile.id = Uuid::new_v4();
        let id = tile.id;
        p.tiles.insert(id, tile);
        let mut layer = CompositionLayer::split(road);
        layer.shape = CompositionShape::AutoRoad;
        layer.position = 0.4;
        layer.feather = 0.1;
        layer.wobble = 0.02;
        let authored = MaterialComposition {
            base,
            layers: vec![layer.clone()],
            tiling: CompositionTiling::default(),
        };
        p.material_compositions.insert(id, authored.clone());
        let mut region = Region::new();
        let map = &mut region.map;
        for (i, cell) in [[0, 0], [1, 0], [1, 1]].into_iter().enumerate() {
            super::super::tests::add_floor(
                map,
                i as u32,
                id,
                cell.map(|v| v as f32),
                cell.map(|v| v as f32 + 1.),
            );
        }
        p.map_material_bindings.insert(
            id,
            MapMaterialBinding {
                map: map.id,
                surface: CompositionSurface::RectTile { sector: 0 },
            },
        );
        p.regions.push(region);
        (p, id)
    }
    #[test]
    fn auto_road_map_edits_join_bend_and_erase_without_changing_authored_layers() {
        let (mut p, id) = fixture();
        let authored = p.material_compositions[&id].clone();
        let first = super::super::bundle(id, &p, None).unwrap();
        let asset = EditableMaterialAsset::reopen(&first).unwrap();
        let pages = super::super::surface_pages::decode(&asset)
            .unwrap()
            .expect("Copied auto-road placements need world pages, not one repeated owner tile");
        assert_eq!(pages.size, [2, 2]);
        assert_eq!(pages.units_per_page, 1);
        let program = asset
            .program(&denrim_materials::integration::InstanceRecord::new(
                "auto-road-test",
            ))
            .unwrap();
        let e = denrim_materials::cpu::Evaluator::new(&program);
        let sample = |uv| {
            e.sample(
                &asset.selected_material,
                denrim_materials::ir::Channel::BaseColor,
                uv,
            )
            .unwrap()[0]
        };
        assert!(sample([0.9999, 0.5]) > 0.9 && sample([1.0001, 0.5]) > 0.9);
        assert!(sample([1.5, 0.9999]) > 0.9 && sample([1.5, 1.0001]) > 0.9);
        assert!(sample([0.1, 0.1]) < 0.1);
        p.regions.last_mut().unwrap().map.sectors.pop();
        let after = super::super::bundle(id, &p, None).unwrap();
        assert_ne!(
            first, after,
            "Erasing a neighbour changes the derived road field"
        );
        assert_eq!(p.material_compositions[&id], authored);
    }
    #[test]
    fn auto_road_picker_copies_infer_the_grid_and_erased_owners_leave_no_ghost_arm() {
        let (mut p, id) = fixture();
        // Saved picker copies have no map binding; painting supplies context.
        p.map_material_bindings.shift_remove(&id);
        let first = super::super::bundle(id, &p, None).unwrap();
        assert!(
            EditableMaterialAsset::reopen(&first)
                .unwrap()
                .instances
                .iter()
                .any(|i| i.id == super::super::SURFACE_DOMAIN)
        );
        let map = &mut p.regions.last_mut().unwrap().map;
        let stale = shared::material_composition::MapMaterialBinding {
            map: map.id,
            surface: CompositionSurface::RectTile { sector: 0 },
        };
        map.sectors.remove(0);
        p.map_material_bindings.insert(id, stale);
        let asset =
            EditableMaterialAsset::reopen(&super::super::bundle(id, &p, None).unwrap()).unwrap();
        let program = asset
            .program(&denrim_materials::integration::InstanceRecord::new(
                "erased-owner",
            ))
            .unwrap();
        let e = denrim_materials::cpu::Evaluator::new(&program);
        assert!(
            e.sample(
                &asset.selected_material,
                denrim_materials::ir::Channel::BaseColor,
                [0.5, 0.5]
            )
            .unwrap()[0]
                < 0.1,
            "Erased owner must not keep a phantom road arm"
        );
        let mut other = p.regions.last().unwrap().clone();
        other.map.id = Uuid::new_v4();
        p.regions.push(other);
        assert!(
            super::super::bundle(id, &p, None)
                .unwrap_err()
                .contains("multiple map grids")
        );
    }
    #[test]
    #[ignore = "native GPU automatic-road bake and minimap preview"]
    fn auto_road_gpu_bakes_distinct_connected_world_pages() {
        use crate::procedural_material::gpu;
        let (p, id) = fixture();
        let bundle = super::super::bundle(id, &p, None).unwrap();
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let worker = gpu::Worker::new(device, queue);
        worker.send(gpu::Job {
            pixels_per_tile: 16,
            tile: id,
            revision: 1,
            bundle,
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let frame = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(60))
            .unwrap()
            .frame
            .unwrap();
        let pages = frame.surface_pages.unwrap();
        assert_eq!(pages.layout.size, [2, 2]);
        assert_eq!(pages.textures.len(), 4);
        assert_ne!(pages.textures[0].data, pages.textures[1].data);
        assert_ne!(pages.textures[1].data, pages.textures[3].data);
        assert!(
            pages.textures[2].data.chunks_exact(4).all(|p| p[0] < 60),
            "The unused cell must remain the background"
        );
        assert!(
            frame.images[0].chunks_exact(4).any(|p| p[0] > 128),
            "The minimap preview must show the owner road segment"
        );
    }
}
