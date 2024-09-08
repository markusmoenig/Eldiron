use crate::editor::TILEDRAWER;
use crate::prelude::*;

use rayon::prelude::*;

pub fn draw_minimap(region: &Region, buffer: &mut TheRGBABuffer, palette: &ThePalette) {
    // let background = *ui
    //     .style
    //     .theme()
    //     .color(TheThemeColors::DefaultWidgetDarkBackground);

    let background = BLACK;

    buffer.fill(background);
    let dim = buffer.dim();

    let width = dim.width as usize;

    let region_width = (region.width * region.grid_size) as usize;
    let region_height = region.height * region.grid_size;

    let minimap_width = dim.width;
    let minimap_height = dim.height;

    let scale_x = region_width as f32 / minimap_width as f32;
    let scale_y = region_height as f32 / minimap_height as f32;

    let tile_drawer = TILEDRAWER.lock().unwrap();

    let time = TheTime::default();

    let mut material_params: FxHashMap<Uuid, Vec<Vec<f32>>> = FxHashMap::default();
    for (id, material) in &tile_drawer.materials {
        let params = material.load_parameters(&time);
        material_params.insert(*id, params);
    }

    let grid_size = region.grid_size as f32;

    let pixels = buffer.pixels_mut();
    pixels
        .par_rchunks_exact_mut(width * 4)
        .enumerate()
        .for_each(|(j, line)| {
            for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                let i = j * minimap_width as usize + i;

                let x = ((i % minimap_width as usize) as f32 * scale_x) as i32;
                let y = ((minimap_height - (i / minimap_width as usize) as i32 - 1) as f32
                    * scale_y) as i32;

                let tile_x = x / region.grid_size;
                let tile_y = y / region.grid_size;

                let tile_x_f = x as f32 / region.grid_size as f32;
                let tile_y_f = y as f32 / region.grid_size as f32;

                let xx = x % region.grid_size;
                let yy = y % region.grid_size;

                let mut color = background;

                let mut has_hit = false;
                if let Some(mask) = region.heightmap.get_material_mask(tile_x, tile_y) {
                    let mut hit = Hit {
                        two_d: true,
                        ..Default::default()
                    };
                    let terrain_uv = vec2f(tile_x_f.fract(), tile_y_f.fract());

                    if let Some(material_mask) = mask.at_f(terrain_uv) {
                        let index = (material_mask[0] - 1) as usize;
                        if let Some((_id, material)) = tile_drawer.materials.get_index(index) {
                            let mut mat_obj_params: Vec<Vec<f32>> = vec![];

                            if let Some(m_params) = material_params.get(&material.id) {
                                mat_obj_params.clone_from(m_params);
                            }

                            hit.normal = vec3f(0.0, 1.0, 0.0);
                            hit.hit_point = vec3f(tile_x_f, 0.0, tile_y_f);

                            hit.uv = terrain_uv;
                            hit.global_uv = vec2f(tile_x_f, tile_y_f);
                            hit.pattern_pos = hit.global_uv;

                            if material.follow_geo_trail(&time, &mut hit, &mat_obj_params) {
                                if hit.interior_distance <= 0.01 {
                                    hit.value = 0.0;
                                } else {
                                    hit.value = 1.0;
                                }
                            }
                            material.compute(
                                &mut hit,
                                palette,
                                &tile_drawer.tiles,
                                &mat_obj_params,
                            );

                            color = TheColor::from_vec3f(hit.mat.base_color).to_u8_array();
                            has_hit = true;
                        }

                        // Overlay the 2nd material
                        if has_hit {
                            let index = (material_mask[1] - 1) as usize;
                            if let Some((_id, material)) = tile_drawer.materials.get_index(index) {
                                let mut mat_obj_params: Vec<Vec<f32>> = vec![];

                                if let Some(m_params) = material_params.get(&material.id) {
                                    mat_obj_params.clone_from(m_params);
                                }

                                //let mut h = hit.clone();
                                material.compute(
                                    &mut hit,
                                    palette,
                                    &tile_drawer.tiles,
                                    &mat_obj_params,
                                );
                                color = TheColor::from_vec3f(hit.mat.base_color).to_u8_array();
                            }
                        }
                    }
                }

                // Tiles
                if let Some(tile) = region.tiles.get(&(tile_x, tile_y)) {
                    for tile_index in 0..tile.layers.len() {
                        if let Some(tile_uuid) = tile.layers[tile_index] {
                            if let Some(data) = tile_drawer.tiles.get(&tile_uuid) {
                                let index = 0;

                                if let Some(c) = data.buffer[index].at(vec2i(xx, yy)) {
                                    color = tile_drawer.mix_color(&color, &c, c[3] as f32 / 255.0);
                                }
                            }
                        }
                    }
                }

                let p = vec2f(x as f32, y as f32);
                let mut hit = Hit {
                    global_uv: vec2f(
                        tile_x as f32 + xx as f32 / region.grid_size as f32,
                        tile_y as f32 + yy as f32 / region.grid_size as f32,
                    ),
                    uv: vec2f(
                        xx as f32 / region.grid_size as f32,
                        yy as f32 / region.grid_size as f32,
                    ),
                    two_d: true,
                    ..Default::default()
                };

                if let Some(geo_ids) = region.geometry_areas.get(&vec3i(tile_x, 0, tile_y)) {
                    let mut ground_dist = f32::INFINITY;
                    let mut wall_dist = f32::INFINITY;
                    let mut had_wall = false;
                    for geo_id in geo_ids {
                        if let Some(geo_obj) = region.geometry.get(geo_id) {
                            // We have to make sure walls have priority
                            let mut is_legit = false;
                            let d = geo_obj.distance(&TheTime::default(), p, grid_size, &mut None);
                            if d.0 < 0.0 {
                                let role = geo_obj.get_layer_role();
                                if role == Some(Layer2DRole::Ground)
                                    && d.0 < ground_dist
                                    && d.0 < hit.distance
                                    && !had_wall
                                {
                                    is_legit = true;
                                    ground_dist = d.0;
                                } else if role == Some(Layer2DRole::Wall) {
                                    is_legit = true;
                                    had_wall = true;
                                    wall_dist = d.0;
                                }
                            }
                            if is_legit {
                                let mut c = WHITE;

                                hit.mat.base_color = vec3f(0.5, 0.5, 0.5);
                                hit.value = 1.0;

                                hit.distance = min(ground_dist, wall_dist);

                                if let Some(material) =
                                    tile_drawer.materials.get(&geo_obj.material_id)
                                {
                                    hit.normal = vec3f(0.0, 1.0, 0.0);
                                    hit.hit_point = vec3f(p.x, 0.0, p.y);

                                    let mut mat_obj_params: Vec<Vec<f32>> = vec![];

                                    if let Some(m_params) =
                                        material_params.get(&geo_obj.material_id)
                                    {
                                        mat_obj_params.clone_from(m_params);
                                    }

                                    material.get_distance(
                                        &TheTime::default(),
                                        p / grid_size,
                                        &mut hit,
                                        geo_obj,
                                        grid_size,
                                        &mat_obj_params,
                                    );

                                    if material.test_height_profile(
                                        &mut hit,
                                        geo_obj,
                                        &mat_obj_params,
                                    ) {
                                        material.compute(
                                            &mut hit,
                                            palette,
                                            &tile_drawer.tiles,
                                            &mat_obj_params,
                                        );

                                        let col =
                                            TheColor::from_vec3f(hit.mat.base_color).to_u8_array();
                                        c = col;
                                    }
                                } else {
                                    let col =
                                        TheColor::from_vec3f(hit.mat.base_color).to_u8_array();
                                    c = col;
                                }
                                //     let t = smoothstep(-1.0, 0.0, d.0);
                                //     color = self.mix_color(&c, &color, t);
                                color = c;
                            }
                        }
                    }
                }

                pixel.copy_from_slice(&color);
            }
        });
}
