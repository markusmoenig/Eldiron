use crate::{Assets, Batch2D, Map, PixelSource, Scene, Value};
use theframework::prelude::*;
use uuid::Uuid;
use vek::Vec2;

pub struct D2Builder {
    pub activated_widgets: Vec<u32>,
}

impl Default for D2Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl D2Builder {
    pub fn new() -> Self {
        Self {
            activated_widgets: vec![],
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets, screen_size: Vec2<f32>) -> Scene {
        let mut scene = Scene::empty();

        // Sort sectors by layer (ascending), then rect before non-rect within same layer
        let mut sorted_sectors: Vec<_> = map.sectors.iter().collect();
        sorted_sectors.sort_by(|a, b| {
            let layer_a = a.properties.get_int_default("layer", 0);
            let layer_b = b.properties.get_int_default("layer", 0);
            let is_rect_a = a.properties.contains("rect");
            let is_rect_b = b.properties.contains("rect");

            match layer_a.cmp(&layer_b) {
                std::cmp::Ordering::Equal => {
                    // Within same layer, rect sectors come first (true > false when reversed)
                    is_rect_b.cmp(&is_rect_a)
                }
                other => other,
            }
        });

        for sector in sorted_sectors {
            if let Some(geo) = sector.generate_geometry(map) {
                let mut vertices: Vec<[f32; 2]> = vec![];
                let mut uvs: Vec<[f32; 2]> = vec![];
                let bbox = sector.bounding_box(map);

                let mut repeat = true;
                if sector.properties.get_int_default("tile_mode", 1) == 0 {
                    repeat = false;
                }

                // Use the floor or ceiling source
                let mut source = sector.properties.get_default_source();
                if
                /*source.is_none() ||*/
                self.activated_widgets.contains(&sector.id) {
                    source = sector.properties.get_source("ceiling_source");
                }

                if let Some(pixelsource) = source {
                    if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                        for vertex in &geo.0 {
                            let local = self.map_grid_to_local(
                                screen_size,
                                Vec2::new(vertex[0], vertex[1]),
                                map,
                            );

                            if !repeat {
                                let uv = [
                                    (vertex[0] - bbox.min.x) / (bbox.max.x - bbox.min.x),
                                    (vertex[1] - bbox.min.y) / (bbox.max.y - bbox.min.y),
                                ];
                                uvs.push(uv);
                            } else {
                                let texture_scale = 1.0;
                                let uv = [
                                    (vertex[0] - bbox.min.x) / texture_scale,
                                    (vertex[1] - bbox.min.y) / texture_scale,
                                ];
                                uvs.push(uv);
                            }
                            vertices.push([local.x, local.y]);
                        }

                        if let Some(texture_index) = assets.tile_index(&tile.id) {
                            let batch = Batch2D::new(vertices, geo.1, uvs)
                                .repeat_mode(if repeat {
                                    crate::RepeatMode::RepeatXY
                                } else {
                                    crate::RepeatMode::ClampXY
                                })
                                .source(PixelSource::StaticTileIndex(texture_index));
                            scene.d2_static.push(batch);
                        }
                    }
                }
            }
        }

        // Walls
        for sector in &map.sectors {
            if let Some(hash) = sector.generate_wall_geometry_by_linedef(map) {
                for (linedef_id, geo) in hash.iter() {
                    let mut source = None;

                    if let Some(linedef) = map.find_linedef(*linedef_id) {
                        if let Some(Value::Source(pixelsource)) =
                            linedef.properties.get("row1_source")
                        {
                            source = Some(pixelsource);
                        }
                    }

                    let mut vertices: Vec<[f32; 2]> = vec![];
                    let mut uvs: Vec<[f32; 2]> = vec![];
                    let bbox = sector.bounding_box(map);

                    let repeat = true;

                    if let Some(pixelsource) = source {
                        if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                            for vertex in &geo.0 {
                                let local = self.map_grid_to_local(
                                    screen_size,
                                    Vec2::new(vertex[0], vertex[1]),
                                    map,
                                );

                                if !repeat {
                                    let uv = [
                                        (vertex[0] - bbox.min.x) / (bbox.max.x - bbox.min.x),
                                        (vertex[1] - bbox.min.y) / (bbox.max.y - bbox.min.y),
                                    ];
                                    uvs.push(uv);
                                } else {
                                    let texture_scale = 1.0;
                                    let uv = [
                                        (vertex[0] - bbox.min.x) / texture_scale,
                                        (vertex[1] - bbox.min.y) / texture_scale,
                                    ];
                                    uvs.push(uv);
                                }
                                vertices.push([local.x, local.y]);
                            }

                            if let Some(texture_index) = assets.tile_index(&tile.id) {
                                let batch = Batch2D::new(vertices, geo.1.clone(), uvs)
                                    .repeat_mode(if repeat {
                                        crate::RepeatMode::RepeatXY
                                    } else {
                                        crate::RepeatMode::ClampXY
                                    })
                                    .source(PixelSource::StaticTileIndex(texture_index));
                                scene.d2_static.push(batch);
                            }
                        }
                    }
                }
            }
        }

        // Add standalone walls
        for linedef in &map.linedefs {
            if linedef.sector_ids.is_empty()
                && linedef.properties.get_float_default("wall_width", 0.0) > 0.0
            {
                if let Some(hash) =
                    crate::map::geometry::generate_line_segments_d2(map, &[linedef.id])
                {
                    for (_linedef_id, geo) in hash.iter() {
                        let mut source = None;

                        if let Some(Value::Source(pixelsource)) =
                            linedef.properties.get("row1_source")
                        {
                            source = Some(pixelsource);
                        }

                        let mut vertices: Vec<[f32; 2]> = vec![];
                        let mut uvs: Vec<[f32; 2]> = vec![];

                        if let Some(pixelsource) = source {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    for vertex in &geo.0 {
                                        let local = self.map_grid_to_local(
                                            screen_size,
                                            Vec2::new(vertex[0], vertex[1]),
                                            map,
                                        );

                                        let texture_scale = 1.0;
                                        let uv = [
                                            (vertex[0]) / texture_scale,
                                            (vertex[1]) / texture_scale,
                                        ];
                                        uvs.push(uv);
                                        vertices.push([local.x, local.y]);
                                    }

                                    let batch = Batch2D::new(vertices, geo.1.clone(), uvs)
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .source(PixelSource::StaticTileIndex(texture_index));
                                    scene.d2_static.push(batch);
                                }
                            }
                        }
                    }
                }
            }
        }

        let tiles = assets.blocking_tiles();
        scene.mapmini = map.as_mini(&tiles);
        scene
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_entities_items(
        &self,
        map: &Map,
        assets: &Assets,
        scene: &mut Scene,
        screen_size: Vec2<f32>,
    ) {
        scene.dynamic_lights = vec![];

        let mut repeated_batches: Vec<Batch2D> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();

        let mut textures = vec![];

        // Items
        for item in &map.items {
            let item_pos = Vec2::new(item.position.x, item.position.z);
            let pos = self.map_grid_to_local(screen_size, Vec2::new(item_pos.x, item_pos.y), map);
            let size = 1.0;
            let hsize = 0.5;

            if let Some(Value::Light(light)) = item.attributes.get("light") {
                if light.active {
                    let mut light = light.clone();
                    light.set_position(item.position);
                    scene.dynamic_lights.push(light.compile());
                }
            }

            if let Some(Value::Source(source)) = item.attributes.get("source") {
                if item.attributes.get_bool_default("visible", false) {
                    if let Some(tile) = source.tile_from_tile_list(assets) {
                        if let Some(texture_index) = assets.tile_index(&tile.id) {
                            let mut batch = Batch2D::empty()
                                .source(PixelSource::StaticTileIndex(texture_index))
                                .receives_light(true);

                            batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                            textures.push(tile.clone());
                            repeated_offsets.insert(tile.id, repeated_batches.len());
                            repeated_batches.push(batch);
                        }
                    }
                }
            }
        }

        // We dont show entities and items in Effects Mode to avoid overlapping icons
        // Entities
        for entity in &map.entities {
            let entity_pos = Vec2::new(entity.position.x, entity.position.z);
            let pos =
                self.map_grid_to_local(screen_size, Vec2::new(entity_pos.x, entity_pos.y), map);
            let size = 1.0;
            let hsize = 0.5;

            // Find light on entity
            if let Some(Value::Light(light)) = entity.attributes.get("light") {
                if light.active {
                    let mut light = light.clone();
                    light.set_position(entity.position);
                    scene.dynamic_lights.push(light.compile());
                }
            }

            // Find light on entity items
            for (_, item) in entity.iter_inventory() {
                if let Some(Value::Light(light)) = item.attributes.get("light") {
                    if light.active {
                        let mut light = light.clone();
                        light.set_position(entity.position);
                        scene.dynamic_lights.push(light.compile());
                    }
                }
            }

            if let Some(Value::Source(source)) = entity.attributes.get("source") {
                if entity.attributes.get_bool_default("visible", false) {
                    if let Some(tile) = source.tile_from_tile_list(assets) {
                        if let Some(texture_index) = assets.tile_index(&tile.id) {
                            let mut batch = Batch2D::empty()
                                .source(PixelSource::StaticTileIndex(texture_index))
                                .receives_light(true);

                            batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                            textures.push(tile.clone());
                            repeated_offsets.insert(tile.id, repeated_batches.len());
                            repeated_batches.push(batch);
                        }
                    }
                }
            } else if let Some(Value::Source(source)) = entity.attributes.get("_source_seq") {
                if entity.attributes.get_bool_default("visible", false) {
                    if let Some(entity_tile) = source.entity_tile_id(entity.id, assets) {
                        let batch =
                            Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                                .source(entity_tile);
                        scene.d2_dynamic.push(batch);
                    }
                }
            }
        }

        scene.d2_dynamic = repeated_batches;
        scene.dynamic_textures = textures;
    }

    #[inline(always)]
    fn map_grid_to_local(
        &self,
        _screen_size: Vec2<f32>,
        grid_pos: Vec2<f32>,
        _map: &Map,
    ) -> Vec2<f32> {
        // let grid_space_pos = grid_pos * map.grid_size;
        // grid_space_pos + Vec2::new(map.offset.x, -map.offset.y) + screen_size / 2.0
        grid_pos
    }
}
