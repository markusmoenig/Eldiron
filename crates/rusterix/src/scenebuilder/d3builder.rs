use crate::{
    Assets, Batch3D, D3Camera, Map, PixelSource, Scene, SceneHandler, Value, ValueContainer,
    avatar_builder::AvatarRuntimeBuilder,
};
use scenevm::{Atom, DynamicObject, GeoId, Light};
use uuid::Uuid;
use vek::{Vec2, Vec3};

pub struct D3Builder {}

impl Default for D3Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl D3Builder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(
        &mut self,
        _map: &Map,
        _assets: &Assets,
        _screen_size: Vec2<f32>,
        _camera_id: &str,
        _properties: &ValueContainer,
    ) -> Scene {
        Scene::default()
    }

    pub fn build_entities_items(
        &self,
        map: &Map,
        camera: &dyn D3Camera,
        assets: &Assets,
        scene: &mut Scene,
        scene_handler: &mut SceneHandler,
    ) {
        scene_handler.vm.execute(Atom::ClearDynamics);
        scene_handler.vm.execute(Atom::ClearLights);
        scene_handler.add_sector_campfire_lights(map);

        let basis = camera.basis_vectors();
        let creator_geo_id = |creator_id: Uuid, fallback: u32, is_item: bool| {
            let raw = creator_id.as_u128() as u32;
            if is_item {
                GeoId::Item(if raw == 0 { fallback } else { raw })
            } else {
                GeoId::Character(if raw == 0 { fallback } else { raw })
            }
        };

        scene.dynamic_lights = vec![];
        let mut batches = vec![];

        fn add_billboard(center: Vec3<f32>, size: f32, camera: &dyn D3Camera, batch: &mut Batch3D) {
            let (_view_forward, view_right, view_up) = camera.basis_vectors();
            batch.add_vertex_billboard(center, view_right, view_up, size);
        }

        /*
        // Billboard sectors (Rect)
        for sector in self.map.sectors.iter() {
            if sector.layer.is_some() {
                let render_mode = sector.properties.get_int_default("rect_rendering", 0);

                if let Some(source) = sector.properties.get_default_source() {
                    if render_mode == 0 {
                        // Billboard
                        let mut scale = 1.0;
                        if let PixelSource::TileId(tile_id) = source {
                            if let Some(tile) = assets.tiles.get(tile_id) {
                                scale = tile.scale;
                            }
                        }
                        if let Some(position) = sector.center(&self.map) {
                            let center3 = Vec3::new(position.x, scale * 0.5, position.y);
                            if let Some(tile) = source.tile_from_tile_list(assets) {
                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    let mut batch = Batch3D::empty()
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .source(PixelSource::StaticTileIndex(texture_index));

                                    add_billboard(center3, scale, camera, &mut batch);
                                    batches.push(batch);
                                }
                            }
                        }
                    }
                }
            }
        }*/

        // Entities
        for entity in &map.entities {
            let show_entity = true; // !(entity.is_player() && camera.id() == "firstp");

            if show_entity {
                // Find light on entity
                if let Some(Value::Light(light)) = entity.attributes.get("light") {
                    let light = light.clone();
                    scene_handler.vm.execute(Atom::AddLight {
                        id: GeoId::ItemLight(entity.id),
                        light: Light::new_pointlight(entity.position)
                            .with_color(Vec3::from(light.get_color().map(|c| c.powf(2.2)))) // Convert light to linear
                            .with_intensity(light.get_intensity())
                            .with_emitting(light.active)
                            .with_start_distance(light.get_start_distance())
                            .with_end_distance(light.get_end_distance())
                            .with_flicker(light.get_flicker()),
                    });
                }

                // Find light on entity items
                for (_, item) in entity.iter_inventory() {
                    if let Some(Value::Light(light)) = item.attributes.get("light") {
                        let light = light.clone();
                        scene_handler.vm.execute(Atom::AddLight {
                            id: GeoId::ItemLight(item.id),
                            light: Light::new_pointlight(entity.position)
                                .with_color(Vec3::from(light.get_color().map(|c| c.powf(2.2)))) // Convert light to linear
                                .with_intensity(light.get_intensity())
                                .with_emitting(light.active)
                                .with_start_distance(light.get_start_distance())
                                .with_end_distance(light.get_end_distance())
                                .with_flicker(light.get_flicker()),
                        });
                    }
                }

                if let Some(Value::Source(source)) = entity.attributes.get("source") {
                    if entity.attributes.get_bool_default("visible", false) {
                        let size = entity.attributes.get_float_default("size", 2.0).max(0.01);
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            let center3 = Vec3::new(
                                entity.position.x,
                                entity.position.y + size * 0.5,
                                entity.position.z,
                            );

                            let dynamic = DynamicObject::billboard_tile(
                                GeoId::Item(entity.id),
                                tile.id,
                                center3,
                                basis.1,
                                basis.2,
                                size,
                                size,
                            );
                            scene_handler
                                .vm
                                .execute(Atom::AddDynamic { object: dynamic });
                        }

                        let center3 = Vec3::new(
                            entity.position.x,
                            entity.position.y + size * 0.5,
                            entity.position.z,
                        );
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            if let Some(texture_index) = assets.tile_index(&tile.id) {
                                let mut batch = Batch3D::empty()
                                    .repeat_mode(crate::RepeatMode::RepeatXY)
                                    .source(PixelSource::StaticTileIndex(texture_index));

                                add_billboard(center3, size, camera, &mut batch);
                                batches.push(batch);
                            }
                        }
                    }
                } else if let Some(Value::Source(source)) = entity.attributes.get("_source_seq") {
                    if entity.attributes.get_bool_default("visible", false) {
                        let size = entity.attributes.get_float_default("size", 2.0).max(0.01);
                        let center3 = Vec3::new(
                            entity.position.x,
                            entity.position.y + size * 0.5,
                            entity.position.z,
                        );
                        if let Some(entity_tile) = source.entity_tile_id(entity.id, assets) {
                            let mut batch = Batch3D::empty()
                                .repeat_mode(crate::RepeatMode::RepeatXY)
                                .source(entity_tile);

                            add_billboard(center3, size, camera, &mut batch);
                            batches.push(batch);
                        }
                    }
                } else {
                    let size = entity.attributes.get_float_default("size", 1.0).max(0.5);
                    let center3 = Vec3::new(
                        entity.position.x,
                        entity.position.y + size * 0.5,
                        entity.position.z,
                    );
                    let icon = if Some(entity.creator_id) == map.selected_entity_item {
                        scene_handler.character_on
                    } else {
                        scene_handler.character_off
                    };
                    let dynamic = DynamicObject::billboard_tile(
                        creator_geo_id(entity.creator_id, 10_000, false),
                        icon,
                        center3,
                        basis.1,
                        basis.2,
                        size,
                        size,
                    );
                    scene_handler
                        .vm
                        .execute(Atom::AddDynamic { object: dynamic });
                }
            }
        }

        // Items
        for item in &map.items {
            let show_entity = true; // !(entity.is_player() && camera.id() == "firstp");

            if show_entity {
                if let Some(Value::Light(light)) = item.attributes.get("light") {
                    // let mut light = light.clone();
                    // light.set_position(item.position);
                    // scene.dynamic_lights.push(light.compile());
                    scene_handler.vm.execute(Atom::AddLight {
                        id: GeoId::ItemLight(item.id),
                        light: Light::new_pointlight(item.position)
                            .with_color(Vec3::from(light.get_color().map(|c| c.powf(2.2)))) // Convert light to linear
                            .with_intensity(light.get_intensity())
                            .with_emitting(light.active)
                            .with_start_distance(light.get_start_distance())
                            .with_end_distance(light.get_end_distance())
                            .with_flicker(light.get_flicker()),
                    });
                }

                if let Some(Value::Source(source)) = item.attributes.get("source") {
                    if item.attributes.get_bool_default("visible", false) {
                        let size = 1.0;
                        let pos_xz = item.get_pos_xz();
                        let mut ground_y = map
                            .geometry_floor_height_nearest(pos_xz, item.position.y)
                            .or_else(|| {
                                map.find_sector_at(pos_xz)
                                    .map(|s| s.properties.get_float_default("floor_height", 0.0))
                            })
                            .unwrap_or(0.0);
                        if ground_y == 0.0 {
                            let config =
                                crate::chunkbuilder::terrain_generator::TerrainConfig::default();
                            ground_y = crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                                map, pos_xz, &config,
                            );
                        }
                        let is_spell_like = item.attributes.get_bool_default("is_spell", false)
                            || item.attributes.get_bool_default("spell_impacting", false);
                        let y = if is_spell_like {
                            item.position.y + size * 0.5
                        } else {
                            ground_y + size * 0.5
                        };
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            let center3 = Vec3::new(item.position.x, y, item.position.z);

                            let dynamic = DynamicObject::billboard_tile(
                                creator_geo_id(item.creator_id, 20_000, true),
                                tile.id,
                                center3,
                                basis.1,
                                basis.2,
                                size,
                                size,
                            );
                            scene_handler
                                .vm
                                .execute(Atom::AddDynamic { object: dynamic });
                        }

                        let center3 = Vec3::new(item.position.x, y, item.position.z);
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            if let Some(texture_index) = assets.tile_index(&tile.id) {
                                let mut batch = Batch3D::empty()
                                    .repeat_mode(crate::RepeatMode::RepeatXY)
                                    .source(PixelSource::StaticTileIndex(texture_index));

                                add_billboard(center3, size, camera, &mut batch);
                                batches.push(batch);
                            }
                        }
                    }
                } else if item.attributes.get_bool_default("visible", false)
                    && let Some(tile) = AvatarRuntimeBuilder::explicit_item_tile(item, assets)
                {
                    let size = 1.0;
                    let pos_xz = item.get_pos_xz();
                    let mut ground_y = map
                        .geometry_floor_height_nearest(pos_xz, item.position.y)
                        .or_else(|| {
                            map.find_sector_at(pos_xz)
                                .map(|s| s.properties.get_float_default("floor_height", 0.0))
                        })
                        .unwrap_or(0.0);
                    if ground_y == 0.0 {
                        let config =
                            crate::chunkbuilder::terrain_generator::TerrainConfig::default();
                        ground_y = crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                            map, pos_xz, &config,
                        );
                    }
                    let center3 =
                        Vec3::new(item.position.x, ground_y + size * 0.5, item.position.z);
                    let dynamic = DynamicObject::billboard_tile(
                        creator_geo_id(item.creator_id, 20_000, true),
                        tile.id,
                        center3,
                        basis.1,
                        basis.2,
                        size,
                        size,
                    );
                    scene_handler
                        .vm
                        .execute(Atom::AddDynamic { object: dynamic });
                } else if let Some(Value::Source(source)) = item.attributes.get("_source_seq") {
                    if item.attributes.get_bool_default("visible", false) {
                        let size = 2.0;
                        let center3 = Vec3::new(
                            item.position.x,
                            item.position.y + size * 0.5,
                            item.position.z,
                        );
                        if let Some(item_tile) = source.item_tile_id(item.id, assets) {
                            let mut batch = Batch3D::empty()
                                .repeat_mode(crate::RepeatMode::RepeatXY)
                                .source(item_tile);

                            add_billboard(center3, size, camera, &mut batch);
                            batches.push(batch);
                        }
                    }
                } else {
                    let size = 1.0;
                    let pos_xz = item.get_pos_xz();
                    let mut ground_y = map
                        .geometry_floor_height_nearest(pos_xz, item.position.y)
                        .or_else(|| {
                            map.find_sector_at(pos_xz)
                                .map(|s| s.properties.get_float_default("floor_height", 0.0))
                        })
                        .unwrap_or(0.0);
                    if ground_y == 0.0 {
                        let config =
                            crate::chunkbuilder::terrain_generator::TerrainConfig::default();
                        ground_y = crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                            map, pos_xz, &config,
                        );
                    }
                    let center3 =
                        Vec3::new(item.position.x, ground_y + size * 0.5, item.position.z);
                    let icon = if Some(item.creator_id) == map.selected_entity_item {
                        scene_handler.item_on
                    } else {
                        scene_handler.item_off
                    };
                    let dynamic = DynamicObject::billboard_tile(
                        creator_geo_id(item.creator_id, 20_000, true),
                        icon,
                        center3,
                        basis.1,
                        basis.2,
                        size,
                        size,
                    );
                    scene_handler
                        .vm
                        .execute(Atom::AddDynamic { object: dynamic });
                }
            }
        }

        // Vertices with billboards
        for vertex in &map.vertices {
            if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                vertex.properties.get("source")
            {
                let size = vertex.properties.get_float_default("source_size", 1.0);
                let center3 = Vec3::new(vertex.x, vertex.z + size * 0.5, vertex.y);

                let dynamic = DynamicObject::billboard_tile(
                    GeoId::Vertex(vertex.id),
                    *tile_id,
                    center3,
                    basis.1,
                    basis.2,
                    size,
                    size,
                );
                scene_handler
                    .vm
                    .execute(Atom::AddDynamic { object: dynamic });
            }
        }

        // Billboards (doors/gates)
        for (geo_id, billboard) in &scene_handler.billboards {
            // TODO: Query server/client for current state of this GeoId
            // For now, always render billboards (you can add state checking later)
            let is_visible = true;

            if is_visible {
                // Calculate animation offset based on animation type and state
                // For now, render at static position (you can add animation interpolation later)
                let animated_center = billboard.center;

                let dynamic = DynamicObject::billboard_tile(
                    *geo_id,
                    billboard.tile_id,
                    animated_center,
                    billboard.up,
                    billboard.right,
                    billboard.size,
                    billboard.size,
                )
                .with_repeat_mode(billboard.repeat_mode);
                scene_handler
                    .vm
                    .execute(Atom::AddDynamic { object: dynamic });
            }
        }

        scene.d3_dynamic = batches;
        scene.dynamic_textures = vec![];
        scene.compute_dynamic_normals();
    }
}
