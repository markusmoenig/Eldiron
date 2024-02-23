use crate::prelude::*;
use rand::Rng;
use rayon::prelude::*;
use theframework::prelude::*;

/// Settings for the region draw operation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionDrawSettings {
    pub anim_counter: usize,
    pub delta_in_tick: f32,
    pub offset: Vec2i,
    pub delta: f32,
    pub daylight: Vec3f,

    pub show_fx_marker: bool,

    pub time: TheTime,
    pub center_on_character: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl RegionDrawSettings {
    pub fn new() -> Self {
        Self {
            anim_counter: 0,
            delta_in_tick: 0.0,
            offset: Vec2i::zero(),
            delta: 0.0,
            daylight: Vec3f::one(),

            show_fx_marker: false,

            time: TheTime::default(),
            center_on_character: None,
        }
    }
}

pub struct TileDrawer {
    pub tiles: FxHashMap<Uuid, TheRGBATile>,
    tiles_by_name: FxHashMap<String, TheRGBATile>,
}

#[allow(clippy::new_without_default)]
impl TileDrawer {
    pub fn new() -> Self {
        Self {
            tiles: FxHashMap::default(),
            tiles_by_name: FxHashMap::default(),
        }
    }

    /// Set the tiles.
    pub fn set_tiles(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        self.tiles = tiles;
        self.tiles_by_name.clear();
        for tile in self.tiles.values() {
            self.tiles_by_name.insert(tile.name.clone(), tile.clone());
        }
    }

    /// Draw the region
    pub fn draw_region(
        &self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        update: &mut RegionUpdate,
        settings: &RegionDrawSettings,
    ) -> Vec<(Vec2i, Uuid, Uuid)> {
        let _start = self.get_time();

        let server_tick = update.server_tick;

        let width = buffer.dim().width as usize;

        let region_width = (region.width * region.grid_size) as usize;
        let region_height = region.height * region.grid_size;

        let grid_size = region.grid_size as f32;
        let mut offset = settings.offset;

        // Collect the character positions.

        // The pixel position of the characters with their tile id.
        let mut characters: Vec<(Vec2i, Uuid, Uuid)> = vec![];

        for (id, character) in &mut update.characters {
            let draw_pos = if let Some((start, end)) = &mut character.moving {
                // pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
                //     let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
                //     t * t * (3.0 - 2.0 * t)
                // }

                let sum = (settings.delta + character.move_delta).clamp(0.0, 1.0);
                // let d = smoothstep(0.0, 1.0, sum);//.clamp(0.0, 1.0);
                let d = sum;
                // let d = if sum < 0.5 {
                //     2.0 * sum * sum
                // } else {
                //     1.0 - (-2.0 * sum + 2.0).powi(2) / 2.0
                // };
                let x = start.x * (1.0 - d) + end.x * d;
                let y = start.y * (1.0 - d) + end.y * d;
                character.move_delta = sum;
                vec2i(
                    (x * grid_size).round() as i32,
                    (y * grid_size).round() as i32,
                )
            } else {
                vec2i(
                    (character.position.x * grid_size) as i32,
                    (character.position.y * grid_size) as i32,
                )
            };

            if Some(*id) == settings.center_on_character {
                let center_x = (buffer.dim().width as f32 / 2.0) as i32 - region.grid_size / 2;
                let center_y = (buffer.dim().height as f32 / 2.0) as i32 + region.grid_size / 2;
                offset.x += draw_pos.x - center_x;
                offset.y += region_height - (draw_pos.y + center_y);
            }

            if let Some(tile_id) = self.get_tile_id_by_name(character.tile_name.clone()) {
                characters.push((draw_pos, tile_id, character.id));
            }
        }

        // Fill the code level with the blocking info and collect lights
        let mut level = TheCodeLevel::new(region.width, region.height, settings.time);
        region.fill_code_level(&mut level, &self.tiles, update);

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

                    let mut xx = x % region.grid_size;
                    let mut yy = y % region.grid_size;

                    let mut color = BLACK;
                    let mut tile_is_empty = true;

                    let mut daylight = settings.daylight;
                    let mut show_fx_marker = false;

                    if let Some(tile) = region.tiles.get(&(tile_x, tile_y)) {
                        tile_is_empty = false;

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
                        if settings.show_fx_marker && tile.tilefx.is_some() {
                            show_fx_marker = true;
                        }

                        // Check for FX
                        if let Some(tilefx) = &tile.tilefx {
                            if let Some(TheValue::FloatRange(value, _)) = tilefx.get(
                                str!("Daylight"),
                                str!("Attenuation"),
                                &settings.time,
                                TheInterpolation::Linear,
                            ) {
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
                            if let Some(TheValue::FloatRange(brightness, _)) = tilefx.get(
                                str!("Brightness"),
                                str!("Brightness"),
                                &settings.time,
                                TheInterpolation::Linear,
                            ) {
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
                    }

                    // Items
                    for item in update.items.values() {
                        if tile_x == item.position.x as i32 && tile_y == item.position.y as i32 {
                            tile_is_empty = false;
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
                    for (pos, tile, _) in &characters {
                        if let Some(data) = self.tiles.get(tile) {
                            let index = settings.anim_counter % data.buffer.len();

                            let xx = x - pos.x;
                            let yy = y - pos.y;

                            if let Some(c) = data.buffer[index].at(vec2i(xx, yy)) {
                                tile_is_empty = false;
                                color = self.mix_color(&color, &c, c[3] as f32 / 255.0);
                            }
                        }
                    }

                    if !tile_is_empty {
                        self.render(vec2i(x, y), &mut color, region, update, &level, daylight);
                    }

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
        // println!("drawing time {:?}", _stop - _start);

        characters
    }

    /// Sample the lights and apply all TileFX for the pixel.
    pub fn render(
        &self,
        p: Vec2i,
        c: &mut [u8; 4],
        region: &Region,
        _update: &RegionUpdate,
        level: &TheCodeLevel,
        daylight: Vec3f,
    ) {
        //let mut rng = rand::thread_rng();

        let mut color = TheColor::from_u8_array(*c).to_vec3f();

        let ro = Vec2f::from(p) / region.grid_size as f32;

        // If no lights apply world brightness
        if level.lights.is_empty() {
            color *= daylight;
        } else {
            // Sample the lights
            let mut total_light = Vec3f::new(0.0, 0.0, 0.0);
            for (light_grid, light_coll) in &level.lights {
                let light_pos = Vec2f::from(*light_grid) + vec2f(0.5, 0.5);
                let light_max_distance = 10.0;
                let mut light_strength = light_coll.get_f32_default("Emission Strength", 1.0);
                let light_sampling_off = light_coll.get_f32_default("Sample Offset", 0.5);
                let light_samples = light_coll.get_i32_default("Samples #", 5) as usize;
                let light_color = light_coll.get_i32_default("Light Color", 0);
                let light_limiter = light_coll.get_i32_default("Limit Direction", 0);

                if light_color == 1 {
                    light_strength = daylight.x;
                }

                if light_limiter == 1 && ro.y > light_pos.y {
                    continue;
                }
                if light_limiter == 2 && ro.x < light_pos.x {
                    continue;
                }
                if light_limiter == 3 && ro.y < light_pos.y {
                    continue;
                }
                if light_limiter == 4 && ro.x > light_pos.x {
                    continue;
                }

                let offsets = [
                    ro,
                    ro - vec2f(0.0, light_sampling_off),
                    ro - vec2f(light_sampling_off, 0.0),
                    ro + vec2f(light_sampling_off, 0.0),
                    ro + vec2f(0.0, light_sampling_off),
                    ro - vec2f(light_sampling_off, light_sampling_off),
                    ro + vec2f(light_sampling_off, light_sampling_off),
                ];

                for s in offsets.iter().take(light_samples) {
                    let ro = s;

                    let mut light_dir = light_pos - ro;
                    let light_dist = length(light_dir);

                    if light_dist < light_max_distance {
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
                            let intensity = 1.0 - (max_t / light_max_distance).clamp(0.0, 1.0);
                            //intensity *= if s == 0 { 2.0 } else { 1.0 };
                            total_light += intensity * light_strength / light_samples as f32;
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
    }

    #[inline(always)]
    fn _ray_casting(
        &self,
        ro: Vec2f,
        rd: Vec2f,
        pixels_per_cell: f32,
        target: Vec2i,
        level: &TheCodeLevel,
    ) -> Option<f32> {
        let (mut x, mut y) = (ro.x / pixels_per_cell, ro.y / pixels_per_cell); // Convert to grid coords

        let step_x = if rd.x > 0.0 { 1 } else { -1 };
        let step_y = if rd.y > 0.0 { 1 } else { -1 };

        let mut t_max_x = if rd.x > 0.0 {
            ((x.floor() + 1.0) * pixels_per_cell - ro.x) / rd.x
        } else {
            (x.floor() * pixels_per_cell - ro.x) / rd.x
        };

        let mut t_max_y = if rd.y > 0.0 {
            ((y.floor() + 1.0) * pixels_per_cell - ro.y) / rd.y
        } else {
            (y.floor() * pixels_per_cell - ro.y) / rd.y
        };

        let t_delta_x = pixels_per_cell / rd.x.abs();
        let t_delta_y = pixels_per_cell / rd.y.abs();

        for _ in 0..10 {
            let ix = x as i32;
            let iy = y as i32;

            if ix == target.x && iy == target.y {
                let distance = ((ro.x - x * pixels_per_cell).powi(2)
                    + (ro.y - y * pixels_per_cell).powi(2))
                .sqrt();
                return Some(distance); // Return distance in pixels
            }

            if level.is_blocking((ix, iy)) {
                break;
            }

            // Advance to next grid cell
            if t_max_x < t_max_y {
                t_max_x += t_delta_x;
                x += step_x as f32;
            } else {
                t_max_y += t_delta_y;
                y += step_y as f32;
            }
        }

        None // No obstacle hit
    }

    #[inline(always)]
    fn _ray_casting_2(
        &self,
        ro: Vec2f,
        rd: Vec2f,
        pixels_per_cell: f32,
        level: &TheCodeLevel,
    ) -> Option<(f32, Vec2i, Vec2f)> {
        // Return type changed to include normal vector
        let (mut x, mut y) = (ro.x / pixels_per_cell, ro.y / pixels_per_cell);

        let step_x = if rd.x > 0.0 { 1 } else { -1 };
        let step_y = if rd.y > 0.0 { 1 } else { -1 };

        let mut t_max_x = if rd.x > 0.0 {
            ((x.floor() + 1.0) * pixels_per_cell - ro.x) / rd.x
        } else {
            (x.floor() * pixels_per_cell - ro.x) / rd.x
        };

        let mut t_max_y = if rd.y > 0.0 {
            ((y.floor() + 1.0) * pixels_per_cell - ro.y) / rd.y
        } else {
            (y.floor() * pixels_per_cell - ro.y) / rd.y
        };

        let t_delta_x = pixels_per_cell / rd.x.abs();
        let t_delta_y = pixels_per_cell / rd.y.abs();

        for _ in 0..3 {
            let ix = x as i32;
            let iy = y as i32;

            if level.is_blocking((ix, iy)) {
                let distance = ((ro.x - x * pixels_per_cell).powi(2)
                    + (ro.y - y * pixels_per_cell).powi(2))
                .sqrt();

                let normal = if t_max_x < t_max_y {
                    // Hit was on a vertical wall
                    Vec2f::new(-step_x as f32, 0.0)
                } else {
                    // Hit was on a horizontal wall
                    Vec2f::new(0.0, -step_y as f32)
                };

                return Some((distance, vec2i(ix, iy), normal)); // Return distance, grid hit and normal
            }

            // Advance to next grid cell
            if t_max_x < t_max_y {
                t_max_x += t_delta_x;
                x += step_x as f32;
            } else {
                t_max_y += t_delta_y;
                y += step_y as f32;
            }
        }

        None // No obstacle hit
    }

    fn _uniform_vector(&self, rng: &mut ThreadRng) -> Vec2f {
        let an = rng.gen_range(0.0..=1.0) * 2.0 * std::f32::consts::PI;
        vec2f(an.sin(), an.cos())
    }

    fn _box_intersect(&self, ro: Vec2f, rd: Vec2f, bo: Vec4f) -> f32 {
        let oc = ro - bo.xy();
        let m = 1.0 / rd;
        let n = -m * oc;
        let k = abs(m) * bo.zw();

        let t1 = n - k;
        let t2 = n + k;

        let tn = max(t1.x, t1.y);
        let tf = min(t2.x, t2.y);

        if tn > tf || tf < 0.0 {
            return -1.;
        }

        let q = abs(oc) - bo.zw();
        let g = max(q.x, q.y);

        if g > 0.0 {
            tn
        } else {
            tf
        }
    }

    /*
    pub fn draw_tile(
        &self,
        at: Vec2i,
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
        at: Vec2i,
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
        ctx: &mut TheContext,
    ) {
        for t in tiles {
            let x = (t.0 * grid) as usize;
            let y = (t.1 * grid) as usize;
            let stride = buffer.stride();
            ctx.draw
                .rect_outline(buffer.pixels_mut(), &(x, y, 24, 24), stride, &color);
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

                if let Some(background) = buffer.at(vec2i(x as i32, y as i32)) {
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
