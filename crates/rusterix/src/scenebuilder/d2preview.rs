use crate::{
    Assets, Batch2D, Map, MapToolType, PixelSource, Rect, Scene, SceneHandler, Surface, Tile,
    Value, ValueContainer,
};
use MapToolType::*;
use scenevm::{Atom, DynamicObject, GeoId, Light};
use theframework::prelude::*;
use toml::*;
use vek::Vec2;

pub struct D2PreviewBuilder {
    map_tool_type: MapToolType,
    /// Hover geometry info
    pub hover: (Option<u32>, Option<u32>, Option<u32>),
    /// The current grid hover position
    pub hover_cursor: Option<Vec2<f32>>,
    /// Camera Position
    pub camera_pos: Option<vek::Vec3<f32>>,
    /// Camera Center
    pub look_at: Option<Vec3<f32>>,

    /// Clipping rectangle
    pub clip_rect: Option<Rect>,

    /// Draw Grid Switch
    pub draw_grid: bool,

    /// Stores textures for dynamic access
    pub textures: Vec<Tile>,

    /// Do not draw Rect based geometry
    no_rect_geo: bool,

    /// Editing slice
    editing_slice: f32,

    tile_size: i32,
}

impl Default for D2PreviewBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl D2PreviewBuilder {
    pub fn new() -> Self {
        Self {
            map_tool_type: Linedef,

            hover: (None, None, None),
            hover_cursor: None,

            camera_pos: None,
            look_at: None,

            clip_rect: None,
            draw_grid: true,

            textures: Vec::new(),

            no_rect_geo: false,
            editing_slice: 0.0,

            tile_size: 128,
        }
    }

    pub fn set_properties(&mut self, properties: &ValueContainer) {
        self.no_rect_geo = properties.get_bool_default("no_rect_geo", true);
        self.tile_size = properties.get_int_default("tile_size", 128);
        self.editing_slice = properties.get_float_default("editing_slice", 0.0);
    }

    pub fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        screen_size: Vec2<f32>,
        _properties: &ValueContainer,
    ) -> Scene {
        let mut scene = Scene::empty();

        for sector in &map.sectors {
            if let Some(geo) = sector.generate_geometry(map) {
                let mut vertices: Vec<[f32; 2]> = vec![];
                let mut uvs: Vec<[f32; 2]> = vec![];
                let bbox = sector.bounding_box(map);
                let shader_index = sector
                    .shader
                    .and_then(|shader_id| {
                        map.shaders
                            .get(&shader_id)
                            .map(|m| scene.add_shader(&m.build_shader()))
                    })
                    .flatten();

                let mut repeat = true;
                if sector.properties.get_int_default("tile_mode", 1) == 0 {
                    repeat = false;
                }

                // Use the floor or ceiling source
                let source = sector.properties.get_default_source();

                let mut processed = false;
                for vertex in &geo.0 {
                    let local =
                        self.map_grid_to_local(screen_size, Vec2::new(vertex[0], vertex[1]), map);

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

                if let Some(pixelsource) = source {
                    if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                        if let Some(texture_index) = assets.tile_index(&tile.id) {
                            let mut batch =
                                Batch2D::new(vertices.clone(), geo.1.clone(), uvs.clone())
                                    .repeat_mode(if repeat {
                                        crate::RepeatMode::RepeatXY
                                    } else {
                                        crate::RepeatMode::ClampXY
                                    })
                                    .source(PixelSource::StaticTileIndex(texture_index));
                            batch.shader = shader_index;
                            scene.d2_static.push(batch);
                            processed = true;
                        }
                    }
                }

                if let Some(shader_index) = shader_index
                    && processed == false
                {
                    let batch = Batch2D::new(vertices, geo.1, uvs).shader(shader_index);
                    scene.d2_static.push(batch);
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
        editing_surface: &Option<Surface>,
        scene_handler: &mut SceneHandler,
        draw_sectors: bool,
    ) {
        // let screen_aspect = screen_size.x / screen_size.y;
        let screen_pixel_size = 4.0;
        let size_x = screen_pixel_size / map.grid_size;
        // let size_y = size_x * screen_aspect / 2.0;

        scene.dynamic_lights = vec![];
        scene.d2_dynamic = vec![];

        scene_handler.clear_overlay();
        scene_handler.vm.execute(Atom::ClearDynamics);
        scene_handler.vm.execute(Atom::ClearLights);

        // Grid
        // if self.draw_grid {
        //     if scene.background.is_none() {
        //         let grid_shader = GridShader::new();
        //         scene.background = Some(Box::new(grid_shader));
        //     }
        // } else {
        //     scene.background = None;
        // }

        // Adjust the grid shader
        // if let Some(grid_shader) = &mut scene.background {
        //     grid_shader.set_parameter_f32("grid_size", map.grid_size);
        //     grid_shader.set_parameter_f32("subdivisions", map.subdivisions);
        //     grid_shader.set_parameter_vec2("offset", Vec2::new(map.offset.x, -map.offset.y));
        // }

        // Add the clipping area
        if let Some(clip_rect) = self.clip_rect {
            let rect = (
                Vec2::new(clip_rect.x, clip_rect.y),
                Vec2::new(
                    clip_rect.x + clip_rect.width,
                    clip_rect.y + clip_rect.height,
                ),
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(20),
                Vec2::new(rect.0.x, rect.0.y),
                Vec2::new(rect.1.x, rect.0.y),
                scene_handler.outline,
                1000,
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(21),
                Vec2::new(rect.0.x, rect.0.y),
                Vec2::new(rect.0.x, rect.1.y),
                scene_handler.outline,
                1000,
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(22),
                Vec2::new(rect.1.x, rect.1.y),
                Vec2::new(rect.1.x, rect.0.y),
                scene_handler.outline,
                1000,
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(23),
                Vec2::new(rect.1.x, rect.1.y),
                Vec2::new(rect.0.x, rect.1.y),
                scene_handler.outline,
                1000,
            );
        }

        // If this is an editing surface, add the outline
        if let Some(surface) = editing_surface {
            // Project into the surface’s profile 2D space
            let ring2d: Vec<Vec2<f32>> = surface
                .world_vertices
                .iter()
                .map(|p| surface.world_to_uv(*p))
                .collect();

            // let mut batch = Batch2D::empty()
            //     .source(PixelSource::Pixel(self.unselected_with_same_geometry))
            //     .mode(crate::PrimitiveMode::Lines);

            for i in 0..ring2d.len() {
                let mut a = ring2d[i];
                let mut b = ring2d[(i + 1) % ring2d.len()];
                a.y = -a.y;
                b.y = -b.y;
                // batch.add_line(a, b, 0.05);
                scene_handler.add_overlay_2d_line(
                    GeoId::Unknown(i as u32),
                    a,
                    b,
                    scene_handler.outline,
                    1000,
                );
            }
            // scene.d2_dynamic.push(batch);
        }

        if draw_sectors {
            let sectors = map.sorted_sectors_by_area();
            for sector in &sectors {
                if sector.intersects_vertical_slice(map, self.editing_slice, 1.0) {
                    let bbox = sector.bounding_box(map);

                    let is_rect = sector.properties.contains("rect")
                        || sector.properties.contains("rect_rendering");

                    if let Some(geo) = sector.generate_geometry(map) {
                        let mut vertices: Vec<[f32; 2]> = vec![];
                        let mut uvs: Vec<[f32; 2]> = vec![];

                        let mut repeat = true;
                        if sector.properties.get_int_default("tile_mode", 1) == 0 {
                            repeat = false;
                        }

                        // Use the floor or ceiling source
                        let source = sector.properties.get_default_source();
                        // if source.is_none() {
                        //     //     //|| self.activated_widgets.contains(&sector.id) {
                        //     //     source = sector.properties.get("ceiling_source");
                        //     //
                        //     source = Some(&default_source);
                        // }
                        //

                        for vertex in &geo.0 {
                            let local = Vec2::new(vertex[0], vertex[1]);

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

                        // Layer sorting with "rect" based layers with a lower priority
                        // This is mostly for screen widgets which get their layer prio from the TOML data.
                        let mut layer = 0;
                        if let Some(crate::Value::Str(data)) = sector.properties.get("data") {
                            if let Ok(table) = data.parse::<Table>() {
                                if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                                    if let Some(value) = ui.get("layer") {
                                        if let Some(v) = value.as_integer() {
                                            layer = v as i32;
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(pixelsource) = source {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                scene_handler.overlay_2d.add_poly_2d(
                                    GeoId::Sector(sector.id),
                                    tile.id,
                                    vertices,
                                    uvs,
                                    geo.1,
                                    if is_rect { 9 + layer } else { 10 + layer },
                                    true,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Add Vertices

        // let mut selected_batch = Batch2D::empty().source(PixelSource::Pixel(self.selection_color));
        // let mut batch = Batch2D::empty().source(PixelSource::Pixel([128, 128, 128, 255]));

        if self.map_tool_type == MapToolType::Selection
            || self.map_tool_type == MapToolType::Vertex
            || self.map_tool_type == MapToolType::Sector
            || self.map_tool_type == MapToolType::Linedef
        {
            for vertex in &map.vertices {
                if !vertex.intersects_vertical_slice(self.editing_slice, 1.0) {
                    continue;
                }
                if let Some(vertex_pos) = map.get_vertex(vertex.id) {
                    if self.map_tool_type == MapToolType::Linedef {
                        // In linedef mode, only show vertices that are part of selected linedefs
                        let mut found = false;
                        for linedef_id in map.selected_linedefs.iter() {
                            if let Some(linedef) = map.find_linedef(*linedef_id) {
                                if linedef.start_vertex == vertex.id
                                    || linedef.end_vertex == vertex.id
                                {
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            continue;
                        }
                    } else if self.map_tool_type == MapToolType::Sector {
                        // In sector mode, only show vertices that are part of selected sectors
                        let mut found = false;
                        for sector_id in map.selected_sectors.iter() {
                            if let Some(sector) = map.find_sector(*sector_id) {
                                for linedef_id in sector.linedefs.iter() {
                                    if let Some(linedef) = map.find_linedef(*linedef_id) {
                                        if linedef.start_vertex == vertex.id
                                            || linedef.end_vertex == vertex.id
                                        {
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if !found {
                            continue;
                        }
                    }

                    let pos = self.map_grid_to_local(screen_size, vertex_pos, map);

                    if self.hover.0 == Some(vertex.id) || map.selected_vertices.contains(&vertex.id)
                    {
                        scene_handler.overlay_2d.add_square_2d(
                            GeoId::Vertex(vertex.id),
                            scene_handler.selected,
                            [pos.x, pos.y],
                            size_x * 2.0,
                            10000,
                            true,
                        );
                        // selected_batch.add_rectangle(
                        //     pos.x - size_x,
                        //     pos.y - size_y,
                        //     size_x * 2.0,
                        //     size_y * 2.0,
                        // );
                    } else {
                        scene_handler.overlay_2d.add_square_2d(
                            GeoId::Vertex(vertex.id),
                            scene_handler.white,
                            [pos.x, pos.y],
                            size_x * 2.0,
                            10000,
                            true,
                        );
                        // batch.add_rectangle(
                        //     pos.x - size_x,
                        //     pos.y - size_y,
                        //     size_x * 2.0,
                        //     size_y * 2.0,
                        // );
                    }
                }
            }
        }
        // scene.d2_dynamic.push(selected_batch);
        // scene.d2_dynamic.push(batch);

        // Add Lines
        if self.map_tool_type == MapToolType::Selection
            || self.map_tool_type == MapToolType::Linedef
            || self.map_tool_type == MapToolType::Sector
            || self.map_tool_type == MapToolType::Effects
            || self.map_tool_type == MapToolType::MiniMap
            || self.map_tool_type == MapToolType::General
            || self.map_tool_type == MapToolType::Rect
        {
            let mut selected_lines = vec![];
            let mut non_selected_lines = vec![];
            let mut non_selected_lines_with_selected_graph = vec![];

            for linedef in &map.linedefs {
                if !linedef.intersects_vertical_slice(map, self.editing_slice, 1.0) {
                    continue;
                }

                let mut draw = true;

                // No outlines for the rect tool based sectors in the minimap or if no_rect_geo is enabled.
                if self.map_tool_type == MapToolType::MiniMap || self.no_rect_geo {
                    let mut only_unnamed_rect_sectors = true;
                    for sector in &map.sectors {
                        if sector.linedefs.contains(&linedef.id) {
                            let is_rect = sector.properties.contains("rect")
                                || sector.properties.contains("rect_rendering");
                            if !is_rect || !sector.name.is_empty() {
                                // This linedef belongs to a non-rect sector or a named rect sector
                                only_unnamed_rect_sectors = false;
                                break;
                            }
                        }
                    }
                    if only_unnamed_rect_sectors {
                        // Only hide if ALL sectors this linedef belongs to are unnamed rect sectors
                        for sector in &map.sectors {
                            if sector.linedefs.contains(&linedef.id) {
                                if (sector.properties.contains("rect")
                                    || sector.properties.contains("rect_rendering"))
                                    && sector.name.is_empty()
                                {
                                    draw = false;
                                    break;
                                }
                            }
                        }
                    }

                    // If the linedef is not found in any sector and has a wall width of 0.0, don't draw it.
                    // Prevents deleted rect tool based sectors to be drawn.
                    // Problem: Also hides standalone walls
                    // if draw
                    //     && !found_in_sector
                    //     && linedef.properties.get_float_default("wall_width", 0.0) == 0.0
                    //     && !map.possible_polygon.contains(&linedef.id)
                    // {
                    //     draw = false;
                    // }
                }

                if draw {
                    if let Some(start_vertex) = map.get_vertex(linedef.start_vertex) {
                        let start_pos = self.map_grid_to_local(screen_size, start_vertex, map);
                        if let Some(end_vertex) = map.get_vertex(linedef.end_vertex) {
                            let end_pos = self.map_grid_to_local(screen_size, end_vertex, map);

                            // Special color for wall profile
                            if linedef.properties.contains("profile") {
                                non_selected_lines_with_selected_graph.push((
                                    GeoId::Linedef(linedef.id),
                                    start_pos,
                                    end_pos,
                                ));
                            }

                            // ---
                            // Check for wall lights
                            for i in 1..=4 {
                                if let Some(light) = super::get_linedef_light_from_geo_graph(
                                    &linedef.properties,
                                    i,
                                    map,
                                    start_vertex,
                                    end_vertex,
                                    i as f32 - 0.5,
                                ) {
                                    scene.dynamic_lights.push(light);
                                }
                            }
                            // --

                            let mut selected = false;
                            if self.hover.1 == Some(linedef.id)
                                || map.selected_linedefs.contains(&linedef.id)
                            {
                                selected = true;
                            } else if self.map_tool_type == MapToolType::Sector
                                || self.map_tool_type == MapToolType::General
                                || self.map_tool_type == MapToolType::Selection
                            {
                                for sector_id in &linedef.sector_ids {
                                    if self.hover.2 == Some(*sector_id)
                                        || map.selected_sectors.contains(sector_id)
                                    {
                                        selected = true;
                                    }
                                }
                            }

                            if selected {
                                selected_lines.push((
                                    GeoId::Linedef(linedef.id),
                                    start_pos,
                                    end_pos,
                                ));
                            } else {
                                non_selected_lines.push((
                                    GeoId::Linedef(linedef.id),
                                    start_pos,
                                    end_pos,
                                ));
                            }
                        }
                    }
                }
            }

            // Draw non-selected lines first
            // let mut batch = Batch2D::empty()
            //     .source(PixelSource::Pixel(WHITE))
            //     .mode(crate::PrimitiveMode::Lines);
            for (id, start_pos, end_pos) in non_selected_lines {
                // batch.add_line(start_pos, end_pos, 0.05);
                scene_handler.add_overlay_2d_line(id, start_pos, end_pos, scene_handler.white, 900);
            }
            // scene.d2_dynamic.push(batch);

            if !non_selected_lines_with_selected_graph.is_empty() {
                // let mut batch = Batch2D::empty()
                //     .source(PixelSource::Pixel(self.unselected_with_same_geometry))
                //     .mode(crate::PrimitiveMode::Lines);
                for (id, start_pos, end_pos) in non_selected_lines_with_selected_graph {
                    // batch.add_line(start_pos, end_pos, 0.05);
                    scene_handler.add_overlay_2d_line(
                        id,
                        start_pos,
                        end_pos,
                        scene_handler.outline,
                        900,
                    );
                }
                // scene.d2_dynamic.push(batch);
            }

            // Draw selected lines
            // let mut batch = Batch2D::empty()
            //     .source(PixelSource::Pixel(self.selection_color))
            //     .mode(crate::PrimitiveMode::Lines);
            for (id, start_pos, end_pos) in selected_lines {
                // batch.add_line(start_pos, end_pos, 0.05);
                scene_handler.add_overlay_2d_line(
                    id,
                    start_pos,
                    end_pos,
                    scene_handler.selected,
                    1000,
                );
            }
            // scene.d2_dynamic.push(batch);
        }

        if self.map_tool_type != MapToolType::Effects {
            // Items
            let mut item_counter = 0;
            for item in &map.items {
                let item_pos = Vec2::new(item.position.x, item.position.z);
                let pos =
                    self.map_grid_to_local(screen_size, Vec2::new(item_pos.x, item_pos.y), map);
                let size = 1.0;
                let hsize = 0.5;

                if let Some(Value::Light(light)) = item.attributes.get("light") {
                    // if light.active {
                    //     let mut light = light.clone();
                    //     light.set_position(item.position);
                    //     scene.dynamic_lights.push(light.compile());
                    // }

                    scene_handler.vm.execute(Atom::AddLight {
                        id: GeoId::ItemLight(item.id),
                        light: Light::new_pointlight(item.position)
                            .with_color(Vec3::from(light.get_color()))
                            .with_intensity(light.get_intensity())
                            .with_emitting(light.active)
                            .with_start_distance(light.get_start_distance())
                            .with_end_distance(light.get_end_distance())
                            .with_flicker(light.get_flicker()),
                    });
                }

                if let Some(Value::Source(source)) = item.attributes.get("source") {
                    if item.attributes.get_bool_default("visible", false) {
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            // scene_handler.overlay.add_square_2d(
                            //     GeoId::Character(item.id),
                            //     tile.id,
                            //     [pos.x, pos.y],
                            //     size,
                            //     100,
                            //     true,
                            // );

                            let dynamic = DynamicObject::billboard_tile_2d(
                                GeoId::Item(item.id),
                                tile.id,
                                pos,
                                1.0,
                                1.0,
                            );
                            scene_handler
                                .vm
                                .execute(Atom::AddDynamic { object: dynamic });

                            // if let Some(texture_index) = assets.tile_index(&tile.id) {
                            //     let batch = Batch2D::from_rectangle(
                            //         pos.x - hsize,
                            //         pos.y - hsize,
                            //         size,
                            //         size,
                            //     )
                            //     .source(PixelSource::StaticTileIndex(texture_index));
                            //     scene.d2_dynamic.push(batch);
                            // }
                        }
                    }
                } else if let Some(Value::Source(source)) = item.attributes.get("_source_seq") {
                    if item.attributes.get_bool_default("visible", false) {
                        if let Some(entity_tile) = source.item_tile_id(item.id, assets) {
                            let batch =
                                Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                                    .source(entity_tile);
                            scene.d2_dynamic.push(batch);
                        }
                    }
                } else if Some(item.creator_id) == map.selected_entity_item {
                    let batch = Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                        .source(PixelSource::DynamicTileIndex(2));
                    scene.d2_dynamic.push(batch);

                    scene_handler.overlay_2d.add_square_2d(
                        GeoId::Item(item_counter),
                        scene_handler.item_on,
                        [pos.x, pos.y],
                        size,
                        100,
                        true,
                    );
                    item_counter += 1;
                } else {
                    let batch = Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                        .source(PixelSource::DynamicTileIndex(3));
                    scene.d2_dynamic.push(batch);

                    scene_handler.overlay_2d.add_square_2d(
                        GeoId::Item(item_counter),
                        scene_handler.item_off,
                        [pos.x, pos.y],
                        size,
                        100,
                        true,
                    );
                    item_counter += 1;
                }
            }

            // If the server is not running, entities do not have ids yet, so we give a fake one
            let mut entity_counter = 0;
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
                            // scene_handler.overlay.add_square_2d(
                            //     GeoId::Character(entity.id),
                            //     tile.id,
                            //     [pos.x, pos.y],
                            //     size,
                            //     100,
                            //     true,
                            // );

                            let dynamic = DynamicObject::billboard_tile_2d(
                                GeoId::Character(entity.id),
                                tile.id,
                                pos,
                                1.0,
                                1.0,
                            );
                            scene_handler
                                .vm
                                .execute(Atom::AddDynamic { object: dynamic });

                            // if let Some(texture_index) = assets.tile_index(&tile.id) {
                            //     let batch = Batch2D::from_rectangle(
                            //         pos.x - hsize,
                            //         pos.y - hsize,
                            //         size,
                            //         size,
                            //     )
                            //     .source(PixelSource::StaticTileIndex(texture_index));
                            //     scene.d2_dynamic.push(batch);
                            // }
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
                } else if Some(entity.creator_id) == map.selected_entity_item {
                    // let batch = Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                    //     .source(PixelSource::DynamicTileIndex(0));
                    // scene.d2_dynamic.push(batch);

                    scene_handler.overlay_2d.add_square_2d(
                        GeoId::Character(entity_counter),
                        scene_handler.character_on,
                        [pos.x, pos.y],
                        size,
                        100,
                        true,
                    );
                    entity_counter += 1;
                } else {
                    // let batch = Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                    //     .source(PixelSource::DynamicTileIndex(1));
                    // scene.d2_dynamic.push(batch);

                    scene_handler.overlay_2d.add_square_2d(
                        GeoId::Character(entity_counter),
                        scene_handler.character_off,
                        [pos.x, pos.y],
                        size,
                        100,
                        true,
                    );
                    entity_counter += 1;
                }
            }
        }

        // let mut white_batch = Batch2D::empty()
        //     .source(PixelSource::Pixel(WHITE))
        //     .mode(crate::PrimitiveMode::Lines);

        // For rectangle selection preview
        if let Some(rect) = map.curr_rectangle {
            /*
            white_batch.add_line(
                Vec2::new(rect.0.x, rect.0.y),
                Vec2::new(rect.1.x, rect.0.y),
                1.0,
            );
            white_batch.add_line(
                Vec2::new(rect.0.x, rect.0.y),
                Vec2::new(rect.0.x, rect.1.y),
                1.0,
            );
            white_batch.add_line(
                Vec2::new(rect.1.x, rect.1.y),
                Vec2::new(rect.1.x, rect.0.y),
                1.0,
            );
            white_batch.add_line(
                Vec2::new(rect.1.x, rect.1.y),
                Vec2::new(rect.0.x, rect.1.y),
                1.0,
            );*/

            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(50),
                Vec2::new(rect.0.x, rect.0.y),
                Vec2::new(rect.1.x, rect.0.y),
                scene_handler.white,
                1000,
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(51),
                Vec2::new(rect.0.x, rect.0.y),
                Vec2::new(rect.0.x, rect.1.y),
                scene_handler.white,
                1000,
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(52),
                Vec2::new(rect.1.x, rect.1.y),
                Vec2::new(rect.1.x, rect.0.y),
                scene_handler.white,
                1000,
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(53),
                Vec2::new(rect.1.x, rect.1.y),
                Vec2::new(rect.0.x, rect.1.y),
                scene_handler.white,
                1000,
            );
        }

        // Draw linedefs in possible_polygon (polygon being constructed)
        for (i, linedef_id) in map.possible_polygon.iter().enumerate() {
            if let Some(linedef) = map.find_linedef(*linedef_id) {
                if let Some(start_vertex) = map.get_vertex(linedef.start_vertex) {
                    if let Some(end_vertex) = map.get_vertex(linedef.end_vertex) {
                        let start_pos = self.map_grid_to_local(screen_size, start_vertex, map);
                        let end_pos = self.map_grid_to_local(screen_size, end_vertex, map);
                        scene_handler.add_overlay_2d_line(
                            GeoId::Unknown(1000 + i as u32),
                            start_pos,
                            end_pos,
                            scene_handler.white,
                            1001,
                        );
                    }
                }
            }
        }

        // For line action previews
        if let Some(grid_pos) = map.curr_grid_pos {
            if let Some(mouse_pos) = map.curr_mouse_pos {
                // white_batch.add_line(grid_pos, mouse_pos, 1.0)
                scene_handler.add_overlay_2d_line(
                    GeoId::Unknown(25),
                    grid_pos,
                    mouse_pos,
                    scene_handler.white,
                    1000,
                );
            }
        }
        // scene.d2_dynamic.push(white_batch);

        // Hover Cursor
        if self.map_tool_type != MapToolType::Rect {
            if let Some(hover_pos) = self.hover_cursor {
                // let mut yellow_batch =
                //     Batch2D::empty().source(PixelSource::Pixel(vek::Rgba::yellow().into_array()));
                // yellow_batch.add_rectangle(
                //     hover_pos.x - size_x,
                //     hover_pos.y - size_y,
                //     size_x * 2.0,
                //     size_y * 2.0,
                // );
                // scene.d2_dynamic.push(yellow_batch);

                scene_handler.overlay_2d.add_square_2d(
                    GeoId::Triangle(1000),
                    scene_handler.yellow,
                    [hover_pos.x, hover_pos.y],
                    size_x * 2.0,
                    10000,
                    true,
                );
            }
        }

        scene_handler.set_overlay();
        scene.dynamic_textures = self.textures.clone();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_linedefs_cpu(&self, map: &Map, scene: &mut Scene, screen_size: Vec2<f32>) {
        let mut batch = Batch2D::empty()
            .source(PixelSource::Pixel(WHITE))
            .mode(crate::PrimitiveMode::Lines);

        for linedef in &map.linedefs {
            if let Some(start_vertex) = map.get_vertex(linedef.start_vertex) {
                let start_pos = self.map_grid_to_local(screen_size, start_vertex, map);
                if let Some(end_vertex) = map.get_vertex(linedef.end_vertex) {
                    let end_pos = self.map_grid_to_local(screen_size, end_vertex, map);

                    batch.add_line(start_pos, end_pos, 0.05);
                }
            }
        }

        scene.d2_dynamic.push(batch);
    }

    pub fn set_map_tool_type(&mut self, tool: MapToolType) {
        self.map_tool_type = tool;
    }

    pub fn set_map_hover_info(
        &mut self,
        hover: (Option<u32>, Option<u32>, Option<u32>),
        hover_cursor: Option<Vec2<f32>>,
    ) {
        self.hover = hover;
        self.hover_cursor = hover_cursor;
    }

    pub fn set_camera_info(&mut self, pos: Option<vek::Vec3<f32>>, look_at: Option<Vec3<f32>>) {
        self.camera_pos = pos;
        self.look_at = look_at;
    }

    pub fn set_clip_rect(&mut self, clip_rect: Option<Rect>) {
        self.clip_rect = clip_rect;
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
