use crate::{
    Assets, Batch3D, D3Camera, Map, PixelSource, Scene, SceneHandler, Value, ValueContainer,
};
use scenevm::{Atom, DynamicObject, GeoId, Light};
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
        /*
        let scene = Scene::empty();
        self.map = map.clone();

        let mut sample_mode = SampleMode::Nearest;
        if let Some(Value::SampleMode(sm)) = properties.get("sample_mode") {
            sample_mode = *sm;
        }

        // let atlas_size = atlas.width as f32;
        self.tile_size = properties.get_int_default("tile_size", 128);

        let mut textures = vec![Tile::from_texture(assets.atlas.clone())];

        let atlas_batch = Batch::emptyd3();

        // Repeated tile textures have their own batches
        let mut repeated_batches: Vec<Batch<[f32; 4]>> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();

        // Create sectors
        for sector in &map.sectors {
            // // Add Floor Light
            // if let Some(Value::Light(light)) = sector.properties.get("floor_light") {
            //     if let Some(center) = sector.center(map) {
            //         let bbox = sector.bounding_box(map);
            //         let light = light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
            //         scene.lights.push(light);
            //     }
            // }
            // // Add Ceiling Light
            // if let Some(Value::Light(light)) = sector.properties.get("ceiling_light") {
            //     if let Some(center) = sector.center(map) {
            //         let bbox = sector.bounding_box(map);
            //         let light = light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
            //         scene.lights.push(light);
            //     }
            // }

            let mut add_it = true;

            // Special cases from the Rect tool
            let mut add_it_as_box = false;
            let mut add_it_as_floor = false;

            // Make sure we add Rect sectors with a rendering mode of "Box" as a box
            if sector.layer.is_some() {
                let render_mode = sector.properties.get_int_default("rect_rendering", 0);
                match render_mode {
                    0 => add_it = false,
                    1 => add_it_as_box = true,
                    2 => add_it_as_floor = true,
                    _ => {}
                }
            }

            if add_it {
                let material: Option<Material> =
                    super::get_material_from_geo_graph(&sector.properties, 2, map);

                if let Some((vertices, indices)) = sector.generate_geometry(map) {
                    let sector_elevation = sector.properties.get_float_default("floor_height", 0.0);

                    // Generate floor geometry
                    if !add_it_as_box {
                        if let Some(Value::Source(pixelsource)) =
                            sector.properties.get("floor_source")
                        {
                            if let Some(tile) = pixelsource.to_tile(
                                assets,
                                self.tile_size as usize,
                                &sector.properties,
                                map,
                            ) {
                                let floor_vertices = vertices
                                    .iter()
                                    .map(|&v| {
                                        [
                                            v[0],
                                            sector_elevation
                                                + if add_it_as_floor { 0.2 } else { 0.0 },
                                            v[1],
                                            1.0,
                                        ]
                                    })
                                    .collect();

                                let floor_uvs = vertices.iter().map(|&v| [v[0], v[1]]).collect();

                                if material.is_some() {
                                    let texture_index = textures.len();
                                    let mut batch = Batch::emptyd3()
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .texture_index(texture_index);
                                    batch.material = material;
                                    batch.add(floor_vertices, indices.clone(), floor_uvs);

                                    textures.push(tile.clone());
                                    repeated_offsets.insert(tile.id, repeated_batches.len());
                                    repeated_batches.push(batch);
                                } else if let Some(offset) = repeated_offsets.get(&tile.id) {
                                    repeated_batches[*offset].add(
                                        floor_vertices,
                                        indices.clone(),
                                        floor_uvs,
                                    );
                                } else {
                                    let texture_index = textures.len();

                                    let mut batch = Batch::emptyd3()
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .sample_mode(sample_mode)
                                        .texture_index(texture_index);

                                    batch.add(floor_vertices, indices.clone(), floor_uvs);

                                    textures.push(tile.clone());
                                    repeated_offsets.insert(tile.id, repeated_batches.len());
                                    repeated_batches.push(batch);
                                }
                            }
                        }
                    }

                    // Generate ceiling geometry

                    let mut create_ceiling = true;
                    if camera_id == "iso"
                        && sector.properties.get_int_default("ceiling_in_iso", 0) == 1
                    {
                        create_ceiling = false;
                    }

                    if create_ceiling || add_it_as_box {
                        let material: Option<Material> =
                            super::get_material_from_geo_graph(&sector.properties, 3, map);

                        let source = if add_it_as_box {
                            sector.properties.get("floor_source")
                        } else {
                            sector.properties.get("ceiling_source")
                        };

                        if let Some(Value::Source(pixelsource)) = &source {
                            if let Some(tile) = pixelsource.to_tile(
                                assets,
                                self.tile_size as usize,
                                &sector.properties,
                                map,
                            ) {
                                let ceiling_vertices = vertices
                                    .iter()
                                    .map(|&v| {
                                        [
                                            v[0],
                                            sector
                                                .properties
                                                .get_float_default("ceiling_height", 0.0),
                                            v[1],
                                            1.0,
                                        ]
                                    })
                                    .collect();

                                let ceiling_uvs = vertices.iter().map(|&v| [v[0], v[1]]).collect();
                                // let ceiling_indices =
                                //     indices.iter().map(|&v| (v.2, v.1, v.0)).collect();

                                if material.is_some() {
                                    let texture_index = textures.len();
                                    let mut batch = Batch::emptyd3()
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .texture_index(texture_index);
                                    batch.material = material;
                                    batch.add(ceiling_vertices, indices.clone(), ceiling_uvs);

                                    textures.push(tile.clone());
                                    repeated_offsets.insert(tile.id, repeated_batches.len());
                                    repeated_batches.push(batch);
                                } else if let Some(offset) = repeated_offsets.get(&tile.id) {
                                    repeated_batches[*offset].add(
                                        ceiling_vertices,
                                        indices,
                                        ceiling_uvs,
                                    );
                                } else {
                                    let texture_index = textures.len();

                                    let mut batch = Batch::emptyd3()
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .sample_mode(sample_mode)
                                        .texture_index(texture_index);

                                    batch.add(ceiling_vertices, indices, ceiling_uvs);

                                    textures.push(tile.clone());
                                    repeated_offsets.insert(tile.id, repeated_batches.len());
                                    repeated_batches.push(batch);
                                }
                            }
                        }
                    }

                    // Generate wall geometry
                    if !add_it_as_floor {
                        for &linedef_id in &sector.linedefs {
                            if let Some(linedef) = map.find_linedef(linedef_id) {
                                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                                        // Check for wall lights
                                        for i in 1..=4 {
                                            if let Some(light) =
                                                super::get_linedef_light_from_geo_graph(
                                                    &linedef.properties,
                                                    i,
                                                    map,
                                                    start_vertex.as_vec2(),
                                                    end_vertex.as_vec2(),
                                                    i as f32 - 0.5,
                                                )
                                            {
                                                scene.lights.push(light);
                                            }
                                        }
                                        // --

                                        let repeat_sources =
                                            linedef.properties.get_int_default("source_repeat", 0)
                                                == 0;
                                        self.add_wall(
                                            sector_elevation,
                                            &start_vertex.as_vec2(),
                                            &end_vertex.as_vec2(),
                                            linedef
                                                .properties
                                                .get_float_default("wall_height", 0.0),
                                            linedef
                                                .properties
                                                .get("row1_source")
                                                .and_then(|v| v.to_source()),
                                            linedef
                                                .properties
                                                .get("row2_source")
                                                .and_then(|v| v.to_source()),
                                            linedef
                                                .properties
                                                .get("row3_source")
                                                .and_then(|v| v.to_source()),
                                            linedef
                                                .properties
                                                .get("row4_source")
                                                .and_then(|v| v.to_source()),
                                            repeat_sources,
                                            assets,
                                            &linedef.properties,
                                            map,
                                            &mut repeated_offsets,
                                            &mut repeated_batches,
                                            &mut textures,
                                            &sample_mode,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add standalone walls
        for linedef in &map.linedefs {
            if linedef.front_sector.is_none() && linedef.back_sector.is_none() {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                        let repeat_sources =
                            linedef.properties.get_int_default("source_repeat", 0) == 0;
                        self.add_wall(
                            0.0,
                            &start_vertex.as_vec2(),
                            &end_vertex.as_vec2(),
                            linedef.properties.get_float_default("wall_height", 0.0),
                            linedef
                                .properties
                                .get("row1_source")
                                .and_then(|v| v.to_source()),
                            linedef
                                .properties
                                .get("row2_source")
                                .and_then(|v| v.to_source()),
                            linedef
                                .properties
                                .get("row3_source")
                                .and_then(|v| v.to_source()),
                            linedef
                                .properties
                                .get("row4_source")
                                .and_then(|v| v.to_source()),
                            repeat_sources,
                            assets,
                            &linedef.properties,
                            map,
                            &mut repeated_offsets,
                            &mut repeated_batches,
                            &mut textures,
                            &sample_mode,
                        );
                    }
                }
            }
        }

        if camera_id != "iso" {
            // Add Sky
            if let Some(sky_texture_id) = map.sky_texture {
                Self::add_sky(
                    &sky_texture_id,
                    &assets.tiles,
                    &mut repeated_offsets,
                    &mut repeated_batches,
                    &mut textures,
                );
            }
        }

        // ---

        let mut batches = repeated_batches;
        batches.extend(vec![atlas_batch]);

        scene.mapmini = map.as_mini(&assets.blocking_tiles());
        scene.d3_static = batches;
        scene.textures = textures;
        scene.compute_static_normals();
        scene
        */
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

        let basis = camera.basis_vectors();

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
                        let size = 2.0;
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            let center3 =
                                Vec3::new(entity.position.x, size * 0.5, entity.position.z);

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

                        let center3 = Vec3::new(entity.position.x, size * 0.5, entity.position.z);
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
                        let size = 2.0;
                        let center3 = Vec3::new(entity.position.x, size * 0.5, entity.position.z);
                        if let Some(entity_tile) = source.entity_tile_id(entity.id, assets) {
                            let mut batch = Batch3D::empty()
                                .repeat_mode(crate::RepeatMode::RepeatXY)
                                .source(entity_tile);

                            add_billboard(center3, size, camera, &mut batch);
                            batches.push(batch);
                        }
                    }
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
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            let center3 = Vec3::new(item.position.x, size * 0.5, item.position.z);

                            let dynamic = DynamicObject::billboard_tile(
                                GeoId::Item(item.id),
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

                        let center3 = Vec3::new(item.position.x, size * 0.5, item.position.z);
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
                } else if let Some(Value::Source(source)) = item.attributes.get("_source_seq") {
                    if item.attributes.get_bool_default("visible", false) {
                        let size = 2.0;
                        let center3 = Vec3::new(item.position.x, size * 0.5, item.position.z);
                        if let Some(item_tile) = source.item_tile_id(item.id, assets) {
                            let mut batch = Batch3D::empty()
                                .repeat_mode(crate::RepeatMode::RepeatXY)
                                .source(item_tile);

                            add_billboard(center3, size, camera, &mut batch);
                            batches.push(batch);
                        }
                    }
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
