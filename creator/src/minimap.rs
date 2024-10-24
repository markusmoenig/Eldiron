use crate::editor::TILEDRAWER;
use crate::prelude::*;

use rayon::prelude::*;

pub fn draw_minimap(region: &Region, buffer: &mut TheRGBABuffer, lighting: bool) {
    // let background = *ui
    //     .style
    //     .theme()
    //     .color(TheThemeColors::DefaultWidgetDarkBackground);

    // pub fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
    //     [
    //         (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
    //         (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
    //         (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
    //         255,
    //     ]
    // }

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

    //let grid_size = region.grid_size as f32;

    // let mut render_mode = 0;

    // if lighting {
    //     if let Some(value) = region.regionfx.get_render_settings().get("2D Renderer") {
    //         if let Some(value) = value.to_i32() {
    //             render_mode = value;
    //         }
    //     }
    // }

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

                let xx = x % region.grid_size;
                let yy = y % region.grid_size;

                let mut color = background;
                if let Some(mask) = region.heightmap.get_material_mask(tile_x, tile_y) {
                    if let Some(material_mask) = mask.at(vec2i(xx, yy)) {
                        color[0] = material_mask[0];
                        color[1] = material_mask[1];
                        color[2] = material_mask[2];

                        // Lambertian shading
                        if lighting {
                            //&& render_mode == 1 {
                            let tile_x_f = x as f32 / region.grid_size as f32;
                            let tile_y_f = y as f32 / region.grid_size as f32;

                            let light_dir = vec3f(-0.24192198, 0.9702957, 0.0); //normalize(vec3f(1.0, 1.0, -1.0));

                            let normal = region.heightmap.calculate_normal_with_material(
                                vec3f(tile_x_f, 0.0, tile_y_f),
                                0.1,
                            );

                            let intensity = dot(normal, light_dir).max(0.0);

                            color[0] = (((color[0] as f32 / 255.0) * intensity) * 255.0) as u8;
                            color[1] = (((color[1] as f32 / 255.0) * intensity) * 255.0) as u8;
                            color[2] = (((color[2] as f32 / 255.0) * intensity) * 255.0) as u8;
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

                pixel.copy_from_slice(&color);
            }
        });
}
