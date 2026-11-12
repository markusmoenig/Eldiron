use eldiron_shared::{project::Project, runtime_assets::publish_visual_tiles};
use rusterix::{Assets, GeometryObject, PixelSource, Texture, Tile};
use theframework::prelude::{TheColor, Uuid};
use vek::Vec3;
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
