use crate::texture::RgbaTexture;
use crate::{prelude::*, server::context::MapToolType};
use euc::*;
use rayon::prelude::*;
use rect_packer::*;
use theframework::prelude::*;
use vek::*;

pub struct MapRender {
    pub textures: FxHashMap<Uuid, TheRGBATile>,

    atlas_size: f32,
    sampler: Option<Tiled<Nearest<RgbaTexture>>>,
    elements: FxHashMap<Uuid, Vec<Vec4i>>,

    texture_sampler: FxHashMap<Uuid, Vec<Tiled<Nearest<RgbaTexture>>>>,

    pub materials: IndexMap<Uuid, MaterialFXObject>,
    pub position: Vec3f,
    pub hover_pos: Option<Vec3i>,
}

#[allow(clippy::new_without_default)]
impl MapRender {
    pub fn new() -> Self {
        Self {
            textures: FxHashMap::default(),

            atlas_size: 1.0,
            sampler: None,
            elements: FxHashMap::default(),
            texture_sampler: FxHashMap::default(),

            materials: IndexMap::default(),
            position: Vec3f::zero(),
            hover_pos: None,
        }
    }

    pub fn set_region(&mut self, _region: &Region) {}

    pub fn set_textures(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        let atlas_size = 1024;
        let mut packer = Packer::new(Config {
            width: atlas_size,
            height: atlas_size,
            border_padding: 0,
            rectangle_padding: 0,
        });

        let mut elements: FxHashMap<Uuid, Vec<Vec4i>> = FxHashMap::default();
        let mut texture_sampler: FxHashMap<Uuid, Vec<Tiled<Nearest<RgbaTexture>>>> =
            FxHashMap::default();

        // Create rectangles
        for (id, tile) in tiles.iter() {
            let mut array: Vec<Vec4i> = vec![];
            let mut sammpler_array: Vec<Tiled<Nearest<RgbaTexture>>> = vec![];
            for b in &tile.buffer {
                if let Some(rect) = packer.pack(b.dim().width, b.dim().height, false) {
                    array.push(vec4i(rect.x, rect.y, rect.width, rect.height));
                }

                let texture = RgbaTexture::new(
                    b.pixels().to_vec(),
                    b.dim().width as usize,
                    b.dim().height as usize,
                );
                sammpler_array.push(texture.nearest().tiled());
            }
            elements.insert(*id, array);
            texture_sampler.insert(*id, sammpler_array);
        }

        // Create atlas
        let mut atlas = vec![0; atlas_size as usize * atlas_size as usize * 4];

        // Copy textures into atlas
        for (id, tile) in tiles.iter() {
            if let Some(rects) = elements.get(id) {
                for (buffer, rect) in tile.buffer.iter().zip(rects) {
                    let width = buffer.dim().width as usize;
                    let height = buffer.dim().height as usize;
                    let rect_x = rect.x as usize;
                    let rect_y = rect.y as usize;

                    for y in 0..height {
                        for x in 0..width {
                            let src_index = (y * width + x) * 4;
                            let dest_index =
                                ((rect_y + y) * atlas_size as usize + (rect_x + x)) * 4;

                            atlas[dest_index..dest_index + 4]
                                .copy_from_slice(&buffer.pixels()[src_index..src_index + 4]);
                        }
                    }
                }
            }
        }

        let texture = RgbaTexture::new(atlas, atlas_size as usize, atlas_size as usize);
        self.sampler = Some(texture.nearest().tiled());
        self.atlas_size = atlas_size as f32;
        self.elements = elements;

        self.textures = tiles;
        self.texture_sampler = texture_sampler;
    }

    pub fn set_position(&mut self, position: Vec3f) {
        self.position = position;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        update: &mut RegionUpdate,
        settings: &mut RegionDrawSettings,
        server_ctx: Option<&ServerContext>,
        compute_delta: bool,
        _palette: &ThePalette,
    ) {
        let _start = self.get_time();

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;

        let screen_size = vec2f(width as f32, height as f32);
        let region_height = region.height * region.grid_size;

        let grid_size = region.map.grid_size;

        if compute_delta {
            update.generate_character_pixel_positions(
                grid_size,
                &self.textures,
                vec2i(width as i32, height as i32),
                region_height,
                settings,
            );
        }

        // let max_render_distance = 20;

        // Fill the code level with the blocking info and collect lights
        let mut level = Level::new(region.width, region.height, settings.time);
        region.fill_code_level(&mut level, &self.textures, update, region);

        // Collect the material params
        let mut material_params: FxHashMap<Uuid, Vec<Vec<f32>>> = FxHashMap::default();
        for (id, material) in &self.materials {
            let params = material.load_parameters(&settings.time);
            material_params.insert(*id, params);
        }

        // Collect the render settings params
        // let render_settings_params: Vec<Vec<f32>> = region.regionfx.load_parameters(&settings.time);

        if let Some(server_ctx) = server_ctx {
            let mut drawer = EucDraw::new(width, height);

            if region.map.camera == MapCamera::TwoD {
                // Draw Grid
                buffer
                    .pixels_mut()
                    .par_rchunks_exact_mut(width * 4)
                    .enumerate()
                    .for_each(|(j, line)| {
                        for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                            let i = j * width + i;

                            let xx = (i % width) as f32;
                            let yy = (i / width) as f32;
                            // let x = xx / width as f32;
                            // let y = yy / height as f32;

                            let col = self.grid_at(
                                vec2f(xx, yy),
                                vec2f(width as f32, height as f32),
                                region.map.grid_size,
                                region.map.offset,
                                region.map.subdivisions,
                            );

                            pixel.copy_from_slice(&TheColor::from_vec4f(col).to_u8_array());
                        }
                    });

                // Draw Sectors
                if server_ctx.curr_map_tool_type == MapToolType::General
                    || server_ctx.curr_map_tool_type == MapToolType::Selection
                    || server_ctx.curr_map_tool_type == MapToolType::Sector
                {
                    for sector in &region.map.sectors {
                        if let Some(geo) = sector.generate_geometry(&region.map) {
                            // Convert the triangles from grid to local coordinates
                            let mut vertices: Vec<Vec2f> = vec![];
                            let mut uvs: Vec<Vec2f> = vec![];
                            let bbox = sector.bounding_box(&region.map);

                            let repeat = true;

                            if let Some(floor_texture_id) = &sector.floor_texture {
                                if let Some(el) = self.elements.get(floor_texture_id) {
                                    for vertex in &geo.0 {
                                        let local = ServerContext::map_grid_to_local(
                                            screen_size,
                                            vec2f(vertex[0], vertex[1]),
                                            &region.map,
                                        );

                                        // Scale up to polygon bbox
                                        if !repeat {
                                            let uv = vec2f(
                                                (el[0].x as f32
                                                    + ((vertex[0] - bbox.0.x)
                                                        / (bbox.1.x - bbox.0.x)
                                                        * el[0].z as f32))
                                                    / self.atlas_size,
                                                (el[0].y as f32
                                                    + (vertex[1] - bbox.0.y)
                                                        / (bbox.1.y - bbox.0.y)
                                                        * el[0].w as f32)
                                                    / self.atlas_size,
                                            );
                                            uvs.push(uv);
                                        } else {
                                            let texture_scale = 1.0;
                                            let uv = vec2f(
                                                (vertex[0] - bbox.0.x) / texture_scale,
                                                (vertex[1] - bbox.0.y) / texture_scale,
                                            );
                                            uvs.push(uv);
                                        }
                                        vertices.push(local);
                                    }

                                    drawer.add_textured_polygon(vertices, geo.1, uvs);
                                    if !repeat {
                                        if let Some(sampler) = &self.sampler {
                                            drawer.draw_as_textured_triangles(sampler, false);
                                        }
                                    } else if let Some(sampler_array) =
                                        self.texture_sampler.get(floor_texture_id)
                                    {
                                        let index = settings.anim_counter % sampler_array.len();
                                        drawer.draw_as_textured_triangles(
                                            &sampler_array[index],
                                            false,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // Draw Items
                for item in update.items.values() {
                    if let Some(tile_uuid) = self.get_tile_id_by_name(item.tile_name.clone()) {
                        if let Some(sampler_vec) = self.texture_sampler.get(&tile_uuid) {
                            let index = settings.anim_counter % sampler_vec.len();

                            let pos = ServerContext::map_grid_to_local(
                                screen_size,
                                vec2f(item.position.x, item.position.y),
                                &region.map,
                            );

                            drawer.add_textured_box(
                                pos.x as f32,
                                pos.y as f32,
                                grid_size,
                                grid_size,
                                [0.0, 0.0],
                                [1.0, 1.0],
                            );

                            drawer.draw_as_textured_triangles(&sampler_vec[index], true);
                        }
                    }
                }

                // Draw Characters
                for (pos, tile, _, _) in &update.characters_pixel_pos {
                    if let Some(sampler_vec) = self.texture_sampler.get(tile) {
                        let index = settings.anim_counter % sampler_vec.len();

                        let pos = ServerContext::map_grid_to_local(
                            screen_size,
                            vec2f(pos.x, pos.y),
                            &region.map,
                        );

                        drawer.add_textured_box(
                            pos.x as f32,
                            pos.y as f32,
                            grid_size,
                            grid_size,
                            [0.0, 0.0],
                            [1.0, 1.0],
                        );

                        drawer.draw_as_textured_triangles(&sampler_vec[index], true);
                    }
                }

                // Draw Vertices
                if server_ctx.curr_map_tool_type == MapToolType::Selection
                    || server_ctx.curr_map_tool_type == MapToolType::Vertex
                {
                    for vertex in &region.map.vertices {
                        let pos = ServerContext::map_grid_to_local(
                            screen_size,
                            vertex.as_vec2f(),
                            &region.map,
                        );

                        let color = if server_ctx.hover.0 == Some(vertex.id)
                            || region.map.selected_vertices.contains(&vertex.id)
                        {
                            [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0]
                        } else {
                            [0.5, 0.5, 0.5, 1.0]
                        };

                        let size = 4.0;
                        drawer.add_box(
                            pos.x - size,
                            pos.y - size,
                            size * 2.0,
                            size * 2.0,
                            Rgba::new(color[0], color[1], color[2], color[3]),
                        );
                    }
                }

                drawer.draw_as_triangles();
                drawer.blend_into(buffer);

                // Draw Lines
                if server_ctx.curr_map_tool_type == MapToolType::Selection
                    || server_ctx.curr_map_tool_type == MapToolType::Linedef
                    || server_ctx.curr_map_tool_type == MapToolType::Sector
                {
                    for linedef in &region.map.linedefs {
                        if let Some(start_vertex) = region.map.find_vertex(linedef.start_vertex) {
                            let start_pos = ServerContext::map_grid_to_local(
                                screen_size,
                                start_vertex.as_vec2f(),
                                &region.map,
                            );
                            if let Some(end_vertex) = region.map.find_vertex(linedef.end_vertex) {
                                let end_pos = ServerContext::map_grid_to_local(
                                    screen_size,
                                    end_vertex.as_vec2f(),
                                    &region.map,
                                );

                                let mut selected = false;
                                if server_ctx.hover.1 == Some(linedef.id)
                                    || region.map.selected_linedefs.contains(&linedef.id)
                                {
                                    selected = true;
                                } else if server_ctx.curr_map_tool_type == MapToolType::Sector
                                    || server_ctx.curr_map_tool_type == MapToolType::General
                                {
                                    // Check for sector selection when in sector mode.
                                    if let Some(front_sector) = linedef.front_sector {
                                        if let Some(sector) = region.map.find_sector(front_sector) {
                                            if server_ctx.hover.2 == Some(sector.id)
                                                || region.map.selected_sectors.contains(&sector.id)
                                            {
                                                selected = true;
                                            }
                                        }
                                    }
                                    if let Some(back_sector) = linedef.back_sector {
                                        if let Some(sector) = region.map.find_sector(back_sector) {
                                            if server_ctx.hover.2 == Some(sector.id)
                                                || region.map.selected_sectors.contains(&sector.id)
                                            {
                                                selected = true;
                                            }
                                        }
                                    }
                                }

                                #[allow(clippy::collapsible_else_if)]
                                let color = if selected {
                                    [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0]
                                } else {
                                    if region.map.is_linedef_in_closed_polygon(linedef.id) {
                                        [1.0, 1.0, 1.0, 1.0]
                                    } else {
                                        [0.6, 0.6, 0.6, 1.0]
                                    }
                                };

                                drawer.add_line(
                                    start_pos.x,
                                    start_pos.y,
                                    end_pos.x,
                                    end_pos.y,
                                    Rgba::new(color[0], color[1], color[2], color[3]),
                                );
                            }
                        }
                    }
                }

                // For line action previews
                if let Some(grid_pos) = region.map.curr_grid_pos {
                    let local =
                        ServerContext::map_grid_to_local(screen_size, grid_pos, &region.map);
                    if let Some(mouse_pos) = region.map.curr_mouse_pos {
                        drawer.add_line(local.x, local.y, mouse_pos.x, mouse_pos.y, Rgba::white());
                    }
                }

                // For rectangle selection preview
                if let Some(rect) = region.map.curr_rectangle {
                    drawer.add_line(rect.0.x, rect.0.y, rect.1.x, rect.0.y, Rgba::white());
                    drawer.add_line(rect.0.x, rect.0.y, rect.0.x, rect.1.y, Rgba::white());
                    drawer.add_line(rect.1.x, rect.1.y, rect.1.x, rect.0.y, Rgba::white());
                    drawer.add_line(rect.1.x, rect.1.y, rect.0.x, rect.1.y, Rgba::white());
                }

                drawer.draw_as_lines();

                // Hover Cursor
                if let Some(hover_pos) = server_ctx.hover_cursor {
                    let pos = ServerContext::map_grid_to_local(screen_size, hover_pos, &region.map);
                    let size = 4.0;
                    drawer.add_box(
                        pos.x - size,
                        pos.y - size,
                        size * 2.0,
                        size * 2.0,
                        Rgba::yellow(),
                    );
                    drawer.draw_as_triangles();
                }

                // Camera Pos
                if let Some(camera_pos) = region.map.camera_xz {
                    let pos =
                        ServerContext::map_grid_to_local(screen_size, camera_pos, &region.map);
                    let size = 4.0;
                    drawer.add_box(
                        pos.x - size,
                        pos.y - size,
                        size * 2.0,
                        size * 2.0,
                        Rgba::red(),
                    );
                    drawer.draw_as_triangles();
                }

                drawer.blend_into(buffer);
            } else {
                // Render in 3D
                //if region.map.camera == MapCamera::ThreeDIso {}

                buffer.fill(BLACK);

                let geo_map = generate_map_geometry(&region.map, self.atlas_size, &self.elements);
                //drawer.add_mesh(geo.vertices, geo.indices, geo.uvs);

                let (mvp, camera_pos) = if region.map.camera == MapCamera::ThreeDIso {
                    let scale = 2.0;
                    let aspect_ratio = width as f32 / height as f32;
                    let left = -scale * aspect_ratio;
                    let right = scale * aspect_ratio;
                    let bottom = -scale;
                    let top = scale;
                    let near = -100.0;
                    let far = 100.0;
                    let orthographic_planes = FrustumPlanes {
                        left,
                        right,
                        bottom,
                        top,
                        near,
                        far,
                    };
                    let projection = vek::Mat4::orthographic_rh_no(orthographic_planes);
                    let camera_pos = vek::Vec3::new(
                        region.editing_position_3d.x - 10.0,
                        region.editing_position_3d.y + 10.0,
                        region.editing_position_3d.z + 10.0,
                    );
                    let look_at = vek::Vec3::new(
                        region.editing_position_3d.x,
                        region.editing_position_3d.y,
                        region.editing_position_3d.z,
                    );
                    let up = vek::Vec3::new(0.0, 1.0, 0.0);
                    let view = vek::Mat4::look_at_rh(camera_pos, look_at, up);
                    (projection * view, camera_pos)
                } else {
                    let projection = vek::Mat4::perspective_fov_rh_no(
                        1.4,
                        width as f32,
                        height as f32,
                        0.01,
                        100.0,
                    );

                    let camera_pos = vek::Vec3::new(
                        region.editing_position_3d.x,
                        1.5,
                        region.editing_position_3d.z,
                    );

                    let look_at = vek::Vec3::new(
                        region.editing_position_3d.x,
                        1.5,
                        region.editing_position_3d.z - 1.0,
                    );

                    let view: vek::Mat4<f32> =
                        vek::Mat4::look_at_rh(camera_pos, look_at, vek::Vec3::new(0.0, 1.0, 0.0));
                    (projection * view, camera_pos)
                };

                for (id, geo_vec) in geo_map.geometries.iter() {
                    if let Some(sampler) = self.texture_sampler.get(id) {
                        for geo in geo_vec.iter() {
                            drawer.add_mesh(
                                geo.vertices.clone(),
                                geo.indices.clone(),
                                geo.uvs.clone(),
                            );
                        }
                        let index = settings.anim_counter % sampler.len();
                        drawer.draw_as_mesh(mvp, &sampler[index], false);
                    }
                }

                // Draw Characters via billboarding
                for (pos, tile, _, _) in &update.characters_pixel_pos {
                    if let Some(sampler_vec) = self.texture_sampler.get(tile) {
                        let index = settings.anim_counter % sampler_vec.len();

                        let sprite_position = vek::Vec3::new(pos.x, 0.5, pos.y);
                        let sprite_size = vek::Vec2::new(1.0, 1.0);

                        let direction = (sprite_position - camera_pos).normalized();
                        let flat_direction =
                            vek::Vec3::new(direction.x, 0.0, direction.z).normalized();

                        let angle = flat_direction.x.atan2(flat_direction.z);
                        let rotation_matrix = vek::Mat4::rotation_y(angle);

                        let half_width = sprite_size.x * 0.5;
                        let half_height = sprite_size.y * 0.5;

                        let quad_vertices = [
                            vek::Vec3::new(-half_width, -half_height, 0.0),
                            vek::Vec3::new(half_width, -half_height, 0.0),
                            vek::Vec3::new(-half_width, half_height, 0.0),
                            vek::Vec3::new(half_width, half_height, 0.0),
                        ];

                        let transformed_vertices: Vec<Vec3f> = quad_vertices
                            .iter()
                            .map(|v| {
                                let v4 = vek::Vec4::new(v.x, v.y, v.z, 1.0);
                                let rotated = rotation_matrix * v4;
                                let r = rotated.xyz() + sprite_position;
                                vec3f(r.x, r.y, r.z)
                            })
                            .collect();

                        drawer.add_mesh(
                            transformed_vertices,
                            vec![0, 1, 2, 2, 1, 3],
                            vec![
                                Vec2f::new(0.0, 1.0),
                                Vec2f::new(1.0, 1.0),
                                Vec2f::new(0.0, 0.0),
                                Vec2f::new(1.0, 0.0),
                            ],
                        );
                        drawer.draw_as_mesh(mvp, &sampler_vec[index], true);
                    }
                }
                drawer.copy_into(buffer);
            }
        } else {
            // No server ctx, we are live
            let mut drawer = EucDraw::new(width, height);

            if region.map.camera == MapCamera::TwoD {
                for sector in &region.map.sectors {
                    if let Some(geo) = sector.generate_geometry(&region.map) {
                        // Convert the triangles from grid to local coordinates
                        let mut vertices: Vec<Vec2f> = vec![];
                        let mut uvs: Vec<Vec2f> = vec![];
                        let bbox = sector.bounding_box(&region.map);

                        let repeat = true;

                        if let Some(floor_texture_id) = &sector.floor_texture {
                            if let Some(el) = self.elements.get(floor_texture_id) {
                                for vertex in &geo.0 {
                                    let local = ServerContext::map_grid_to_local(
                                        screen_size,
                                        vec2f(vertex[0], vertex[1]),
                                        &region.map,
                                    );

                                    // Scale up to polygon bbox
                                    if !repeat {
                                        let uv = vec2f(
                                            (el[0].x as f32
                                                + ((vertex[0] - bbox.0.x) / (bbox.1.x - bbox.0.x)
                                                    * el[0].z as f32))
                                                / self.atlas_size,
                                            (el[0].y as f32
                                                + (vertex[1] - bbox.0.y) / (bbox.1.y - bbox.0.y)
                                                    * el[0].w as f32)
                                                / self.atlas_size,
                                        );
                                        uvs.push(uv);
                                    } else {
                                        let texture_scale = 1.0;
                                        let uv = vec2f(
                                            (vertex[0] - bbox.0.x) / texture_scale,
                                            (vertex[1] - bbox.0.y) / texture_scale,
                                        );
                                        uvs.push(uv);
                                    }
                                    vertices.push(local);
                                }

                                drawer.add_textured_polygon(vertices, geo.1, uvs);
                                if !repeat {
                                    if let Some(sampler) = &self.sampler {
                                        drawer.draw_as_textured_triangles(sampler, false);
                                    }
                                } else if let Some(sampler_array) =
                                    self.texture_sampler.get(floor_texture_id)
                                {
                                    let index = settings.anim_counter % sampler_array.len();
                                    drawer.draw_as_textured_triangles(&sampler_array[index], false);
                                }
                            }
                        }
                    }
                }
            }

            drawer.copy_into(buffer);
        }
        let _stop = self.get_time();
        //println!("render time {:?}", _stop - _start);
    }

    /// Get the tile id of the given name.
    pub fn get_tile_id_by_name(&self, name: String) -> Option<Uuid> {
        for (id, tile) in &self.textures {
            if tile.name == name {
                return Some(*id);
            }
        }
        None
    }

    // Draw the grid
    fn grid_at(
        &self,
        position: Vec2f,
        size: Vec2f,
        grid_size: f32,
        offset: Vec2f,
        subdivisions: f32,
    ) -> Vec4f {
        fn odd(n: i32) -> bool {
            n % 2 != 0
        }

        // Return the multiple of delta closest to value
        fn closest_mul(delta: Vec2f, value: Vec2f) -> Vec2f {
            delta * round(value / delta)
        }

        // Return the distance of value to the closest multiple of delta
        fn mul_dist(delta: Vec2f, value: Vec2f) -> Vec2f {
            abs(value - closest_mul(delta, value))
        }

        // Align the given point to a pixel center if thickness is odd,
        // otherwise align the point to a crossing point between pixels
        fn align_pixel(point: Vec2f, thickness: i32) -> Vec2f {
            if odd(thickness) {
                round(point - Vec2f::new(0.5, 0.5)) + Vec2f::new(0.5, 0.5)
            } else {
                round(point)
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn draw_grid(
            position: Vec2f,
            origin: Vec2f,
            grid_size: Vec2f,
            sub_grid_div: Vec2f,
            thickness: i32,
            sub_thickness: i32,
            dot_radius: f32,
            squared_dots: bool,
            bg_color: Vec4f,
            line_color: Vec4f,
            sub_line_color: Vec4f,
            dots_color: Vec4f,
            x_axis_color: Vec4f,
            y_axis_color: Vec4f,
        ) -> Vec4f {
            let th = thickness as f32;
            let sth = sub_thickness as f32;

            let aligned_origin = align_pixel(origin, thickness);
            let rel_p = position - aligned_origin;

            // Draw the axes
            if abs(rel_p.y) < th * 0.5 {
                return x_axis_color;
            }
            if abs(rel_p.x) < th * 0.5 {
                return y_axis_color;
            }

            let mul = closest_mul(grid_size, rel_p);

            // Pixel distance
            let dist = mul_dist(grid_size, rel_p);

            if dot_radius > 0.0 {
                // Antialiasing threshold
                let aa = 1.0;

                let dot_dist = if squared_dots {
                    max(dist.x, dist.y)
                } else {
                    length(dist)
                };

                // Prevent dots from being drawn on the axes
                let draw_dots = abs(mul.x) > 0.5 && abs(mul.y) > 0.5;

                if draw_dots && dot_dist <= dot_radius + aa {
                    // Draw the dots
                    let val = max(dot_dist - dot_radius, 0.0) / aa;
                    return lerp(dots_color, bg_color, val);
                }
            }

            if min(dist.x, dist.y) <= th * 0.5 {
                return line_color;
            }

            let dist_to_floor = abs(rel_p - grid_size * floor(rel_p / grid_size));
            let sub_size = grid_size / round(sub_grid_div);

            let dist = if odd(thickness) != odd(sub_thickness) {
                abs(dist_to_floor - Vec2f::new(0.5, 0.5))
            } else {
                dist_to_floor
            };

            let sub_dist = mul_dist(sub_size, dist);

            // Number of columns and rows
            let rc = round(dist / sub_size);

            // Extra pixels for the last row/column
            let extra = grid_size - sub_size * sub_grid_div;

            let sub_dist = Vec2f::new(
                if rc.x == sub_grid_div.x {
                    sub_dist.x + extra.x
                } else {
                    sub_dist.x
                },
                if rc.y == sub_grid_div.y {
                    sub_dist.y + extra.y
                } else {
                    sub_dist.y
                },
            );

            if min(sub_dist.x, sub_dist.y) <= sth * 0.5 {
                return sub_line_color;
            }

            // Default to background color
            bg_color
        }

        let origin = size / 2.0 + offset; //vec2f(0.5, 0.5); //size + size / 2.0;
        let grid_size = Vec2f::new(grid_size, grid_size);
        let sub_grid_div = Vec2f::new(subdivisions, subdivisions);

        let thickness = 1;
        let sub_thickness = 1;
        let dot_radius = 0.0;
        let squared_dots = false;

        let bg_color = Vec4f::new(0.05, 0.05, 0.05, 1.0);
        let line_color = Vec4f::new(0.15, 0.15, 0.15, 1.0);
        let sub_line_color = Vec4f::new(0.1, 0.1, 0.1, 1.0);
        let dots_color = Vec4f::new(0.3, 0.3, 0.3, 1.0);
        let x_axis_color = Vec4f::new(212.0 / 255.0, 28.0 / 255.0, 15.0 / 255.0, 1.0);
        let y_axis_color = Vec4f::new(21.0 / 255.0, 191.0 / 255.0, 83.0 / 255.0, 1.0);

        draw_grid(
            position,
            origin,
            grid_size,
            sub_grid_div,
            thickness,
            sub_thickness,
            dot_radius,
            squared_dots,
            bg_color,
            line_color,
            sub_line_color,
            dots_color,
            x_axis_color,
            y_axis_color,
        )
    }

    /// Gets the current time in milliseconds
    fn get_time(&self) -> u128 {
        let time;
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            let t = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            time = t.as_millis();
        }
        #[cfg(target_arch = "wasm32")]
        {
            time = web_sys::window().unwrap().performance().unwrap().now() as u128;
        }
        time
    }
}
