use crate::prelude::*;
use rand::Rng;
use rayon::prelude::*;
use theframework::prelude::*;

/// Settings for the region draw operation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionDrawSettings {
    pub anim_counter: usize,
    pub delta_in_tick: f32,
    pub offset: Vec2<i32>,
    pub display_offset: Vec2<i32>,
    pub delta: f32,

    pub daylight: Vec3<f32>,
    pub daylight_intensity: f32,
    pub sun_direction: Vec3<f32>,

    pub show_fx_marker: bool,

    pub center_3d: Vec3<f32>,
    pub facing_3d: Vec3<f32>,

    pub pbr: bool,

    pub time: TheTime,
    pub center_on_character: Option<Uuid>,

    pub conceptual_display: Option<f32>,
    pub curr_geo_object: Option<Uuid>,
    pub curr_geo_node: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl RegionDrawSettings {
    pub fn new() -> Self {
        Self {
            anim_counter: 0,
            delta_in_tick: 0.0,
            offset: Vec2::zero(),
            display_offset: Vec2::zero(),
            delta: 0.0,

            daylight: Vec3::one(),
            daylight_intensity: 0.0,
            sun_direction: Vec3::zero(),

            show_fx_marker: false,

            center_3d: Vec3::zero(),
            facing_3d: Vec3::zero(),

            pbr: false,

            time: TheTime::default(),
            center_on_character: None,

            conceptual_display: None,
            curr_geo_object: None,
            curr_geo_node: None,
        }
    }
}

pub struct TileDrawer {
    pub tiles: FxHashMap<Uuid, TheRGBATile>,
    pub materials: IndexMap<Uuid, MaterialFXObject>,
}

#[allow(clippy::new_without_default)]
impl TileDrawer {
    pub fn new() -> Self {
        Self {
            tiles: FxHashMap::default(),
            materials: IndexMap::default(),
        }
    }

    /// Set the tiles.
    pub fn set_tiles(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        self.tiles = tiles;
    }

    /// Set the materials.
    pub fn set_materials(&mut self, materials: IndexMap<Uuid, MaterialFXObject>) {
        self.materials = materials;
    }

    /// Get the tile which best fits the tags.
    pub fn get_tile_by_tags(&self, role: u8, tags: &str) -> Option<TheValue> {
        let mut best_fit: Option<Uuid> = None;
        let tags: Vec<&str> = tags.split(',').map(|tag| tag.trim()).collect();
        let mut best_fit_count = 0;

        for (id, tile) in self.tiles.iter() {
            if role > 0 && tile.role != role - 1 {
                continue;
            }

            let name = tile.name.to_lowercase();
            let tile_tags: Vec<&str> = name.split(',').map(|tag| tag.trim()).collect();
            let mut match_count = 0;
            for tag in tags.iter() {
                if tile_tags.contains(tag) {
                    match_count += 1;
                }
            }
            if match_count == tags.len() {
                return Some(TheValue::Tile(tile.name.clone(), tile.id));
            }
            if match_count > best_fit_count {
                best_fit = Some(*id);
                best_fit_count = match_count;
            }
        }

        if let Some(id) = best_fit {
            if let Some(tile) = self.tiles.get(&id) {
                return Some(TheValue::Tile(tile.name.clone(), tile.id));
            }
        }

        None
    }

    /// Draw the region
    pub fn draw_region(
        &self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        update: &mut RegionUpdate,
        settings: &mut RegionDrawSettings,
        compute_delta: bool,
        palette: &ThePalette,
    ) {
        /*
        // pub fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
        //     [
        //         (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
        //         (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
        //         (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
        //         255,
        //     ]
        // }

        let _start = self.get_time();

        let server_tick = update.server_tick;

        let width = buffer.dim().width as usize;

        let region_width = (region.width * region.grid_size) as usize;
        let region_height = region.height * region.grid_size;

        let grid_size = region.grid_size as f32;

        if compute_delta {
            // Collect the character positions.
            update.generate_character_pixel_positions(
                grid_size,
                &self.tiles,
                vec2i(width as i32, buffer.dim().height),
                region_height,
                settings,
            );
        }

        let mut offset = if settings.display_offset != Vec2i::zero() {
            settings.display_offset
        } else {
            settings.offset
        };
        if region_height == buffer.dim().height {
            offset = Vec2i::zero();
        }

        // Fill the code level with the blocking info and collect lights
        let mut level = Level::new(region.width, region.height, settings.time);
        region.fill_code_level(&mut level, &self.tiles, update, region);

        // Collect the material params
        let mut material_params: FxHashMap<Uuid, Vec<Vec<f32>>> = FxHashMap::default();
        for (id, material) in &self.materials {
            let params = material.load_parameters(&settings.time);
            material_params.insert(*id, params);
        }

        // Collect the regionfx params
        let regionfx_params: Vec<Vec<f32>> = region.regionfx.load_parameters(&settings.time);

        // Collect the tilefx params
        let mut tilefx_params: FxHashMap<Vec3i, Vec<Vec<f32>>> = FxHashMap::default();
        for (pos, tilefx) in &region.effects {
            let params = tilefx.load_parameters(&settings.time);
            tilefx_params.insert(*pos, params);
        }

        let mut render_mode = 0;

        if let Some(value) = region.regionfx.get_render_settings().get("2D Renderer") {
            if let Some(value) = value.to_i32() {
                render_mode = value;
            }
        }

        let pixels = buffer.pixels_mut();
        pixels
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = (j + offset.y as usize) * region_width + i + offset.x as usize;

                    let x = (i % region_width) as i32;
                    let y = region_height - (i / region_width) as i32 - 1;

                    let tile_x = x / region.grid_size;
                    let tile_y = y / region.grid_size;

                    let tile_x_f = x as f32 / region.grid_size as f32;
                    let tile_y_f = y as f32 / region.grid_size as f32;

                    let mut xx = x % region.grid_size;
                    let mut yy = y % region.grid_size;

                    let mut color = BLACK;

                    // let mut daylight = settings.daylight;
                    let mut show_fx_marker = false;

                    // let mut mirror: Option<(i32, i32)> = None;

                    if let Some(mask) = region.heightmap.get_material_mask(tile_x, tile_y) {
                        if let Some(material_mask) = mask.at(vec2i(xx, yy)) {
                            color[0] = material_mask[0];
                            color[1] = material_mask[1];
                            color[2] = material_mask[2];
                            color[3] = 255;

                            // Lambertian shading
                            if render_mode == 1 {
                                let normal = region.heightmap.calculate_normal_with_material(
                                    vec3f(tile_x_f, 0.0, tile_y_f),
                                    0.1,
                                );

                                let intensity = dot(normal, settings.sun_direction).max(0.0);

                                color[0] = (((color[0] as f32 / 255.0) * intensity) * 255.0) as u8;
                                color[1] = (((color[1] as f32 / 255.0) * intensity) * 255.0) as u8;
                                color[2] = (((color[2] as f32 / 255.0) * intensity) * 255.0) as u8;
                            }
                        }
                    }

                    if let Some(tile) = region.tiles.get(&(tile_x, tile_y)) {
                        for tile_index in 0..tile.layers.len() {
                            if let Some(tile_uuid) = tile.layers[tile_index] {
                                if let Some(data) = self.tiles.get(&tile_uuid) {
                                    let index = settings.anim_counter % data.buffer.len();

                                    if tile_index == Layer2DRole::Wall as usize {
                                        let mut alpha: f32 = 1.0;

                                        let mut valid = true;
                                        if let Some(wallfx) = update.wallfx.get(&(tile_x, tile_y)) {
                                            let d = (server_tick - wallfx.at_tick) as f32
                                                + settings.delta_in_tick
                                                - 1.0;
                                            if d < 1.0 {
                                                let t = (d * region.grid_size as f32) as i32;
                                                if wallfx.prev_fx != WallFX::Normal {
                                                    wallfx.prev_fx.apply(
                                                        &mut xx,
                                                        &mut yy,
                                                        &mut alpha,
                                                        &(region.grid_size - t),
                                                        &(1.0 - d),
                                                    );
                                                } else {
                                                    wallfx.fx.apply(
                                                        &mut xx, &mut yy, &mut alpha, &t, &d,
                                                    );
                                                }
                                            } else if wallfx.fx != WallFX::Normal {
                                                valid = false;
                                            }
                                        }

                                        if valid {
                                            if let Some(c) = data.buffer[index].at(vec2i(xx, yy)) {
                                                let wall_alpha = c[3] as f32 / 255.0;
                                                color =
                                                    self.mix_color(&color, &c, wall_alpha * alpha);
                                            }
                                        }
                                    } else if let Some(c) = data.buffer[index].at(vec2i(xx, yy)) {
                                        color = self.mix_color(&color, &c, c[3] as f32 / 255.0);
                                    }
                                }
                            }
                        }

                        // Show orange FX marker
                        if settings.show_fx_marker
                            && region.effects.contains_key(&vec3i(tile_x, 0, tile_y))
                        {
                            show_fx_marker = true;
                        }

                        // Check for FX
                        /*
                        if let Some(tilefx) = &tile.tilefx {
                            if let Some(v) = tilefx.get(
                                str!("Daylight"),
                                str!("Attenuation"),
                                &settings.time,
                                TheInterpolation::Linear,
                            ) {
                                if let Some(value) = v.to_f32() {
                                    if let Some(TheValue::TileMask(tile)) = tilefx.get(
                                        str!("Daylight"),
                                        str!("Mask"),
                                        &settings.time,
                                        TheInterpolation::Linear,
                                    ) {
                                        if tile.contains(vec2i(xx, yy)) {
                                            color[0] += ((daylight.x * value) * 255.0) as u8;
                                            color[1] += ((daylight.y * value) * 255.0) as u8;
                                            color[2] += ((daylight.z * value) * 255.0) as u8;
                                            color[3] = 255;
                                        }
                                    }
                                }
                            }
                            if let Some(v) = tilefx.get(
                                str!("Brightness"),
                                str!("Brightness"),
                                &settings.time,
                                TheInterpolation::Linear,
                            ) {
                                if let Some(brightness) = v.to_f32() {
                                    if let Some(TheValue::TileMask(tile)) = tilefx.get(
                                        str!("Brightness"),
                                        str!("Mask"),
                                        &settings.time,
                                        TheInterpolation::Linear,
                                    ) {
                                        if tile.is_empty() || tile.contains(vec2i(xx, yy)) {
                                            daylight *= brightness;
                                        }
                                    } else {
                                        daylight *= brightness;
                                    }
                                }
                            }
                            if let Some(TheValue::IntRange(range, _)) = tilefx.get(
                                str!("Mirror"),
                                str!("Range"),
                                &settings.time,
                                TheInterpolation::Linear,
                            ) {
                                if let Some(TheValue::TextList(index, _)) = tilefx.get(
                                    str!("Mirror"),
                                    str!("Direction"),
                                    &settings.time,
                                    TheInterpolation::Linear,
                                ) {
                                    if let Some(TheValue::TileMask(tile)) = tilefx.get(
                                        str!("Mirror"),
                                        str!("Mask"),
                                        &settings.time,
                                        TheInterpolation::Linear,
                                    ) {
                                        if tile.is_empty() || tile.contains(vec2i(xx, yy)) {
                                            mirror = Some((range, index));
                                        }
                                    }
                                }
                            }
                        }*/
                    }

                    let p = vec2f(x as f32, y as f32);
                    /*
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
                    */

                    if let Some(geo_ids) = region.geometry_areas.get(&vec3i(tile_x, 0, tile_y)) {
                        for geo_id in geo_ids {
                            let mut pos = Vec2f::zero();

                            let mut is_vertical = false;
                            if let Some(geo_obj) = region.geometry.get(geo_id) {
                                pos = geo_obj.get_position_2d();
                                is_vertical = geo_obj.is_vertical();
                            }
                            let p = if !is_vertical {
                                vec2f(
                                    tile_x as f32 - pos.x
                                        + 0.5
                                        + xx as f32 / region.grid_size as f32,
                                    tile_y as f32 - pos.y + 1.0
                                        - yy as f32 / region.grid_size as f32,
                                )
                            } else {
                                vec2f(
                                    tile_y as f32 - pos.y +// 1.0
                                        yy as f32 / region.grid_size as f32,
                                    tile_x as f32 - pos.x + 1.0
                                        - xx as f32 / region.grid_size as f32,
                                )
                            };

                            if let Some(ft_ctx) = region.compiled_geometry.get(geo_id) {
                                if let Some(col) = ft_ctx.face_pixel_at(p) {
                                    color = col;
                                } else {
                                    //color = BLACK;
                                }
                            }
                        }
                    }

                    // Items
                    for item in update.items.values() {
                        if tile_x == item.position.x as i32 && tile_y == item.position.y as i32 {
                            if let Some(tile_uuid) =
                                self.get_tile_id_by_name(item.tile_name.clone())
                            {
                                if let Some(data) = self.tiles.get(&tile_uuid) {
                                    let index = settings.anim_counter % data.buffer.len();

                                    if let Some(c) = data.buffer[index].at(vec2i(xx, yy)) {
                                        color = self.mix_color(&color, &c, c[3] as f32 / 255.0);
                                    }
                                }
                            }
                        }
                    }

                    // Characters
                    /*
                    for (pos, tile, _, _) in &update.characters_pixel_pos {
                        if let Some(data) = self.tiles.get(tile) {
                            let index = settings.anim_counter % data.buffer.len();

                            let xx = x - pos.x;
                            let yy = y - pos.y;

                            if let Some(c) = data.buffer[index].at(vec2i(xx, yy)) {
                                color = self.mix_color(&color, &c, c[3] as f32 / 255.0);
                            }
                        }
                    }*/

                    // self.render(
                    //     vec2i(x, y),
                    //     &mut color,
                    //     region,
                    //     update,
                    //     &level,
                    //     daylight,
                    //     settings,
                    //     mirror,
                    // );

                    let mut col = TheColor::from(color).to_vec3f();

                    // If no lights apply world brightness
                    if level.lights.is_empty() {
                        col *= settings.daylight;
                    } else {
                        let ro = Vec2f::from(p) / region.grid_size as f32;

                        // Sample the lights
                        let mut total_light = Vec3f::new(0.0, 0.0, 0.0);
                        for (light_grid, light) in &level.lights {
                            let light_pos = Vec2f::from(*light_grid) + vec2f(0.5, 0.5);
                            let mut light_strength = light.strength;

                            if light.color_type == 1 {
                                light_strength = settings.daylight.x;
                            }

                            if light.limiter == 1 && ro.y > light_pos.y {
                                continue;
                            }
                            if light.limiter == 2 && ro.x < light_pos.x {
                                continue;
                            }
                            if light.limiter == 3 && ro.y < light_pos.y {
                                continue;
                            }
                            if light.limiter == 4 && ro.x > light_pos.x {
                                continue;
                            }

                            let offsets = [
                                ro,
                                ro - vec2f(0.0, light.sampling_offset),
                                ro - vec2f(light.sampling_offset, 0.0),
                                ro + vec2f(light.sampling_offset, 0.0),
                                ro + vec2f(0.0, light.sampling_offset),
                                ro - vec2f(light.sampling_offset, light.sampling_offset),
                                ro + vec2f(light.sampling_offset, light.sampling_offset),
                            ];

                            for s in offsets.iter().take(light.samples) {
                                let ro = s;

                                let mut light_dir = light_pos - ro;
                                let light_dist = length(light_dir);

                                if light_dist < light.max_distance {
                                    light_dir = normalize(light_dir);

                                    let mut t = 0.0;
                                    let max_t = light_dist;

                                    let mut hit = false;

                                    while t < max_t {
                                        let pos = ro + light_dir * t;
                                        let tile = vec2i(pos.x as i32, pos.y as i32);

                                        if tile == *light_grid {
                                            hit = true;
                                            break;
                                        }
                                        if level.is_blocking((tile.x, tile.y)) {
                                            hit = false;
                                            break;
                                        }

                                        t += 1.0 / 4.0;
                                    }

                                    if hit {
                                        let intensity =
                                            1.0 - (max_t / light.max_distance).clamp(0.0, 1.0);
                                        //intensity *= if s == 0 { 2.0 } else { 1.0 };
                                        let mut light_color = Vec3f::from(
                                            intensity * light_strength / light.samples as f32,
                                        );
                                        if light.color_type == 0 {
                                            light_color *= light.color
                                        }
                                        total_light += light_color;
                                    }
                                }
                            }
                        }

                        col = clamp(
                            col * settings.daylight + col * total_light,
                            col * settings.daylight,
                            col,
                        );
                    }

                    if let Some(tilefx) = region.effects.get(&vec3i(tile_x, 0, tile_y)) {
                        // Execute Effects
                        if let Some(params) = tilefx_params.get(&vec3i(tile_x, 0, tile_y)) {
                            tilefx.fx(
                                region,
                                palette,
                                vec3f(tile_x_f, tile_y_f, 0.0),
                                &mut col,
                                false,
                                params,
                            );
                        }
                    }

                    // RegionFX
                    region.regionfx.fx_2d(
                        region,
                        palette,
                        vec2i(tile_x, tile_y),
                        &mut col,
                        &regionfx_params,
                    );

                    color = TheColor::from_vec3f(col).to_u8_array();

                    // Show the fx marker if necessary
                    if show_fx_marker {
                        let triangle_size = 4;
                        if xx < triangle_size && yy < triangle_size && yy < triangle_size - xx {
                            color[0] = 212;
                            color[1] = 128;
                            color[2] = 77;
                            color[3] = 255;
                        }
                    }

                    pixel.copy_from_slice(&color);
                }
            });

        let _stop = self.get_time();
        // println!("draw time {:?}", _stop - _start);
        */
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    /// Sample the lights and apply all TileFX for the pixel.
    pub fn render(
        &self,
        p: Vec2<i32>,
        c: &mut [u8; 4],
        region: &Region,
        update: &RegionUpdate,
        level: &Level,
        daylight: Vec3<f32>,
        settings: &RegionDrawSettings,
        mirror: Option<(i32, i32)>,
    ) {
        //let mut rng = rand::thread_rng();
        /*
        let mut color = TheColor::from_u8_array(*c).to_vec3f();

        let ro = Vec2f::from(p) / region.grid_size as f32;
        let roi = Vec2i::from(ro);

        let grid_size = region.grid_size as f32;

        // Apply mirror
        if let Some((range, direction)) = mirror {
            let d = match direction {
                0 => Vec2f::new(0.0, -1.0),
                1 => Vec2f::new(1.0, 0.0),
                2 => Vec2f::new(0.0, 1.0),
                _ => Vec2f::new(-1.0, 0.0),
            };

            let mut t = 0.0;
            let max_t = range as f32;
            let mut hit = false;

            while !hit && t < max_t {
                let pos = ro + d * t;
                let tile = vec2i(pos.x as i32, pos.y as i32);

                // Characters
                for (_character_pos, tile_id, _, _) in &update.characters_pixel_pos {
                    if let Some(data) = self.tiles.get(tile_id) {
                        let index = settings.anim_counter % data.buffer.len();

                        let w = data.buffer[index].dim().width;
                        let h = data.buffer[index].dim().height;

                        let mut xx = (pos.x * grid_size) as i32; // - character_pos.x;
                        let mut yy = (pos.y * grid_size) as i32; // - character_pos.y;

                        if xx >= 0 && xx < w && yy >= 0 && yy < h {
                            if direction == 0 || direction == 2 {
                                yy = p.y % h;
                            } else {
                                xx = p.x % w;
                            }

                            if let Some(c) = data.buffer[index].at(vec2i(xx, yy)) {
                                color = TheColor::from_u8_array(c).to_vec3f();
                                hit = true;
                                break;
                            }
                        }
                    }
                }
                if !hit && roi != tile && level.is_blocking((tile.x, tile.y)) {
                    if let Some(tile) = region.tiles.get(&(tile.x, tile.y)) {
                        if let Some(wall) = tile.layers[1] {
                            if let Some(wall_tile) = self.tiles.get(&wall) {
                                let index = settings.anim_counter % wall_tile.buffer.len();
                                let xx = p.x as f32 % grid_size;
                                let yy = p.y as f32 % grid_size;

                                if let Some(c) =
                                    wall_tile.buffer[index].at(vec2i(xx as i32, yy as i32))
                                {
                                    let p = TheColor::from_u8_array(c).to_vec3f();
                                    color = p;
                                    if c[3] == 255 {
                                        hit = true;
                                    }
                                }
                            }
                        }
                    }
                }

                t += 1.0;
            }
        }

        // If no lights apply world brightness
        if level.lights.is_empty() {
            color *= daylight;
        } else {
            // Sample the lights
            let mut total_light = Vec3f::new(0.0, 0.0, 0.0);
            for (light_grid, light) in &level.lights {
                let light_pos = Vec2f::from(*light_grid) + vec2f(0.5, 0.5);
                let mut light_strength = light.strength;

                if light.color_type == 1 {
                    light_strength = daylight.x;
                }

                if light.limiter == 1 && ro.y > light_pos.y {
                    continue;
                }
                if light.limiter == 2 && ro.x < light_pos.x {
                    continue;
                }
                if light.limiter == 3 && ro.y < light_pos.y {
                    continue;
                }
                if light.limiter == 4 && ro.x > light_pos.x {
                    continue;
                }

                let offsets = [
                    ro,
                    ro - vec2f(0.0, light.sampling_offset),
                    ro - vec2f(light.sampling_offset, 0.0),
                    ro + vec2f(light.sampling_offset, 0.0),
                    ro + vec2f(0.0, light.sampling_offset),
                    ro - vec2f(light.sampling_offset, light.sampling_offset),
                    ro + vec2f(light.sampling_offset, light.sampling_offset),
                ];

                for s in offsets.iter().take(light.samples) {
                    let ro = s;

                    let mut light_dir = light_pos - ro;
                    let light_dist = length(light_dir);

                    if light_dist < light.max_distance {
                        light_dir = normalize(light_dir);

                        let mut t = 0.0;
                        let max_t = light_dist;

                        let mut hit = false;

                        while t < max_t {
                            let pos = ro + light_dir * t;
                            let tile = vec2i(pos.x as i32, pos.y as i32);

                            if tile == *light_grid {
                                hit = true;
                                break;
                            }
                            if level.is_blocking((tile.x, tile.y)) {
                                hit = false;
                                break;
                            }

                            t += 1.0 / 4.0;
                        }

                        if hit {
                            let intensity = 1.0 - (max_t / light.max_distance).clamp(0.0, 1.0);
                            //intensity *= if s == 0 { 2.0 } else { 1.0 };
                            let mut light_color =
                                Vec3f::from(intensity * light_strength / light.samples as f32);
                            if light.color_type == 0 {
                                light_color *= light.color
                            }
                            total_light += light_color;
                        }
                    }
                }
            }

            color = clamp(
                color * daylight + color * total_light,
                color * daylight,
                color,
            );
        }

        /*
        let dx = target.x - self.x;
        let dy = target.y - self.y;
        let angle_to_target = (dy as f32).atan2(dx as f32);

        // Add a random angle within the specified randomness range
        let random_angle = rng.gen_range(-randomness..randomness);
        let final_angle = angle_to_target + random_angle;

        (final_angle.cos(), final_angle.sin())

        if let Some(t) = self.ray_casting(Vec2f::from(ro), rd, 24.0, vec2i(39, 33), &level) {}
        */
        /* 2D Pathtracer
        let spp = 32;
        let bounces = 5;

        let mut tot = Vec3f::zero();

        // Samples
        for _ in 0..spp {
            // Render

            let mut col = vec3f(1.0, 1.0, 1.0);
            let mut ro = Vec2f::from(p);
            let mut rd = self.uniform_vector(&mut rng);

            // Bounces
            let mut bounce_hit = false;
            for _ in 0..bounces {
                if let Some((t, hit, normal)) = self.ray_casting(ro, rd, 24.0, &level) {
                    let p = Vec2f::from(hit);
                    bounce_hit = true;

                    let mat_col = vec3f(0.5, 0.5, 0.5); // TODO Get pixel from tile

                    if hit == light {
                        break;
                    }
                    col *= mat_col;

                    ro = p + normal * 0.001; // new ray origin
                                             //rd = reflect(rd, normal); // new ray direction
                                             //I - 2.0 * dot(N, I) * N
                    rd = p - 2.0 * dot(normal, p) * normal;
                }
            }
            if bounce_hit {
                tot += col;
            }
        }
        tot /= spp as f32 / 4.0;

        color = clamp(color * tot, color * world_brightness, color);
        */

        /*
        if region.distance(p / region.grid_size, light) <= 10.0 {
            let mut c = 0.0_f32;
            // Samples
            for _ in 0..32 {


                let rd = self.uniform_vector(&mut rng);
                if let Some(t) = self.ray_casting(Vec2f::from(ro), rd, 24.0, vec2i(39, 33), &level) {

                }
            }
            c /= 8.0;
            c = c.clamp(world_brightness, 1.0);

            color *= c;
            */
        //}

        c[0] = (color.x * 255.0) as u8;
        c[1] = (color.y * 255.0) as u8;
        c[2] = (color.z * 255.0) as u8;
        */
    }

    pub fn get_tile(&self, tile: &Uuid) -> Option<&TheRGBATile> {
        self.tiles.get(tile)
    }

    pub fn draw_tile(
        &self,
        at: Vec2<i32>,
        buffer: &mut TheRGBABuffer,
        grid: i32,
        tile: Uuid,
        anim_counter: &usize,
    ) -> bool {
        if let Some(data) = self.tiles.get(&tile) {
            let x = (at.x * grid) as usize;
            let y = (at.y * grid) as usize;
            let stride = buffer.stride();
            self.blend_slice(
                buffer,
                data.buffer[anim_counter % data.buffer.len()].pixels(),
                &(x, y, grid as usize, grid as usize),
                stride,
            );
            true
        } else {
            false
        }
    }

    /*
    pub fn draw_tile_at_pixel(
        &self,
        at: Vec2i,
        buffer: &mut TheRGBABuffer,
        tile: Uuid,
        anim_counter: &usize,
    ) -> bool {
        if let Some(data) = self.tiles.get(&tile) {
            let off = anim_counter % data.buffer.len();
            /*
            let stride = buffer.stride();
            self.blend_slice(
                buffer,
                data.buffer[off].pixels(),
                &(
                    x,
                    y,
                    data.buffer[off].dim().width as usize,
                    data.buffer[off].dim().height as usize,
                ),
                stride,
            );*/
            buffer.blend_into(at.x, at.y, &data.buffer[off]);
            true
        } else {
            false
        }
    }

    pub fn draw_tile_outline(
        &self,
        at: Vec2i,
        buffer: &mut TheRGBABuffer,
        grid: i32,
        color: [u8; 4],
        ctx: &mut TheContext,
    ) {
        let x = (at.x * grid) as usize;
        let y = (at.y * grid) as usize;
        let stride = buffer.stride();
        ctx.draw
            .rect_outline(buffer.pixels_mut(), &(x, y, 24, 24), stride, &color);
    }*/

    pub fn draw_tile_outline_at_pixel(
        &self,
        at: Vec2<i32>,
        buffer: &mut TheRGBABuffer,
        color: [u8; 4],
        ctx: &mut TheContext,
    ) {
        let x = at.x as usize;
        let y = at.y as usize;
        let stride = buffer.stride();
        ctx.draw
            .rect_outline(buffer.pixels_mut(), &(x, y, 24, 24), stride, &color);
    }

    /// Get the tile id of the given name.
    pub fn get_tile_id_by_name(&self, name: String) -> Option<Uuid> {
        for (id, tile) in &self.tiles {
            if tile.name == name {
                return Some(*id);
            }
        }
        None
    }

    pub fn draw_tile_selection(
        &self,
        tiles: &FxHashSet<(i32, i32)>,
        buffer: &mut TheRGBABuffer,
        grid: i32,
        color: [u8; 4],
        _ctx: &mut TheContext,
    ) {
        for t in tiles {
            let x = t.0 * grid;
            let y = t.1 * grid;

            // Determine which edges to draw based on neighboring tiles
            let draw_left = !tiles.contains(&(t.0 - 1, t.1));
            let draw_right = !tiles.contains(&(t.0 + 1, t.1));
            let draw_top = !tiles.contains(&(t.0, t.1 - 1));
            let draw_bottom = !tiles.contains(&(t.0, t.1 + 1));

            // Draw the appropriate sides of the rectangle
            if draw_left {
                buffer.draw_vertical_line(x, y, y + grid, color);
            }
            if draw_right {
                buffer.draw_vertical_line(x + grid, y, y + grid, color);
            }
            if draw_top {
                buffer.draw_horizontal_line(x, x + grid, y, color);
            }
            if draw_bottom {
                buffer.draw_horizontal_line(x, x + grid, y + grid, color);
            }
        }
    }

    /// Mixes two colors.
    #[inline(always)]
    pub fn mix_color(&self, a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
        [
            (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[3] as f32 / 255.0) + b[3] as f32 / 255.0 * v) * 255.0) as u8,
        ]
    }

    /// Blends rect from the source frame into the dest frame
    pub fn blend_slice(
        &self,
        buffer: &mut TheRGBABuffer,
        source: &[u8],
        rect: &(usize, usize, usize, usize),
        dest_stride: usize,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;

            for x in 0..rect.2 {
                let dd = d + x * 4;
                let ss = s + x * 4;

                if let Some(background) = buffer.at(Vec2::new(x as i32, y as i32)) {
                    let color = &[source[ss], source[ss + 1], source[ss + 2], source[ss + 3]];
                    buffer.pixels_mut()[dd..dd + 4].copy_from_slice(&self.mix_color(
                        &background,
                        color,
                        (color[3] as f32) / 255.0,
                    ));
                }
            }
        }
    }

    /// Gets the current time in milliseconds
    fn _get_time(&self) -> u128 {
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
