use rusterix::{
    Assets, Map, PixelSource, Texture, Tile, Value, Vertex, VertexBlendPreset,
    isometric::{SpriteLayout, materials::SurfaceMaterials},
};
use theframework_core::prelude::Uuid;
fn tile(color: [u8; 4]) -> Tile {
    let mut tile = Tile::from_texture(Texture::from_color(color));
    tile.isometric = Some(SpriteLayout {
        ground: true,
        pixels_per_unit: 32.,
        ..Default::default()
    });
    tile
}
fn sector(map: &mut Map, source: Uuid) -> u32 {
    for (i, p) in [[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
        .into_iter()
        .enumerate()
    {
        map.vertices.push(Vertex::new(i as u32, p[0], p[1]));
    }
    for i in 0..4 {
        map.create_linedef_manual(i, (i + 1) % 4);
    }
    let id = map.close_polygon_manual().unwrap();
    map.find_sector_mut(id)
        .unwrap()
        .properties
        .set("source", Value::Source(PixelSource::TileId(source)));
    id
}
fn get(assets: &Assets, id: Uuid) -> &Tile {
    &assets.tile_list[assets.tile_index(&id).unwrap() as usize]
}
#[test]
fn floor_sources_share_world_sector_precedence_and_keep_raised_overrides_local() {
    let tiles = [
        [20, 40, 160, 255],
        [180, 30, 20, 255],
        [20, 160, 40, 255],
        [190, 170, 20, 255],
    ]
    .map(tile);
    let ids = tiles.each_ref().map(|t| t.id);
    let mut assets = Assets::default();
    assets.set_tiles(tiles.into_iter().map(|t| (t.id, t)).collect());
    let mut map = Map::default();
    let id = sector(&mut map, ids[0]);
    map.properties.set(
        "tiles",
        Value::TileOverrides(
            [
                ((0, 0), PixelSource::TileId(ids[1])),
                ((-1, -1), PixelSource::TileId(ids[1])),
            ]
            .into_iter()
            .collect(),
        ),
    );
    map.properties.set(
        "blend_tiles",
        Value::BlendOverrides(
            [(
                (0, 0),
                (VertexBlendPreset::Left, PixelSource::TileId(ids[1])),
            )]
            .into_iter()
            .collect(),
        ),
    );
    let floor = map.find_sector_mut(id).unwrap();
    floor.properties.set(
        "tiles",
        Value::TileOverrides(
            [((0, 0), PixelSource::TileId(ids[2]))]
                .into_iter()
                .collect(),
        ),
    );
    floor.properties.set(
        "blend_tiles",
        Value::BlendOverrides(
            [(
                (0, 0),
                (VertexBlendPreset::Right, PixelSource::TileId(ids[3])),
            )]
            .into_iter()
            .collect(),
        ),
    );
    let before = serde_json::to_value(&map).unwrap();
    let policy = SurfaceMaterials::for_floor(&map, &map.sectors[0], get(&assets, ids[0]), &assets);
    let selected = policy.cell((0, 0)).unwrap();
    assert!(std::ptr::eq(selected.primary, get(&assets, ids[2])));
    let (weights, secondary) = selected.secondary.unwrap();
    assert_eq!(weights, VertexBlendPreset::Right.weights());
    assert_eq!(secondary.id, ids[3]);
    assert_eq!(policy.cell((-1, -1)).unwrap().primary.id, ids[1]);
    assert_eq!(policy.cell((2, 2)).unwrap().primary.id, ids[0]);
    assert_eq!(serde_json::to_value(&map).unwrap(), before);
    map.sectors[0]
        .properties
        .set("floor_height", Value::Float(1.));
    let raised = SurfaceMaterials::for_floor(&map, &map.sectors[0], get(&assets, ids[0]), &assets);
    assert_eq!(raised.cell((-1, -1)).unwrap().primary.id, ids[0]);
    assert_eq!(raised.cell((0, 0)).unwrap().primary.id, ids[2]);
}
#[test]
fn terrain_sources_resolve_derived_outputs_and_fall_back_when_bakes_are_absent() {
    use rusterix::chunkbuilder::terrain_material;
    let mut map = Map::default();
    let base = tile([20, 40, 160, 255]);
    let ground = tile([20, 160, 40, 255]);
    let paint = tile([180, 30, 20, 255]);
    let blend = tile([190, 170, 20, 255]);
    let mut baked = tile([100, 60, 30, 255]);
    baked.id = terrain_material::output_id(map.id, paint.id);
    let ids = [base.id, ground.id, paint.id, blend.id, baked.id];
    sector(&mut map, ground.id);
    map.properties.set(
        "default_terrain_tile",
        Value::Source(PixelSource::TileId(base.id)),
    );
    map.properties.set(
        "tiles",
        Value::TileOverrides(
            [((0, 0), PixelSource::TileId(paint.id))]
                .into_iter()
                .collect(),
        ),
    );
    map.properties.set(
        "blend_tiles",
        Value::BlendOverrides(
            [(
                (0, 0),
                (VertexBlendPreset::Left, PixelSource::TileId(blend.id)),
            )]
            .into_iter()
            .collect(),
        ),
    );
    map.properties.set(
        terrain_material::OUTPUTS,
        Value::Str(
            serde_json::to_string(
                &[(paint.id, baked.id)]
                    .into_iter()
                    .collect::<terrain_material::Outputs>(),
            )
            .unwrap(),
        ),
    );
    let mut assets = Assets::default();
    assets.set_tiles(
        [base, ground, paint, blend, baked]
            .into_iter()
            .map(|t| (t.id, t))
            .collect(),
    );
    let selected = SurfaceMaterials::for_terrain(&map, &assets)
        .cell((0, 0))
        .unwrap();
    assert_eq!(selected.primary.id, ids[4]);
    assert_eq!(selected.secondary.unwrap().1.id, ids[3]);
    assert_eq!(
        SurfaceMaterials::for_terrain(&map, &assets)
            .cell((3, 3))
            .unwrap()
            .primary
            .id,
        ids[0]
    );
    let mut tiles = assets.tiles.clone();
    tiles.shift_remove(&ids[4]);
    assets.set_tiles(tiles);
    assert_eq!(
        SurfaceMaterials::for_terrain(&map, &assets)
            .cell((0, 0))
            .unwrap()
            .primary
            .id,
        ids[2]
    );
    map.properties.remove("tiles");
    assert_eq!(
        SurfaceMaterials::for_terrain(&map, &assets)
            .cell((0, 0))
            .unwrap()
            .primary
            .id,
        ids[1]
    );
    map.properties.remove("default_terrain_tile");
    assert!(
        SurfaceMaterials::for_terrain(&map, &assets)
            .cell((3, 3))
            .is_none()
    );
}
#[test]
fn source_texels_preserve_persisted_bake_precedence_without_mutating_serialized_assets() {
    let mut raw = Texture::new([80, 110, 150, 255].repeat(4), 2, 2);
    raw.set_materials_all(0.2, 0.4, 0.8, 0.6);
    let mut baked = raw.clone();
    baked.set_materials_all(0.8, 0.2, 1., 0.1);
    let mut tile = Tile::from_texture(raw.clone());
    tile.baked_material_data = vec![baked.material_bytes()];
    let serialized = serde_json::to_value(&tile).unwrap();
    let restored: Tile = serde_json::from_value(serialized.clone()).unwrap();
    let (r, m, o, e) = baked.get_materials(0, 0);
    assert!(
        restored.source_surface_texels(0).unwrap()
            .eq([([80, 110, 150, 255], [r, m, o, e]); 4])
    );
    for y in 0..2 {
        for x in 0..2 {
            let source = restored.source_surface_texel(0, x, y).unwrap();
            assert_eq!(source.0, [80, 110, 150, 255]);
            assert_eq!(source.1, [r, m, o, e]);
        }
    }
    assert_eq!(serde_json::to_value(&restored).unwrap(), serialized);
    tile.material.preset = "stone".into();
    tile.material.finish = "wet".into();
    let expected = tile
        .material
        .rmoe_values_for_color([80, 110, 150, 255])
        .unwrap();
    assert_eq!(
        tile.source_surface_texel(0, 0, 0).unwrap().1,
        expected,
        "authored material preset takes precedence over baked/raw channels"
    );
    assert!(
        tile.source_surface_texels(0).unwrap()
            .eq([([80, 110, 150, 255], expected); 4])
    );
    tile.material.preset = "default".into();
    tile.material.finish = "natural".into();
    tile.baked_material_data.clear();
    let (r, m, o, e) = raw.get_materials(0, 0);
    assert_eq!(tile.source_surface_texel(0, 0, 0).unwrap().1, [r, m, o, e]);
    tile.textures[0].data_ext = None;
    assert_eq!(
        tile.source_surface_texel(0, 0, 0).unwrap().1,
        [0.95, 0., 1., 0.]
    );
    assert!(tile.source_surface_texel(1, 0, 0).is_none());
    assert!(tile.source_surface_texel(0, 2, 0).is_none());
    assert!(tile.source_surface_texel(0, 0, 2).is_none());
    tile.textures[0].data.truncate(3);
    assert!(tile.source_surface_texel(0, 0, 0).is_none());
    assert!(tile.source_surface_texels(0).is_none());
}

#[test]
fn scene_material_clock_keeps_animation_separate_from_spatial_pages() {
    use rusterix::isometric::{SceneCache, materials::animation_frame};
    let map = Map::default();
    let assets = Assets::default();
    let avatars = std::collections::HashMap::new();
    let mut cache = SceneCache::default();
    for (seconds, expected) in [
        (-1., 0),
        (f32::NAN, 0),
        (0., 0),
        (0.249, 0),
        (0.25, 1),
        (0.5, 2),
        (1.25, 5),
    ] {
        assert_eq!(animation_frame(seconds), expected);
        assert_eq!(
            cache
                .prepare(&map, &assets, seconds, &avatars)
                .material_frame,
            expected
        );
    }
}

#[test]
fn source_sampling_boundaries_own_signed_cells_and_half_paint_texels() {
    use rusterix::isometric::materials::sampling_coordinate as sample;
    for n in -192..=192 {
        let boundary = n as f32 / 64.;
        for delta in [-0.0000005, 0., 0.0000005] {
            assert_eq!(sample(boundary + delta), boundary);
        }
        for delta in [-0.00001, 0.00001] {
            assert_eq!(sample(boundary + delta), boundary + delta);
        }
    }
}
