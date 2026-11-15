use eldiron_shared::{project::Project, runtime_assets::publish_visual_tiles};
use rusterix::{Assets, GeometryObject, PixelSource, Texture, Tile};
use theframework_core::prelude::{TheColor, Uuid};
use vek::Vec3;
#[test]
fn paint_wire_fields_and_terrain_publications_keep_frozen_pixel_ownership() {
    use eldiron_shared::{
        iso_paint::{IsoPaintBakedChunk, IsoPaintLayer, IsoPaintOwner, IsoPaintSelection},
        iso_paint_terrain,
    };
    use rusterix::isometric::paint::TERRAIN_PAINT_GEO;
    let mut chunk = IsoPaintBakedChunk::new(
        IsoPaintOwner::Terrain { x: 0, z: 0 },
        TERRAIN_PAINT_GEO,
        [0, 0],
    );
    chunk.color_rgba[..4].copy_from_slice(&[24, 190, 72, 192]);
    let wire = serde_json::json!({
        "owner": { "Terrain": { "x": 0, "z": 0 } },
        "paint_geo": TERRAIN_PAINT_GEO, "origin": [0, 0], "revision": 0,
        "color_rgba": chunk.color_rgba.to_vec(),
        "material_rgba": chunk.material_rgba.to_vec(),
    });
    assert_eq!(serde_json::to_value(&chunk).unwrap(), wire);
    let restored: IsoPaintBakedChunk = serde_json::from_value(wire).unwrap();
    assert_eq!(restored, chunk);
    let mut layer = IsoPaintLayer::default();
    layer.baked_chunks.insert("original".into(), chunk);
    let mut selection = IsoPaintSelection::new(IsoPaintOwner::Terrain { x: 0, z: 0 });
    selection.set_pixel(
        IsoPaintOwner::Terrain { x: 0, z: 0 },
        TERRAIN_PAINT_GEO,
        [0, 0],
    );
    layer.set_active_selection(selection);
    layer.surface_commit_strokes.push(Uuid::new_v4());
    let snapshot = layer.presentation_source(true);
    assert!(snapshot.surface_commit_strokes.is_empty());
    assert!(snapshot.baked_chunks.same_storage(&layer.baked_chunks));
    assert!(
        snapshot
            .active_selection()
            .unwrap()
            .chunks
            .same_storage(&layer.active_selection().unwrap().chunks)
    );
    let wire = serde_json::to_value(&layer).unwrap();
    assert_eq!(
        wire["baked_chunks"],
        serde_json::to_value(&*layer.baked_chunks).unwrap()
    );
    let restored: IsoPaintLayer = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(restored.baked_chunks, layer.baked_chunks);
    assert_eq!(restored.selection_masks, layer.selection_masks);
    assert_eq!(serde_json::to_value(&restored).unwrap(), wire);
    let map = Uuid::new_v4();
    let mut assets = Assets::default();
    iso_paint_terrain::sync(&mut assets, map, &mut layer);
    let frozen = assets.terrain_paint[&map].clone();
    let original = &layer.baked_chunks["original"];
    assert_eq!(
        frozen.chunks[&[0, 0]].color.as_ptr(),
        original.color_rgba.as_ptr()
    );
    assert_eq!(
        frozen.chunks[&[0, 0]].material.as_ptr(),
        original.material_rgba.as_ptr()
    );
    let changed = layer.baked_chunks.get_mut("original").unwrap();
    changed.color_rgba[..4].copy_from_slice(&[240, 20, 60, 128]);
    changed.material_rgba[..4].copy_from_slice(&[254, 1, 1, 255]);
    changed.revision += 1;
    assert!(!snapshot.baked_chunks.same_storage(&layer.baked_chunks));
    assert_eq!(
        &snapshot.baked_chunks["original"].color_rgba[..4],
        &[24, 190, 72, 192]
    );
    layer
        .selection_masks
        .get_mut(&layer.active_selection_id.unwrap())
        .unwrap()
        .chunks
        .values_mut()
        .next()
        .unwrap()
        .mask[0] = 0;
    assert_eq!(
        snapshot
            .active_selection()
            .unwrap()
            .chunks
            .values()
            .next()
            .unwrap()
            .mask[0],
        255
    );
    iso_paint_terrain::sync(&mut assets, map, &mut layer);
    assert_eq!(frozen.sample([0., 0.]).unwrap().0, [24, 190, 72, 192]);
    assert_eq!(
        assets.terrain_paint[&map].sample([0., 0.]).unwrap().0,
        [240, 20, 60, 128]
    );
    assert_eq!(frozen.sample([0., 0.]).unwrap().1, [254, 0, 0, 0]);
}
#[test]
fn cpu_visual_sources_preserve_project_data_engine_entries_and_palette_geometry_indices() {
    let mut project = Project::new();
    let authored = Tile::from_texture(Texture::new(vec![10, 40, 70, 255], 1, 1));
    let id = authored.id;
    project.tiles.insert(id, authored.clone());
    let color = TheColor::new(0.3, 0.25, 0.2, 1.);
    project.palette.colors[2] = Some(color.clone());
    let mut object = GeometryObject::box_("source fixture", Vec3::zero(), Vec3::broadcast(1.));
    object.faces[0].tile = Some(PixelSource::Color(color.clone()));
    project.regions[0].map.geometry_objects.push(object);
    let before = serde_json::to_value(&project).unwrap();
    let engine = Tile::from_texture(Texture::new(vec![200, 30, 140, 255], 1, 1));
    let engine_id = engine.id;
    let mut assets = Assets::default();
    assets.set_tiles([(engine.id, engine.clone())].into_iter().collect());
    let engine_index = assets.tile_index(&engine_id).unwrap();
    let mut palette_bound = Tile::from_texture(Texture::new(vec![80, 90, 100, 255], 1, 1));
    palette_bound.id = id;
    assets
        .palette_tile_overrides
        .insert(id, palette_bound.clone());
    publish_visual_tiles(&mut assets, &project);
    assert_eq!(assets.tile_index(&engine_id), Some(engine_index));
    assert_eq!(assets.tiles[&engine_id], engine);
    assert_eq!(assets.tiles[&id], palette_bound);
    let palette_id = Uuid::from_u128(0x50414C455454455F0000000000000000u128 | 2);
    let geometry_id = PixelSource::color_tile_uuid(&color);
    for source in [id, palette_id, geometry_id] {
        let index = assets.tile_index(&source).unwrap();
        assert_eq!(assets.tile_list[index as usize], assets.tiles[&source]);
    }
    assert_eq!(
        assets.tiles[&palette_id].textures[0].data,
        color.to_u8_array()
    );
    assert_eq!(
        assets.tiles[&geometry_id].textures[0].data,
        color.to_u8_array()
    );
    assert_eq!(
        assets.palette_material_ids,
        project
            .palette_materials
            .iter()
            .map(|m| m.material_id())
            .collect::<Vec<_>>()
    );
    let indices = [id, palette_id, geometry_id].map(|id| assets.tile_index(&id));
    let count = assets.tile_list.len();
    publish_visual_tiles(&mut assets, &project);
    assert_eq!(
        [id, palette_id, geometry_id].map(|id| assets.tile_index(&id)),
        indices
    );
    assert_eq!(assets.tile_list.len(), count);
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}
